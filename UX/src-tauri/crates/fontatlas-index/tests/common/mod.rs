#![allow(dead_code)]
use std::path::PathBuf;

use fontatlas_index::FaceProfile;

/// Fixture dùng chung với core.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fontatlas-core/tests/fixtures")
        .join(name)
}

/// Phân tích fixture, lấy face 0.
pub fn profile(name: &str) -> FaceProfile {
    let r = fontatlas_core::analyze_path(&fixture(name)).expect("fixture");
    FaceProfile::from_report(&r.hash_hex, &r.faces[0])
}

use fontatlas_index::{AxisId, AxisQuantiles, FaceKey, Q, Reading, ReferenceTable};

pub const M_WIDTH: &str = "o_bbox_width_over_xh";
pub const M_CONTRAST: &str = "o_radial_thickness_max_min";
pub const M_STRESS: &str = "o_harmonic_fit";
pub const M_STEM: &str = "n_stem_at_35pct_xh";
pub const M_XH: &str = "x_bbox_top";
pub const M_SERIF: &str = "baseline_foot_over_stem_median_iln";

/// Profile giả: mọi trục Failed (method mặc định), trừ các trục được cho.
pub fn synth(family: &str, axes: &[(AxisId, f32, Q, &str)]) -> FaceProfile {
    let mut readings: Vec<Reading> = AxisId::ALL
        .iter()
        .map(|a| Reading {
            value: None,
            q: Q::Failed,
            method: a.default_method().into(),
        })
        .collect();
    for (a, v, q, m) in axes {
        readings[a.index()] = Reading {
            value: Some(*v),
            q: *q,
            method: (*m).into(),
        };
    }
    FaceProfile {
        key: FaceKey {
            hash: format!("hash-{family}"),
            index: 0,
        },
        family: family.into(),
        style: "Regular".into(),
        designer: None,
        manufacturer: None,
        weight_class: Some(400),
        wght_range: None,
        named_instances: Vec::new(),
        open_license: true,
        vi_missing: 0,
        vi_overflow: None,
        readings,
        terminal: Reading {
            value: None,
            q: Q::Failed,
            method: "c_upper_terminal_normal_over_thin".into(),
        },
        embedding: None,
    }
}

/// Reference giả: aperture/c_angular_opening tuyến tính 0..180 độ.
pub fn ref_aperture() -> ReferenceTable {
    ReferenceTable {
        version: "ref-test".into(),
        source: "test".into(),
        analyzer_version: fontatlas_core::ANALYZER_VERSION.into(),
        faces: 500,
        quantiles: vec![AxisQuantiles {
            axis: "aperture".into(),
            method: "c_angular_opening".into(),
            n: 500,
            q: (0..=100).map(|i| i as f32 * 1.8).collect(),
        }],
        d_fam: 0.476,
        d_fam_pairs: 294,
    }
}

pub fn find(
    v: Vec<fontatlas_index::AxisLabel>,
    a: fontatlas_index::PairAxis,
) -> fontatlas_index::AxisLabel {
    v.into_iter().find(|l| l.axis == a).expect("đủ 8 nhãn")
}

use fontatlas_index::{FamilyFacts, Gate, PairAssessment, PairAxis, Relation};

pub const FACTS_OK: FamilyFacts = FamilyFacts {
    has_regular: true,
    has_bold: true,
};

/// Reference giả: q tuyến tính hợp lý cho mọi trục (method mặc định), d_fam = 0.476.
pub fn test_reference() -> ReferenceTable {
    let range = |a: AxisId| -> (f32, f32) {
        match a {
            AxisId::XHeightEm => (0.35, 0.60),
            AxisId::XHeightCap => (0.55, 0.80),
            AxisId::AscenderEm => (0.60, 0.85),
            AxisId::StemNXh => (0.03, 0.50),
            AxisId::Darkness => (0.10, 0.80),
            AxisId::Contrast => (1.0, 8.0),
            AxisId::StressDeg => (-30.0, 30.0),
            AxisId::Aperture => (0.0, 180.0),
            AxisId::ApertureE => (0.0, 0.6),
            AxisId::Width => (0.6, 1.3),
            AxisId::MeanAdvance => (0.7, 1.4),
            AxisId::WidthCv => (0.0, 0.4),
            AxisId::Spacing => (0.05, 0.35),
            AxisId::SlantDeg => (-25.0, 5.0),
            AxisId::SerifFoot => (0.5, 3.5),
            AxisId::Counter => (0.05, 0.7),
        }
    };
    ReferenceTable {
        version: "ref-test".into(),
        source: "test".into(),
        analyzer_version: fontatlas_core::ANALYZER_VERSION.into(),
        faces: 500,
        quantiles: AxisId::ALL
            .iter()
            .map(|a| {
                let (lo, hi) = range(*a);
                AxisQuantiles {
                    axis: a.key().into(),
                    method: a.default_method().into(),
                    n: 500,
                    q: (0..=100)
                        .map(|i| lo + (hi - lo) * i as f32 / 100.0)
                        .collect(),
                }
            })
            .collect(),
        d_fam: 0.476,
        d_fam_pairs: 294,
    }
}

