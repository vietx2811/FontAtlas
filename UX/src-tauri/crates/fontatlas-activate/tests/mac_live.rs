//! CoreText thật. Phần chỉ đọc chạy mặc định; phần đăng ký có `#[ignore]` **và** cổng
//! `FONTATLAS_LIVE_ACTIVATION=1`, chỉ chạy khi người dùng đồng ý. Chỉ dùng bản copy của fixture OFL
//! trong thư mục tạm; mọi đăng ký được gỡ qua `LiveGuard` kể cả khi panic.
#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use fontatlas_activate::{ActivationEngine, EngineError, MacCoreTextEngine, Scope};

const FIXTURE_PS: &str = "LibreBaskerville-Regular";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures/libre_baskerville.ttf")
}

/// Gỡ đăng ký khi ra khỏi phạm vi, bỏ qua lỗi.
struct LiveGuard<'a> {
    engine: &'a MacCoreTextEngine,
    path: PathBuf,
    scope: Scope,
}

impl Drop for LiveGuard<'_> {
    fn drop(&mut self) {
        let _ = self.engine.unregister(&self.path, self.scope);
    }
}

/// Cổng an toàn chung cho mọi test sống. `None` = bỏ qua (đã in lý do).
fn live_copy(dir: &Path, name: &str) -> Option<PathBuf> {
    if std::env::var("FONTATLAS_LIVE_ACTIVATION").as_deref() != Ok("1") {
        eprintln!("bỏ qua: cần FONTATLAS_LIVE_ACTIVATION=1 (và sự đồng ý của người dùng)");
        return None;
    }
    let e = MacCoreTextEngine;
    if e.available_postscript_names().unwrap().contains(FIXTURE_PS) {
        eprintln!("bỏ qua: {FIXTURE_PS} đã có trên máy — không đụng font thật của người dùng");
        return None;
    }
    let p = dir.join(name);
    std::fs::copy(fixture(), &p).unwrap();
    let p = std::fs::canonicalize(&p).unwrap();
    eprintln!(
        "test sống: đăng ký {} ({FIXTURE_PS}). Nếu còn sót sau test: Font Book → chuột phải Libre Baskerville → Remove.",
        p.display()
    );
    Some(p)
}

fn visible() -> bool {
    MacCoreTextEngine.available_postscript_names().unwrap().contains(FIXTURE_PS)
}

#[test]
fn mac_reads_ps_names_from_fixture() {
    assert_eq!(MacCoreTextEngine.postscript_names_in_file(&fixture()).unwrap(), vec![FIXTURE_PS.to_string()]);
}

#[test]
fn mac_lists_system_names() {
    assert!(MacCoreTextEngine.available_postscript_names().unwrap().contains("Helvetica"));
}

#[test]
fn mac_providers_of_system_font() {
    let v = MacCoreTextEngine.providers("Helvetica").unwrap();
    assert!(!v.is_empty());
    for p in &v {
        assert!(p.starts_with("/System/Library/"), "{}", p.display());
    }
}

#[test]
#[ignore = "live CoreText: chạy khi người dùng đồng ý"]
fn live_session_register_is_visible_and_idempotent() {
    let d = tempfile::tempdir().unwrap();
    let Some(p) = live_copy(d.path(), "libre.ttf") else { return };
    let e = MacCoreTextEngine;
    e.register(&p, Scope::Session).unwrap();
    let _g = LiveGuard { engine: &e, path: p.clone(), scope: Scope::Session };
    assert_eq!(e.register(&p, Scope::Session), Err(EngineError::AlreadyRegistered));
    assert!(visible());
    e.unregister(&p, Scope::Session).unwrap();
    assert_eq!(e.unregister(&p, Scope::Session), Err(EngineError::NotRegistered));
    assert!(!visible());
}

#[test]
#[ignore = "live CoreText: chạy khi người dùng đồng ý"]
fn live_wrong_scope_unregister_is_not_registered() {
    let d = tempfile::tempdir().unwrap();
    let Some(p) = live_copy(d.path(), "libre.ttf") else { return };
    let e = MacCoreTextEngine;
    e.register(&p, Scope::Persistent).unwrap();
    let _g = LiveGuard { engine: &e, path: p.clone(), scope: Scope::Persistent };
    assert_eq!(e.unregister(&p, Scope::Session), Err(EngineError::NotRegistered));
    e.unregister(&p, Scope::Persistent).unwrap();
    assert!(!visible());
}

#[test]
#[ignore = "live CoreText: chạy khi người dùng đồng ý"]
fn live_same_url_other_scope() {
    let d = tempfile::tempdir().unwrap();
    let Some(p) = live_copy(d.path(), "libre.ttf") else { return };
    let e = MacCoreTextEngine;
    e.register(&p, Scope::Session).unwrap();
    let _gs = LiveGuard { engine: &e, path: p.clone(), scope: Scope::Session };
    let second = e.register(&p, Scope::Persistent);
    let _gp = LiveGuard { engine: &e, path: p.clone(), scope: Scope::Persistent };
    // Kết quả này quyết định thứ tự của `set_scope` (Task 6); in ra để ghi vào báo cáo.
    eprintln!("KẾT QUẢ live_same_url_other_scope: register Persistent khi đang Session → {second:?}");
    assert!(matches!(second, Ok(()) | Err(EngineError::AlreadyRegistered)), "{second:?}");
}

#[test]
#[ignore = "live CoreText: chạy khi người dùng đồng ý"]
fn live_provider_paths_are_canonical() {
    // Đăng ký qua đường dẫn /tmp (symlink → /private/tmp); providers phải trả dạng canonical.
    let d = tempfile::Builder::new().tempdir_in("/tmp").unwrap();
    let Some(canon) = live_copy(d.path(), "libre.ttf") else { return };
    let e = MacCoreTextEngine;
    e.register(&canon, Scope::Session).unwrap();
    let _g = LiveGuard { engine: &e, path: canon.clone(), scope: Scope::Session };
    let prov = e.providers(FIXTURE_PS).unwrap();
    assert!(prov.contains(&canon), "{prov:?} không chứa {}", canon.display());
}
