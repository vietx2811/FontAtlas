//! fontatlas-store: library.db (dẫn xuất) + user.db (người dùng).
mod error;
mod fold;
mod ingest;
mod open;
mod query;
mod rescan;
mod schema;
mod user;

pub use error::StoreError;
pub use fold::vi_fold;
pub use ingest::{Analysis, FaceKey, IngestItem, IngestSummary, ProblemFile, ScannedFile};
pub use rescan::{FONT_EXTENSIONS, INGEST_BATCH, RescanStats, RescanStatus, walk_fonts};
pub use query::{DEFAULT_LIMIT, FacePage, FaceQuery, FaceRow, MAX_LIMIT, SortKey, ViFilter};
pub use open::{BACKUP_DIR, LIBRARY_FILE, OpenReport, Store, USER_FILE};
pub use schema::{LIBRARY_VERSION, USER_VERSION};
pub use user::{Collection, Source, SourceKind, Tag};
