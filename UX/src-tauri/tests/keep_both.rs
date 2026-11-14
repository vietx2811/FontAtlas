//! G7 + G7b "Giữ cả hai": VF của thư viện trùng PostScript name với static đã cài → bật BẢN SAO đổi tên; các trường hợp khác lùi về G7.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::activation::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
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

fn core_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

struct Fx {
    _d: tempfile::TempDir,
    st: Arc<AppState>,
    eng: Arc<FakeEngine>,
    vf: FaceKeyDto,
    stat: FaceKeyDto,
    vf_path: PathBuf,
    stat_path: PathBuf,
}

fn face_of(st: &AppState, family: &str) -> FaceKeyDto {
    let q = FaceQueryDto { text: Some(family.into()), limit: 50, ..Default::default() };
    let r = library::query_faces(st, &q).unwrap().items.into_iter().find(|r| r.family == family).unwrap();
    FaceKeyDto { hash_hex: r.hash_hex, face_index: r.face_index }
}

/// Thư viện có Oswald (VF thật) và Roboto (static) mà DB cho Roboto PS name của Oswald: một cặp trùng tên.
fn fx() -> Fx {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    let dir = d.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["oswald_vf.ttf", "roboto.ttf"] {
        std::fs::copy(core_fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    let ps: String = st.with_store(|s| Ok(s.conn().query_row("SELECT postscript_name FROM faces WHERE family_name='Oswald'", [], |r| r.get(0)).unwrap())).unwrap();
    st.with_store(|s| {
        s.conn().execute("UPDATE faces SET postscript_name=?1 WHERE family_name='Roboto'", [&ps]).unwrap();
        Ok(())
    })
    .unwrap();
    let (vf_path, stat_path) = (std::fs::canonicalize(dir.join("oswald_vf.ttf")).unwrap(), std::fs::canonicalize(dir.join("roboto.ttf")).unwrap());
    eng.set_names_for(&vf_path, &[ps.as_str()]);
    eng.set_names_for(&stat_path, &[ps.as_str()]);
    let (vf, stat) = (face_of(&st, "Oswald"), face_of(&st, "Roboto"));
    Fx { _d: d, st, eng, vf, stat, vf_path, stat_path }
}

fn kind(r: &FileResultDto) -> String {
    serde_json::to_value(&r.outcome).unwrap()["kind"].as_str().unwrap().to_string()
}

#[test]
fn vf_over_an_installed_static_activates_a_renamed_copy_and_leaves_the_original_alone() {
    let f = fx();
    activation::activate(&f.st, std::slice::from_ref(&f.stat), Some(ScopeDto::Session), false, &mut |_| {}).unwrap();
    assert_eq!(f.eng.registrations().len(), 1);

    let rep = activation::keep_both(&f.st, std::slice::from_ref(&f.vf), Some(ScopeDto::Session), &mut |_| {}).unwrap();
    assert_eq!(rep.results.len(), 1);
    let r = &rep.results[0];
    assert_eq!(r.hash_hex, f.vf.hash_hex, "kết quả trả dưới hash của font GỐC đã chọn");
    assert_eq!(kind(r), "activated");
    assert_eq!(r.warning.as_deref(), Some("renamed_copy"));
    let copy = r.path.clone().unwrap();
    assert!(copy.contains("derived") && copy.ends_with("oswald_vf-VF.ttf"), "{copy}");
    let regs: Vec<PathBuf> = f.eng.registrations().into_iter().map(|x| x.0).collect();
    assert!(regs.contains(&f.stat_path) && regs.iter().any(|p| p.to_str() == Some(copy.as_str())), "{regs:?}");
    assert!(!regs.contains(&f.vf_path), "file VF gốc KHÔNG được đăng ký");
    assert_eq!(regs.len(), 2);

    // lặp lại: idempotent, không tạo bản sao thứ hai
    let again = activation::keep_both(&f.st, std::slice::from_ref(&f.vf), Some(ScopeDto::Session), &mut |_| {}).unwrap();
    assert_eq!(kind(&again.results[0]), "already_active");
    assert_eq!(f.eng.registrations().len(), 2);
}

#[test]
fn static_over_an_installed_vf_falls_back_to_the_g7_behaviour_without_a_copy() {
    let f = fx();
    activation::activate(&f.st, std::slice::from_ref(&f.vf), Some(ScopeDto::Session), false, &mut |_| {}).unwrap();
    let rep = activation::keep_both(&f.st, std::slice::from_ref(&f.stat), Some(ScopeDto::Session), &mut |_| {}).unwrap();
    assert_eq!(kind(&rep.results[0]), "activated");
    assert_eq!(rep.results[0].hash_hex, f.stat.hash_hex);
    assert!(rep.results[0].warning.is_none(), "không có bản sao: lùi về G7 (đăng ký trùng tên)");
    let regs: Vec<PathBuf> = f.eng.registrations().into_iter().map(|x| x.0).collect();
    assert_eq!(regs.len(), 2);
    assert!(regs.contains(&f.vf_path) && regs.contains(&f.stat_path));
    assert!(!f._d.path().join("data/derived").exists(), "không tạo bản sao khi VF đã bật sẵn");
}

#[test]
fn without_any_clash_keep_both_is_a_plain_activation() {
    let f = fx();
    let rep = activation::keep_both(&f.st, std::slice::from_ref(&f.vf), Some(ScopeDto::Session), &mut |_| {}).unwrap();
    assert_eq!(kind(&rep.results[0]), "activated");
    assert!(rep.results[0].warning.is_none());
    assert_eq!(f.eng.registrations().len(), 1);
    assert_eq!(f.eng.registrations()[0].0, f.vf_path);
}

#[test]
fn a_failed_copy_falls_back_with_a_warning_code() {
    let f = fx();
    activation::activate(&f.st, std::slice::from_ref(&f.stat), Some(ScopeDto::Session), false, &mut |_| {}).unwrap();
    // làm bản sao không tạo được: thư mục <data>/derived là một FILE
    std::fs::write(f.st.dirs.data.join("derived"), b"x").unwrap();
    let rep = activation::keep_both(&f.st, std::slice::from_ref(&f.vf), Some(ScopeDto::Session), &mut |_| {}).unwrap();
    assert_eq!(kind(&rep.results[0]), "activated", "lùi về G7: vẫn bật");
    assert!(rep.results[0].warning.as_deref().is_some_and(|w| w.starts_with("rename_failed:")), "{:?}", rep.results[0].warning);
}
