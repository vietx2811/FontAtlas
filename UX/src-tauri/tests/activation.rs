use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app_lib::dto::activation::*;
use app_lib::dto::events::{ActivationEvent, BootEvent};
use app_lib::dto::library::*;
use app_lib::dto::settings::BootPhase;
use app_lib::services::lifecycle::on_exit;
use app_lib::services::{activation, boot, library, onboarding, sources};
use app_lib::state::{AppDirs, AppState};
use fontatlas_activate::{ActivationEngine, ActivationService, EngineError, FakeEngine, Scope, WindowsGdiEngine};

/// Engine giả dùng chung giữa các `AppState` (mô phỏng hệ điều hành sống qua lần mở lại app).
struct Shared(Arc<FakeEngine>);

impl ActivationEngine for Shared {
    fn method(&self) -> &'static str {
        self.0.method()
    }
    fn register(&self, p: &Path, s: Scope) -> Result<(), EngineError> {
        self.0.register(p, s)
    }
    fn unregister(&self, p: &Path, s: Scope) -> Result<(), EngineError> {
        self.0.unregister(p, s)
    }
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> {
        self.0.available_postscript_names()
    }
    fn providers(&self, ps: &str) -> Result<Vec<PathBuf>, EngineError> {
        self.0.providers(ps)
    }
    fn postscript_names_in_file(&self, p: &Path) -> Result<Vec<String>, EngineError> {
        self.0.postscript_names_in_file(p)
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn new_state(root: &Path, eng: &Arc<FakeEngine>) -> Arc<AppState> {
    AppState::new(AppDirs::under(&root.join("data")), Box::new(ActivationService::new(Shared(eng.clone()))))
}

fn library3(root: &Path, eng: &Arc<FakeEngine>) -> (Arc<AppState>, Vec<FaceKeyDto>, PathBuf) {
    let st = new_state(root, eng);
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf"] {
        std::fs::copy(fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let faces = library::query_faces(&st, &FaceQueryDto { limit: 10, ..Default::default() })
        .unwrap()
        .items
        .iter()
        .map(|r| FaceKeyDto { hash_hex: r.hash_hex.clone(), face_index: r.face_index })
        .collect();
    (st, faces, dir)
}

fn noop(_: ProgressDto) {}

#[test]
fn activate_collection_via_command_reports_progress() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let (st, faces, _) = library3(d.path(), &eng);
    let events = Arc::new(Mutex::new(Vec::new()));
    let e2 = events.clone();
    st.hubs.activation.subscribe(Box::new(move |e| e2.lock().unwrap().push(e)));
    let col = library::create_collection(&st, "Menu", None).unwrap();
    library::add_to_collection(&st, col.id, &faces[..2]).unwrap();
    let mut prog = Vec::new();
    let r = activation::activate_collection(&st, col.id, Some(ScopeDto::Persistent), false, &mut |p| prog.push(p)).unwrap();
    assert_eq!((r.changed, r.failed), (2, 0));
    assert!(!prog.is_empty() && prog.last().unwrap().done == prog.last().unwrap().total);
    let rows = activation::state_list(&st).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|x| x.state == DesiredStateDto::Active && x.status == ActivationStatusDto::Registered && x.faces == vec![0]));
    assert!(rows.iter().all(|x| x.reasons == vec![ReasonDto::Collection { collection_id: col.id }]));
    assert!(events.lock().unwrap().iter().any(|e| matches!(e, ActivationEvent::Changed { hashes } if hashes.len() == 2)));
    assert_eq!(eng.registrations().len(), 2);
    // activate thủ công thêm face thứ ba rồi preflight
    assert_eq!(activation::preflight(&st, &faces[2..]).unwrap().results.len(), 1);
}

#[test]
fn boot_runs_reconcile_before_ready() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = new_state(d.path(), &eng);
    let log: Arc<Mutex<Vec<String>>> = Arc::default();
    let (l1, l2) = (log.clone(), log.clone());
    st.hubs.boot.subscribe(Box::new(move |e| {
        if let BootEvent::Phase { phase } = e {
            l1.lock().unwrap().push(format!("boot:{phase:?}"));
        }
    }));
    st.hubs.activation.subscribe(Box::new(move |e| {
        if let ActivationEvent::Reconciled { .. } = e {
            l2.lock().unwrap().push("activation:reconciled".into());
        }
    }));
    assert_eq!(boot::run_boot(&st).phase, BootPhase::Ready);
    let l = log.lock().unwrap();
    let pos = |s: &str| l.iter().position(|x| x == s).unwrap_or_else(|| panic!("{s} not in {l:?}"));
    assert!(pos("boot:Activation") < pos("activation:reconciled") && pos("activation:reconciled") < pos("boot:Ready"), "{l:?}");
}

