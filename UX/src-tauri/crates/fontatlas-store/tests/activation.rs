mod common;
use common::*;
use fontatlas_store::{
    ActivationReason, ActivationRow, ActivationScope, ActivationStatus, DesiredState, FaceKey, FaceName,
    create_user_v1_for_test,
};

fn row(hash: [u8; 32], status: ActivationStatus) -> ActivationRow {
    ActivationRow {
        reasons: vec![],
        hash,
        state: DesiredState::Active,
        scope: ActivationScope::Session,
        method: "fake".into(),
        active_path: Some("/f.ttf".into()),
        status,
        conflict_ps: None,
        last_error: None,
        activated_at: None,
        verified_at: None,
        updated_at: 0,
        engine_version: 1,
    }
}

#[test]
fn user_v2_migrates_v1_rows() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_v1_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute(
            "INSERT INTO activation_state(hash,state,scope,method,active_path,updated_at) \
             VALUES (?1,'active','persistent','coretext','/x.ttf',1)",
            [&[7u8; 32][..]],
        )
        .unwrap();
    }
    let (s, rep) = fontatlas_store::Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), 2);
    assert!(rep.user_backup.is_some());
    let r = s.activation_row(&[7; 32]).unwrap().unwrap();
    assert_eq!(r.state, DesiredState::Active);
    assert_eq!(r.scope, ActivationScope::Persistent);
    assert_eq!(r.active_path.as_deref(), Some("/x.ttf"));
    assert_eq!(r.status, ActivationStatus::Unknown);
    assert_eq!(r.engine_version, 1);
    assert_eq!(r.reasons, vec![ActivationReason::Manual]);
}

#[test]
fn reasons_add_remove_and_check() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = [1u8; 32];
    s.put_activation(&row(h, ActivationStatus::Registered)).unwrap();
    assert!(s.add_activation_reason(&h, ActivationReason::Manual).unwrap());
    assert!(s.add_activation_reason(&h, ActivationReason::Collection(3)).unwrap());
    assert!(!s.add_activation_reason(&h, ActivationReason::Collection(3)).unwrap());
    assert_eq!(
        s.activation_row(&h).unwrap().unwrap().reasons,
        vec![ActivationReason::Manual, ActivationReason::Collection(3)]
    );
    assert_eq!(s.remove_activation_reason(&h, ActivationReason::Collection(3)).unwrap(), vec![ActivationReason::Manual]);
    assert!(
        s.conn()
            .execute("INSERT INTO u.activation_reasons VALUES (?1,'collection:x',1)", [&h[..]])
            .is_err()
    );
    assert!(
        s.conn()
            .execute("INSERT INTO u.activation_reasons VALUES (?1,'collection:',1)", [&h[..]])
            .is_err()
    );
    assert_eq!(s.add_activation_reason(&[9; 32], ActivationReason::Manual).unwrap_err().code(), "not_found");
    assert_eq!(s.hashes_with_reason(ActivationReason::Manual).unwrap(), vec![h]);
    assert_eq!(s.clear_activation_reasons(&h).unwrap(), 1);
    assert!(s.activation_row(&h).unwrap().unwrap().reasons.is_empty());
}

#[test]
fn reason_db_and_json_forms() {
    assert_eq!(ActivationReason::Manual.as_db(), "manual");
    assert_eq!(ActivationReason::Collection(12).as_db(), "collection:12");
    assert_eq!(ActivationReason::parse_db("collection:12"), Some(ActivationReason::Collection(12)));
    assert_eq!(ActivationReason::parse_db("manual"), Some(ActivationReason::Manual));
    assert_eq!(ActivationReason::parse_db("collection:x"), None);
    assert_eq!(serde_json::to_string(&ActivationReason::Manual).unwrap(), r#"{"kind":"manual"}"#);
    assert_eq!(
        serde_json::to_string(&ActivationReason::Collection(3)).unwrap(),
        r#"{"kind":"collection","collectionId":3}"#
    );
    assert!(ActivationReason::Manual < ActivationReason::Collection(1));
    assert!(ActivationReason::Collection(1) < ActivationReason::Collection(2));
}

#[test]
fn delete_activation_cascades_reasons() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = [2u8; 32];
    s.put_activation(&row(h, ActivationStatus::Registered)).unwrap();
    s.add_activation_reason(&h, ActivationReason::Manual).unwrap();
    assert!(s.delete_activation(&h).unwrap());
    assert!(!s.delete_activation(&h).unwrap());
    assert_eq!(count(&s, "SELECT count(*) FROM u.activation_reasons"), 0);
}

