//! FontAtlas: lớp lệnh Tauri mỏng trên các crate `fontatlas-*`.
pub mod bindings;
pub mod commands;
pub mod dto;
pub mod error;
pub mod hub;
pub mod index_bundle;
pub mod services;
pub mod signals;
pub mod state;
pub mod glass;
pub mod trash;

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

/// QA bản debug: sự kiện `qa-snap` (payload = tên) → ảnh TIFF của webview vào FONTATLAS_DATA_DIR.
/// Dùng `WKWebView.takeSnapshot`, chỉ chụp nội dung của chính app.
#[cfg(all(debug_assertions, target_os = "macos"))]
mod qa_snapshot {
    use tauri::{Listener, Manager};

    pub fn listen(app: &tauri::AppHandle) {
        // Cửa sổ QA bị che khi người dùng đang làm việc: App Nap hãm timer JS, kịch bản
        // dừng giữa chừng (từng treo ở bước dark-snapshots). Giữ app thức trong suốt lượt QA.
        {
            use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};
            let token = NSProcessInfo::processInfo().beginActivityWithOptions_reason(
                NSActivityOptions::UserInitiated | NSActivityOptions::LatencyCritical,
                &NSString::from_str("FontAtlas QA"),
            );
            std::mem::forget(token);
        }
        // App Nap chỉ giữ tiến trình app; nội dung web chạy ở tiến trình WebContent riêng và
        // bị WebKit hãm/treo timer khi cửa sổ bị che. Tắt hãm cho webview QA (macOS 14+).
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.with_webview(|wv| unsafe {
                let wk: &objc2_web_kit::WKWebView = &*(wv.inner() as *const objc2_web_kit::WKWebView);
                wk.configuration()
                    .preferences()
                    .setInactiveSchedulingPolicy(objc2_web_kit::WKInactiveSchedulingPolicy::None);
            });
        }
        let Some(dir) = std::env::var_os("FONTATLAS_DATA_DIR") else { return };
        let dir = std::path::PathBuf::from(dir);
        let h = app.clone();
        app.listen("qa-snap", move |ev| {
            let name: String = serde_json::from_str(ev.payload()).unwrap_or_else(|_| "snap".into());
            let name: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
            let path = dir.join(format!("snap-{name}.tiff"));
            let Some(w) = h.get_webview_window("main") else { return };
            let _ = w.with_webview(move |wv| unsafe {
                let wk: &objc2_web_kit::WKWebView = &*(wv.inner() as *const objc2_web_kit::WKWebView);
                let block = block2::RcBlock::new(move |img: *mut objc2_app_kit::NSImage, _e: *mut objc2_foundation::NSError| {
                    if let Some(img) = img.as_ref()
                        && let Some(data) = img.TIFFRepresentation()
                    {
                        let _ = std::fs::write(&path, data.to_vec());
                    }
                });
                wk.takeSnapshotWithConfiguration_completionHandler(None, &block);
            });
        });
    }
}

#[cfg(debug_assertions)]
mod qa {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    pub static SCRIPT: OnceLock<String> = OnceLock::new();
    pub static PAGE_LOADED: AtomicBool = AtomicBool::new(false);
    static RAN: AtomicBool = AtomicBool::new(false);
    /// Chạy script QA đúng một lần, khi đã có script và trang đã load.
    pub fn run_if_ready<R: tauri::Runtime>(w: &tauri::WebviewWindow<R>) {
        if !PAGE_LOADED.load(Ordering::SeqCst) {
            return;
        }
        if let Some(js) = SCRIPT.get()
            && !RAN.swap(true, Ordering::SeqCst)
        {
            let _ = w.eval(js);
        }
    }
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
            crate::signals::install();
            crate::signals::watch(app.handle().clone());
            crate::glass::init(app.handle());
            state.system_hooks.store(true, std::sync::atomic::Ordering::SeqCst);
            crate::services::boot::start(state);
            // QA nội bộ (chỉ bản debug): chạy một tệp JS trong webview thật để đi các luồng chính
            // trên IPC thật, cùng FONTATLAS_DATA_DIR tạm. Không có trong bản release.
            #[cfg(debug_assertions)]
            if let Some(path) = std::env::var_os("FONTATLAS_QA_SCRIPT") {
                let _ = qa::SCRIPT.set(std::fs::read_to_string(path)?);
                #[cfg(target_os = "macos")]
                qa_snapshot::listen(app.handle());
                // Trang có thể đã load xong trước setup: thử chạy ngay nếu vậy.
                if let Some(w) = app.get_webview_window("main") {
                    qa::run_if_ready(&w);
                }
            }
            Ok(())
        })
        // QA: chạy script khi trang load xong thay vì sau một khoảng chờ cố định
        // (máy bận → trang load > 4 s → script cũ bị mất, QA treo không ghi bước nào).
        .on_page_load(|_webview, _payload| {
            #[cfg(debug_assertions)]
            if _payload.event() == tauri::webview::PageLoadEvent::Finished {
                qa::PAGE_LOADED.store(true, std::sync::atomic::Ordering::SeqCst);
                if let Some(w) = _webview.app_handle().get_webview_window("main") {
                    qa::run_if_ready(&w);
                }
            }
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
            // Link `fontatlas://` (macOS gửi khi app đang chạy hoặc vừa được mở bởi link): chỉ xếp hàng
            // rồi báo giao diện; không bao giờ tự làm gì với nội dung link.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = &event
                && let Some(state) = app.try_state::<std::sync::Arc<AppState>>()
            {
                let any = urls.iter().fold(false, |acc, u| crate::services::figma::enqueue_link(&state, u.as_str()) | acc);
                if any {
                    use tauri::Emitter;
                    let _ = app.emit("figma-import-pending", ());
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.set_focus();
                    }
                }
            }
        });
}
