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
