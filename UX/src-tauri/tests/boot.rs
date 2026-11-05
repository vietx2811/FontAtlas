use std::path::Path;
use std::sync::{Arc, Mutex};

use app_lib::dto::events::BootEvent;
use app_lib::dto::settings::BootPhase;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, onboarding};
use app_lib::state::{AppDirs, AppState};

fn state(dir: &Path) -> Arc<AppState> {
    AppState::new(AppDirs::under(dir), Box::new(NoActivation))
}

fn sha(p: &Path) -> Option<Vec<u8>> {
    std::fs::read(p).ok()
}

/// DB cũ tối thiểu: bảng fonts(id, file_path, is_favorite, tags) + một yêu thích.
fn make_legacy(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("fontatlas.db");
    let c = rusqlite::Connection::open(&p).unwrap();
    c.execute_batch(
        "CREATE TABLE fonts(id TEXT PRIMARY KEY, file_path TEXT, is_favorite INTEGER, tags TEXT);
         INSERT INTO fonts VALUES ('A','/nowhere/a.ttf',1,'[\"x\"]'), ('B','/nowhere/b.ttf',0,'[]');",
    )
    .unwrap();
    p
}

#[test]
fn fresh_boot_reaches_ready() {
    let d = tempfile::tempdir().unwrap();
    let st = state(d.path());
    let events = Arc::new(Mutex::new(Vec::new()));
    let e2 = events.clone();
    st.hubs.boot.subscribe(Box::new(move |e| e2.lock().unwrap().push(e)));
    let s = boot::run_boot(&st);
    assert_eq!(s.phase, BootPhase::Ready);
    assert!(s.store_error.is_none() && !s.legacy.found);
    assert!(st.store().is_ok() && st.user().is_ok());
    let phases: Vec<_> = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| if let BootEvent::Phase { phase } = e { Some(*phase) } else { None })
        .collect();
    assert_eq!(phases, [BootPhase::Starting, BootPhase::Indexing, BootPhase::Activation, BootPhase::Ready]);
}

#[test]
fn legacy_detected_not_imported_until_command() {
    let d = tempfile::tempdir().unwrap();
    make_legacy(d.path());
    let st = state(d.path());
    let s = boot::run_boot(&st);
    assert!(s.legacy.found && s.legacy.imported.is_none());
    assert_eq!(s.legacy.preview.as_ref().map(|p| (p.fonts, p.favorites)), Some((2, 1)));
    assert!(st.with_store(|x| Ok(x.collections()?.len())).unwrap() == 0);
    let l = boot::legacy_import(&st).unwrap();
    assert!(l.error.is_none());
    assert_eq!(l.imported.as_ref().map(|i| i.fonts), Some(2));
    assert!(boot::legacy_state(&st).imported.is_some());
    // nhập lại (retry) không lỗi
    assert!(boot::legacy_import(&st).unwrap().imported.is_some());
}

#[test]
fn corrupt_legacy_db_is_reported_and_untouched() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("fontatlas.db");
    std::fs::write(&p, vec![7u8; 4096]).unwrap();
    let before = sha(&p);
    let st = state(d.path());
    let s = boot::run_boot(&st);
    assert_eq!(s.phase, BootPhase::Ready);
    assert!(s.legacy.found && s.legacy.preview.is_none());
    assert_eq!(s.legacy.error.as_ref().map(|e| e.code.as_str()), Some("legacy"));
    assert_eq!(sha(&p), before);
    let l = boot::legacy_import(&st).unwrap();
    assert_eq!(l.error.map(|e| e.code), Some("legacy".into()));
    assert_eq!(sha(&p), before);
}

#[test]
fn newer_user_db_fails_without_writing() {
    let d = tempfile::tempdir().unwrap();
    let u = d.path().join("user.db");
    {
        let c = rusqlite::Connection::open(&u).unwrap();
        c.execute_batch(&format!("CREATE TABLE keep(x); PRAGMA user_version = {};", fontatlas_store::USER_VERSION + 1)).unwrap();
    }
    let before = sha(&u);
    let st = state(d.path());
    let s = boot::run_boot(&st);
    assert_eq!(s.phase, BootPhase::Failed);
    assert_eq!(s.store_error.as_ref().map(|e| e.code.as_str()), Some("newer_schema"));
    assert_eq!(sha(&u), before);
    assert!(!d.path().join("backups").exists() || std::fs::read_dir(d.path().join("backups")).unwrap().count() == 0);
    assert!(!d.path().join("library.db").exists());
    assert_eq!(st.store().err().map(|e| e.code), Some("store_unavailable".into()));
}

#[test]
fn onboarding_step_roundtrip() {
    let d = tempfile::tempdir().unwrap();
    let st = state(d.path());
    boot::run_boot(&st);
    let o = onboarding::onboarding_state(&st).unwrap();
    assert!(o.step.is_none() && !o.has_sources && !o.activation_supported && o.font_managers.len() == 5);
    onboarding::set_onboarding_step(&st, Some("o3".into())).unwrap();
    assert_eq!(onboarding::onboarding_state(&st).unwrap().step.as_deref(), Some("o3"));
    onboarding::set_onboarding_step(&st, None).unwrap();
    assert!(onboarding::onboarding_state(&st).unwrap().step.is_none());
}
