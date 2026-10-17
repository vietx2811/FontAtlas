//! PCA-whitening có phiên bản: định dạng file, áp dụng, và fit (feature "fit").
//! File: b"FAWHITEN" | u32 LE độ dài header | header JSON | μ[raw_dim] f32 LE | W[raw_dim×dim] f32 LE (hàng-trước).
use sha2::{Digest, Sha256};

use crate::EmbedError;

const MAGIC: &[u8; 8] = b"FAWHITEN";

#[derive(Debug, Clone, PartialEq)]
pub struct Whitening {
    pub version: String,
    pub sha256: String,
    pub raw_dim: usize,
    pub dim: usize,
    mean: Vec<f32>,
    w: Vec<f32>,
}

fn bad(m: &str) -> EmbedError {
    EmbedError::Model(format!("whitening: {m}"))
}

fn l2(v: &mut [f32]) -> bool {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if !(n.is_finite() && n > 0.0) {
        return false;
    }
    v.iter_mut().for_each(|x| *x /= n);
    true
}

fn f32s(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

impl Whitening {
    pub fn from_bytes(b: &[u8]) -> Result<Self, EmbedError> {
        if b.len() < 12 || &b[..8] != MAGIC {
            return Err(bad("magic"));
        }
        let hl = u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize;
        let hend = 12usize.checked_add(hl).filter(|e| *e <= b.len()).ok_or_else(|| bad("header"))?;
        let h: serde_json::Value = serde_json::from_slice(&b[12..hend]).map_err(|_| bad("header json"))?;
        let get = |k: &str| h.get(k).and_then(|v| v.as_u64()).map(|v| v as usize).ok_or_else(|| bad(k));
        let (raw_dim, dim) = (get("raw_dim")?, get("dim")?);
        let version = h.get("version").and_then(|v| v.as_str()).ok_or_else(|| bad("version"))?.to_string();
        let need = raw_dim.checked_mul(dim + 1).and_then(|x| x.checked_mul(4)).ok_or_else(|| bad("size"))?;
        if raw_dim == 0 || dim == 0 || b.len() - hend != need {
            return Err(bad("size"));
        }
        let mean = f32s(&b[hend..hend + raw_dim * 4]);
        let w = f32s(&b[hend + raw_dim * 4..]);
        if mean.iter().chain(&w).any(|x| !x.is_finite()) {
            return Err(bad("non-finite"));
        }
        let sha256 = format!("{:x}", Sha256::digest(b));
        Ok(Whitening { version, sha256, raw_dim, dim, mean, w })
    }

    /// L2(thô) → trừ μ → nhân W → L2. Sai chiều hoặc chuẩn 0 → None.
    pub fn apply(&self, raw: &[f32]) -> Option<Vec<f32>> {
        if raw.len() != self.raw_dim {
            return None;
        }
        let mut x = raw.to_vec();
        if !l2(&mut x) {
            return None;
        }
        x.iter_mut().zip(&self.mean).for_each(|(a, m)| *a -= m);
        let mut out = vec![0f32; self.dim];
        for (r, xv) in x.iter().enumerate() {
            let row = &self.w[r * self.dim..(r + 1) * self.dim];
            out.iter_mut().zip(row).for_each(|(o, w)| *o += xv * w);
        }
        l2(&mut out).then_some(out)
    }

    pub fn to_bytes(&self, header: &serde_json::Value) -> Vec<u8> {
        let h = header.to_string();
        let mut b = MAGIC.to_vec();
        b.extend_from_slice(&(h.len() as u32).to_le_bytes());
        b.extend_from_slice(h.as_bytes());
        for v in self.mean.iter().chain(&self.w) {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b
    }
}

/// Fit PCA-whitening: hiệp phương sai của các vector đã L2 và tâm hoá; giữ `dim` trục,
/// chia trục i cho √λᵢ + shrink·√λ₁.
#[cfg(feature = "fit")]
pub fn fit(raw: &[Vec<f32>], dim: usize, shrink: f32) -> Result<Whitening, EmbedError> {
    use nalgebra::{DMatrix, SymmetricEigen};
    let rd = raw.first().map(|v| v.len()).ok_or_else(|| bad("empty"))?;
    if raw.len() < dim + 1 || dim > rd || raw.iter().any(|v| v.len() != rd) {
        return Err(bad("samples"));
    }
    let rows: Vec<Vec<f32>> = raw
        .iter()
        .filter_map(|v| {
            let mut x = v.clone();
            l2(&mut x).then_some(x)
        })
        .collect();
    let n = rows.len();
    if n < dim + 1 {
        return Err(bad("samples"));
    }
    let mut mean = vec![0f64; rd];
    for r in &rows {
        r.iter().zip(mean.iter_mut()).for_each(|(x, m)| *m += *x as f64);
    }
    mean.iter_mut().for_each(|m| *m /= n as f64);
    let x = DMatrix::<f64>::from_fn(n, rd, |i, j| rows[i][j] as f64 - mean[j]);
    let cov = (x.transpose() * &x) / n as f64;
    let eig = SymmetricEigen::new(cov);
    let mut order: Vec<usize> = (0..rd).collect();
    order.sort_by(|a, b| eig.eigenvalues[*b].total_cmp(&eig.eigenvalues[*a]));
    let l1 = eig.eigenvalues[order[0]].max(0.0).sqrt();
    let mut w = vec![0f32; rd * dim];
    for (c, &k) in order.iter().take(dim).enumerate() {
        let scale = eig.eigenvalues[k].max(0.0).sqrt() + shrink as f64 * l1;
        if !(scale > 0.0) {
            return Err(bad("degenerate"));
        }
        for r in 0..rd {
            w[r * dim + c] = (eig.eigenvectors[(r, k)] / scale) as f32;
        }
    }
    let mut out = Whitening {
        version: String::new(),
        sha256: String::new(),
        raw_dim: rd,
        dim,
        mean: mean.iter().map(|m| *m as f32).collect(),
        w,
    };
    // version/sha chỉ có nghĩa sau khi ghi file; tính theo header mặc định để apply/roundtrip nhất quán.
    let b = out.to_bytes(&serde_json::json!({"version":"white-fit","raw_dim":rd,"dim":dim,"shrink":shrink,"n":n}));
    out.version = "white-fit".into();
    out.sha256 = format!("{:x}", Sha256::digest(&b));
    Ok(out)
}
