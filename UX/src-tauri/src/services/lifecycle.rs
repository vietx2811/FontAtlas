//! Vòng đời app: thoát gọn (huỷ và chờ quét, tải model, embed; rồi gỡ font session).
use crate::state::AppState;

/// Gọi từ `RunEvent::Exit`; test gọi trực tiếp (không cần webview). Tổng ≈ ≤ 5 s mỗi worker.
pub fn on_exit(state: &AppState) {
    // đặt cờ huỷ cho cả ba hàng đợi trước, rồi mới chờ: tổng thời gian ≈ một lượt chờ, không cộng dồn
    state.scans.request_shutdown();
    crate::services::model::cancel_download(state);
    state.embed.request_stop();
    state.langs.request_stop();
    state.scans.shutdown();
    crate::services::model::wait_download(state, std::time::Duration::from_secs(5));
    state.embed.shutdown();
    state.langs.shutdown();
    crate::services::activation::on_exit(state);
}
