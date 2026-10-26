//! Thư viện tham chiếu: phân vị từng trục theo cặp (trục, method), cộng D_fam.
use serde::{Deserialize, Serialize};

use crate::profile::{AxisId, FaceProfile};

/// Bảng có ít mẫu hơn ngưỡng này thì không chuẩn hoá.
pub const MIN_SAMPLES: u32 = 30;

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("reference: {0}")]
    Reference(String),
    #[error("không có face {0:?}")]
    NotFound(crate::profile::FaceKey),
    #[error("face {0:?} không đủ trục để so")]
    NotComparable(crate::profile::FaceKey),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisQuantiles {
    pub axis: String,
    pub method: String,
    pub n: u32,
    /// Đúng 101 giá trị (phân vị 0..100), không giảm.
    pub q: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceTable {
    pub version: String,
    pub source: String,
    pub analyzer_version: String,
    pub faces: u32,
    pub quantiles: Vec<AxisQuantiles>,
    /// Khoảng cách toàn cục trung vị giữa các face cùng family (R1/R7).
    pub d_fam: f32,
    pub d_fam_pairs: u32,
}

impl ReferenceTable {
    pub fn from_json(s: &str) -> Result<Self, IndexError> {
        let t: ReferenceTable =
            serde_json::from_str(s).map_err(|e| IndexError::Reference(e.to_string()))?;
        for aq in &t.quantiles {
            let ok = aq.q.len() == 101
                && aq.q.iter().all(|v| v.is_finite())
                && aq.q.windows(2).all(|w| w[0] <= w[1]);
            if !ok {
                return Err(IndexError::Reference(format!(
                    "{}/{}: cần 101 giá trị hữu hạn, không giảm",
                    aq.axis, aq.method
                )));
            }
        }
        if !t.d_fam.is_finite() {
            return Err(IndexError::Reference("d_fam không hữu hạn".into()));
        }
        Ok(t)
    }

    /// Bảng phát hành kèm crate (google/fonts ofl+apache ở SHA ghim). File hỏng → panic,
    /// bắt bởi test `bundled_reference_is_valid` trước khi phát hành.
    pub fn bundled() -> Self {
        match Self::from_json(include_str!("../data/reference-v1.json")) {
            Ok(t) => t,
            Err(e) => panic!("reference-v1.json hỏng: {e}"),
        }
    }

    fn table(&self, axis: AxisId, method: &str) -> Option<&[f32]> {
        self.quantiles
            .iter()
            .find(|q| q.axis == axis.key() && q.method == method && q.n >= MIN_SAMPLES)
            .map(|q| q.q.as_slice())
    }

    /// Phân vị 0–1 của `v` theo bảng (trục, method); hạng giữa khi trùng giá trị.
    pub fn percentile(&self, axis: AxisId, method: &str, v: f32) -> Option<f32> {
        if !v.is_finite() {
            return None;
        }
        let q = self.table(axis, method)?;
        let (first, last) = (*q.first()?, *q.last()?);
        // Đoạn giá trị trùng v (|q−v| ≤ 1e-6) → giữa đoạn. q không giảm và q−v đơn điệu theo q,
        // nên đoạn liền nhau: tìm nhị phân, không cấp phát.
        let lo = q.partition_point(|x| x - v < -1e-6);
        let hi = q.partition_point(|x| x - v <= 1e-6);
        if lo < hi {
            return Some((lo + hi - 1) as f32 / 200.0);
        }
        if v < first {
            return Some(0.0);
        }
        if v > last {
            return Some(1.0);
        }
        // Chỉ số lớn nhất có q[i] < v.
        let i = q.partition_point(|x| *x < v).checked_sub(1)?;
        let span = q[i + 1] - q[i];
        let frac = if span > 0.0 { (v - q[i]) / span } else { 0.0 };
        Some((i as f32 + frac) / 100.0)
    }

    /// Phân vị của 16 trục (None khi thiếu giá trị hoặc thiếu bảng của method).
    pub fn normalize(&self, p: &FaceProfile) -> [Option<f32>; 16] {
        let mut out = [None; 16];
        for (i, a) in AxisId::ALL.iter().enumerate() {
            let r = p.get(*a);
            out[i] = r.value.and_then(|v| self.percentile(*a, &r.method, v));
        }
        out
    }
}
