//! Tiện ích dùng chung cho test tích hợp của app.

/// Dùng chung cho test: đường dẫn thật trên Windows dùng `\`; so sánh theo dạng có `/`.
#[allow(dead_code)]
pub fn nz(s: impl AsRef<str>) -> String {
    s.as_ref().replace('\\', "/")
}
