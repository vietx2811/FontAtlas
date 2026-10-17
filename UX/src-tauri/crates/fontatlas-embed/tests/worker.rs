mod common;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Duration;

use common::*;
use fontatlas_embed::{EmbedError, EmbedWorker, Encode, OrtEncoder, Progress, Whitening, WorkerConfig};
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
    Box::new(OrtEncoder::load(&data("tiny_encoder.onnx"), 1).unwrap())
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
