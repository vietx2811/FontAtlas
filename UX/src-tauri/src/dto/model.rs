//! DTO model SigLIP, hàng đợi embedding và cài đặt.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dto::library::FaceKeyDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ModelState {
    Missing,
    Downloading,
    Paused,
    Ready,
    ChecksumMismatch,
    Corrupt,
    Unpublished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ErrorInfoDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ModelStatusDto {
    pub state: ModelState,
    #[ts(type = "number")]
    pub done_bytes: u64,
    #[ts(type = "number")]
    pub total_bytes: u64,
    pub bytes_per_sec: f64,
    #[ts(type = "number | null")]
    pub eta_ms: Option<u64>,
    pub location: String,
    #[ts(type = "number")]
    pub disk_free: u64,
    /// Kích thước × 1,1.
    #[ts(type = "number")]
    pub required_bytes: u64,
    pub last_error: Option<ErrorInfoDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EmbedStatusDto {
    pub enabled: bool,
    pub running: bool,
    pub paused: bool,
    #[ts(type = "number")]
    pub pending: u64,
    #[ts(type = "number")]
    pub done: u64,
    #[ts(type = "number")]
    pub failed: u64,
    /// "metal" | "cpu"
    pub device: Option<String>,
    pub ms_per_face: Option<f32>,
    #[ts(type = "number | null")]
    pub eta_ms: Option<u64>,
    pub model_version: Option<String>,
    /// Mã lỗi khi không nạp được model (E7).
    pub load_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct EmbedFailureDto {
    pub face: FaceKeyDto,
    pub code: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SettingsDto {
    pub embed_enabled: bool,
    pub license_notice_ack: bool,
    /// "vi" | "en"; null = chưa chọn (theo ngôn ngữ hệ thống)
    pub ui_lang: Option<String>,
    pub sample_text: String,
    pub project_vi_default: bool,
    pub keep_active_on_exit: bool,
    pub rescan_on_launch: bool,
    /// Tự tải font trên ổ cloud để xem trước (thẻ đang hiện). Tắt: bỏ qua bản chưa tải về, không đọc file dataless.
    pub cloud_preview_download: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, rename_all = "camelCase")]
pub struct SettingsPatchDto {
    pub embed_enabled: Option<bool>,
    pub license_notice_ack: Option<bool>,
    pub ui_lang: Option<String>,
    pub sample_text: Option<String>,
    pub project_vi_default: Option<bool>,
    pub keep_active_on_exit: Option<bool>,
    pub rescan_on_launch: Option<bool>,
    pub cloud_preview_download: Option<bool>,
}
