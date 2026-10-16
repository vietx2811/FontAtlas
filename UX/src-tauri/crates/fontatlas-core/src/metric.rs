//! Giá trị đo kèm chất lượng — không bao giờ trả mặc định im lặng.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailReason {
    NoLatinLowercase,
    MissingGlyph,
    NoOutlines,
    LowContrast,
    Degenerate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "reason", rename_all = "snake_case")]
pub enum Quality {
    Measured,
    Estimated,
    Failed(FailReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Metric {
    pub value: Option<f32>,
    pub quality: Quality,
    pub method: &'static str,
}

impl Metric {
    fn with(v: f64, q: Quality, method: &'static str) -> Self {
        if v.is_finite() {
            Metric { value: Some(v as f32), quality: q, method }
        } else {
            Metric::failed(FailReason::Degenerate, method)
        }
    }
    pub fn measured(v: f64, method: &'static str) -> Self {
        Self::with(v, Quality::Measured, method)
    }
    pub fn estimated(v: f64, method: &'static str) -> Self {
        Self::with(v, Quality::Estimated, method)
    }
    pub fn failed(reason: FailReason, method: &'static str) -> Self {
        Metric { value: None, quality: Quality::Failed(reason), method }
    }
    /// Từ Option: None → Failed(reason).
    pub fn from_opt(v: Option<f64>, q: Quality, reason: FailReason, method: &'static str) -> Self {
        match v {
            Some(v) => Self::with(v, q, method),
            None => Self::failed(reason, method),
        }
    }
}
