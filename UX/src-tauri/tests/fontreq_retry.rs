//! Mở nguồn font request lỗi (NoSource): ghi mã lỗi cho Settings rồi thử lại định kỳ tới khi mở được. Không đụng CoreText.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use app_lib::services::fontreq::start_with;
use app_lib::state::{AppDirs, AppState};
use fontatlas_activate::ActivationService;

fn state(d: &std::path::Path) -> Arc<AppState> {
    AppState::new(
        AppDirs::under(&d.join("data")),
        Box::new(ActivationService::new(fontatlas_activate::WindowsGdiEngine)),
    )
}

fn wait(mut f: impl FnMut() -> bool) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(5) {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn failure_is_recorded_then_retried_until_open() {
    let d = tempfile::tempdir().unwrap();
    let st = state(d.path());
    let tries = Arc::new(AtomicUsize::new(0));
    let t2 = tries.clone();
    start_with(&st, Duration::from_millis(40), move || {
        if t2.fetch_add(1, Ordering::SeqCst) < 2 {
            Err(("refused", "từ chối".into()))
        } else {
            Ok(Box::new(()) as Box<dyn std::any::Any + Send>)
        }
    });
    assert_eq!(st.fontreq_error.lock().unwrap().as_deref(), Some("refused"), "lỗi ghi ngay sau lần thử đầu");
    assert!(st.fontreq.lock().unwrap().is_none());
    assert!(wait(|| st.fontreq.lock().unwrap().is_some()), "thử lại cho tới khi mở được");
    assert_eq!(*st.fontreq_error.lock().unwrap(), None, "mở được thì xoá lỗi");
    assert_eq!(tries.load(Ordering::SeqCst), 3);
}

#[test]
fn stop_ends_the_retry_loop() {
    let d = tempfile::tempdir().unwrap();
    let st = state(d.path());
    let tries = Arc::new(AtomicUsize::new(0));
    let t2 = tries.clone();
    start_with(&st, Duration::from_millis(40), move || {
        t2.fetch_add(1, Ordering::SeqCst);
        Err(("timeout", "chậm".into()))
    });
    app_lib::services::fontreq::stop(&st);
    std::thread::sleep(Duration::from_millis(200));
    let n = tries.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(tries.load(Ordering::SeqCst), n, "sau stop không thử nữa");
}

mod probe {
    use super::*;
    use app_lib::services::fontreq::{ProbeOutcome, note_probe_seen, probe};

    fn open_state(d: &std::path::Path) -> Arc<AppState> {
        let st = state(d);
        start_with(&st, Duration::from_secs(60), || Ok(Box::new(()) as Box<dyn std::any::Any + Send>));
        assert!(st.fontreq.lock().unwrap().is_some());
        st
    }

    #[test]
    fn unknown_when_source_is_not_open_and_nothing_is_spawned() {
        let d = tempfile::tempdir().unwrap();
        let st = state(d.path());
        let spawned = AtomicUsize::new(0);
        let r = probe(&st, Duration::from_millis(50), |_| {
            spawned.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert_eq!(r, ProbeOutcome::Unknown);
        assert_eq!(spawned.load(Ordering::SeqCst), 0);
        assert_eq!(*st.fontreq_error.lock().unwrap(), None);
    }

    #[test]
    fn starved_when_the_request_never_arrives_then_recovers() {
        let d = tempfile::tempdir().unwrap();
        let st = open_state(d.path());
        let names = std::sync::Mutex::new(Vec::<String>::new());
        let r = probe(&st, Duration::from_millis(80), |n| {
            names.lock().unwrap().push(n.to_string());
            Ok(()) // tiến trình con "hỏi" nhưng nguồn của ta không nhận
        });
        assert_eq!(r, ProbeOutcome::Starved);
        assert_eq!(st.fontreq_error.lock().unwrap().as_deref(), Some("starved"));
        assert!(fontatlas_fontreq::is_probe_name(&names.lock().unwrap()[0]), "tên probe có tiền tố riêng");

        // lần sau nguồn nhận được (app kia thoát) → xoá cờ
        let st2 = st.clone();
        let r = probe(&st, Duration::from_secs(2), move |n| {
            let (st3, n) = (st2.clone(), n.to_string());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(30));
                note_probe_seen(&st3, &n);
            });
            Ok(())
        });
        assert_eq!(r, ProbeOutcome::Reached);
        assert_eq!(*st.fontreq_error.lock().unwrap(), None);
    }

    #[test]
    fn spawn_failure_changes_nothing_and_other_errors_are_kept() {
        let d = tempfile::tempdir().unwrap();
        let st = open_state(d.path());
        *st.fontreq_error.lock().unwrap() = Some("timeout".into());
        assert_eq!(probe(&st, Duration::from_millis(50), |_| Err("không chạy được".into())), ProbeOutcome::Unknown);
        assert_eq!(st.fontreq_error.lock().unwrap().as_deref(), Some("timeout"));
        // Reached chỉ xoá "starved", không xoá lỗi khác
        let st2 = st.clone();
        let r = probe(&st, Duration::from_secs(2), move |n| {
            note_probe_seen(&st2, n);
            Ok(())
        });
        assert_eq!(r, ProbeOutcome::Reached);
        assert_eq!(st.fontreq_error.lock().unwrap().as_deref(), Some("timeout"));
    }
}
