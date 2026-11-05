//! DTO preview (khớp `PreviewInfo` và `preview_prewarm` của Plan 5).
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PreviewInfoDto {
    pub renderer_version: String,
    #[ts(type = "number")]
    pub generation: u64,
    pub buckets: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PreviewItemDto {
    pub hash_hex: String,
    pub face_index: u32,
    pub px: u32,
    pub text: String,
    #[ts(type = "number")]
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct PreviewCacheInfoDto {
    #[ts(type = "number")]
    pub bytes: u64,
    #[ts(type = "number")]
    pub entries: u64,
}
