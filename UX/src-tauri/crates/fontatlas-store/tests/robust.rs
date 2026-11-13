//! D12 (0.5.0): thư viện tự vệ — gỡ nguồn sống sót qua crash giữa hai DB, rescan tự dọn rác, dựng lại thư viện.
mod common;
use common::*;
use fontatlas_store::{FaceKey, SourceKind, Store, StoreError};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

fn setup(names: &[&str]) -> (tempfile::TempDir, PathBuf, Store, i64) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in names {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    (d, fonts, s, src)
}

fn first_key(s: &Store) -> FaceKey {
    let hash: Vec<u8> = s.conn().query_row("SELECT hash FROM files ORDER BY file_id LIMIT 1", [], |r| r.get(0)).unwrap();
    let mut h = [0u8; 32];
    h.copy_from_slice(&hash);
    FaceKey { hash: h, face_index: 0 }
}

#[test]
fn crash_between_the_two_writes_leaves_only_orphans_that_heal() {
    let (d, _fonts, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    let key = first_key(&s);
    let tag = s.tag("giữ").unwrap();
    s.tag_faces(tag, &[key]).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2);

    // "crash" ngay sau khi u.sources đã commit, trước khi xoá vị trí
    let err = s.remove_source_with(src, &mut || Err(StoreError::Invalid("crash".into()))).unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)));
    drop(s); // tiến trình chết: kết nối đóng, không dọn gì thêm

    let mut s = open_store(&d.path().join("db"));
    // trạng thái sau crash: nguồn đã biến mất khỏi danh sách (không "còn tên mà hết font"), vị trí mồ côi còn nằm lại
    assert!(s.sources().unwrap().is_empty());
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2);
    assert_eq!(count(&s, "SELECT count(*) FROM u.face_tags"), 1, "dữ liệu người dùng nguyên vẹn");

    // khởi động lại dọn
    assert_eq!(s.sweep_orphan_locations().unwrap(), 2);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM u.face_tags"), 1);
    assert_eq!(s.sweep_orphan_locations().unwrap(), 0, "idempotent");
}

#[test]
fn removing_again_after_a_crash_is_safe_and_finishes_the_job() {
    let (_d, _f, mut s, src) = setup(&["roboto.ttf"]);
    let _ = s.remove_source_with(src, &mut || Err(StoreError::Invalid("crash".into())));
    s.remove_source(src).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 0);
}

#[test]
fn sweep_keeps_locations_of_existing_sources() {
    let (_d, _f, mut s, _src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    assert_eq!(s.sweep_orphan_locations().unwrap(), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2);
}

#[test]
fn rescan_collects_garbage_after_a_file_is_deleted_for_good() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    let ms = 24 * 3600 * 1000i64;
    let gone_hash: Vec<u8> = s
        .conn()
        .query_row("SELECT fi.hash FROM files fi JOIN file_locations l USING(file_id) WHERE l.path LIKE '%roboto.ttf'", [], |r| r.get(0))
        .unwrap();
    std::fs::remove_file(fonts.join("roboto.ttf")).unwrap();

    // vừa xoá: đánh missing nhưng còn trong thời gian ân hạn (file có thể chỉ chuyển chỗ)
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!((st.missing, st.purged), (1, 0));
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 2);

    // quá ân hạn: lần quét kế tiếp tự dọn location, file, face, phân tích và pipeline_state của hash đó
    s.conn().execute("UPDATE file_locations SET last_seen_at = last_seen_at - ?1 WHERE missing = 1", [31 * ms]).unwrap();
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(st.purged, 1);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 1);
    for t in ["analyses", "pipeline_state"] {
        let n: i64 = s.conn().query_row(&format!("SELECT count(*) FROM {t} WHERE hash = ?1"), [&gone_hash[..]], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "{t} của hash đã xoá phải sạch");
    }
    assert!(count(&s, "SELECT count(*) FROM faces") > 0, "font còn trên đĩa không bị đụng");
    fts_ok(&s);
}

#[test]
fn cancelled_or_unavailable_scan_never_purges() {
    let (_d, fonts, mut s, src) = setup(&["roboto.ttf"]);
    std::fs::remove_file(fonts.join("roboto.ttf")).unwrap();
    s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    s.conn().execute("UPDATE file_locations SET last_seen_at = 0 WHERE missing = 1", []).unwrap();
    // ổ rút (thư mục gốc không còn): không đánh gì, không dọn gì
    let moved = fonts.with_file_name("Fonts_unplugged");
    std::fs::rename(&fonts, &moved).unwrap();
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(st.purged, 0);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 1);
    // huỷ
    std::fs::rename(&moved, &fonts).unwrap();
    let st = s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(true)).unwrap();
    assert_eq!(st.purged, 0);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 1);
}

#[test]
fn rebuild_clears_library_keeps_user_data_and_rescan_restores_it_on_a_temp_dir() {
    let (_d, _f, mut s, src) = setup(&["roboto.ttf", "open_sans.ttf"]);
    let key = first_key(&s);
    let tag = s.tag("giữ").unwrap();
    s.tag_faces(tag, &[key]).unwrap();
    s.set_favorite(&key, true).unwrap();
    let faces = count(&s, "SELECT count(*) FROM faces");
    assert!(faces >= 2);

    let rep = s.rebuild_library().unwrap();
    assert_eq!((rep.files, rep.faces as i64, rep.locations), (2, faces, 2));
    for t in ["files", "faces", "file_locations", "analyses", "pipeline_state", "embeddings", "face_langs", "face_lang_state"] {
        assert_eq!(count(&s, &format!("SELECT count(*) FROM {t}")), 0, "{t}");
    }
    fts_ok(&s);
    assert_eq!(fts_hits(&s, "roboto"), 0);
    // user.db nguyên vẹn, nguồn còn
    assert_eq!(s.sources().unwrap().len(), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.face_tags"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.favorites"), 1);

    // quét lại nguồn ⇒ thư viện trở lại như cũ, gán thẻ/yêu thích vẫn khớp theo hash
    s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), faces);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2);
    assert!(fts_hits(&s, "roboto") > 0);
    assert_eq!(count(&s, "SELECT count(*) FROM u.favorites"), 1);
}
