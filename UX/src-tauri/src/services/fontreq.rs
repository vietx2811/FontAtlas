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
    let backend = crate::services::broker::AppBackend::new(state);
    let (b_ensure, b_on) = (backend.clone(), backend);
    let responder = Arc::new(Responder::new(move |req| b_ensure.ensure(req), move || b_on.enabled(), PidAppNamer));
    // Ghi thời gian trả lời: app hỏi chờ trong lúc này (Photoshop chờ bao lâu chưa đo; xem docs L2).
    let observer: Arc<Observer> = Arc::new(|pid, req, answer, took| {
        let name = req.ps.as_deref().or(req.family.as_deref()).unwrap_or("");
        if took > Duration::from_secs(2) {
            log::warn!("font request pid {pid} \"{name}\" trả lời chậm: {took:?}");
        } else {
            log::debug!("font request pid {pid} \"{name}\" → {} ({took:?})", if matches!(answer, fontatlas_fontreq::Answer::Nothing) { "không có" } else { "file" });
        }
    });
    start_with(state, RETRY_EVERY, move || {
        FontRequestSource::start(responder.clone(), Some(observer.clone()))
            .map(|s| Box::new(s) as Box<dyn std::any::Any + Send>)
            .map_err(|e| (e.code(), e.to_string()))
    });
}

/// Giữa hai lần thử lại khi mở nguồn không được. Nguồn là tài nguyên của cả phiên đăng nhập: có thể đang bận (máy tải nặng,
/// dịch vụ font chưa sẵn sàng sau khi đăng nhập) và rảnh lại sau đó, nên không bỏ cuộc sau lần đầu.
pub const RETRY_EVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// Mở nguồn bằng `open`; lỗi thì ghi mã vào `state.fontreq_error` (Settings đọc qua `auto_status`) và thử lại mỗi `retry`
/// trên một luồng nền cho tới khi mở được hoặc app thoát. Tách khỏi CoreText để test đường lỗi.
#[doc(hidden)]
pub fn start_with<F>(state: &Arc<AppState>, retry: std::time::Duration, open: F)
where
    F: Fn() -> Result<Box<dyn std::any::Any + Send>, (&'static str, String)> + Send + 'static,
{
    use std::sync::atomic::Ordering;
    if attempt(state, &open, retry) {
        return;
    }
    let weak = Arc::downgrade(state);
    let _ = std::thread::Builder::new().name("fontatlas-fontreq-retry".into()).spawn(move || loop {
        // ngủ từng nhịp ngắn để thoát app không phải chờ cả khoảng thử lại
        let until = std::time::Instant::now() + retry;
        while std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(50).min(retry));
            match weak.upgrade() {
                Some(s) if !s.fontreq_stop.load(Ordering::SeqCst) => {}
                _ => return,
            }
        }
        let Some(s) = weak.upgrade() else { return };
        if s.fontreq_stop.load(Ordering::SeqCst) || attempt(&s, &open, retry) {
            return;
        }
    });
}

fn attempt<F>(state: &AppState, open: &F, retry: std::time::Duration) -> bool
where
    F: Fn() -> Result<Box<dyn std::any::Any + Send>, (&'static str, String)>,
{
    let Ok(mut slot) = state.fontreq.lock() else { return true };
    if slot.is_some() {
        return true;
    }
    match open() {
        Ok(src) => {
            log::info!("nguồn font request đã mở");
            *slot = Some(src);
            if let Ok(mut e) = state.fontreq_error.lock() {
                *e = None;
            }
            true
        }
        Err((code, msg)) => {
            log::warn!("không mở được nguồn font request ({code}): {msg}; thử lại sau {retry:?}");
            if let Ok(mut e) = state.fontreq_error.lock() {
                *e = Some(code.to_string());
            }
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn start(_state: &Arc<AppState>) {}

/// Đóng nguồn (chờ luồng dừng, tối đa một nhịp runloop ~0,25 s).
pub fn stop(state: &AppState) {
    state.fontreq_stop.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut slot) = state.fontreq.lock() {
        slot.take();
    }
}
