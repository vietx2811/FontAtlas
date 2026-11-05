mod common;
use common::*;
use fontatlas_store::{RescanStatus, ScanPhase, ScanProgress, SourceKind, Store};
use std::sync::atomic::{AtomicBool, Ordering};

fn many(n: usize) -> (tempfile::TempDir, std::path::PathBuf, Store, i64) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    let raw = std::fs::read(core_fixture("roboto.ttf")).unwrap();
    for i in 0..n {
        let mut b = raw.clone();
        b.extend_from_slice(&(i as u32).to_le_bytes()); // thêm 4 byte cuối để hash khác nhau
        std::fs::write(fonts.join(format!("r{i:04}.ttf")), b).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    (d, fonts, s, src)
}

fn missing(s: &Store) -> i64 {
    count(s, "SELECT count(*) FROM file_locations WHERE missing=1")
}

#[test]
fn progress_phases_in_order() {
    let (_d, _f, mut s, src) = many(600);
    let mut ev: Vec<ScanProgress> = Vec::new();
    let st = s.rescan_source_with(src, &mut |p| ev.push(p.clone()), &AtomicBool::new(false)).unwrap();
    assert_eq!(st.analyzed, 600);
    let phases: Vec<ScanPhase> = ev.iter().map(|e| e.phase).collect();
    use ScanPhase::*;
    assert_eq!(phases, vec![Walking, Hashing, Analyzing, Analyzing, Analyzing, MarkingMissing]);
    let last = ev.iter().rfind(|e| e.phase == Analyzing).unwrap();
    assert_eq!((last.done, last.total), (600, 600));
    assert_eq!(ev.iter().map(|e| e.faces_written).sum::<usize>(), 600);
}

#[test]
fn cancelled_scan_marks_nothing_missing() {
    let (_d, fonts, mut s, src) = many(300);
    s.rescan_source(src).unwrap();
    for i in 0..10 {
        std::fs::remove_file(fonts.join(format!("r{i:04}.ttf"))).unwrap();
    }
    for i in 300..700 {
        let mut b = std::fs::read(core_fixture("roboto.ttf")).unwrap();
        b.extend_from_slice(&(i as u32 + 9000).to_le_bytes());
        std::fs::write(fonts.join(format!("n{i}.ttf")), b).unwrap();
    }
    let cancel = AtomicBool::new(false);
    let st = s
        .rescan_source_with(src, &mut |p| {
            if p.phase == ScanPhase::Analyzing {
                cancel.store(true, Ordering::SeqCst);
            }
        }, &cancel)
        .unwrap();
    assert_eq!(st.status, RescanStatus::Cancelled);
    assert_eq!(missing(&s), 0);
}

#[test]
fn unplugged_mid_scan_marks_nothing_missing() {
    let (d, fonts, mut s, src) = many(600);
    let moved = d.path().join("Fonts-gone");
    let mut done = false;
    let st = s
        .rescan_source_with(src, &mut |p| {
            if p.phase == ScanPhase::Analyzing && !done {
                done = true;
                std::fs::rename(&fonts, &moved).unwrap();
            }
        }, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(st.status, RescanStatus::SourceUnavailable);
    assert_eq!(missing(&s), 0);
}

#[test]
fn io_error_stays_pending_not_failed() {
    let (d, fonts, mut s, src) = many(600);
    let moved = d.path().join("Fonts-gone");
    let mut done = false;
    let st = s
        .rescan_source_with(src, &mut |p| {
            if p.phase == ScanPhase::Analyzing && !done {
                done = true;
                std::fs::rename(&fonts, &moved).unwrap();
            }
        }, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(st.status, RescanStatus::SourceUnavailable);
    assert_eq!(count(&s, "SELECT count(*) FROM pipeline_state WHERE status='failed'"), 0);
    // trả thư mục: quét lại phân tích nốt
    std::fs::rename(&moved, &fonts).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.failed, 0);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE missing=0"), 600);
}

#[test]
fn progress_callback_runs_outside_transaction() {
    let (d, _f, mut s, src) = many(300);
    let lib = d.path().join("db").join(fontatlas_store::LIBRARY_FILE);
    let mut ok = 0;
    s.rescan_source_with(src, &mut |p| {
        if p.phase == ScanPhase::Analyzing {
            let c = rusqlite::Connection::open(&lib).unwrap();
            c.busy_timeout(std::time::Duration::from_millis(0)).unwrap();
            c.execute_batch("BEGIN IMMEDIATE; COMMIT;").expect("callback phải chạy ngoài transaction");
            ok += 1;
        }
    }, &AtomicBool::new(false))
    .unwrap();
    assert!(ok >= 2);
}
