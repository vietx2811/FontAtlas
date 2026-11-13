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

// ---- suggest_partners (G4/G5): font hợp đi cùng một font đang chọn ----

/// Id phong cách pair: contrast | superfamily | same_structure | same_era | display_vs_neutral | harmony.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct SuggestPartnersReqDto {
    pub anchor_role: RoleRefDto,
    pub target_role: RoleRefDto,
    /// Rỗng = mọi phong cách (kết quả ghi phong cách nào khớp).
    pub styles: Vec<String>,
    pub project_vi: bool,
    pub limit: u32,
    pub offset: u32,
    pub filter: Option<FaceQueryDto>,
}

impl Default for SuggestPartnersReqDto {
    fn default() -> Self {
        let r = |n: &str| RoleRefDto { name: n.into(), profile: None, size: None };
        SuggestPartnersReqDto { anchor_role: r("heading"), target_role: r("body"), styles: vec![], project_vi: false, limit: 24, offset: 0, filter: None }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PartnerDto {
    pub face: FaceKeyDto,
    pub row: FaceRowDto,
    /// "safe" | "controlled" | "experimental" (cổng va chạm của Pairs; không có "blocked").
    pub gate: String,
    /// Các phong cách (id) mà cặp này đạt ≥ ok.
    pub styles: Vec<String>,
    pub role_fit: FitDto,
    pub style_fit: FitDto,
    pub reasons: Vec<WhyDto>,
    pub warnings: Vec<WhyDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PartnerPageDto {
    pub items: Vec<PartnerDto>,
    #[ts(type = "number")]
    pub total: u64,
}

// ---- desk_suggest (G5): các nguồn của cột 2 ----

/// `source`: "similar" | "fits_role" | "pinned". (Recent: `recent_views`.)
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct DeskSuggestReqDto {
    pub source: String,
    pub anchor: Option<FaceKeyDto>,
    #[ts(type = "number | null")]
    pub case_id: Option<i64>,
    pub target_role: RoleRefDto,
    pub filter: Option<FaceQueryDto>,
    pub project_vi: bool,
    pub limit: u32,
    pub offset: u32,
}

impl Default for DeskSuggestReqDto {
    fn default() -> Self {
        DeskSuggestReqDto {
            source: "fits_role".into(),
            anchor: None,
            case_id: None,
            target_role: RoleRefDto { name: "body".into(), profile: None, size: None },
            filter: None,
            project_vi: false,
            limit: 24,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DeskItemDto {
    pub face: FaceKeyDto,
    pub row: FaceRowDto,
    /// Mức hợp vai đích (nếu tính được).
    pub fit: Option<FitDto>,
    pub reasons: Vec<WhyDto>,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DeskPageDto {
    pub source: String,
    pub items: Vec<DeskItemDto>,
    #[ts(type = "number")]
    pub total: u64,
}

// ---- suggest_combos (G6) ----

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct LockedSlotDto {
    pub role: RoleRefDto,
    pub face: FaceKeyDto,
}

/// `roles[0]` là vai tiêu đề (neo): tối đa 1 gợi ý mỗi family của vai này. `scope`: "library" (mặc định) | "case".
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct SuggestCombosReqDto {
    pub styles: Vec<String>,
    pub roles: Vec<RoleRefDto>,
    pub locked: Vec<LockedSlotDto>,
    pub scope: String,
    #[ts(type = "number | null")]
    pub case_id: Option<i64>,
    pub project_vi: bool,
    /// 3–6.
    pub k: u32,
    /// Cùng seed + cùng thư viện → cùng kết quả.
    #[ts(type = "number")]
    pub seed: u64,
}

impl Default for SuggestCombosReqDto {
    fn default() -> Self {
        let r = |n: &str| RoleRefDto { name: n.into(), profile: None, size: None };
        SuggestCombosReqDto {
            styles: vec![],
            roles: vec![r("heading"), r("body")],
            locked: vec![],
            scope: "library".into(),
            case_id: None,
            project_vi: false,
            k: 4,
            seed: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ComboSlotSuggestionDto {
    /// Đúng như người gọi gửi (giữ tên vai tuỳ ý).
    pub role: RoleRefDto,
    pub face: FaceKeyDto,
    pub row: FaceRowDto,
    pub fit: FitDto,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ComboSuggestionDto {
    pub slots: Vec<ComboSlotSuggestionDto>,
    /// Phong cách pair tốt nhất của cặp tiêu đề–vai kế (id); `None` nếu chỉ có một vai.
    pub style: Option<String>,
    pub reasons: Vec<WhyDto>,
    /// Cổng va chạm / cảnh báo của các cặp (mã).
    pub warnings: Vec<WhyDto>,
    /// Tổng số ký tự tiếng Việt còn thiếu của các font trong bộ (0 = đủ).
    pub vi_missing: u32,
    #[ts(type = "number")]
    pub seed: u64,
}
