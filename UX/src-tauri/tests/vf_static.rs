//! G7 qua lớp lệnh: cờ VF/static trong xung đột và lựa chọn đã nhớ cho cặp trùng tên.
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

fn state(root: &Path) -> (Arc<AppState>, Arc<FakeEngine>) {
    let eng = Arc::new(FakeEngine::new());
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    (st, eng)
}

#[test]
fn vf_policy_defaults_to_ask_roundtrips_and_ask_forgets_it() {
    let d = tempfile::tempdir().unwrap();
    let (st, _e) = state(d.path());
    assert_eq!(activation::vf_policy(&st).unwrap(), VfPolicyDto::Ask);
    for p in [VfPolicyDto::Vf, VfPolicyDto::Static, VfPolicyDto::Both] {
        activation::set_vf_policy(&st, p).unwrap();
        assert_eq!(activation::vf_policy(&st).unwrap(), p);
    }
    activation::set_vf_policy(&st, VfPolicyDto::Ask).unwrap();
    assert_eq!(activation::vf_policy(&st).unwrap(), VfPolicyDto::Ask, "Ask xoá lựa chọn đã nhớ");
    // đổi giá trị bậy trong DB (bản mới hơn / sửa tay) → hỏi mỗi lần, không lỗi
    st.with_user(|u| Ok(u.set_setting(activation::SETTING_VF_POLICY, "bay-ba")?)).unwrap();
    assert_eq!(activation::vf_policy(&st).unwrap(), VfPolicyDto::Ask);
}

#[test]
fn vf_policy_json_is_snake_case() {
    assert_eq!(serde_json::to_string(&VfPolicyDto::Static).unwrap(), "\"static\"");
    assert_eq!(serde_json::from_str::<VfPolicyDto>("\"vf\"").unwrap(), VfPolicyDto::Vf);
}

#[test]
fn conflict_json_says_which_side_is_variable() {
    let d = tempfile::tempdir().unwrap();
    let (st, eng) = state(d.path());
    let dir = d.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("roboto.ttf"), dir.join("roboto.ttf")).unwrap(); // static trong thư viện
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let row = library::query_faces(&st, &FaceQueryDto { limit: 5, ..Default::default() }).unwrap().items.remove(0);
    let key = FaceKeyDto { hash_hex: row.hash_hex, face_index: row.face_index };
    // bản VF do app khác đăng ký, trùng PostScript name với font static của thư viện
    let foreign = d.path().join("Thu vien.rightfontlibrary/fonts/Roboto[wght].ttf");
    std::fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    std::fs::copy(fixture("oswald_vf.ttf"), &foreign).unwrap();
    let foreign = std::fs::canonicalize(foreign).unwrap();
    eng.set_names_for(&foreign, &["Roboto-Regular"]);
    eng.register(&foreign, Scope::Persistent).unwrap();

    let rep = activation::activate(&st, std::slice::from_ref(&key), None, false, &mut |_| {}).unwrap();
    let v = serde_json::to_value(&rep.results[0].outcome).unwrap();
    assert_eq!(v["kind"], "needs_confirmation");
    assert_eq!(v["conflicts"][0]["variable"], false);
    assert_eq!(v["conflicts"][0]["existingVariable"], true);
}
