mod common;
use common::*;
use fontatlas_store::{CasePatch, ComboPatch, FaceKey, MAX_CASE_FACES, MAX_COMBOS, NewCombo, Store, create_user_v5_for_test};

fn key(n: u16) -> FaceKey {
    let mut h = [0u8; 32];
    h[..2].copy_from_slice(&n.to_le_bytes());
    FaceKey { hash: h, face_index: 0 }
}

fn open() -> (tempfile::TempDir, Store) {
    let d = tmp();
    let s = open_store(d.path());
    (d, s)
}

#[test]
fn migration_v5_to_v6_keeps_data_and_backs_up() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_v5_for_test(&user);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute("INSERT INTO collections(name,created_at,updated_at) VALUES ('Giữ',1,1)", []).unwrap();
        c.execute(
            "INSERT INTO pairs(a_hash,a_face,a_role,b_hash,b_face,b_role,created_at) VALUES (zeroblob(32),0,'heading',zeroblob(32),1,'body',1)",
            [],
        )
        .unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert_eq!(count(&s, "PRAGMA u.user_version"), 6);
    assert_eq!(s.collections().unwrap().len(), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM u.pairs"), 1);
    for t in ["cases", "case_faces", "combos", "combo_slots"] {
        assert_eq!(count(&s, &format!("SELECT count(*) FROM u.sqlite_schema WHERE type='table' AND name='{t}'")), 1, "{t}");
    }
    let b = rep.user_backup.expect("có backup");
    assert!(b.file_name().unwrap().to_string_lossy().starts_with("user.v5."));
    let bc = rusqlite::Connection::open(b).unwrap();
    assert_eq!(bc.query_row("SELECT count(*) FROM collections", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    assert_eq!(bc.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0)).unwrap(), 5);
}

