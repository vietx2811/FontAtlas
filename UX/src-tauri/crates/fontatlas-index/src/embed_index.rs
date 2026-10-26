//! Embedding ảnh trong Index: lượng tử int8 (scale theo từng vector) cho lượt quét toàn bộ,
//! xếp lại bằng f32 đọc từ nguồn ngoài (store) cho các ứng viên đầu.
use crate::profile::FaceKey;

/// Số ứng viên (theo khoảng cách kết hợp int8) được xếp lại bằng f32.
pub const RERANK_CANDIDATES: usize = 400;

/// Nguồn vector f32 chính xác (app: bọc `StoreEmbeddingSource`; test/eval: bản trong bộ nhớ).
pub trait ExactEmbeddings: Send + Sync {
    fn fetch(&self, keys: &[FaceKey]) -> Vec<Option<Vec<f32>>>;
}

/// Mảng liền: `dim` byte/face + một hệ số. Vector lệch số chiều chung, chuẩn 0 hoặc không hữu hạn → không có.
pub(crate) struct Q8 {
    dim: usize,
    q: Vec<i8>,
    /// scale / ‖v‖ — cos ≈ Σ qa·qb · inv_a · inv_b.
    inv: Vec<f32>,
    has: Vec<bool>,
}

fn valid(v: &[f32]) -> bool {
    !v.is_empty() && v.iter().all(|x| x.is_finite()) && v.iter().any(|x| *x != 0.0)
}

impl Q8 {
    /// Số chiều chung = số chiều xuất hiện nhiều nhất trong các vector hợp lệ (hoà → nhỏ hơn).
    pub(crate) fn build(embs: &[Option<Vec<f32>>]) -> Q8 {
        let mut counts: std::collections::BTreeMap<usize, usize> = Default::default();
        for v in embs.iter().flatten().filter(|v| valid(v)) {
            *counts.entry(v.len()).or_default() += 1;
        }
        let dim = counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map_or(0, |(d, _)| *d);
        let n = embs.len();
        let mut q = vec![0i8; n * dim];
        let mut inv = vec![0f32; n];
        let mut has = vec![false; n];
        for (i, v) in embs.iter().enumerate() {
            let Some(v) = v.as_deref().filter(|v| v.len() == dim && valid(v)) else {
                continue;
            };
            let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            let maxabs = v.iter().fold(0f32, |m, x| m.max(x.abs()));
            let s = maxabs / 127.0;
            for (o, x) in q[i * dim..(i + 1) * dim].iter_mut().zip(v) {
                *o = (x / s).round().clamp(-127.0, 127.0) as i8;
            }
            inv[i] = s / norm;
            has[i] = true;
        }
        Q8 { dim, q, inv, has }
    }

    pub(crate) fn has(&self, i: usize) -> bool {
        self.has[i]
    }

    pub(crate) fn cos(&self, i: usize, j: usize) -> Option<f32> {
        if !(self.has[i] && self.has[j]) {
            return None;
        }
        let (a, b) = (
            &self.q[i * self.dim..(i + 1) * self.dim],
            &self.q[j * self.dim..(j + 1) * self.dim],
        );
        let dot: i32 = a.iter().zip(b).map(|(x, y)| *x as i32 * *y as i32).sum();
        Some((dot as f32 * self.inv[i] * self.inv[j]).clamp(-1.0, 1.0))
    }
}
