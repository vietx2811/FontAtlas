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
    let fam = Request { ps: None, family: Some("AdobeSansMM".into()), style: Some("Regular".into()) };
    assert_eq!(rg.r.answer(4242, &fam), Answer::Nothing, "family nội bộ cũng bị lọc");
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
fn busy_and_not_downloaded_are_remembered_only_briefly_other_failures_not_at_all() {
    for st in [FontStatus::Busy, FontStatus::NotDownloaded] {
        let rg = rig(st, None);
        assert_eq!(rg.r.answer(4242, &ps("T-Regular")), Answer::Nothing);
        assert_eq!(rg.r.answer(4242, &ps("T-Regular")), Answer::Nothing);
        assert_eq!(rg.calls.load(Ordering::SeqCst), 1, "{st:?}: trong SLOW_TTL không dồn request vào thư viện");
    }
    for st in [FontStatus::Failed, FontStatus::Unavailable, FontStatus::Conflict] {
        let rg = rig(st, None);
        assert_eq!(rg.r.answer(4242, &ps("T-Regular")), Answer::Nothing, "{st:?}");
        assert_eq!(rg.r.answer(4242, &ps("T-Regular")), Answer::Nothing);
        assert_eq!(rg.calls.load(Ordering::SeqCst), 2, "{st:?} phải được hỏi lại");
    }
    // hết SLOW_TTL thì hỏi lại
    let c = Arc::new(AtomicUsize::new(0));
    let c2 = c.clone();
    let r = Responder::new(
        move |_q: &EnsureRequest| {
            c2.fetch_add(1, Ordering::SeqCst);
            Ok(resp(FontStatus::NotDownloaded, None))
        },
        || true,
        |_: i32| None,
    )
    .with_slow_ttl(Duration::ZERO);
    r.answer(1, &ps("X-Regular"));
    r.answer(1, &ps("X-Regular"));
    assert_eq!(c.load(Ordering::SeqCst), 2);
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

#[test]
fn slow_backend_returns_at_the_deadline_then_serves_the_late_result_next_time() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("late.ttf");
    std::fs::write(&f, b"x").unwrap();
    let fp = f.to_string_lossy().into_owned();
    let calls = Arc::new(AtomicUsize::new(0));
    let c2 = calls.clone();
    let slow = Responder::new(
        move |_q: &EnsureRequest| {
            c2.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(500));
            Ok(resp(FontStatus::Activated, Some(&fp)))
        },
        || true,
        |_: i32| None,
    )
    .with_deadline(Duration::from_millis(80));
    let t = std::time::Instant::now();
    assert_eq!(slow.answer(1, &ps("Slow-Regular")), Answer::Nothing);
    assert!(t.elapsed() < Duration::from_millis(400), "{:?}", t.elapsed());
    // trong lúc việc nền còn chạy: trả rỗng NGAY (không xếp hàng thêm một lần chờ nữa)
    let t = std::time::Instant::now();
    assert_eq!(slow.answer(1, &ps("Slow-Regular")), Answer::Nothing);
    assert!(t.elapsed() < Duration::from_millis(50), "{:?}", t.elapsed());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // việc nền xong → kết quả tốt đến muộn được dùng lại, không gọi lại thư viện
    let t0 = std::time::Instant::now();
    while slow.inflight() > 0 && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(slow.answer(1, &ps("Slow-Regular")), Answer::Files(vec![f]));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn panicking_backend_answers_nothing_and_releases_its_slot() {
    let boom = Responder::new(|_q: &EnsureRequest| -> Result<EnsureResponse, EnsureError> { panic!("lỗi giả") }, || true, |_: i32| None);
    for i in 0..10 {
        assert_eq!(boom.answer(1, &ps(&format!("Boom{i}-Regular"))), Answer::Nothing);
    }
    let t0 = std::time::Instant::now();
    while boom.inflight() > 0 && t0.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(boom.inflight(), 0, "panic không làm kẹt chỗ việc nền");
}

#[test]
fn flood_of_slow_unknown_names_is_capped_and_does_not_pile_up_threads() {
    let gate = Arc::new(std::sync::Barrier::new(1));
    let _ = gate;
    let started = Arc::new(AtomicUsize::new(0));
    let s2 = started.clone();
    let r = Responder::new(
        move |_q: &EnsureRequest| {
            s2.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(400));
            Ok(resp(FontStatus::Missing, None))
        },
        || true,
        |_: i32| None,
    )
    .with_deadline(Duration::from_millis(20));
    for i in 0..20 {
        assert_eq!(r.answer(1, &ps(&format!("Name{i}-Regular"))), Answer::Nothing);
    }
    assert!(started.load(Ordering::SeqCst) <= fontatlas_fontreq::MAX_INFLIGHT, "{} việc nền", started.load(Ordering::SeqCst));
}

#[test]
fn close_cuts_a_waiting_request_short() {
    let r = Arc::new(
        Responder::new(
            |_q: &EnsureRequest| {
                std::thread::sleep(Duration::from_secs(2));
                Ok(resp(FontStatus::Missing, None))
            },
            || true,
            |_: i32| None,
        )
        .with_deadline(Duration::from_secs(10)),
    );
    let r2 = r.clone();
    let h = std::thread::spawn(move || {
        let t = std::time::Instant::now();
        let a = r2.answer(1, &ps("Wait-Regular"));
        (a, t.elapsed())
    });
    std::thread::sleep(Duration::from_millis(150));
    r.close();
    let (a, took) = h.join().unwrap();
    assert_eq!(a, Answer::Nothing);
    assert!(took < Duration::from_millis(600), "{took:?}");
    assert_eq!(r.answer(1, &ps("After-Regular")), Answer::Nothing, "đã đóng thì mọi request trả rỗng");
}
