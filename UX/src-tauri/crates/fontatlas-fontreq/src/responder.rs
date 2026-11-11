use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use fontatlas_broker::{EnsureError, EnsureRequest, EnsureResponse, FontReq, FontStatus, is_internal_name};

/// Cùng một tên thư viện không có: trong ngần này giây trả rỗng ngay, không hỏi lại thư viện.
/// (Photoshop hỏi lại mỗi lần dựng chữ; xem spike 06-1.)
pub const NEGATIVE_TTL: Duration = Duration::from_secs(10);

/// Request chạy quá hạn (hoặc `busy`/`not_downloaded`): trả rỗng ngay trong ngần này giây để hàng đợi của MỌI app không bị
/// giữ lại bởi cùng một tên chậm; việc vẫn chạy nốt ở nền và kết quả tốt được dùng lại ở lần hỏi sau.
pub const SLOW_TTL: Duration = Duration::from_secs(3);

/// Kết quả TỐT từ việc chạy nền (đến muộn) được nhớ ngần này giây để lần Photoshop hỏi lại trả ngay.
pub const LATE_RESULT_TTL: Duration = Duration::from_secs(20);

/// Hạn tối đa một request được giữ app hỏi (và cả hàng đợi: nguồn CoreText chỉ có MỘT luồng runloop, callback nào treo thì
/// request của mọi app xếp sau nó). Ngắn hơn `AUTO_DOWNLOAD_TIMEOUT` (8 s) của tải file đám mây có chủ đích: tải vẫn chạy nốt ở nền.
pub const ANSWER_DEADLINE: Duration = Duration::from_secs(3);

/// Số việc nền chạy cùng lúc tối đa; hơn nữa thì trả rỗng ngay (chống một app hỏi dồn tên lạ làm đầy luồng).
pub const MAX_INFLIGHT: usize = 4;

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

type Ensure = dyn Fn(&EnsureRequest) -> Result<EnsureResponse, EnsureError> + Send + Sync + 'static;

#[derive(Clone)]
enum Cached {
    Nothing(Instant),
    Files(PathBuf, Instant),
}

pub struct Responder {
    ensure: Arc<Ensure>,
    enabled: Box<dyn Fn() -> bool + Send + Sync>,
    namer: Box<dyn AppNamer>,
    ttl: Duration,
    slow_ttl: Duration,
    deadline: Duration,
    closed: Arc<AtomicBool>,
    inflight: Arc<AtomicUsize>,
    cache: Arc<Mutex<HashMap<String, Cached>>>,
}

/// Giảm đếm việc nền kể cả khi `ensure` panic.
struct InflightGuard(Arc<AtomicUsize>);
impl Drop for InflightGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn put(cache: &Mutex<HashMap<String, Cached>>, key: String, v: Cached, overwrite: bool) {
    if let Ok(mut m) = cache.lock() {
        if m.len() > 512 {
            let now = Instant::now();
            m.retain(|_, c| match c {
                Cached::Nothing(u) | Cached::Files(_, u) => *u > now,
            });
        }
        let live = |c: &Cached| match c {
            Cached::Nothing(u) | Cached::Files(_, u) => *u > Instant::now(),
        };
        if overwrite || !m.get(&key).is_some_and(live) {
            m.insert(key, v);
        }
    }
}

