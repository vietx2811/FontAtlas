//! DTO của Case và Combo (user.db v6) — Font Desk.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::dto::library::{FaceKeyDto, FaceRowDto};

/// Vai trong một Combo. Giọng ①②③ do giao diện suy ra từ vai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ComboRoleDto {
    Display,
    Heading,
    Subheading,
    Body,
    Caption,
    Accent,
    Quote,
}

impl ComboRoleDto {
    pub fn as_str(self) -> &'static str {
        match self {
            ComboRoleDto::Display => "display",
            ComboRoleDto::Heading => "heading",
            ComboRoleDto::Subheading => "subheading",
            ComboRoleDto::Body => "body",
            ComboRoleDto::Caption => "caption",
            ComboRoleDto::Accent => "accent",
            ComboRoleDto::Quote => "quote",
        }
    }

    pub fn parse(s: &str) -> Option<ComboRoleDto> {
        [
            ComboRoleDto::Display,
            ComboRoleDto::Heading,
            ComboRoleDto::Subheading,
            ComboRoleDto::Body,
            ComboRoleDto::Caption,
            ComboRoleDto::Accent,
            ComboRoleDto::Quote,
        ]
        .into_iter()
        .find(|r| r.as_str() == s)
    }
}

/// Font vào Case bằng cách nào.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum CaseOriginDto {
    Added,
    Pinned,
    Brief,
    Peek,
}

impl CaseOriginDto {
    pub fn as_str(self) -> &'static str {
        match self {
            CaseOriginDto::Added => "added",
            CaseOriginDto::Pinned => "pinned",
            CaseOriginDto::Brief => "brief",
            CaseOriginDto::Peek => "peek",
        }
    }
}

/// Mức ghép đang dùng khi mở Case (mang theo từ side peek).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum PairingLevelDto {
    Safe,
    Controlled,
    Experimental,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct CaseSettingsDto {
    /// Tính thêm tiếng Việt khi xếp hạng và lọc.
    pub vi: bool,
    pub mode: Option<PairingLevelDto>,
}

impl Default for CaseSettingsDto {
    fn default() -> Self {
        CaseSettingsDto { vi: true, mode: None }
    }
}

// ---- Brief ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum BriefUseDto {
    Editorial,
    Branding,
    Poster,
    UiWeb,
    Packaging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum TonePairDto {
    /// −1 = Cổ điển (classic) · +1 = Đương đại (modern)
    ClassicModern,
    /// −1 = Ấm (warm) · +1 = Lạnh (cool)
    WarmCool,
    /// −1 = Trầm (delicate) · +1 = Nổi bật (strong)
    QuietBold,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ToneDto {
    pub pair: TonePairDto,
    /// −1.0 … +1.0; |v| < 0.25 coi như chưa chọn.
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct BriefNeedsDto {
    pub vi: bool,
    pub installed_only: bool,
    pub italic: bool,
    pub variable: bool,
    /// License mở (ofl/apache) — app chỉ phân biệt được hai lớp này.
    pub open_license: bool,
}

impl Default for BriefNeedsDto {
    fn default() -> Self {
        BriefNeedsDto { vi: true, installed_only: false, italic: false, variable: false, open_license: false }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct BriefDto {
    pub uses: Vec<BriefUseDto>,
    pub tones: Vec<ToneDto>,
    pub needs: BriefNeedsDto,
    /// Từ khoá tự do (qua `describe`).
    pub text: Option<String>,
}

// ---- Case ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CaseSummaryDto {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub face_count: u32,
    pub combo_count: u32,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub opened_at: i64,
    #[ts(type = "number | null")]
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CaseFaceDto {
    pub key: FaceKeyDto,
    pub origin: CaseOriginDto,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub added_at: i64,
    /// `None` khi font không còn trong thư viện (file bị xoá khỏi mọi nguồn).
    pub row: Option<FaceRowDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ComboSlotDto {
    pub role: ComboRoleDto,
    pub key: FaceKeyDto,
    pub row: Option<FaceRowDto>,
    pub note: Option<String>,
    /// weight, italic, case, size_step, tracking, leading… do giao diện định nghĩa.
    #[ts(type = "unknown")]
    pub style: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ComboDto {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub case_id: i64,
    /// 1-based, liên tục.
    pub idx: u32,
    pub name: String,
    pub is_main: bool,
    pub note: Option<String>,
    pub slots: Vec<ComboSlotDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CaseDto {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub brief: Option<BriefDto>,
    pub settings: CaseSettingsDto,
    /// Trạng thái giao diện của Pairing/Compare (cột, ghim, sắp xếp, nhóm đóng…); backend chỉ lưu.
    #[ts(type = "unknown")]
    pub view_state: Value,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub opened_at: i64,
    #[ts(type = "number | null")]
    pub deleted_at: Option<i64>,
    pub faces: Vec<CaseFaceDto>,
    pub combos: Vec<ComboDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct CasePatchDto {
    pub name: Option<String>,
    pub brief: Option<BriefDto>,
    /// true = xoá brief (ưu tiên hơn `brief`).
    pub clear_brief: bool,
    pub settings: Option<CaseSettingsDto>,
    #[ts(type = "unknown")]
    pub view_state: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct ComboPatchDto {
    pub name: Option<String>,
    pub is_main: Option<bool>,
    pub note: Option<String>,
    /// true = xoá ghi chú (ưu tiên hơn `note`).
    pub clear_note: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct NewSlotDto {
    pub role: ComboRoleDto,
    pub face: FaceKeyDto,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct NewComboDto {
    pub name: Option<String>,
    pub slots: Vec<NewSlotDto>,
}

impl Default for NewComboDto {
    fn default() -> Self {
        NewComboDto { name: None, slots: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AddToCaseDto {
    #[ts(type = "number")]
    pub case_id: i64,
    pub added: u32,
    pub already_in: u32,
    /// true khi lệnh phải tạo Case mới (chưa có Case nào đang mở).
    pub created: bool,
}
