use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::library::FaceQueryDto;
use app_lib::dto::preview::PreviewItemDto;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, library, preview, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn booted(root: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    st
}

fn with_roboto(root: &Path) -> (Arc<AppState>, String) {
    let st = booted(root);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("roboto.ttf"), dir.join("roboto.ttf")).unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let h = library::query_faces(&st, &FaceQueryDto::default()).unwrap().items[0].hash_hex.clone();
    (st, h)
}

#[test]
fn preview_png_with_cors() {
    let d = tempfile::tempdir().unwrap();
    let (st, h) = with_roboto(d.path());
    let uri = format!("http://fontatlas-preview.localhost/{h}_0_48.png?t=Hamburg&g=0&r=x");
    let ok = preview::serve_preview(&st, &uri, Some("http://tauri.localhost"));
    assert_eq!((ok.status, ok.content_type), (200, "image/png"));
    assert_eq!(ok.allow_origin.as_deref(), Some("http://tauri.localhost"));
    assert_eq!(&ok.body[1..4], b"PNG");
    let evil = preview::serve_preview(&st, &uri, Some("https://evil.example"));
    assert_eq!(evil.status, 200);
    assert!(evil.allow_origin.is_none(), "origin lạ không được đọc phản hồi");
    let resp = preview::to_http(ok);
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["Content-Type"], "image/png");
    assert_eq!(resp.headers()["Access-Control-Allow-Origin"], "http://tauri.localhost");
    let missing = format!("http://fontatlas-preview.localhost/{}_0_48.png?t=A&g=0", "ab".repeat(32));
    assert_eq!(preview::serve_preview(&st, &missing, None).status, 404);
}

#[test]
fn font_scheme_serves_face0_and_rejects_ttc_face_1() {
    let d = tempfile::tempdir().unwrap();
    let (st, h) = with_roboto(d.path());
    let f0 = preview::serve_font(&st, &format!("http://fontatlas-font.localhost/{h}_0.font"), Some("tauri://localhost"));
    assert_eq!((f0.status, f0.content_type), (200, "font/ttf"));
    let f1 = preview::serve_font(&st, &format!("http://fontatlas-font.localhost/{h}_1.font"), None);
    assert_eq!(f1.status, 422);
}

#[test]
fn preview_info_matches_plan5_shape() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let info = preview::preview_info(&st).unwrap();
    let v = serde_json::to_value(&info).unwrap();
    assert_eq!(v["rendererVersion"], fontatlas_preview::RENDERER_VERSION);
    assert_eq!(v["generation"], 0);
    assert_eq!(v["buckets"], serde_json::json!([32, 48, 64, 96]));
    assert_eq!(v.as_object().unwrap().len(), 3);
    assert_eq!(preview::bump_generation(&st).unwrap(), 1);
    assert_eq!(preview::preview_info(&st).unwrap().generation, 1);
}

#[test]
fn no_store_returns_503() {
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(NoActivation)); // chưa boot
    let r = preview::serve_preview(&st, &format!("http://fontatlas-preview.localhost/{}_0_48.png", "ab".repeat(32)), None);
    assert_eq!(r.status, 503);
    assert_eq!(preview::serve_font(&st, "http://fontatlas-font.localhost/x_0.font", None).status, 503);
    assert_eq!(preview::preview_info(&st).unwrap_err().code, "store_unavailable");
}

#[test]
fn prewarm_then_clear_preview_cache_empties() {
    let d = tempfile::tempdir().unwrap();
    let (st, h) = with_roboto(d.path());
    let items: Vec<PreviewItemDto> = [32u32, 48, 64]
        .iter()
        .map(|px| PreviewItemDto { hash_hex: h.clone(), face_index: 0, px: *px, text: "Hamburg".into(), generation: 0 })
        .collect();
    preview::prewarm(&st, &items).unwrap();
    let info = preview::cache_info(&st).unwrap();
    assert_eq!(info.entries, 3);
    assert!(info.bytes > 0);
    preview::clear_cache(&st).unwrap();
    let info = preview::cache_info(&st).unwrap();
    assert_eq!((info.entries, info.bytes), (0, 0));
    // lô thế hệ cũ không làm hỏng lệnh
    preview::bump_generation(&st).unwrap();
    preview::prewarm(&st, &items).unwrap();
    assert_eq!(preview::cache_info(&st).unwrap().entries, 0);
}

#[test]
fn missing_chars_uses_the_real_cmap() {
    use app_lib::dto::library::FaceKeyDto;
    let d = tempfile::tempdir().unwrap();
    let (st, h) = with_roboto(d.path());
    let face = FaceKeyDto { hash_hex: h, face_index: 0 };
    let chars: Vec<String> = ["H", "a", "ấ", "龍", " "].iter().map(|s| s.to_string()).collect();
    assert_eq!(preview::missing_chars(&st, &face, &chars).unwrap(), vec!["龍".to_string()]);
    let ghost = FaceKeyDto { hash_hex: "0".repeat(64), face_index: 0 };
    assert_eq!(preview::missing_chars(&st, &ghost, &chars).unwrap_err().code, "not_found");
    let many: Vec<String> = (0..600).map(|_| "a".to_string()).collect();
    assert_eq!(preview::missing_chars(&st, &face, &many).unwrap_err().code, "invalid_arg");
}
