mod common;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use common::*;
use fontatlas_activate::FakeEngine;
use fontatlas_broker::{BROKER_FILE, BrokerFile, OwnedBackend, SETTING_AUTO_ACTIVATION, Server, ServerConfig};

struct Resp {
    status: u16,
    head: String,
    body: String,
}

/// Một yêu cầu thô, một kết nối. `host` None = Host đúng của server.
fn raw(srv: &Server, method: &str, path: &str, extra: &[(&str, &str)], body: &str, host: Option<&str>) -> Resp {
    let port = srv.addr().port();
    let host = host.map(String::from).unwrap_or_else(|| format!("127.0.0.1:{port}"));
    let mut s = TcpStream::connect(srv.addr()).unwrap();
    let mut msg = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n", body.len());
    for (k, v) in extra {
        msg.push_str(&format!("{k}: {v}\r\n"));
    }
    msg.push_str("\r\n");
    s.write_all(msg.as_bytes()).unwrap();
    s.write_all(body.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    Resp { status, head: head.to_string(), body: body.to_string() }
}

fn auth(srv: &Server) -> String {
    format!("Bearer {}", srv.token())
}

fn post_ensure(srv: &Server, body: &str) -> Resp {
    let a = auth(srv);
    raw(srv, "POST", "/v1/ensure", &[("Authorization", &a), ("Content-Type", "application/json")], body, None)
}

fn start(names: &[&str], burst: u32) -> (Env, Server, Arc<OwnedBackend<FakeEngine>>) {
    let (env, b) = backend(names);
    let b = Arc::new(b);
    let mut cfg = ServerConfig::new(env.dir.path());
    cfg.burst = burst;
    cfg.per_sec = 1;
    let srv = Server::start(cfg, b.clone()).unwrap();
    (env, srv, b)
}

const ONE: &str = r#"{"v":1,"app":"photoshop","doc":"a.psd","fonts":[{"ps":"LibreBaskerville-Regular"},{"ps":"Nope-Regular"}]}"#;

#[test]
fn ensure_happy_path_over_http() {
    let (_e, srv, b) = start(&[LIBRE], 100);
    let r = post_ensure(&srv, ONE);
    assert_eq!(r.status, 200, "{}", r.body);
    let j: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    assert_eq!(j["ok"], true);
    assert_eq!(j["found"], serde_json::json!([LIBRE_PS]));
    assert_eq!(j["missing"], serde_json::json!(["Nope-Regular"]));
    assert_eq!(j["results"][0]["status"], "activated");
    assert_eq!(j["results"][1]["status"], "missing");
    assert!(r.head.to_lowercase().contains("cache-control: no-store"));
    assert_eq!(b.service.engine().registrations().len(), 1);
}

#[test]
fn ping_needs_token_too() {
    let (_e, srv, _b) = start(&[LIBRE], 100);
    assert_eq!(raw(&srv, "GET", "/v1/ping", &[], "", None).status, 401);
    let a = auth(&srv);
    let r = raw(&srv, "GET", "/v1/ping", &[("Authorization", &a)], "", None);
    assert_eq!(r.status, 200);
    assert!(r.body.contains("\"ok\":true"));
}

#[test]
fn missing_or_wrong_token_is_401_and_does_nothing() {
    let (_e, srv, b) = start(&[LIBRE], 100);
    assert_eq!(raw(&srv, "POST", "/v1/ensure", &[], ONE, None).status, 401);
    let wrong = format!("Bearer {}", "0".repeat(srv.token().len()));
    assert_eq!(raw(&srv, "POST", "/v1/ensure", &[("Authorization", &wrong)], ONE, None).status, 401);
    assert_eq!(raw(&srv, "POST", "/v1/ensure", &[("Authorization", srv.token())], ONE, None).status, 401, "thiếu chữ Bearer");
    let short = "Bearer abc";
    assert_eq!(raw(&srv, "POST", "/v1/ensure", &[("Authorization", short)], ONE, None).status, 401);
    assert!(b.service.engine().registrations().is_empty());
}

#[test]
fn foreign_host_is_rejected_dns_rebinding() {
    let (_e, srv, b) = start(&[LIBRE], 100);
    let a = auth(&srv);
    let r = raw(&srv, "POST", "/v1/ensure", &[("Authorization", &a)], ONE, Some("evil.example.com"));
    assert_eq!(r.status, 403);
    let r = raw(&srv, "POST", "/v1/ensure", &[("Authorization", &a)], ONE, Some("127.0.0.1:1"));
    assert_eq!(r.status, 403, "sai cổng");
    assert!(b.service.engine().registrations().is_empty());
    let ok = raw(&srv, "GET", "/v1/ping", &[("Authorization", &a)], "", Some(&format!("localhost:{}", srv.addr().port())));
    assert_eq!(ok.status, 200);
}

#[test]
fn web_origins_are_rejected_even_with_token() {
    let (_e, srv, b) = start(&[LIBRE], 100);
    let a = auth(&srv);
    for o in ["https://evil.example.com", "http://localhost:3000", "https://www.figma.com", "chrome-extension://abc"] {
        let r = raw(&srv, "POST", "/v1/ensure", &[("Authorization", &a), ("Origin", o)], ONE, None);
        assert_eq!(r.status, 403, "{o}");
    }
    assert!(b.service.engine().registrations().is_empty());
}

#[test]
fn cep_null_origin_works_with_preflight_and_cors_header() {
    let (_e, srv, _b) = start(&[LIBRE], 100);
    let pf = raw(
        &srv,
        "OPTIONS",
        "/v1/ensure",
        &[("Origin", "null"), ("Access-Control-Request-Method", "POST"), ("Access-Control-Request-Headers", "authorization")],
        "",
        None,
    );
    assert_eq!(pf.status, 204);
    let h = pf.head.to_lowercase();
    assert!(h.contains("access-control-allow-origin: null"), "{h}");
    assert!(h.contains("access-control-allow-headers: authorization"), "{h}");
    let a = auth(&srv);
    let r = raw(&srv, "POST", "/v1/ensure", &[("Authorization", &a), ("Origin", "null")], ONE, None);
    assert_eq!(r.status, 200);
    assert!(r.head.to_lowercase().contains("access-control-allow-origin: null"));
    // preflight từ origin web: không được cấp
    let bad = raw(&srv, "OPTIONS", "/v1/ensure", &[("Origin", "https://evil.example.com")], "", None);
    assert_eq!(bad.status, 403);
    // OPTIONS không Origin không phải preflight
    assert_eq!(raw(&srv, "OPTIONS", "/v1/ensure", &[], "", None).status, 400);
}

#[test]
fn extra_origin_can_be_allowed_explicitly() {
    let (env, b) = backend(&[LIBRE]);
    let mut cfg = ServerConfig::new(env.dir.path());
    cfg.extra_origins = vec!["https://plugin.example".into()];
    let srv = Server::start(cfg, Arc::new(b)).unwrap();
    let a = auth(&srv);
    let r = raw(&srv, "POST", "/v1/ensure", &[("Authorization", &a), ("Origin", "https://plugin.example")], ONE, None);
    assert_eq!(r.status, 200);
}

#[test]
fn routes_methods_and_bodies() {
    let (_e, srv, _b) = start(&[LIBRE], 100);
    let a = auth(&srv);
    let h = [("Authorization", a.as_str())];
    assert_eq!(raw(&srv, "GET", "/nope", &h, "", None).status, 404);
    assert_eq!(raw(&srv, "GET", "/v1/ensure", &h, "", None).status, 405);
    assert_eq!(raw(&srv, "POST", "/v1/ping", &h, "", None).status, 405);
    let r = raw(&srv, "POST", "/v1/ensure", &h, "{khong phai json", None);
    assert_eq!(r.status, 400);
    assert!(r.body.contains("bad_json"));
    let r = raw(&srv, "POST", "/v1/ensure", &h, r#"{"v":9,"app":"x","fonts":[]}"#, None);
    assert_eq!(r.status, 400);
    let huge = format!(r#"{{"v":1,"app":"x","fonts":[{{"ps":"{}"}}]}}"#, "a".repeat(200_000));
    assert_eq!(raw(&srv, "POST", "/v1/ensure", &h, &huge, None).status, 413);
    let r = raw(&srv, "POST", "/v1/ensure", &h, r#"{"v":1,"app":"x","fonts":[{"ps":"a\u0007b"}]}"#, None);
    assert_eq!(r.status, 400);
}

#[test]
fn rate_limit_returns_429_with_retry_after() {
    let (_e, srv, _b) = start(&[LIBRE], 3);
    let a = auth(&srv);
    let codes: Vec<u16> = (0..6).map(|_| raw(&srv, "GET", "/v1/ping", &[("Authorization", &a)], "", None).status).collect();
    assert_eq!(&codes[..3], &[200, 200, 200]);
    assert!(codes[3..].contains(&429), "{codes:?}");
    let r = raw(&srv, "GET", "/v1/ping", &[("Authorization", &a)], "", None);
    if r.status == 429 {
        assert!(r.head.to_lowercase().contains("retry-after"));
    }
}

#[test]
fn disabled_setting_returns_503() {
    let (_e, srv, b) = start(&[LIBRE], 100);
    b.store.lock().unwrap().set_setting(SETTING_AUTO_ACTIVATION, "0").unwrap();
    let r = post_ensure(&srv, ONE);
    assert_eq!(r.status, 503);
    assert!(b.service.engine().registrations().is_empty());
}

#[test]
fn broker_file_is_private_and_removed_on_shutdown() {
    let (env, srv, _b) = start(&[LIBRE], 100);
    let p = env.dir.path().join(BROKER_FILE);
    assert!(srv.addr().ip().is_loopback());
    let bf = BrokerFile::read(env.dir.path()).unwrap();
    assert_eq!(bf.port, srv.addr().port());
    assert_eq!(bf.token, srv.token());
    assert_eq!(bf.token.len(), 64);
    assert_eq!(bf.pid, std::process::id());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
    }
    // không còn file tạm
    let leftovers: Vec<_> = std::fs::read_dir(env.dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty());
    srv.shutdown();
    assert!(!p.exists());
}

#[test]
fn restart_gets_new_port_or_token_and_overwrites_file() {
    let (env, b) = backend(&[LIBRE]);
    let b = Arc::new(b);
    let s1 = Server::start(ServerConfig::new(env.dir.path()), b.clone()).unwrap();
    let t1 = s1.token().to_string();
    s1.shutdown();
    let s2 = Server::start(ServerConfig::new(env.dir.path()), b).unwrap();
    assert_ne!(s2.token(), t1);
    assert_eq!(BrokerFile::read(env.dir.path()).unwrap().token, s2.token());
}

#[test]
fn concurrent_clients_register_once() {
    let (_e, srv, b) = start(&[LIBRE], 1000);
    let srv = Arc::new(srv);
    let hs: Vec<_> = (0..8)
        .map(|_| {
            let srv = srv.clone();
            std::thread::spawn(move || post_ensure(&srv, ONE).status)
        })
        .collect();
    for h in hs {
        assert_eq!(h.join().unwrap(), 200);
    }
    assert_eq!(b.service.engine().registrations().len(), 1);
    let rows = b.store.lock().unwrap().activation_rows().unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn token_is_random_per_start() {
    let a = fontatlas_broker::new_token().unwrap();
    let b = fontatlas_broker::new_token().unwrap();
    assert_ne!(a, b);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
}
