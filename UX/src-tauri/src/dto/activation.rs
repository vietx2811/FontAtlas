//! DTO kích hoạt font: bản sao trong app của kiểu Plan 7 (hình dạng JSON giữ nguyên fixture của Plan 7).
use fontatlas_activate as fa;
use fontatlas_store::{ActivationReason, ActivationScope, ActivationStatus, DesiredState};
use serde::Serialize;
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
}

impl From<&ActivationReason> for ReasonDto {
    fn from(r: &ActivationReason) -> Self {
        match r {
            ActivationReason::Manual => ReasonDto::Manual,
            ActivationReason::Collection(id) => ReasonDto::Collection { collection_id: *id },
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum FileOutcomeDto {
    Ready,
    Activated,
    AlreadyActive,
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
            F::Deactivated => Self::Deactivated,
            F::AlreadyInactive => Self::AlreadyInactive,
            F::Busy => Self::Busy,
            F::NeedsConfirmation { conflicts } => Self::NeedsConfirmation {
                conflicts: conflicts
                    .iter()
                    .map(|c| ConflictDto { postscript_name: c.postscript_name.clone(), existing_path: c.existing_path.clone(), owner: c.owner.into() })
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FileResultDto {
    pub hash_hex: String,
    pub path: Option<String>,
    pub requested_faces: Vec<u32>,
    pub extra_faces: Vec<u32>,
    pub outcome: FileOutcomeDto,
}

impl From<&fa::FileResult> for FileResultDto {
    fn from(r: &fa::FileResult) -> Self {
        FileResultDto {
            hash_hex: r.hash_hex.clone(),
            path: r.path.clone(),
            requested_faces: r.requested_faces.clone(),
            extra_faces: r.extra_faces.clone(),
            outcome: (&r.outcome).into(),
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
    /// "collection_deleted" | "left_collection"
    pub cause: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ReconcileReportDto {
    pub auto_deactivated: Vec<AutoDeactivatedDto>,
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
        ReconcileReportDto {
            auto_deactivated: r
                .auto_deactivated
                .iter()
                .map(|a| AutoDeactivatedDto {
                    hash_hex: a.hash_hex.clone(),
                    faces: a.faces.clone(),
                    removed_reasons: a.removed_reasons.iter().map(ReasonDto::from).collect(),
                    cause: a.cause.clone(),
                })
                .collect(),
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
        !self.auto_deactivated.is_empty() || self.reregistered + self.moved + self.missing + self.not_downloaded + self.failed > 0
    }
}
