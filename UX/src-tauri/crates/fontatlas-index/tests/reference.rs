use fontatlas_index::{AxisId, ReferenceTable};

fn table() -> ReferenceTable {
    // contrast/o: q[i] = 1 + i*0.05 (1.0..6.0); stem/H (chữ hoa) chỉ 10 mẫu → không dùng.
    let q: Vec<f32> = (0..=100).map(|i| 1.0 + i as f32 * 0.05).collect();
    let j = serde_json::json!({"version":"ref-test","source":"t","analyzer_version":fontatlas_core::ANALYZER_VERSION,"faces":500,
      "quantiles":[{"axis":"contrast","method":"o_radial_thickness_max_min","n":500,"q":q},
                   {"axis":"stem","method":"H_stem_over_cap","n":10,"q":q}],
      "d_fam":0.476,"d_fam_pairs":294});
    ReferenceTable::from_json(&j.to_string()).unwrap()
}

#[test]
fn percentile_interpolates_and_clamps() {
    let t = table();
    let p = |v| {
        t.percentile(AxisId::Contrast, "o_radial_thickness_max_min", v)
            .unwrap()
    };
    assert!((p(1.0) - 0.0).abs() < 1e-6);
    assert!((p(3.5) - 0.5).abs() < 1e-3);
    assert_eq!(p(0.1), 0.0);
    assert_eq!(p(99.0), 1.0);
}

#[test]
fn percentile_is_per_method() {
    let t = table();
    // method khác (nhánh chữ hoa) không tra bảng của chữ thường
    assert_eq!(
        t.percentile(AxisId::Contrast, "O_radial_thickness_max_min", 3.5),
        None
    );
    // bảng có n < MIN_SAMPLES → None
    assert_eq!(t.percentile(AxisId::StemNXh, "H_stem_over_cap", 3.5), None);
}

#[test]
fn ties_use_mid_rank() {
    // slant: 60% face có slant = 0 → q[0..=59] = 0, rồi tăng dần
    let q: Vec<f32> = (0..=100)
        .map(|i| if i < 60 { 0.0 } else { (i - 59) as f32 * 0.5 })
        .collect();
    let j = serde_json::json!({"version":"t","source":"t","analyzer_version":"x","faces":500,
      "quantiles":[{"axis":"slant_deg","method":"l_or_I_midline_offset","n":500,"q":q}],"d_fam":0.5,"d_fam_pairs":1});
    let t = ReferenceTable::from_json(&j.to_string()).unwrap();
    let p0 = t
        .percentile(AxisId::SlantDeg, "l_or_I_midline_offset", 0.0)
        .unwrap();
    assert!((p0 - 0.295).abs() < 1e-3, "{p0}"); // giữa của đoạn 0..0.59
}

#[test]
fn rejects_bad_quantiles() {
    let j = r#"{"version":"x","source":"x","analyzer_version":"x","faces":1,"d_fam":0.5,"d_fam_pairs":1,
      "quantiles":[{"axis":"contrast","method":"m","n":50,"q":[3.0,2.0]}]}"#;
    assert!(ReferenceTable::from_json(j).is_err()); // không đủ 101 hoặc giảm dần
}

#[test]
fn bundled_reference_is_valid() {
    let r = ReferenceTable::bundled();
    assert!(r.version.starts_with("ref-"));
    assert_eq!(r.analyzer_version, fontatlas_core::ANALYZER_VERSION);
    assert!(r.faces >= 1000, "{}", r.faces);
    assert!(r.d_fam > 0.2 && r.d_fam < 1.5, "{}", r.d_fam);
    for a in [
        AxisId::Contrast,
        AxisId::StemNXh,
        AxisId::Aperture,
        AxisId::XHeightEm,
    ] {
        assert!(
            r.percentile(a, a.default_method(), 1.0).is_some()
                || r.percentile(a, a.default_method(), 100.0).is_some()
        );
    }
}

/// Thuật toán cũ (dò tuần tự, cấp phát) — để chứng minh bản tìm nhị phân cho kết quả y hệt.
fn percentile_linear(q: &[f32], v: f32) -> Option<f32> {
    if !v.is_finite() {
        return None;
    }
    let (first, last) = (*q.first()?, *q.last()?);
    let eq: Vec<usize> = (0..q.len()).filter(|&i| (q[i] - v).abs() <= 1e-6).collect();
    if let (Some(lo), Some(hi)) = (eq.first(), eq.last()) {
        return Some((lo + hi) as f32 / 200.0);
    }
    if v < first {
        return Some(0.0);
    }
    if v > last {
        return Some(1.0);
    }
    let i = (0..q.len() - 1).rev().find(|&i| q[i] < v)?;
    let span = q[i + 1] - q[i];
    let frac = if span > 0.0 { (v - q[i]) / span } else { 0.0 };
    Some((i as f32 + frac) / 100.0)
}

#[test]
fn percentile_matches_linear_scan_on_bundled_reference() {
    let r = ReferenceTable::bundled();
    let mut s: u64 = 20261027;
    let mut rnd = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as f32 / (1u64 << 31) as f32
    };
    let mut checked = 0;
    for aq in &r.quantiles {
        let Some(axis) = AxisId::ALL.iter().copied().find(|a| a.key() == aq.axis) else {
            continue;
        };
        let (lo, hi) = (aq.q[0], aq.q[100]);
        let mut vals: Vec<f32> = Vec::new();
        for &x in &aq.q {
            vals.extend([
                x,
                x + 1e-6,
                x - 1e-6,
                x + 5e-7,
                x - 5e-7,
                x.next_up(),
                x.next_down(),
            ]);
        }
        for _ in 0..2000 {
            vals.push(lo - 0.1 * (hi - lo) + rnd() * 1.2 * (hi - lo));
        }
        vals.extend([
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            lo - 1.0,
            hi + 1.0,
        ]);
        for v in vals {
            let a = r.percentile(axis, &aq.method, v);
            let b = percentile_linear(&aq.q, v);
            assert_eq!(
                a.map(f32::to_bits),
                b.map(f32::to_bits),
                "{}/{} v={v}",
                aq.axis,
                aq.method
            );
            checked += 1;
        }
    }
    assert!(checked > 30_000, "{checked}");
}

#[test]
fn magnitude_percentile_symmetric() {
    use fontatlas_index::{AxisQuantiles, ReferenceTable};
    let t = ReferenceTable {
        version: "t".into(),
        source: "t".into(),
        analyzer_version: "t".into(),
        faces: 100,
        quantiles: vec![AxisQuantiles {
            axis: "stress_deg".into(),
            method: "m".into(),
            n: 100,
            q: (0..=100).map(|i| -30.0 + 0.6 * i as f32).collect(),
        }],
        d_fam: 0.0,
        d_fam_pairs: 0,
    };
    let m = |v: f32| t.magnitude_percentile(AxisId::StressDeg, "m", v).unwrap();
    assert!((m(15.0) - 0.5).abs() < 0.02, "{}", m(15.0));
    assert!((m(-15.0) - 0.5).abs() < 0.02);
    assert!(m(0.0) < 0.02);
    assert!(t.magnitude_percentile(AxisId::Contrast, "m", 1.0).is_none());
}
