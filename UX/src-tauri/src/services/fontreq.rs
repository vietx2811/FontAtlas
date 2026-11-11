//! Nối nguồn font request của hệ thống (`fontatlas-fontreq`) vào app: mở sau broker, đóng trước broker khi thoát.
//! Photoshop/After Effects hỏi font còn thiếu → thư viện bật session và trả file. Công tắc là setting
//! `auto_activation_enabled` (đọc mỗi request, không cần dừng/mở nguồn). Windows/Linux: không làm gì.
use std::sync::Arc;

use crate::state::AppState;

#[cfg(target_os = "macos")]
pub fn start(state: &Arc<AppState>) {
    use fontatlas_broker::Backend;
    use fontatlas_fontreq::{FontRequestSource, Observer, PidAppNamer, Responder};
    use std::time::Duration;

    if !state.activation.supported() || !state.system_hooks.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    // Nguồn này là của CẢ phiên đăng nhập (mọi app hỏi font đều tới). Bản chạy với thư mục dữ liệu riêng
    // (FONTATLAS_DATA_DIR: QA, test, thử máy) không được nhận thay thư viện thật; ép mở bằng FONTATLAS_FONTREQ=1.
    if std::env::var_os("FONTATLAS_DATA_DIR").is_some() && std::env::var_os("FONTATLAS_FONTREQ").is_none() {
        return;
    }
    let Ok(mut slot) = state.fontreq.lock() else { return };
    if slot.is_some() {
        return;
    }
    let backend = crate::services::broker::AppBackend::new(state);
    let (b_ensure, b_on) = (backend.clone(), backend);
    let responder = Arc::new(Responder::new(move |req| b_ensure.ensure(req), move || b_on.enabled(), PidAppNamer));
    // Ghi thời gian trả lời: app hỏi chờ trong lúc này (Photoshop chờ bao lâu chưa đo; xem docs L2).
    let observer: Arc<Observer> = Arc::new(|pid, req, answer, took| {
        let name = req.ps.as_deref().or(req.family.as_deref()).unwrap_or("");
        if took > Duration::from_secs(3) {
            log::warn!("font request pid {pid} \"{name}\" trả lời chậm: {took:?}");
        } else {
            log::debug!("font request pid {pid} \"{name}\" → {} ({took:?})", if matches!(answer, fontatlas_fontreq::Answer::Nothing) { "không có" } else { "file" });
        }
    });
    match FontRequestSource::start(responder, Some(observer)) {
        Ok(src) => {
            log::info!("nguồn font request đã mở");
            *slot = Some(Box::new(src));
        }
        Err(e) => log::warn!("không mở được nguồn font request: {e}"),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn start(_state: &Arc<AppState>) {}

/// Đóng nguồn (chờ luồng dừng, tối đa một nhịp runloop ~0,25 s).
pub fn stop(state: &AppState) {
    if let Ok(mut slot) = state.fontreq.lock() {
        slot.take();
    }
}
