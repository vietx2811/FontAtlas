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
    /// serif | sans | mono | script | display | blackletter | symbol | uncertain
    pub class: String,
    /// Nhãn hiển thị tiếng Anh (thuật ngữ font giữ tiếng Anh): "Serif", "Uncertain"…
    pub label: String,
    /// snake_case, `null` khi chưa đủ chắc.
    pub subclass: Option<String>,
    pub confidence: f32,
    pub signals: Vec<ClassSignalDto>,
    /// "text" | "display" | "ui"
    pub usage: Vec<String>,
    /// "auto" | "override"
    pub source: String,
    /// Bản tự động khi đang bị ghi đè (để UI hiện "Máy nghĩ: …"); `null` nếu không ghi đè.
    pub auto_class: Option<String>,
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
