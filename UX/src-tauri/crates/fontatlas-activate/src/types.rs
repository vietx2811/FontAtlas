//! Lỗi và kiểu dùng chung.
use serde::Serialize;

use fontatlas_store::StoreError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    #[error("file đã được đăng ký ở scope này")]
    AlreadyRegistered,
    #[error("file chưa được đăng ký ở scope này")]
    NotRegistered,
    #[error("nền tảng này chưa hỗ trợ kích hoạt font")]
    Unsupported,
    #[error("hệ điều hành từ chối ({code}): {message}")]
    Rejected { code: i64, message: String },
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::AlreadyRegistered => "already_registered",
            EngineError::NotRegistered => "not_registered",
            EngineError::Unsupported => "unsupported",
            EngineError::Rejected { .. } => "rejected",
        }
    }
}

/// Ai có thể đang cung cấp một font trùng tên — suy đoán theo đường dẫn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    System,
    UserInstalled,
    AdobeFonts,
    RightFont,
    FontBase,
    FontAtlas,
    Other,
}

#[derive(Debug, thiserror::Error)]
pub enum ActivationError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Engine(EngineError),
    #[error("nền tảng này chưa hỗ trợ kích hoạt font")]
    Unsupported,
    #[error("không tìm thấy: {0}")]
    NotFound(String),
    #[error("hash không hợp lệ: {0}")]
    BadHash(String),
}

impl ActivationError {
    pub fn code(&self) -> &'static str {
        match self {
            ActivationError::Store(_) => "store",
            ActivationError::Engine(_) => "engine",
            ActivationError::Unsupported => "unsupported",
            ActivationError::NotFound(_) => "not_found",
            ActivationError::BadHash(_) => "bad_hash",
        }
    }
}
