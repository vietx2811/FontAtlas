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

fn row(s: &Store, family: &str) -> fontatlas_store::FaceRow {
    use fontatlas_store::FaceQuery;
    s.query_faces(&FaceQuery { text: Some(family.into()), include_missing: true, ..Default::default() })
        .unwrap()
        .items
        .into_iter()
        .next()
        .expect(family)
}

#[test]
fn evicted_file_is_marked_cloud_only_then_cleared_when_downloaded() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    std::fs::rename(fonts.join("open_sans.ttf"), fonts.join("evicted_open_sans.ttf")).unwrap();
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    assert!(!row(&s, "open sans").on_cloud_only, "đang ở máy");

    // OneDrive giải phóng file: dataless
    let st = scan(&mut s, &fonts, src, &mut Vec::new());
    assert_eq!((st.not_downloaded, st.missing), (1, 0));
    let r = row(&s, "open sans");
    assert!(r.on_cloud_only && !r.missing, "{r:?}");
    assert!(!row(&s, "roboto").on_cloud_only);
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 2, "giữ dữ liệu phân tích cũ");

    // người dùng tải về lại (cùng size/mtime): cờ được xoá
    let st = s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    assert_eq!(st.not_downloaded, 0);
    assert!(!row(&s, "open sans").on_cloud_only);
}

#[test]
fn one_local_copy_is_enough_to_not_be_cloud_only() {
    // cùng nội dung ở hai vị trí: một bản dataless, một bản trên máy → face dùng được ngay
    let (d, fonts, mut s, src) = setup(&["open_sans.ttf"]);
    let other = d.path().join("Local");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(fonts.join("open_sans.ttf"), other.join("open_sans.ttf")).unwrap();
    let src2 = s.add_source(SourceKind::Directory, "Local", Some(&other)).unwrap();
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    s.rescan_root_with_probe(src2, &other, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    let evict_all = |p: &Path, _m: &std::fs::Metadata| p.starts_with(&fonts);
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &evict_all).unwrap();
    assert!(!row(&s, "open sans").on_cloud_only);
}

#[test]
fn migration_v3_to_v4_adds_cloud_only_defaulting_to_local() {
    use fontatlas_store::create_library_v3_for_test;
    let d = tmp();
    {
        let c = rusqlite::Connection::open(d.path().join("library.db")).unwrap();
        create_library_v3_for_test(&c);
        c.execute("INSERT INTO files (file_id, hash, size, container, num_faces, first_seen_at) VALUES (1, zeroblob(32), 7, 'sfnt', 1, 1)", []).unwrap();
        c.execute("INSERT INTO file_locations (file_id, source_id, path, size, mtime_ns, last_seen_at) VALUES (1, 1, '/x/a.ttf', 7, 1, 1)", []).unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(!rep.library_recreated);
    assert_eq!(count(&s, "SELECT cloud_only FROM file_locations WHERE path='/x/a.ttf'"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 1, "dữ liệu cũ còn nguyên");
}

#[test]
fn face_local_path_never_returns_a_cloud_placeholder() {
    use fontatlas_store::FaceKey;
    let (d, fonts, mut s, src) = setup(&["open_sans.ttf"]);
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    let r = row(&s, "open sans");
    let key = FaceKey::from_hex(&r.hash_hex, r.face_index).unwrap();
    assert!(s.face_local_path(&key).unwrap().is_some(), "đang ở máy");

    // OneDrive giải phóng bản duy nhất: xem preview/FontFace không được kéo nó về ngầm
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| true).unwrap();
    assert!(s.face_local_path(&key).unwrap().is_none(), "chỉ còn placeholder → không có đường dẫn đọc được");
    assert!(s.face_path(&key).unwrap().is_some(), "face_path (hiện đường dẫn, Finder…) vẫn trả vị trí");

    // thêm một bản sao trên máy → dùng bản đó
    let other = d.path().join("Local");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(fonts.join("open_sans.ttf"), other.join("copy.ttf")).unwrap();
    let src2 = s.add_source(SourceKind::Directory, "Local", Some(&other)).unwrap();
    s.rescan_root_with_probe(src2, &other, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    let p = s.face_local_path(&key).unwrap().expect("bản trên máy");
    assert!(p.starts_with(&other), "{p:?}");
}

#[test]
fn face_read_path_prefers_a_local_copy_and_falls_back_to_the_cloud_one() {
    use fontatlas_store::FaceKey;
    let (d, fonts, mut s, src) = setup(&["open_sans.ttf"]);
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    let r = row(&s, "open sans");
    let key = FaceKey::from_hex(&r.hash_hex, r.face_index).unwrap();
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| true).unwrap();
    // chỉ còn bản cloud: cho phép đọc (sẽ được tải về lười) khi người dùng bật tự tải xem trước
    assert_eq!(s.face_read_path(&key).unwrap(), Some(fonts.join("open_sans.ttf")));
    // thêm bản trên máy: ưu tiên bản đó dù location_id lớn hơn
    let other = d.path().join("Local");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(fonts.join("open_sans.ttf"), other.join("copy.ttf")).unwrap();
    let src2 = s.add_source(SourceKind::Directory, "Local", Some(&other)).unwrap();
    s.rescan_root_with_probe(src2, &other, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    assert_eq!(s.face_read_path(&key).unwrap(), Some(other.join("copy.ttf")));
}

#[test]
fn cloud_only_count_per_source_counts_faces_with_a_placeholder_copy_there() {
    let (_d, fonts, mut s, src) = setup(&["open_sans.ttf", "roboto.ttf"]);
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    assert_eq!(s.cloud_only_count(src).unwrap(), 0);
    let evict = |p: &Path, _m: &std::fs::Metadata| p.ends_with("open_sans.ttf");
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &evict).unwrap();
    assert_eq!(s.cloud_only_count(src).unwrap(), 1);
    assert_eq!(s.cloud_only_count(src + 99).unwrap(), 0);
}

#[test]
fn is_dataless_reads_only_the_flag_and_is_false_for_ordinary_or_missing_files() {
    use fontatlas_store::is_dataless;
    let (_d, fonts, _s, _src) = setup(&["roboto.ttf"]);
    assert!(!is_dataless(&fonts.join("roboto.ttf")));
    assert!(!is_dataless(&fonts.join("khong-co.ttf")), "không stat được → không coi là dataless");
    assert!(!is_dataless(&fonts), "thư mục không phải file giữ chỗ");
}

#[test]
fn refresh_cloud_flag_follows_the_real_state_of_that_file() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    let flag = |s: &Store, f: &str| count(s, &format!("SELECT cloud_only FROM file_locations WHERE path LIKE '%{f}'"));
    s.conn().execute("UPDATE file_locations SET cloud_only = 1", []).unwrap();
    // đã tải về: stat lại thấy không còn dataless → cờ về 0, chỉ đúng vị trí đó
    assert_eq!(s.refresh_cloud_flag_with(&fonts.join("roboto.ttf"), &|_, _| false).unwrap(), Some(false));
    assert_eq!((flag(&s, "roboto.ttf"), flag(&s, "open_sans.ttf")), (0, 1));
    // bị giải phóng lại: cờ lên 1
    assert_eq!(s.refresh_cloud_flag_with(&fonts.join("roboto.ttf"), &|_, _| true).unwrap(), Some(true));
    assert_eq!(flag(&s, "roboto.ttf"), 1);
    // file không stat được: không biết → không đổi gì
    assert_eq!(s.refresh_cloud_flag_with(&fonts.join("khong-co.ttf"), &|_, _| false).unwrap(), None);
    assert_eq!(s.refresh_cloud_flag(&fonts.join("khong-co.ttf")).unwrap(), None);
    // bản dùng stat thật: file thường → 0
    assert_eq!(s.refresh_cloud_flag(&fonts.join("roboto.ttf")).unwrap(), Some(false));
    assert_eq!(flag(&s, "roboto.ttf"), 0);
}

