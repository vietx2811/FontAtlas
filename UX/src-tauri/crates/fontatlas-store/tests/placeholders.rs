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

/// Đổi `ANALYZER_VERSION` (core-0.8.0 → hiện tại) KHÔNG được kéo nội dung file dataless: phân tích lại chỉ chạy trên
/// file đã có trên máy; file dataless giữ bản phân tích cũ (vẫn dùng được), đếm là "chờ phân tích lại", và được
/// phân tích lại ở lần quét đầu tiên sau khi file đã tải về.
#[test]
fn analyzer_version_bump_skips_dataless_files_and_reanalyzes_after_download() {
    use std::os::unix::fs::PermissionsExt;
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(st.analyzed, 2);
    assert_eq!(s.stale_analysis_count().unwrap(), 0);
    // mô phỏng thư viện phân tích bằng bản cũ
    s.conn().execute_batch("UPDATE analyses SET analyzer_version='core-0.8.0'; UPDATE pipeline_state SET version='core-0.8.0';").unwrap();
    let before_faces = count(&s, "SELECT count(*) FROM faces");
    assert_eq!(s.stale_analysis_count().unwrap(), before_faces as u64);
    // open_sans bị OneDrive giải phóng (dataless) và chặn đọc: nếu bị mở sẽ thành "unreadable"
    let ph = fonts.join("open_sans.ttf"); // cùng đường dẫn như lúc đã phân tích
    std::fs::set_permissions(&ph, std::fs::Permissions::from_mode(0o000)).unwrap();
    let dataless = |p: &Path, _m: &std::fs::Metadata| p.file_name().is_some_and(|n| n == "open_sans.ttf");
    let st = s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &dataless).unwrap();
    assert_eq!(st.status, RescanStatus::Ok);
    assert!(st.unreadable.is_empty(), "file dataless bị mở: {:?}", st.unreadable);
    assert_eq!((st.analyzed, st.not_downloaded, st.missing), (1, 1, 0), "chỉ file có trên máy được phân tích lại");
    // dữ liệu cũ của file dataless vẫn đó (dùng được), còn đang chờ phân tích lại
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), before_faces, "không mất face nào");
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE missing=1"), 0);
    let pending = s.stale_analysis_count().unwrap();
    assert!(pending > 0 && pending < before_faces as u64, "{pending}/{before_faces}");
    assert_eq!(count(&s, "SELECT count(*) FROM analyses WHERE analyzer_version='core-0.8.0'") as u64, pending);
    // file đã tải về máy (không còn dataless): lần quét sau phân tích lại
    std::fs::set_permissions(&ph, std::fs::Permissions::from_mode(0o644)).unwrap();
    let st = s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    assert_eq!(st.analyzed, 1);
    assert_eq!(s.stale_analysis_count().unwrap(), 0);
}
