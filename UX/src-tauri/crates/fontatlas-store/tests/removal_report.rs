//! R7: `remove_source` trả số liệu thật và không đụng dữ liệu người dùng; `source_removal_report` chỉ đọc.
mod common;
use common::*;
use fontatlas_store::{FaceKey, SourceKind};
use std::sync::atomic::AtomicBool;

#[test]
fn report_counts_active_fonts_and_user_links_then_remove_changes_nothing_of_the_user() {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in ["roboto.ttf", "open_sans.ttf"] {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();

    let hash: Vec<u8> = s.conn().query_row("SELECT hash FROM files ORDER BY file_id LIMIT 1", [], |r| r.get(0)).unwrap();
    let mut h = [0u8; 32];
    h.copy_from_slice(&hash);
    let key = FaceKey { hash: h, face_index: 0 };
    s.conn()
        .execute("INSERT INTO u.activation_state(hash,state,scope,method,updated_at) VALUES (?1,'active','session','fake',0)", [&hash[..]])
        .unwrap();
    let tag = s.tag("giữ").unwrap();
    s.tag_faces(tag, &[key.clone()]).unwrap();
    s.set_favorite(&key, true).unwrap();

    let before = s.source_removal_report(src).unwrap();
    assert_eq!((before.removed_locations, before.faces_left_without_location, before.active_fonts, before.tags_affected), (2, 2, 1, 1));
    assert_eq!(s.source_removal_report(src).unwrap(), before, "chỉ đọc");

    let report = s.remove_source(src).unwrap();
    assert_eq!(report, before);
    assert_eq!(count(&s, "SELECT count(*) FROM u.face_tags"), 1, "gán thẻ còn");
    assert_eq!(count(&s, "SELECT count(*) FROM u.favorites"), 1, "yêu thích còn");
    assert_eq!(count(&s, "SELECT count(*) FROM u.activation_state WHERE state='active'"), 1, "font đang bật không tự tắt");
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 0);
}

#[test]
fn faces_whose_only_other_locations_are_missing_leave_the_grid() {
    let d = tmp();
    let a = d.path().join("A");
    let b = d.path().join("B");
    for p in [&a, &b] {
        std::fs::create_dir_all(p).unwrap();
        std::fs::copy(core_fixture("roboto.ttf"), p.join("roboto.ttf")).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let sa = s.add_source(SourceKind::Directory, "A", Some(&a)).unwrap();
    let sb = s.add_source(SourceKind::Directory, "B", Some(&b)).unwrap();
    for id in [sa, sb] {
        s.rescan_source_with(id, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    }
    assert_eq!(s.source_removal_report(sa).unwrap().faces_left_without_location, 0, "B còn hiện diện");
    // vị trí ở B bị đánh missing ⇒ gỡ A là font rời lưới
    s.conn().execute("UPDATE file_locations SET missing = 1 WHERE source_id = ?1", [sb]).unwrap();
    assert_eq!(s.source_removal_report(sa).unwrap().faces_left_without_location, 1);
}
