//! Cài/gỡ panel nền cho Illustrator (`fontatlas-cep`). Lệnh Tauri cho Settings nằm ở L5; đây là phần dịch vụ.
use fontatlas_broker::BROKER_FILE;
use fontatlas_cep::{InstallError, InstallReport, PanelInstaller, PanelStatus, default_extensions_dir};

use crate::state::AppState;

fn installer(state: &AppState) -> Option<PanelInstaller> {
    let ext = default_extensions_dir()?;
    Some(PanelInstaller::new(ext, state.dirs.data.join("cep"), state.dirs.data.join(BROKER_FILE), env!("CARGO_PKG_VERSION")))
}

/// Trạng thái panel Illustrator (`None` nếu không xác định được thư mục của Adobe).
pub fn status(state: &AppState) -> Option<PanelStatus> {
    Some(installer(state)?.status())
}

pub fn install(state: &AppState) -> Result<InstallReport, InstallError> {
    installer(state).ok_or_else(|| InstallError::Io(std::io::Error::other("không có thư mục HOME")))?.install()
}

pub fn remove(state: &AppState) -> Result<bool, InstallError> {
    installer(state).ok_or_else(|| InstallError::Io(std::io::Error::other("không có thư mục HOME")))?.remove()
}

/// Khoá ký của máy này đang được giữ không (để Settings nói rõ và có nút xoá).
pub fn key_present(state: &AppState) -> bool {
    installer(state).is_some_and(|i| i.key_present())
}

pub fn forget_key(state: &AppState) -> Result<bool, InstallError> {
    installer(state).ok_or_else(|| InstallError::Io(std::io::Error::other("không có thư mục HOME")))?.forget_key()
}
