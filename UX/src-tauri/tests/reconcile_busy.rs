//! Đối soát gặp font BẬN (thao tác khác đang giữ hash): không bỏ qua im lặng — xếp lượt BÙ sau ~1,5 s, tối đa 2 lượt.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;

use app_lib::dto::activation::*;
use app_lib::dto::events::ActivationEvent;
use app_lib::dto::library::*;
use app_lib::services::{activation, boot, library, sources};
use app_lib::state::{AppDirs, AppState};
use fontatlas_activate::{ActivationEngine, ActivationService, EngineError, FakeEngine, Scope};

struct Shared(Arc<FakeEngine>);
impl ActivationEngine for Shared {
    fn method(&self) -> &'static str { self.0.method() }
    fn register(&self, p: &Path, s: Scope) -> Result<(), EngineError> { self.0.register(p, s) }
    fn unregister(&self, p: &Path, s: Scope) -> Result<(), EngineError> { self.0.unregister(p, s) }
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError> { self.0.available_postscript_names() }
    fn providers(&self, ps: &str) -> Result<Vec<PathBuf>, EngineError> { self.0.providers(ps) }
    fn postscript_names_in_file(&self, p: &Path) -> Result<Vec<String>, EngineError> { self.0.postscript_names_in_file(p) }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

/// Dựng trạng thái: một font đang được kích hoạt dở (bị chặn ở engine) nên đối soát thấy nó BẬN.
struct Fx {
    _d: tempfile::TempDir,
    st: Arc<AppState>,
    eng: Arc<FakeEngine>,
    key: FaceKeyDto,
    passes: Arc<AtomicUsize>,
    go: Sender<()>,
    waits: Arc<AtomicUsize>,
}

fn fx() -> Fx {
    let d = tempfile::tempdir().unwrap();
    let eng = Arc::new(FakeEngine::new());
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(ActivationService::new(Shared(eng.clone()))));
    boot::run_boot(&st);
    let dir = d.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("roboto.ttf"), dir.join("roboto.ttf")).unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let r = library::query_faces(&st, &FaceQueryDto { limit: 5, ..Default::default() }).unwrap().items.remove(0);
    let key = FaceKeyDto { hash_hex: r.hash_hex, face_index: r.face_index };
    // đếm lượt đối soát: mỗi lượt có một dòng ⇒ một Progress
    let passes = Arc::new(AtomicUsize::new(0));
    let p2 = passes.clone();
    st.hubs.activation.subscribe(Box::new(move |e| {
        if matches!(e, ActivationEvent::Progress { .. }) {
            p2.fetch_add(1, Ordering::SeqCst);
        }
    }));
    // "chờ" trước mỗi lượt bù = đợi tín hiệu của test (không ngủ theo đồng hồ)
    let (go, rx) = channel::<()>();
    let rx = Mutex::new(rx);
    let waits = Arc::new(AtomicUsize::new(0));
    let w2 = waits.clone();
    st.reconcile_makeup.set_waiter(move || {
        w2.fetch_add(1, Ordering::SeqCst);
        let _ = rx.lock().unwrap().recv_timeout(Duration::from_secs(60));
    });
    Fx { _d: d, st, eng, key, passes, go, waits }
}

fn wait_until(what: &str, mut f: impl FnMut() -> bool) {
    let t0 = std::time::Instant::now();
    while !f() {
        assert!(t0.elapsed() < Duration::from_secs(60), "hết hạn chờ: {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Bắt đầu kích hoạt dở (giữ hash bận tới khi `release`). Trả handle + barrier.
fn hold_busy(f: &Fx) -> (std::thread::JoinHandle<()>, Arc<Barrier>) {
    let barrier = Arc::new(Barrier::new(2));
    f.eng.block_register_until(barrier.clone());
    let (st2, k2) = (f.st.clone(), f.key.clone());
    let t = std::thread::spawn(move || {
        activation::activate(&st2, std::slice::from_ref(&k2), Some(ScopeDto::Session), false, &mut |_| {}).unwrap();
    });
    wait_until("engine bị chặn", || f.eng.is_blocked());
    (t, barrier)
}

#[test]
fn a_busy_font_gets_exactly_one_makeup_pass_once_it_is_free() {
    let f = fx();
    let (t, barrier) = hold_busy(&f);
    let base = f.passes.load(Ordering::SeqCst);
    activation::reconcile_now(&f.st, false);
    assert_eq!(f.passes.load(Ordering::SeqCst) - base, 1, "lượt đầu thấy hash bận");
    wait_until("lượt bù được xếp", || f.waits.load(Ordering::SeqCst) == 1);
    assert_eq!(f.st.reconcile_makeup.pending(), 1);
    barrier.wait(); // hết bận TRƯỚC khi cho lượt bù chạy
    t.join().unwrap();
    f.go.send(()).unwrap();
    wait_until("lượt bù chạy xong", || f.st.reconcile_makeup.pending() == 0);
    assert_eq!(f.passes.load(Ordering::SeqCst) - base, 2, "đúng một lượt bù");
    assert_eq!(f.waits.load(Ordering::SeqCst), 1, "hash đã rảnh ⇒ không xếp thêm lượt nữa");
}

#[test]
fn a_font_that_stays_busy_gets_at_most_two_makeup_passes() {
    let f = fx();
    let (t, barrier) = hold_busy(&f);
    let base = f.passes.load(Ordering::SeqCst);
    activation::reconcile_now(&f.st, false);
    for n in 1..=2 {
        wait_until("lượt bù được xếp", || f.waits.load(Ordering::SeqCst) == n);
        f.go.send(()).unwrap(); // hash VẪN bận khi lượt bù chạy
        wait_until("lượt bù chạy xong", || f.passes.load(Ordering::SeqCst) - base == 1 + n);
    }
    wait_until("không còn lượt chờ", || f.st.reconcile_makeup.pending() == 0);
    assert_eq!(f.waits.load(Ordering::SeqCst), 2, "dừng sau 2 lượt bù, không lặp vô hạn");
    assert_eq!(f.passes.load(Ordering::SeqCst) - base, 3);
    barrier.wait();
    t.join().unwrap();
}

#[test]
fn no_makeup_pass_runs_after_the_app_started_shutting_down() {
    let f = fx();
    let (t, barrier) = hold_busy(&f);
    let base = f.passes.load(Ordering::SeqCst);
    activation::reconcile_now(&f.st, false);
    wait_until("lượt bù được xếp", || f.waits.load(Ordering::SeqCst) == 1);
    f.st.scans.request_shutdown(); // tắt app trong lúc lượt bù đang chờ
    barrier.wait();
    t.join().unwrap();
    f.go.send(()).unwrap();
    wait_until("lượt chờ kết thúc", || f.st.reconcile_makeup.pending() == 0);
    assert_eq!(f.passes.load(Ordering::SeqCst) - base, 1, "lượt bù KHÔNG chạy sau khi tắt app");
}
