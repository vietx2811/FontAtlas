//! DTO của Case và Combo (user.db v6) — Font Desk.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::dto::library::{FaceKeyDto, FaceQueryDto, FaceRowDto};

/// Vai trong một Combo: chuỗi tuỳ ý (0.4.0 G3), 1–40 ký tự, không phân biệt hoa thường trong cùng Combo.
/// 7 khoá gợi sẵn: `display`, `heading`, `subheading`, `body`, `caption`, `accent`, `quote` (giao diện tự dịch tên hiển thị;
/// vai tuỳ ý hiện nguyên chuỗi). Giọng ①②③ do giao diện suy ra từ vai.
pub type ComboRoleDto = String;

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
    /// `row` vắng hoặc file đang thiếu trên đĩa — giao diện hiện NoteMark "font đã đổi hoặc bị xoá".
    pub missing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ComboSlotDto {
    pub role: ComboRoleDto,
    pub key: FaceKeyDto,
    pub row: Option<FaceRowDto>,
    /// Như `CaseFaceDto.missing`. Hash của file đổi (sửa file) cũng thành `missing`; chưa tự nối lại sang hash mới.
    pub missing: bool,
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
    /// Vai CÓ font (đã đặt). Vai rỗng giữ chỗ chỉ có tên trong `roles`.
    pub slots: Vec<ComboSlotDto>,
    /// Mọi vai của Combo theo thứ tự hiển thị, kể cả vai rỗng giữ chỗ. Rỗng = chưa có vai nào (giao diện tự dùng bộ mặc định).
    pub roles: Vec<String>,
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
    /// `None` khi không đánh giá được; xem `reason`.
    pub assessment: Option<crate::dto::analysis::PairCandidateDto>,
    /// Vì sao `assessment` là `None`: `missing_font` (một trong hai font không còn trong thư viện) ·
    /// `not_analyzed` (chưa có hồ sơ phân tích) · `unavailable` (lỗi khác; `reasonDetail` là thông báo).
    pub reason: Option<String>,
    pub reason_detail: Option<String>,
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
    /// Số vai có font không còn trong thư viện (hoặc file thiếu). > 0 ⇒ `worstViMissing` và `licenses` KHÔNG đủ:
    /// giao diện không được coi là "sạch".
    pub missing_count: u32,
    /// Số ký tự tiếng Việt thiếu lớn nhất trong các font còn trong thư viện; `None` khi không font nào còn.
    pub worst_vi_missing: Option<u32>,
    /// Các lớp license khác nhau của font còn trong thư viện (ofl, apache, other, unknown).
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
    /// Chỉ dùng khi Case được TẠO ở lệnh này; Case có sẵn giữ nguyên cài đặt của nó.
    pub settings: CaseSettingsDto,
    /// true = người dùng đã đồng ý thay font đang giữ vai neo (xem `occupied` của kết quả).
    #[serde(default)]
    pub replace: bool,
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
    /// Vai neo đã có font KHÁC và `replace = false`: không ghi gì vào vai; giao diện hỏi rồi gửi lại với `replace = true`.
    pub occupied: Option<PeekOccupiedDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PeekOccupiedDto {
    pub role: ComboRoleDto,
    pub key: FaceKeyDto,
    pub row: Option<FaceRowDto>,
}

// ---- Brief starter ----

/// Ghi chú máy sinh cho giao diện (giao diện dịch theo `code`, `detail` là dữ liệu thô).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct BriefNoteDto {
    /// `no_terms` · `branding_class_only` · `installed_none_active` · `weight_relaxed` · `few_anchors`
    /// · `no_body_pair` · `index_unavailable` · `installed_truncated` · `body_set_truncated` · `langs_incomplete` (độ phủ ngôn ngữ còn đang tính nền: có thể thiếu kết quả)
    /// · `packaging_inferred` (packaging là ánh xạ suy ra từ term có sẵn, không phải term riêng; `detail` = "display=display+heavy; body=readable_small+clean")
    /// · `packaging_inferred` (packaging là ánh xạ suy ra: `detail` = "display=display+heavy; body=readable_small+clean") · `langs_incomplete` (độ phủ ngôn ngữ còn đang tính nền: kết quả có thể thiếu)
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
