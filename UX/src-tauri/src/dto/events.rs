//! Sự kiện đẩy qua `tauri::ipc::Channel`: `{ type: "...", ... }`.
use serde::Serialize;
use ts_rs::TS;

use crate::dto::settings::BootPhase;
use crate::error::CmdError;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum BootEvent {
    Phase { phase: BootPhase },
    Failed { error: CmdError },
}
