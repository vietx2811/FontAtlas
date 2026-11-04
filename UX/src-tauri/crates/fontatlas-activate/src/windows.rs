//! `WindowsGdiEngine`: stub. Biên dịch trên mọi nền tảng; mọi thao tác trả `Unsupported`.
//!
//! Phương án cho Plan 7b (nghiên cứu ở `docs/team/spikes/04_activation.md` §3, chưa chạy thật):
//!
//! - **Session:** `AddFontResourceExW(path, 0, None)` (KHÔNG `FR_PRIVATE` — cờ đó chỉ cho tiến trình gọi
//!   thấy), rồi `SendMessageTimeoutW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0, SMTO_ABORTIFHUNG, 2000)`.
//!   Gỡ: `RemoveFontResourceExW(path, 0, None)` + broadcast lại. Mất sau khi khởi động lại máy.
//! - **Persistent, per-user, không admin (Windows 10 1809+):** copy file vào
//!   `%LOCALAPPDATA%\Microsoft\Windows\Fonts\` (`SHGetKnownFolderPath(FOLDERID_LocalAppData)`), ghi REG_SZ
//!   `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts` — tên `"<Family> <Style> (TrueType)"`,
//!   dữ liệu là **đường dẫn đầy đủ** — rồi `AddFontResourceExW` + broadcast. Gỡ: `RemoveFontResourceExW`,
//!   xoá giá trị HKCU, broadcast, xoá file (file bị khoá khi đang nạp → hàng đợi dọn lần mở app sau).
//! - **Feature crate `windows` 0.62:** `Win32_Graphics_Gdi`, `Win32_UI_WindowsAndMessaging`,
//!   `Win32_System_Registry`, `Win32_UI_Shell`, `Win32_System_Com`, `Win32_Foundation`.
//! - **Câu hỏi mở:** giá trị HKCU trỏ thẳng vào file thư viện (không copy) có được nạp lại khi đăng nhập
//!   không; app Adobe có phản ứng với `WM_FONTCHANGE` khi đang mở không; GPO có thể chặn font per-user;
//!   `canonicalize` trả dạng `\\?\C:\…` — bỏ tiền tố trước khi gọi GDI hay không.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::engine::{ActivationEngine, Scope};
use crate::types::EngineError;

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsGdiEngine;

impl ActivationEngine for WindowsGdiEngine {
    fn method(&self) -> &'static str {
        "gdi"
    }
    fn register(&self, _path: &Path, _scope: Scope) -> Result<(), EngineError> {
        Err(EngineError::Unsupported)
    }
    fn unregister(&self, _path: &Path, _scope: Scope) -> Result<(), EngineError> {
        Err(EngineError::Unsupported)
    }
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> {
        Err(EngineError::Unsupported)
    }
    fn providers(&self, _postscript_name: &str) -> Result<Vec<PathBuf>, EngineError> {
        Err(EngineError::Unsupported)
    }
    fn postscript_names_in_file(&self, _path: &Path) -> Result<Vec<String>, EngineError> {
        Err(EngineError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_is_unsupported() {
        let e = WindowsGdiEngine;
        let p = Path::new("C:/x.ttf");
        assert_eq!(e.method(), "gdi");
        assert_eq!(e.register(p, Scope::Session), Err(EngineError::Unsupported));
        assert_eq!(e.unregister(p, Scope::Persistent), Err(EngineError::Unsupported));
        assert_eq!(e.available_postscript_names(), Err(EngineError::Unsupported));
        assert_eq!(e.providers("X"), Err(EngineError::Unsupported));
        assert_eq!(e.postscript_names_in_file(p), Err(EngineError::Unsupported));
    }
}
