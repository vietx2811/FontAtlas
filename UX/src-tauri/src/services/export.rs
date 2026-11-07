//! Lưu ảnh PNG do UI vẽ (Compare export). Rust tự mở hộp thoại lưu; JS không có quyền ghi đĩa hay mở rộng dialog.
use std::path::{Path, PathBuf};

use base64::Engine;

use crate::error::CmdError;

/// Giới hạn sau khi giải mã.
pub const MAX_PNG_BYTES: usize = 32 * 1024 * 1024;
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// Giải base64, kiểm kích thước và magic PNG.
pub fn decode_png(png_base64: &str) -> Result<Vec<u8>, CmdError> {
    // chặn sớm trước khi cấp phát: 32 MB nhị phân ≈ 42,7 MB base64
    if png_base64.len() > MAX_PNG_BYTES / 3 * 4 + 8 {
        return Err(CmdError::invalid_arg("ảnh PNG quá lớn (tối đa 32 MB)"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|e| CmdError::invalid_arg(format!("base64 không hợp lệ: {e}")))?;
    if bytes.len() > MAX_PNG_BYTES {
        return Err(CmdError::invalid_arg("ảnh PNG quá lớn (tối đa 32 MB)"));
    }
    if bytes.len() < PNG_MAGIC.len() || bytes[..8] != PNG_MAGIC {
        return Err(CmdError::invalid_arg("không phải tệp PNG (sai chữ ký)"));
    }
    Ok(bytes)
}

/// Tên gợi ý an toàn: bỏ đường dẫn/ký tự điều khiển, luôn đuôi `.png`.
pub fn safe_name(suggested: &str) -> String {
    let base = suggested.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base.chars().filter(|c| !c.is_control() && !matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect();
    let stem = cleaned.trim().trim_end_matches(".png").trim_end_matches(".PNG").trim().trim_matches('.');
    format!("{}.png", if stem.is_empty() { "compare" } else { stem })
}

/// Ép đuôi `.png` cho đường dẫn người dùng chọn (không phân biệt hoa thường).
pub fn force_png_ext(p: &Path) -> PathBuf {
    if p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("png")) {
        p.to_path_buf()
    } else {
        let mut s = p.as_os_str().to_owned();
        s.push(".png");
        PathBuf::from(s)
    }
}

/// `pick(tên gợi ý)` mở hộp thoại lưu và trả đường dẫn hoặc `None` khi huỷ.
pub fn save_png_with(suggested_name: &str, png_base64: &str, pick: &dyn Fn(&str) -> Option<PathBuf>) -> Result<Option<String>, CmdError> {
    let bytes = decode_png(png_base64)?;
    let Some(path) = pick(&safe_name(suggested_name)) else { return Ok(None) };
    let forced = force_png_ext(&path);
    if forced == path {
        // đường dẫn người dùng chọn đã được hộp thoại xác nhận ghi đè
        std::fs::write(&forced, &bytes).map_err(|e| CmdError::app_io("ghi PNG", &e))?;
    } else {
        // ta tự thêm đuôi nên hộp thoại chưa xác nhận tệp này: không bao giờ ghi đè
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&forced).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                CmdError::new("exists", format!("{} đã tồn tại; chọn tên khác", forced.display()))
            } else {
                CmdError::app_io("ghi PNG", &e)
            }
        })?;
        f.write_all(&bytes).map_err(|e| CmdError::app_io("ghi PNG", &e))?;
    }
    Ok(Some(forced.to_string_lossy().into_owned()))
}
