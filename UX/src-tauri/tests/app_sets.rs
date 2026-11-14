//! B5 qua lớp ứng dụng: quét app đã cài, liên kết Case ↔ app, nhịp đồng bộ + sự kiện.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app_lib::dto::activation::*;
use app_lib::dto::events::ActivationEvent;
use app_lib::dto::library::*;
use app_lib::services::{app_sets, boot, library, sources};
use app_lib::state::{AppDirs, AppState};
use fontatlas_activate::{ActivationEngine, ActivationService, AppSetTracker, EngineError, FakeEngine, Scope};

struct Shared(Arc<FakeEngine>);
impl ActivationEngine for Shared {
    fn method(&self) -> &'static str { self.0.method() }
    fn register(&self, p: &Path, s: Scope) -> Result<(), EngineError> { self.0.register(p, s) }
    fn unregister(&self, p: &Path, s: Scope) -> Result<(), EngineError> { self.0.unregister(p, s) }
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> { self.0.available_postscript_names() }
    fn providers(&self, ps: &str) -> Result<Vec<PathBuf>, EngineError> { self.0.providers(ps) }
    fn postscript_names_in_file(&self, p: &Path) -> Result<Vec<String>, EngineError> { self.0.postscript_names_in_file(p) }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn running(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

fn fake_app(root: &Path, rel: &str, bundle: Option<&str>, name: Option<&str>) {
    let c = root.join(rel).join("Contents");
    std::fs::create_dir_all(&c).unwrap();
    let mut p = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict>");
    if let Some(b) = bundle {
        p += &format!("<key>CFBundleIdentifier</key><string>{b}</string>");
    }
    if let Some(n) = name {
        p += &format!("<key>CFBundleName</key><string>{n}</string>");
    }
    p += "</dict></plist>";
    std::fs::write(c.join("Info.plist"), p).unwrap();
}

#[test]
fn scan_reads_bundle_id_name_one_level_down_dedupes_and_sorts() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    fake_app(r, "Zeta.app", Some("com.example.zeta"), Some("Zeta"));
    fake_app(r, "Adobe Illustrator 2026/Adobe Illustrator.app", Some("com.adobe.illustrator"), Some("Adobe Illustrator"));
    fake_app(r, "alpha.app", Some("com.example.alpha"), None); // tên lấy từ tên gói
    fake_app(r, "Dup.app", Some("com.example.zeta"), Some("Zeta copy"));
    fake_app(r, "NoId.app", None, Some("NoId"));
    std::fs::create_dir_all(r.join("Broken.app")).unwrap(); // không có Info.plist
    std::fs::write(r.join("note.txt"), "x").unwrap();
    let apps = app_sets::scan_apps(&[r.to_path_buf(), PathBuf::from("/nonexistent-root")], &running(&["com.adobe.illustrator"]));
    let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Adobe Illustrator", "alpha", "Zeta"]);
    assert!(apps[0].running && !apps[1].running);
    assert_eq!(apps[0].bundle_id, "com.adobe.illustrator");
}

#[test]
fn installed_apps_finds_real_system_apps() {
    // Chỉ kiểm không panic và ra danh sách có bundle id hợp lệ (máy CI Linux không có /Applications: rỗng).
    for a in app_sets::installed_apps() {
        assert!(a.bundle_id.len() >= 3 && !a.name.is_empty());
    }
}

fn state(root: &Path) -> (Arc<AppState>, Arc<FakeEngine>) {
    let eng = Arc::new(FakeEngine::new());
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    (st, eng)
}

fn setup_case(d: &Path) -> (Arc<AppState>, Arc<FakeEngine>, i64, FaceKeyDto) {
    let (st, eng) = state(d);
    let dir = d.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("libre_baskerville.ttf"), dir.join("lb.ttf")).unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let row = library::query_faces(&st, &FaceQueryDto { limit: 5, ..Default::default() }).unwrap().items.remove(0);
    let key = FaceKeyDto { hash_hex: row.hash_hex, face_index: row.face_index };
    let case = library::create_collection(&st, "Brand Kit A", None).unwrap().id;
    library::add_to_collection(&st, case, std::slice::from_ref(&key)).unwrap();
    (st, eng, case, key)
}

