use fontatlas_cep::{BUNDLE_ID, InstallError, PanelInstaller, PanelStatus};

struct Env {
    _d: tempfile::TempDir,
    ext: std::path::PathBuf,
    state: std::path::PathBuf,
    broker: std::path::PathBuf,
}

fn env() -> Env {
    let d = tempfile::tempdir().unwrap();
    let (ext, state) = (d.path().join("ext"), d.path().join("state"));
    let broker = d.path().join("data/broker.json");
    Env { _d: d, ext, state, broker }
}

fn inst(e: &Env, v: &str) -> PanelInstaller {
    PanelInstaller::new(&e.ext, &e.state, &e.broker, v)
}

#[test]
fn install_status_remove_cycle() {
    let e = env();
    let i = inst(&e, "0.3.0");
    assert_eq!(i.status(), PanelStatus::NotInstalled);
    assert!(!i.remove().unwrap());

    let r = i.install().unwrap();
    assert!(!r.replaced && r.restart_illustrator);
    assert_eq!(r.dir, e.ext.join(BUNDLE_ID));
    for f in ["CSXS/manifest.xml", "index.html", "main.js", "jsx/host.jsx", "mimetype", "fontatlas.json", "META-INF/signatures.xml"] {
        assert!(r.dir.join(f).is_file(), "{f}");
    }
    let cfg: serde_json::Value = serde_json::from_slice(&std::fs::read(r.dir.join("fontatlas.json")).unwrap()).unwrap();
    assert_eq!(cfg["broker"], e.broker.to_string_lossy().as_ref());
    assert!(std::fs::read_to_string(r.dir.join("CSXS/manifest.xml")).unwrap().contains("ExtensionBundleVersion=\"0.3.0\""));
    assert_eq!(i.status(), PanelStatus::Installed { version: "0.3.0".into() });

    assert!(i.remove().unwrap());
    assert_eq!(i.status(), PanelStatus::NotInstalled);
    assert!(!e.ext.join(BUNDLE_ID).exists());
    // khoá ký được giữ lại để lần cài sau dùng lại
    assert!(e.state.join("signing.pem").is_file());
}

#[test]
fn signing_key_is_private_and_reused() {
    use std::os::unix::fs::PermissionsExt;
    let e = env();
    inst(&e, "1").install().unwrap();
    let mode = std::fs::metadata(e.state.join("signing.pem")).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let cert1 = std::fs::read(e.state.join("signing.cert.der")).unwrap();
    inst(&e, "2").install().unwrap();
    assert_eq!(std::fs::read(e.state.join("signing.cert.der")).unwrap(), cert1, "cùng máy → cùng chứng chỉ");
}

#[test]
fn reinstall_replaces_and_stale_states_are_detected() {
    let e = env();
    inst(&e, "0.3.0").install().unwrap();
    // phiên bản mới của app → panel cũ báo Stale, cài lại thì ổn
    let newer = inst(&e, "0.3.1");
    assert!(matches!(newer.status(), PanelStatus::Stale { ref reason } if reason.contains("phiên bản")), "{:?}", newer.status());
    assert!(newer.install().unwrap().replaced);
    assert_eq!(newer.status(), PanelStatus::Installed { version: "0.3.1".into() });

    // broker.json chuyển chỗ (đổi thư mục dữ liệu)
    let moved = PanelInstaller::new(&e.ext, &e.state, e._d.path().join("elsewhere/broker.json"), "0.3.1");
    assert!(matches!(moved.status(), PanelStatus::Stale { ref reason } if reason.contains("broker")), "{:?}", moved.status());

    // ai đó sửa tệp panel → chữ ký hỏng
    std::fs::write(e.ext.join(BUNDLE_ID).join("main.js"), "alert(1)").unwrap();
    assert!(matches!(newer.status(), PanelStatus::Stale { ref reason } if reason.contains("main.js")), "{:?}", newer.status());
    // thêm tệp lạ
    newer.install().unwrap();
    std::fs::write(e.ext.join(BUNDLE_ID).join("extra.js"), "x").unwrap();
    assert!(matches!(newer.status(), PanelStatus::Stale { .. }));
    // mất khoá ký (xoá state) → chữ ký của khoá khác
    newer.install().unwrap();
    std::fs::remove_dir_all(&e.state).unwrap();
    assert!(matches!(newer.status(), PanelStatus::Stale { .. }));
}

#[test]
fn never_touches_a_foreign_directory() {
    let e = env();
    let dir = e.ext.join(BUNDLE_ID);
    std::fs::create_dir_all(dir.join("CSXS")).unwrap();
    std::fs::write(dir.join("CSXS/manifest.xml"), "<ExtensionManifest ExtensionBundleId=\"other.thing\"/>").unwrap();
    std::fs::write(dir.join("keep.txt"), "mine").unwrap();
    let i = inst(&e, "1");
    assert_eq!(i.status(), PanelStatus::Foreign);
    assert!(matches!(i.install(), Err(InstallError::Foreign(_))));
    assert!(matches!(i.remove(), Err(InstallError::Foreign(_))));
    assert_eq!(std::fs::read_to_string(dir.join("keep.txt")).unwrap(), "mine");
}

#[test]
fn leftover_temp_dir_from_a_crashed_install_does_not_break_the_next_one() {
    let e = env();
    std::fs::create_dir_all(e.ext.join(format!(".{BUNDLE_ID}.tmp-{}", std::process::id()))).unwrap();
    let i = inst(&e, "1");
    i.install().unwrap();
    assert_eq!(i.status(), PanelStatus::Installed { version: "1".into() });
}
