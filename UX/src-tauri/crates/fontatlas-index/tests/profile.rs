mod common;
use common::*;
use fontatlas_index::{AxisId, FaceProfile, Q};

#[test]
fn profile_from_report_has_16_readings_in_order() {
    let p = profile("playfair_display.ttf");
    assert_eq!(p.readings.len(), 16);
    assert_eq!(p.get(AxisId::Contrast).method, "o_radial_thickness_max_min");
    assert!(p.get(AxisId::Contrast).value.unwrap() > 4.0);
    assert_eq!(p.family, "Playfair Display");
    assert!(p.open_license);
}

#[test]
fn stress_reading_is_estimated() {
    let p = profile("playfair_display.ttf");
    assert_eq!(p.get(AxisId::StressDeg).q, Q::Estimated);
}

#[test]
fn json_roundtrip_matches_report() {
    // Plan 2 lưu JSON nguyên văn của Metadata/Vietnamese/Anatomy; dựng lại phải cho cùng profile.
    let r = fontatlas_core::analyze_path(&fixture("roboto.ttf")).unwrap();
    let f = &r.faces[0];
    let a = FaceProfile::from_report(&r.hash_hex, f);
    let b = FaceProfile::from_json(
        &r.hash_hex,
        f.index,
        &serde_json::to_value(&f.metadata).unwrap(),
        &serde_json::to_value(&f.vietnamese).unwrap(),
        &serde_json::to_value(&f.anatomy).unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
}

#[test]
fn from_json_rejects_garbage() {
    let v = serde_json::json!({"x": 1});
    assert!(FaceProfile::from_json("00", 0, &v, &v, &v).is_none());
}

#[test]
fn vf_profile_has_wght_range() {
    let p = profile("public_sans_vf.ttf");
    let (lo, hi) = p.wght_range.unwrap();
    assert!(lo <= 400.0 && hi >= 700.0, "{lo}..{hi}");
    assert_eq!(
        profile("roboto.ttf").wght_range.is_some(),
        !fontatlas_core::analyze_path(&fixture("roboto.ttf"))
            .unwrap()
            .faces[0]
            .metadata
            .axes
            .is_empty()
    );
}

#[test]
fn profile_reads_terminal_but_not_in_similar_axes() {
    let p = profile("playfair_display.ttf");
    assert_eq!(p.terminal.q, Q::Estimated);
    assert_eq!(p.terminal.method, "c_upper_terminal_normal_over_thin");
    assert_eq!(p.readings.len(), 16, "terminal không vào 16 chiều Similar");
}
