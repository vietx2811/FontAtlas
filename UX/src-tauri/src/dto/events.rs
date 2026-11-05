//! Sự kiện đẩy qua `tauri::ipc::Channel`: `{ type: "...", ... }`.
use serde::Serialize;
use ts_rs::TS;

use crate::dto::library::{RescanStatsDto, ScanProgressDto};
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

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum ScanEvent {
    Queued {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
    },
    Progress {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        progress: ScanProgressDto,
    },
    /// Sau mỗi lô có font mới: UI nạp thêm trang.
    FacesAdded {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        count: u32,
    },
    Paused,
    Resumed,
    Finished {
        #[serde(rename = "sourceId")]
        #[ts(type = "number", rename = "sourceId")]
        source_id: i64,
        stats: RescanStatsDto,
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
