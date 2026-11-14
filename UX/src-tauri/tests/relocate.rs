//! B7 qua lớp ứng dụng: lệnh chuyển đăng ký sang bản chép trong Organized Library.
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

#[test]
fn relocate_command_switches_to_the_copy_and_reports() {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    let orig = d.path().join("Fonts");
    let org = d.path().join("Organized");
    std::fs::create_dir_all(&orig).unwrap();
    std::fs::create_dir_all(&org).unwrap();
    std::fs::copy(fixture("libre_baskerville.ttf"), orig.join("lb.ttf")).unwrap();
    std::fs::copy(fixture("libre_baskerville.ttf"), org.join("lb.ttf")).unwrap();
    sources::add_source(&st, orig.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let row = library::query_faces(&st, &FaceQueryDto { limit: 5, ..Default::default() }).unwrap().items.remove(0);
    let key = FaceKeyDto { hash_hex: row.hash_hex, face_index: row.face_index };
    activation::activate(&st, std::slice::from_ref(&key), Some(ScopeDto::Persistent), false, &mut |_| {}).unwrap();
    let old = eng.registrations()[0].0.clone();
    assert!(old.starts_with(std::fs::canonicalize(&orig).unwrap()));

    // thêm Organized Library như một nguồn (bản chép cùng hash → thêm vị trí)
    sources::add_source(&st, org.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));

    let rep = activation::relocate(&st, std::slice::from_ref(&key), org.to_str(), &mut |_| {}).unwrap();
    assert_eq!(serde_json::to_value(&rep.results[0].outcome).unwrap()["kind"], "activated");
    let regs = eng.registrations();
    assert_eq!(regs.len(), 1);
    assert!(regs[0].0.starts_with(std::fs::canonicalize(&org).unwrap()), "{regs:?}");
    assert_eq!(regs[0].1, Scope::Persistent, "giữ nguyên scope");
    // người dùng chưa bấm thì không tự đổi: một lần đối soát không kéo đăng ký đi đâu cả
    activation::reconcile_now(&st, false);
    assert_eq!(eng.registrations(), regs);

    // Hoàn tác: đưa về đường cũ đã ghi, kết quả theo file
    let back = activation::relocate_to(&st, &[RelocateTargetDto { face: key.clone(), path: old.to_str().unwrap().to_string() }], &mut |_| {}).unwrap();
    assert_eq!(serde_json::to_value(&back.results[0].outcome).unwrap()["kind"], "activated");
    assert_eq!(eng.registrations(), vec![(old.clone(), Scope::Persistent)]);
    let missing = activation::relocate_to(&st, &[RelocateTargetDto { face: key.clone(), path: d.path().join("khong-co.ttf").to_str().unwrap().to_string() }], &mut |_| {}).unwrap();
    assert_eq!(serde_json::to_value(&missing.results[0].outcome).unwrap()["kind"], "missing_file");
    assert_eq!(serde_json::to_value(&RelocateTargetDto { face: key, path: "/x".into() }).unwrap()["face"]["hashHex"].as_str().map(|s| s.len()), Some(64));
}
