//! Đường dẫn: canonical, trạng thái file (thiếu / iCloud chưa tải), đoán chủ sở hữu font trùng tên.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::types::Owner;

/// Trạng thái file trước khi đăng ký. Không bao giờ đọc nội dung (tránh kích hoạt tải iCloud ngầm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileProbe {
    Ready,
    Missing,
    NotDownloaded,
}

/// Chuỗi đưa cho hệ điều hành. Trên Windows có dạng `\\?\C:\…`; lưu nguyên văn (Plan 7b quyết).
pub fn canonical_path(p: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(p)
}

/// Cờ `SF_DATALESS` của APFS: file chỉ còn placeholder, nội dung nằm trên iCloud.
#[cfg(target_os = "macos")]
const SF_DATALESS: u32 = 0x4000_0000;

/// Hai file có cùng nội dung không? So kích thước trước (rẻ), chỉ khi bằng nhau mới đọc và so từng khối.
/// Lỗi đọc (không có file, không quyền) → false: coi là khác, để người dùng quyết định.
pub fn same_content(a: &Path, b: &Path) -> bool {
    use std::io::Read;
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else { return false };
    if ma.len() != mb.len() {
        return false;
    }
    let (Ok(mut fa), Ok(mut fb)) = (std::fs::File::open(a), std::fs::File::open(b)) else { return false };
    let (mut x, mut y) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    let mut remaining = ma.len();
    while remaining > 0 {
        let n = remaining.min(x.len() as u64) as usize;
        if fa.read_exact(&mut x[..n]).is_err() || fb.read_exact(&mut y[..n]).is_err() || x[..n] != y[..n] {
            return false;
        }
        remaining -= n as u64;
    }
    true
}

pub fn probe_file(p: &Path) -> FileProbe {
    match std::fs::metadata(p) {
        Ok(_m) => {
            #[cfg(target_os = "macos")]
            {
                use std::os::macos::fs::MetadataExt;
                if _m.st_flags() & SF_DATALESS != 0 {
                    return FileProbe::NotDownloaded;
                }
            }
            FileProbe::Ready
        }
        Err(_) if icloud_placeholder(p).is_some_and(|ph| ph.exists()) => FileProbe::NotDownloaded,
        Err(_) => FileProbe::Missing,
    }
}

/// `dir/x.ttf` → `dir/.x.ttf.icloud` (kiểu placeholder cũ của iCloud Drive).
fn icloud_placeholder(p: &Path) -> Option<PathBuf> {
    let name = p.file_name()?.to_str()?;
    Some(p.with_file_name(format!(".{name}.icloud")))
}

/// Đoán ai đang cung cấp font ở `path`. Khớp đầu tiên thắng; UI ghi "có thể là …".
pub fn classify_owner(path: &Path, home: &Path, ours: &HashSet<PathBuf>) -> Owner {
    if ours.contains(path) {
        return Owner::FontAtlas;
    }
    if path.starts_with("/System/Library") || path.starts_with("/Library/Fonts") {
        return Owner::System;
    }
    if path.starts_with(home.join("Library/Fonts")) {
        return Owner::UserInstalled;
    }
    let comps: Vec<String> = path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    if comps.windows(4).any(|w| w[0] == "Adobe" && w[1] == "CoreSync" && w[2] == "plugins" && w[3] == "livetype") {
        return Owner::AdobeFonts;
    }
    if comps.iter().any(|c| c.ends_with(".rightfontlibrary")) {
        return Owner::RightFont;
    }
    if comps.iter().any(|c| c.eq_ignore_ascii_case("FontBase")) {
        return Owner::FontBase;
    }
    Owner::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_owner_table() {
        let home = Path::new("/Users/u");
        let ours: HashSet<PathBuf> = [PathBuf::from("/lib/mine.ttf")].into();
        let cases = [
            ("/lib/mine.ttf", Owner::FontAtlas),
            ("/System/Library/Fonts/Helvetica.ttc", Owner::System),
            ("/Library/Fonts/Arial.ttf", Owner::System),
            ("/Users/u/Library/Fonts/Inter.ttf", Owner::UserInstalled),
            ("/Users/u/Library/Application Support/Adobe/CoreSync/plugins/livetype/.r/.123.otf", Owner::AdobeFonts),
            (
                "/Users/u/Library/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/A/x.ttf",
                Owner::RightFont,
            ),
            ("/Users/u/fontbase/Sans/x.ttf", Owner::FontBase),
            ("/Volumes/Ext/fonts/x.ttf", Owner::Other),
            ("/Users/u/Library/FontsOld/x.ttf", Owner::Other),
        ];
        for (p, want) in cases {
            assert_eq!(classify_owner(Path::new(p), home, &ours), want, "{p}");
        }
    }

    #[test]
    fn probe_missing_and_ready() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("a.ttf");
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(probe_file(&f), FileProbe::Ready);
        assert_eq!(probe_file(&d.path().join("khong-co.ttf")), FileProbe::Missing);
    }

    #[test]
    fn probe_icloud_placeholder() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".x.ttf.icloud"), b"").unwrap();
        assert_eq!(probe_file(&d.path().join("x.ttf")), FileProbe::NotDownloaded);
    }

    #[cfg(unix)]
    #[test]
    fn canonical_resolves_symlink() {
        let d = tempfile::tempdir().unwrap();
        let target = d.path().join("real.ttf");
        std::fs::write(&target, b"x").unwrap();
        let link = d.path().join("link.ttf");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(canonical_path(&link).unwrap(), std::fs::canonicalize(&target).unwrap());
    }
}
