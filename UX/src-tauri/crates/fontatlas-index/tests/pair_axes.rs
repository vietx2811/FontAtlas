mod common;
use common::*;
use fontatlas_index::{AxisId, PairAxis, Q, Relation, Role, Thresholds, label_axes};

#[test]
fn ln_ratio_bands() {
    let t = Thresholds::v1();
    let r = ref_aperture();
    let a = synth("A", &[(AxisId::Width, 1.00, Q::Measured, M_WIDTH)]);
    let rel = |w| {
        label_axes(
            &a,
            Role::Body,
            &synth("B", &[(AxisId::Width, w, Q::Measured, M_WIDTH)]),
            Role::Body,
            &t,
            &r,
        )
        .into_iter()
        .find(|l| l.axis == PairAxis::Width)
        .unwrap()
        .relation
    };
    assert_eq!(rel(1.05), Relation::Harmony); // ln 1.05 = 0.049 ≤ 0.10
    assert_eq!(rel(1.20), Relation::Clash); // 0.182 giữa 0.10 và 0.25
    assert_eq!(rel(1.40), Relation::Contrast); // 0.336 ≥ 0.25
}

#[test]
fn failed_axis_is_unsure_not_clash() {
    let a = synth("A", &[(AxisId::Contrast, 3.0, Q::Measured, M_CONTRAST)]);
    let b = synth("B", &[]); // contrast Failed
    let l = find(
        label_axes(
            &a,
            Role::Heading,
            &b,
            Role::Body,
            &Thresholds::v1(),
            &ref_aperture(),
        ),
        PairAxis::Contrast,
    );
    assert_eq!(l.relation, Relation::Unsure);
}

#[test]
fn estimated_both_sides_is_unsure() {
    let mk = |v| {
        synth(
            "X",
            &[
                (AxisId::StressDeg, v, Q::Estimated, M_STRESS),
                (AxisId::Contrast, 3.0, Q::Measured, M_CONTRAST),
            ],
        )
    };
    let l = find(
        label_axes(
            &mk(0.0),
            Role::Heading,
            &mk(40.0),
            Role::Body,
            &Thresholds::v1(),
            &ref_aperture(),
        ),
        PairAxis::Stress,
    );
    assert_eq!(l.relation, Relation::Unsure); // 0.6·0.6 = 0.36 < 0.5
}

#[test]
fn method_mismatch_is_unsure() {
    let a = synth(
        "A",
        &[(AxisId::StemNXh, 0.15, Q::Measured, "n_stem_at_35pct_xh")],
    );
    let b = synth(
        "B",
        &[(AxisId::StemNXh, 0.15, Q::Measured, "H_stem_over_cap")],
    );
    assert_eq!(
        find(
            label_axes(
                &a,
                Role::Heading,
                &b,
                Role::Body,
                &Thresholds::v1(),
                &ref_aperture()
            ),
            PairAxis::Weight
        )
        .relation,
        Relation::Unsure
    );
}

#[test]
fn weight_never_clash_and_reversal_flag() {
    let h = synth("H", &[(AxisId::StemNXh, 0.15, Q::Measured, M_STEM)]);
    let b = synth("B", &[(AxisId::StemNXh, 0.19, Q::Measured, M_STEM)]); // ln = 0.237
    let l = find(
        label_axes(
            &h,
            Role::Heading,
            &b,
            Role::Body,
            &Thresholds::v1(),
            &ref_aperture(),
        ),
        PairAxis::Weight,
    );
    assert_eq!(l.relation, Relation::WeightHint);
    assert!(l.role_reversed);
}

#[test]
fn x_height_only_judged_at_same_size() {
    let a = synth("A", &[(AxisId::XHeightEm, 0.50, Q::Measured, M_XH)]);
    let b = synth("B", &[(AxisId::XHeightEm, 0.44, Q::Measured, M_XH)]); // ln = 0.128 → vùng Clash
    let t = Thresholds::v1();
    let r = ref_aperture();
    assert_eq!(
        find(
            label_axes(&a, Role::Heading, &b, Role::Body, &t, &r),
            PairAxis::XHeight
        )
        .relation,
        Relation::Harmony
    );
    assert_eq!(
        find(
            label_axes(&a, Role::Body, &b, Role::Body, &t, &r),
            PairAxis::XHeight
        )
        .relation,
        Relation::Clash
    );
}

#[test]
fn contrast_clash_follows_serif_change() {
    let serif = synth(
        "S",
        &[
            (AxisId::SerifFoot, 2.4, Q::Measured, M_SERIF),
            (AxisId::Contrast, 2.4, Q::Measured, M_CONTRAST),
        ],
    );
    let sans = synth(
        "N",
        &[
            (AxisId::SerifFoot, 1.0, Q::Measured, M_SERIF),
            (AxisId::Contrast, 1.5, Q::Measured, M_CONTRAST),
        ],
    ); // ln = 0.47
    let ls = label_axes(
        &serif,
        Role::Heading,
        &sans,
        Role::Body,
        &Thresholds::v1(),
        &ref_aperture(),
    );
    assert_eq!(
        find(ls.clone(), PairAxis::Serif).relation,
        Relation::Contrast
    );
    assert_eq!(
        find(ls, PairAxis::Contrast).relation,
        Relation::ContrastWithSerif
    );
}

#[test]
fn nonpositive_or_nonfinite_is_unsure() {
    let t = Thresholds::v1();
    let r = ref_aperture();
    let a = synth("A", &[(AxisId::Width, 1.0, Q::Measured, M_WIDTH)]);
    for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let b = synth("B", &[(AxisId::Width, bad, Q::Measured, M_WIDTH)]);
        assert_eq!(
            find(
                label_axes(&a, Role::Body, &b, Role::Body, &t, &r),
                PairAxis::Width
            )
            .relation,
            Relation::Unsure,
            "{bad}"
        );
    }
}

#[test]
fn real_fixtures_label_without_panic() {
    let all = [
        "roboto.ttf",
        "open_sans.ttf",
        "playfair_display.ttf",
        "libre_baskerville.ttf",
        "source_code_pro.ttf",
        "public_sans_vf.ttf",
    ];
    for x in all {
        for y in all {
            let v = label_axes(
                &profile(x),
                Role::Heading,
                &profile(y),
                Role::Body,
                &Thresholds::v1(),
                &ref_aperture(),
            );
            assert_eq!(v.len(), 8);
        }
    }
}
