use fontatlas_core::CoreError;

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("cỡ chữ không hợp lệ: {0}")]
    BadSize(u32),
    #[error("yêu cầu sai: {0}")]
    BadRequest(String),
    #[error("không tìm thấy face")]
    NotFound,
    #[error("file đã thay đổi sau khi quét")]
    StaleFile,
    #[error("yêu cầu thuộc thế hệ cũ")]
    StaleGeneration,
    #[error("font không có outline")]
    NoOutlines,
    #[error("không glyph nào vừa canvas")]
    NothingFits,
    #[error("face trong TTC không phục vụ được qua FontFace")]
    TtcFace,
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("cache: {0}")]
    Cache(#[from] rusqlite::Error),
    #[error("mã hoá PNG: {0}")]
    Encode(String),
    #[error("hết thời gian chờ tải font từ ổ cloud")]
    CloudTimeout,
    #[error("không tải được font từ ổ cloud: {0}")]
    CloudUnavailable(String),
}

impl PreviewError {
    pub fn code(&self) -> &'static str {
        match self {
            PreviewError::BadSize(_) => "bad_size",
            PreviewError::BadRequest(_) => "bad_request",
            PreviewError::NotFound => "not_found",
            PreviewError::StaleFile => "stale_file",
            PreviewError::StaleGeneration => "stale_generation",
            PreviewError::NoOutlines => "no_outlines",
            PreviewError::NothingFits => "nothing_fits",
            PreviewError::TtcFace => "ttc_face",
            PreviewError::Core(_) => "core",
            PreviewError::Cache(_) => "cache",
            PreviewError::Encode(_) => "encode",
            PreviewError::CloudTimeout => "cloud_timeout",
            PreviewError::CloudUnavailable(_) => "cloud_unavailable",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            PreviewError::BadSize(_) | PreviewError::BadRequest(_) => 400,
            PreviewError::NotFound => 404,
            PreviewError::StaleFile => 409,
            PreviewError::StaleGeneration => 410,
            PreviewError::NoOutlines | PreviewError::NothingFits | PreviewError::TtcFace => 422,
            PreviewError::Core(e) if e.code() == "io" => 404,
            PreviewError::Core(_) => 422,
            PreviewError::Cache(_) | PreviewError::Encode(_) => 500,
            PreviewError::CloudTimeout => 504,
            PreviewError::CloudUnavailable(_) => 502,
        }
    }
}
