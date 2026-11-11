mod common;
use common::*;
use fontatlas_store::{FaceKey, NewPair, Store, create_user_v1_for_test, create_user_v2_for_test, create_user_v3_for_test, create_user_v4_for_test};

fn key(n: u8) -> FaceKey {
    FaceKey { hash: [n; 32], face_index: 0 }
}

fn backups(dir: &std::path::Path, prefix: &str) -> usize {
    std::fs::read_dir(dir.join("backups"))
        .map(|rd| rd.flatten().filter(|e| e.file_name().to_string_lossy().starts_with(prefix)).count())
        .unwrap_or(0)
}

#[test]
fn v1_to_v3_migrates() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_v1_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute("INSERT INTO collections(name,created_at,updated_at) VALUES ('Giữ',1,1)", []).unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(fontatlas_store::USER_VERSION >= 6);
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    assert_eq!(s.collections().unwrap().len(), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.sqlite_schema WHERE name IN ('activation_reasons','pairs')"), 2);
    assert!(rep.user_backup.unwrap().file_name().unwrap().to_string_lossy().starts_with("user.v1."));
}

#[test]
fn v2_to_v3_migrates_keeps_activation_rows() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_v2_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute(
            "INSERT INTO activation_state(hash,state,scope,method,active_path,updated_at) VALUES (?1,'active','persistent','fake','/x.ttf',1)",
            [&[5u8; 32][..]],
        )
        .unwrap();
        c.execute("INSERT INTO activation_reasons(hash,reason,added_at) VALUES (?1,'manual',1)", [&[5u8; 32][..]]).unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    assert_eq!(count(&s, "SELECT count(*) FROM u.activation_state"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.activation_reasons"), 1);
    assert!(rep.user_backup.unwrap().file_name().unwrap().to_string_lossy().starts_with("user.v2."));
}

#[test]
fn v3_failure_rolls_back_to_v2() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_v2_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute_batch("CREATE TABLE pairs(x); INSERT INTO pairs VALUES (1);").unwrap();
    }
    let err = Store::open(d.path()).err().expect("phải lỗi");
    match err {
        fontatlas_store::StoreError::Migration { from, to, .. } => assert_eq!((from, to), (2, 3)),
        e => panic!("{e:?}"),
    }
    let c = rusqlite::Connection::open(&user).unwrap();
    assert_eq!(c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0)).unwrap(), 2);
    assert_eq!(c.query_row("SELECT x FROM pairs", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    assert_eq!(backups(d.path(), "user.v2."), 1);
}

#[test]
fn pairs_roundtrip_and_collection_delete_sets_null() {
    let d = tmp();
    let mut s = open_store(d.path());
    let col = s.create_collection("Menu", None).unwrap();
    let np = |a: u8, b: u8, c: Option<i64>| NewPair {
        a: key(a),
        a_role: "heading".into(),
        b: key(b),
        b_role: "body".into(),
        mode: Some("safe".into()),
        preset: Some("menu".into()),
        sample_text: Some("Phở bò".into()),
        collection_id: c,
        note: None,
    };
    let p1 = s.save_pair(&np(1, 2, Some(col))).unwrap();
    let p2 = s.save_pair(&np(3, 4, None)).unwrap();
    assert_eq!(s.pairs(None).unwrap().len(), 2);
    let in_col = s.pairs(Some(col)).unwrap();
    assert_eq!(in_col.len(), 1);
    assert_eq!((in_col[0].id, in_col[0].a, in_col[0].b_role.as_str()), (p1, key(1), "body"));
    assert_eq!(in_col[0].sample_text.as_deref(), Some("Phở bò"));
    s.delete_collection(col).unwrap();
    let all = s.pairs(None).unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|p| p.collection_id.is_none()));
    s.delete_pair(p2).unwrap();
    assert_eq!(s.pairs(None).unwrap().len(), 1);
    assert_eq!(s.delete_pair(999).unwrap_err().code(), "not_found");
    let mut bad = np(1, 2, None);
    bad.a_role = "title".into();
    assert!(s.save_pair(&bad).is_err());
}

#[test]
fn v3_to_v4_migrates_keeps_data_and_adds_overrides() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_v3_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute("INSERT INTO collections(name,created_at,updated_at) VALUES ('Giữ v3',1,1)", []).unwrap();
        c.execute(
            "INSERT INTO activation_state(hash,state,scope,method,active_path,updated_at) VALUES (?1,'active','persistent','fake','/x.ttf',1)",
            [&[7u8; 32][..]],
        )
        .unwrap();
    }
    let (mut s, rep) = Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    assert_eq!(s.collections().unwrap().len(), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.activation_state"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.sqlite_schema WHERE name = 'classification_overrides'"), 1);
    s.set_classification_override(&key(7), "serif", Some("didone")).unwrap();
    assert_eq!(s.classification_overrides().unwrap().len(), 1);
    assert!(rep.user_backup.unwrap().file_name().unwrap().to_string_lossy().starts_with("user.v3."));
}

#[test]
fn v4_to_v5_drops_display_overrides_keeps_the_rest() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_v4_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        for (n, class, sub) in [(1u8, "serif", Some("didone")), (2, "display", None), (3, "script", Some("formal"))] {
            c.execute(
                "INSERT INTO classification_overrides(hash,face_index,class,subclass,updated_at) VALUES (?1,0,?2,?3,9)",
                rusqlite::params![&[n; 32][..], class, sub],
            )
            .unwrap();
        }
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    let left: Vec<String> = s.classification_overrides().unwrap().into_iter().map(|o| o.class).collect();
    assert_eq!(left, vec!["serif".to_string(), "script".to_string()]);
    // CHECK mới không còn cho phép 'display'
    let mut s = s;
    // taxonomy v2: Display là class hợp lệ; class lạ vẫn bị CHECK từ chối
    assert!(s.set_classification_override(&key(9), "display", None).is_ok());
    assert!(s.set_classification_override(&key(9), "wiggly", None).is_err());
    assert_eq!(rep.dropped_display_overrides, 1);
    assert!(rep.user_backup.unwrap().file_name().unwrap().to_string_lossy().starts_with("user.v4."));
}
