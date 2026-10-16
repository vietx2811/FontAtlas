//! Lỗi cấp file, có mã cấu trúc để lưu DB và hiện UI.

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("không đọc được file: {0}")]
    Io(#[from] std::io::Error),
    #[error("không phải file font")]
    NotAFont,
    #[error("file bị cắt cụt: {0}")]
    Truncated(String),
    #[error("thiếu bảng {0}")]
    MissingTable(String),
    #[error("giải nén WOFF lỗi: {0}")]
    WoffDecode(String),
    #[error("lỗi parse: {0}")]
    Parse(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Io(_) => "io",
            CoreError::NotAFont => "not_a_font",
            CoreError::Truncated(_) => "truncated",
            CoreError::MissingTable(_) => "missing_table",
            CoreError::WoffDecode(_) => "woff_decode",
            CoreError::Parse(_) => "parse",
        }
    }
}
