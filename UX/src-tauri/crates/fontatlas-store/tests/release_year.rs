//! Library v5 (R2): năm phát hành ước lượng + ngày tạo file; sắp xếp NULL luôn cuối.
mod common;
use common::*;
use fontatlas_store::{FaceQuery, SortKey, SourceKind, Store, create_library_v4_for_test};
use std::sync::atomic::AtomicBool;

fn scanned(names: &[&str]) -> (tempfile::TempDir, Store) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in names {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    (d, s)
}

fn order(s: &Store, sort: SortKey, descending: bool) -> Vec<String> {
    let q = FaceQuery { sort, descending, limit: 100, ..Default::default() };
    s.query_faces(&q).unwrap().items.into_iter().map(|r| r.family).collect()
}

#[test]
fn scan_fills_release_year_and_file_created() {
    let (_d, s) = scanned(&["roboto.ttf", "open_sans.ttf", "playfair_display.ttf"]);
    let page = s.query_faces(&FaceQuery { limit: 10, ..Default::default() }).unwrap();
    assert_eq!(page.items.len(), 3);
    for r in &page.items {
        assert!(r.release_year.is_some_and(|y| (1990..=2027).contains(&y)), "{}: {:?}", r.family, r.release_year);
        assert!(matches!(r.release_year_source.as_deref(), Some("copyright" | "head_created" | "head_modified")), "{:?}", r.release_year_source);
        assert!(r.file_created_at.is_some_and(|t| t > 1_000_000_000_000), "{}: {:?}", r.family, r.file_created_at);
        assert_eq!(r.file_created_source.as_deref(), Some("disk"), "thư mục tạm không phải ổ cloud");
    }
}

#[test]
fn sort_released_and_file_created_put_unknown_last_in_both_directions() {
    let (_d, s) = scanned(&["roboto.ttf", "open_sans.ttf", "playfair_display.ttf"]);
    // ép giá trị: Roboto 2010, Open Sans 2020, Playfair không rõ (none)
    s.conn()
        .execute_batch(
            "UPDATE faces SET release_year=2010, release_year_src='copyright' WHERE family_name='Roboto';
             UPDATE faces SET release_year=2020, release_year_src='head_created' WHERE family_name='Open Sans';
             UPDATE faces SET release_year=NULL, release_year_src='none' WHERE family_name='Playfair Display';",
        )
        .unwrap();
    assert_eq!(order(&s, SortKey::Released, false), ["Roboto", "Open Sans", "Playfair Display"]);
    assert_eq!(order(&s, SortKey::Released, true), ["Open Sans", "Roboto", "Playfair Display"], "không rõ vẫn cuối khi giảm dần");
    let none = s.query_faces(&FaceQuery { limit: 10, ..Default::default() }).unwrap().items;
    let pf = none.iter().find(|r| r.family == "Playfair Display").unwrap();
    assert_eq!((pf.release_year, pf.release_year_source.clone()), (None, None), "`none` ẩn khỏi DTO");

    // ngày tạo file: Roboto cũ nhất, Open Sans mới nhất, Playfair chưa có
    s.conn()
        .execute_batch(
            "UPDATE file_locations SET created_ns = 1000000000000000000, created_src='disk'
               WHERE file_id IN (SELECT file_id FROM faces WHERE family_name='Roboto');
             UPDATE file_locations SET created_ns = 1700000000000000000, created_src='disk'
               WHERE file_id IN (SELECT file_id FROM faces WHERE family_name='Open Sans');
             UPDATE file_locations SET created_ns = NULL, created_src=NULL
               WHERE file_id IN (SELECT file_id FROM faces WHERE family_name='Playfair Display');",
        )
        .unwrap();
    assert_eq!(order(&s, SortKey::FileCreated, false), ["Roboto", "Open Sans", "Playfair Display"]);
    assert_eq!(order(&s, SortKey::FileCreated, true), ["Open Sans", "Roboto", "Playfair Display"]);
}

#[test]
fn ties_fall_back_to_family_order() {
    let (_d, s) = scanned(&["roboto.ttf", "open_sans.ttf"]);
    s.conn().execute_batch("UPDATE faces SET release_year=2015, release_year_src='copyright'").unwrap();
    assert_eq!(order(&s, SortKey::Released, false), ["Open Sans", "Roboto"]);
    assert_eq!(order(&s, SortKey::Released, true), ["Open Sans", "Roboto"], "cùng năm: luôn theo Family tăng dần");
}

#[test]
fn migration_v4_to_v5_backfills_from_stored_analysis_without_reading_files() {
    // dựng thư viện, hạ về v4 (bỏ cột v5), rồi mở lại: backfill từ JSON đã lưu
    let d = tmp();
    {
        let fonts = d.path().join("Fonts");
        std::fs::create_dir_all(&fonts).unwrap();
        std::fs::copy(core_fixture("roboto.ttf"), fonts.join("roboto.ttf")).unwrap();
        std::fs::copy(core_fixture("open_sans.ttf"), fonts.join("open_sans.ttf")).unwrap();
        let mut s = open_store(&d.path().join("db"));
        let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
        s.rescan_source_with(src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        // một bản phân tích không có `extras` (analyzer cũ) → phải giữ NULL
        s.conn()
            .execute_batch("UPDATE analyses SET metadata_json = json_remove(metadata_json, '$.extras') WHERE hash IN (SELECT fi.hash FROM faces f JOIN files fi USING(file_id) WHERE f.family_name='Open Sans');")
            .unwrap();
        s.conn().execute_batch("UPDATE faces SET release_year=NULL, release_year_src=NULL;").unwrap();
    }
    let s = Store::open(&d.path().join("db")).unwrap().0;
    let rows = s.query_faces(&FaceQuery { limit: 10, ..Default::default() }).unwrap().items;
    let roboto = rows.iter().find(|r| r.family == "Roboto").unwrap();
    assert!(roboto.release_year.is_some(), "backfill từ JSON đã lưu");
    let sans = rows.iter().find(|r| r.family == "Open Sans").unwrap();
    assert_eq!(sans.release_year, None, "chưa có extras: chờ phân tích lại");
    let pending: i64 = s.conn().query_row("SELECT count(*) FROM faces WHERE release_year_src IS NULL", [], |r| r.get(0)).unwrap();
    assert_eq!(pending, 1, "Open Sans vẫn chưa tính (NULL), không bị đánh `none`");
    let _ = create_library_v4_for_test; // helper cho test migration cấp schema (xem placeholders.rs)
}
