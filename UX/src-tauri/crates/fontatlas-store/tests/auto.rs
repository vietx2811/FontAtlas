mod common;
use common::*;
use fontatlas_store::{
    ActivationReason, ActivationRow, ActivationScope, ActivationStatus, AutoApp, AutoLogEntry, DesiredState, Store,
    USER_FILE, create_user_v7_for_test, AUTO_LOG_KEEP,
};

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
fn find_by_postscript_and_family_style() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("libre_baskerville.ttf");
    s.ingest(vec![item(1, "/fonts/lb.ttf", &r)], 1).unwrap();
    let h = hash_of(&r);

    let ps = s.faces_by_postscript("LibreBaskerville-Regular").unwrap();
    assert_eq!(ps.len(), 1);
    assert_eq!(ps[0].hash, h);
    assert!(s.faces_by_postscript("libreBaskerville-regular").unwrap().is_empty(), "PS name phân biệt hoa thường");
    assert!(s.faces_by_postscript("Nope-Regular").unwrap().is_empty());

    assert_eq!(s.faces_by_family_style("Libre Baskerville", "Regular").unwrap().len(), 1);
    assert_eq!(s.faces_by_family_style("libre  baskerville", "").unwrap().len(), 1, "style rỗng = Regular");
    assert!(s.faces_by_family_style("Libre Baskerville", "Bold").unwrap().is_empty(), "không đoán gần đúng");
    assert!(s.faces_by_family_style("Libre", "Regular").unwrap().is_empty(), "family phải khớp đủ");
}

#[test]
fn missing_files_are_not_found() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("libre_baskerville.ttf");
    s.ingest(vec![item(1, "/fonts/lb.ttf", &r)], 1).unwrap();
    s.conn().execute("UPDATE main.file_locations SET missing = 1", []).unwrap();
    assert!(s.faces_by_postscript("LibreBaskerville-Regular").unwrap().is_empty());
    assert!(s.faces_by_family_style("Libre Baskerville", "Regular").unwrap().is_empty());
}

#[test]
fn auto_reason_roundtrip_and_sort() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = [7u8; 32];
    s.put_activation(&row(h)).unwrap();
    assert!(s.add_activation_reason(&h, ActivationReason::Auto(AutoApp::Photoshop)).unwrap());
    assert!(s.add_activation_reason(&h, ActivationReason::Manual).unwrap());
    assert!(!s.add_activation_reason(&h, ActivationReason::Auto(AutoApp::Photoshop)).unwrap());
    let got = s.activation_row(&h).unwrap().unwrap();
    assert_eq!(got.reasons, vec![ActivationReason::Manual, ActivationReason::Auto(AutoApp::Photoshop)]);
    assert_eq!(s.hashes_with_reason(ActivationReason::Auto(AutoApp::Photoshop)).unwrap(), vec![h]);
    let left = s.remove_activation_reason(&h, ActivationReason::Manual).unwrap();
    assert_eq!(left, vec![ActivationReason::Auto(AutoApp::Photoshop)]);
    let j = serde_json::to_string(&ActivationReason::Auto(AutoApp::AfterEffects)).unwrap();
    assert_eq!(j, r#"{"kind":"auto","app":"aftereffects"}"#);
}

#[test]
fn db_rejects_bad_auto_reason() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = [8u8; 32];
    s.put_activation(&row(h)).unwrap();
    for bad in ["auto:", "auto:Photoshop", "auto:photo shop", "auto:1", "auto:a;b", "auto"] {
        let r = s.conn().execute("INSERT INTO u.activation_reasons(hash,reason,added_at) VALUES (?1,?2,1)", rusqlite::params![&h[..], bad]);
        assert!(r.is_err(), "{bad} phải bị CHECK chặn");
    }
}

#[test]
fn log_keeps_newest_and_orders_desc() {
    let d = tmp();
    let mut s = open_store(d.path());
    for i in 0..(AUTO_LOG_KEEP + 25) {
        s.log_auto(&AutoLogEntry {
            app: AutoApp::Illustrator,
            doc: Some(format!("doc{i}")),
            requested: format!("Font{i}"),
            outcome: "activated".into(),
            hash: if i % 2 == 0 { Some([1u8; 32]) } else { None },
        })
        .unwrap();
    }
    let n = count(&s, "SELECT count(*) FROM u.auto_activation_log");
    assert_eq!(n, AUTO_LOG_KEEP);
    let top = s.recent_auto_log(3).unwrap();
    assert_eq!(top.len(), 3);
    assert_eq!(top[0].requested, format!("Font{}", AUTO_LOG_KEEP + 24));
    assert!(top[0].id > top[1].id);
    assert_eq!(top[0].app, "illustrator");
}

#[test]
fn user_v8_migrates_v7_keeping_reasons_and_backs_up() {
    let d = tmp();
    let user = d.path().join(USER_FILE);
    create_user_v7_for_test(&user);
    let hash = vec![9u8; 32];
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute(
            "INSERT INTO activation_state(hash,state,scope,method,status,updated_at) VALUES (?1,'active','session','fake','registered',1)",
            [&hash],
        )
        .unwrap();
        c.execute("INSERT INTO activation_reasons(hash,reason,added_at) VALUES (?1,'manual',5)", [&hash]).unwrap();
        c.execute("INSERT INTO activation_reasons(hash,reason,added_at) VALUES (?1,'collection:3',6)", [&hash]).unwrap();
    }
    let (mut s, rep) = Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    let b = rep.user_backup.expect("có backup trước khi migrate");
    assert!(b.file_name().unwrap().to_string_lossy().starts_with("user.v7."));
    let bc = rusqlite::Connection::open(b).unwrap();
    assert_eq!(bc.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0)).unwrap(), 7);
    assert_eq!(bc.query_row("SELECT count(*) FROM activation_reasons", [], |r| r.get::<_, i64>(0)).unwrap(), 2);

    let mut h = [0u8; 32];
    h.copy_from_slice(&hash);
    let got = s.activation_row(&h).unwrap().unwrap();
    assert_eq!(got.reasons, vec![ActivationReason::Manual, ActivationReason::Collection(3)]);
    assert_eq!(count(&s, "SELECT count(*) FROM u.sqlite_schema WHERE type='index' AND name='activation_reasons_reason'"), 1);
    // lý do mới dùng được, FK cascade vẫn chạy
    assert!(s.add_activation_reason(&h, ActivationReason::Auto(AutoApp::Indesign)).unwrap());
    assert!(s.delete_activation(&h).unwrap());
    assert_eq!(count(&s, "SELECT count(*) FROM u.activation_reasons"), 0);
}

#[test]
fn log_skips_identical_repeat_within_window() {
    let d = tmp();
    let mut s = open_store(d.path());
    let e = |outcome: &str| AutoLogEntry {
        app: AutoApp::Photoshop,
        doc: None,
        requested: "Foo-Regular".into(),
        outcome: outcome.into(),
        hash: None,
    };
    assert!(s.log_auto(&e("missing")).unwrap());
    assert!(!s.log_auto(&e("missing")).unwrap(), "y hệt trong cửa sổ: bỏ qua");
    assert!(s.log_auto(&e("activated")).unwrap(), "kết quả khác: ghi");
    let other_app = AutoLogEntry { app: AutoApp::Illustrator, ..e("missing") };
    assert!(s.log_auto(&other_app).unwrap(), "app khác: ghi");
    assert_eq!(s.recent_auto_log(10).unwrap().len(), 3);
}
