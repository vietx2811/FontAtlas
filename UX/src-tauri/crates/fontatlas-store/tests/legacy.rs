mod common;
use common::*;
use fontatlas_store::{FaceQuery, Store};
use unicode_normalization::UnicodeNormalization;

struct Env { _d: tempfile::TempDir, fonts: std::path::PathBuf, old: std::path::PathBuf, s: Store }

fn env(with_collections: bool) -> Env {
    let d = tmp();
    let fonts = d.path().join("Phông chữ"); // tên thư mục tiếng Việt
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), fonts.join("roboto.ttf")).unwrap();
    std::fs::copy(core_fixture("playfair_display.ttf"), fonts.join("playfair.ttf")).unwrap();
    std::fs::copy(core_fixture("open_sans.ttf"), fonts.join("open.ttf")).unwrap();
    let p = |n: &str| fonts.join(n).to_string_lossy().to_string();
    // Đường dẫn của Playfair lưu ở dạng NFD như HFS+ cũ.
    let playfair_nfd: String = p("playfair.ttf").nfd().collect();
    let (pr, po, pm) = (p("roboto.ttf"), p("open.ttf"), p("mat.ttf"));
    let old = d.path().join("fontatlas.db");
    make_legacy_db(&old, with_collections, &fonts, &[
        LegacyFont { id: "A", path: &pr, family: "Roboto", subfamily: "Regular", tags: r#"["Cổ điển","display"]"#, collection: Some("col-1"), favorite: true, source: Some("src-1") },
        LegacyFont { id: "B", path: &pm, family: "Mất", subfamily: "Regular", tags: "[]", collection: None, favorite: true, source: Some("src-1") },
        LegacyFont { id: "C", path: &playfair_nfd, family: "Playfair Display", subfamily: "Regular", tags: "[]", collection: Some("col-ma"), favorite: false, source: None },
        LegacyFont { id: "D", path: &po, family: "Open Sans", subfamily: "Regular", tags: "not json", collection: None, favorite: false, source: Some("src-1") },
    ]);
    let s = open_store(&d.path().join("db"));
    Env { _d: d, fonts, old, s }
}

#[test]
fn import_maps_sources_collections_and_keeps_old_db_intact() {
    let mut e = env(true);
    let before = std::fs::read(&e.old).unwrap();
    let r = e.s.import_legacy(&e.old).unwrap();
    assert_eq!(r.fonts, 4);
    assert_eq!(r.sources, 1);
    assert_eq!(r.collections, 1);
    assert_eq!(r.recovered_collections, 1, "col-ma không tồn tại -> 'Đã khôi phục'");
    assert_eq!(r.bad_tags, 1);
    assert_eq!(std::fs::read(&e.old).unwrap(), before, "DB cũ mở chỉ đọc");
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.legacy_ids WHERE hash IS NULL"), 4);
    assert!(e.s.collections().unwrap().iter().any(|c| c.name.starts_with("Đã khôi phục")));
}

#[test]
fn import_is_idempotent() {
    let mut e = env(true);
    let a = e.s.import_legacy(&e.old).unwrap();
    let b = e.s.import_legacy(&e.old).unwrap();
    assert_eq!(a, b);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.legacy_ids"), 4);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.collections"), 2);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.sources"), 1);
}

#[test]
fn missing_collections_table_is_tolerated() {
    let mut e = env(false);
    let r = e.s.import_legacy(&e.old).unwrap();
    assert_eq!(r.collections, 0);
    assert_eq!(r.recovered_collections, 2, "col-1 và col-ma đều phải khôi phục");
}

#[test]
fn resolve_after_rescan_applies_user_data() {
    let mut e = env(true);
    e.s.import_legacy(&e.old).unwrap();
    let src = e.s.sources().unwrap()[0].id;
    e.s.rescan_source(src).unwrap();
    let r = e.s.resolve_legacy().unwrap();
    assert_eq!(r, fontatlas_store::LegacyResolve { resolved: 3, unresolved: 1, ambiguous_faces: 0 });
    let fav = e.s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap().items;
    assert_eq!(fav.iter().map(|f| f.family.as_str()).collect::<Vec<_>>(), vec!["Roboto"]);
    let roboto = e.s.legacy_face("A").unwrap().expect("A đã nối");
    let tags: Vec<String> = e.s.tags_of(&roboto).unwrap().into_iter().map(|t| t.name).collect();
    assert_eq!(tags, vec!["Cổ điển".to_string(), "display".to_string()]);
    let cols = e.s.collections().unwrap();
    assert_eq!(cols.iter().find(|c| c.name == "Bộ cũ").unwrap().item_count, 1);
    assert!(e.s.legacy_face("C").unwrap().is_some(), "đường dẫn NFD phải khớp file NFC");
    assert!(e.s.legacy_face("B").unwrap().is_none(), "file đã mất vẫn chờ");
    // chạy lại không nhân đôi
    assert_eq!(e.s.resolve_legacy().unwrap().resolved, 0);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.face_tags"), 2);
}

