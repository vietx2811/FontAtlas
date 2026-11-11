//! Activation broker trong app thật (AppState + FakeEngine): mở sau boot, bật font theo yêu cầu, đóng khi thoát.
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::activation::*;
use app_lib::services::lifecycle::on_exit;
use app_lib::services::{activation, boot, broker, sources};
use app_lib::state::{AppDirs, AppState};
use fontatlas_activate::{ActivationService, FakeEngine};
use fontatlas_broker::{BROKER_FILE, BrokerFile, SETTING_AUTO_ACTIVATION};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn booted(root: &Path) -> (Arc<AppState>, Arc<FakeEngine>) {
    let eng = Arc::new(FakeEngine::new());
    struct Shared(Arc<FakeEngine>);
    use fontatlas_activate::{ActivationEngine, EngineError, Scope};
    use std::collections::HashSet;
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
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("libre_baskerville.ttf"), dir.join("libre_baskerville.ttf")).unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    (st, eng)
}

fn post(bf: &BrokerFile, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", bf.port)).unwrap();
    let msg = format!(
        "POST /v1/ensure HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        bf.port,
        bf.token,
        body.len(),
        body
    );
    s.write_all(msg.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let status = out.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, out.split_once("\r\n\r\n").map(|x| x.1).unwrap_or("").to_string())
}

const REQ: &str = r#"{"v":1,"app":"Adobe Illustrator","doc":"logo.ai","fonts":[{"ps":"LibreBaskerville-Regular"}]}"#;

#[test]
fn broker_starts_after_boot_serves_ensure_and_stops_on_exit() {
    let d = tempfile::tempdir().unwrap();
    let (st, eng) = booted(d.path());
    let data = d.path().join("data");
    let bf = BrokerFile::read(&data).expect("broker.json sau boot");
    assert_eq!(Some(bf.port), broker::port(&st));
    assert_eq!(bf.token.len(), 64);

    let (code, body) = post(&bf, REQ);
    assert_eq!(code, 200, "{body}");
    assert!(body.contains("\"activated\""), "{body}");
    assert_eq!(eng.registrations().len(), 1);
    let rows = activation::state_list(&st).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reasons, vec![ReasonDto::Auto { app: "illustrator".into() }]);
    assert_eq!(rows[0].scope, ScopeDto::Session);

    on_exit(&st);
    assert!(!data.join(BROKER_FILE).exists(), "thoát app phải xoá broker.json");
    assert_eq!(broker::port(&st), None);
    // lúc thoát font session được gỡ như mọi kích hoạt khác
    assert!(eng.registrations().is_empty());
}

#[test]
fn switch_off_returns_503_without_activating() {
    let d = tempfile::tempdir().unwrap();
    let (st, eng) = booted(d.path());
    let bf = BrokerFile::read(&d.path().join("data")).unwrap();
    st.with_store(|s| Ok(s.set_setting(SETTING_AUTO_ACTIVATION, "0")?)).unwrap();
    let (code, _) = post(&bf, REQ);
    assert_eq!(code, 503);
    assert!(eng.registrations().is_empty());
    st.with_store(|s| Ok(s.set_setting(SETTING_AUTO_ACTIVATION, "1")?)).unwrap();
    assert_eq!(post(&bf, REQ).0, 200);
    on_exit(&st);
}

#[test]
fn unsupported_platform_engine_does_not_open_a_listener() {
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(ActivationService::new(fontatlas_activate::WindowsGdiEngine)));
    boot::run_boot(&st);
    assert_eq!(broker::port(&st), None);
    assert!(!d.path().join("data").join(BROKER_FILE).exists());
}

#[test]
fn status_pings_the_broker_instead_of_trusting_that_a_port_is_open() {
    let d = tempfile::tempdir().unwrap();
    let (st, _eng) = booted(d.path());
    assert!(broker::healthy(&st), "broker mở và trả lời ping");
    let s = app_lib::services::auto::status(&st).unwrap();
    assert!(s.broker_running);
    on_exit(&st);
    assert!(!broker::healthy(&st), "đã đóng → không còn khoẻ");
    assert!(!app_lib::services::auto::status(&st).unwrap().broker_running);
}