#[test]
fn auto_deactivated_pushed_to_ui() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let (st, faces, _) = library3(d.path(), &eng);
    let col = library::create_collection(&st, "Tạm", None).unwrap();
    library::add_to_collection(&st, col.id, &faces[..1]).unwrap();
    activation::activate_collection(&st, col.id, Some(ScopeDto::Persistent), false, &mut noop).unwrap();
    // xoá collection bằng SQL trực tiếp (bỏ qua service) rồi mở lại app
    st.with_user(|u| {
        u.conn().execute("DELETE FROM u.collections WHERE collection_id = ?1", [col.id]).unwrap();
        Ok(())
    })
    .unwrap();
    drop(st);
    let st2 = new_state(d.path(), &eng);
    let got = Arc::new(Mutex::new(None));
    let g2 = got.clone();
    st2.hubs.activation.subscribe(Box::new(move |e| {
        if let ActivationEvent::Reconciled { report } = e {
            *g2.lock().unwrap() = Some(report);
        }
    }));
    boot::run_boot(&st2);
    let rep = got.lock().unwrap().clone().expect("Reconciled");
    assert_eq!(rep.auto_deactivated.len(), 1);
    assert_eq!(rep.auto_deactivated[0].cause, "collection_deleted");
}

#[test]
fn delete_collection_command_deactivates_first() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let (st, faces, _) = library3(d.path(), &eng);
    let col = library::create_collection(&st, "Poster", None).unwrap();
    library::add_to_collection(&st, col.id, &faces[..2]).unwrap();
    activation::activate_collection(&st, col.id, Some(ScopeDto::Persistent), false, &mut noop).unwrap();
    assert_eq!(eng.registrations().len(), 2);
    library::delete_collection(&st, col.id).unwrap();
    assert!(eng.registrations().is_empty(), "font của collection phải được gỡ");
    assert!(activation::state_list(&st).unwrap().iter().all(|r| r.reasons.is_empty() || !r.reasons.contains(&ReasonDto::Collection { collection_id: col.id })));
    // lần đối soát sau không còn gì để tự gỡ
    let got = Arc::new(Mutex::new(None));
    let g2 = got.clone();
    st.hubs.activation.subscribe(Box::new(move |e| {
        if let ActivationEvent::Reconciled { report } = e {
            *g2.lock().unwrap() = Some(report);
        }
    }));
    activation::reconcile_now(&st, true);
    assert!(got.lock().unwrap().clone().unwrap().auto_deactivated.is_empty());
}

#[test]
fn unplugged_active_font_stays_desired() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let (st, faces, dir) = library3(d.path(), &eng);
    activation::activate(&st, &faces[..1], Some(ScopeDto::Persistent), false, &mut noop).unwrap();
    let away = d.path().join("Fonts-rut");
    std::fs::rename(&dir, &away).unwrap();
    let src = sources::list_sources(&st).unwrap()[0].id;
    sources::rescan_sources(&st, Some(vec![src])).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let row = activation::state_list(&st).unwrap().into_iter().find(|r| r.hash_hex == faces[0].hash_hex).unwrap();
    assert_eq!((row.state, row.status), (DesiredStateDto::Active, ActivationStatusDto::MissingFile));
    std::fs::rename(&away, &dir).unwrap();
    sources::rescan_sources(&st, Some(vec![src])).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let row = activation::state_list(&st).unwrap().into_iter().find(|r| r.hash_hex == faces[0].hash_hex).unwrap();
    assert_eq!((row.state, row.status), (DesiredStateDto::Active, ActivationStatusDto::Registered));
}

#[test]
fn exit_shuts_down_session_only() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let (st, faces, _) = library3(d.path(), &eng);
    activation::activate(&st, &faces[..1], Some(ScopeDto::Session), false, &mut noop).unwrap();
    activation::activate(&st, &faces[1..2], Some(ScopeDto::Persistent), false, &mut noop).unwrap();
    assert_eq!(eng.registrations().len(), 2);
    on_exit(&st);
    let regs = eng.registrations();
    assert_eq!(regs.len(), 1);
    assert_eq!(regs[0].1, Scope::Persistent);
}

#[test]
fn set_keep_switches_scope_and_marks_onboarded() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let (st, faces, _) = library3(d.path(), &eng);
    activation::activate(&st, &faces[..1], None, false, &mut noop).unwrap();
    let scope_of = |st: &AppState| activation::state_list(st).unwrap()[0].scope;
    assert_eq!(scope_of(&st), ScopeDto::Session);
    assert!(!onboarding::onboarding_state(&st).unwrap().activation_onboarded);
    activation::set_keep(&st, true).unwrap();
    assert_eq!(scope_of(&st), ScopeDto::Persistent);
    let o = onboarding::onboarding_state(&st).unwrap();
    assert!(o.activation_onboarded && o.keep_active_on_exit && o.activation_supported);
    activation::set_keep(&st, false).unwrap();
    assert_eq!(scope_of(&st), ScopeDto::Session);
    assert_eq!(eng.registrations().len(), 1);
}

#[test]
fn gdi_engine_returns_unsupported() {
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(ActivationService::new(WindowsGdiEngine)));
    boot::run_boot(&st);
    let caps = activation::caps(&st);
    assert_eq!((caps.method.as_str(), caps.supported), ("gdi", false));
    let face = FaceKeyDto { hash_hex: "11".repeat(32), face_index: 0 };
    let e = activation::activate(&st, std::slice::from_ref(&face), None, false, &mut noop).unwrap_err();
    assert_eq!(e.code, "activation_unsupported");
    assert_eq!(activation::deactivate_all(&st, &mut noop).unwrap_err().code, "activation_unsupported");
    assert!(activation::state_list(&st).unwrap().is_empty());
    // xoá collection vẫn chạy trên nền tảng không hỗ trợ
    let c = library::create_collection(&st, "A", None).unwrap();
    library::delete_collection(&st, c.id).unwrap();
}
