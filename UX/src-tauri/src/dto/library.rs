//! DTO nguồn font và quét.
use fontatlas_store::{RescanStats, RescanStatus, ScanPhase, ScanProgress};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum ScanPhaseDto {
    Walking,
    Hashing,
    Analyzing,
    MarkingMissing,
}

impl From<ScanPhase> for ScanPhaseDto {
    fn from(p: ScanPhase) -> Self {
        match p {
            ScanPhase::Walking => Self::Walking,
            ScanPhase::Hashing => Self::Hashing,
            ScanPhase::Analyzing => Self::Analyzing,
            ScanPhase::MarkingMissing => Self::MarkingMissing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ScanProgressDto {
    pub phase: ScanPhaseDto,
    pub done: u32,
    pub total: u32,
    pub faces_written: u32,
}

impl From<&ScanProgress> for ScanProgressDto {
    fn from(p: &ScanProgress) -> Self {
        ScanProgressDto { phase: p.phase.into(), done: p.done as u32, total: p.total as u32, faces_written: p.faces_written as u32 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum RescanStatusDto {
    Ok,
    SourceUnavailable,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct RescanStatsDto {
    pub status: RescanStatusDto,
    pub seen: u32,
    pub unchanged: u32,
    pub relinked: u32,
    pub analyzed: u32,
    pub failed: u32,
    pub missing: u32,
    pub unreadable: Vec<(String, String)>,
}

impl From<&RescanStats> for RescanStatsDto {
    fn from(s: &RescanStats) -> Self {
        RescanStatsDto {
            status: match s.status {
                RescanStatus::Ok => RescanStatusDto::Ok,
                RescanStatus::SourceUnavailable => RescanStatusDto::SourceUnavailable,
                RescanStatus::Cancelled => RescanStatusDto::Cancelled,
            },
            seen: s.seen as u32,
            unchanged: s.unchanged as u32,
            relinked: s.relinked as u32,
            analyzed: s.analyzed as u32,
            failed: s.failed as u32,
            missing: s.missing as u32,
            unreadable: s.unreadable.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct SourceDto {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub path: Option<String>,
    pub enabled: bool,
    pub available: bool,
    pub face_count: u32,
    pub last_scan: Option<RescanStatsDto>,
    pub scanning: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ScanStatusDto {
    pub running: bool,
    pub paused: bool,
    #[ts(type = "number | null")]
    pub current_source_id: Option<i64>,
    #[ts(type = "Array<number>")]
    pub queued: Vec<i64>,
}
