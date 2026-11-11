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
    /// Đoán theo đường dẫn có thành phần `Typeface`.
    Typeface,
    /// Extensis Connect / Suitcase Fusion — đoán theo đường dẫn.
    Suitcase,
    FontAtlas,
    Other,
}

impl Owner {
    /// Chuỗi snake_case ổn định (giống serde), ghi vào sổ khôi phục.
    pub fn as_str(&self) -> &'static str {
        match self {
            Owner::System => "system",
            Owner::UserInstalled => "user_installed",
            Owner::AdobeFonts => "adobe_fonts",
            Owner::RightFont => "right_font",
            Owner::FontBase => "font_base",
            Owner::Typeface => "typeface",
            Owner::Suitcase => "suitcase",
            Owner::FontAtlas => "font_atlas",
            Owner::Other => "other",
        }
    }
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

/// `"1"` → scope mặc định `Persistent` ("Giữ kích hoạt khi tắt app"); khác → `Session`.
pub const SETTING_ACTIVATION_KEEP: &str = "activation_keep_after_quit";
/// `"1"` sau khi onboarding đã hỏi về kích hoạt.
pub const SETTING_ACTIVATION_ONBOARDED: &str = "activation_onboarded";
/// Ghi vào mỗi dòng `activation_state`.
pub const ENGINE_VERSION: u32 = 1;

/// Tắt thủ công một font mà collection còn giữ: hỏi lại (mặc định) hay gỡ khỏi mọi lý do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeactivatePolicy {
    #[default]
    Ask,
    Force,
}

/// Khi có font trùng PostScript name: hỏi người dùng (mặc định) hay vẫn kích hoạt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    #[default]
    Ask,
    Proceed,
    /// "Tắt bản cũ và bật bản này": gỡ các bản đăng ký xung đột (nếu ma trận cho phép) rồi bật bản của Font Atlas.
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub postscript_name: String,
    pub existing_path: String,
    pub owner: Owner,
    /// Nội dung file đang cài GIỐNG HỆT file trong thư viện (cùng kích thước, từng byte): font đã dùng được rồi,
    /// không cần hỏi. Chỉ khác phiên bản/khác file mới là xung đột thật.
    pub same_file: bool,
    /// Ma trận "Thay thế" cho bản này, tính ở backend trên đường dẫn đã canonical hoá (UI dùng nguyên, không tự đoán).
    pub replace_tier: ReplaceTier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum FileOutcome {
    /// Chỉ `preflight` trả: không có xung đột, sẵn sàng kích hoạt.
    Ready,
    Activated,
    AlreadyActive,
    /// Font đã dùng được nhờ một file GIỐNG HỆT do bên khác cung cấp (`owner`, `path`). Font Atlas không đăng ký
    /// gì và không ghi dòng kích hoạt; bên kia gỡ thì font quay về "chưa bật".
    AlreadyAvailable { owner: Owner, path: String },
    Deactivated,
    AlreadyInactive,
    /// Một thao tác khác đang chạy trên cùng file.
    Busy,
    NeedsConfirmation { conflicts: Vec<Conflict> },
    /// Tắt thủ công nhưng collection vẫn giữ font này (policy `Ask`): không đổi gì, UI hỏi lại.
    HeldByCollections { collection_ids: Vec<i64> },
    /// Đã gỡ lý do được yêu cầu nhưng font vẫn bật vì lý do khác (thủ công / collection khác).
    KeptActive { manual: bool, collection_ids: Vec<i64> },
    MissingFile,
    NotDownloaded,
    Failed { code: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub hash_hex: String,
    pub path: Option<String>,
    pub requested_faces: Vec<u32>,
    /// Face khác của cùng file (TTC) cũng được kích hoạt / tắt theo.
    pub extra_faces: Vec<u32>,
    pub outcome: FileOutcome,
    /// Bản cũ đã gỡ để thay thế (policy `Replace`); mỗi mục có `id` để "Khôi phục bản cũ".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replaced: Vec<ReplacedFont>,
    /// Đã làm xong việc được yêu cầu nhưng có điều người dùng cần biết (vd `replace_not_effective`: font của ta đã bật
    /// nhưng bản khác vẫn đang cung cấp cùng tên). Kết quả vẫn là `Activated`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplacedFont {
    pub id: i64,
    pub path: String,
    pub owner: Owner,
    /// Scope mà lệnh gỡ trả OK (khôi phục đăng ký lại đúng các scope này).
    pub scopes: Vec<fontatlas_store::ActivationScope>,
}

/// Ma trận "Thay thế" (xem docs/team/research/2026-11-11-replace-activation.md §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplaceTier {
    /// Gỡ được qua CoreText và có thể khôi phục.
    Replace,
    /// Chỉ hướng dẫn người dùng tắt trong app đó (Adobe, Connect/Suitcase).
    Guide,
    /// Không bao giờ đụng: /System, /Library/Fonts, ~/Library/Fonts.
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreOutcome {
    Restored,
    AlreadyRestored,
    /// File bản cũ không còn: giữ nguyên sổ để thử lại khi ổ cắm lại.
    MissingFile,
    /// File bản cũ nằm trên ổ cloud và chưa tải về: không đăng ký (sẽ treo / tải ngầm); giữ sổ.
    NotDownloaded,
}

/// Xung đột mà người dùng đã thấy và đồng ý thay: (hash file của thư viện, đường dẫn bản đang cài).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedConflict {
    pub hash: [u8; 32],
    pub existing_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivationReport {
    pub results: Vec<FileResult>,
    pub changed: usize,
    pub needs_confirmation: usize,
    pub failed: usize,
}

impl ActivationReport {
    pub(crate) fn push(&mut self, r: FileResult) {
        match r.outcome {
            FileOutcome::Activated | FileOutcome::Deactivated => self.changed += 1,
            FileOutcome::NeedsConfirmation { .. } => self.needs_confirmation += 1,
            FileOutcome::Failed { .. } => self.failed += 1,
            _ => {}
        }
        self.results.push(r);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub hash_hex: String,
}

/// Một dòng trạng thái kích hoạt cho UI (lệnh `activation_state` của Plan 6a).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationStateDto {
    pub hash_hex: String,
    pub state: fontatlas_store::DesiredState,
    pub scope: fontatlas_store::ActivationScope,
    pub status: fontatlas_store::ActivationStatus,
    pub reasons: Vec<fontatlas_store::ActivationReason>,
    pub active_path: Option<String>,
    pub conflict_ps: Option<String>,
    pub last_error: Option<String>,
}
