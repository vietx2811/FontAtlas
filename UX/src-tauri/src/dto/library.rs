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
    /// Nguồn nằm trên ổ cloud (theo đường dẫn): `one_drive` | `google_drive` | `dropbox` | `box` | `icloud` | `other`.
    pub cloud_provider: Option<String>,
    /// Số font của nguồn còn là placeholder trên ổ cloud (chưa tải về máy), theo lần quét gần nhất.
    pub cloud_only_count: u32,
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

use crate::dto::langs::{LangBadgeDto, LangFilterDto, LangModeDto};
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

/// Đúng 20 khoá của `FaceRow` của store (parser Plan 5 từ chối khoá lạ).
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
    /// Mọi bản còn thấy là placeholder trên ổ cloud (chưa tải về): chưa dùng/bật ngay được.
    pub on_cloud_only: bool,
    pub is_favorite: bool,
    #[ts(type = "number")]
    pub added_at: i64,
    /// Năm phát hành ƯỚC LƯỢNG (độ tin cậy trung bình-thấp): năm trong dòng copyright, nếu không có thì `head.created`,
    /// rồi `head.modified`; giá trị rõ ràng sai (0, 1904, tương lai) bị bỏ. `null` = không rõ hoặc bản phân tích cũ chưa
    /// có dữ liệu (analyzer < core-0.8.0, có sau khi phân tích lại).
    pub release_year: Option<u16>,
    /// `copyright` | `head_created` | `head_modified`; `null` khi `releaseYear` là `null`.
    pub release_year_source: Option<String>,
    /// Ngày tạo file sớm nhất (ms từ epoch) trong các vị trí còn thấy; `null` = không đọc được / chưa quét lại.
    /// Độ tin cậy thấp làm "ngày font ra đời": sao chép/đồng bộ đặt lại ngày.
    #[ts(type = "number | null")]
    pub file_created_at: Option<i64>,
    /// `disk` | `cloud_sync` (file trên ổ cloud: có thể là lúc đồng bộ); `null` khi `fileCreatedAt` là `null`.
    pub file_created_source: Option<String>,
    /// Chỉ báo ngôn ngữ trên thẻ, theo `FaceQueryDto.badgeLang` (mặc định `vie`). `null` khi mã ngôn ngữ lạ.
    /// `viMissing` giữ nguyên cho tương thích; giao diện dùng `langBadge` thay chỉ báo `−N`.
    pub lang_badge: Option<LangBadgeDto>,
}

