//! Thống kê cho thư viện tham chiếu: phân vị, D_fam, Spearman.
use std::collections::{BTreeMap, HashMap};

use fontatlas_index::{
    AxisId, AxisQuantiles, FaceProfile, MIN_SAMPLES, ReferenceTable, Role, Thresholds,
    global_distance,
};

/// 101 phân vị (q[i] = giá trị ở vị trí round(i/100·(n−1))); < MIN_SAMPLES → None.
pub fn quantiles(values: &mut Vec<f32>) -> Option<[f32; 101]> {
    values.retain(|v| v.is_finite());
    if (values.len() as u32) < MIN_SAMPLES {
        return None;
    }
    values.sort_by(f32::total_cmp);
    let n = values.len();
    let mut q = [0.0f32; 101];
    for (i, slot) in q.iter_mut().enumerate() {
        let pos = ((i as f64 / 100.0) * (n - 1) as f64).round() as usize;
        *slot = values[pos.min(n - 1)];
    }
    Some(q)
}

/// Hạng trung bình (giá trị bằng nhau nhận hạng giữa).
fn ranks(v: &[f32]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut r = vec![0.0; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        let mid = (i + j) as f64 / 2.0;
        for k in i..=j {
            r[idx[k]] = mid;
        }
        i = j + 1;
    }
    r
}

/// Spearman r; n < 30, độ dài lệch, giá trị không hữu hạn hoặc phương sai hạng 0 → None.
pub fn spearman(x: &[f32], y: &[f32]) -> Option<f32> {
    if x.len() != y.len() || x.len() < 30 || x.iter().chain(y).any(|v| !v.is_finite()) {
        return None;
    }
    let (rx, ry) = (ranks(x), ranks(y));
    let n = rx.len() as f64;
    let (mx, my) = (rx.iter().sum::<f64>() / n, ry.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in rx.iter().zip(&ry) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return None;
    }
    let r = (sxy / (sxx * syy).sqrt()) as f32;
    r.is_finite().then_some(r)
}

/// D_fam: p95 của khoảng cách toàn cục giữa hai độ đậm kề nhau (Δw ≤ 100) trong cùng family,
/// chỉ face đứng có weight_class. Trả (p95, số cặp).
pub fn d_fam(profiles: &[FaceProfile], t: &Thresholds, r: &ReferenceTable) -> Option<(f32, u32)> {
    let mut fam: HashMap<String, Vec<&FaceProfile>> = HashMap::new();
    for p in profiles {
        let upright = p.get(AxisId::SlantDeg).value.is_some_and(|s| s.abs() < 1.0);
        if upright && p.weight_class.is_some() {
            fam.entry(p.family.to_lowercase()).or_default().push(p);
        }
    }
    let mut d: Vec<f32> = Vec::new();
    for v in fam.values_mut() {
        v.sort_by_key(|p| (p.weight_class, p.key.clone()));
        for w in v.windows(2) {
            let (a, b) = (w[0], w[1]);
            let dw = b.weight_class? as i32 - a.weight_class? as i32;
            if dw > 0 && dw <= 100 {
                d.extend(global_distance(a, Role::Body, b, Role::Body, t, r));
            }
        }
    }
    d.retain(|x| x.is_finite());
    if d.is_empty() {
        return None;
    }
    d.sort_by(f32::total_cmp);
    let pos = ((0.95 * (d.len() - 1) as f64).round() as usize).min(d.len() - 1);
    Some((d[pos], d.len() as u32))
}

/// Dựng bảng tham chiếu 2 lượt: phân vị theo (trục, method), rồi D_fam bằng chính bảng đó.
pub fn build_reference(profiles: &[FaceProfile], version: &str, source: &str) -> ReferenceTable {
    let mut groups: BTreeMap<(usize, String), Vec<f32>> = BTreeMap::new();
    for p in profiles {
        for (i, a) in AxisId::ALL.iter().enumerate() {
            let r = p.get(*a);
            if let Some(v) = r.value.filter(|v| v.is_finite()) {
                groups.entry((i, r.method.clone())).or_default().push(v);
            }
        }
    }
    let quantiles: Vec<AxisQuantiles> = groups
        .into_iter()
        .filter_map(|((i, method), mut v)| {
            let n = v.len() as u32;
            let q = self::quantiles(&mut v)?;
            Some(AxisQuantiles {
                axis: AxisId::ALL[i].key().into(),
                method,
                n,
                q: q.to_vec(),
            })
        })
        .collect();
    let mut t = ReferenceTable {
        version: version.into(),
        source: source.into(),
        analyzer_version: fontatlas_core::ANALYZER_VERSION.into(),
        faces: profiles.len() as u32,
        quantiles,
        d_fam: 0.0,
        d_fam_pairs: 0,
    };
    if let Some((d, n)) = d_fam(profiles, &Thresholds::v1(), &t) {
        t.d_fam = d;
        t.d_fam_pairs = n;
    }
    t
}
