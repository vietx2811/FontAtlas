//! Chuyển một tệp vào Thùng rác hệ điều hành (KHÔNG BAO GIỜ xoá vĩnh viễn) và biết nó nằm đâu để hoàn tác.
//! macOS: `NSFileManager.trashItemAtURL:resultingItemURL:error:` trả về URL mới trong Thùng rác.
use std::path::{Path, PathBuf};

/// Trả đường dẫn mới của tệp trong Thùng rác.
#[cfg(target_os = "macos")]
pub fn trash_item(path: &Path) -> Result<PathBuf, String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let p = path.to_str().ok_or("đường dẫn không phải UTF-8")?;
    if !path.is_file() {
        return Err(format!("không có tệp: {p}"));
    }
    let fm = NSFileManager::defaultManager();
    let url = NSURL::fileURLWithPath(&NSString::from_str(p));
    let mut out = None;
    fm.trashItemAtURL_resultingItemURL_error(&url, Some(&mut out))
        .map_err(|e| e.localizedDescription().to_string())?;
    let dest = out.and_then(|u| u.path()).map(|s| s.to_string()).ok_or("hệ thống không trả đường dẫn trong Thùng rác")?;
    Ok(PathBuf::from(dest))
}

#[cfg(not(target_os = "macos"))]
pub fn trash_item(_path: &Path) -> Result<PathBuf, String> {
    Err("chuyển vào Thùng rác chỉ hỗ trợ macOS".into())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    /// Tệp tạm của chính test: vào Thùng rác rồi đưa về chỗ cũ, không để lại gì.
    #[test]
    fn trashing_returns_the_trash_location_and_the_file_can_be_put_back() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("fontatlas-trash-test.txt");
        std::fs::write(&f, b"x").unwrap();
        let t = trash_item(&f).unwrap();
        assert!(!f.exists());
        assert!(t.exists(), "tệp nằm trong Thùng rác: {t:?}");
        std::fs::rename(&t, &f).unwrap();
        assert!(f.exists());
    }

    #[test]
    fn trashing_a_missing_file_fails_without_side_effects() {
        let d = tempfile::tempdir().unwrap();
        assert!(trash_item(&d.path().join("khong-co.ttf")).is_err());
    }
}