#[test]
fn prune_collection_reasons_drops_deleted_and_removed_items() {
    let d = tmp();
    let mut s = open_store(d.path());
    let (h, k) = ([3u8; 32], [4u8; 32]);
    let a = s.create_collection("A", None).unwrap();
    let b = s.create_collection("B", None).unwrap();
    let c = s.create_collection("C", None).unwrap();
    let fh = FaceKey { hash: h, face_index: 0 };
    let fk = FaceKey { hash: k, face_index: 0 };
    s.add_to_collection(a, &[fh]).unwrap();
    s.add_to_collection(b, &[fh]).unwrap();
    s.add_to_collection(c, &[fk]).unwrap();
    for (hash, coll) in [(h, a), (h, b), (k, c)] {
        if s.activation_row(&hash).unwrap().is_none() {
            s.put_activation(&row(hash, ActivationStatus::Registered)).unwrap();
        }
        s.add_activation_reason(&hash, ActivationReason::Collection(coll)).unwrap();
    }
    s.delete_collection(a).unwrap();
    s.remove_from_collection(b, &[fh]).unwrap();
    let pruned = s.prune_collection_reasons().unwrap();
    assert_eq!(pruned, vec![(h, vec![ActivationReason::Collection(a), ActivationReason::Collection(b)])]);
    assert!(s.activation_row(&h).unwrap().unwrap().reasons.is_empty());
    assert_eq!(s.activation_row(&k).unwrap().unwrap().reasons, vec![ActivationReason::Collection(c)]);
}

#[test]
fn put_activation_roundtrip_and_upsert() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = [5u8; 32];
    s.put_activation(&row(h, ActivationStatus::Unknown)).unwrap();
    s.add_activation_reason(&h, ActivationReason::Manual).unwrap();
    let mut r2 = row(h, ActivationStatus::Registered);
    r2.conflict_ps = Some("X-Regular".into());
    s.put_activation(&r2).unwrap();
    let rows = s.activation_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, ActivationStatus::Registered);
    assert_eq!(rows[0].conflict_ps.as_deref(), Some("X-Regular"));
    assert!(rows[0].updated_at > 0);
    assert_eq!(rows[0].reasons, vec![ActivationReason::Manual], "upsert không được xoá lý do");
}

#[test]
fn with_tx_commits_row_and_reason_together_or_not_at_all() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = [6u8; 32];
    let err = s
        .with_tx(|tx| {
            fontatlas_store::put_activation_tx(tx, &row(h, ActivationStatus::Unknown))?;
            fontatlas_store::add_activation_reason_tx(tx, &h, ActivationReason::Manual)?;
            Err::<(), _>(fontatlas_store::StoreError::NotFound("giả lỗi".into()))
        })
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
    assert!(s.activation_row(&h).unwrap().is_none());
    s.with_tx(|tx| {
        fontatlas_store::put_activation_tx(tx, &row(h, ActivationStatus::Unknown))?;
        fontatlas_store::add_activation_reason_tx(tx, &h, ActivationReason::Manual)?;
        let left = fontatlas_store::remove_activation_reason_tx(tx, &h, ActivationReason::Collection(1))?;
        assert_eq!(left, vec![ActivationReason::Manual]);
        Ok(())
    })
    .unwrap();
    assert_eq!(s.activation_row(&h).unwrap().unwrap().reasons, vec![ActivationReason::Manual]);
}

#[test]
fn activation_check_constraints() {
    let d = tmp();
    let s = open_store(d.path());
    assert!(
        s.conn()
            .execute(
                "INSERT INTO u.activation_state(hash,state,scope,method,status,updated_at,engine_version) \
                 VALUES (zeroblob(32),'on','session','x','unknown',1,1)",
                [],
            )
            .is_err()
    );
    assert!(
        s.conn()
            .execute(
                "INSERT INTO u.activation_state(hash,state,scope,method,status,updated_at,engine_version) \
                 VALUES (zeroblob(32),'active','session','x','bogus',1,1)",
                [],
            )
            .is_err()
    );
}

#[test]
fn file_target_lists_locations_and_ps_names() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("libre_baskerville.ttf");
    s.ingest(vec![item(1, "/fonts/libre_baskerville.ttf", &r)], 1).unwrap();
    let t = s.file_target(&hash_of(&r)).unwrap().unwrap();
    assert_eq!(t.hash, hash_of(&r));
    assert_eq!(t.locations.len(), 1);
    assert_eq!(t.locations[0].path, std::path::PathBuf::from("/fonts/libre_baskerville.ttf"));
    assert!(!t.locations[0].missing);
    assert_eq!(
        t.faces,
        vec![FaceName { face_index: 0, postscript_name: Some("LibreBaskerville-Regular".into()) }]
    );
}

#[test]
fn file_target_unknown_hash_is_none() {
    let d = tmp();
    let s = open_store(d.path());
    assert!(s.file_target(&[0xAB; 32]).unwrap().is_none());
}

#[test]
fn collection_faces_returns_items_and_not_found() {
    let d = tmp();
    let mut s = open_store(d.path());
    let c = s.create_collection("Dự án", None).unwrap();
    let keys = [FaceKey { hash: [1; 32], face_index: 0 }, FaceKey { hash: [1; 32], face_index: 1 }];
    s.add_to_collection(c, &keys).unwrap();
    let got = s.collection_faces(c).unwrap();
    assert_eq!(got.len(), 2);
    assert!(got.contains(&keys[0]) && got.contains(&keys[1]));
    assert_eq!(s.collection_faces(999).unwrap_err().code(), "not_found");
}
