//! Khoảng cách Similar: trục chuẩn hoá theo phân vị, trọng số nhóm, embedding tuỳ chọn.
use serde::{Deserialize, Serialize};

use crate::profile::{AxisId, FaceKey};

/// Số chiều tối thiểu cả hai bên cùng có để so được.
pub const MIN_SHARED_DIMS: usize = 8;
/// Trọng số phần ảnh khi cả hai có embedding. Chọn trên nửa tune của Google Fonts @47d54ca
/// (eval, R@1+R@5 cùng họ tăng đều tới trần: w 0,5 → 67,0/85,6 %; 0,8 → 70,7/89,1 %).
pub const EMBED_WEIGHT: f32 = 0.8;
/// Trần trọng số ảnh: phần trục luôn còn ≥ 1 − MAX_EMBED_WEIGHT.
pub const MAX_EMBED_WEIGHT: f32 = 0.8;

/// Khoảng cách kết hợp: (1 − w)·d_trục + w·(1 − cos)/2.
pub fn combine(d_axis: f32, cos: f32, w: f32) -> f32 {
    (1.0 - w) * d_axis + w * (1.0 - cos) / 2.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Proportion,
    Weight,
    Structure,
    Detail,
}

impl AxisId {
    pub fn group(self) -> Group {
        match self {
            AxisId::XHeightEm
            | AxisId::XHeightCap
            | AxisId::AscenderEm
            | AxisId::Width
            | AxisId::MeanAdvance
            | AxisId::WidthCv
            | AxisId::Spacing => Group::Proportion,
            AxisId::StemNXh | AxisId::Darkness | AxisId::Contrast => Group::Weight,
            AxisId::Aperture | AxisId::ApertureE | AxisId::StressDeg | AxisId::Counter => {
                Group::Structure
            }
            AxisId::SerifFoot | AxisId::SlantDeg => Group::Detail,
        }
    }
}

/// Tuỳ chọn Similar theo yêu cầu. `embed_weight`: `None` = mặc định của Index; `Some(0.0)` = chỉ trục.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimilarOptions {
    pub weights: GroupWeights,
    pub embed_weight: Option<f32>,
    pub k: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroupWeights {
    pub proportion: f32,
    pub weight: f32,
    pub structure: f32,
    pub detail: f32,
}

impl Default for GroupWeights {
    fn default() -> Self {
        GroupWeights {
            proportion: 1.0,
            weight: 1.0,
            structure: 1.0,
            detail: 1.0,
        }
    }
}

impl GroupWeights {
    pub fn of(&self, g: Group) -> f32 {
        let w = match g {
            Group::Proportion => self.proportion,
            Group::Weight => self.weight,
            Group::Structure => self.structure,
            Group::Detail => self.detail,
        };
        if w.is_finite() && w > 0.0 { w } else { 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarSource {
    Axes,
    AxesAndImage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarHit {
    pub key: FaceKey,
    pub distance: f32,
    pub source: SimilarSource,
    /// "Giống ở": tối đa 3 trục gần nhất (|Δp| ≤ 0,10).
    pub same: Vec<AxisId>,
    /// "Khác ở": tối đa 3 trục xa nhất (|Δp| ≥ 0,25).
    pub differ: Vec<AxisId>,
}

/// d = √(Σ wᵢ (pa−pb)² / Σ wᵢ), wᵢ = trọng số nhóm × conf_a × conf_b; trả (d, số chiều chung).
/// Thiếu chiều chung (< MIN_SHARED_DIMS) hoặc Σw = 0 → None.
pub fn axis_distance(
    pa: &[Option<f32>; 16],
    ca: &[f32; 16],
    pb: &[Option<f32>; 16],
    cb: &[f32; 16],
    w: &GroupWeights,
) -> Option<(f32, usize)> {
    let (mut num, mut den, mut shared) = (0.0f32, 0.0f32, 0usize);
    for (i, axis) in AxisId::ALL.iter().enumerate() {
        if let (Some(x), Some(y)) = (pa[i], pb[i]) {
            shared += 1;
            let wi = w.of(axis.group()) * ca[i] * cb[i];
            num += wi * (x - y) * (x - y);
            den += wi;
        }
    }
    if shared < MIN_SHARED_DIMS || den <= 0.0 {
        return None;
    }
    let d = (num / den).sqrt();
    d.is_finite().then_some((d, shared))
}

/// cos giữa hai embedding; lệch chiều, chuẩn 0 hoặc không hữu hạn → None.
pub fn embed_cos(a: Option<&[f32]>, b: Option<&[f32]>) -> Option<f32> {
    let (a, b) = (a?, b?);
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if !(na > 0.0 && nb > 0.0) {
        return None;
    }
    let c = dot / (na * nb);
    c.is_finite().then_some(c.clamp(-1.0, 1.0))
}
