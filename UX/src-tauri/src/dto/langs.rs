//! DTO ngôn ngữ (Hyperglot): bộ lọc, chỉ báo trên thẻ, danh sách ngôn ngữ, chi tiết ký tự thiếu.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum LangLevelDto {
    Full,
    Near,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LangFilterDto {
    /// Khoá ngôn ngữ: ISO 639-3 (`vie`), orthography thứ hai của cùng ngôn ngữ: `srp-Latin`.
    pub code: String,
    /// `full` = đủ 100% base + marks; `near` = ≥ 90% base, đủ marks và đủ chữ cái riêng (chấp nhận cả full).
    pub level: LangLevelDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum LangModeDto {
    #[default]
    All,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum LangBadgeStateDto {
    Full,
    Near,
    /// 25–90% base (chỉ biết chính xác với `vie`; ngôn ngữ khác gọi `lang_coverage`).
    Partial,
    /// < 25% base.
    Low,
    /// Chưa tính, hoặc dưới mức near ở ngôn ngữ không phải `vie`.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LangBadgeDto {
    pub code: String,
    /// Mã cho `Intl.DisplayNames`.
    pub bcp47: String,
    pub state: LangBadgeStateDto,
    pub have: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LangReadyDto {
    #[ts(type = "number")]
    pub done: u64,
    #[ts(type = "number")]
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LangItemDto {
    pub code: String,
    pub bcp47: String,
    pub name_en: String,
    pub autonym: String,
    pub script: String,
    /// Số face đủ 100% (trong tập lọc hiện tại, bỏ điều kiện `langs`; chỉ face đã tính).
    pub full: u32,
    /// Số face đạt ít nhất `near` (gồm cả `full`).
    pub near: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LangListDto {
    /// `hg-<sha8>-<n>`: phiên bản dữ liệu ngôn ngữ.
    pub db_version: String,
    /// `false` khi còn face chưa tính độ phủ: số đếm chỉ là số đã tính.
    pub complete: bool,
    pub ready: LangReadyDto,
    /// Chỉ ngôn ngữ có ≥ 1 face (đã tính) trong tập lọc, theo mã.
    pub items: Vec<LangItemDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum LangCoverageLevelDto {
    Full,
    Near,
    Partial,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LangCoverageDto {
    pub code: String,
    pub level: LangCoverageLevelDto,
    pub base_have: u32,
    pub base_total: u32,
    pub marks_have: u32,
    pub marks_total: u32,
    /// Grapheme `base` thiếu (tối đa 200).
    pub missing: Vec<String>,
    /// Combining mark thiếu (tối đa 200).
    pub missing_marks: Vec<String>,
    /// `true` nếu `missing`/`missingMarks` bị cắt ở 200 (UI ghi "+N" theo `baseTotal - baseHave`).
    pub truncated: bool,
}
