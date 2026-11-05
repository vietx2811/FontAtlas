//! Cấu hình Tauri: CSP, không sidecar, quyền tối thiểu, giữ cấu hình cửa sổ của Plan 6b T1.
use std::path::Path;

use serde_json::Value;

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn json(rel: &str) -> Value {
    serde_json::from_str(&read(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn directive(csp: &str, name: &str) -> Vec<String> {
    csp.split(';')
        .map(str::trim)
        .find_map(|d| d.strip_prefix(name).filter(|r| r.starts_with(' ')).map(|r| r.split_whitespace().map(String::from).collect()))
        .unwrap_or_default()
}

#[test]
fn csp_has_required_sources() {
    let conf = json("tauri.conf.json");
    let csp = conf["app"]["security"]["csp"].as_str().expect("csp");
    for (name, want) in [
        ("default-src", vec!["'self'"]),
        ("script-src", vec!["'self'"]),
        ("style-src", vec!["'self'", "'unsafe-inline'"]),
        ("img-src", vec!["'self'", "data:", "blob:", "fontatlas-preview:", "http://fontatlas-preview.localhost"]),
        ("font-src", vec!["'self'", "fontatlas-font:", "http://fontatlas-font.localhost"]),
        ("connect-src", vec!["'self'", "ipc:", "http://ipc.localhost", "fontatlas-preview:", "http://fontatlas-preview.localhost"]),
    ] {
        let got = directive(csp, name);
        assert_eq!(got, want, "{name}");
    }
    assert!(!csp.contains("asset:"), "không còn asset: protocol");
}

#[test]
fn no_external_bin_and_no_sidecar() {
    let conf = json("tauri.conf.json");
    assert!(conf["bundle"].get("externalBin").is_none());
    let cargo = read("Cargo.toml");
    for dep in ["qdrant-client", "reqwest", "uuid", "chrono", "tauri-plugin-shell", "tauri-plugin-fs"] {
        assert!(!cargo.contains(dep), "{dep} còn trong Cargo.toml");
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    assert!(!root.join("python-sidecar").exists() && !root.join("binaries").exists());
    for f in ["qdrant.rs", "db.rs", "logger.rs", "lib_logger_snippet.rs"] {
        assert!(!Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(f).exists(), "{f}");
    }
}

#[test]
fn capabilities_minimal() {
    let c = json("capabilities/default.json");
    let mut got: Vec<&str> = c["permissions"].as_array().unwrap().iter().map(|p| p.as_str().expect("chỉ quyền dạng chuỗi")).collect();
    got.sort();
    assert_eq!(got, ["core:default", "core:window:allow-start-dragging", "dialog:allow-open"]);
}

#[test]
fn window_chrome_settings_preserved() {
    let conf = json("tauri.conf.json");
    assert_eq!(conf["app"]["macOSPrivateApi"], true);
    let w = &conf["app"]["windows"][0];
    assert_eq!(w["transparent"], true);
    assert!(w["windowEffects"].is_object(), "windowEffects");
    assert!(read("Cargo.toml").contains("macos-private-api"));
}

#[test]
fn commands_touching_store_are_async() {
    let src = read("src/commands.rs");
    let v = app_lib::commands::async_violations(app_lib::commands::COMMANDS, &src);
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn async_violations_detects_bad_tables() {
    use app_lib::commands::{CommandSig, async_violations};
    let bad = [CommandSig { name: "query_faces", args: &[], result: "", is_async: false }];
    let v = async_violations(&bad, "pub fn query_faces(");
    assert!(v.iter().any(|m| m.contains("SYNC_OK")), "{v:?}");
    let liar = [CommandSig { name: "query_faces", args: &[], result: "", is_async: true }];
    assert!(!async_violations(&liar, "pub fn query_faces(").is_empty());
    let naked = "#[tauri::command]\npub fn heavy(state: X) {}";
    assert!(async_violations(&[], naked).iter().any(|m| m.contains("heavy")));
}

#[test]
fn logging_is_rust_only() {
    let caps = read("capabilities/default.json");
    assert!(!caps.contains("\"log:"));
}
