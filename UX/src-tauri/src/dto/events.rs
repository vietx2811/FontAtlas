//! Sự kiện đẩy qua `tauri::ipc::Channel`: `{ type: "...", ... }`.
use serde::Serialize;
use ts_rs::TS;

use crate::dto::library::{RescanStatsDto, ScanProgressDto};
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
