use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fontatlas_broker::{EnsureError, EnsureRequest, EnsureResponse, FontResult, FontStatus};
use fontatlas_fontreq::{Answer, Request, Responder, app_label_from_exe_path, is_internal_name};

fn ps(n: &str) -> Request {
    Request { ps: Some(n.into()), ..Default::default() }
}

fn resp(status: FontStatus, path: Option<&str>) -> EnsureResponse {
    EnsureResponse {
        ok: true,
        v: 1,
        app: "x".into(),
        results: vec![FontResult { name: "n".into(), status, path: path.map(Into::into) }],
        found: vec![],
        missing: vec![],
    }
}

struct Rig {
    r: Responder,
    calls: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<EnsureRequest>>>,
    on: Arc<std::sync::atomic::AtomicBool>,
}

fn rig(status: FontStatus, path: Option<&'static str>) -> Rig {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(Mutex::new(vec![]));
    let on = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let (c, s, o) = (calls.clone(), seen.clone(), on.clone());
    let r = Responder::new(
        move |q: &EnsureRequest| -> Result<EnsureResponse, EnsureError> {
            c.fetch_add(1, Ordering::SeqCst);
            s.lock().unwrap().push(q.clone());
            Ok(resp(status, path))
        },
        move || o.load(Ordering::SeqCst),
        |pid: i32| (pid == 4242).then(|| "Adobe Photoshop 2026".to_string()),
    );
    Rig { r, calls, seen, on }
}

#[test]
fn internal_names_are_filtered_without_asking_the_library() {
    for n in ["AdobeSansMM", "AdobeSerifMM", "adobesansmm-extra", ".SFNS-Regular", ".AppleSystemUIFont", "LastResort", "", "  ", "a\u{0}b"] {
        assert!(is_internal_name(n), "{n:?}");
    }
    for n in ["LibreBaskerville-Regular", "AdobeGaramondPro-Bold", "Adobe Clean", "Helvetica"] {
        assert!(!is_internal_name(n), "{n:?}");
    }
    let rg = rig(FontStatus::Activated, Some("/f.otf"));
    assert_eq!(rg.r.answer(4242, &ps("AdobeSansMM")), Answer::Nothing);
    assert_eq!(rg.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn found_font_returns_the_file_and_names_the_app_from_the_pid() {
    let rg = rig(FontStatus::Activated, Some("/lib/LibreBaskerville.ttf"));
    let a = rg.r.answer(4242, &ps("LibreBaskerville-Regular"));
    assert_eq!(a, Answer::Files(vec!["/lib/LibreBaskerville.ttf".into()]));
    let seen = rg.seen.lock().unwrap();
    assert_eq!(seen[0].app, "Adobe Photoshop 2026");
    assert_eq!(seen[0].fonts[0].ps.as_deref(), Some("LibreBaskerville-Regular"));
    assert_eq!(seen[0].doc, None);
}

#[test]
fn unknown_pid_becomes_other() {
    let rg = rig(FontStatus::Activated, Some("/f.ttf"));
    rg.r.answer(7, &ps("X-Regular"));
    assert_eq!(rg.seen.lock().unwrap()[0].app, "other");
}

#[test]
fn missing_is_remembered_briefly_then_asked_again() {
    let rg = rig(FontStatus::Missing, None);
    for _ in 0..5 {
        assert_eq!(rg.r.answer(4242, &ps("Nope-Regular")), Answer::Nothing);
    }
    assert_eq!(rg.calls.load(Ordering::SeqCst), 1, "Photoshop hỏi lặp lại: chỉ lần đầu chạm thư viện");
    // tên khác vẫn được hỏi
    rg.r.answer(4242, &ps("Other-Regular"));
    assert_eq!(rg.calls.load(Ordering::SeqCst), 2);

    let rg = rig(FontStatus::Missing, None);
    let r = Responder::new(
        {
            let c = rg.calls.clone();
            move |_q: &EnsureRequest| {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(resp(FontStatus::Missing, None))
            }
        },
        || true,
        |_: i32| None,
    )
    .with_negative_ttl(Duration::ZERO);
    r.answer(1, &ps("A-Regular"));
    r.answer(1, &ps("A-Regular"));
    assert_eq!(rg.calls.load(Ordering::SeqCst), 2, "hết hạn nhớ → hỏi lại (font có thể mới được thêm vào thư viện)");
}

#[test]
fn temporary_failures_are_not_remembered() {
    for st in [FontStatus::Busy, FontStatus::NotDownloaded, FontStatus::Failed, FontStatus::Unavailable, FontStatus::Conflict] {
        let rg = rig(st, None);
        assert_eq!(rg.r.answer(4242, &ps("T-Regular")), Answer::Nothing, "{st:?}");
        assert_eq!(rg.r.answer(4242, &ps("T-Regular")), Answer::Nothing);
        assert_eq!(rg.calls.load(Ordering::SeqCst), 2, "{st:?} phải được hỏi lại");
    }
}

#[test]
fn switch_off_answers_nothing_and_takes_effect_immediately() {
    let rg = rig(FontStatus::Activated, Some("/f.ttf"));
    assert!(matches!(rg.r.answer(4242, &ps("A-Regular")), Answer::Files(_)));
    rg.on.store(false, Ordering::SeqCst);
    assert_eq!(rg.r.answer(4242, &ps("A-Regular")), Answer::Nothing);
    rg.on.store(true, Ordering::SeqCst);
    assert!(matches!(rg.r.answer(4242, &ps("A-Regular")), Answer::Files(_)));
}

#[test]
fn ensure_error_and_empty_request_answer_nothing() {
    let r = Responder::new(|_q: &EnsureRequest| Err(EnsureError::Internal("x".into())), || true, |_: i32| None);
    assert_eq!(r.answer(1, &ps("A-Regular")), Answer::Nothing);
    assert_eq!(r.answer(1, &Request::default()), Answer::Nothing);
}

#[test]
fn own_process_is_never_served() {
    let rg = rig(FontStatus::Activated, Some("/f.ttf"));
    assert_eq!(rg.r.answer(std::process::id() as i32, &ps("A-Regular")), Answer::Nothing);
    assert_eq!(rg.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn family_style_request_is_forwarded() {
    let rg = rig(FontStatus::Activated, Some("/f.ttf"));
    let q = Request { ps: None, family: Some("Libre Baskerville".into()), style: Some("Bold".into()) };
    assert!(matches!(rg.r.answer(4242, &q), Answer::Files(_)));
    let seen = rg.seen.lock().unwrap();
    assert_eq!(seen[0].fonts[0].family.as_deref(), Some("Libre Baskerville"));
    assert_eq!(seen[0].fonts[0].style.as_deref(), Some("Bold"));
}

#[test]
fn app_label_comes_from_the_outermost_app_bundle() {
    assert_eq!(
        app_label_from_exe_path("/Applications/Adobe Photoshop 2026/Adobe Photoshop 2026.app/Contents/MacOS/Adobe Photoshop 2026").as_deref(),
        Some("Adobe Photoshop 2026")
    );
    assert_eq!(
        app_label_from_exe_path("/Applications/Foo.app/Contents/Helpers/Bar.app/Contents/MacOS/Bar").as_deref(),
        Some("Foo")
    );
    assert_eq!(app_label_from_exe_path("/usr/bin/python3"), None);
}
