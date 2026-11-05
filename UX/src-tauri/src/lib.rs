//! FontAtlas: lớp lệnh Tauri mỏng trên các crate `fontatlas-*`.
pub mod bindings;
pub mod commands;
pub mod dto;
pub mod error;
pub mod services;
pub mod state;

use tauri::Manager;

use crate::services::activation::NoActivation;
use crate::state::{AppDirs, AppState};

/// Thư mục dữ liệu. Bản debug cho phép ghi đè bằng `FONTATLAS_DATA_DIR` (test, smoke).
fn resolve_dirs(app: &tauri::AppHandle) -> Result<AppDirs, tauri::Error> {
    #[cfg(debug_assertions)]
    if let Some(root) = std::env::var_os("FONTATLAS_DATA_DIR") {
        return Ok(AppDirs::under(std::path::Path::new(&root)));
    }
    let data = app.path().app_data_dir()?;
    let cache = app.path().app_cache_dir()?;
    let models = data.join("models");
    Ok(AppDirs { data, cache, models })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dirs = resolve_dirs(app.handle())?;
            let state = AppState::new(dirs, Box::new(NoActivation));
            app.manage(state);
            Ok(())
        })
        .invoke_handler(app_handler!())
        .run(tauri::generate_context!())
        .expect("không chạy được FontAtlas");
}
