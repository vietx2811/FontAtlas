use std::path::Path;

use fontatlas_indesign::{Product, SCRIPT_NAME, StartupInstaller, TargetStatus};

fn prefs() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    for (prod, ver, loc) in [("Adobe InDesign", "21.0", "en_US"), ("Adobe InDesign", "22.0", "vi_VN"), ("Adobe InCopy", "21.0", "en_US")] {
        std::fs::create_dir_all(d.path().join(prod).join(format!("Version {ver}")).join(loc).join("Scripts Panel")).unwrap();
    }
    // bản đã có thư mục nhưng chưa chạy lần nào (không có locale) và rác khác
    std::fs::create_dir_all(d.path().join("Adobe InDesign/Version 20.0")).unwrap();
    std::fs::create_dir_all(d.path().join("Adobe InDesign/Version 21.0/Scripts Panel")).unwrap();
    std::fs::write(d.path().join("Adobe InDesign/Version 21.0/notes.txt"), "x").unwrap();
    d
}

fn inst(root: &Path, broker: &str, v: &str) -> StartupInstaller {
    StartupInstaller::new(root, broker, v)
}

#[test]
fn finds_only_versions_that_have_run_and_have_a_locale() {
    let d = prefs();
    let t = inst(d.path(), "/x/broker.json", "0.3.0").targets();
    let got: Vec<_> = t.iter().map(|t| (t.product, t.version.as_str(), t.locale.as_str())).collect();
    assert_eq!(got, vec![(Product::InDesign, "21.0", "en_US"), (Product::InDesign, "22.0", "vi_VN"), (Product::InCopy, "21.0", "en_US")]);
    assert!(t[0].dir.ends_with("Version 21.0/en_US/Scripts/startup scripts"));
}

#[test]
fn install_status_remove() {
    let d = prefs();
    let i = inst(d.path(), "/Users/a b/Library/Application Support/com.x/broker.json", "0.3.0");
    assert!(i.status().iter().all(|(_, s)| *s == TargetStatus::NotInstalled));
    let r = i.install();
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|r| r.result == Ok(TargetStatus::Installed)));
    assert!(i.status().iter().all(|(_, s)| *s == TargetStatus::Installed));
    let text = std::fs::read_to_string(i.targets()[0].script_path()).unwrap();
    assert!(text.starts_with("// FontAtlas startup script 0.3.0"));
    assert!(text.contains("\"/Users/a b/Library/Application Support/com.x/broker.json\""));
    assert!(!text.contains('@'), "mọi chỗ giữ chỗ phải được thay: {}", text.lines().find(|l| l.contains('@')).unwrap_or(""));
    assert_eq!(i.remove().unwrap(), 3);
    assert!(i.status().iter().all(|(_, s)| *s == TargetStatus::NotInstalled));
    assert_eq!(i.remove().unwrap(), 0);
}

#[test]
fn path_with_quotes_and_backslashes_is_escaped_into_a_valid_js_string() {
    let d = prefs();
    let i = inst(d.path(), "/tmp/a\"b\\c\nd/broker.json", "1");
    let t = i.script_text();
    assert!(t.contains(r#"var CFG = "/tmp/a\"b\\c d/broker.json";"#), "{}", t.lines().find(|l| l.contains("CFG =")).unwrap());
}

#[test]
fn newer_app_version_or_moved_broker_makes_it_stale_and_reinstall_fixes() {
    let d = prefs();
    inst(d.path(), "/a/broker.json", "0.3.0").install();
    let newer = inst(d.path(), "/a/broker.json", "0.3.1");
    assert!(newer.status().iter().all(|(_, s)| *s == TargetStatus::Stale));
    let moved = inst(d.path(), "/b/broker.json", "0.3.0");
    assert!(moved.status().iter().all(|(_, s)| *s == TargetStatus::Stale));
    newer.install();
    assert!(newer.status().iter().all(|(_, s)| *s == TargetStatus::Installed));
    // ai đó sửa script
    let p = newer.targets()[0].script_path();
    let mut s = std::fs::read_to_string(&p).unwrap();
    s.push_str("\n// sửa tay\n");
    std::fs::write(&p, s).unwrap();
    assert_eq!(newer.status()[0].1, TargetStatus::Stale);
}

#[test]
fn a_foreign_file_with_the_same_name_is_never_overwritten_or_removed() {
    let d = prefs();
    let i = inst(d.path(), "/a/broker.json", "1");
    let first = i.targets()[0].clone();
    std::fs::create_dir_all(&first.dir).unwrap();
    std::fs::write(first.script_path(), "// của người khác\nalert(1)").unwrap();
    assert_eq!(i.target_status(&first), TargetStatus::Foreign);
    let r = i.install();
    assert!(r[0].result.is_err());
    assert!(r[1].result.is_ok() && r[2].result.is_ok(), "một nơi lỗi không chặn nơi khác");
    assert_eq!(std::fs::read_to_string(first.script_path()).unwrap(), "// của người khác\nalert(1)");
    i.remove().unwrap();
    assert!(first.script_path().exists());
    assert_eq!(SCRIPT_NAME, "FontAtlas_Startup.jsx");
}

#[test]
fn nothing_to_do_when_no_adobe_prefs() {
    let d = tempfile::tempdir().unwrap();
    let i = inst(d.path(), "/a/broker.json", "1");
    assert!(i.targets().is_empty() && i.install().is_empty() && i.remove().unwrap() == 0);
}
