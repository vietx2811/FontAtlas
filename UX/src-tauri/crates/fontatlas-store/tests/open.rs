mod common;
use common::*;
use fontatlas_store::{LIBRARY_VERSION, Store, USER_VERSION};

fn pragma_i64(s: &Store, sql: &str) -> i64 {
    s.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}
fn pragma_str(s: &Store, sql: &str) -> String {
    s.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn fresh_open_creates_both_files_with_pragmas() {
    let d = tmp();
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(!rep.library_recreated && rep.user_backup.is_none());
    assert!(d.path().join("library.db").exists() && d.path().join("user.db").exists());
    assert_eq!(pragma_i64(&s, "PRAGMA main.page_size"), 16384);
    assert_eq!(pragma_str(&s, "PRAGMA main.journal_mode"), "wal");
    assert_eq!(pragma_str(&s, "PRAGMA u.journal_mode"), "wal");
    assert_eq!(pragma_i64(&s, "PRAGMA main.user_version"), LIBRARY_VERSION as i64);
    assert_eq!(pragma_i64(&s, "PRAGMA u.user_version"), USER_VERSION as i64);
    assert_eq!(pragma_i64(&s, "PRAGMA foreign_keys"), 1);
    for t in ["files", "file_locations", "faces", "faces_fts", "analyses", "embeddings", "pipeline_state", "community_snapshots", "meta"] {
        let n: i64 = s.conn().query_row("SELECT count(*) FROM main.sqlite_schema WHERE name=?1", [t], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "library thiếu {t}");
    }
    for t in ["sources", "favorites", "collections", "collection_items", "tags", "face_tags", "activation_state", "legacy_ids", "settings"] {
        let n: i64 = s.conn().query_row("SELECT count(*) FROM u.sqlite_schema WHERE name=?1", [t], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "user thiếu {t}");
    }
}

#[test]
fn reopen_is_idempotent_and_makes_no_backup() {
    let d = tmp();
    drop(Store::open(d.path()).unwrap());
    let (_s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.user_backup.is_none() && !rep.library_recreated);
    let n = std::fs::read_dir(d.path().join("backups")).map(|r| r.count()).unwrap_or(0);
    assert_eq!(n, 0);
}

#[test]
fn newer_user_db_is_refused_and_untouched() {
    let d = tmp();
    let up = d.path().join("user.db");
    {
        let c = rusqlite::Connection::open(&up).unwrap();
        c.execute_batch("CREATE TABLE keep(x); INSERT INTO keep VALUES (42); PRAGMA user_version = 99;").unwrap();
    }
    let before = std::fs::read(&up).unwrap();
    let err = Store::open(d.path()).err().expect("phải từ chối");
    assert_eq!(err.code(), "newer_schema");
    assert_eq!(std::fs::read(&up).unwrap(), before);
}

#[test]
fn newer_user_db_in_wal_is_untouched() {
    let d = tmp();
    let up = d.path().join("user.db");
    let wal = d.path().join("user.db-wal");
    let c = rusqlite::Connection::open(&up).unwrap();
    c.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
         CREATE TABLE keep(x); INSERT INTO keep VALUES (42); PRAGMA user_version = 99;",
    )
    .unwrap();
    std::mem::forget(c); // giữ kết nối mở để user.db-wal còn khung chưa checkpoint
    let wal_len = std::fs::metadata(&wal).unwrap().len();
    assert!(wal_len > 0, "phải có dữ liệu trong WAL");
    let before = (std::fs::read(&up).unwrap(), std::fs::read(&wal).unwrap());
    let err = Store::open(d.path()).err().expect("phải từ chối");
    assert_eq!(err.code(), "newer_schema");
    assert_eq!((std::fs::read(&up).unwrap(), std::fs::read(&wal).unwrap()), before);
}

#[test]
fn newer_or_broken_library_is_recreated_and_user_data_kept() {
    let d = tmp();
    {
        let (s, _) = Store::open(d.path()).unwrap();
        s.conn().execute("INSERT INTO u.settings(key, value) VALUES ('probe', '1')", []).unwrap();
    }
    {
        let c = rusqlite::Connection::open(d.path().join("library.db")).unwrap();
        c.execute_batch("PRAGMA user_version = 99;").unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.library_recreated);
    let v: String = s.conn().query_row("SELECT value FROM u.settings WHERE key='probe'", [], |r| r.get(0)).unwrap();
    assert_eq!(v, "1");
    let broken = std::fs::read_dir(d.path()).unwrap().filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with("library.broken-"));
    assert!(broken, "phải giữ file library cũ dưới tên library.broken-*");
}

#[test]
fn garbage_library_is_recreated() {
    let d = tmp();
    {
        let (s, _) = Store::open(d.path()).unwrap();
        s.conn().execute("INSERT INTO u.settings(key, value) VALUES ('probe', '2')", []).unwrap();
    }
    for f in ["library.db-wal", "library.db-shm"] {
        let _ = std::fs::remove_file(d.path().join(f));
    }
    std::fs::write(d.path().join("library.db"), b"rac rac rac").unwrap();
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.library_recreated);
    assert_eq!(pragma_i64(&s, "PRAGMA main.page_size"), 16384);
    let v: String = s.conn().query_row("SELECT value FROM u.settings WHERE key='probe'", [], |r| r.get(0)).unwrap();
    assert_eq!(v, "2");
}