fn load_version(s: &Store) -> String {
    // trạng thái của hash mà vị trí `roboto.ttf` đang trỏ tới (hash cũ có thể còn dòng riêng cho tới khi gc)
    s.conn()
        .query_row(
            "SELECT p.version FROM pipeline_state p JOIN files fi ON fi.hash = p.hash JOIN file_locations l ON l.file_id = fi.file_id
             WHERE p.face_index = -1 AND p.stage = 'load' AND l.path LIKE '%roboto.ttf'",
            [],
            |r| r.get(0),
        )
        .unwrap()
}

#[test]
fn unchanged_placeholder_keeps_its_analysis_fresh() {
    use fontatlas_core::ANALYZER_VERSION;
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf"]);
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| true).unwrap();
    assert_eq!(load_version(&s), ANALYZER_VERSION, "size/mtime khớp → dữ liệu cũ vẫn đáng tin");
    assert!(row(&s, "roboto").on_cloud_only);
}

#[test]
fn placeholder_whose_size_or_mtime_changed_is_not_trusted_and_is_reanalyzed_after_download() {
    use fontatlas_core::ANALYZER_VERSION;
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf"]);
    s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    // nội dung đổi trên cloud (cùng đường dẫn, khác size) rồi file bị giải phóng: ta chỉ thấy placeholder mới
    std::fs::copy(core_fixture("playfair_display.ttf"), fonts.join("roboto.ttf")).unwrap();
    let st = s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| true).unwrap();
    assert_eq!((st.not_downloaded, st.missing, st.analyzed, st.failed), (1, 0, 0, 0), "không đọc nội dung");
    assert_ne!(load_version(&s), ANALYZER_VERSION, "không giữ kết quả của hash cũ như thể còn đúng");
    let new_size = std::fs::metadata(fonts.join("roboto.ttf")).unwrap().len() as i64;
    assert_eq!(count(&s, "SELECT size FROM file_locations WHERE path LIKE '%roboto.ttf'"), new_size);
    assert!(row(&s, "roboto").on_cloud_only, "vẫn hiện (dữ liệu cũ) và vẫn ghi là trên cloud");
    // tải về: lần quét sau băm lại và phân tích lại nội dung mới
    let st = s.rescan_root_with_probe(src, &fonts, &mut |_| {}, &AtomicBool::new(false), &|_, _| false).unwrap();
    assert_eq!(st.analyzed, 1);
    let p = row(&s, "playfair");
    assert!(!p.on_cloud_only);
    assert_eq!(load_version(&s), ANALYZER_VERSION);
}
