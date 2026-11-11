#![cfg(target_os = "macos")]
//! Thật sự đi qua CoreText: nguồn font request chạy trong tiến trình test, một tiến trình khác hỏi font không có.
//! Chỉ TRẢ URL file fixture (không đăng ký font nào vào hệ thống), nên không để lại gì sau test.
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fontatlas_broker::{EnsureRequest, EnsureResponse, FontResult, FontStatus};
use fontatlas_fontreq::{Answer, FontRequestSource, Observer, PidAppNamer, Request, Responder};

type Seen = Vec<(i32, Request, Answer, Duration)>;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures/libre_baskerville.ttf").canonicalize().unwrap()
}

fn requester() -> PathBuf {
    let exe = std::env::current_exe().unwrap(); // target/<profile>/deps/coretext_e2e-xxxx
    exe.parent().unwrap().parent().unwrap().join("examples").join("requester")
}

/// Hỏi font bằng tiến trình khác, có hạn: hệ thống đã từng giữ yêu cầu chờ XPC rất lâu (1/6 lần chạy, nguồn của ta không
/// hề được gọi), nên test không được phép treo theo. Quá hạn → giết tiến trình con và báo lỗi rõ.
fn ask(name: &str) -> String {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = Command::new(requester()).arg(name).stdout(Stdio::piped()).spawn().expect("chạy requester (cargo build --examples)");
    let t0 = std::time::Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if t0.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("requester treo >30 s khi hỏi {name} (hệ thống không gọi nguồn?)");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    out
}

#[test]
fn another_process_gets_the_font_we_serve_and_unknown_names_get_nothing() {
    let path = fixture();
    let asked = Arc::new(AtomicUsize::new(0));
    let a2 = asked.clone();
    let want = path.to_string_lossy().into_owned();
    let responder = Arc::new(Responder::new(
        move |q: &EnsureRequest| {
            a2.fetch_add(1, Ordering::SeqCst);
            let ps = q.fonts[0].ps.clone().unwrap_or_default();
            let hit = ps == "LibreBaskerville-Regular";
            Ok(EnsureResponse {
                ok: true,
                v: 1,
                app: q.app.clone(),
                results: vec![FontResult {
                    name: ps,
                    status: if hit { FontStatus::Activated } else { FontStatus::Missing },
                    path: hit.then(|| want.clone()),
                }],
                found: vec![],
                missing: vec![],
            })
        },
        || true,
        PidAppNamer,
    ));
    let log: Arc<Mutex<Seen>> = Arc::default();
    let l2 = log.clone();
    let obs: Arc<Observer> = Arc::new(move |pid, req, ans, d| l2.lock().unwrap().push((pid, req.clone(), ans.clone(), d)));
    let src = FontRequestSource::start(responder, Some(obs)).expect("nguồn font request");

    let served = ask("LibreBaskerville-Regular");
    assert!(served.contains("got=LibreBaskerville-Regular"), "app hỏi phải nhận đúng font ta trả: {served}");
    assert!(served.contains("url=yes"), "{served}");

    let none = ask("FontAtlasNoSuchFont-Regular");
    assert!(!none.contains("got=FontAtlasNoSuchFont-Regular"), "{none}");

    drop(src);
    let log = log.lock().unwrap();
    let hit = log.iter().find(|(_, r, _, _)| r.ps.as_deref() == Some("LibreBaskerville-Regular")).expect("nguồn nhận request");
    assert_eq!(hit.2, Answer::Files(vec![path]));
    assert!(hit.3 < Duration::from_secs(2), "trả lời chậm: {:?}", hit.3);
    assert!(log.iter().any(|(_, r, a, _)| r.ps.as_deref() == Some("FontAtlasNoSuchFont-Regular") && *a == Answer::Nothing));
    assert!(asked.load(Ordering::SeqCst) >= 2);
}

#[test]
fn source_stops_on_drop_and_can_start_again() {
    let mk = || Arc::new(Responder::new(|_q: &EnsureRequest| Err(fontatlas_broker::EnsureError::Internal("x".into())), || true, PidAppNamer));
    let a = FontRequestSource::start(mk(), None).expect("lần 1");
    drop(a);
    let b = FontRequestSource::start(mk(), None).expect("lần 2");
    drop(b);
}
