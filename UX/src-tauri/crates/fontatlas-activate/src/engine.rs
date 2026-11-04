//! Trait nói chuyện với hệ điều hành. Không biết gì về Store.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::types::EngineError;

pub use fontatlas_store::ActivationScope as Scope;

pub trait ActivationEngine: Send + Sync {
    /// "coretext" | "gdi" | "fake" — ghi vào `activation_state.method`.
    fn method(&self) -> &'static str;
    /// Đăng ký đúng chuỗi `path` được truyền vào (service đã canonicalize).
    fn register(&self, path: &Path, scope: Scope) -> Result<(), EngineError>;
    fn unregister(&self, path: &Path, scope: Scope) -> Result<(), EngineError>;
    /// Mọi PostScript name hệ thống đang cung cấp (gọi một lần mỗi lô).
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError>;
    /// Các file đang cung cấp PS name này (có thể > 1 khi trùng).
    fn providers(&self, postscript_name: &str) -> Result<Vec<PathBuf>, EngineError>;
    /// PS name đọc từ file, không đăng ký. Dự phòng khi library.db thiếu tên.
    fn postscript_names_in_file(&self, path: &Path) -> Result<Vec<String>, EngineError>;
}

/// Cho phép `ActivationService<Box<dyn ActivationEngine>>` (engine chọn lúc chạy).
impl<E: ActivationEngine + ?Sized> ActivationEngine for Box<E> {
    fn method(&self) -> &'static str {
        (**self).method()
    }
    fn register(&self, path: &Path, scope: Scope) -> Result<(), EngineError> {
        (**self).register(path, scope)
    }
    fn unregister(&self, path: &Path, scope: Scope) -> Result<(), EngineError> {
        (**self).unregister(path, scope)
    }
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> {
        (**self).available_postscript_names()
    }
    fn providers(&self, postscript_name: &str) -> Result<Vec<PathBuf>, EngineError> {
        (**self).providers(postscript_name)
    }
    fn postscript_names_in_file(&self, path: &Path) -> Result<Vec<String>, EngineError> {
        (**self).postscript_names_in_file(path)
    }
}
