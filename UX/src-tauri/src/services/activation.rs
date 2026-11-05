//! Lớp mỏng trên service kích hoạt của Plan 7 (Task 17 đầy đủ). Ở đây chỉ có phần AppState cần.

/// Bọc object-safe để `AppState` giữ engine thật hoặc `FakeEngine` mà không generic hoá.
pub trait ActivationApi: Send + Sync {
    /// "coretext" | "gdi" | "fake" | "none"
    fn method(&self) -> &'static str;
    /// Engine có kích hoạt được trên máy này không (Windows stub: không).
    fn supported(&self) -> bool;
}

/// Trước khi Task 17 nối service thật.
pub struct NoActivation;

impl ActivationApi for NoActivation {
    fn method(&self) -> &'static str {
        "none"
    }

    fn supported(&self) -> bool {
        false
    }
}

/// Thoát app: Task 17 gọi `shutdown` của Plan 7 (gỡ font scope session).
pub fn on_exit(_state: &crate::state::AppState) {}

/// Bật/tắt giữ font kích hoạt sau khi thoát. Task 17 nối thêm `set_scope` của Plan 7 và cờ onboarded.
pub fn set_keep(state: &crate::state::AppState, on: bool) -> Result<(), crate::error::CmdError> {
    state.with_user(|u| Ok(u.set_setting(fontatlas_activate::SETTING_ACTIVATION_KEEP, if on { "1" } else { "0" })?))
}
