//! fontatlas-store: library.db (dẫn xuất) + user.db (người dùng).
mod error;
mod fold;
mod ingest;
mod open;
mod schema;

pub use error::StoreError;
pub use fold::vi_fold;
pub use ingest::{Analysis, FaceKey, IngestItem, IngestSummary, ProblemFile, ScannedFile};
pub use open::{BACKUP_DIR, LIBRARY_FILE, OpenReport, Store, USER_FILE};
pub use schema::{LIBRARY_VERSION, USER_VERSION};
