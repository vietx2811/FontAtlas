//! Khoá setting của app (bảng `u.settings`). Khoá của Plan 4/7 nằm trong fontatlas-store.
pub const ONBOARDING_STEP: &str = "onboarding_step";
pub const LEGACY_IMPORT: &str = "legacy_import";
pub const LEGACY_RESOLVE: &str = "legacy_resolve";
pub const ANALYZER_SEEN: &str = "analyzer_version_seen";
pub const RESCAN_ON_LAUNCH: &str = "rescan_on_launch";
/// Tự tải font còn trên ổ cloud khi thẻ vào khung nhìn để vẽ preview (mặc định bật).
pub const CLOUD_PREVIEW_DOWNLOAD: &str = "cloud_preview_download";
/// Tự phát hiện thay đổi trên thư mục nguồn cloud (FSEvents → quét lại). Mặc định bật.
pub const CLOUD_WATCH: &str = "cloud_watch";
pub const UI_LANG: &str = "ui_lang";
pub const SAMPLE_TEXT: &str = "sample_text";
pub const PROJECT_VI_DEFAULT: &str = "project_vi_default";
pub const MODEL_LAST_ERROR: &str = "model_last_error";
/// Thư mục gốc của Organized Library (B7); rỗng/không có = chưa tạo.
pub const ORGANIZED_ROOT: &str = "organized_root";
