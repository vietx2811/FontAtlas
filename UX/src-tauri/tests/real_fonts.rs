//! Kiểm luồng chính qua IPC thật (mock runtime) trên font hệ thống macOS thật.
//! Chỉ chạy local: cargo test --release --test real_fonts -- --ignored --nocapture
//! Đọc font hệ thống (chỉ đọc), dữ liệu app trong thư mục tạm; không phân phối font.
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, sources};
use app_lib::state::{AppDirs, AppState};
use serde_json::{Value, json};
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;

fn ipc(app: &tauri::App<tauri::test::MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    use tauri::Manager;
    let w = app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(app, "main", Default::default()).build().unwrap()
    });
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

fn face_of(app: &tauri::App<tauri::test::MockRuntime>, family: &str, style: &str) -> Value {
    let page = ipc(app, "query_faces", json!({"q": {"text": family, "limit": 200}})).unwrap();
    let row = page["items"].as_array().unwrap().iter()
        .find(|r| r["family"] == family && r["subfamily"] == style)
        .unwrap_or_else(|| panic!("không thấy {family} {style}"));
    json!({"hashHex": row["hashHex"], "faceIndex": row["faceIndex"]})
}

#[test]
#[ignore = "local macOS only: system fonts"]
fn real_system_fonts_main_flows() {
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    // Onboarding nhiều nguồn: hai thư mục thật.
    for p in ["/System/Library/Fonts", "/Library/Fonts"] {
        sources::add_source(&st, p).unwrap();
    }
    // Thư mục lồng trong nguồn đã có bị từ chối, không quét trùng.
    let e = sources::add_source(&st, "/System/Library/Fonts/Supplemental").unwrap_err();
    assert_eq!(e.code, "nested_source");
    assert!(st.scans.wait_idle(Duration::from_secs(900)));
    app_lib::index_bundle::rebuild_index(&st).unwrap();
    let app = { let s: Arc<AppState> = st; mock_builder().manage(s).invoke_handler(app_lib::app_handler!()).build(mock_context(noop_assets())).unwrap() };

    let srcs = ipc(&app, "list_sources", json!({})).unwrap();
    println!("NGUỒN: {}", srcs.as_array().map(|a| a.len()).unwrap_or(0));
    let all = ipc(&app, "query_faces", json!({"q": {"limit": 1}})).unwrap();
    println!("TỔNG FACE: {}", all["total"]);

    // Similar theo family, loại family nguồn, không trùng giữa các trang.
    for (fam, style) in [("Avenir Next", "Regular"), ("Times New Roman", "Regular"), ("Futura", "Medium")] {
        let face = face_of(&app, fam, style);
        let mut seen = HashSet::new();
        let mut names = vec![];
        for off in [0, 6, 12] {
            let p = ipc(&app, "similar_families", json!({"face": face, "opts": {}, "q": {"offset": off, "limit": 6}})).unwrap();
            for it in p["items"].as_array().unwrap() {
                let f = it["family"].as_str().unwrap().to_string();
                assert_ne!(f, fam, "Similar trả lại chính family nguồn");
                assert!(!f.starts_with('.'), "font hệ thống ẩn lọt vào Similar: {f}");
                assert!(seen.insert(f.clone()), "family trùng giữa trang: {f}");
                names.push(format!("{f}({})", it["count"]));
            }
        }
        println!("SIMILAR {fam} {style}: {}", names.join(", "));
        // Family scope: mọi style, rồi lọc AND italic.
        let styles = ipc(&app, "get_family_styles", json!({"family": fam})).unwrap();
        let keys: Vec<Value> = styles.as_array().unwrap().iter().map(|s| s["face"].clone()).collect();
        let fam_all = ipc(&app, "query_faces", json!({"q": {"faceKeys": keys, "limit": 500}})).unwrap();
        let fam_it = ipc(&app, "query_faces", json!({"q": {"faceKeys": keys, "italic": true, "limit": 500}})).unwrap();
        println!("FAMILY {fam}: {} style; lọc italic {}", fam_all["total"], fam_it["total"]);
    }

    // Thống kê trục thật cho slider.
    let stats = ipc(&app, "axis_stats", json!({"q": {}})).unwrap();
    for s in stats.as_array().unwrap() {
        println!("STATS {:<14} {:<6} n={:<4} min={} p1={} p99={} max={}", s["axis"].as_str().unwrap(), s["unit"].as_str().unwrap(), s["count"], s["min"], s["p1"], s["p99"], s["max"]);
    }
    // Lọc theo khoảng số thật (p1..p99 x-height) + italic.
    let xh = stats.as_array().unwrap().iter().find(|s| s["axis"] == "x_height_em").unwrap();
    let r = ipc(&app, "query_faces", json!({"q": {"rawAxisRanges": [{"axis": "x_height_em", "range": [xh["p1"], xh["p99"]]}], "limit": 1}})).unwrap();
    let r2 = ipc(&app, "query_faces", json!({"q": {"rawAxisRanges": [{"axis": "x_height_em", "range": [0.45, 0.50]}], "italic": true, "limit": 1}})).unwrap();
    println!("LỌC x-height p1..p99: {} ; 0,45–0,50 em + italic: {}", r["total"], r2["total"]);
    assert!(r["total"].as_u64().unwrap() > 0 && r2["total"].as_u64().unwrap() <= r["total"].as_u64().unwrap());

    // Số đo hiển thị: giá trị thô + đơn vị + chất lượng.
    for (fam, style) in [("Helvetica Neue", "Regular"), ("Times New Roman", "Regular"), ("Didot", "Regular"), ("Avenir Next Condensed", "Medium Italic")] {
        let face = face_of(&app, fam, style);
        let d = ipc(&app, "get_face", json!({"face": face, "lang": "vi"})).unwrap();
        let axes: Vec<String> = d["axes"].as_array().unwrap().iter().map(|a| format!("{}={}{} [{}]", a["key"].as_str().unwrap(), a["rawValue"], a["unit"].as_str().map(|u| format!(" {u}")).unwrap_or_default(), a["quality"].as_str().unwrap_or("?"))).collect();
        println!("ANATOMY {fam} {style}: {}", axes.join(" · "));
        assert!(d["axes"].as_array().unwrap().iter().any(|a| a["rawValue"].is_number() && a["unit"].is_string()));
    }
}
