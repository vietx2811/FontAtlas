//! fontatlas-store: library.db (dẫn xuất) + user.db (người dùng).
mod embed;
mod error;
mod fold;
mod ingest;
mod legacy;
mod open;
mod query;
mod rescan;
mod schema;
mod user;

pub use embed::{EmbedJob, EmbedReader, EnqueueStats, JobResult, SETTING_EMBED_ENABLED, SETTING_LICENSE_NOTICE_ACK};
pub use error::StoreError;
pub use fold::vi_fold;
pub use legacy::{LEGACY_DB_FILE, LegacyImport, LegacyResolve};
pub use ingest::{Analysis, FaceKey, IngestItem, IngestSummary, ProblemFile, ScannedFile};
pub use rescan::{FONT_EXTENSIONS, INGEST_BATCH, RescanStats, RescanStatus, walk_fonts};
pub use query::{DEFAULT_LIMIT, FacePage, FaceQuery, FaceRow, MAX_LIMIT, SortKey, ViFilter};
pub use open::{BACKUP_DIR, LIBRARY_FILE, OpenReport, Store, USER_FILE};
pub use schema::{LIBRARY_VERSION, USER_VERSION};
pub use user::{Collection, Source, SourceKind, Tag};
