//! Cài/gỡ startup script cho InDesign/InCopy (`fontatlas-indesign`). Lệnh Tauri cho Settings nằm ở L5.
use fontatlas_broker::BROKER_FILE;
use fontatlas_indesign::{Report, StartupInstaller, Target, TargetStatus, default_prefs_root};

use crate::state::AppState;

fn installer(state: &AppState) -> Option<StartupInstaller> {
    Some(StartupInstaller::new(default_prefs_root()?, state.dirs.data.join(BROKER_FILE), env!("CARGO_PKG_VERSION")))
}

pub fn status(state: &AppState) -> Vec<(Target, TargetStatus)> {
    installer(state).map(|i| i.status()).unwrap_or_default()
}

pub fn install(state: &AppState) -> Vec<Report> {
    installer(state).map(|i| i.install()).unwrap_or_default()
}

pub fn remove(state: &AppState) -> Result<usize, fontatlas_indesign::Error> {
    match installer(state) {
        Some(i) => i.remove(),
        None => Ok(0),
    }
}
