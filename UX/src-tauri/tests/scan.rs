use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app_lib::dto::events::ScanEvent;
use app_lib::dto::library::{RescanStatusDto, ScanPhaseDto};
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn booted(dir: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&dir.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    st
}

fn fonts_dir(root: &Path, n: usize) -> PathBuf {
    let d = root.join("Fonts");
    std::fs::create_dir_all(&d).unwrap();
    let raw = std::fs::read(fixture("roboto.ttf")).unwrap();
    for i in 0..n {
        let mut b = raw.clone();
        b.extend_from_slice(&(i as u32).to_le_bytes());
        std::fs::write(d.join(format!("r{i:04}.ttf")), b).unwrap();
    }
    d
}

type Log = Arc<Mutex<Vec<ScanEvent>>>;

fn record(st: &AppState) -> Log {
    let log: Log = Arc::default();
    let l2 = log.clone();
    st.hubs.scan.subscribe(Box::new(move |e| l2.lock().unwrap().push(e)));
    log
}

fn finished(log: &Log) -> Vec<ScanEvent> {
    log.lock().unwrap().iter().filter(|e| matches!(e, ScanEvent::Finished { .. })).cloned().collect()
}

#[test]
fn add_and_scan_reports_progress_then_finished() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let log = record(&st);
    let dir = fonts_dir(d.path(), 5);
    let s = sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert_eq!(s.kind, "directory");
    assert!(st.scans.wait_idle(Duration::from_secs(30)));
    let ev = log.lock().unwrap().clone();
    assert!(matches!(ev.first(), Some(ScanEvent::Queued { .. })));
    assert!(ev.iter().any(|e| matches!(e, ScanEvent::Progress { progress, .. } if progress.phase == ScanPhaseDto::Analyzing)));
    assert!(ev.iter().any(|e| matches!(e, ScanEvent::FacesAdded { count, .. } if *count == 5)));
    assert!(matches!(ev.last(), Some(ScanEvent::Idle)));
    let ScanEvent::Finished { stats, .. } = &finished(&log)[0] else { unreachable!() };
    assert_eq!((stats.status, stats.analyzed), (RescanStatusDto::Ok, 5));
    let list = sources::list_sources(&st).unwrap();
    assert_eq!((list[0].face_count, list[0].scanning, list[0].available), (5, false, true));
    assert!(list[0].last_scan.is_some());
}

#[test]
fn nested_source_error_has_existing_id_and_path() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let dir = fonts_dir(d.path(), 1);
    let s = sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    st.scans.wait_idle(Duration::from_secs(30));
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let e = sources::add_source(&st, sub.to_str().unwrap()).unwrap_err();
    assert_eq!(e.code, "nested_source");
    let det = e.detail.unwrap();
    assert_eq!(det["existingId"], s.id);
    assert_eq!(det["existingPath"], s.path.unwrap());
    assert_eq!(sources::add_source(&st, "/khong/co/thu/muc").unwrap_err().code, "invalid_arg");
}

#[test]
fn duplicate_rescan_is_single_flight() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let dir = fonts_dir(d.path(), 3);
    let log = record(&st);
    st.scans.pause(&st); // giữ lần quét đầu ở callback đầu tiên
    let s = sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    sources::rescan_sources(&st, Some(vec![s.id])).unwrap();
    sources::rescan_sources(&st, None).unwrap();
    let status = st.scans.status();
    assert!(status.running && status.queued.is_empty(), "{status:?}");
    st.scans.resume(&st);
    assert!(st.scans.wait_idle(Duration::from_secs(30)));
    assert_eq!(finished(&log).len(), 1);
}

#[test]
fn two_subscribers_get_same_events() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let (a, b) = (record(&st), record(&st));
    let dir = fonts_dir(d.path(), 3);
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(30)));
    assert_eq!(*a.lock().unwrap(), *b.lock().unwrap());
    assert!(a.lock().unwrap().len() > 3);
}

#[test]
fn unavailable_source_reported() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let dir = fonts_dir(d.path(), 2);
    let s = sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(30)));
    let log = record(&st);
    std::fs::rename(&dir, d.path().join("Fonts-rut")).unwrap();
    sources::rescan_sources(&st, Some(vec![s.id])).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(30)));
    let ScanEvent::Finished { stats, .. } = &finished(&log)[0] else { unreachable!() };
    assert_eq!(stats.status, RescanStatusDto::SourceUnavailable);
    let list = sources::list_sources(&st).unwrap();
    assert!(!list[0].available);
    assert_eq!(list[0].face_count, 2, "không đánh missing khi ổ rút");
}

#[test]
fn favorite_during_scan_latency_under_200ms() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let dir = fonts_dir(d.path(), 600);
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    let key = fontatlas_store::FaceKey { hash: [3; 32], face_index: 0 };
    let mut worst = Duration::ZERO;
    let mut n = 0;
    let t0 = Instant::now();
    while st.scans.status().running && t0.elapsed() < Duration::from_secs(60) {
        let t = Instant::now();
        st.with_user(|u| Ok(u.set_favorite(&key, n % 2 == 0)?)).expect("không lỗi busy");
        worst = worst.max(t.elapsed());
        n += 1;
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(n > 5, "quét quá nhanh để đo: {n}");
    assert!(worst < Duration::from_millis(200), "trễ tệ nhất {worst:?} / {n} lần");
}

#[test]
fn pause_then_resume_completes() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let dir = fonts_dir(d.path(), 20);
    let log = record(&st);
    st.scans.pause(&st);
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let progress_while_paused = log.lock().unwrap().iter().filter(|e| matches!(e, ScanEvent::Progress { .. })).count();
    assert_eq!(progress_while_paused, 0);
    st.scans.resume(&st);
    assert!(st.scans.wait_idle(Duration::from_secs(30)));
    assert_eq!(finished(&log).len(), 1);
    let ev = log.lock().unwrap();
    assert!(ev.iter().any(|e| matches!(e, ScanEvent::Paused)) && ev.iter().any(|e| matches!(e, ScanEvent::Resumed)));
}
