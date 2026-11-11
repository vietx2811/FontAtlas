//! "Tắt bản cũ và bật bản này" qua lớp lệnh: gỡ bản do app khác đăng ký, bật bản của Font Atlas, khôi phục được.
//! Chỉ dùng engine giả + font fixture OFL trong thư mục tạm.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::activation::*;
use app_lib::dto::library::*;
use app_lib::services::{activation, boot, library, sources};
use app_lib::state::{AppDirs, AppState};
use fontatlas_activate::{ActivationEngine, ActivationService, EngineError, FakeEngine, Scope};

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

fn noop(_: ProgressDto) {}

/// Thư viện có Roboto; một bản KHÁC nội dung của Roboto do "RightFont" đăng ký persistent.
fn setup(root: &Path) -> (Arc<AppState>, Arc<FakeEngine>, FaceKeyDto, PathBuf) {
    let eng = Arc::new(FakeEngine::new());
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("roboto.ttf"), dir.join("roboto.ttf")).unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let row = library::query_faces(&st, &FaceQueryDto { limit: 5, ..Default::default() }).unwrap().items.remove(0);
    let key = FaceKeyDto { hash_hex: row.hash_hex, face_index: row.face_index };
    let foreign = root.join("Thu vien.rightfontlibrary/fonts/r.ttf");
    std::fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    std::fs::write(&foreign, b"ban khac cua roboto").unwrap();
    let foreign = std::fs::canonicalize(foreign).unwrap();
    eng.set_names_for(&foreign, &["Roboto-Regular"]);
    eng.register(&foreign, Scope::Persistent).unwrap();
    (st, eng, key, foreign)
}

#[test]
fn replace_swaps_the_foreign_registration_and_restore_brings_it_back() {
    let d = tempfile::tempdir().unwrap();
    let (st, eng, key, foreign) = setup(d.path());
    let ask = activation::activate(&st, &[key.clone()], None, false, &mut noop).unwrap();
    assert!(matches!(ask.results[0].outcome, FileOutcomeDto::NeedsConfirmation { .. }), "{:?}", ask.results[0].outcome);

    let rep = activation::replace(&st, &[key.clone()], None, &mut noop).unwrap();
    assert!(matches!(rep.results[0].outcome, FileOutcomeDto::Activated), "{:?}", rep.results[0].outcome);
    let r = &rep.results[0].replaced;
    assert_eq!(r.len(), 1);
    assert_eq!((r[0].owner, r[0].scopes.clone(), r[0].path.as_str()), (OwnerDto::RightFont, vec![ScopeDto::Persistent], foreign.to_str().unwrap()));
    let regs = eng.registrations();
    assert_eq!(regs.len(), 1);
    assert_eq!(regs[0].1, Scope::Session, "chỉ còn bản của Font Atlas");

    assert_eq!(activation::restore_replaced(&st, r[0].id).unwrap(), RestoreOutcomeDto::Restored);
    assert_eq!(eng.registrations(), vec![(foreign, Scope::Persistent)]);
    assert_eq!(activation::restore_replaced(&st, r[0].id).unwrap(), RestoreOutcomeDto::AlreadyRestored);
    assert_eq!(activation::restore_replaced(&st, 12345).unwrap_err().code, "activation_not_found");
}

#[test]
fn replace_without_a_conflict_reports_no_replaced_entries() {
    let d = tempfile::tempdir().unwrap();
    let (st, eng, key, foreign) = setup(d.path());
    eng.unregister(&foreign, Scope::Persistent).unwrap();
    let rep = activation::replace(&st, &[key], None, &mut noop).unwrap();
    assert!(matches!(rep.results[0].outcome, FileOutcomeDto::Activated));
    assert!(rep.results[0].replaced.is_empty());
}

#[test]
fn replace_json_carries_the_replaced_list_in_camel_case() {
    let d = tempfile::tempdir().unwrap();
    let (st, _eng, key, _foreign) = setup(d.path());
    let rep = activation::replace(&st, &[key], None, &mut noop).unwrap();
    let v = serde_json::to_value(&rep.results[0]).unwrap();
    assert_eq!(v["replaced"][0]["owner"], "right_font");
    assert_eq!(v["replaced"][0]["scopes"][0], "persistent");
    assert!(v["replaced"][0]["id"].is_number());
}
