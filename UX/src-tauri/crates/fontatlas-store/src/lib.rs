//! fontatlas-store: library.db (dẫn xuất) + user.db (người dùng).
mod activation;
mod analysis;
mod embed;
mod error;
mod fold;
mod ingest;
mod legacy;
mod open;
mod pairs;
mod query;
mod rescan;
mod schema;
mod user;

pub use activation::{
    ActivationReason, ActivationRow, ActivationScope, ActivationStatus, DesiredState, FaceName, FileLocation, FileTarget,
    PrunedReasons,
    add_activation_reason_tx, put_activation_tx, remove_activation_reason_tx,
};
pub use analysis::StoredAnalysis;
pub use embed::{EmbedJob, EmbedReader, EnqueueStats, JobResult, SETTING_EMBED_ENABLED, SETTING_LICENSE_NOTICE_ACK};
pub use error::StoreError;
pub use fold::vi_fold;
pub use legacy::{LEGACY_DB_FILE, LegacyImport, LegacyPreview, LegacyResolve, legacy_preview};
pub use ingest::{Analysis, FaceKey, IngestItem, IngestSummary, ProblemFile, ScannedFile};
pub use rescan::{FONT_EXTENSIONS, INGEST_BATCH, RescanStats, RescanStatus, ScanPhase, ScanProgress, walk_fonts};
pub use query::{DEFAULT_LIMIT, FacePage, FaceQuery, FaceRow, MAX_LIMIT, SortKey, ViFilter};
pub use open::{BACKUP_DIR, LIBRARY_FILE, OpenReport, Store, USER_FILE};
pub use pairs::{NewPair, SavedPair};
pub use schema::{LIBRARY_VERSION, USER_VERSION, create_user_v1_for_test, create_user_v2_for_test};
pub use user::{Collection, Source, SourceKind, Tag};