#[test]
fn case_apps_roundtrip_validates_and_json_is_camel_case() {
    let d = tempfile::tempdir().unwrap();
    let (st, _e, case, _k) = setup_case(d.path());
    assert!(app_sets::case_apps(&st, case).unwrap().is_empty());
    let a = CaseAppDto { bundle_id: "com.adobe.illustrator".into(), name: "Adobe Illustrator".into() };
    app_sets::set_case_apps(&st, case, std::slice::from_ref(&a)).unwrap();
    assert_eq!(app_sets::case_apps(&st, case).unwrap(), vec![a.clone()]);
    assert_eq!(serde_json::to_value(&a).unwrap()["bundleId"], "com.adobe.illustrator");
    assert!(app_sets::set_case_apps(&st, case, &[CaseAppDto { bundle_id: "bad id".into(), name: String::new() }]).is_err());
    assert!(app_sets::set_case_apps(&st, 99999, std::slice::from_ref(&a)).is_err());
    assert_eq!(app_sets::case_apps(&st, case).unwrap().len(), 1, "lỗi không làm mất danh sách cũ");
}

#[test]
fn tick_turns_the_case_on_then_off_after_grace_and_emits_events() {
    let d = tempfile::tempdir().unwrap();
    let (st, eng, case, key) = setup_case(d.path());
    app_sets::set_case_apps(&st, case, &[CaseAppDto { bundle_id: "com.adobe.illustrator".into(), name: "Adobe Illustrator".into() }]).unwrap();
    let seen = Arc::new(Mutex::new(Vec::<AppSetEventDto>::new()));
    {
        let seen = seen.clone();
        st.hubs.activation.subscribe(Box::new(move |e| {
            if let ActivationEvent::AppSet { event } = e {
                seen.lock().unwrap().push(event);
            }
        }));
    }
    let grace = Duration::from_secs(30);
    let t0 = Instant::now();
    let mut tr = AppSetTracker::new();
    assert!(app_sets::tick(&st, &running(&[]), t0, grace, &mut tr).is_empty());

    let ev = app_sets::tick(&st, &running(&["com.adobe.illustrator"]), t0, grace, &mut tr);
    assert_eq!(ev.len(), 1);
    assert_eq!(eng.registrations().len(), 1, "font đã đăng ký thật vào engine");
    let on = seen.lock().unwrap().clone();
    assert_eq!((on[0].collection_id, on[0].kind, on[0].changed), (case, AppSetEventKindDto::On, 1));

    // trạng thái hiển thị: lý do caseapp
    let rows = app_lib::services::activation::state_list(&st).unwrap();
    assert!(matches!(rows[0].reasons[0], ReasonDto::CaseApp { collection_id } if collection_id == case));
    assert_eq!(serde_json::to_value(&rows[0].reasons[0]).unwrap()["kind"], "case_app");
    assert_eq!(rows[0].hash_hex, key.hash_hex);

    app_sets::tick(&st, &running(&[]), t0 + Duration::from_secs(5), grace, &mut tr);
    assert_eq!(eng.registrations().len(), 1, "trong ân hạn");
    let ev = app_sets::tick(&st, &running(&[]), t0 + Duration::from_secs(40), grace, &mut tr);
    assert_eq!(ev.len(), 1);
    assert!(eng.registrations().is_empty(), "hết ân hạn: đã gỡ khỏi engine");
    assert_eq!(seen.lock().unwrap().last().unwrap().kind, AppSetEventKindDto::Off);
}

#[test]
fn exit_releases_the_app_reasons_before_unregistering_session_fonts() {
    let d = tempfile::tempdir().unwrap();
    let (st, eng, case, _k) = setup_case(d.path());
    app_sets::set_case_apps(&st, case, &[CaseAppDto { bundle_id: "com.adobe.indesign".into(), name: "Adobe InDesign".into() }]).unwrap();
    let mut tr = AppSetTracker::new();
    app_sets::tick(&st, &running(&["com.adobe.indesign"]), Instant::now(), Duration::from_secs(30), &mut tr);
    assert_eq!(eng.registrations().len(), 1);
    app_lib::services::activation::on_exit(&st);
    assert!(eng.registrations().is_empty());
    let rows = app_lib::services::activation::state_list(&st).unwrap();
    assert!(rows.iter().all(|r| r.reasons.is_empty()), "lần mở sau không bật lại font của app không chạy");
}

#[cfg(target_os = "macos")]
#[test]
fn live_workspace_lists_finder() {
    assert!(app_sets::running_bundle_ids().contains("com.apple.finder"), "NSWorkspace phải thấy Finder đang chạy");
}
