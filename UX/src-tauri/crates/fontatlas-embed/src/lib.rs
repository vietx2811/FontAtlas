//! fontatlas-embed: embedding ảnh font (SigLIP 2 qua candle), whitening, hàng đợi nền.
use serde::Serialize;

pub mod candle_enc;
pub use candle_enc::{Backend, CandleEncoder, select_backend};
pub mod encoder;
pub mod model;
pub mod render;
pub use model::{DownloadProgress, Manifest, ModelStatus, ModelStore, READ_TIMEOUT, Source, VISION_PREFIX, bundled_manifest, disk_free, extract_vision_file, make_manifest, sha256_file, vision_header};
pub mod whiten;
pub mod source;
pub use source::StoreEmbeddingSource;
pub mod worker;
pub use worker::{EmbedWorker, Progress, WorkerConfig};
#[cfg(feature = "fit")]
pub use whiten::fit;
pub use whiten::Whitening;
pub use encoder::Encode;
pub use render::{Gray224, coverage_ratio, render_input, to_tensor};

/// Model, runtime, chuỗi render: thành phần của `raw_version`.
pub const MODEL_NAME: &str = "siglip2-base-p16-224@75de2d5";
/// CPU và Metal dùng chung version: cos ≥ 0,9999 giữa hai thiết bị (test cpu_and_metal_agree, whitenfit --compare-cpu).
pub const RUNTIME_ID: &str = "candle0.11";
pub const RENDER_ID: &str = "render-v2";

pub const SAMPLE_LINES: [&str; 3] = ["Hamburg", "Người Việt", "Rgaeo 0123"];
pub const CANVAS: u32 = 224;
pub const MARGIN: f32 = 12.0;
pub const BATCH: usize = 4;
pub const RAW_DIM: usize = 768;
pub const DIM: usize = 128;
/// Chuẩn hoá SigLIP: (v/255 − 0.5)/0.5.
pub const NORM_MEAN: f32 = 0.5;
pub const NORM_STD: f32 = 0.5;

/// Version của vector thô (không gồm whitening). `model_sha256` = sha của file safetensors vision cục bộ.
pub fn raw_version(model_sha256: &str) -> String {
    format!("{MODEL_NAME}|st-vision@{}|{RUNTIME_ID}|{RENDER_ID}", short(model_sha256))
}

/// Version của vector đã whiten = raw_version + whitening.
pub fn model_version(raw_version: &str, whitening_version: &str, whitening_sha256: &str) -> String {
    format!("{raw_version}|{whitening_version}@{}", short(whitening_sha256))
}

fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

/// Chia họ thành nửa "tune"/"test" cố định: byte đầu của blake3(lowercase(family)) chẵn → tune.
pub fn family_split(family: &str) -> &'static str {
    let h = blake3::hash(family.to_lowercase().as_bytes());
    if h.as_bytes()[0].is_multiple_of(2) { "tune" } else { "test" }
}

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("font: {0}")]
    Font(String),
    #[error("chuỗi mẫu thiếu quá nhiều ký tự")]
    NoGlyphs,
    #[error("render: {0}")]
    Render(String),
    #[error("model: {0}")]
    Model(String),
    #[error("checksum lệch: cần {expected}, nhận {got}")]
    ChecksumMismatch { expected: String, got: String },
    #[error("đã huỷ")]
    Cancelled,
    #[error("embedding đang tắt")]
    Disabled,
    #[error("store: {0}")]
    Store(#[from] fontatlas_store::StoreError),
}

impl EmbedError {
    pub fn code(&self) -> &'static str {
        match self {
            EmbedError::Io(_) => "io",
            EmbedError::Font(_) => "font",
            EmbedError::NoGlyphs => "no_glyphs",
            EmbedError::Render(_) => "render_failed",
            EmbedError::Model(_) => "inference",
            EmbedError::ChecksumMismatch { .. } => "checksum",
            EmbedError::Cancelled => "cancelled",
            EmbedError::Disabled => "disabled",
            EmbedError::Store(_) => "store",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeReason {
    Disabled,
    ModelMissing,
    ModelPartial,
    ModelCorrupt,
    LoadFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedMode {
    MeasuresOnly(ModeReason),
    Images { model_version: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compose() {
        let raw = raw_version(&"ab".repeat(32));
        assert_eq!(raw, format!("siglip2-base-p16-224@75de2d5|st-vision@{}|candle0.11|render-v2", "ab".repeat(6)));
        assert_eq!(model_version(&raw, "white-v1", &"cd".repeat(32)), format!("{raw}|white-v1@{}", "cd".repeat(6)));
    }

    #[test]
    fn any_component_changes_version() {
        let r1 = raw_version(&"ab".repeat(32));
        assert_ne!(r1, raw_version(&"ac".repeat(32)));
        assert_ne!(model_version(&r1, "white-v1", &"cd".repeat(32)), model_version(&r1, "white-v2", &"cd".repeat(32)));
    }

    #[test]
    fn family_split_deterministic_balanced_case_insensitive() {
        let t = (0..1000).filter(|i| family_split(&format!("Family {i}")) == "tune").count();
        assert!((400..600).contains(&t), "{t}");
        assert_eq!(family_split("Roboto"), family_split("roboto"));
    }

    #[test]
    fn error_codes_stable() {
        assert_eq!(EmbedError::NoGlyphs.code(), "no_glyphs");
        assert_eq!(EmbedError::Cancelled.code(), "cancelled");
        assert_eq!(EmbedError::Disabled.code(), "disabled");
    }
}