#[test]
fn lost_file_resolves_when_it_reappears() {
    let mut e = env(true);
    e.s.import_legacy(&e.old).unwrap();
    let src = e.s.sources().unwrap()[0].id;
    std::fs::copy(core_fixture("libre_baskerville.ttf"), e.fonts.join("mat.ttf")).unwrap();
    e.s.rescan_source(src).unwrap();
    e.s.resolve_legacy().unwrap();
    assert!(e.s.legacy_face("B").unwrap().is_some());
    let fav = e.s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap();
    assert_eq!(fav.total, 2);
}

#[test]
fn garbage_or_foreign_db_is_rejected_without_partial_writes() {
    let mut e = env(true);
    let junk = e.fonts.join("junk.db");
    std::fs::write(&junk, b"SQLite format 3\0 nhung hong").unwrap();
    assert_eq!(e.s.import_legacy(&junk).unwrap_err().code(), "legacy");
    let other = e.fonts.join("other.db");
    rusqlite::Connection::open(&other).unwrap().execute_batch("CREATE TABLE x(y);").unwrap();
    assert_eq!(e.s.import_legacy(&other).unwrap_err().code(), "legacy");
    for t in ["legacy_ids", "sources", "collections", "tags"] {
        assert_eq!(count(&e.s, &format!("SELECT count(*) FROM u.{t}")), 0, "{t} phải rỗng");
    }
}

#[test]
fn old_db_missing_newer_columns_still_imports() {
    let d = tmp();
    let old = d.path().join("fontatlas.db");
    {
        let c = rusqlite::Connection::open(&old).unwrap();
        // Bản rất cũ: chưa có is_favorite, family_name, source_id...
        c.execute_batch("CREATE TABLE fonts (id TEXT PRIMARY KEY, name TEXT NOT NULL, file_path TEXT NOT NULL UNIQUE,
            tags TEXT, collection_id TEXT, date_added TEXT NOT NULL, date_modified TEXT NOT NULL);
            INSERT INTO fonts VALUES ('Z','Cũ','/x/cu.ttf','[\"a\"]',NULL,'t','t');
            INSERT INTO fonts VALUES ('N','Null tags','/x/n.ttf',NULL,NULL,'t','t');
            INSERT INTO fonts VALUES ('I','Int tags','/x/i.ttf',5,NULL,'t','t');").unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let r = s.import_legacy(&old).unwrap();
    assert_eq!((r.fonts, r.sources, r.bad_tags), (3, 0, 1), "NULL không tính là lỗi; số nguyên là lỗi");
}

#[test]
fn failure_inside_import_transaction_leaves_nothing() {
    let mut e = env(true);
    e.s.conn()
        .execute_batch(
            "CREATE TEMP TRIGGER boom BEFORE INSERT ON u.legacy_ids
             WHEN (SELECT count(*) FROM u.legacy_ids) >= 2
             BEGIN SELECT RAISE(ABORT, 'boom'); END;",
        )
        .unwrap();
    assert!(e.s.import_legacy(&e.old).is_err());
    for t in ["legacy_ids", "sources", "collections", "tags", "settings"] {
        assert_eq!(count(&e.s, &format!("SELECT count(*) FROM u.{t}")), 0, "{t} phải rỗng sau rollback");
    }
}

#[test]
fn nested_legacy_sources_keep_parent_regardless_of_id_order() {
    let d = tmp();
    let root = d.path().join("Fonts");
    let sub = root.join("Sub");
    std::fs::create_dir_all(&sub).unwrap();
    let old = d.path().join("fontatlas.db");
    {
        let c = rusqlite::Connection::open(&old).unwrap();
        c.execute_batch(LEGACY_FONTS_DDL).unwrap();
        // "src-10" < "src-2" theo chuỗi: con được đọc trước cha
        c.execute("INSERT INTO sources VALUES ('src-10','Con','directory',?1,NULL,'t')", [sub.to_string_lossy()]).unwrap();
        c.execute("INSERT INTO sources VALUES ('src-2','Cha','directory',?1,NULL,'t')", [root.to_string_lossy()]).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let r = s.import_legacy(&old).unwrap();
    let paths: Vec<String> = s.sources().unwrap().into_iter().filter_map(|x| x.path).collect();
    assert_eq!(paths, vec![root.to_string_lossy().to_string()]);
    assert_eq!((r.sources, r.skipped_sources), (1, 1));
}
