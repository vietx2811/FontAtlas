//! fontatlas-store: library.db (dẫn xuất) + user.db (người dùng).
mod activation;
mod analysis;
mod embed;
mod error;
mod fold;
mod ingest;
mod langs;
mod legacy;
mod open;
mod overrides;
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
pub use analysis::{DupRow, StoredAnalysis};
pub use embed::{EmbedJob, EmbedReader, EnqueueStats, JobResult, SETTING_EMBED_ENABLED, SETTING_LICENSE_NOTICE_ACK};
pub use error::StoreError;
pub use fold::vi_fold;
pub use legacy::{LEGACY_DB_FILE, LegacyImport, LegacyPreview, LegacyResolve, legacy_preview};
pub use ingest::{Analysis, FaceKey, IngestItem, IngestSummary, ProblemFile, ScannedFile};
pub use rescan::{FONT_EXTENSIONS, INGEST_BATCH, PlaceholderProbe, RescanStats, RescanStatus, ScanPhase, ScanProgress, os_placeholder, walk_fonts};
pub use query::{DEFAULT_LIMIT, FacePage, FaceQuery, FaceRow, LangFilter, LangHit, MAX_LIMIT, SortKey, ViFilter};
pub use langs::{LangCount, LangPending, LangProgress, LangRow};
pub use open::{BACKUP_DIR, LIBRARY_FILE, OpenReport, Store, USER_FILE};
pub use overrides::ClassificationOverride;
pub use pairs::{NewPair, SavedPair};
pub use schema::{LIBRARY_VERSION, USER_VERSION, create_library_v1_for_test, create_user_v1_for_test, create_user_v2_for_test, create_user_v3_for_test, create_user_v4_for_test};
pub use user::{Collection, Source, SourceKind, Tag};
