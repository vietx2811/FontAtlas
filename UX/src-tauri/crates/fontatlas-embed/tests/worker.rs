mod common;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Duration;

use common::*;
use fontatlas_embed::{EmbedError, EmbedWorker, Encode, Progress, Whitening, WorkerConfig};
use fontatlas_store::SETTING_EMBED_ENABLED;

const RAW: &str = "raw-test|render-v2";

fn cfg(d: &tempfile::TempDir) -> WorkerConfig {
    WorkerConfig {
        store_dir: d.path().join("data"),
        raw_version: RAW.into(),
        whitening: Some(Whitening::from_bytes(&identity_whitening_bytes(768, 128)).unwrap()),
        render_threads: 2,
    }
}

fn tiny() -> Box<dyn Encode> {
    tiny_768()
}

/// Encoder chậm: báo vào barrier khi bắt đầu lô đầu tiên, rồi chờ `delay` (kiểm huỷ mỗi 10 ms).
struct SlowEncoder {
    inner: Box<dyn Encode>,
    in_run: Arc<Barrier>,
    first: AtomicBool,
    delay: Duration,
}

impl Encode for SlowEncoder {
    fn raw_dim(&self) -> usize {
        768
    }
    fn encode(&mut self, t: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        if !self.first.swap(true, Ordering::SeqCst) {
            self.in_run.wait();
        }
        let start = std::time::Instant::now();
        while start.elapsed() < self.delay {
            if cancel.load(Ordering::SeqCst) {
                return Err(EmbedError::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        self.inner.encode(t, cancel)
    }
}

#[test]
fn processes_all_fonts_and_finishes() {
    let d = store_dir_with(&[("roboto.ttf", 0), ("playfair_display.ttf", 0), ("source_code_pro.ttf", 0), ("open_sans.ttf", 0)]);
    let w = EmbedWorker::start(cfg(&d), tiny(), Box::new(|_| {})).unwrap();
    let mv = w.model_version().unwrap().to_string();
    w.join().unwrap();
    let s = open_data(&d);
    assert_eq!((s.count_embeddings(RAW).unwrap(), s.count_embeddings(&mv).unwrap()), (4, 4));
}

#[test]
fn worker_refuses_when_disabled() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    open_data(&d).set_setting(SETTING_EMBED_ENABLED, "0").unwrap();
    assert!(matches!(EmbedWorker::start(cfg(&d), tiny(), Box::new(|_| {})), Err(EmbedError::Disabled)));
}

#[test]
fn cancel_mid_run_writes_nothing() {
    let d = store_dir_with(&(0..8).map(|i| ("roboto.ttf", i)).collect::<Vec<_>>());
    let in_run = Arc::new(Barrier::new(2));
    let enc = Box::new(SlowEncoder { inner: tiny(), in_run: in_run.clone(), first: AtomicBool::new(false), delay: Duration::from_secs(5) });
    let w = EmbedWorker::start(cfg(&d), enc, Box::new(|_| {})).unwrap();
    in_run.wait(); // chắc chắn đang ở giữa encode
    w.cancel();
    w.join().unwrap();
    let s = open_data(&d);
    assert_eq!(s.count_embeddings(RAW).unwrap(), 0);
    assert_eq!(count_status(&s, "running"), 0);
    assert_eq!(count_status(&s, "pending"), 8);
}

#[test]
fn resume_after_restart_completes() {
    let d = store_dir_with(&(0..10).map(|i| ("roboto.ttf", i)).collect::<Vec<_>>());
    {
        let mut s = open_data(&d);
        s.enqueue_embeddings(RAW).unwrap();
        s.claim_embedding_batch(RAW, 4).unwrap(); // "app chết" khi 4 face đang running
    }
    EmbedWorker::start(cfg(&d), tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    assert_eq!(open_data(&d).count_embeddings(RAW).unwrap(), 10);
}

#[test]
fn pause_then_resume_deterministic() {
    let d = store_dir_with(&(0..12).map(|i| ("roboto.ttf", i)).collect::<Vec<_>>());
    let (tx, rx) = std::sync::mpsc::channel::<Progress>();
    let gate = Arc::new(Barrier::new(2));
    let g = gate.clone();
    let w = EmbedWorker::start(
        cfg(&d),
        tiny(),
        Box::new(move |p| {
            let started = matches!(p, Progress::Started { .. });
            tx.send(p).ok();
            if started {
                g.wait();
            }
        }),
    )
    .unwrap();
    w.pause();
    gate.wait(); // pause đã bật trước khi lô đầu được claim
    assert!(matches!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), Progress::Started { .. }));
    assert!(matches!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), Progress::Paused));
    w.resume();
    w.join().unwrap();
    let rest: Vec<Progress> = rx.try_iter().collect();
    assert!(matches!(rest.first(), Some(Progress::Resumed)), "{rest:?}");
    assert!(matches!(rest.last(), Some(Progress::Finished { total_done: 12 })), "{rest:?}");
}

