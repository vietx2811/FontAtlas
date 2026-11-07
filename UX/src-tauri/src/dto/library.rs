//! DTO nguồn font và quét.
use fontatlas_store::{RescanStats, RescanStatus, ScanPhase, ScanProgress};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ScanPhaseDto {
    Walking,
    Hashing,
    Analyzing,
    MarkingMissing,
}

impl From<ScanPhase> for ScanPhaseDto {
    fn from(p: ScanPhase) -> Self {
        match p {
            ScanPhase::Walking => Self::Walking,
            ScanPhase::Hashing => Self::Hashing,
            ScanPhase::Analyzing => Self::Analyzing,
            ScanPhase::MarkingMissing => Self::MarkingMissing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ScanProgressDto {
    pub phase: ScanPhaseDto,
    pub done: u32,
    pub total: u32,
    pub faces_written: u32,
    /// Font còn là placeholder đám mây (chưa tải về): giao diện hiện "N font chưa tải về".
    pub not_downloaded: u32,
}

impl From<&ScanProgress> for ScanProgressDto {
    fn from(p: &ScanProgress) -> Self {
        ScanProgressDto {
            phase: p.phase.into(),
            done: p.done as u32,
            total: p.total as u32,
            faces_written: p.faces_written as u32,
            not_downloaded: p.not_downloaded as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum RescanStatusDto {
    Ok,
    SourceUnavailable,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RescanStatsDto {
    pub status: RescanStatusDto,
    pub seen: u32,
    pub unchanged: u32,
    pub not_downloaded: u32,
    pub relinked: u32,
    pub analyzed: u32,
    pub failed: u32,
    pub missing: u32,
    pub unreadable: Vec<(String, String)>,
}

impl From<&RescanStats> for RescanStatsDto {
    fn from(s: &RescanStats) -> Self {
        RescanStatsDto {
            status: match s.status {
                RescanStatus::Ok => RescanStatusDto::Ok,
                RescanStatus::SourceUnavailable => RescanStatusDto::SourceUnavailable,
                RescanStatus::Cancelled => RescanStatusDto::Cancelled,
            },
            seen: s.seen as u32,
            unchanged: s.unchanged as u32,
            not_downloaded: s.not_downloaded as u32,
            relinked: s.relinked as u32,
            analyzed: s.analyzed as u32,
            failed: s.failed as u32,
            missing: s.missing as u32,
            unreadable: s.unreadable.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SourceDto {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub path: Option<String>,
    pub enabled: bool,
    pub available: bool,
    pub face_count: u32,
    pub last_scan: Option<RescanStatsDto>,
    pub scanning: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ScanStatusDto {
    pub running: bool,
    pub paused: bool,
    #[ts(type = "number | null")]
    pub current_source_id: Option<i64>,
    #[ts(type = "Array<number>")]
    pub queued: Vec<i64>,
}

// ---- thư viện font ----

use fontatlas_store::{Collection, FaceKey, FacePage, FaceRow, ProblemFile, SortKey, Tag};
use serde::Deserialize;

use crate::error::CmdError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FaceKeyDto {
    pub hash_hex: String,
    pub face_index: u32,
}

impl TryFrom<&FaceKeyDto> for FaceKey {
    type Error = CmdError;
    fn try_from(k: &FaceKeyDto) -> Result<Self, CmdError> {
        FaceKey::from_hex(&k.hash_hex, k.face_index)
            .map_err(|e| CmdError::invalid_arg(e.to_string()))
    }
}

impl From<&FaceKey> for FaceKeyDto {
    fn from(k: &FaceKey) -> Self {
        FaceKeyDto {
            hash_hex: k.hash_hex(),
            face_index: k.face_index,
        }
    }
}

pub fn keys_of(v: &[FaceKeyDto]) -> Result<Vec<FaceKey>, CmdError> {
    v.iter().map(FaceKey::try_from).collect()
}

/// Đúng 19 khoá của `FaceRow` của store (parser Plan 5 từ chối khoá lạ).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FaceRowDto {
    #[ts(type = "number")]
    pub face_id: i64,
    pub hash_hex: String,
    pub face_index: u32,
    pub family: String,
    pub subfamily: Option<String>,
    pub full_name: Option<String>,
    pub weight_class: Option<u16>,
    pub width_class: Option<u16>,
    pub is_italic: bool,
    pub is_variable: bool,
    pub vi_status: String,
    pub supports_vi: bool,
    pub vi_missing: u32,
    pub license_class: String,
    pub path: Option<String>,
    #[ts(type = "number")]
    pub file_size: u64,
    pub missing: bool,
    pub is_favorite: bool,
    #[ts(type = "number")]
    pub added_at: i64,
}

impl From<FaceRow> for FaceRowDto {
    fn from(r: FaceRow) -> Self {
        FaceRowDto {
            face_id: r.face_id,
            hash_hex: r.hash_hex,
            face_index: r.face_index,
            family: r.family,
            subfamily: r.subfamily,
            full_name: r.full_name,
            weight_class: r.weight_class,
            width_class: r.width_class,
            is_italic: r.is_italic,
            is_variable: r.is_variable,
            vi_status: r.vi_status,
            supports_vi: r.supports_vi,
            vi_missing: r.vi_missing,
            license_class: r.license_class,
            path: r.path,
            file_size: r.file_size,
            missing: r.missing,
            is_favorite: r.is_favorite,
            added_at: r.added_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FacePageDto {
    #[ts(type = "number")]
    pub total: u64,
    pub offset: u32,
    pub items: Vec<FaceRowDto>,
}

impl From<FacePage> for FacePageDto {
    fn from(p: FacePage) -> Self {
        FacePageDto {
            total: p.total,
            offset: p.offset,
            items: p.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum SortKeyDto {
    #[default]
    Family,
    Weight,
    Added,
    Match,
}

impl From<SortKeyDto> for SortKey {
    fn from(s: SortKeyDto) -> Self {
        match s {
            SortKeyDto::Family => SortKey::Family,
            SortKeyDto::Weight => SortKey::Weight,
            SortKeyDto::Added => SortKey::Added,
            SortKeyDto::Match => SortKey::Match,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ViFilterDto {
    Full,
    Supported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum StrengthDto {
    Required,
    Preferred,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AxisFilterDto {
    pub axis: String,
    pub range: Option<[f32; 2]>,
    pub equals: Option<String>,
    pub strength: StrengthDto,
    pub weight: f32,
}

/// Khoảng đóng theo số đo vật lý gốc (không phải phân vị).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RawAxisRangeDto {
    pub axis: String,
    pub range: [f32; 2],
}

/// Mọi trường `FaceQuery` của store + lọc theo trục + `faceKeys` ("Gần đây", "Đang bật").
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct FaceQueryDto {
    pub text: Option<String>,
    pub vi: Option<ViFilterDto>,
    pub license: Vec<String>,
    pub variable: Option<bool>,
    pub italic: Option<bool>,
    #[ts(type = "Array<number>")]
    pub source_ids: Vec<i64>,
    pub favorites_only: bool,
    #[ts(type = "number | null")]
    pub collection_id: Option<i64>,
    #[ts(type = "Array<number>")]
    pub tag_ids: Vec<i64>,
    pub include_missing: bool,
    pub sort: SortKeyDto,
    pub descending: bool,
    pub offset: u32,
    pub limit: u32,
    pub axis_filters: Vec<AxisFilterDto>,
    pub raw_axis_ranges: Vec<RawAxisRangeDto>,
    /// Ràng buộc cứng từ `describe` ("vi_supported" | "vi_complete"): loại face không đạt.
    pub hard: Vec<String>,
    /// Chỉ các face này (≤ 2000), giữ thứ tự khi `sort = "match"`.
    pub face_keys: Option<Vec<FaceKeyDto>>,
    /// Lọc theo class (OR trong nhóm): serif | sans | mono | script | display | blackletter | symbol | uncertain
    /// | unclassified (face chưa có hồ sơ phân tích trong Index — tách riêng khỏi uncertain).
    /// Dùng phân loại hiệu lực (ghi đè của người dùng ưu tiên hơn tự động).
    pub classes: Vec<String>,
    /// Lọc theo subclass (snake_case, OR trong nhóm; AND với `classes`).
    pub subclasses: Vec<String>,
    /// Lọc theo tag hình thái (OR): stencil | inline_outline | rounded | fat_face | pixel | decorative.
    pub traits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CollectionDto {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub item_count: u32,
}

impl From<Collection> for CollectionDto {
    fn from(c: Collection) -> Self {
        CollectionDto {
            id: c.id,
            name: c.name,
            color: c.color,
            item_count: c.item_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct TagDto {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub face_count: u32,
}

impl From<Tag> for TagDto {
    fn from(t: Tag) -> Self {
        TagDto {
            id: t.id,
            name: t.name,
            face_count: t.face_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ProblemDto {
    pub hash_hex: String,
    pub face_index: Option<u32>,
    pub stage: String,
    pub code: String,
    pub detail: Option<String>,
    pub attempts: u32,
    pub path: Option<String>,
}

impl From<ProblemFile> for ProblemDto {
    fn from(p: ProblemFile) -> Self {
        ProblemDto {
            hash_hex: p.hash_hex,
            face_index: p.face_index,
            stage: p.stage,
            code: p.code,
            detail: p.detail,
            attempts: p.attempts,
            path: p.path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AxisCoordDto {
    pub tag: String,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct InstanceDto {
    pub name: String,
    pub coords: Vec<AxisCoordDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FamilyStyleDto {
    pub face: FaceKeyDto,
    pub subfamily: Option<String>,
    pub weight: Option<u16>,
    pub italic: bool,
    pub instance: Option<InstanceDto>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct UiStateDto {
    pub sidebar_collapsed: bool,
    pub open_sections: Vec<String>,
    pub layout: String,
    pub sample_size: u32,
    pub info_expanded: bool,
}

impl Default for UiStateDto {
    fn default() -> Self {
        UiStateDto {
            sidebar_collapsed: false,
            open_sections: vec!["col".into(), "src".into(), "tag".into()],
            layout: "grid".into(),
            sample_size: 34,
            info_expanded: false,
        }
    }
}
