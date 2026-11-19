//! TaskHub: một danh sách việc nền cho khay "Tác vụ" (xem docs/design/taskhub-be-beta5.md). Chỉ để HIỂN THỊ và huỷ.
use serde::Serialize;
use std::collections::BTreeMap;
use ts_rs::TS;

use crate::dto::events::ScanTrigger;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum TaskKind {
    Scan,
    Analyze,
    Embed,
    CloudDownload,
    Activation,
    ResolveDuplicates,
    Export,
    Other,
}

/// Đơn vị của `done`/`total` để FE hiện "3/12", "62%" hay dung lượng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum TaskUnit {
    Items,
    Percent,
    Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum TaskState {
    Queued,
    Running,
    /// Đã yêu cầu huỷ, chờ việc dừng ở ranh giới an toàn; vẫn gửi done/total cuối.
    Cancelling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum TaskOutcome {
    Done,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    /// Bộ đếm đơn điệu toàn hub (tăng ở mọi upsert VÀ removed): FE bỏ sự kiện có rev ≤ rev đã thấy.
    #[ts(type = "number")]
    pub rev: u64,
    pub kind: TaskKind,
    pub trigger: ScanTrigger,
    /// Khoá i18n của tiêu đề; `title_args` chỉ chứa tên ngắn (nguồn/Case), không bao giờ đường dẫn đầy đủ.
    pub title_key: String,
    #[ts(type = "Record<string, string | number>")]
    pub title_args: BTreeMap<String, TaskArg>,
    #[ts(type = "number")]
    pub done: u64,
    /// `None` = chưa biết (không dùng 0 thay cho "chưa biết").
    #[ts(type = "number | null")]
    pub total: Option<u64>,
    pub unit: TaskUnit,
    pub state: TaskState,
    pub cancellable: bool,
    /// Có = huỷ phá huỷ một phần, FE hỏi nhỏ trong dòng (khoá i18n); không = huỷ ngay.
    #[ts(optional)]
    pub cancel_confirm_key: Option<String>,
    /// Khoá gộp dòng (vd loại + nguồn); FE tự gộp khi nhiều dòng cùng loại.
    #[ts(optional)]
    pub group_key: Option<String>,
    /// Epoch ms.
    #[ts(type = "number")]
    pub started_at: u64,
}

/// Giá trị tham số tiêu đề: chuỗi hoặc số.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TaskArg {
    Text(String),
    Num(i64),
}

impl From<&str> for TaskArg {
    fn from(s: &str) -> Self {
        TaskArg::Text(s.to_string())
    }
}
impl From<String> for TaskArg {
    fn from(s: String) -> Self {
        TaskArg::Text(s)
    }
}
impl From<i64> for TaskArg {
    fn from(n: i64) -> Self {
        TaskArg::Num(n)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct TaskListDto {
    #[ts(type = "number")]
    pub rev: u64,
    pub tasks: Vec<TaskDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum TaskEvent {
    Upsert {
        task: TaskDto,
    },
    Removed {
        id: String,
        #[ts(type = "number")]
        rev: u64,
        outcome: TaskOutcome,
        #[serde(rename = "errorKey", skip_serializing_if = "Option::is_none")]
        #[ts(optional, rename = "errorKey")]
        error_key: Option<String>,
    },
}
