//! DTO Organized Library (B7): chọn vị trí, chép vào thư viện có tổ chức, "Sắp xếp lại", gói khôi phục.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dto::library::FaceKeyDto;

/// Kiểm một vị trí trước khi tạo Organized Library. Không chặn việc chọn ổ cloud: chỉ cảnh báo (`isCloud`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizedLocationCheckDto {
    pub path: String,
    /// Tạo được thư mục và ghi được file thử.
    pub writable: bool,
    /// Nằm trên ổ cloud (OneDrive/Google Drive/Dropbox/Box/iCloud): nên cảnh báo, thư viện nên ở ổ máy hoặc ổ ngoài.
    pub is_cloud: bool,
    pub cloud_provider: Option<String>,
    #[ts(type = "number")]
    pub free_bytes: u64,
    /// Nằm trong/chứa một nguồn thư mục có sẵn (không tạo được).
    pub nested_source: bool,
    /// `not_a_directory` | `not_writable` | `nested_source` khi không dùng được; `None` = dùng được.
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizedStatusDto {
    pub configured: bool,
    pub root: Option<String>,
    /// `<root>/Library`: thư mục chứa `Class/Họ/file`; cũng là một nguồn thư mục của app.
    pub library_dir: Option<String>,
    #[ts(type = "number | null")]
    pub source_id: Option<i64>,
    /// Số font đã chép (theo sổ).
    pub files: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    #[ts(type = "number")]
    pub free_bytes: u64,
    pub is_cloud: bool,
    /// Thư mục gốc không còn truy cập được (ổ ngoài chưa cắm).
    pub available: bool,
    pub snapshots: u32,
    /// Thư mục gói khôi phục mới nhất.
    pub last_snapshot: Option<String>,
}

/// Phạm vi chép: `sourceIds` rỗng/`null` = mọi font ngoài thư viện có tổ chức.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizeScopeDto {
    #[ts(type = "number[] | null")]
    pub source_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizePlanItemDto {
    pub src: String,
    pub dest: String,
    pub class_folder: String,
}

/// Xem trước việc chép (KHÔNG ghi gì). `sample` tối đa 50 mục đầu để minh hoạ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizePlanDto {
    pub to_copy: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    /// Đã có bản trong thư viện (bỏ qua).
    pub already_there: u32,
    /// Còn là placeholder trên ổ cloud: không chép (tải về máy trước).
    pub cloud_not_downloaded: u32,
    #[ts(type = "number")]
    pub free_bytes: u64,
    /// `freeBytes` đủ cho `bytes` + 5% dự phòng.
    pub enough_space: bool,
    pub sample: Vec<OrganizePlanItemDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizeProgressDto {
    pub done: u32,
    pub total: u32,
    #[ts(type = "number")]
    pub bytes_done: u64,
    pub failed: u32,
}

/// `code`: `cloud_not_downloaded` | `source_missing` | `verify_failed` | `disk_full` | `io`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizeIssueDto {
    pub path: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OrganizeResultDto {
    pub copied: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    pub already_there: u32,
    /// Tối đa 200 mục; `issueCount` là tổng thật.
    pub issues: Vec<OrganizeIssueDto>,
    pub issue_count: u32,
    pub cancelled: bool,
    /// Gói khôi phục chụp ngay sau lần chép (nếu có chép được gì).
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResortMoveDto {
    pub face: FaceKeyDto,
    pub from: String,
    pub to: String,
    pub class_folder: String,
    pub family: String,
}

/// "Sắp xếp lại": chỉ đề xuất (class đổi sau khi đã xếp); không bao giờ tự di chuyển.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResortPlanDto {
    pub total: u32,
    /// Tối đa 200 mục đầu.
    pub moves: Vec<ResortMoveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ResortResultDto {
    #[ts(type = "number")]
    pub batch_id: i64,
    pub moved: u32,
    pub failed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PathRemapDto {
    pub from: String,
    pub to: String,
}

/// Thông tin gói khôi phục (đọc từ manifest, đã kiểm toàn vẹn).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PackInfoDto {
    pub path: String,
    #[ts(type = "number")]
    pub created_at: i64,
    pub app_version: String,
    pub files: u32,
    pub faces: u32,
    pub embeddings: u32,
    pub sources: u32,
    /// Gốc các nguồn lúc xuất: gợi ý để người dùng đổi tiền tố khi đường dẫn đã khác.
    pub source_roots: Vec<String>,
}

/// G7b: kết quả tạo bản sao đổi tên của một VF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct DerivedCopyDto {
    /// Face (index 0) của BẢN SAO; 06 bật cái này cùng bản static.
    pub face: FaceKeyDto,
    pub path: String,
    /// `false` khi bản sao đã có từ trước (idempotent).
    pub created: bool,
    /// `OS/2.fsType` hạn chế: vẫn tạo; UI hiện ghi chú "giấy phép của font hạn chế, bạn chịu trách nhiệm".
    pub license_restricted: bool,
}
