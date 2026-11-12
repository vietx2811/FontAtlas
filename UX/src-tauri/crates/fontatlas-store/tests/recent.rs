mod common;
use common::*;
use fontatlas_store::{FaceKey, RECENT_MAX, Store, create_user_pre_auto_for_test};

fn key(n: u16, i: u32) -> FaceKey {
    let mut h = [0u8; 32];
    h[..2].copy_from_slice(&n.to_le_bytes());
    FaceKey { hash: h, face_index: i }
}

#[test]
fn newest_first_revisit_moves_to_top_no_duplicates() {
    let d = tmp();
    let mut s = open_store(d.path());
    for n in 1..=3 {
        s.record_view(&key(n, 0)).unwrap();
    }
    s.record_view(&key(1, 0)).unwrap();
    let got: Vec<FaceKey> = s.recent_views(10).unwrap().into_iter().map(|x| x.0).collect();
    assert_eq!(got, vec![key(1, 0), key(3, 0), key(2, 0)]);
}

#[test]
fn capped_at_max_and_keeps_newest() {
    let d = tmp();
    let mut s = open_store(d.path());
    for n in 0..(RECENT_MAX as u16 + 10) {
        s.record_view(&key(n, 0)).unwrap();
    }
    let got = s.recent_views(1000).unwrap();
    assert_eq!(got.len(), RECENT_MAX);
    assert_eq!(got[0].0, key(RECENT_MAX as u16 + 9, 0));
    assert_eq!(got.last().unwrap().0, key(10, 0));
    assert_eq!(s.recent_views(5).unwrap().len(), 5);
}

#[test]
fn faces_of_one_file_are_distinct_and_clear_empties() {
    let d = tmp();
    let mut s = open_store(d.path());
    s.record_view(&key(1, 0)).unwrap();
    s.record_view(&key(1, 1)).unwrap();
    assert_eq!(s.recent_views(10).unwrap().len(), 2);
    assert_eq!(s.clear_recent_views().unwrap(), 2);
    assert!(s.recent_views(10).unwrap().is_empty());
    assert_eq!(s.clear_recent_views().unwrap(), 0);
}

#[test]
fn migration_from_previous_user_version_keeps_data() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_pre_auto_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute("INSERT INTO collections(name,created_at,updated_at) VALUES ('Giữ',1,1)", []).unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    assert_eq!(s.collections().unwrap().len(), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.sqlite_schema WHERE name='recent_faces'"), 1);
    assert!(rep.user_backup.is_some());
}
