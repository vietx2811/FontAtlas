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
