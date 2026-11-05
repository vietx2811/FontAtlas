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
