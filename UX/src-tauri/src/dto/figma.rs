//! DTO nhập font từ Figma (B6). Hợp đồng: `docs/team/plans/2026-11-13-resolve-fonts-contract.md`.
//! Không điểm số: chỉ `status` và thứ tự. Backend không trả câu chữ (frontend dịch theo `status`/mã lỗi).
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dto::library::FaceKeyDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FontNameDto {
    pub family: String,
    pub style: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FigmaFontItemDto {
    pub family: String,
    pub style: String,
    /// Figma báo font này không khả dụng trong máy người dùng.
    pub figma_missing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FigmaImportDto {
    /// Tên file Figma (riêng tư, tuỳ chọn; giao diện cho người dùng bỏ).
    pub file: Option<String>,
    pub items: Vec<FigmaFontItemDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum MatchStatusDto {
    /// Đúng một face khớp family + style.
    Exact,
    /// Từ hai face trở lên khớp (nhiều bản/nguồn): người dùng chọn, không đoán.
    Ambiguous,
    /// Có họ nhưng không có style này; `faces` là các style của họ.
    FamilyOnly,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FaceMatchDto {
    pub face: FaceKeyDto,
    pub family: String,
    pub style: String,
    /// File nguồn đang thiếu trên đĩa.
    pub missing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct FontMatchDto {
    pub index: u32,
    pub status: MatchStatusDto,
    /// Đã xếp hạng, tối đa 5 (xem `FACES_MAX`).
    pub faces: Vec<FaceMatchDto>,
}