impl From<FaceRow> for FaceRowDto {
    fn from(r: FaceRow) -> Self {
        let lang_badge = crate::services::langs::badge_of(crate::services::langs::DEFAULT_BADGE_LANG, r.lang_hit.as_ref(), r.lang_computed, r.vi_missing);
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
            on_cloud_only: r.on_cloud_only,
            is_favorite: r.is_favorite,
            added_at: r.added_at,
            release_year: r.release_year,
            release_year_source: r.release_year_source,
            file_created_at: r.file_created_at,
            file_created_source: r.file_created_source,
            lang_badge,
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
    // Không chọn sort (mặc định): như `family` nhưng có chữ tìm thì vẫn xếp khớp đầu từ trước.
    #[default]
    Auto,
    Family,
    Weight,
    Added,
    /// Năm phát hành ước lượng; không rõ luôn cuối bất kể chiều.
    Released,
    /// Ngày tạo file sớm nhất; không đọc được luôn cuối bất kể chiều.
    FileCreated,
    Match,
}

impl From<SortKeyDto> for SortKey {
    fn from(s: SortKeyDto) -> Self {
        match s {
            SortKeyDto::Auto => SortKey::Auto,
            SortKeyDto::Family => SortKey::Family,
            SortKeyDto::Weight => SortKey::Weight,
            SortKeyDto::Added => SortKey::Added,
            SortKeyDto::Released => SortKey::Released,
            SortKeyDto::FileCreated => SortKey::FileCreated,
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
    /// Lọc theo class (OR trong nhóm): serif | slab | sans | mono | script | handwriting | display | blackletter | symbol | uncertain
    /// | unclassified (face chưa có hồ sơ phân tích trong Index — tách riêng khỏi uncertain).
    /// Dùng phân loại hiệu lực (ghi đè của người dùng ưu tiên hơn tự động).
    pub classes: Vec<String>,
    /// Lọc theo subclass (snake_case, OR trong nhóm; AND với `classes`).
    pub subclasses: Vec<String>,
    /// `true`: `classes`/`subclasses` khớp cả class PHỤ (hybrid, `ClassificationDto.secondary`) của phân loại tự động,
    /// không chỉ class chính. Face bị người dùng ghi đè không có class phụ. Mặc định `false` (chỉ class chính).
    pub include_secondary: bool,
    /// `true`: chỉ giữ face có nhãn phân loại tự tin: độ tin cậy ≥ `CONF_MIN` (0.88). Face đã bị người dùng ghi đè
    /// luôn tính là tự tin; face `uncertain` (dưới ngưỡng) bị loại. Áp dụng cả khi `classes` rỗng.
    pub confident_only: bool,
    /// Ngưỡng độ tin cậy tuỳ ý 0–1 (nâng ngưỡng của `confidentOnly`, không hạ thấp). `null` = không đặt thêm.
    pub min_confidence: Option<f32>,
    /// Lọc theo tag hình thái (OR): stencil | inline_outline | rounded | fat_face | pixel | decorative | reverse_contrast | high_contrast | monoline.
    pub traits: Vec<String>,
    /// Lọc theo ngôn ngữ (Hyperglot). Rỗng = không lọc. Chỉ face đã tính độ phủ mới khớp.
    pub langs: Vec<LangFilterDto>,
    /// `all` (mặc định): phải đủ mọi ngôn ngữ trong `langs`; `any`: đủ ít nhất một.
    pub lang_mode: LangModeDto,
    /// Ngôn ngữ cho chỉ báo `langBadge` trên thẻ (mã ISO 639-3, vd `vie`); `null` = `vie`.
    pub badge_lang: Option<String>,
    /// usWeightClass, khoảng đóng trong 100..=900 (font cũ ghi 1–9 được coi là ×100).
    pub weight_class: Option<[u16; 2]>,
    /// usWidthClass, khoảng đóng trong 1..=9.
    pub width_class: Option<[u16; 2]>,
    /// Font Variable cũng khớp khi dải trục `wght`/`wdth` giao khoảng đã chọn (mặc định bật cả hai).
    pub var_axis_match: VarAxisMatchDto,
    /// `true`: `queryFaces` xếp theo NHÓM family (mỗi family liền nhau; thứ tự nhóm theo `sort`/`descending`:
    /// family → tên, weight/added/released/file_created → kiểu nhẹ nhất|cũ nhất khi tăng, nặng nhất|mới nhất khi giảm,
    /// match → điểm cao nhất; không có giá trị → cuối). Trong nhóm: Weight tăng → thẳng trước nghiêng → tên.
    /// `familyGroups` với cùng truy vấn trả khoảng `[start, start+count)` của từng nhóm trong thứ tự này.
    pub group_by_family: bool,
}

/// Bật/tắt khớp theo dải trục Variable. `wdth` (%) quy về class theo bảng OS/2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct VarAxisMatchDto {
    pub wght: bool,
    pub wdth: bool,
}

impl Default for VarAxisMatchDto {
    fn default() -> Self {
        VarAxisMatchDto { wght: true, wdth: true }
    }
}

/// Một nhóm family trong kết quả `familyGroups` (xem `FaceQueryDto.groupByFamily`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FamilyGroupDto {
    pub family: String,
    /// Vị trí face đầu của nhóm trong thứ tự `queryFaces` (`groupByFamily = true`): khớp `offset` của `pagedFaces`.
    pub start: u32,
    /// Số face của family trong truy vấn.
    pub count: u32,
    /// Số kiểu khớp bộ lọc (= `count`).
    pub match_count: u32,
    /// Tổng số kiểu của family trong thư viện, không áp bộ lọc — chip "matchCount/styleTotal kiểu".
    pub style_total: u32,
}

impl From<fontatlas_store::FamilyGroup> for FamilyGroupDto {
    fn from(g: fontatlas_store::FamilyGroup) -> Self {
        FamilyGroupDto { family: g.family, start: g.start, count: g.count, match_count: g.match_count, style_total: g.style_total }
    }
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

/// Danh mục tag hệ thống (phong cách, mood, mục đích dùng): KHÔNG đo bằng máy, chỉ là tag người dùng gán.
/// Tên lưu: `namespace:key` (ví dụ `mood:calm`); `create_tag` chuẩn hoá và từ chối tiền tố dành riêng không có trong danh mục.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SystemTagGroupDto {
    /// "style" | "mood" | "use"
    pub namespace: String,
    pub label: String,
    pub tags: Vec<SystemTagDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SystemTagDto {
    pub key: String,
    pub label: String,
    /// Tên tag đầy đủ để gọi `create_tag` / so với `TagDto.name`: `namespace:key`.
    pub name: String,
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
