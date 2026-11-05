//! Lệnh qua IPC thật của Tauri (mock runtime): tên lệnh snake_case, đối số và trường JSON camelCase.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, sources};
use app_lib::state::{AppDirs, AppState};
use serde_json::{Value, json};
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn ipc(app: &tauri::App<tauri::test::MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    use tauri::Manager;
    let w = app
        .get_webview_window("main")
        .unwrap_or_else(|| tauri::WebviewWindowBuilder::new(app, "main", Default::default()).build().unwrap());
    let req = InvokeRequest {
        cmd: cmd.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    get_ipc_response(&w, req).map(|b| b.deserialize::<Value>().unwrap())
}

fn app_with(state: Arc<AppState>) -> tauri::App<tauri::test::MockRuntime> {
    mock_builder().manage(state).invoke_handler(app_lib::app_handler!()).build(mock_context(noop_assets())).unwrap()
}

#[test]
fn ipc_query_faces_uses_camel_case_args() {
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = d.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("roboto.ttf"), dir.join("roboto.ttf")).unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let app = app_with(st);
    let page = ipc(&app, "query_faces", json!({ "q": { "text": "robo", "favoritesOnly": false, "includeMissing": false, "limit": 10 } })).unwrap();
    assert_eq!(page["total"], 1);
    let row = &page["items"][0];
    assert_eq!(row["family"], "Roboto");
    assert!(row["hashHex"].is_string() && row["supportsVi"].is_boolean());
    // đối số sai kiểu → lỗi IPC, không panic
    assert!(ipc(&app, "query_faces", json!({ "q": { "limit": "nhieu" } })).is_err());
    // activation_caps (sync) và lệnh chưa có Store
    let caps = ipc(&app, "activation_caps", json!({})).unwrap();
    assert_eq!(caps, json!({ "method": "none", "supported": false }));
}

#[test]
fn ipc_errors_are_cmd_error_shaped() {
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(NoActivation)); // chưa boot → chưa có Store
    let app = app_with(st);
    let e = ipc(&app, "list_collections", json!({})).unwrap_err();
    assert_eq!(e["code"], "store_unavailable");
    assert!(e["message"].is_string() && e.get("detail").is_some());
    let e = ipc(&app, "set_setting", json!({ "key": "ui.x", "value": "1" })).unwrap_err();
    assert_eq!(e["code"], "store_unavailable");
    let e = ipc(&app, "get_setting", json!({ "key": "khong_ui" })).unwrap_err();
    assert_eq!(e["code"], "invalid_arg");
}
