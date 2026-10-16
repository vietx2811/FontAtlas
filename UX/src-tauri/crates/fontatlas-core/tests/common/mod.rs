#![allow(dead_code)]
use std::path::PathBuf;

pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

pub fn fixture_bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture(name)).expect("fixture")
}

/// Font hệ thống macOS cho test có điều kiện; None nếu không có (CI Linux/Windows).
pub fn system_font(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

pub const SAMPLES: [&str; 5] = [
    "roboto.ttf",
    "open_sans.ttf",
    "playfair_display.ttf",
    "libre_baskerville.ttf",
    "source_code_pro.ttf",
];

/// Đổi tag một bảng trong table directory (giữ thứ tự sort: "head" → "heae").
pub fn rename_table(mut d: Vec<u8>, from: &[u8; 4], to: &[u8; 4]) -> Vec<u8> {
    let n = u16::from_be_bytes([d[4], d[5]]) as usize;
    for i in 0..n {
        let p = 12 + 16 * i;
        if &d[p..p + 4] == from {
            d[p..p + 4].copy_from_slice(to);
        }
    }
    d
}
