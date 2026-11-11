//! DTO phân tích: chi tiết face, Similar, Pairing, cặp đã lưu, tìm bằng mô tả.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dto::library::{AxisFilterDto, FaceKeyDto, FaceRowDto};

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AxisReadingDto {
    pub key: String,
    #[ts(type = "number | string | null")]
    pub value: Option<serde_json::Value>,
    /// Giá trị số gốc, kể cả khi `value` là lớp serif; không đổi dấu hay làm tròn.
    pub raw_value: Option<f64>,
    pub unit: Option<String>,
    /// "measured" | "estimated" | "failed"
    pub quality: String,
    pub reason: Option<String>,
    pub method: Option<String>,
    pub percentile: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum SimilarSourceDto {
    Axes,
    AxesAndImage,
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SimilarHitDto {
    pub face: FaceKeyDto,
    pub row: FaceRowDto,
    /// Có trong DTO nhưng giao diện không hiển thị điểm hay phần trăm.
    pub distance: f32,
    pub source: SimilarSourceDto,
    pub same: Vec<String>,
    pub differ: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SimilarFamilyDto {
    pub family: String,
    pub representative: SimilarHitDto,
    pub faces: Vec<FaceKeyDto>,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SimilarFamilyPageDto {
    pub total: u32,
    pub offset: u32,
    pub items: Vec<SimilarFamilyDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AxisStatsDto {
    pub axis: String,
    pub unit: String,
    pub count: u32,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub p1: Option<f32>,
    pub p99: Option<f32>,
}

/// Histogram của một trục trên tập đang lọc **trừ chính trục đó** (kéo slider không làm histogram tự co).
/// `lo..hi` là miền hiển thị (p1–p99 của tập chưa lọc trục nào; giá trị ngoài miền, kể cả đúng `hi`, dồn vào
/// bin cuối/đầu); khi mọi giá trị bằng nhau miền được nới ±0.5. Trục rời rạc (`weight_class`, `width_class`)
/// có đúng 9 bin, mỗi bin một nấc.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AxisHistogramDto {
    pub axis: String,
    pub unit: String,
    /// Tổng số face đã đếm vào `bins`.
    pub count: u32,
    pub lo: f32,
    pub hi: f32,
    /// Mỗi face một lần: trục đo → giá trị đo; trục class → class của instance mặc định.
    pub bins: Vec<u32>,
    /// Chuỗi thứ hai cho UI vẽ chồng: như `bins` nhưng font Variable còn được đếm vào mọi nấc mà dải trục
    /// `wght`/`wdth` của nó phủ (đúng tập mà `varAxisMatch` bật sẽ khớp). Trục đo: giống `bins`.
    pub span_bins: Vec<u32>,
    /// true = không face nào (trong tập chưa lọc trục) có số đo cho trục này: không vẽ gì.
    pub empty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct GroupWeightsDto {
    pub proportion: f32,
    pub weight: f32,
    pub structure: f32,
    pub detail: f32,
}

impl Default for GroupWeightsDto {
    fn default() -> Self {
        GroupWeightsDto {
            proportion: 1.0,
            weight: 1.0,
            structure: 1.0,
            detail: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct SimilarOptsDto {
    pub weights: GroupWeightsDto,
    pub use_image: bool,
    /// ≤ 100
    pub k: u32,
    pub vi_only: bool,
    #[ts(type = "Array<number>")]
    pub source_ids: Vec<i64>,
    pub weight_range: Option<[u16; 2]>,
}

impl Default for SimilarOptsDto {
    fn default() -> Self {
        SimilarOptsDto {
            weights: Default::default(),
            use_image: true,
            k: 12,
            vi_only: false,
            source_ids: vec![],
            weight_range: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum PairRoleDto {
    Heading,
    Body,
    Code,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum PairModeDto {
    Safe,
    Controlled,
    Experimental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum LangDto {
    Vi,
    En,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PairOptsDto {
    pub role: PairRoleDto,
    pub mode: Option<PairModeDto>,
    pub project_vi: bool,
    /// ≤ 100
    pub k: u32,
    #[ts(type = "number | null")]
    pub collection_id: Option<i64>,
    pub lang: LangDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PairLabelDto {
    pub axis: String,
    pub relation: String,
    /// ● hoà hợp · ◐ tương phản · ◆ va chạm · ○ chưa chắc · ◑ gợi ý độ đậm
    pub symbol: String,
    pub delta: Option<f32>,
    #[ts(type = "number | null")]
    pub a: Option<f32>,
    #[ts(type = "number | null")]
    pub b: Option<f32>,
    pub role_reversed: bool,
}

/// Không có trường điểm số nào.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PairCandidateDto {
    pub face: FaceKeyDto,
    pub row: FaceRowDto,
    pub gate: String,
    pub labels: Vec<PairLabelDto>,
    pub clashes: Vec<String>,
    pub body_warnings: Vec<String>,
    pub heading_overflow: bool,
    pub insufficient_data: bool,
    pub weight_hint: Option<String>,
    pub block: Option<String>,
    pub sentences: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct NewPairDto {
    pub a: FaceKeyDto,
    pub a_role: PairRoleDto,
    pub b: FaceKeyDto,
    pub b_role: PairRoleDto,
    pub mode: Option<String>,
    pub preset: Option<String>,
    pub sample_text: Option<String>,
    #[ts(type = "number | null")]
    pub collection_id: Option<i64>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SavedPairDto {
    #[ts(type = "number")]
    pub id: i64,
    pub a: FaceKeyDto,
    pub a_role: String,
    pub b: FaceKeyDto,
    pub b_role: String,
    pub mode: Option<String>,
    pub preset: Option<String>,
    pub sample_text: Option<String>,
    #[ts(type = "number | null")]
    pub collection_id: Option<i64>,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct MatchedTermDto {
    pub id: String,
    pub text: String,
    pub group: String,
    pub subjectivity: String,
    pub culture: String,
    pub rationale_vi: String,
    pub rationale_en: String,
    pub explanation_banned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DescribeConflictDto {
    pub axis: String,
    pub term_a: String,
    pub term_b: String,
    pub range_a: Option<[f32; 2]>,
    pub range_b: Option<[f32; 2]>,
}

/// 1-1 với `Parsed` của fontatlas-describe; `filters` gửi thẳng vào `query_faces.axisFilters`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DescribeDto {
    pub dictionary_version: String,
    pub matched: Vec<MatchedTermDto>,
    pub filters: Vec<AxisFilterDto>,
    /// "vi_supported" | "vi_complete"
    pub hard: Vec<String>,
    pub conflicts: Vec<DescribeConflictDto>,
    pub unmappable: Vec<(String, String)>,
    pub partial: Vec<(String, String)>,
    pub unknown_words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FileInfoDto {
    pub format: String,
    pub container: Option<String>,
    /// "glyf" | "cff" | "cff2" | "none"
    pub outline: String,
    pub num_glyphs: u32,
    #[ts(type = "number")]
    pub size_bytes: u64,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct VariableAxisDto {
    pub tag: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FeaturesDto {
    pub gsub: Vec<String>,
    pub gpos: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct MetadataDto {
    pub license: Option<String>,
    pub license_url: Option<String>,
    pub designer: Option<String>,
    pub manufacturer: Option<String>,
    pub version: Option<String>,
    pub copyright: Option<String>,
    pub vendor_url: Option<String>,
    pub variable_axes: Vec<VariableAxisDto>,
    pub named_instances: Vec<crate::dto::library::InstanceDto>,
    pub scripts: Vec<String>,
    pub opsz_range: Option<[f32; 2]>,
    pub features: FeaturesDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct VietnameseDto {
    pub status: String,
    pub supports: bool,
    pub missing: Vec<String>,
    /// Tổng số ký tự tiếng Việt cần có (để vẽ thanh X/N).
    pub total: u32,
    pub precomposed_composite: bool,
    pub gpos_mark: bool,
    pub gpos_mkmk: bool,
    pub gsub_ccmp: bool,
    pub dong_sign: bool,
    pub stacked_marks_top_em: Option<f32>,
    pub stacked_marks_overflow: bool,
    pub stack_style: Option<String>,
    pub mark_weight_ratio: AxisReadingDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FaceDetailDto {
    pub row: FaceRowDto,
    pub summary: String,
    /// Sans · Serif · Mono: dòng dưới tên.
    pub classification: Option<String>,
    /// Phân loại đầy đủ: class, subclass, độ tin cậy, lý do, tag dùng, nguồn (tự động / ghi đè).
    pub classification_info: ClassificationDto,
    /// "axes" | "axes_and_image"
    pub source: String,
    pub has_image: bool,
    pub file: FileInfoDto,
    pub metadata: MetadataDto,
    pub vietnamese: VietnameseDto,
    pub axes: Vec<AxisReadingDto>,
    pub similar_top: Vec<SimilarHitDto>,
    /// Dữ liệu D2 (vendor, năm, nhóm feature, kerning, cấu trúc style, fsType). Rỗng với phân tích cũ (chưa chạy lại).
    pub extras: FaceExtrasDto,
}

// ---- phân loại (Font Desk D1) ----

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClassSignalDto {
    /// "measure" | "panose" | "os2_class" | "name" | "cmap" | "post"
    pub source: String,
    pub detail: String,
    /// class mà tín hiệu này bỏ phiếu (snake_case).
    pub vote: String,
    pub weight: f32,
}

/// Kết quả phân loại hai tầng. `source = "override"` khi người dùng đã ghi đè (khi đó `signals` vẫn là của
/// bản tự động, để giải thích vì sao máy nghĩ khác).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClassificationDto {
    /// serif | slab | sans | mono | script | handwriting | display | blackletter | symbol | uncertain (taxonomy v2)
    pub class: String,
    /// Nhãn hiển thị tiếng Anh (thuật ngữ font giữ tiếng Anh): "Serif", "Uncertain"…
    pub label: String,
    /// snake_case, `null` khi chưa đủ chắc.
    pub subclass: Option<String>,
    pub confidence: f32,
    /// Khi `class = "uncertain"` vì độ tin cậy thấp: lớp máy đoán (để hiện "có lẽ là …"); ngược lại `null`.
    pub candidate: Option<String>,
    /// Tag hình thái độc lập với class: stencil | inline_outline | rounded | fat_face | pixel | decorative (và các trait v2 khi có).
    pub traits: Vec<String>,
    pub signals: Vec<ClassSignalDto>,
    /// "text" | "display" | "ui". Khoá `display` hiển thị là **"Headline"** ở UI (tránh nhầm với class `display`); đổi tên chỉ ở nhãn, không đổi khoá.
    pub usage: Vec<String>,
    /// "auto" | "override"
    pub source: String,
    /// Bản tự động khi đang bị ghi đè (để UI hiện "Máy nghĩ: …"); `null` nếu không ghi đè.
    pub auto_class: Option<String>,
    /// Class phụ (hybrid) chỉ để hiển thị/tìm kiếm, vd. Display/stencil có phụ Sans; lọc/nhóm luôn theo `class`. `null` nếu không lai.
    pub secondary: Option<ClassSecondaryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClassSecondaryDto {
    pub class: String,
    pub subclass: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClassificationOverrideDto {
    pub face: FaceKeyDto,
    pub class: String,
    pub subclass: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SubclassFacetDto {
    pub subclass: String,
    pub count: u32,
}

/// Facet đếm cho bộ lọc phân loại ở Library (đã tính ghi đè của người dùng).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ClassFacetDto {
    pub class: String,
    pub count: u32,
    pub subclasses: Vec<SubclassFacetDto>,
}

// ---- dữ liệu D2 (Compare): vendor, năm, nhóm OpenType feature, kerning, cấu trúc family, fsType ----

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct YearsDto {
    /// `head.created` (năm), `None` khi 0/không hợp lý.
    pub created: Option<u16>,
    /// `head.modified`.
    pub modified: Option<u16>,
    /// Năm sớm nhất tìm thấy trong chuỗi copyright (1900–2099).
    pub copyright: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct StylisticSetDto {
    /// "ss01"…"ss20"
    pub tag: String,
    /// Tên trong bảng `name` (FeatureParams.UINameID); `None` nếu font không khai.
    pub name: Option<String>,
}

/// Nhóm feature dễ đọc. `group` ∈ figures | small_caps | fractions | case | ligatures | swash | stylistic_sets | other.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OtGroupDto {
    pub group: String,
    /// Các tag thuộc nhóm có trong font (đã sort).
    pub tags: Vec<String>,
    /// Chỉ có ở `stylistic_sets`: tên từng ssXX.
    pub sets: Vec<StylisticSetDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct KerningDto {
    /// "gpos" | "kern" | "none"
    pub source: String,
    /// Số cặp glyph khai tường minh (GPOS PairPos format 1 / bảng kern).
    pub pairs: u32,
    /// Số ô ma trận lớp có giá trị khác 0 (PairPos format 2; mỗi ô áp cho nhiều cặp glyph). Khác đơn vị với `pairs`: không cộng hai số.
    pub class_pairs: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct StyleStructureDto {
    /// "upright" | "italic" | "oblique"
    pub italic_kind: String,
    /// Có trục `ital` / `slnt`.
    pub ital_axis: bool,
    pub slnt_axis: bool,
    pub wght_range: Option<[f32; 2]>,
    pub wdth_range: Option<[f32; 2]>,
    pub opsz_range: Option<[f32; 2]>,
}

/// Diễn giải OS/2 fsType (bit nhúng font).
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EmbeddingDto {
    pub fs_type: Option<u16>,
    /// "installable" | "restricted" | "preview_print" | "editable" | "unknown"
    pub level: String,
    /// "no_subsetting" | "bitmap_only"
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FaceExtrasDto {
    /// `false` khi face được phân tích trước core-0.8.0 (chưa phân tích lại): mọi trường dưới đây chỉ là mặc định,
    /// KHÔNG phải "font không có" (vd kerning.source = "none"). UI nên hiện "chưa phân tích lại".
    pub extras_available: bool,
    /// OS/2 achVendID, đã bỏ khoảng trắng.
    pub vendor_id: Option<String>,
    pub years: YearsDto,
    pub ot_groups: Vec<OtGroupDto>,
    pub kerning: KerningDto,
    pub style: StyleStructureDto,
    pub embedding: EmbeddingDto,
    /// "ofl" | "apache" | "ufl" | "eula" | "other" | "unknown" — chỉ để ghi chú, không chặn.
    pub license_kind: String,
}

// ---- Compare ----

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CompareOptsDto {
    pub lang: LangDto,
}

/// Một số đo của một face, kèm chênh lệch so với face mốc. Không có điểm tổng.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CompareAxisDto {
    pub key: String,
    pub unit: Option<String>,
    pub raw_value: Option<f64>,
    /// "measured" | "estimated" | "failed"
    pub quality: String,
    /// `quality == "estimated"` (UI hiện dấu ≈).
    pub estimated: bool,
    pub percentile: Option<f32>,
    /// Chênh lệch tuyệt đối so với mốc (cùng đơn vị); `None` ở mốc hoặc khi một bên thiếu số.
    pub delta: Option<f64>,
    /// (giá trị − mốc) / |mốc| × 100; `None` ở mốc, thiếu số, khi |mốc| < 1e-3, và với trục đơn vị "deg" (góc: chỉ dùng `delta`).
    pub delta_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CompareVietnameseDto {
    pub supports: bool,
    pub missing: u32,
    pub total: u32,
    /// Đỉnh chữ hoa có dấu chồng (em).
    pub stacked_marks_top_em: Option<f32>,
    pub hhea_ascender_em: Option<f32>,
    pub cap_height_em: Option<f32>,
    /// Dấu chồng vượt hhea.ascender (dễ va dòng), theo % em; 0 khi không vượt.
    pub over_ascender_pct_em: Option<f32>,
    /// Dấu chồng vượt cap height, theo % em (luôn ≥ 0): mức "cao hơn chữ hoa".
    pub over_cap_pct_em: Option<f32>,
    pub overflow: bool,
    pub stack_style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LicenseNoteDto {
    pub kind: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub embedding: EmbeddingDto,
}

/// Cấu trúc family trong thư viện (từ các face cùng họ đã quét).
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FamilyStructureDto {
    pub family: String,
    pub faces: u32,
    pub weights: Vec<u16>,
    pub widths: Vec<u16>,
    pub has_italic: bool,
    pub has_oblique: bool,
    pub has_variable: bool,
    pub has_opsz: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CompareFaceDto {
    pub face: FaceKeyDto,
    pub row: FaceRowDto,
    pub is_baseline: bool,
    pub classification: ClassificationDto,
    pub axes: Vec<CompareAxisDto>,
    pub vietnamese: CompareVietnameseDto,
    pub license: LicenseNoteDto,
    pub extras: FaceExtrasDto,
    pub family: FamilyStructureDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CompareDto {
    pub baseline: FaceKeyDto,
    /// Theo thứ tự đầu vào; phần tử đầu là mốc.
    pub faces: Vec<CompareFaceDto>,
}

// ---- Pair these ----

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PairTheseOptsDto {
    pub project_vi: bool,
    pub lang: LangDto,
    /// 1..=3 (mặc định 3)
    pub max_combos: u32,
}

/// Một Combo: font nào đặt vai heading, font nào đặt vai body, và đánh giá cặp (nhãn ●◐◆○, cổng, giải thích).
/// Không có điểm số.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PairComboDto {
    pub heading: FaceRowDto,
    pub body: FaceRowDto,
    /// `face` = body; `row` = body; nhãn/va chạm/câu giải thích tính theo (heading, body).
    pub assessment: PairCandidateDto,
    /// Thứ hạng 1-based sau khi xếp lexicographic.
    pub rank: u32,
}

// ---- Trùng lặp và phiên bản (D2) — chỉ báo, không xoá/di chuyển file nào ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum DuplicateKindDto {
    /// Cùng nội dung (BLAKE3) ở nhiều đường dẫn.
    SameFile,
    /// Cùng PostScript name, khác nội dung: các phiên bản của một font.
    Versions,
    /// Cùng font ở các định dạng khác nhau (TTF/OTF/WOFF…).
    Formats,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DuplicateMemberDto {
    /// Với `same_file` của tệp TTC/OTC: `row` là face đầu tiên của tệp (một nhóm cho cả tệp).
    pub row: FaceRowDto,
    /// Với `same_file`: mỗi thành viên là một đường dẫn của cùng nội dung.
    pub path: Option<String>,
    /// ttf | otf | ttc | otc | woff | woff2 | sfnt
    pub format: String,
    /// Chuỗi phiên bản (name ID 5), ví dụ "Version 2.137".
    pub version: Option<String>,
    /// head.fontRevision — CHỈ điền cho nhóm `versions` (đọc lười từ tệp; null khi tệp mất / iCloud chưa tải / WOFF).
    pub revision: Option<f32>,
    /// head.modified (unix giây UTC) — như `revision`.
    #[ts(type = "number | null")]
    pub modified: Option<i64>,
    /// Chỉ `versions`: bản mới nhất khi phân biệt được rõ (fontRevision, rồi head.modified); ngược lại false.
    pub is_newest: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DuplicateGroupDto {
    pub kind: DuplicateKindDto,
    /// same_file: hash hex · versions: PostScript name · formats: "family|style|weight|italic".
    pub key: String,
    pub faces: Vec<DuplicateMemberDto>,
    /// `versions`: có đúng một bản mới nhất xác định được (nếu false thì không face nào có `isNewest`).
    pub newest_known: bool,
}

/// Số nhóm theo loại, cho sidebar ("Trùng lặp" N).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DuplicateSummaryDto {
    pub same_file: u32,
    pub versions: u32,
    pub formats: u32,
    pub total: u32,
}
