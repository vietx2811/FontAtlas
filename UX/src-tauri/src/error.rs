//! Lỗi lệnh gửi qua IPC: `{ code, message, detail }`.
//!
//! Mã `io` có hai nguồn, phân biệt bằng `detail.source`: `"store"` (`StoreError::Io`) và `"app"`
//! (I/O của chính app: embed, quét thư mục…).
use fontatlas_embed::EmbedError;
use fontatlas_index::IndexError;
use fontatlas_preview::PreviewError;
use fontatlas_store::StoreError;
use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CmdError {
    pub code: String,
    pub message: String,
    #[ts(type = "unknown | null")]
    pub detail: Option<Value>,
}

impl CmdError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        CmdError { code: code.into(), message: message.into(), detail: None }
    }

    pub fn with_detail(mut self, detail: Value) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn invalid_arg(message: impl Into<String>) -> Self {
        Self::new("invalid_arg", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    pub fn busy(message: impl Into<String>) -> Self {
        Self::new("busy", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    /// I/O của chính app (không phải của kho): `detail.source = "app"`.
    pub fn app_io(op: &str, e: &std::io::Error) -> Self {
        Self::new("io", e.to_string()).with_detail(json!({ "source": "app", "op": op }))
    }
}

impl std::fmt::Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CmdError {}

fn from_store(e: &StoreError) -> CmdError {
    let mut out = CmdError::new(e.code(), e.to_string());
    match e {
        // `add_source` bổ sung `existingPath` bằng cách tra nguồn này
        StoreError::NestedSource { existing_id } => out.detail = Some(json!({ "existingId": existing_id })),
        StoreError::Io(_) => out.detail = Some(json!({ "source": "store" })),
        _ => {}
    }
    out
}

impl From<StoreError> for CmdError {
    fn from(e: StoreError) -> Self {
        from_store(&e)
    }
}

impl From<IndexError> for CmdError {
    fn from(e: IndexError) -> Self {
        match e {
            IndexError::NotFound(_) => CmdError::not_found(e.to_string()),
            IndexError::NotComparable(_) => CmdError::new("not_comparable", e.to_string()),
            IndexError::Reference(_) => CmdError::internal(e.to_string()),
        }
    }
}

impl From<EmbedError> for CmdError {
    fn from(e: EmbedError) -> Self {
        match &e {
            EmbedError::Cancelled => CmdError::new("cancelled", e.to_string()),
            EmbedError::Io(io) => CmdError::app_io("embed", io),
            EmbedError::Store(s) => from_store(s),
            _ => CmdError::new(e.code(), e.to_string()),
        }
    }
}

impl From<PreviewError> for CmdError {
    fn from(e: PreviewError) -> Self {
        CmdError::new(e.code(), e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_source_carries_existing_id() {
        let e: CmdError = StoreError::NestedSource { existing_id: 7 }.into();
        assert_eq!(e.code, "nested_source");
        assert_eq!(e.detail, Some(json!({ "existingId": 7 })));
    }

    #[test]
    fn io_code_has_two_sources() {
        let store: CmdError = StoreError::Io(std::io::Error::other("x")).into();
        assert_eq!((store.code.as_str(), store.detail), ("io", Some(json!({ "source": "store" }))));
        let app = CmdError::app_io("scan", &std::io::Error::other("y"));
        assert_eq!(app.detail, Some(json!({ "source": "app", "op": "scan" })));
        let emb: CmdError = EmbedError::Io(std::io::Error::other("z")).into();
        assert_eq!(emb.detail.unwrap()["source"], "app");
    }

    #[test]
    fn codes_pass_through() {
        assert_eq!(CmdError::from(StoreError::NotFound("x".into())).code, "not_found");
        assert_eq!(CmdError::from(EmbedError::Cancelled).code, "cancelled");
        assert_eq!(CmdError::from(PreviewError::StaleGeneration).code, "stale_generation");
        assert_eq!(CmdError::from(IndexError::NotComparable(fontatlas_index::FaceKey { hash: "h".into(), index: 0 })).code, "not_comparable");
    }

    #[test]
    fn serializes_camel_case() {
        let v = serde_json::to_value(CmdError::invalid_arg("x")).unwrap();
        assert_eq!(v, json!({ "code": "invalid_arg", "message": "x", "detail": null }));
    }
}
