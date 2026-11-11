//! fontatlas-cep: panel CEP nền cho Illustrator.
//!
//! - [`Identity`] + [`sign`]: ký gói theo định dạng ZXP (XML-DSig, RSA-SHA1 trên C14N, digest SHA-256, chứng chỉ TỰ KÝ,
//!   không timestamp). Định dạng đã đối chiếu byte-từng-byte với một panel nền thật trên máy (digest manifest khớp,
//!   chữ ký verify bằng openssl): xem `tests/format.rs`. Khoá + chứng chỉ sinh riêng cho từng máy lúc cài, không có
//!   khoá bí mật nào trong repo hay trong app.
//! - [`PanelInstaller`]: cài/gỡ/xem trạng thái panel ở `~/Library/Application Support/Adobe/CEP/extensions`.
//!
//! Panel chạy TRONG tiến trình Illustrator (không Apple Events, không hộp Allow); nghe sự kiện tài liệu, đọc font thiếu
//! (`panel/jsx/host.jsx`), hỏi broker (`POST /v1/ensure`) rồi nhờ Illustrator vẽ lại.
mod identity;
mod install;
mod sign;

pub use identity::{Identity, IdentityError};
pub use install::{BUNDLE_ID, InstallError, InstallReport, PanelInstaller, PanelStatus, default_extensions_dir};
pub use sign::{SignError, VerifyError, canonical_manifest, canonical_signed_info, sign_package, verify_package};
