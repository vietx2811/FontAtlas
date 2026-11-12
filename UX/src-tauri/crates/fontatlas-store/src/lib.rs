//! fontatlas-store: library.db (dẫn xuất) + user.db (người dùng).
mod activation;
mod auto;
mod cases;
mod cloud;
mod analysis;
mod dup_resolve;
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
mod release;
mod rescan;
mod schema;
mod user;

pub use activation::{
    ActivationReason, ActivationRow, ActivationScope, ActivationStatus, DesiredState, FaceName, FileLocation, FileTarget, NewReplaced, ReplacedRow,
    PrunedReasons,
    add_activation_reason_tx, put_activation_tx, remove_activation_reason_tx,
};
pub use auto::{AutoApp, AutoLogEntry, AutoLogRow, AUTO_LOG_DEDUPE_MS, AUTO_LOG_KEEP};
pub use analysis::{DupRow, StoredAnalysis};
pub use cases::{CaseFaceRow, CasePatch, CaseRow, CaseSummary, ComboPatch, ComboRow, MAX_CASE_FACES, MAX_COMBOS, MAX_ROLES_PER_COMBO, MAX_ROLE_CHARS, NewCombo, PeekOutcome, ROLES, SlotRow, TRASH_DAYS};
pub use cloud::{CloudProvider, cloud_provider_of};
pub use dup_resolve::{MemberInfo, NewResolution, ResolutionRow};
pub use embed::{EmbedJob, EmbedReader, EnqueueStats, JobResult, SETTING_EMBED_ENABLED, SETTING_LICENSE_NOTICE_ACK};
pub use error::StoreError;
pub use fold::vi_fold;
pub use legacy::{LEGACY_DB_FILE, LegacyImport, LegacyPreview, LegacyResolve, legacy_preview};
pub use ingest::{Analysis, FaceKey, IngestItem, IngestSummary, ProblemFile, ScannedFile};
pub use rescan::{FONT_EXTENSIONS, INGEST_BATCH, PlaceholderProbe, RescanStats, RescanStatus, ScanPhase, ScanProgress, is_dataless, os_placeholder, walk_fonts};
pub use rescan::SourceRemovalReport;
pub use query::{ClassAxis, ClassHistogram, DEFAULT_LIMIT, FacePage, FaceQuery, FaceRow, FamilyGroup, LangFilter, LangHit, MAX_LIMIT, SortKey, ViFilter, WIDTH_PCT, width_pct_span};
pub use langs::{LangCount, LangPending, LangProgress, LangRow};
pub use open::{BACKUP_DIR, LIBRARY_FILE, OpenReport, Store, USER_FILE};
pub use overrides::ClassificationOverride;
pub use pairs::{NewPair, SavedPair};
pub use schema::{LIBRARY_VERSION, USER_VERSION, create_library_v1_for_test, create_library_v2_for_test, create_library_v3_for_test, create_library_v4_for_test, create_user_v1_for_test, create_user_v2_for_test, create_user_v3_for_test, create_user_v4_for_test, create_user_v5_for_test, create_user_pre_taxonomy_for_test, create_user_pre_replaced_for_test, create_user_pre_auto_for_test, create_user_pre_combo_roles_for_test};
pub use user::{Collection, Source, SourceKind, Tag};
