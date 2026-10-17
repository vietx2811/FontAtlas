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
