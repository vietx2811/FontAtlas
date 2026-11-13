//! Lỗi của kho dữ liệu, kèm mã cấu trúc cho UI.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("lỗi SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("lỗi đọc/ghi: {0}")]
    Io(#[from] std::io::Error),
    #[error("{db} có schema v{found}, app chỉ hỗ trợ tới v{supported}")]
    NewerSchema { db: &'static str, found: u32, supported: u32 },
    #[error("migration {db} v{from}→v{to} lỗi: {source}")]
    Migration {
        db: &'static str,
        from: u32,
        to: u32,
        #[source]
        source: rusqlite::Error,
    },
    #[error("hash không hợp lệ: {0}")]
    BadHash(String),
    #[error("DB cũ không đọc được: {0}")]
    Legacy(String),
    #[error("không tìm thấy: {0}")]
    NotFound(String),
    #[error("tham số không hợp lệ: {0}")]
    Invalid(String),
    /// Trùng với bản ghi khác của người dùng (vd đổi tên Tag sang tên đã có): không gộp ngầm.
    #[error("trùng tên: {0}")]
    Conflict(String),
    #[error("đường dẫn lồng với nguồn #{existing_id}")]
    NestedSource { existing_id: i64 },
    /// Lỗi vai trong Combo, mã riêng cho giao diện: `role_name_invalid`, `role_duplicate`, `role_not_found`,
    /// `role_limit`, `role_set_mismatch`.
    #[error("{msg}")]
    Role { code: &'static str, msg: String },
}

impl StoreError {
    pub fn code(&self) -> &'static str {
        match self {
            StoreError::Sqlite(_) => "sqlite",
            StoreError::Io(_) => "io",
            StoreError::NewerSchema { .. } => "newer_schema",
            StoreError::Migration { .. } => "migration",
            StoreError::BadHash(_) => "bad_hash",
            StoreError::Legacy(_) => "legacy",
            StoreError::NotFound(_) => "not_found",
            StoreError::Invalid(_) => "invalid_arg",
            StoreError::Conflict(_) => "conflict",
            StoreError::NestedSource { .. } => "nested_source",
            StoreError::Role { code, .. } => code,
        }
    }
}
