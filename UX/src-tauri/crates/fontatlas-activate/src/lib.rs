//! fontatlas-activate: kích hoạt / tắt font của thư viện cho mọi app trên máy, không copy file.
//!
//! - [`ActivationEngine`]: lớp nói chuyện với hệ điều hành (CoreText trên macOS; Windows là stub).
//! - [`FakeEngine`]: bản trong bộ nhớ, mô phỏng ngữ nghĩa CoreText đo ở `docs/team/spikes/04_activation.md`.
mod app_sets;
mod deactivate;
mod engine;
mod fake;
#[cfg(target_os = "macos")]
mod mac;
mod paths;
mod reconcile;
mod relocate;
mod service;
mod types;
mod windows;

pub use engine::{ActivationEngine, Scope};
pub use fake::FakeEngine;
#[cfg(target_os = "macos")]
pub use mac::MacCoreTextEngine;
pub use paths::{in_protected_font_dir, is_variable_font, replace_tier, CLOUD_DOWNLOAD_TIMEOUT, EnsureLocalError, FileProbe, canonical_path, classify_owner, ensure_local, ensure_local_holding, ensure_local_holding_cancellable, probe_file};
pub use app_sets::{AppSetEvent, AppSetEventKind, AppSetTracker, DEFAULT_GRACE};
pub use relocate::PENDING_KEY as RELOCATE_PENDING_KEY;
pub use reconcile::{AutoDeactivated, ReconcileReport};
pub use service::{ActivationService, Availability};
pub use types::{
    ActivationError, ActivationReport, ActivationStateDto, Conflict, ConfirmedConflict, ConflictPolicy, DeactivatePolicy, ENGINE_VERSION, EngineError, FileOutcome, FileResult,
    Owner, Progress, ReplaceTier, ReplacedFont, RestoreOutcome, SETTING_ACTIVATION_KEEP, SETTING_ACTIVATION_ONBOARDED,
};
pub use windows::WindowsGdiEngine;

/// Engine của nền tảng đang chạy: macOS → CoreText; nơi khác → stub Windows (`Unsupported`).
pub fn platform_engine() -> Box<dyn ActivationEngine> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacCoreTextEngine)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(WindowsGdiEngine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_engine_method() {
        let e = platform_engine();
        let want = if cfg!(target_os = "macos") { "coretext" } else { "gdi" };
        assert_eq!(e.method(), want);
    }

    #[test]
    fn boxed_engine_delegates() {
        let e: Box<dyn ActivationEngine> = Box::new(FakeEngine::new());
        assert_eq!(e.method(), "fake");
        e.register(std::path::Path::new("/a.ttf"), Scope::Session).unwrap();
        assert_eq!(e.register(std::path::Path::new("/a.ttf"), Scope::Session), Err(EngineError::AlreadyRegistered));
    }
}