#[test]
fn case_crud_validation_soft_delete_and_purge() {
    let (_d, mut s) = open();
    assert!(s.create_case("  ", None, None).is_err());
    assert!(s.create_case("A", Some("not json"), None).is_err());
    let a = s.create_case("Poster bia", Some(r#"{"uses":["poster"]}"#), Some(r#"{"vi":true}"#)).unwrap();
    let b = s.create_case("Menu", None, None).unwrap();
    assert_eq!(s.latest_case().unwrap(), Some(b));
    s.touch_case(a).unwrap();
    assert_eq!(s.latest_case().unwrap(), Some(a));
    let c = s.update_case(a, &CasePatch { name: Some("Poster".into()), view_state: Some(r#"{"mode":"compare"}"#.into()), ..Default::default() }).unwrap();
    assert_eq!((c.name.as_str(), c.view_state.as_str()), ("Poster", r#"{"mode":"compare"}"#));
    assert_eq!(c.brief.as_deref(), Some(r#"{"uses":["poster"]}"#), "không đụng trường không nằm trong patch");
    let c = s.update_case(a, &CasePatch { brief: Some(None), ..Default::default() }).unwrap();
    assert!(c.brief.is_none());
    assert!(s.update_case(a, &CasePatch { settings: Some("{".into()), ..Default::default() }).is_err());
    assert!(s.case(999).is_err());

    s.delete_case(a).unwrap();
    assert!(s.case(a).is_err(), "đã xoá mềm");
    assert_eq!(s.cases(false).unwrap().len(), 1);
    assert_eq!(s.cases(true).unwrap().len(), 2);
    assert_eq!(s.latest_case().unwrap(), Some(b));
    s.restore_case(a).unwrap();
    assert!(s.case(a).is_ok());
    // purge: chỉ Case xoá quá 30 ngày
    s.delete_case(a).unwrap();
    assert_eq!(s.purge_cases(fontatlas_store_now()).unwrap(), 0);
    assert_eq!(s.purge_cases(fontatlas_store_now() + 31 * 24 * 3600 * 1000).unwrap(), 1);
    assert_eq!(s.cases(true).unwrap().len(), 1);
}

fn fontatlas_store_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64
}

#[test]
fn faces_in_case_dedupe_limit_and_remove_cascades_slots() {
    let (_d, mut s) = open();
    let c = s.create_case("C", None, None).unwrap();
    assert_eq!(s.add_to_case(c, &[key(1), key(2), key(1)], "added").unwrap(), (2, 1));
    assert_eq!(s.add_to_case(c, &[key(2), key(3)], "peek").unwrap(), (1, 1));
    assert!(s.add_to_case(c, &[key(9)], "weird").is_err());
    let faces = s.case_faces(c).unwrap();
    assert_eq!(faces.len(), 3);
    assert_eq!(faces.iter().find(|f| f.key == key(2)).unwrap().origin, "added", "giữ origin đầu tiên");

    let m = s.create_combo(c, None, None).unwrap();
    s.set_combo_slot(m, "heading", Some(&key(1)), None, None).unwrap();
    s.set_combo_slot(m, "body", Some(&key(2)), None, None).unwrap();
    assert_eq!(s.remove_from_case(c, &[key(1)]).unwrap(), 1);
    let combo = s.combo(m).unwrap();
    assert_eq!(combo.slots.len(), 1, "vai dùng font bị bỏ cũng bị gỡ");
    assert_eq!(combo.slots[0].role, "body");

    // giới hạn
    let many: Vec<FaceKey> = (100..100 + MAX_CASE_FACES as u16 + 1).map(key).collect();
    assert!(s.add_to_case(c, &many, "added").is_err());
    assert_eq!(s.case_faces(c).unwrap().len(), 2, "lỗi quá giới hạn không thêm gì");
}

#[test]
fn combos_main_order_duplicate_and_slots() {
    let (_d, mut s) = open();
    let c = s.create_case("C", None, None).unwrap();
    let c1 = s.create_combo(c, None, None).unwrap();
    let c2 = s.create_combo(c, Some("Lạnh"), None).unwrap();
    let c3 = s.create_combo(c, None, None).unwrap();
    let all = s.combos(c).unwrap();
    assert_eq!(all.iter().map(|x| (x.name.as_str(), x.idx, x.is_main)).collect::<Vec<_>>(), vec![("Combo 1", 1, true), ("Lạnh", 2, false), ("Combo 3", 3, false)]);

    assert!(s.set_combo_slot(c1, "nope", Some(&key(1)), None, None).is_err());
    assert!(s.set_combo_slot(c1, "body", Some(&key(1)), None, Some("{")).is_err());
    s.set_combo_slot(c1, "heading", Some(&key(1)), Some("neo"), Some(r#"{"weight":700}"#)).unwrap();
    let r = s.set_combo_slot(c1, "body", Some(&key(2)), None, None).unwrap();
    assert_eq!(r.slots.iter().map(|x| x.role.as_str()).collect::<Vec<_>>(), vec!["heading", "body"], "theo thứ tự vai");
    assert_eq!(s.case_faces(c).unwrap().len(), 2, "đặt vào vai tự thêm font vào Case");
    // đổi font giữ ghi chú/kiểu cũ nếu không truyền
    let r = s.set_combo_slot(c1, "heading", Some(&key(5)), None, None).unwrap();
    let h = r.slots.iter().find(|x| x.role == "heading").unwrap();
    assert_eq!((h.note.as_deref(), h.style.as_str(), h.key), (Some("neo"), r#"{"weight":700}"#, key(5)));
    let r = s.set_combo_slot(c1, "body", None, None, None).unwrap();
    assert_eq!(r.slots.len(), 1);

    // nhân bản
    let dup = s.create_combo(c, Some("Bản sao"), Some(c1)).unwrap();
    assert_eq!(s.combo(dup).unwrap().slots, s.combo(c1).unwrap().slots);
    assert!(!s.combo(dup).unwrap().is_main);

    // đặt chính: loại trừ nhau
    let r = s.update_combo(c2, &ComboPatch { is_main: Some(true), ..Default::default() }).unwrap();
    assert!(r.is_main);
    assert!(!s.combo(c1).unwrap().is_main);
    // bỏ cờ chính của combo duy nhất chính → tự đặt lại combo đầu
    s.update_combo(c2, &ComboPatch { is_main: Some(false), ..Default::default() }).unwrap();
    assert_eq!(s.combos(c).unwrap().iter().filter(|x| x.is_main).count(), 1);
    assert!(s.update_combo(c2, &ComboPatch { name: Some(" ".into()), ..Default::default() }).is_err());

    // sắp xếp lại
    assert!(s.reorder_combos(c, &[c1, c2]).is_err(), "thiếu combo");
    assert!(s.reorder_combos(c, &[c1, c2, c3, c3]).is_err());
    s.reorder_combos(c, &[dup, c3, c2, c1]).unwrap();
    assert_eq!(s.combos(c).unwrap().iter().map(|x| x.id).collect::<Vec<_>>(), vec![dup, c3, c2, c1]);
    assert_eq!(s.combos(c).unwrap().iter().map(|x| x.idx).collect::<Vec<_>>(), vec![1, 2, 3, 4]);

    // xoá combo chính → combo đầu thành chính, idx liên tục
    let main = s.combos(c).unwrap().into_iter().find(|x| x.is_main).unwrap().id;
    s.delete_combo(main).unwrap();
    let left = s.combos(c).unwrap();
    assert_eq!(left.len(), 3);
    assert_eq!(left.iter().map(|x| x.idx).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(left.iter().filter(|x| x.is_main).count(), 1);
    assert!(s.combo(main).is_err());
}

#[test]
fn create_combos_is_atomic_and_limits_apply() {
    let (_d, mut s) = open();
    let c = s.create_case("C", None, None).unwrap();
    let good = NewCombo { name: Some("OK".into()), slots: vec![("heading".into(), key(1)), ("body".into(), key(2))] };
    let bad = NewCombo { name: None, slots: vec![("lạ".into(), key(3))] };
    assert!(s.create_combos(c, &[good.clone(), bad]).is_err());
    assert!(s.combos(c).unwrap().is_empty(), "Combo thứ hai lỗi thì Combo thứ nhất cũng không được tạo");
    assert!(s.case_faces(c).unwrap().is_empty());
    let ids = s.create_combos(c, &[good.clone(), NewCombo { name: None, slots: vec![("heading".into(), key(2))] }]).unwrap();
    assert_eq!(ids.len(), 2);
    let all = s.combos(c).unwrap();
    assert!(all[0].is_main && !all[1].is_main);
    assert_eq!(all[0].slots.len(), 2);
    assert_eq!(s.case_faces(c).unwrap().len(), 2);

    let many: Vec<NewCombo> = (0..MAX_COMBOS).map(|_| good.clone()).collect();
    assert!(s.create_combos(c, &many).is_err());
    assert_eq!(s.combos(c).unwrap().len(), 2, "vượt giới hạn: không tạo gì");
}

#[test]
fn deleting_case_row_cascades_and_deleted_case_rejects_writes() {
    let (_d, mut s) = open();
    let c = s.create_case("C", None, None).unwrap();
    let m = s.create_combo(c, None, None).unwrap();
    s.set_combo_slot(m, "body", Some(&key(1)), None, None).unwrap();
    s.delete_case(c).unwrap();
    assert!(s.add_to_case(c, &[key(2)], "added").is_err());
    assert!(s.create_combo(c, None, None).is_err());
    s.purge_cases(fontatlas_store_now() + 31 * 24 * 3600 * 1000).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM u.combos"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM u.combo_slots"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM u.case_faces"), 0);
}

#[test]
fn peek_open_is_atomic_idempotent_and_never_overwrites_silently() {
    let (_d, mut s) = open();
    let (a, c, other) = (key(1), key(2), key(3));
    let first = s.peek_open(None, r#"{"vi":true}"#, &a, Some(&c), "display", false).unwrap();
    assert!(first.created && first.occupied.is_none());
    let again = s.peek_open(None, r#"{"vi":false}"#, &a, Some(&c), "display", false).unwrap();
    assert_eq!((again.case_id, again.combo_id, again.created), (first.case_id, first.combo_id, false), "lần hai: cùng Case/Combo");
    assert_eq!(s.combos(first.case_id).unwrap().len(), 1);
    assert_eq!(s.combos(first.case_id).unwrap()[0].slots.len(), 1);
    assert_eq!(s.case_faces(first.case_id).unwrap().len(), 2);
    assert_eq!(s.case(first.case_id).unwrap().settings, r#"{"vi":true}"#, "Case có sẵn giữ cài đặt, không bị peek ghi đè");

    // vai đã có font khác: không ghi, trả occupied; replace = true mới thay
    let held = s.peek_open(None, "{}", &other, None, "display", false).unwrap();
    assert_eq!(held.occupied, Some(a));
    let slot = |s: &Store| s.combo(first.combo_id).unwrap().slots.into_iter().find(|x| x.role == "display").unwrap().key;
    assert_eq!(slot(&s), a, "chưa đồng ý thì giữ nguyên");
    assert!(s.case_faces(first.case_id).unwrap().iter().any(|f| f.key == other), "nhưng font vẫn vào Case để khỏi mất");
    let replaced = s.peek_open(None, "{}", &other, None, "display", true).unwrap();
    assert!(replaced.occupied.is_none());
    assert_eq!(slot(&s), other);
    // cùng font đang giữ vai thì không coi là bị chiếm
    assert!(s.peek_open(None, "{}", &other, None, "display", false).unwrap().occupied.is_none());

    // lỗi giữa chừng (Case đã đủ 500 font): không đổi gì, cài đặt giữ nguyên
    let big = s.create_case("Đầy", None, Some(r#"{"vi":true}"#)).unwrap();
    let fill: Vec<FaceKey> = (1000..1000 + MAX_CASE_FACES as u16).map(key).collect();
    s.add_to_case(big, &fill, "added").unwrap();
    let before = (s.case_faces(big).unwrap().len(), s.combos(big).unwrap().len());
    assert!(s.peek_open(Some(big), r#"{"vi":false}"#, &key(9999), None, "body", false).is_err());
    assert_eq!((s.case_faces(big).unwrap().len(), s.combos(big).unwrap().len()), before, "hoàn tác hết, kể cả Combo chính vừa tạo");
    assert_eq!(s.case(big).unwrap().settings, r#"{"vi":true}"#);

    // lỗi ở bước cuối trên thư viện chưa có Case nào: không để lại Case nào
    let (_d2, mut s2) = open();
    s2.conn()
        .execute_batch("CREATE TRIGGER u.deny_slot BEFORE INSERT ON combo_slots BEGIN SELECT RAISE(ABORT, 'boom'); END;")
        .unwrap();
    assert!(s2.peek_open(None, "{}", &a, Some(&c), "body", false).is_err());
    for t in ["cases", "case_faces", "combos", "combo_slots"] {
        assert_eq!(count(&s2, &format!("SELECT count(*) FROM u.{t}")), 0, "{t} phải trống sau khi hoàn tác");
    }
    assert!(s2.peek_open(None, "{}", &a, None, "lạ", false).is_err());
    assert_eq!(count(&s2, "SELECT count(*) FROM u.cases"), 0);
    s2.conn().execute_batch("DROP TRIGGER u.deny_slot").unwrap();
    assert!(s2.peek_open(None, "{}", &a, None, "body", false).is_ok(), "savepoint đã giải phóng, store dùng lại được");
}

#[test]
fn add_to_latest_case_is_atomic_and_reuses_the_open_case() {
    let (_d, mut s) = open();
    let (id1, added, already, created) = s.add_to_latest_case(&[key(1), key(2)], "added").unwrap();
    assert!(created && (added, already) == (2, 0));
    let (id2, added, already, created) = s.add_to_latest_case(&[key(2), key(3)], "added").unwrap();
    assert_eq!((id2, added, already, created), (id1, 1, 1, false));
    // quá giới hạn: không tạo Case mới nào ở thư viện trống
    let (_d2, mut s2) = open();
    let many: Vec<FaceKey> = (0..MAX_CASE_FACES as u16 + 1).map(key).collect();
    assert!(s2.add_to_latest_case(&many, "added").is_err());
    assert_eq!(count(&s2, "SELECT count(*) FROM u.cases"), 0, "Case vừa tạo bị hoàn tác cùng lỗi");
}

#[test]
fn trashed_case_rejects_combo_writes_and_purge_ignores_a_rewound_clock() {
    let (_d, mut s) = open();
    let c = s.create_case("C", None, None).unwrap();
    let m = s.create_combo(c, None, None).unwrap();
    s.set_combo_slot(m, "body", Some(&key(1)), None, None).unwrap();
    s.delete_case(c).unwrap();
    assert!(s.set_combo_slot(m, "heading", Some(&key(2)), None, None).is_err());
    assert!(s.update_combo(m, &ComboPatch { name: Some("x".into()), ..Default::default() }).is_err());
    assert!(s.delete_combo(m).is_err());
    assert!(s.create_combo(c, None, Some(m)).is_err());
    assert_eq!(s.combo(m).unwrap().slots.len(), 1, "không đổi gì");
    s.restore_case(c).unwrap();
    assert!(s.set_combo_slot(m, "heading", Some(&key(2)), None, None).is_ok(), "khôi phục thì ghi lại được");

    // đồng hồ lùi: purge không xoá gì dù `now - 30 ngày` tụt dưới mốc xoá cũ
    s.delete_case(c).unwrap();
    s.conn().execute("UPDATE u.cases SET deleted_at = 1000, created_at = 1000, updated_at = 1000, opened_at = 1000", []).unwrap();
    assert_eq!(s.purge_cases(500).unwrap(), 0, "now < mốc đã ghi ⇒ bỏ qua");
    assert_eq!(s.cases(true).unwrap().len(), 1);
    // chỉ Case có deleted_at mới bị xoá
    let live = s.create_case("Sống", None, None).unwrap();
    s.conn().execute("UPDATE u.cases SET created_at = 1, updated_at = 1, opened_at = 1 WHERE id = ?1", [live]).unwrap();
    assert_eq!(s.purge_cases(1000 + 31 * 24 * 3600 * 1000).unwrap(), 1);
    assert_eq!(s.cases(true).unwrap().iter().map(|x| x.id).collect::<Vec<_>>(), vec![live]);
}

#[test]
fn create_combos_rejects_duplicate_roles() {
    let (_d, mut s) = open();
    let c = s.create_case("C", None, None).unwrap();
    let dup = NewCombo { name: None, slots: vec![("body".into(), key(1)), ("body".into(), key(2))] };
    assert!(s.create_combos(c, &[dup]).is_err());
    assert!(s.combos(c).unwrap().is_empty());
}
