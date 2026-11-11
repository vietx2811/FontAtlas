//! Khớp font app hỏi với thư viện rồi bật (scope session, lý do `auto:<app>`).
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use fontatlas_activate::{ActivationEngine, ActivationError, ActivationReport, ActivationService, FileOutcome};
use fontatlas_store::{AutoApp, AutoLogEntry, FaceKey, Store};
use serde::{Deserialize, Serialize};

use crate::filter::is_internal_name;
use crate::server::Backend;
use crate::{PROTOCOL_VERSION, SETTING_AUTO_ACTIVATION};

/// Tối đa font mỗi yêu cầu.
pub const MAX_FONTS: usize = 256;
/// Tối đa ký tự mỗi chuỗi (tên font, style, tên tài liệu).
pub const MAX_TEXT: usize = 512;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct FontReq {
    /// PostScript name; khớp trước.
    #[serde(default)]
    pub ps: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub style: Option<String>,
}

impl FontReq {
    /// Nhãn trả lại cho client và ghi nhật ký: PS name, hoặc "family / style".
    fn label(&self) -> String {
        match (nonempty(&self.ps), nonempty(&self.family)) {
            (Some(ps), _) => ps.to_string(),
            (None, Some(f)) => format!("{f} / {}", nonempty(&self.style).unwrap_or("Regular")),
            (None, None) => String::new(),
        }
    }
}

