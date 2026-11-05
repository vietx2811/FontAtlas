//! Model SigLIP: trạng thái, tải tiếp được, huỷ, tải lại, gỡ. Tải chạy ở luồng nền; tiến độ qua `hubs.model`.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use fontatlas_embed::{DownloadProgress, EmbedError, Manifest, ModelStatus, ModelStore};
use serde_json::json;

use crate::dto::events::ModelEvent;
use crate::dto::model::{ErrorInfoDto, ModelState, ModelStatusDto};
use crate::error::CmdError;
use crate::services::keys;
use crate::state::AppState;

#[derive(Default)]
pub struct ModelRuntime {
    downloading: AtomicBool,
    cancel: AtomicBool,
    handle: Mutex<Option<JoinHandle<()>>>,
    /// (đã tải, tổng, byte/s) mới nhất của lần tải đang chạy.
    progress: Mutex<(u64, u64, f64)>,
}

impl ModelRuntime {
    pub fn is_downloading(&self) -> bool {
        self.downloading.load(Ordering::SeqCst)
    }
}

fn store(state: &AppState) -> ModelStore {
    ModelStore::new(state.dirs.models.clone(), state.deps.manifest.clone())
}

fn unpublished(m: &Manifest) -> bool {
    m.source.url.trim().is_empty()
}

fn last_error(state: &AppState) -> Option<ErrorInfoDto> {
    let raw = state.with_user(|u| Ok(u.setting(keys::MODEL_LAST_ERROR)?)).ok()??;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    Some(ErrorInfoDto { code: v.get("code")?.as_str()?.into(), message: v.get("message")?.as_str()?.into() })
}

fn save_error(state: &AppState, e: &EmbedError) {
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let v = json!({ "code": e.code(), "message": e.to_string(), "atMs": ms });
    let _ = state.with_user(|u| Ok(u.set_setting(keys::MODEL_LAST_ERROR, &v.to_string())?));
}

fn clear_error(state: &AppState) {
    let _ = state.with_user(|u| Ok(u.set_setting(keys::MODEL_LAST_ERROR, "")?));
}

/// Nhãn trạng thái cho onboarding: missing | paused | ready | corrupt | unpublished | checksum_mismatch | downloading.
pub fn model_state_label(state: &AppState) -> &'static str {
    match state_of(state) {
        ModelState::Missing => "missing",
        ModelState::Downloading => "downloading",
        ModelState::Paused => "paused",
        ModelState::Ready => "ready",
        ModelState::ChecksumMismatch => "checksum_mismatch",
        ModelState::Corrupt => "corrupt",
        ModelState::Unpublished => "unpublished",
    }
}

pub fn state_of(state: &AppState) -> ModelState {
    if unpublished(&state.deps.manifest) {
        return ModelState::Unpublished;
    }
    if state.model.is_downloading() {
        return ModelState::Downloading;
    }
    match store(state).status() {
        ModelStatus::Ready(_) => ModelState::Ready,
        ModelStatus::Corrupt => ModelState::Corrupt,
        ModelStatus::Partial { .. } => ModelState::Paused,
        ModelStatus::Missing => {
            if last_error(state).is_some_and(|e| e.code == "checksum") {
                ModelState::ChecksumMismatch
            } else {
                ModelState::Missing
            }
        }
    }
}

/// Đường dẫn file model khi `Ready`.
pub fn ready_path(state: &AppState) -> Option<std::path::PathBuf> {
    match store(state).status() {
        ModelStatus::Ready(p) => Some(p),
        _ => None,
    }
}

pub fn model_status(state: &AppState) -> ModelStatusDto {
    let st = state_of(state);
    let m = &state.deps.manifest;
    let (mut done, mut total, mut bps) = (0, m.size, 0.0);
    match (st, store(state).status()) {
        (ModelState::Downloading, _) => {
            if let Ok(p) = state.model.progress.lock() {
                (done, total, bps) = *p;
            }
        }
        (_, ModelStatus::Partial { bytes, total: t }) => (done, total) = (bytes, t),
        (ModelState::Ready, _) => done = m.size,
        _ => {}
    }
    let eta_ms = (bps > 0.0 && total > done).then(|| ((total - done) as f64 / bps * 1000.0) as u64);
    ModelStatusDto {
        state: st,
        done_bytes: done,
        total_bytes: total,
        bytes_per_sec: bps,
        eta_ms,
        location: state.dirs.models.to_string_lossy().into_owned(),
        disk_free: (state.deps.disk_free)(&state.dirs.models).unwrap_or(0),
        required_bytes: (m.size as f64 * 1.1) as u64,
        last_error: last_error(state).filter(|e| !e.code.is_empty()),
    }
}

