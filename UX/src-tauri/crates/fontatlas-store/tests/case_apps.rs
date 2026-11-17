mod common;
use common::*;
use fontatlas_store::{ActivationReason, ActivationRow, ActivationScope, ActivationStatus, CaseApp, DesiredState, Store, StoreError, USER_FILE, create_user_pre_case_apps_for_test, create_user_pre_case_apps_by_case_for_test};

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
    let c = s.create_case("Brand Kit A", None, None).unwrap();
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
    let c = s.create_case("X", None, None).unwrap();
    assert!(matches!(s.set_case_apps(c, &[app("a b", "")]), Err(StoreError::Invalid(_))));
    assert!(matches!(s.set_case_apps(c, &[app("ab", "")]), Err(StoreError::Invalid(_))));
    assert!(matches!(s.set_case_apps(9999, &[app("com.a.b", "")]), Err(StoreError::NotFound(_))));
    assert!(s.case_apps(c).unwrap().is_empty(), "lỗi không để lại nửa chừng");
}

#[test]
fn bindings_group_by_case_and_cascade_on_delete() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.create_case("A", None, None).unwrap();
    let b = s.create_case("B", None, None).unwrap();
    s.set_case_apps(a, &[app("com.x.one", ""), app("com.x.two", "")]).unwrap();
    s.set_case_apps(b, &[app("com.x.one", "")]).unwrap();
    let bind = s.case_bindings().unwrap();
    assert_eq!(bind.len(), 2);
    assert_eq!(bind[0].case_id, a);
    assert_eq!(bind[0].bundles, ["com.x.one", "com.x.two"]);
    s.delete_case(a).unwrap();
    let bind = s.case_bindings().unwrap();
    assert_eq!(bind.len(), 1, "Case vào thùng rác thì hết bị đồng bộ app set");
    assert_eq!(bind[0].case_id, b);
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
    assert_eq!(s.case_app_reason_cases().unwrap(), vec![7]);
    assert_eq!(s.hashes_with_reason(ActivationReason::CaseApp(7)).unwrap(), vec![h]);
    let left = s.remove_activation_reason(&h, ActivationReason::CaseApp(7)).unwrap();
    assert_eq!(left, vec![ActivationReason::Manual], "gỡ lý do app không đụng lý do bật tay");
    assert!(s.case_app_reason_cases().unwrap().is_empty());
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

/// L2 (beta.2): app set thuộc về Case. Một id chỉ là collection (không có Case) KHÔNG nhận app set, và Case không cần có collection nào.
#[test]
fn app_set_belongs_to_the_case_not_to_a_collection_with_the_same_id() {
    let d = tmp();
    let mut s = open_store(d.path());
    // Case #1 và collection #1 cùng số nhưng là hai thực thể khác nhau
    let case = s.create_case("Case", None, None).unwrap();
    let coll = s.create_collection("Collection", None).unwrap();
    assert_eq!(case, coll, "điều kiện của test: trùng số");
    s.set_case_apps(case, &[app("com.x.one", "")]).unwrap();
    // collection thứ 2 không có Case cùng số: không được nhận app set (trước đây đây là đường chạy được còn Case thì báo "không tìm thấy")
    let only_coll = s.create_collection("Only collection", None).unwrap();
    assert_ne!(only_coll, case);
    assert!(matches!(s.set_case_apps(only_coll, &[app("com.x.two", "")]), Err(StoreError::NotFound(_))));
    // Case mới không cần collection nào
    let case2 = s.create_case("Case 2", None, None).unwrap();
    s.set_case_apps(case2, &[app("com.x.three", "")]).unwrap();
    // xoá collection trùng số không đụng tới app set của Case
    s.delete_collection(coll).unwrap();
    assert_eq!(s.case_apps(case).unwrap().len(), 1);
    // Case trong thùng rác: không còn là binding và không nhận ghi thêm; khôi phục thì quay lại
    s.delete_case(case).unwrap();
    assert!(s.case_bindings().unwrap().iter().all(|b| b.case_id != case));
    assert!(matches!(s.set_case_apps(case, &[app("com.x.one", "")]), Err(StoreError::NotFound(_))));
    s.restore_case(case).unwrap();
    assert!(s.case_bindings().unwrap().iter().any(|b| b.case_id == case));
}

