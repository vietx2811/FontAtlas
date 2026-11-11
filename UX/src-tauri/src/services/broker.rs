//! Nối activation broker (`fontatlas-broker`) vào app: mở khi boot xong, đóng khi thoát.
//! Broker chỉ nghe 127.0.0.1; client đọc cổng + token ở `<data dir>/broker.json` (0600).
use std::sync::{Arc, Mutex, Weak};

use fontatlas_activate::{ActivationError, ActivationReport};
use fontatlas_broker::{AutoActivator, Backend, EnsureError, EnsureRequest, EnsureResponse, Server, ServerConfig, auto_enabled, ensure_in_library};
use fontatlas_store::{AutoApp, FaceKey, Store};

use crate::state::AppState;

/// Giữ `Weak` để `AppState` → `Server` → backend → `AppState` không thành vòng giữ nhau.
pub(crate) struct AppBackend(Weak<AppState>);

impl AppBackend {
    pub(crate) fn new(state: &Arc<AppState>) -> Arc<Self> {
        Arc::new(Self(Arc::downgrade(state)))
    }
}

struct ViaApi<'a>(&'a AppState);

impl AutoActivator for ViaApi<'_> {
    fn activate_auto(&self, store: &Mutex<Store>, keys: &[FaceKey], app: AutoApp) -> Result<ActivationReport, ActivationError> {
        self.0.activation.activate_auto(store, keys, app)
    }
}

impl Backend for AppBackend {
    fn enabled(&self) -> bool {
        self.0.upgrade().and_then(|s| s.store.get().map(auto_enabled)).unwrap_or(false)
    }

    fn ensure(&self, req: &EnsureRequest) -> Result<EnsureResponse, EnsureError> {
        let state = self.0.upgrade().ok_or_else(|| EnsureError::Internal("app đang thoát".into()))?;
        let store = state.store().map_err(|e| EnsureError::Internal(e.message))?;
        ensure_in_library(&ViaApi(&state), store, req)
    }
}

/// Mở broker (nếu chưa mở). Lỗi chỉ ghi log: app vẫn chạy, chỉ mất tự bật font.
pub fn start(state: &Arc<AppState>) {
    if !state.activation.supported() {
        return;
    }
    let Ok(mut slot) = state.broker.lock() else { return };
    if slot.is_some() {
        return;
    }
    let backend = AppBackend::new(state);
    match Server::start(ServerConfig::new(&state.dirs.data), backend) {
        Ok(srv) => {
            log::info!("activation broker nghe {}", srv.addr());
            *slot = Some(srv);
        }
        Err(e) => log::warn!("không mở được activation broker: {e}"),
    }
}

/// Đóng broker và xoá `broker.json`.
pub fn stop(state: &AppState) {
    if let Ok(mut slot) = state.broker.lock()
        && let Some(srv) = slot.take()
    {
        srv.shutdown();
    }
}

/// Cổng broker đang nghe (cho trạng thái Settings và test).
pub fn port(state: &AppState) -> Option<u16> {
    state.broker.lock().ok()?.as_ref().map(|s| s.addr().port())
}

/// Broker thật sự trả lời không (ping qua socket). Cổng còn mở chưa đủ: worker có thể đã chết.
pub fn healthy(state: &AppState) -> bool {
    state.broker.lock().ok().and_then(|g| g.as_ref().map(|s| s.probe())).unwrap_or(false)
}
