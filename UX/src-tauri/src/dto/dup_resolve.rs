//! DTO "Giải quyết trùng lặp" (0.3.0). Hợp đồng cho UI: docs/team/contracts/2026-11-11-duplicates-resolve.md.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dto::analysis::DuplicateKindDto;
use crate::dto::library::FaceKeyDto;

/// Mã + tham số để UI dựng một câu thường (i18n). `code` thuộc tập cố định, xem hợp đồng.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ReasonDto {
    pub code: String,
    #[ts(type = "Record<string, string>")]
    pub args: BTreeMap<String, String>,
}

/// Phạm vi xem trước.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case", tag = "type")]
pub enum ResolveScopeDto {
    /// Một nhóm (kể cả khi máy không tự quyết: trả quyết định mặc định + `review`).
    Group { kind: DuplicateKindDto, key: String },
    /// Mọi nhóm AN TOÀN (không cần người dùng xem), theo bộ lọc hiện tại của trang.
    AllSafe,
    /// Mọi nhóm (gồm cả nhóm cần xem), để hiện danh sách đầy đủ.
    All,
}

/// Được phép chuyển vào Thùng rác không.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct TrashEligibilityDto {
    pub allowed: bool,
    /// true: chỉ được khi người dùng chọn rõ TỪNG tệp này (`explicitTrashPaths`), vd ~/Library/Fonts.
    pub needs_explicit: bool,
    /// managed_source | system_path | collection_file | missing_file (khi `allowed` = false)
    pub blocked: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PlanMemberDto {
    pub face: FaceKeyDto,
    pub family: String,
    pub style: Option<String>,
    pub path: String,
    /// ttf | otf | ttc | otc | woff | woff2 | sfnt
    pub format: String,
    pub version: Option<String>,
    pub revision: Option<f32>,
    #[ts(type = "number | null")]
    pub modified: Option<i64>,
    pub num_glyphs: Option<u32>,
    #[ts(type = "number")]
    pub file_size: u64,
    /// Số ký tự tiếng Việt còn thiếu (0 = đủ).
    pub vi_missing: u32,
    pub is_variable: bool,
    #[ts(type = "number")]
    pub source_id: i64,
    pub source_name: String,
    /// directory | system | google_fonts | adobe_fonts | community
    pub source_kind: String,
    /// Đang được bật trên máy.
    pub active: bool,
    pub trash: TrashEligibilityDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PlanItemDto {
    pub kind: DuplicateKindDto,
    pub key: String,
    pub keeper: PlanMemberDto,
    pub losers: Vec<PlanMemberDto>,
    /// Vì sao giữ bản này ("Bản mới hơn: 2.137 so với 2.001").
    pub reason: ReasonDto,
    /// Tự động an toàn: có trong "Giải quyết tất cả an toàn".
    pub auto_safe: bool,
    /// Vì sao máy KHÔNG tự quyết (khi `autoSafe` = false).
    pub review: Option<ReasonDto>,
    /// Lưu ý không chặn (vd `may_reflow`).
    pub warnings: Vec<ReasonDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResolvePlanDto {
    pub items: Vec<PlanItemDto>,
    /// Số nhóm cần người dùng xem (không nằm trong `items` khi scope = all_safe).
    pub needs_review: u32,
    /// Số bản sẽ ẩn / chuyển đi, số bản giữ.
    pub hide_count: u32,
    pub keep_count: u32,
    /// Tổng dung lượng các bản thua (nếu chuyển vào Thùng rác).
    #[ts(type = "number")]
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ResolveActionDto {
    /// Ẩn khỏi thư viện + không cho bật. KHÔNG đụng file.
    Hide,
    /// Như Hide + chuyển file vào Thùng rác (hoàn tác được). Không bao giờ xoá vĩnh viễn.
    Trash,
}

/// Một nhóm cần giải quyết: người dùng có thể đổi `keeperPath`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResolveItemDto {
    pub kind: DuplicateKindDto,
    pub key: String,
    pub keeper_path: String,
    pub loser_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResolveRequestDto {
    pub items: Vec<ResolveItemDto>,
    pub action: ResolveActionDto,
    /// Với `trash`: các tệp người dùng đã chọn rõ để chuyển dù nằm ở nơi cần xác nhận riêng (vd ~/Library/Fonts).
    #[serde(default)]
    pub explicit_trash_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResolutionDto {
    #[ts(type = "number")]
    pub id: i64,
    pub batch_id: String,
    pub kind: DuplicateKindDto,
    pub group_key: String,
    pub keeper: FaceKeyDto,
    pub keeper_path: String,
    pub loser: FaceKeyDto,
    pub loser_path: String,
    pub action: ResolveActionDto,
    /// Tệp đang nằm ở đâu trong Thùng rác (chỉ khi `trash`).
    pub trash_path: Option<String>,
    pub reason: ReasonDto,
    pub was_active: bool,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub undone_at: Option<i64>,
    /// Số liên kết (yêu thích, collection, tag, Case, cặp, slot) đã chuyển sang bản giữ.
    pub links_moved: u32,
    /// Tên hiển thị của bản thua, để liệt kê trong "Đã giải quyết".
    pub family: Option<String>,
    pub style: Option<String>,
    /// Có thể hoàn tác (chưa hoàn tác; nếu `trash` thì tệp vẫn còn trong Thùng rác).
    pub can_undo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResolveFailureDto {
    pub path: String,
    /// keeper_missing | keeper_not_in_group | loser_not_in_group | not_allowed | collection_file | active_copy_in_use |
    /// trash_failed | not_found | already_undone | path_occupied | trash_emptied | db_error
    pub code: String,
    /// Chẩn đoán thô, KHÔNG dịch, không hiện thẳng cho người dùng (UI dịch theo `code`): lỗi hệ điều hành / DB cho
    /// `trash_failed`, `db_error`; với `not_allowed` là lý do chặn (`managed_source` | `system_path` | `collection_file` |
    /// `missing_file` | `needs_explicit`).
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResolveResultDto {
    pub batch_id: String,
    pub done: Vec<ResolutionDto>,
    pub failed: Vec<ResolveFailureDto>,
    /// Sau hoàn tác một tệp từ Thùng rác: nên quét lại để thư viện thấy tệp đã về.
    pub needs_rescan: bool,
}
