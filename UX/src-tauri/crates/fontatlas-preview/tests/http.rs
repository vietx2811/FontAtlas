mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_preview::*;
use std::path::PathBuf;

struct One(String, PathBuf);
impl FaceLocator for One {
    fn locate(&self, h: &str, _i: u32) -> Option<PathBuf> { (h == self.0).then(|| self.1.clone()) }
}
fn setup(p: PathBuf) -> (tempfile::TempDir, PreviewService<One>, String) {
    let h = FontFile::open(&p).unwrap().hash_hex();
    let d = tempfile::tempdir().unwrap();
    let c = PreviewCache::open(&d.path().join("p.db"), Budgets::DEFAULT).unwrap();
    (d, PreviewService::new(c, One(h.clone(), p)), h)
}
const APP: Option<&str> = Some("tauri://localhost");

#[test]
fn parses_macos_and_windows_forms() {
    let h = "ab".repeat(32);
    for base in ["fontatlas-preview://localhost/", "http://fontatlas-preview.localhost/"] {
        let r = parse_preview_uri(&format!("{base}{h}_2_64.png?t=%C4%82n%20ph%E1%BB%9F&g=7&r=preview-0.1.0")).unwrap();
        assert_eq!(r, PreviewRequest { hash_hex: h.clone(), face_index: 2, px: 64, text: "Ăn phở".into(), generation: Some(7) });
    }
    let r = parse_preview_uri(&format!("fontatlas-preview://localhost/{h}_0_48.png")).unwrap();
    assert_eq!((r.text.as_str(), r.generation), (DEFAULT_SAMPLE, None));
    assert_eq!(parse_font_uri(&format!("http://fontatlas-font.localhost/{h}_0.font")).unwrap(), (h.clone(), 0));
}

#[test]
fn bad_uris_are_400() {
    let h = "ab".repeat(32);
    for u in [
        format!("fontatlas-preview://localhost/{h}_0_50.png"),
        format!("fontatlas-preview://localhost/{h}_0_48.jpg"),
        "fontatlas-preview://localhost/zz_0_48.png".to_string(),
        format!("fontatlas-preview://localhost/{h}_x_48.png"),
        format!("fontatlas-preview://localhost/{h}_0_48.png?g=abc"),
        "không phải url".to_string(),
    ] {
        assert_eq!(parse_preview_uri(&u).unwrap_err().http_status(), 400, "{u}");
    }
    assert_eq!(parse_font_uri(&format!("fontatlas-font://localhost/{h}.font")).unwrap_err().http_status(), 400);
}

#[test]
fn preview_ok_with_cors_for_app_origin_only() {
    let (_d, s, h) = setup(core_fixture("roboto.ttf"));
    let uri = format!("fontatlas-preview://localhost/{h}_0_48.png?t=Ab");
    let ok = handle_preview_uri(&s, &uri, APP, &CorsPolicy::app());
    assert_eq!((ok.status, ok.content_type, ok.cache_control), (200, "image/png", "private, max-age=86400"));
    assert_eq!(ok.allow_origin.as_deref(), Some("tauri://localhost"));
    assert_eq!(ok.vary, "Origin");
    assert_eq!(&ok.body[1..4], b"PNG");
    assert_eq!(handle_preview_uri(&s, &uri, Some("https://evil.example"), &CorsPolicy::app()).allow_origin, None);
    assert_eq!(handle_preview_uri(&s, &uri, None, &CorsPolicy::app()).allow_origin, None);
    let dev = CorsPolicy::app().with("http://localhost:3000");
    assert_eq!(handle_preview_uri(&s, &uri, Some("http://localhost:3000"), &dev).allow_origin.as_deref(), Some("http://localhost:3000"));
}

#[test]
fn preview_errors_map_to_http() {
    let (_d, s, h) = setup(core_fixture("roboto.ttf"));
    let nf = handle_preview_uri(&s, &format!("fontatlas-preview://localhost/{}_0_48.png", "cd".repeat(32)), APP, &CorsPolicy::app());
    assert_eq!((nf.status, nf.content_type, nf.cache_control), (404, "text/plain; charset=utf-8", "no-store"));
    assert_eq!(String::from_utf8(nf.body).unwrap(), "not_found");
    assert_eq!(nf.allow_origin.as_deref(), Some("tauri://localhost"), "lỗi cũng cần CORS để fetch đọc được mã");
    for _ in 0..3 { s.bump_generation(); }
    assert_eq!(handle_preview_uri(&s, &format!("fontatlas-preview://localhost/{h}_0_48.png?g=2"), APP, &CorsPolicy::app()).status, 410);
}

#[test]
fn font_uri_serves_file_with_type_and_cors() {
    let (_d, s, h) = setup(core_fixture("roboto.ttf"));
    let r = handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{h}_0.font"), APP, &CorsPolicy::app());
    assert_eq!((r.status, r.content_type), (200, "font/ttf"));
    assert_eq!(r.body, std::fs::read(core_fixture("roboto.ttf")).unwrap());
    assert_eq!(r.allow_origin.as_deref(), Some("tauri://localhost"));
    let (_d2, s2, h2) = setup(fixture("pair.ttc"));
    let c = handle_font_uri(s2.locator(), &format!("fontatlas-font://localhost/{h2}_0.font"), APP, &CorsPolicy::app());
    assert_eq!((c.status, c.content_type), (200, "font/collection"));
}

#[test]
fn font_uri_refuses_ttc_face() {
    let (_d, s, h) = setup(fixture("pair.ttc"));
    let r = handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{h}_1.font"), APP, &CorsPolicy::app());
    assert_eq!(r.status, 422);
    assert_eq!(String::from_utf8(r.body).unwrap(), "ttc_face");
}

#[test]
fn font_uri_not_found_and_stale() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("f.ttf");
    std::fs::copy(core_fixture("roboto.ttf"), &path).unwrap();
    let (_d, s, h) = setup(path.clone());
    assert_eq!(handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{}_0.font", "cd".repeat(32)), APP, &CorsPolicy::app()).status, 404);
    std::fs::copy(core_fixture("open_sans.ttf"), &path).unwrap();
    assert_eq!(handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{h}_0.font"), APP, &CorsPolicy::app()).status, 409);
}