#[test]
fn migration_to_case_keyed_app_sets_keeps_rows_whose_case_exists_and_drops_orphans() {
    let d = tmp();
    let user = d.path().join(USER_FILE);
    create_user_pre_case_apps_by_case_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute_batch("PRAGMA foreign_keys=OFF;").unwrap(); // dữ liệu cũ: hàng khoá theo collection_id mà collection đó không còn
        // Case 1 và 2 có thật; hàng cho id 9 là mồ côi (không có Case)
        for _ in 0..2 {
            c.execute("INSERT INTO cases(name,brief,settings,created_at,updated_at,opened_at) VALUES ('c',NULL,'{}',1,1,1)", []).unwrap();
        }
        // collection trùng id 1 và 2: nâng cấp không được đụng tới chúng
        for _ in 0..2 {
            c.execute("INSERT INTO collections(name,created_at,updated_at) VALUES ('Khác',1,1)", []).unwrap();
        }
        for (id, b) in [(1, "com.x.one"), (1, "com.x.two"), (2, "com.x.one"), (9, "com.x.orphan")] {
            c.execute("INSERT INTO case_apps(collection_id,bundle_id,app_name,added_at) VALUES (?1,?2,'N',5)", rusqlite::params![id, b]).unwrap();
        }
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.user_backup.is_some());
    assert_eq!(s.case_apps(1).unwrap().len(), 2);
    assert_eq!(s.case_apps(2).unwrap().len(), 1);
    assert!(s.case_apps(9).unwrap().is_empty());
    assert_eq!(s.case_apps(1).unwrap()[0].app_name, "N");
    // collection cùng id vẫn nguyên và không bị liên kết với app set
    let c = rusqlite::Connection::open(&user).unwrap();
    let n: i64 = c.query_row("SELECT COUNT(*) FROM collections WHERE name='Khác'", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 2);
}

/// v19: lý do `case:<id>` (bật cả Case). Chuỗi DB khép kín, CHECK chặn dạng sai, xếp sau `manual`.
#[test]
fn case_reason_roundtrips_is_checked_and_sorts_after_manual() {
    let r = ActivationReason::Case(42);
    assert_eq!(r.as_db(), "case:42");
    assert_eq!(ActivationReason::parse_db("case:42"), Some(r));
    for bad in ["case:", "case:x", "case:4x", "cases:1"] {
        assert_eq!(ActivationReason::parse_db(bad), None, "{bad}");
    }
    assert!(ActivationReason::Manual < ActivationReason::Case(1) && ActivationReason::Case(1) < ActivationReason::Collection(1));
    let d = tmp();
    let s = open_store(d.path());
    let hash = vec![4u8; 32];
    s.conn().execute("INSERT INTO u.activation_state(hash,state,scope,method,status,updated_at) VALUES (?1,'active','session','fake','registered',1)", [&hash]).unwrap();
    s.conn().execute("INSERT INTO u.activation_reasons(hash,reason,added_at) VALUES (?1,'case:7',1)", [&hash]).unwrap();
    for bad in ["case:", "case:x", "case:1x", "cases:1"] {
        assert!(s.conn().execute("INSERT INTO u.activation_reasons(hash,reason,added_at) VALUES (?1,?2,1)", rusqlite::params![&hash, bad]).is_err(), "{bad}");
    }
}

#[test]
fn migration_to_case_reason_keeps_every_reason_and_backs_up() {
    use fontatlas_store::create_user_pre_case_reason_for_test;
    let d = tmp();
    let user = d.path().join(USER_FILE);
    create_user_pre_case_reason_for_test(&user);
    let (h1, h2) = (vec![1u8; 32], vec![2u8; 32]);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        for h in [&h1, &h2] {
            c.execute("INSERT INTO activation_state(hash,state,scope,method,status,updated_at) VALUES (?1,'active','session','fake','registered',1)", [h]).unwrap();
        }
        // Case từng bật trước v19 chỉ có `manual`: giữ nguyên, không đoán ngược thành `case:`
        for (h, r) in [(&h1, "manual"), (&h1, "collection:3"), (&h2, "manual"), (&h2, "caseapp:2"), (&h2, "auto:illustrator")] {
            c.execute("INSERT INTO activation_reasons(hash,reason,added_at) VALUES (?1,?2,5)", rusqlite::params![h, r]).unwrap();
        }
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.user_backup.is_some());
    let name = rep.user_backup.unwrap().file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("user.v18."), "{name}");
    let n: i64 = s.conn().query_row("SELECT COUNT(*) FROM u.activation_reasons", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 5);
    let none: i64 = s.conn().query_row("SELECT COUNT(*) FROM u.activation_reasons WHERE reason LIKE 'case:%'", [], |r| r.get(0)).unwrap();
    assert_eq!(none, 0);
    s.conn().execute("INSERT INTO u.activation_reasons(hash,reason,added_at) VALUES (?1,'case:9',6)", [&h1]).unwrap();
}
