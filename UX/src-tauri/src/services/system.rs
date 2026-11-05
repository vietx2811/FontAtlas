//! Việc của hệ thống: nhận diện trình quản lý font đã cài.
use std::path::{Path, PathBuf};

use crate::dto::settings::FontManagerDto;

/// (tên hiển thị, tên bundle trong `/Applications`)
const MANAGERS: [(&str, &str); 5] = [
    ("RightFont", "RightFont.app"),
    ("FontBase", "FontBase.app"),
    ("Typeface", "Typeface.app"),
    ("Suitcase Fusion", "Suitcase Fusion.app"),
    ("FontExplorer X", "FontExplorer X Pro.app"),
];

/// Chỉ xem có cài không (`/Applications`, `~/Applications`); `activeCount` do Plan 7 cung cấp sau.
pub fn detect_font_managers_in(roots: &[PathBuf]) -> Vec<FontManagerDto> {
    MANAGERS
        .iter()
        .map(|(name, bundle)| FontManagerDto {
            name: (*name).into(),
            installed: roots.iter().any(|r| r.join(bundle).exists()),
            active_count: None,
        })
        .collect()
}

pub fn detect_font_managers() -> Vec<FontManagerDto> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(Path::new(&home).join("Applications"));
    }
    detect_font_managers_in(&roots)
}
