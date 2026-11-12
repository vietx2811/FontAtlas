//! DTO gợi ý cho Pairing Desk (G4–G6): mức phù hợp + mã lý do, không điểm số.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dto::library::{FaceKeyDto, FaceQueryDto, FaceRowDto};

/// Mức phù hợp. Không có số điểm trong API (cố ý, xem `docs/team/plans/2026-11-12-desk-suggest-api-contract.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum FitDto {
    Unknown,
    Weak,
    Ok,
    Good,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct WhyParamDto {
    pub key: String,
    pub value: f32,
}

/// Mã lý do + tham số số. Backend không trả câu chữ: frontend dịch vi/en (khoá `why.<code>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct WhyDto {
    pub code: String,
    pub params: Vec<WhyParamDto>,
}

/// Cỡ chữ tương đối của một vai tuỳ ý (G3) khi tên vai không nhận ra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum RoleSizeDto {
    Text,
    Heading,
    Display,
}

/// Cách gọi một vai. Thứ tự giải: tên vai (đồng nghĩa) → `profile` (id trong 7 hồ sơ: body, caption, heading, display, quote,
/// label, numerals) → `size` → không nhận ra thì `fit = unknown` + lý do `role_unknown`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RoleRefDto {
    pub name: String,
    pub profile: Option<String>,
    pub size: Option<RoleSizeDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RoleFitDto {
    pub fit: FitDto,
    /// Hồ sơ vai thực sự dùng (id), `None` khi không nhận ra vai.
    pub profile: Option<String>,
    /// Tối đa 3, cổng đứng đầu.
    pub reasons: Vec<WhyDto>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct RoleFitListOptsDto {
    /// Chỉ font có đủ tiếng Việt.
    pub project_vi: bool,
    /// Số kết quả, 1–100 (mặc định 24).
    pub limit: u32,
    pub offset: u32,
    /// Giới hạn phạm vi (class, collection, Case…): dùng chung bộ lọc thư viện.
    pub filter: Option<FaceQueryDto>,
}

impl Default for RoleFitListOptsDto {
    fn default() -> Self {
        RoleFitListOptsDto { project_vi: false, limit: 24, offset: 0, filter: None }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RoleFitHitDto {
    pub face: FaceKeyDto,
    pub row: FaceRowDto,
    pub fit: FitDto,
    pub reasons: Vec<WhyDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RoleFitPageDto {
    pub profile: Option<String>,
    pub items: Vec<RoleFitHitDto>,
    /// Số font đạt good/ok cho vai này (trước phân trang).
    #[ts(type = "number")]
    pub total: u64,
}

/// Một dòng lịch sử xem.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RecentViewDto {
    pub face: FaceKeyDto,
    pub row: Option<FaceRowDto>,
    #[ts(type = "number")]
    pub viewed_at: i64,
}
