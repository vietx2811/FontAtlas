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

/// Xử lý yêu cầu scheme trong `spawn_blocking` (đọc file, vẽ PNG, SQLite).
fn scheme(app: &tauri::AppHandle, request: tauri::http::Request<Vec<u8>>, responder: tauri::UriSchemeResponder, font: bool) {
    let uri = request.uri().to_string();
    let origin = request.headers().get("origin").and_then(|v| v.to_str().ok()).map(String::from);
    let state = app.try_state::<std::sync::Arc<AppState>>().map(|s| s.inner().clone());
    tauri::async_runtime::spawn_blocking(move || {
        let http = match state {
            Some(st) if font => crate::services::preview::serve_font(&st, &uri, origin.as_deref()),
            Some(st) => crate::services::preview::serve_preview(&st, &uri, origin.as_deref()),
            None => fontatlas_preview::PreviewHttp {
                status: 503,
                content_type: "text/plain",
                cache_control: "no-store",
                allow_origin: None,
                vary: "Origin",
                body: b"store_unavailable".to_vec(),
            },
        };
        responder.respond(crate::services::preview::to_http(http));
    });
}

pub fn run() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol(fontatlas_preview::PREVIEW_SCHEME, |ctx, request, responder| {
            scheme(ctx.app_handle(), request, responder, false)
        })
        .register_asynchronous_uri_scheme_protocol(fontatlas_preview::FONT_SCHEME, |ctx, request, responder| {
            scheme(ctx.app_handle(), request, responder, true)
        })
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dirs = resolve_dirs(app.handle())?;
            let state = AppState::new(dirs, platform_api());
            app.manage(state.clone());
            crate::services::boot::start(state);
            // QA nội bộ (chỉ bản debug): chạy một tệp JS trong webview thật để đi các luồng chính
            // trên IPC thật, cùng FONTATLAS_DATA_DIR tạm. Không có trong bản release.
            #[cfg(debug_assertions)]
            if let Some(path) = std::env::var_os("FONTATLAS_QA_SCRIPT") {
                let js = std::fs::read_to_string(path)?;
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(4));
                    if let Some(w) = h.get_webview_window("main") {
                        let _ = w.eval(&js);
                    }
                });
            }
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