#[test]
fn stale_file_failed_others_succeed() {
    let d = store_dir_with(&[("roboto.ttf", 0), ("playfair_display.ttf", 0)]);
    let p = d.path().join("fonts").join("0_roboto.ttf");
    let mut b = std::fs::read(&p).unwrap();
    b.push(0);
    std::fs::write(&p, b).unwrap(); // file đổi sau khi quét → hash khác
    EmbedWorker::start(cfg(&d), tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    let s = open_data(&d);
    assert_eq!(state_of(&s, key_of(&s, "Roboto"), RAW).1.as_deref(), Some("stale_file"));
    assert_eq!(s.count_embeddings(RAW).unwrap(), 1);
}

#[test]
fn no_glyphs_is_skipped() {
    // Roboto chỉ còn U+0021–002F (xem data/roboto_punct_only.SOURCE.txt) → chuỗi mẫu thiếu chữ.
    let d = store_dir_from(vec![("punct.ttf".into(), std::fs::read(data("roboto_punct_only.ttf")).unwrap())]);
    EmbedWorker::start(cfg(&d), tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    let s = open_data(&d);
    assert_eq!(count_status(&s, "skipped"), 1);
    assert_eq!(s.count_embeddings(RAW).unwrap(), 0);
}

#[test]
fn picks_up_faces_added_during_run() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    let dir = d.path().to_owned();
    let added = AtomicBool::new(false);
    let w = EmbedWorker::start(
        cfg(&d),
        tiny(),
        Box::new(move |p| {
            if matches!(p, Progress::Batch { .. }) && !added.swap(true, Ordering::SeqCst) {
                add_font_and_rescan(&dir, "playfair_display.ttf");
            }
        }),
    )
    .unwrap();
    w.join().unwrap();
    assert_eq!(open_data(&d).count_embeddings(RAW).unwrap(), 2);
}

// ---- fix pass sau review cuối ----

/// Encoder không được gọi tới (panic nếu gọi).
struct PanicEncoder;
impl Encode for PanicEncoder {
    fn raw_dim(&self) -> usize {
        768
    }
    fn encode(&mut self, _: &[Vec<f32>], _: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        panic!("không được chạy model");
    }
}

/// Encoder lỗi hệ thống (không phải lỗi của từng face).
struct FailingEncoder;
impl Encode for FailingEncoder {
    fn raw_dim(&self) -> usize {
        768
    }
    fn encode(&mut self, _: &[Vec<f32>], _: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        Err(EmbedError::Model("ort: cấp phát thất bại".into()))
    }
}

/// Whitening "khác": đơn vị nhân 2 (sau L2 vẫn như identity, nhưng sha/version khác).
fn other_whitening() -> Whitening {
    let mut b = identity_whitening_bytes(768, 128);
    let n = b.len();
    b[n - 4..].copy_from_slice(&0.5f32.to_le_bytes()); // đổi một phần tử của W → sha khác
    Whitening::from_bytes(&b).unwrap()
}

#[test]
fn whitening_only_change_rewhitens_without_model() {
    let d = store_dir_with(&[("roboto.ttf", 0), ("playfair_display.ttf", 0)]);
    EmbedWorker::start(cfg(&d), tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    let mut c2 = cfg(&d);
    c2.whitening = Some(other_whitening());
    let w = EmbedWorker::start(c2, Box::new(PanicEncoder), Box::new(|_| {})).unwrap();
    let mv2 = w.model_version().unwrap().to_string();
    w.join().unwrap();
    let s = open_data(&d);
    assert_eq!(s.count_embeddings(&mv2).unwrap(), 2);
    assert_eq!(s.count_embeddings(RAW).unwrap(), 2);
}

#[test]
fn failed_missing_file_retried_on_next_start() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    {
        let mut s = open_data(&d);
        s.enqueue_embeddings(RAW).unwrap();
        s.conn().execute("UPDATE file_locations SET missing=1", []).unwrap(); // ổ ngoài rút ra
        assert!(s.claim_embedding_batch(RAW, 4).unwrap().is_empty());
        s.conn().execute("UPDATE file_locations SET missing=0", []).unwrap(); // cắm lại
    }
    EmbedWorker::start(cfg(&d), tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    assert_eq!(open_data(&d).count_embeddings(RAW).unwrap(), 1);
}

#[test]
fn encoder_error_stops_run_and_releases_batch() {
    let d = store_dir_with(&(0..6).map(|i| ("roboto.ttf", i)).collect::<Vec<_>>());
    let r = EmbedWorker::start(cfg(&d), Box::new(FailingEncoder), Box::new(|_| {})).unwrap().join();
    assert!(r.is_err());
    let s = open_data(&d);
    assert_eq!((count_status(&s, "failed"), count_status(&s, "running"), count_status(&s, "pending")), (0, 0, 6));
}

#[test]
fn whitening_with_wrong_dims_rejected_at_start() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    let mut c = cfg(&d);
    c.whitening = Some(Whitening::from_bytes(&identity_whitening_bytes(512, 128)).unwrap());
    assert!(matches!(EmbedWorker::start(c, tiny(), Box::new(|_| {})), Err(EmbedError::Model(_))));
}

/// Encoder giữ khoá ghi của library.db lâu hơn busy_timeout (5 s) → `finish` lỗi.
struct LockingEncoder {
    inner: Box<dyn Encode>,
    db: std::path::PathBuf,
}
impl Encode for LockingEncoder {
    fn raw_dim(&self) -> usize {
        768
    }
    fn encode(&mut self, t: &[Vec<f32>], c: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        let db = self.db.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let conn = rusqlite::Connection::open(db).unwrap();
            conn.execute_batch("BEGIN EXCLUSIVE").unwrap();
            tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(6500));
            conn.execute_batch("COMMIT").unwrap();
        });
        rx.recv().unwrap();
        self.inner.encode(t, c)
    }
}

#[test]
fn store_error_after_claim_releases_batch() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    let db = d.path().join("data").join(fontatlas_store::LIBRARY_FILE);
    let r = EmbedWorker::start(cfg(&d), Box::new(LockingEncoder { inner: tiny(), db }), Box::new(|_| {})).unwrap().join();
    assert!(r.is_err());
    let s = open_data(&d);
    assert_eq!((count_status(&s, "running"), count_status(&s, "pending")), (0, 1));
}
