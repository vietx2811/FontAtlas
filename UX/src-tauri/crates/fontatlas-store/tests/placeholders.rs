//! File giữ chỗ của OneDrive/iCloud (chưa tải về máy): không đọc nội dung, không đánh missing, giữ dữ liệu cũ.
mod common;
use common::*;
use fontatlas_store::{RescanStatus, ScanProgress, SourceKind, Store};
use std::path::Path;
use std::sync::atomic::AtomicBool;

fn setup(names: &[&str]) -> (tempfile::TempDir, std::path::PathBuf, Store, i64) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in names {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    (d, fonts, s, src)
}

/// Coi file có tên chứa "evicted" là placeholder (thay cờ dataless của hệ điều hành).
fn probe(p: &Path, _m: &std::fs::Metadata) -> bool {
    p.file_name().is_some_and(|n| n.to_string_lossy().contains("evicted"))
}

fn scan(s: &mut Store, root: &Path, src: i64, progress: &mut Vec<ScanProgress>) -> fontatlas_store::RescanStats {
    s.rescan_root_with_probe(src, root, &mut |p| progress.push(p.clone()), &AtomicBool::new(false), &probe).unwrap()
}

#[test]
fn never_downloaded_placeholder_is_not_read_and_counted() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf"]);
    // nội dung không đọc được (nếu bị mở sẽ thành "unreadable"/"failed"): placeholder phải được bỏ qua trước đó
    let ph = fonts.join("evicted.ttf");
    std::fs::write(&ph, b"khong duoc doc").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&ph, std::fs::Permissions::from_mode(0o000)).unwrap();
    let mut prog = Vec::new();
    let st = scan(&mut s, &fonts, src, &mut prog);
    std::fs::set_permissions(&ph, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(st.status, RescanStatus::Ok);
    assert_eq!((st.not_downloaded, st.analyzed, st.failed), (1, 1, 0));
    assert!(st.unreadable.is_empty(), "{:?}", st.unreadable);
    assert_eq!(count(&s, "SELECT count(*) FROM pipeline_state WHERE status='failed'"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE path LIKE '%evicted%'"), 0);
    assert!(prog.iter().all(|p| p.not_downloaded == 1), "{prog:?}");
}

#[test]
fn analyzed_then_evicted_keeps_data_and_is_not_missing() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    std::fs::rename(fonts.join("open_sans.ttf"), fonts.join("evicted_open_sans.ttf")).unwrap();
    let st = s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    assert_eq!((st.analyzed, st.not_downloaded), (2, 0));
    // OneDrive giải phóng file: vẫn nằm đó nhưng dataless
    let st = scan(&mut s, &fonts, src, &mut Vec::new());
    assert_eq!((st.not_downloaded, st.missing), (1, 0));
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE missing=1"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 2, "dữ liệu cũ còn nguyên");
    assert_eq!(count(&s, "SELECT count(*) FROM analyses"), 2);
}

#[test]
fn icloud_stub_counts_as_not_downloaded_and_keeps_old_record() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(st.analyzed, 2);
    // iCloud thay file bằng ".open_sans.ttf.icloud"
    std::fs::remove_file(fonts.join("open_sans.ttf")).unwrap();
    std::fs::write(fonts.join(".open_sans.ttf.icloud"), b"").unwrap();
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!((st.not_downloaded, st.missing, st.seen), (1, 0, 1));
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE missing=1"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 2);
    // stub của file không phải font: bỏ qua, không đếm
    std::fs::write(fonts.join(".notes.txt.icloud"), b"").unwrap();
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(st.not_downloaded, 1);
}

#[test]
fn os_probe_is_false_for_ordinary_files() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf"]);
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!((st.not_downloaded, st.analyzed), (0, 1));
    let _ = fonts;
}
