//! fontatlas-broker: cầu nối giữa app thiết kế (helper font request, panel CEP, startup script) và thư viện.
//!
//! - [`ensure_in_library`]: khớp tên font app hỏi với thư viện, bật scope session, ghi nhật ký.
//! - [`Server`]: máy chủ HTTP chỉ nghe `127.0.0.1`, cổng động, token ngẫu nhiên, giới hạn tần suất, chặn origin lạ.
//! - [`BrokerFile`]: file `broker.json` (quyền 0600) để client tìm cổng và token.
//!
//! Giao thức v1 (JSON, một yêu cầu một kết nối):
//! `POST /v1/ensure` + `Authorization: Bearer <token>`,
//! thân `{"v":1,"app":"photoshop","doc":"a.psd","fonts":[{"ps":"X-Regular"},{"family":"X","style":"Bold"}]}`.
mod config;
mod ensure;
mod limit;
mod server;

pub use config::{BrokerFile, BROKER_FILE, new_token};
pub use ensure::{
    AutoActivator,
    EnsureError, EnsureRequest, EnsureResponse, FontReq, FontResult, FontStatus, MAX_FONTS, MAX_TEXT, OwnedBackend, auto_enabled, ensure_in_library,
};
pub use limit::TokenBucket;
pub use server::{Backend, Server, ServerConfig};

pub const PROTOCOL_VERSION: u32 = 1;
/// Khoá `settings` bật/tắt tự bật font. Mặc định bật (thiếu = bật); `"0"` = tắt.
pub const SETTING_AUTO_ACTIVATION: &str = "auto_activation_enabled";
