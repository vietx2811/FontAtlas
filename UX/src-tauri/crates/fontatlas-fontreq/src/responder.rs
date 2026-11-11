use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use fontatlas_broker::{EnsureError, EnsureRequest, EnsureResponse, FontReq};

/// Cùng một tên không có trong thư viện: trong ngần này giây trả rỗng ngay, không hỏi lại thư viện.
/// (Photoshop hỏi lại mỗi lần dựng chữ; xem spike 06-1.)
pub const NEGATIVE_TTL: Duration = Duration::from_secs(10);

/// Điều CoreText hỏi. `ps` là tên PostScript (Photoshop chỉ gửi cái này); family/style chỉ dùng khi app gửi.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub ps: Option<String>,
    pub family: Option<String>,
    pub style: Option<String>,
}

impl Request {
    fn key(&self) -> String {
        format!("{}|{}|{}", self.ps.as_deref().unwrap_or(""), self.family.as_deref().unwrap_or(""), self.style.as_deref().unwrap_or(""))
    }
    fn has_name(&self) -> bool {
        [&self.ps, &self.family].iter().any(|v| v.as_deref().is_some_and(|s| !s.trim().is_empty()))
    }
}

/// Kết quả gửi lại cho hệ thống.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Mảng rỗng: ta không có font này (hoặc không được phép trả).
    Nothing,
    /// File font (đã được bật session ở thư viện).
    Files(Vec<PathBuf>),
}

/// Tên font nội bộ / hệ thống mà không bao giờ có trong thư viện của người dùng. Trả rỗng ngay, không chạm DB.
/// - Adobe: `AdobeSansMM`, `AdobeSerifMM` (font thay thế nội bộ), `AdobeInvisFont`, `AdobeBlank`...
/// - Tên bắt đầu bằng `.` là font hệ thống ẩn (`.SFNS-Regular`, `.AppleSystemUIFont`); `LastResort`; tên rỗng.
pub fn is_internal_name(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n.starts_with('.') || n.chars().any(char::is_control) || n.len() > 512 {
        return true;
    }
    let l = n.to_ascii_lowercase();
    l.starts_with("adobesansmm") || l.starts_with("adobeserifmm") || l.starts_with("adobeinvis") || l.starts_with("adobeblank") || l == "lastresort"
}

/// "/Applications/Adobe Photoshop 2026/Adobe Photoshop 2026.app/Contents/MacOS/Adobe Photoshop 2026" → "Adobe Photoshop 2026".
/// Lấy thành phần `.app` NGOÀI CÙNG: đó là sản phẩm người dùng mở (helper lồng bên trong vẫn tính cho app mẹ).
/// Trả `None` nếu đường dẫn không nằm trong gói `.app`.
pub fn app_label_from_exe_path(path: &str) -> Option<String> {
    path.split('/').find_map(|c| c.strip_suffix(".app")).map(str::to_string).filter(|s| !s.is_empty())
}

/// Hỏi tên app theo pid (macOS: `proc_pidpath`; test: giả).
pub trait AppNamer: Send + Sync {
    fn app_name(&self, pid: i32) -> Option<String>;
}

impl<F: Fn(i32) -> Option<String> + Send + Sync> AppNamer for F {
    fn app_name(&self, pid: i32) -> Option<String> {
        self(pid)
    }
}

type Ensure = dyn Fn(&EnsureRequest) -> Result<EnsureResponse, EnsureError> + Send + Sync;

pub struct Responder {
    ensure: Box<Ensure>,
    enabled: Box<dyn Fn() -> bool + Send + Sync>,
    namer: Box<dyn AppNamer>,
    ttl: Duration,
    missing: Mutex<HashMap<String, Instant>>,
}

impl Responder {
    /// `ensure`: thường là `ensure_in_library` qua backend của app. `enabled`: công tắc Auto-activation (đọc MỖI request,
    /// nên bật/tắt có hiệu lực ngay, không cần dừng nguồn).
    pub fn new(
        ensure: impl Fn(&EnsureRequest) -> Result<EnsureResponse, EnsureError> + Send + Sync + 'static,
        enabled: impl Fn() -> bool + Send + Sync + 'static,
        namer: impl AppNamer + 'static,
    ) -> Self {
        Self { ensure: Box::new(ensure), enabled: Box::new(enabled), namer: Box::new(namer), ttl: NEGATIVE_TTL, missing: Mutex::new(HashMap::new()) }
    }

    /// Chỉ cho test: đổi thời hạn nhớ "không có".
    pub fn with_negative_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn answer(&self, pid: i32, req: &Request) -> Answer {
        if !(self.enabled)() || !req.has_name() {
            return Answer::Nothing;
        }
        if req.ps.as_deref().is_some_and(is_internal_name) && req.family.is_none() {
            return Answer::Nothing;
        }
        // Chính chúng ta hỏi chính chúng ta (vd xem trước font của app) thì không phải việc của nguồn này.
        if pid == std::process::id() as i32 {
            return Answer::Nothing;
        }
        let key = req.key();
        if let Ok(mut m) = self.missing.lock() {
            if let Some(at) = m.get(&key) {
                if at.elapsed() < self.ttl {
                    return Answer::Nothing;
                }
                m.remove(&key);
            }
            if m.len() > 512 {
                let ttl = self.ttl;
                m.retain(|_, at| at.elapsed() < ttl);
            }
        }
        let app = self.namer.app_name(pid).unwrap_or_else(|| "other".into());
        let er = EnsureRequest {
            v: 1,
            app,
            doc: None,
            fonts: vec![FontReq { ps: req.ps.clone(), family: req.family.clone(), style: req.style.clone() }],
        };
        let resp = match (self.ensure)(&er) {
            Ok(r) => r,
            Err(_) => return Answer::Nothing,
        };
        let path = resp.results.first().filter(|r| r.status.usable()).and_then(|r| r.path.clone());
        match path {
            Some(p) => Answer::Files(vec![PathBuf::from(p)]),
            None => {
                // Chỉ nhớ "không có" khi thư viện thật sự không có; lỗi tạm (busy, failed, chưa tải xong) thì hỏi lại lần sau.
                let definitive = resp.results.first().is_some_and(|r| r.status == fontatlas_broker::FontStatus::Missing);
                if definitive && let Ok(mut m) = self.missing.lock() {
                    m.insert(key, Instant::now());
                }
                Answer::Nothing
            }
        }
    }
}
