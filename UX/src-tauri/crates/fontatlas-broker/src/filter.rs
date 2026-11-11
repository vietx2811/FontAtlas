//! Tên font KHÔNG BAO GIỜ có trong thư viện của người dùng: font nội bộ của app thiết kế và font hệ thống ẩn.
//! Lọc ở lõi (`ensure_in_library`) để mọi đường vào (HTTP, nguồn font request) cùng một luật, và để tên rác không vào
//! nhật ký hay chạm DB.

/// - Adobe: `AdobeSansMM`, `AdobeSerifMM` (font thay thế nội bộ), `AdobeInvis*`, `AdobeBlank*`.
/// - Tên bắt đầu bằng `.` là font hệ thống ẩn (`.SFNS-Regular`, `.AppleSystemUIFont`); `LastResort`.
/// - Rỗng, có ký tự điều khiển, hoặc dài bất thường.
pub fn is_internal_name(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n.starts_with('.') || n.chars().any(char::is_control) || n.len() > 512 {
        return true;
    }
    let l = n.to_ascii_lowercase();
    l.starts_with("adobesansmm") || l.starts_with("adobeserifmm") || l.starts_with("adobeinvis") || l.starts_with("adobeblank") || l == "lastresort"
}
