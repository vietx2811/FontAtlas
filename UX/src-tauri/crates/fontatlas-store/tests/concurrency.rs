mod common;
use common::*;
use fontatlas_store::{FaceKey, Store};
use std::time::{Duration, Instant};

fn key(n: u8) -> FaceKey {
    FaceKey { hash: [n; 32], face_index: 0 }
}

#[test]
fn concurrent_writers_do_not_fail_busy() {
    let d = tmp();
    let dir = d.path().to_path_buf();
    let (mut a, _) = Store::open(&dir).unwrap();
    let (mut b, _) = Store::open(&dir).unwrap();
    let ta = std::thread::spawn(move || {
        for i in 0..200u32 {
            a.set_favorite(&key((i % 50) as u8), i % 2 == 0).unwrap();
        }
    });
    let tb = std::thread::spawn(move || {
        for i in 0..200 {
            let id = b.create_collection(&format!("c{i}"), None).unwrap();
            b.rename_collection(id, &format!("d{i}")).unwrap();
        }
        b
    });
    ta.join().unwrap();
    let b = tb.join().unwrap();
    assert_eq!(b.collections().unwrap().len(), 200);
}

#[test]
fn user_only_handle_unaffected_by_ingest_lock() {
    let d = tmp();
    {
        Store::open(d.path()).unwrap();
    }
    let dir = d.path().to_path_buf();
    let lib = dir.join(fontatlas_store::LIBRARY_FILE);
    let (tx, rx) = std::sync::mpsc::channel();
    // Kết nối ghi library.db KHÔNG attach user.db (như luồng ingest chỉ chạm library): giữ khoá ghi 1 s.
    // (BEGIN IMMEDIATE trên kết nối có attach khoá cả user.db trong lúc giữ; lô ingest ngắn nên chấp nhận.)
    let holder = std::thread::spawn(move || {
        let c = rusqlite::Connection::open(lib).unwrap();
        c.execute_batch("BEGIN IMMEDIATE; INSERT INTO meta(key,value) VALUES('lock','1');").unwrap();
        tx.send(()).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        c.execute_batch("COMMIT").unwrap();
    });
    rx.recv().unwrap();
    let mut u = Store::open_user_only(&dir).unwrap();
    for i in 0..50u8 {
        let t = Instant::now();
        u.set_favorite(&key(i), true).unwrap();
        assert!(t.elapsed() < Duration::from_millis(200), "lần {i}: {:?}", t.elapsed());
    }
    holder.join().unwrap();
}

#[test]
fn open_user_only_does_not_touch_library_db() {
    let d = tmp();
    {
        Store::open(d.path()).unwrap();
    }
    let lib = d.path().join(fontatlas_store::LIBRARY_FILE);
    let before = fingerprint(&lib);
    let mut u = Store::open_user_only(d.path()).unwrap();
    u.set_favorite(&key(1), true).unwrap();
    assert_eq!(fingerprint(&lib), before);
    for ext in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ext}", lib.display()));
    }
    let mut u2 = Store::open_user_only(d.path()).unwrap();
    u2.set_favorite(&key(2), true).unwrap();
    assert_eq!(u2.favorites().unwrap().len(), 2);
}

#[test]
fn reader_is_query_only() {
    let d = tmp();
    let (mut a, _) = Store::open(d.path()).unwrap();
    a.set_favorite(&key(1), true).unwrap();
    let r = Store::open_reader(d.path()).unwrap();
    assert!(r.conn().execute("INSERT INTO meta(key,value) VALUES('x','y')", []).is_err());
    assert_eq!(r.favorites().unwrap().len(), 1);
    // đọc được khi A đang giữ khoá ghi (WAL)
    a.conn().execute_batch("BEGIN IMMEDIATE; INSERT INTO meta(key,value) VALUES('lock','1');").unwrap();
    let n: i64 = r.conn().query_row("SELECT count(*) FROM faces", [], |x| x.get(0)).unwrap();
    assert_eq!(n, 0);
    a.conn().execute_batch("COMMIT").unwrap();
}

#[test]
fn library_only_writer_does_not_lock_user_db() {
    let d = tmp();
    {
        Store::open(d.path()).unwrap();
    }
    let lib = Store::open_library_only(d.path()).unwrap();
    lib.conn().execute_batch("BEGIN IMMEDIATE; INSERT INTO meta(key,value) VALUES('lock','1');").unwrap();
    let mut u = Store::open_user_only(d.path()).unwrap();
    for i in 0..50u8 {
        let t = Instant::now();
        u.set_favorite(&key(i), true).unwrap();
        assert!(t.elapsed() < Duration::from_millis(200), "lần {i}: {:?}", t.elapsed());
    }
    lib.conn().execute_batch("COMMIT").unwrap();
    assert!(lib.source(1).is_err(), "kết nối chỉ library không có bảng u.*");
}