/// Bắt đầu tải (hoặc tải tiếp nếu có `.part`). Không chặn: tiến độ qua `hubs.model`.
/// Đang tải thì chỉ gắn vào lần tải hiện tại.
pub fn start_download(state: &Arc<AppState>) -> Result<(), CmdError> {
    let m = state.deps.manifest.clone();
    if unpublished(&m) {
        return Err(CmdError::new("model_missing", "model chưa được phát hành"));
    }
    if state.model.is_downloading() {
        return Ok(());
    }
    let ms = store(state);
    if let ModelStatus::Ready(_) = ms.status() {
        state.hubs.model.emit(ModelEvent::Ready);
        return Ok(());
    }
    let have = match ms.status() {
        ModelStatus::Partial { bytes, .. } => bytes,
        _ => 0,
    };
    let required = ((m.size as f64 * 1.1) as u64).saturating_sub(have);
    let free = (state.deps.disk_free)(&state.dirs.models).map_err(|e| CmdError::app_io("disk_free", &e))?;
    if free < required {
        return Err(CmdError::new("insufficient_disk", "không đủ chỗ trống để tải model")
            .with_detail(json!({ "required": required, "free": free })));
    }
    if state.model.downloading.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    state.model.cancel.store(false, Ordering::SeqCst);
    clear_error(state);
    let st = Arc::clone(state);
    let h = std::thread::Builder::new().name("fontatlas-model".into()).spawn(move || run_download(&st, ms));
    match h {
        Ok(h) => {
            if let Ok(mut g) = state.model.handle.lock() {
                *g = Some(h);
            }
            Ok(())
        }
        Err(e) => {
            state.model.downloading.store(false, Ordering::SeqCst);
            Err(CmdError::app_io("spawn", &e))
        }
    }
}

fn run_download(state: &Arc<AppState>, ms: ModelStore) {
    let hub = &state.hubs.model;
    let report = |p: DownloadProgress| {
        if let Ok(mut g) = state.model.progress.lock() {
            *g = (p.done, p.total, p.bytes_per_sec);
        }
        let eta_ms = (p.bytes_per_sec > 0.0 && p.total > p.done).then(|| ((p.total - p.done) as f64 / p.bytes_per_sec * 1000.0) as u64);
        hub.emit(ModelEvent::Progress { done: p.done, total: p.total, bytes_per_sec: p.bytes_per_sec, eta_ms, resumed: p.resumed });
    };
    let r = ms.ensure_resumable(&report, &state.model.cancel);
    state.model.downloading.store(false, Ordering::SeqCst);
    match r {
        Ok(_) => {
            clear_error(state);
            hub.emit(ModelEvent::Ready);
            let _ = crate::services::embed::ensure_started(state);
        }
        Err(EmbedError::Cancelled) => hub.emit(ModelEvent::Paused),
        Err(e) => {
            save_error(state, &e);
            hub.emit(ModelEvent::Failed { error: ErrorInfoDto { code: e.code().into(), message: e.to_string() } });
        }
    }
}

/// Chỉ cho test: bật cờ "đang tải" mà không có luồng.
#[doc(hidden)]
pub fn force_downloading_for_test(state: &AppState, on: bool) {
    state.model.downloading.store(on, Ordering::SeqCst);
}

pub fn cancel_download(state: &AppState) {
    state.model.cancel.store(true, Ordering::SeqCst);
}

/// Chờ lần tải hiện tại kết thúc (≤ `timeout`).
pub fn wait_download(state: &AppState, timeout: Duration) -> bool {
    let h = state.model.handle.lock().ok().and_then(|mut g| g.take());
    let Some(h) = h else { return true };
    let t = std::time::Instant::now();
    while !h.is_finished() && t.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(10));
    }
    if h.is_finished() {
        let _ = h.join();
        true
    } else {
        if let Ok(mut g) = state.model.handle.lock() {
            *g = Some(h);
        }
        false
    }
}

pub fn remove_files(dir: &Path, m: &Manifest) -> Result<(), CmdError> {
    ModelStore::new(dir.to_path_buf(), m.clone()).remove().map_err(|e| CmdError::app_io("model_remove", &e))
}

/// Xoá rồi tải lại từ đầu.
pub fn redownload(state: &Arc<AppState>) -> Result<(), CmdError> {
    redownload_within(state, Duration::from_secs(5))
}

/// Lần tải đang chạy không dừng kịp (server treo) → `busy`, KHÔNG xoá file.
pub fn redownload_within(state: &Arc<AppState>, wait: Duration) -> Result<(), CmdError> {
    cancel_download(state);
    if !wait_download(state, wait) || state.model.is_downloading() {
        return Err(CmdError::busy("lần tải trước chưa dừng, thử lại sau"));
    }
    state.embed.stop();
    remove_files(&state.dirs.models, &state.deps.manifest)?;
    clear_error(state);
    start_download(state)
}

/// Dừng worker, xoá file, về `missing`.
pub fn remove(state: &Arc<AppState>) -> Result<(), CmdError> {
    cancel_download(state);
    if !wait_download(state, Duration::from_secs(5)) || state.model.is_downloading() {
        return Err(CmdError::busy("lần tải đang chạy chưa dừng, thử lại sau"));
    }
    state.embed.stop();
    remove_files(&state.dirs.models, &state.deps.manifest)?;
    clear_error(state);
    Ok(())
}
