//! DTO kích hoạt font: bản sao trong app của kiểu Plan 7 (hình dạng JSON giữ nguyên fixture của Plan 7).
use fontatlas_activate as fa;
use fontatlas_store::{ActivationReason, ActivationScope, ActivationStatus, DesiredState};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ActivationCapsDto {
    pub method: String,
    pub supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ScopeDto {
    Session,
    Persistent,
}

impl From<ScopeDto> for fa::Scope {
    fn from(s: ScopeDto) -> Self {
        match s {
            ScopeDto::Session => ActivationScope::Session,
            ScopeDto::Persistent => ActivationScope::Persistent,
        }
    }
}

impl From<ActivationScope> for ScopeDto {
    fn from(s: ActivationScope) -> Self {
        match s {
            ActivationScope::Session => ScopeDto::Session,
            ActivationScope::Persistent => ScopeDto::Persistent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum DesiredStateDto {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ActivationStatusDto {
    Registered,
    Unregistered,
    Conflict,
    MissingFile,
    NotDownloaded,
    Error,
    Unknown,
}

impl From<ActivationStatus> for ActivationStatusDto {
    fn from(s: ActivationStatus) -> Self {
        match s {
            ActivationStatus::Registered => Self::Registered,
            ActivationStatus::Unregistered => Self::Unregistered,
            ActivationStatus::Conflict => Self::Conflict,
            ActivationStatus::MissingFile => Self::MissingFile,
            ActivationStatus::NotDownloaded => Self::NotDownloaded,
            ActivationStatus::Error => Self::Error,
            ActivationStatus::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ReasonDto {
    Manual,
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Collection {
        #[ts(type = "number")]
        collection_id: i64,
    },
    /// Một app thiết kế (photoshop, illustrator, indesign…) đã xin font này.
    Auto { app: String },
}

impl From<&ActivationReason> for ReasonDto {
    fn from(r: &ActivationReason) -> Self {
        match r {
            ActivationReason::Manual => ReasonDto::Manual,
            ActivationReason::Collection(id) => ReasonDto::Collection { collection_id: *id },
            ActivationReason::Auto(app) => ReasonDto::Auto { app: app.slug().to_string() },
        }
    }
}

/// Một dòng theo file (hash). `faces` = các face_index của file trong thư viện (TTC có nhiều face;
/// file một face luôn là `[0]`) để giao diện dựng `faceKeys` cho "Đang bật".
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ActivationStateDto {
    pub hash_hex: String,
    pub faces: Vec<u32>,
    pub state: DesiredStateDto,
    pub scope: ScopeDto,
    pub status: ActivationStatusDto,
    pub reasons: Vec<ReasonDto>,
    pub active_path: Option<String>,
    pub conflict_ps: Option<String>,
    pub last_error: Option<String>,
}

impl ActivationStateDto {
    pub fn from_core(s: fa::ActivationStateDto, faces: Vec<u32>) -> Self {
        ActivationStateDto {
            hash_hex: s.hash_hex,
            faces,
            state: match s.state {
                DesiredState::Active => DesiredStateDto::Active,
                DesiredState::Inactive => DesiredStateDto::Inactive,
            },
            scope: s.scope.into(),
            status: s.status.into(),
            reasons: s.reasons.iter().map(ReasonDto::from).collect(),
            active_path: s.active_path,
            conflict_ps: s.conflict_ps,
            last_error: s.last_error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum OwnerDto {
    System,
    UserInstalled,
    AdobeFonts,
    RightFont,
    FontBase,
    Typeface,
    Suitcase,
    FontAtlas,
    Other,
}

impl From<fa::Owner> for OwnerDto {
    fn from(o: fa::Owner) -> Self {
        match o {
            fa::Owner::System => Self::System,
            fa::Owner::UserInstalled => Self::UserInstalled,
            fa::Owner::AdobeFonts => Self::AdobeFonts,
            fa::Owner::RightFont => Self::RightFont,
            fa::Owner::FontBase => Self::FontBase,
            fa::Owner::Typeface => Self::Typeface,
            fa::Owner::Suitcase => Self::Suitcase,
            fa::Owner::FontAtlas => Self::FontAtlas,
            fa::Owner::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ConflictDto {
    pub postscript_name: String,
    pub existing_path: String,
    pub owner: OwnerDto,
    /// Font đang cài giống hệt file trong thư viện (từng byte): đã dùng được, không cần hỏi.
    pub same_file: bool,
    /// Ma trận "Thay thế" do backend tính (đã canonical hoá đường dẫn): UI dùng nguyên, không tự đoán.
    pub replace_tier: ReplaceTierDto,
    /// File trong thư viện là font variable (VF).
    pub variable: bool,
    /// Bản đang cài là VF? `null` = không đọc được. Cặp VF↔static: `existingVariable === !variable` (G7).
    pub existing_variable: Option<bool>,
}

/// Lựa chọn đã nhớ cho cặp VF↔static trùng tên PostScript (G7). `Ask` = hỏi mỗi lần (mặc định).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum VfPolicyDto {
    Ask,
    /// Luôn dùng bản VF (tắt bản static trùng tên của Font Atlas, hoặc không bật bản static).
    Vf,
    /// Luôn dùng bản static.
    Static,
    /// Luôn giữ cả hai (kèm cảnh báo: app thiết kế sẽ thấy hai mục trùng tên).
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ReplaceTierDto {
    Replace,
    Guide,
    Protected,
}

impl From<fa::ReplaceTier> for ReplaceTierDto {
    fn from(t: fa::ReplaceTier) -> Self {
        match t {
            fa::ReplaceTier::Replace => Self::Replace,
            fa::ReplaceTier::Guide => Self::Guide,
            fa::ReplaceTier::Protected => Self::Protected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum FileOutcomeDto {
    Ready,
    Activated,
    AlreadyActive,
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    AlreadyAvailable { owner: OwnerDto, path: String },
    Deactivated,
    AlreadyInactive,
    Busy,
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    NeedsConfirmation { conflicts: Vec<ConflictDto> },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    HeldByCollections {
        #[ts(type = "Array<number>")]
        collection_ids: Vec<i64>,
    },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    KeptActive {
        manual: bool,
        #[ts(type = "Array<number>")]
        collection_ids: Vec<i64>,
    },
    MissingFile,
    NotDownloaded,
    Failed { code: String, message: String },
}

impl From<&fa::FileOutcome> for FileOutcomeDto {
    fn from(o: &fa::FileOutcome) -> Self {
        use fa::FileOutcome as F;
        match o {
            F::Ready => Self::Ready,
            F::Activated => Self::Activated,
            F::AlreadyActive => Self::AlreadyActive,
            F::AlreadyAvailable { owner, path } => Self::AlreadyAvailable { owner: (*owner).into(), path: path.clone() },
            F::Deactivated => Self::Deactivated,
            F::AlreadyInactive => Self::AlreadyInactive,
            F::Busy => Self::Busy,
            F::NeedsConfirmation { conflicts } => Self::NeedsConfirmation {
                conflicts: conflicts
                    .iter()
                    .map(|c| ConflictDto { postscript_name: c.postscript_name.clone(), existing_path: c.existing_path.clone(), owner: c.owner.into(), same_file: c.same_file, replace_tier: c.replace_tier.into(), variable: c.variable, existing_variable: c.existing_variable })
                    .collect(),
            },
            F::HeldByCollections { collection_ids } => Self::HeldByCollections { collection_ids: collection_ids.clone() },
            F::KeptActive { manual, collection_ids } => Self::KeptActive { manual: *manual, collection_ids: collection_ids.clone() },
            F::MissingFile => Self::MissingFile,
            F::NotDownloaded => Self::NotDownloaded,
            F::Failed { code, message } => Self::Failed { code: code.clone(), message: message.clone() },
        }
    }
}

/// Font đã dùng được nhờ file giống hệt của bên khác ("Có sẵn trên máy"); Font Atlas không bật gì cho nó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AvailabilityDto {
    pub hash_hex: String,
    pub faces: Vec<u32>,
    pub owner: OwnerDto,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FileResultDto {
    pub hash_hex: String,
    pub path: Option<String>,
    pub requested_faces: Vec<u32>,
    pub extra_faces: Vec<u32>,
    pub outcome: FileOutcomeDto,
    /// Bản cũ đã gỡ để thay thế (chỉ khi dùng `activation_replace`); mỗi mục có `id` cho "Khôi phục bản cũ".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub replaced: Option<Vec<ReplacedFontDto>>,
    /// Đã xong nhưng cần báo (vd `replace_not_effective`: bản khác vẫn đang cung cấp font). Kết quả vẫn là `activated`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ReplacedFontDto {
    #[ts(type = "number")]
    pub id: i64,
    pub path: String,
    pub owner: OwnerDto,
    /// Scope mà bản cũ đã được đăng ký (khôi phục đăng ký lại đúng các scope này).
    pub scopes: Vec<ScopeDto>,
}

/// Xung đột người dùng đã thấy và xác nhận thay: file của thư viện + đường dẫn bản đang cài. Backend từ chối nếu tập này đã đổi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ConfirmedConflictDto {
    pub hash_hex: String,
    pub existing_path: String,
}

/// Một bản cũ đã gỡ và chưa khôi phục (Cài đặt → Kích hoạt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ReplacedEntryDto {
    #[ts(type = "number")]
    pub id: i64,
    /// File của thư viện đã thay nó.
    pub hash_hex: String,
    pub path: String,
    pub owner: String,
    pub scopes: Vec<ScopeDto>,
    pub postscript_names: Vec<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum RestoreOutcomeDto {
    Restored,
    AlreadyRestored,
    MissingFile,
    NotDownloaded,
}

impl From<fa::RestoreOutcome> for RestoreOutcomeDto {
    fn from(o: fa::RestoreOutcome) -> Self {
        match o {
            fa::RestoreOutcome::Restored => Self::Restored,
            fa::RestoreOutcome::AlreadyRestored => Self::AlreadyRestored,
            fa::RestoreOutcome::MissingFile => Self::MissingFile,
            fa::RestoreOutcome::NotDownloaded => Self::NotDownloaded,
        }
    }
}

impl From<&fa::FileResult> for FileResultDto {
    fn from(r: &fa::FileResult) -> Self {
        FileResultDto {
            hash_hex: r.hash_hex.clone(),
            path: r.path.clone(),
            requested_faces: r.requested_faces.clone(),
            extra_faces: r.extra_faces.clone(),
            outcome: (&r.outcome).into(),
            warning: r.warning.clone(),
            replaced: (!r.replaced.is_empty()).then(|| {
                r.replaced
                    .iter()
                    .map(|x| ReplacedFontDto { id: x.id, path: x.path.clone(), owner: x.owner.into(), scopes: x.scopes.iter().map(|s| (*s).into()).collect() })
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ActivationReportDto {
    pub results: Vec<FileResultDto>,
    pub changed: u32,
    pub needs_confirmation: u32,
    pub failed: u32,
}

impl From<&fa::ActivationReport> for ActivationReportDto {
    fn from(r: &fa::ActivationReport) -> Self {
        ActivationReportDto {
            results: r.results.iter().map(Into::into).collect(),
            changed: r.changed as u32,
            needs_confirmation: r.needs_confirmation as u32,
            failed: r.failed as u32,
        }
    }
}

/// Kết quả "Bật tất cả" của một Dự án: báo cáo kích hoạt như `activateFaces` (cùng luồng xung đột — `needsConfirmation`
/// → gọi lại với `proceed = true`) + các font của Dự án bị bỏ qua vì `missing` (không còn trong thư viện / thiếu trên đĩa).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CaseActivationDto {
    pub report: ActivationReportDto,
    /// `skipped: missing` — không bị tính vào `report.failed`.
    pub skipped_missing: Vec<crate::dto::library::FaceKeyDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ProgressDto {
    pub done: u32,
    pub total: u32,
    pub hash_hex: String,
}

impl From<&fa::Progress> for ProgressDto {
    fn from(p: &fa::Progress) -> Self {
        ProgressDto { done: p.done as u32, total: p.total as u32, hash_hex: p.hash_hex.clone() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AutoDeactivatedDto {
    pub hash_hex: String,
    pub faces: Vec<u32>,
    pub removed_reasons: Vec<ReasonDto>,
    /// "collection_deleted" | "left_collection" | "no_reason"
    pub cause: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ReconcileReportDto {
    pub auto_deactivated: Vec<AutoDeactivatedDto>,
    /// Tự gỡ thất bại (hệ điều hành từ chối…): font vẫn đăng ký, giao diện nên báo.
    pub auto_deactivate_failed: Vec<AutoDeactivatedDto>,
    pub reregistered: u32,
    pub moved: u32,
    pub missing: u32,
    pub not_downloaded: u32,
    pub cleaned: u32,
    pub failed: u32,
    pub results: Vec<FileResultDto>,
}

impl From<&fa::ReconcileReport> for ReconcileReportDto {
    fn from(r: &fa::ReconcileReport) -> Self {
        let conv = |v: &[fa::AutoDeactivated]| -> Vec<AutoDeactivatedDto> {
            v.iter()
                .map(|a| AutoDeactivatedDto {
                    hash_hex: a.hash_hex.clone(),
                    faces: a.faces.clone(),
                    removed_reasons: a.removed_reasons.iter().map(ReasonDto::from).collect(),
                    cause: a.cause.clone(),
                })
                .collect()
        };
        ReconcileReportDto {
            auto_deactivated: conv(&r.auto_deactivated),
            auto_deactivate_failed: conv(&r.auto_deactivate_failed),
            reregistered: r.reregistered as u32,
            moved: r.moved as u32,
            missing: r.missing as u32,
            not_downloaded: r.not_downloaded as u32,
            cleaned: r.cleaned as u32,
            failed: r.failed as u32,
            results: r.results.iter().map(Into::into).collect(),
        }
    }
}

impl ReconcileReportDto {
    /// Có gì đáng báo cho giao diện không.
    pub fn is_notable(&self) -> bool {
        !self.auto_deactivated.is_empty() || !self.auto_deactivate_failed.is_empty() || self.reregistered + self.moved + self.missing + self.not_downloaded + self.failed > 0
    }
}
