//! Vòng đời app: thoát gọn (huỷ và chờ quét, tải model, embed; rồi gỡ font session).
use crate::state::AppState;

/// Gọi từ `RunEvent::Exit`; test gọi trực tiếp (không cần webview). Tổng ≈ ≤ 5 s mỗi worker.
pub fn on_exit(state: &AppState) {
    state.scans.shutdown();
    crate::services::model::cancel_download(state);
    crate::services::model::wait_download(state, std::time::Duration::from_secs(5));
    state.embed.shutdown();
    crate::services::activation::on_exit(state);
}
