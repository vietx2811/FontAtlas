//! DTO khởi động, onboarding và DB cũ.
use fontatlas_store::{LegacyImport, LegacyPreview, LegacyResolve};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::CmdError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum BootPhase {
    Starting,
    Scanning,
    Indexing,
    Activation,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LegacyPreviewDto {
    pub fonts: u32,
    pub sources: u32,
    pub collections: u32,
    pub favorites: u32,
    pub tags: u32,
}

impl From<LegacyPreview> for LegacyPreviewDto {
    fn from(p: LegacyPreview) -> Self {
        LegacyPreviewDto {
            fonts: p.fonts as u32,
            sources: p.sources as u32,
            collections: p.collections as u32,
            favorites: p.favorites as u32,
            tags: p.tags as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LegacyImportDto {
    pub fonts: u32,
    pub sources: u32,
    pub collections: u32,
    pub recovered_collections: u32,
    pub bad_tags: u32,
    pub skipped_sources: u32,
}

impl From<LegacyImport> for LegacyImportDto {
    fn from(i: LegacyImport) -> Self {
        LegacyImportDto {
            fonts: i.fonts as u32,
            sources: i.sources as u32,
            collections: i.collections as u32,
            recovered_collections: i.recovered_collections as u32,
            bad_tags: i.bad_tags as u32,
            skipped_sources: i.skipped_sources as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LegacyResolveDto {
    pub resolved: u32,
    pub unresolved: u32,
    pub ambiguous_faces: u32,
}

impl From<LegacyResolve> for LegacyResolveDto {
    fn from(r: LegacyResolve) -> Self {
        LegacyResolveDto { resolved: r.resolved as u32, unresolved: r.unresolved as u32, ambiguous_faces: r.ambiguous_faces as u32 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LegacyStateDto {
    pub found: bool,
    pub preview: Option<LegacyPreviewDto>,
    pub imported: Option<LegacyImportDto>,
    pub resolved: Option<LegacyResolveDto>,
    pub error: Option<CmdError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct BootStatusDto {
    pub phase: BootPhase,
    pub store_error: Option<CmdError>,
    pub library_recreated: bool,
    pub user_backup: Option<String>,
    pub legacy: LegacyStateDto,
}

impl Default for BootStatusDto {
    fn default() -> Self {
        BootStatusDto {
            phase: BootPhase::Starting,
            store_error: None,
            library_recreated: false,
            user_backup: None,
            legacy: LegacyStateDto::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FontManagerDto {
    pub name: String,
    pub installed: bool,
    /// Số font đang kích hoạt trong trình quản lý đó; `None` cho tới khi Plan 7 cung cấp.
    pub active_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OnboardingDto {
    pub step: Option<String>,
    pub has_sources: bool,
    pub legacy: LegacyStateDto,
    pub license_notice_ack: bool,
    pub embed_enabled: bool,
    pub model_state: String,
    pub activation_supported: bool,
    pub activation_onboarded: bool,
    pub keep_active_on_exit: bool,
    pub font_managers: Vec<FontManagerDto>,
}
