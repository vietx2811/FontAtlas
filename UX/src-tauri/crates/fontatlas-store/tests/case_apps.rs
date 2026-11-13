mod common;
use common::*;
use fontatlas_store::{ActivationReason, ActivationRow, ActivationScope, ActivationStatus, CaseApp, DesiredState, Store, StoreError, USER_FILE, create_user_pre_case_apps_for_test};

fn app(b: &str, n: &str) -> CaseApp {
    CaseApp { bundle_id: b.into(), app_name: n.into() }
}

fn row(hash: [u8; 32]) -> ActivationRow {
    ActivationRow {
        reasons: vec![],
        hash,
        state: DesiredState::Active,
        scope: ActivationScope::Session,
        method: "fake".into(),
        active_path: Some("/f.ttf".into()),
        status: ActivationStatus::Registered,
        conflict_ps: None,
        last_error: None,
        activated_at: None,
        verified_at: None,
        updated_at: 0,
        engine_version: 1,
    }
}

#[test]
fn set_get_replace_keeps_order_and_dedupes() {
    let d = tmp();
    let mut s = open_store(d.path());
    let c = s.create_collection("Brand Kit A", None).unwrap();
    assert!(s.case_apps(c).unwrap().is_empty());
    s.set_case_apps(c, &[app("com.adobe.Illustrator", "Adobe Illustrator"), app("com.adobe.InDesign", "Adobe InDesign"), app("com.adobe.Illustrator", "dup")]).unwrap();
    let got = s.case_apps(c).unwrap();
    assert_eq!(got.iter().map(|a| a.bundle_id.as_str()).collect::<Vec<_>>(), ["com.adobe.Illustrator", "com.adobe.InDesign"]);
    assert_eq!(got[0].app_name, "Adobe Illustrator", "bản trùng không ghi đè bản đầu");
    s.set_case_apps(c, &[app("com.apple.TextEdit", "TextEdit")]).unwrap();
    assert_eq!(s.case_apps(c).unwrap().len(), 1, "set thay toàn bộ");
    s.set_case_apps(c, &[]).unwrap();
    assert!(s.case_apps(c).unwrap().is_empty());
}

#[test]
fn rejects_bad_bundle_and_unknown_case() {
    let d = tmp();
    let mut s = open_store(d.path());
    let c = s.create_collection("X", None).unwrap();
    assert!(matches!(s.set_case_apps(c, &[app("a b", "")]), Err(StoreError::Invalid(_))));
    assert!(matches!(s.set_case_apps(c, &[app("ab", "")]), Err(StoreError::Invalid(_))));
    assert!(matches!(s.set_case_apps(9999, &[app("com.a.b", "")]), Err(StoreError::NotFound(_))));
    assert!(s.case_apps(c).unwrap().is_empty(), "lỗi không để lại nửa chừng");
}

#[test]
fn bindings_group_by_case_and_cascade_on_delete() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.create_collection("A", None).unwrap();
    let b = s.create_collection("B", None).unwrap();
    s.set_case_apps(a, &[app("com.x.one", ""), app("com.x.two", "")]).unwrap();
    s.set_case_apps(b, &[app("com.x.one", "")]).unwrap();
    let bind = s.case_bindings().unwrap();
    assert_eq!(bind.len(), 2);
    assert_eq!(bind[0].collection_id, a);
    assert_eq!(bind[0].bundles, ["com.x.one", "com.x.two"]);
    s.delete_collection(a).unwrap();
    let bind = s.case_bindings().unwrap();
    assert_eq!(bind.len(), 1, "xoá Case xoá luôn các liên kết app");
    assert_eq!(bind[0].collection_id, b);
}

#[test]
fn caseapp_reason_is_storable_sorted_and_listed() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("libre_baskerville.ttf");
    s.ingest(vec![item(1, "/fonts/lb.ttf", &r)], 1).unwrap();
    let h = hash_of(&r);
    s.put_activation(&row(h)).unwrap();
    assert!(s.add_activation_reason(&h, ActivationReason::Manual).unwrap());
    assert!(s.add_activation_reason(&h, ActivationReason::CaseApp(7)).unwrap());
    assert!(!s.add_activation_reason(&h, ActivationReason::CaseApp(7)).unwrap(), "lần hai không thêm");
    assert_eq!(s.activation_row(&h).unwrap().unwrap().reasons, vec![ActivationReason::Manual, ActivationReason::CaseApp(7)]);
    assert_eq!(s.case_app_reason_collections().unwrap(), vec![7]);
    assert_eq!(s.hashes_with_reason(ActivationReason::CaseApp(7)).unwrap(), vec![h]);
    let left = s.remove_activation_reason(&h, ActivationReason::CaseApp(7)).unwrap();
    assert_eq!(left, vec![ActivationReason::Manual], "gỡ lý do app không đụng lý do bật tay");
    assert!(s.case_app_reason_collections().unwrap().is_empty());
}

#[test]
fn migration_keeps_existing_reasons_and_backs_up() {
    let d = tmp();
    let user = d.path().join(USER_FILE);
    create_user_pre_case_apps_for_test(&user);
    let hash = vec![9u8; 32];
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute("INSERT INTO activation_state(hash,state,scope,method,status,updated_at) VALUES (?1,'active','session','fake','registered',1)", [&hash]).unwrap();
        c.execute("INSERT INTO activation_reasons(hash,reason,added_at) VALUES (?1,'manual',5)", [&hash]).unwrap();
        c.execute("INSERT INTO activation_reasons(hash,reason,added_at) VALUES (?1,'collection:3',6)", [&hash]).unwrap();
        c.execute("INSERT INTO activation_reasons(hash,reason,added_at) VALUES (?1,'auto:illustrator',7)", [&hash]).unwrap();
    }
    let (mut s, rep) = Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    assert!(rep.user_backup.is_some(), "có backup trước khi migrate");
    let mut h = [0u8; 32];
    h.copy_from_slice(&hash);
    assert_eq!(s.activation_row(&h).unwrap().unwrap().reasons.len(), 3);
    assert!(s.add_activation_reason(&h, ActivationReason::CaseApp(2)).unwrap());
    assert_eq!(count(&s, "SELECT count(*) FROM u.sqlite_schema WHERE type='index' AND name='activation_reasons_reason'"), 1);
    assert!(s.delete_activation(&h).unwrap());
    assert_eq!(count(&s, "SELECT count(*) FROM u.activation_reasons"), 0, "FK cascade vẫn chạy");
}

#[test]
fn check_rejects_malformed_caseapp_reason() {
    let d = tmp();
    let s = open_store(d.path());
    let hash = vec![3u8; 32];
    s.conn().execute("INSERT INTO u.activation_state(hash,state,scope,method,status,updated_at) VALUES (?1,'active','session','fake','registered',1)", [&hash]).unwrap();
    for bad in ["caseapp:", "caseapp:x", "caseapp:1x", "caseapps:1"] {
        assert!(s.conn().execute("INSERT INTO u.activation_reasons(hash,reason,added_at) VALUES (?1,?2,1)", rusqlite::params![&hash, bad]).is_err(), "{bad}");
    }
}
