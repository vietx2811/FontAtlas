//! DTO của Case và Combo (user.db v6) — Font Desk.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::dto::library::{FaceKeyDto, FaceQueryDto, FaceRowDto};

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

#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct NewComboDto {
    pub name: Option<String>,
    pub slots: Vec<NewSlotDto>,
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

// ---- Compare Combo ----

/// Quan hệ giữa giọng ① (vai đầu tiên theo thứ tự display → quote) và một vai khác trong Combo.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ComboRelationDto {
    pub from: ComboRoleDto,
    pub to: ComboRoleDto,
    /// Đánh giá cặp (nhãn ●◐◆○, Δ theo trục, cổng, câu giải thích) — `assessment.face` là font của vai `to`.
    pub assessment: crate::dto::analysis::PairCandidateDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ComboColumnDto {
    pub combo: ComboDto,
    /// Rỗng khi Combo có < 2 font khác nhau hoặc font không còn trong thư viện/chưa phân tích.
    pub relations: Vec<ComboRelationDto>,
    /// Số font khác nhau.
    pub font_count: u32,
    /// Số ký tự tiếng Việt thiếu lớn nhất trong các font (VI "thấp nhất"); `None` khi không có font nào còn trong thư viện.
    pub worst_vi_missing: Option<u32>,
    /// Các lớp license khác nhau (ofl, apache, other, unknown).
    pub licenses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct CaseCompareDto {
    #[ts(type = "number")]
    pub case_id: i64,
    pub columns: Vec<ComboColumnDto>,
}

// ---- Pair from peek ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum PeekRoleDto {
    /// "Font này là tiêu đề" → vai `display`
    Heading,
    /// "Font này là nội dung" → vai `body`
    Body,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PeekOpenDto {
    pub face: FaceKeyDto,
    #[serde(rename = "as")]
    #[ts(rename = "as")]
    pub as_role: PeekRoleDto,
    /// Ứng viên đang xem trong tab ∞ — thành lựa chọn đầu tiên của vai còn lại, chưa đặt.
    pub candidate: Option<FaceKeyDto>,
    /// null = Case đang mở gần nhất, chưa có thì tạo mới.
    #[ts(type = "number | null")]
    pub case_id: Option<i64>,
    pub settings: CaseSettingsDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PeekOpenResultDto {
    pub case: CaseDto,
    #[ts(type = "number")]
    pub combo_id: i64,
    pub anchor_role: ComboRoleDto,
    /// Vai còn lại để Cuộn chọn mở vào (body ↔ display).
    pub other_role: ComboRoleDto,
    /// Echo `candidate` để giao diện đặt vào trạng thái "thử" của `other_role`.
    pub trial: Option<FaceKeyDto>,
    /// true khi Case vừa được tạo.
    pub created: bool,
}

// ---- Brief starter ----

/// Ghi chú máy sinh cho giao diện (giao diện dịch theo `code`, `detail` là dữ liệu thô).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct BriefNoteDto {
    /// `no_terms` · `branding_class_only` · `installed_none_active` · `weight_relaxed` · `few_anchors`
    /// · `no_body_pair` · `index_unavailable` · `langs_incomplete` (độ phủ ngôn ngữ còn đang tính nền: có thể thiếu kết quả) · `langs_incomplete` (độ phủ ngôn ngữ còn đang tính nền: kết quả có thể thiếu)
    pub code: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct BriefStarterDto {
    /// Các term của `descriptor-dictionary.json` mà brief đã ánh xạ tới, kèm bộ lọc trục, xung đột, phần không ánh xạ được.
    pub describe: crate::dto::analysis::DescribeDto,
    /// Điều kiện chung (đưa thẳng cho Library: `axisFilters`, `hard`, `langs`, `license`…), sắp theo `match`.
    pub query: FaceQueryDto,
    /// `query` + `weightClass` 600–900 (vai display/heading), `varAxisMatch` bật.
    pub display_query: FaceQueryDto,
    /// `query` + `weightClass` 400–500 (vai body).
    pub body_query: FaceQueryDto,
    /// ≤ 3 font neo cho vai display, khác family.
    pub anchors: Vec<FaceRowDto>,
    /// ≤ 3 Combo khởi đầu: mỗi neo ghép với font body hạng 1 (ưu tiên khác family, không lặp body).
    pub combos: Vec<crate::dto::analysis::PairComboDto>,
    pub notes: Vec<BriefNoteDto>,
    /// Chừa cho "Kết nối AI" (session sau).
    #[ts(type = "unknown")]
    pub ai: Option<Value>,
}