impl Responder {
    /// `ensure`: thường là `ensure_in_library` qua backend của app. `enabled`: công tắc Auto-activation (đọc MỖI request,
    /// nên bật/tắt có hiệu lực ngay, không cần dừng nguồn).
    pub fn new(
        ensure: impl Fn(&EnsureRequest) -> Result<EnsureResponse, EnsureError> + Send + Sync + 'static,
        enabled: impl Fn() -> bool + Send + Sync + 'static,
        namer: impl AppNamer + 'static,
    ) -> Self {
        Self {
            ensure: Arc::new(ensure),
            enabled: Box::new(enabled),
            namer: Box::new(namer),
            ttl: NEGATIVE_TTL,
            slow_ttl: SLOW_TTL,
            deadline: ANSWER_DEADLINE,
            closed: Arc::new(AtomicBool::new(false)),
            inflight: Arc::new(AtomicUsize::new(0)),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Chỉ cho test: đổi thời hạn nhớ "không có".
    pub fn with_negative_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Chỉ cho test: đổi hạn trả lời.
    pub fn with_deadline(mut self, d: Duration) -> Self {
        self.deadline = d;
        self
    }

    /// Chỉ cho test: đổi thời hạn nhớ "chậm".
    pub fn with_slow_ttl(mut self, ttl: Duration) -> Self {
        self.slow_ttl = ttl;
        self
    }

    /// Đang tắt (app thoát): mọi request trả rỗng ngay, không chờ việc nền.
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    /// Việc nền đang chạy (cho test).
    pub fn inflight(&self) -> usize {
        self.inflight.load(Ordering::SeqCst)
    }

    pub fn answer(&self, pid: i32, req: &Request) -> Answer {
        if self.closed.load(Ordering::SeqCst) || !(self.enabled)() || !req.has_name() {
            return Answer::Nothing;
        }
        // Mọi tên được hỏi đều là tên nội bộ/hệ thống → khỏi hỏi thư viện (lõi cũng lọc, đây chỉ là đường tắt rẻ).
        if [&req.ps, &req.family].iter().filter_map(|x| x.as_deref()).all(is_internal_name) {
            return Answer::Nothing;
        }
        // Chính chúng ta hỏi chính chúng ta (vd xem trước font của app) thì không phải việc của nguồn này.
        if pid == std::process::id() as i32 {
            return Answer::Nothing;
        }
        let key = req.key();
        if let Ok(mut m) = self.cache.lock() {
            match m.get(&key).cloned() {
                Some(Cached::Nothing(until)) if until > Instant::now() => return Answer::Nothing,
                Some(Cached::Files(p, until)) if until > Instant::now() && p.exists() => return Answer::Files(vec![p]),
                Some(_) => {
                    m.remove(&key);
                }
                None => {}
            }
        }
        if self.inflight.fetch_add(1, Ordering::SeqCst) >= MAX_INFLIGHT {
            self.inflight.fetch_sub(1, Ordering::SeqCst);
            return Answer::Nothing;
        }
        let app = self.namer.app_name(pid).unwrap_or_else(|| "other".into());
        let er = EnsureRequest {
            v: 1,
            app,
            doc: None,
            fonts: vec![FontReq { ps: req.ps.clone(), family: req.family.clone(), style: req.style.clone() }],
        };
        // Việc chạy ở luồng phụ có hạn cứng: quá `deadline` (hoặc panic) → rỗng, không giữ app hỏi VÀ hàng đợi của app khác.
        // Luồng phụ vẫn chạy nốt và ghi kết quả vào cache cho lần hỏi sau.
        let (tx, rx) = mpsc::channel();
        let (ensure, cache, ttl, slow_ttl, k2) = (self.ensure.clone(), self.cache.clone(), self.ttl, self.slow_ttl, key.clone());
        let guard = InflightGuard(self.inflight.clone());
        let spawned = std::thread::Builder::new().name("fontatlas-fontreq-ensure".into()).spawn(move || {
            let _g = guard;
            let answer = classify(ensure(&er), &cache, &k2, ttl, slow_ttl);
            let _ = tx.send(answer);
        });
        if spawned.is_err() {
            self.inflight.fetch_sub(1, Ordering::SeqCst);
            return Answer::Nothing;
        }
        // Chờ theo lát 50 ms để `close()` (app thoát) cắt ngang ngay thay vì chờ hết hạn.
        let started = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(a) => return a,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Answer::Nothing,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if self.closed.load(Ordering::SeqCst) {
                        return Answer::Nothing;
                    }
                    if started.elapsed() >= self.deadline {
                        // chậm: lần sau trả rỗng ngay trong `slow_ttl`; nếu việc nền kịp xong thì kết quả tốt của nó thắng
                        put(&self.cache, key, Cached::Nothing(Instant::now() + self.slow_ttl), false);
                        return Answer::Nothing;
                    }
                }
            }
        }
    }
}

/// Đổi kết quả của lõi thành câu trả lời + ghi cache thích hợp.
fn classify(r: Result<EnsureResponse, EnsureError>, cache: &Mutex<HashMap<String, Cached>>, key: &str, ttl: Duration, slow_ttl: Duration) -> Answer {
    let Ok(resp) = r else { return Answer::Nothing };
    let Some(first) = resp.results.first() else { return Answer::Nothing };
    if first.status.usable()
        && let Some(p) = &first.path
    {
        let p = PathBuf::from(p);
        put(cache, key.to_string(), Cached::Files(p.clone(), Instant::now() + LATE_RESULT_TTL), true);
        return Answer::Files(vec![p]);
    }
    match first.status {
        // thư viện thật sự không có: nhớ lâu
        FontStatus::Missing => put(cache, key.to_string(), Cached::Nothing(Instant::now() + ttl), true),
        // tạm thời (đang tải đám mây, bận): nhớ ngắn để không dồn request, rồi hỏi lại
        FontStatus::NotDownloaded | FontStatus::Busy => put(cache, key.to_string(), Cached::Nothing(Instant::now() + slow_ttl), true),
        _ => {}
    }
    Answer::Nothing
}
