//! DTO của Settings → Tự bật font (auto-activation): trạng thái từng app thiết kế, nhật ký, trình quản lý font khác.
use fontatlas_store::AutoLogRow;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Cách Font Atlas nói chuyện với app đó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum AutoMethodDto {
    /// Nguồn font request của hệ thống (Photoshop, After Effects, Affinity): không cài gì vào app.
    FontRequest,
    /// Panel CEP nền (Illustrator): cài vào thư mục extensions của Adobe.
    CepPanel,
    /// Startup script (InDesign, InCopy): cài vào thư mục Startup Scripts của người dùng.
    StartupScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum AutoAppStateDto {
    /// Công tắc Tự bật font đang tắt.
    Off,
    /// Hoạt động (font request: nguồn đang chạy; cài đặt: đã cài và khớp bản hiện tại).
    Ready,
    /// Cần bấm Cài.
    NeedsInstall,
    /// Đã cài nhưng cũ (khác phiên bản, khác thư mục dữ liệu, bị sửa): cài lại.
    Stale,
    /// Có tệp/thư mục cùng tên không phải của Font Atlas: không đè.
    Foreign,
    /// App chưa chạy lần nào nên chưa có thư mục để cài: mở app một lần rồi cài lại (InDesign/InCopy).
    WaitingFirstLaunch,
    /// Có việc hỏng (xem `detail`): nguồn font request không mở được, broker không chạy, …
    Error,
    /// Bản chạy với thư mục dữ liệu riêng (QA/test): nguồn font request cố ý không mở.
    IsolatedBuild,
}

/// Mức bằng chứng rằng đường này chạy thật với app đó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum AutoEvidenceDto {
    /// Đã đo với app thật ở máy thử.
    Measured,
    /// Cơ chế có tài liệu/đã thử một phần nhưng chưa đo trọn.
    Partial,
    /// Chưa thử với app thật.
    Untested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AutoAppDto {
    /// photoshop | aftereffects | illustrator | indesign | incopy | affinity
    pub id: String,
    pub label: String,
    pub method: AutoMethodDto,
    pub state: AutoAppStateDto,
    pub evidence: AutoEvidenceDto,
    /// App có trên máy (`/Applications`).
    pub app_installed: bool,
    pub app_running: bool,
    /// Cần có nút Cài/Gỡ cho app này.
    pub installable: bool,
    /// Đã cài (panel/script có mặt, kể cả cũ).
    pub installed: bool,
    /// Cài/gỡ xong thì app đang chạy phải khởi động lại mới có hiệu lực.
    pub restart_needed: bool,
    pub detail: Option<String>,
    /// Lần gần nhất app này hỏi font (unix ms) theo nhật ký; `None` nếu chưa.
    pub last_activity_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct OtherManagerDto {
    pub name: String,
    pub installed: bool,
    pub running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AutoStatusDto {
    pub enabled: bool,
    pub broker_running: bool,
    pub font_request_running: bool,
    pub apps: Vec<AutoAppDto>,
    /// Trình quản lý font khác trên máy: có thể cùng phản ứng với tài liệu thiếu font.
    pub other_managers: Vec<OtherManagerDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename_all = "snake_case")]
pub enum AutoTargetDto {
    /// Panel CEP của Illustrator.
    Illustrator,
    /// Startup script của InDesign và InCopy (mọi phiên bản đã chạy).
    Indesign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct AutoLogDto {
    pub id: i64,
    pub at_ms: i64,
    pub app: String,
    pub doc: Option<String>,
    pub requested: String,
    /// activated | missing | conflict | unavailable | not_downloaded | failed | busy
    pub outcome: String,
    pub hash_hex: Option<String>,
}

impl From<AutoLogRow> for AutoLogDto {
    fn from(r: AutoLogRow) -> Self {
        AutoLogDto { id: r.id, at_ms: r.at, app: r.app, doc: r.doc, requested: r.requested, outcome: r.outcome, hash_hex: r.hash_hex }
    }
}
