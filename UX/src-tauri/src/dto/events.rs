//! Sự kiện đẩy qua `tauri::ipc::Channel`: `{ type: "...", ... }`.
use serde::Serialize;
use ts_rs::TS;

use crate::dto::library::{RescanStatsDto, ScanProgressDto};
use crate::dto::activation::{ProgressDto, ReconcileReportDto};
use crate::dto::model::{EmbedStatusDto, ErrorInfoDto};
use crate::dto::settings::BootPhase;
use crate::error::CmdError;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum BootEvent {
    Phase { phase: BootPhase },
    Failed { error: CmdError },
}

/// Ai xếp lượt quét: `watcher` = tự phát hiện thay đổi (FE không nên nháy "đang quét"), `launch` = quét lúc mở app,
/// `user` = người dùng/lệnh (thêm nguồn, Quét lại, tải về từ cloud…).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum ScanTrigger {
    Watcher,
    Launch,
    User,
}

impl ScanTrigger {
    /// Khi một nguồn đã chờ mà bị xếp lại: giữ bên mạnh hơn (người dùng thắng).
    pub fn rank(self) -> u8 {
        match self {
            ScanTrigger::Watcher => 0,
            ScanTrigger::Launch => 1,
            ScanTrigger::User => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum ScanEvent {
    Queued {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        trigger: ScanTrigger,
    },
    Progress {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        progress: ScanProgressDto,
        trigger: ScanTrigger,
    },
    /// Sau mỗi lô có font mới: UI nạp thêm trang.
    FacesAdded {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        count: u32,
        trigger: ScanTrigger,
    },
    Paused,
    Resumed,
    Finished {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        stats: RescanStatsDto,
        /// Lượt quét có làm đổi thư viện không (font mới/đổi/mất/lỗi/dọn, hoặc huỷ/nguồn không truy cập được)?
        /// `false` = mọi thứ như cũ: FE không cần nạp lại lưới/sidebar.
        changed: bool,
        trigger: ScanTrigger,
    },
    Failed {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        error: CmdError,
    },
    Idle,
}

/// Sự kiện tải model. `resumed:false` khi server bỏ qua Range và tải lại từ đầu.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum ModelEvent {
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Progress {
        #[ts(type = "number")]
        done: u64,
        #[ts(type = "number")]
        total: u64,
        bytes_per_sec: f64,
        #[ts(type = "number | null")]
        eta_ms: Option<u64>,
        resumed: bool,
    },
    Ready,
    Paused,
    Failed { error: ErrorInfoDto },
}

/// Mỗi thay đổi gửi trạng thái; xong gửi `finished`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum EmbedEvent {
    Status { status: EmbedStatusDto },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Finished {
        #[ts(type = "number")]
        total_done: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum ActivationEvent {
    Progress { progress: ProgressDto },
    Reconciled { report: ReconcileReportDto },
    /// Các file vừa đổi trạng thái (UI nạp lại `activation_state`).
    Changed { hashes: Vec<String> },
    /// B5: font của một Case vừa bật/tắt vì app gắn với Case mở/thoát.
    AppSet { event: crate::dto::activation::AppSetEventDto },
}
