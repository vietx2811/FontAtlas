#![allow(dead_code)]
use std::path::{Path, PathBuf};

pub const SAMPLES: [&str; 5] = [
    "roboto.ttf",
    "open_sans.ttf",
    "playfair_display.ttf",
    "libre_baskerville.ttf",
    "source_code_pro.ttf",
];

/// Fixture dùng chung với fontatlas-core.
pub fn core_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fontatlas-core/tests/fixtures")
        .join(name)
}

pub fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// (độ dài, mtime, BLAKE3 nội dung) để chứng minh một file không bị ghi; None nếu chưa tồn tại.
pub fn fingerprint(p: &Path) -> Option<(u64, std::time::SystemTime, String)> {
    let m = std::fs::metadata(p).ok()?;
    let h = blake3::hash(&std::fs::read(p).ok()?).to_hex().to_string();
    Some((m.len(), m.modified().ok()?, h))
}
