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
