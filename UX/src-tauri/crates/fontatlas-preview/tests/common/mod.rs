#![allow(dead_code)]
use std::path::PathBuf;

/// Fixture dùng chung với core (không chép).
pub fn core_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures").join(name)
}
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}
pub fn file(p: &std::path::Path) -> fontatlas_core::FontFile {
    fontatlas_core::FontFile::open(p).unwrap()
}
pub fn rename_table(mut d: Vec<u8>, from: &[u8; 4], to: &[u8; 4]) -> Vec<u8> {
    let n = u16::from_be_bytes([d[4], d[5]]) as usize;
    for i in 0..n {
        let p = 12 + 16 * i;
        if &d[p..p + 4] == from { d[p..p + 4].copy_from_slice(to); }
    }
    d
}