fn nonempty(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EnsureRequest {
    pub v: u32,
    pub app: String,
    #[serde(default)]
    pub doc: Option<String>,
    pub fonts: Vec<FontReq>,
}

impl EnsureRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.v != PROTOCOL_VERSION {
            return Err(format!("phiên bản giao thức {} không được hỗ trợ (cần {PROTOCOL_VERSION})", self.v));
        }
        if self.fonts.len() > MAX_FONTS {
            return Err(format!("quá {MAX_FONTS} font trong một yêu cầu"));
        }
        let bad = |s: &str| s.chars().count() > MAX_TEXT || s.chars().any(char::is_control);
        if bad(&self.app) || self.doc.as_deref().is_some_and(bad) {
            return Err("app/doc quá dài hoặc có ký tự điều khiển".into());
        }
        for f in &self.fonts {
            for s in [&f.ps, &f.family, &f.style].into_iter().flatten() {
                if bad(s) {
                    return Err("tên font quá dài hoặc có ký tự điều khiển".into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FontStatus {
    /// Vừa bật.
    Activated,
    /// Đã bật từ trước (thêm lý do cho app này).
    AlreadyActive,
    /// Máy đã có đúng file này do bên khác cung cấp (hệ thống, RightFont…): dùng được, Font Atlas không làm gì.
    Available,
    /// Thư viện không có font này.
    Missing,
    /// Máy có font cùng tên nhưng KHÁC nội dung: không tự ghi đè, người dùng quyết trong app.
    Conflict,
    /// Có trong thư viện nhưng file đã mất / đã đổi.
    Unavailable,
    /// File là placeholder đám mây (OneDrive/iCloud) và chưa tải xong trong hạn chờ; tải vẫn tiếp tục ngầm, hỏi lại sau.
    NotDownloaded,
    /// Hệ điều hành từ chối.
    Failed,
    /// Đang có thao tác khác trên cùng file: hỏi lại sau.
    Busy,
    /// Yêu cầu không có tên.
    Invalid,
}

impl FontStatus {
    /// Sau lần gọi này app dùng được font?
    pub fn usable(&self) -> bool {
        matches!(self, FontStatus::Activated | FontStatus::AlreadyActive | FontStatus::Available)
    }
    fn as_str(&self) -> &'static str {
        match self {
            FontStatus::Activated => "activated",
            FontStatus::AlreadyActive => "already_active",
            FontStatus::Available => "available",
            FontStatus::Missing => "missing",
            FontStatus::Conflict => "conflict",
            FontStatus::Unavailable => "unavailable",
            FontStatus::NotDownloaded => "not_downloaded",
            FontStatus::Failed => "failed",
            FontStatus::Busy => "busy",
            FontStatus::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontResult {
    pub name: String,
    pub status: FontStatus,
    /// File font dùng được (chỉ khi `status` dùng được). Nguồn font request trả đường dẫn này cho hệ thống.
    /// KHÔNG đi qua HTTP (`skip`): đường dẫn trên đĩa chỉ dành cho nguồn font request trong cùng tiến trình.
    #[serde(skip)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnsureResponse {
    pub ok: bool,
    pub v: u32,
    pub app: String,
    pub results: Vec<FontResult>,
    /// Tên (nhãn) dùng được sau lần gọi này.
    pub found: Vec<String>,
    /// Tên chưa dùng được; lý do ở `results`.
    pub missing: Vec<String>,
}

fn outcome_status(o: &FileOutcome) -> FontStatus {
    match o {
        FileOutcome::Activated => FontStatus::Activated,
        FileOutcome::AlreadyActive => FontStatus::AlreadyActive,
        FileOutcome::AlreadyAvailable { .. } => FontStatus::Available,
        FileOutcome::NeedsConfirmation { .. } => FontStatus::Conflict,
        FileOutcome::NotDownloaded => FontStatus::NotDownloaded,
        FileOutcome::MissingFile => FontStatus::Unavailable,
        FileOutcome::Busy => FontStatus::Busy,
        FileOutcome::Failed { code, .. } if code == "cloud_timeout" => FontStatus::NotDownloaded,
        FileOutcome::Failed { code, .. } if code == "missing_file" => FontStatus::Unavailable,
        FileOutcome::Failed { code, .. } if code == "file_changed" || code == "not_found" => FontStatus::Unavailable,
        _ => FontStatus::Failed,
    }
}

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// Việc broker cần từ lớp kích hoạt. `ActivationService<E>` có sẵn; app dùng bản bọc object-safe của nó.
pub trait AutoActivator {
    fn activate_auto(&self, store: &Mutex<Store>, keys: &[FaceKey], app: AutoApp) -> Result<ActivationReport, ActivationError>;
}

impl<E: ActivationEngine> AutoActivator for ActivationService<E> {
    fn activate_auto(&self, store: &Mutex<Store>, keys: &[FaceKey], app: AutoApp) -> Result<ActivationReport, ActivationError> {
        ActivationService::activate_auto(self, store, keys, app)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureError {
    /// Yêu cầu sai (HTTP 400).
    BadRequest(String),
    /// Lỗi bên trong: store, hệ điều hành (HTTP 500).
    Internal(String),
}

impl EnsureError {
    fn internal(e: impl std::fmt::Display) -> Self {
        EnsureError::Internal(e.to_string())
    }
}

/// Cài đặt "tự bật font": thiếu = bật.
pub fn auto_enabled(store: &Mutex<Store>) -> bool {
    lock(store).setting(SETTING_AUTO_ACTIVATION).ok().flatten().as_deref() != Some("0")
}

fn lock(m: &Mutex<Store>) -> MutexGuard<'_, Store> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Nhiều bản khác nội dung (hash) cùng tên: ưu tiên bản ĐANG BẬT (người dùng/collection đã chọn), rồi bản thấy sau cùng
/// (`first_seen_at` lớn nhất), rồi hash nhỏ nhất cho tất định. Font revision (head.fontRevision) chưa được lưu trong thư
/// viện nên chưa dùng được. Trả (bản chọn, số bản khác nhau).
fn pick(s: &Store, cands: Vec<FaceKey>, active: &std::collections::HashSet<[u8; 32]>) -> Result<(Option<FaceKey>, usize), EnsureError> {
    let mut distinct: Vec<[u8; 32]> = cands.iter().map(|k| k.hash).collect();
    distinct.sort();
    distinct.dedup();
    let mut best: Option<(bool, i64, FaceKey)> = None;
    for k in cands {
        let rank = (active.contains(&k.hash), s.file_first_seen(&k.hash).map_err(EnsureError::internal)?);
        let better = match &best {
            None => true,
            Some((a, t, cur)) => (rank.0, rank.1) > (*a, *t) || ((rank.0, rank.1) == (*a, *t) && (k.hash, k.face_index) < (cur.hash, cur.face_index)),
        };
        if better {
            best = Some((rank.0, rank.1, k));
        }
    }
    Ok((best.map(|b| b.2), distinct.len()))
}

/// Lõi của broker: không biết gì về HTTP.
pub fn ensure_in_library(
    service: &dyn AutoActivator,
    store: &Mutex<Store>,
    req: &EnsureRequest,
) -> Result<EnsureResponse, EnsureError> {
    req.validate().map_err(EnsureError::BadRequest)?;
    let app = AutoApp::parse_lenient(&req.app);

    // 1. Tên → face của thư viện (một lần khoá store). Tên nội bộ bị bỏ; nhiều bản cùng tên thì chọn có quy tắc.
    let mut ambiguous: Vec<(usize, usize)> = vec![]; // (chỉ số font trong yêu cầu, số bản khác nhau)
    let resolved: Vec<Option<FaceKey>> = {
        let s = lock(store);
        let active: std::collections::HashSet<[u8; 32]> = s
            .activation_rows()
            .map_err(EnsureError::internal)?
            .into_iter()
            .filter(|r| r.state == fontatlas_store::DesiredState::Active)
            .filter_map(|r| <[u8; 32]>::try_from(r.hash.as_slice()).ok())
            .collect();
        let mut v = Vec::with_capacity(req.fonts.len());
        for (idx, f) in req.fonts.iter().enumerate() {
            let mut cands: Vec<FaceKey> = vec![];
            if let Some(ps) = nonempty(&f.ps).filter(|p| !is_internal_name(p)) {
                cands = s.faces_by_postscript(ps).map_err(EnsureError::internal)?;
            }
            if cands.is_empty() {
                if let Some(fam) = nonempty(&f.family).filter(|p| !is_internal_name(p)) {
                    cands = s.faces_by_family_style(fam, nonempty(&f.style).unwrap_or("")).map_err(EnsureError::internal)?;
                }
            }
            let (hit, distinct) = pick(&s, cands, &active)?;
            if distinct > 1 {
                ambiguous.push((idx, distinct));
            }
            v.push(hit);
        }
        v
    };

    // 2. Bật các file khác nhau, một lô.
    let mut keys: Vec<FaceKey> = vec![];
    for k in resolved.iter().flatten() {
        if !keys.iter().any(|x| x.hash == k.hash && x.face_index == k.face_index) {
            keys.push(*k);
        }
    }
    let mut by_hash: HashMap<String, (FontStatus, Option<String>)> = HashMap::new();
    if !keys.is_empty() {
        let report = service.activate_auto(store, &keys, app).map_err(EnsureError::internal)?;
        for r in &report.results {
            let path = match &r.outcome {
                FileOutcome::AlreadyAvailable { path, .. } => Some(path.clone()),
                _ => r.path.clone(),
            };
            by_hash.insert(r.hash_hex.clone(), (outcome_status(&r.outcome), path));
        }
    }

    // 3. Gom kết quả theo thứ tự yêu cầu; ghi nhật ký (tên nội bộ không vào nhật ký).
    let mut results = Vec::with_capacity(req.fonts.len());
    let mut logs: Vec<AutoLogEntry> = vec![];
    for (f, hit) in req.fonts.iter().zip(&resolved) {
        let name = f.label();
        let (status, path) = if name.is_empty() {
            (FontStatus::Invalid, None)
        } else {
            match hit {
                None => (FontStatus::Missing, None),
                Some(k) => by_hash.get(&hex(&k.hash)).cloned().unwrap_or((FontStatus::Failed, None)),
            }
        };
        let path = if status.usable() { path } else { None };
        let internal = [&f.ps, &f.family].iter().filter_map(|x| nonempty(x)).all(is_internal_name);
        // Lặp lại vô ích (đã bật, đã có sẵn, yêu cầu rỗng, tên nội bộ) không vào nhật ký.
        if !internal && !matches!(status, FontStatus::AlreadyActive | FontStatus::Available | FontStatus::Invalid) {
            logs.push(AutoLogEntry {
                app,
                doc: req.doc.clone(),
                requested: name.clone(),
                outcome: status.as_str().into(),
                hash: hit.as_ref().map(|k| k.hash),
            });
        }
        results.push(FontResult { name, status, path });
    }
    for (idx, n) in &ambiguous {
        let f = &req.fonts[*idx];
        logs.push(AutoLogEntry {
            app,
            doc: req.doc.clone(),
            requested: format!("{} ×{n}", f.label()),
            outcome: "ambiguous".into(),
            hash: resolved[*idx].as_ref().map(|k| k.hash),
        });
    }
    if !logs.is_empty() {
        let mut s = lock(store);
        for l in &logs {
            s.log_auto(l).map_err(EnsureError::internal)?;
        }
    }

    let found = results.iter().filter(|r| r.status.usable()).map(|r| r.name.clone()).collect();
    let missing = results.iter().filter(|r| !r.status.usable()).map(|r| r.name.clone()).collect();
    Ok(EnsureResponse { ok: true, v: PROTOCOL_VERSION, app: req.app.clone(), results, found, missing })
}

/// Backend sở hữu Store + service: dùng cho test và cho nơi nào không có `AppState`.
pub struct OwnedBackend<E: ActivationEngine> {
    pub service: ActivationService<E>,
    pub store: Mutex<Store>,
}

impl<E: ActivationEngine + 'static> Backend for OwnedBackend<E> {
    fn enabled(&self) -> bool {
        auto_enabled(&self.store)
    }
    fn ensure(&self, req: &EnsureRequest) -> Result<EnsureResponse, EnsureError> {
        ensure_in_library(&self.service, &self.store, req)
    }
}
