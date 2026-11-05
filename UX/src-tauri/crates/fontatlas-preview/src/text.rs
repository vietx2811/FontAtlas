use unicode_normalization::UnicodeNormalization;

use crate::{DEFAULT_SAMPLE, LATIN_FALLBACK, MAX_TEXT_CHARS, PX_BUCKETS, PreviewError};

/// Nguồn của chữ thực sự được vẽ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSource {
    Requested,
    LatinFallback,
    CmapSample,
    Nothing,
}

pub fn check_px(px: u32) -> Result<u32, PreviewError> {
    if PX_BUCKETS.contains(&px) { Ok(px) } else { Err(PreviewError::BadSize(px)) }
}

/// Canvas 2× của vùng preview: W = 13·px, H = 7·px/3.
pub fn canvas_size(px: u32) -> (u32, u32) {
    (13 * px, 7 * px / 3)
}

/// NFC → trim → rỗng thì chữ mặc định → cắt tối đa MAX_TEXT_CHARS ký tự.
pub fn normalize_text(raw: &str) -> String {
    let s: String = raw.nfc().collect();
    let s = s.trim();
    let s = if s.is_empty() { DEFAULT_SAMPLE } else { s };
    s.chars().take(MAX_TEXT_CHARS).collect()
}

/// Chọn chữ vẽ: chữ yêu cầu → chữ Latin dự phòng → mẫu từ cmap → không có gì.
pub fn pick_text(
    requested: &str,
    has: impl Fn(char) -> bool,
    cmap_sample: impl FnOnce() -> String,
) -> (String, TextSource) {
    if requested.chars().filter(|c| !c.is_whitespace()).all(&has) {
        return (requested.to_string(), TextSource::Requested);
    }
    if LATIN_FALLBACK.chars().all(&has) {
        return (LATIN_FALLBACK.to_string(), TextSource::LatinFallback);
    }
    let sample = cmap_sample();
    if sample.is_empty() { (String::new(), TextSource::Nothing) } else { (sample, TextSource::CmapSample) }
}