/// Giá trị ở phân vị u (0–1) của trục trong test_reference.
pub fn value_in_reference_range(a: AxisId, u: f32) -> f32 {
    let r = test_reference();
    let q = &r.quantiles[a.index()].q;
    q[0] + (q[100] - q[0]) * u
}

/// Đủ 16 trục với giá trị trung tính; stress và aperture_e Estimated như core.
pub fn synth_full(family: &str, style: &str, stem: f32) -> FaceProfile {
    let vals: [(AxisId, f32); 16] = [
        (AxisId::XHeightEm, 0.5),
        (AxisId::XHeightCap, 0.7),
        (AxisId::AscenderEm, 0.75),
        (AxisId::StemNXh, stem),
        (AxisId::Darkness, 0.35),
        (AxisId::Contrast, 1.2),
        (AxisId::StressDeg, 0.0),
        (AxisId::Aperture, 100.0),
        (AxisId::ApertureE, 0.3),
        (AxisId::Width, 1.0),
        (AxisId::MeanAdvance, 1.0),
        (AxisId::WidthCv, 0.28),
        (AxisId::Spacing, 0.15),
        (AxisId::SlantDeg, 0.0),
        (AxisId::SerifFoot, 1.0),
        (AxisId::Counter, 0.5),
    ];
    let axes: Vec<(AxisId, f32, Q, &str)> = vals
        .iter()
        .map(|(a, v)| {
            let q = if matches!(a, AxisId::StressDeg | AxisId::ApertureE) {
                Q::Estimated
            } else {
                Q::Measured
            };
            (*a, *v, q, a.default_method())
        })
        .collect();
    let mut p = synth(family, &axes);
    p.style = style.into();
    p.key = key_of(family, style);
    p
}

pub fn synth_with(family: &str, over: &[(AxisId, f32)]) -> FaceProfile {
    let mut p = synth_full(family, "Regular", 0.15);
    for (a, v) in over {
        set_value(&mut p, *a, *v);
    }
    p
}

pub fn set_value(p: &mut FaceProfile, a: AxisId, v: f32) {
    p.readings[a.index()].value = Some(v);
}

pub fn set_q(p: &mut FaceProfile, a: AxisId, q: Q) {
    p.readings[a.index()].q = q;
}

pub fn key_of(family: &str, style: &str) -> FaceKey {
    FaceKey {
        hash: format!("{family}/{style}"),
        index: 0,
    }
}

/// Đánh giá rỗng: 8 nhãn Unsure, không clash, cổng Thử nghiệm.
pub fn blank_assessment() -> PairAssessment {
    PairAssessment {
        labels: PairAxis::ALL
            .iter()
            .map(|a| fontatlas_index::AxisLabel {
                axis: *a,
                relation: Relation::Unsure,
                delta: None,
                a: None,
                b: None,
                role_reversed: false,
            })
            .collect(),
        clashes: Vec::new(),
        related: false,
        same_designer: false,
        global_distance: None,
        gate: Gate::Experimental,
        block: None,
        body_warnings: Vec::new(),
        heading_overflow: false,
        insufficient_data: false,
        weight_hint: None,
    }
}

pub fn set_label(x: &mut PairAssessment, a: PairAxis, r: Relation) {
    if let Some(l) = x.labels.iter_mut().find(|l| l.axis == a) {
        l.relation = r;
    }
}

use fontatlas_index::{Index, SuperfamilyTable};

/// Index với reference giả + bảng superfamily phát hành kèm.
pub fn index_of(profiles: Vec<FaceProfile>) -> Index {
    Index::new(profiles, test_reference(), SuperfamilyTable::bundled())
}

/// 50.000 profile giả, mỗi trục rải đều trong khoảng của reference (LCG cố định).
pub fn synthetic_50k(seed: u64) -> Vec<FaceProfile> {
    let mut s = seed;
    let mut next = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        (s >> 33) as f32 / (1u64 << 31) as f32
    };
    (0..50_000)
        .map(|i| {
            let mut p = synth_full(&format!("F{i}"), "Regular", 0.15);
            for a in AxisId::ALL {
                set_value(&mut p, a, value_in_reference_range(a, next()));
            }
            p
        })
        .collect()
}

/// 8 nhãn Harmony (có số đo), cổng An toàn.
pub fn all_harmony_assessment() -> PairAssessment {
    let mut x = blank_assessment();
    for ax in PairAxis::ALL {
        set_label_values(&mut x, ax, Relation::Harmony, 0.5, 0.5);
    }
    x.gate = Gate::Safe;
    x
}

pub fn set_label_values(x: &mut PairAssessment, a: PairAxis, r: Relation, va: f32, vb: f32) {
    if let Some(l) = x.labels.iter_mut().find(|l| l.axis == a) {
        l.relation = r;
        l.a = Some(va);
        l.b = Some(vb);
    }
}

/// Tổ hợp (trục, quan hệ) hợp lệ: Weight không có Clash; WeightHint chỉ cho Weight;
/// ContrastWithSerif chỉ cho Contrast.
pub fn valid_combo(a: PairAxis, r: Relation) -> bool {
    match r {
        Relation::WeightHint => a == PairAxis::Weight,
        Relation::ContrastWithSerif => a == PairAxis::Contrast,
        Relation::Clash => a != PairAxis::Weight,
        _ => true,
    }
}
