//! FontAtlas: lớp lệnh Tauri mỏng trên các crate `fontatlas-*`.
pub mod bindings;
pub mod commands;
pub mod dto;
pub mod error;
pub mod hub;
pub mod index_bundle;
pub mod services;
pub mod state;

use tauri::Manager;

use crate::services::activation::platform_api;
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
            let state = AppState::new(dirs, platform_api());
            app.manage(state.clone());
            crate::services::boot::start(state);
            Ok(())
        })
        .invoke_handler(app_handler!())
        .build(tauri::generate_context!())
        .expect("không dựng được FontAtlas")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<std::sync::Arc<AppState>>()
            {
                crate::services::lifecycle::on_exit(&state);
            }
        });
}
