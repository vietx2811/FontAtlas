//! G3 (0.4.0): vai Combo tuỳ ý — tên, trùng không phân biệt hoa thường, vai rỗng giữ chỗ, đổi tên/xoá/sắp xếp, migration v10 → v11.
mod common;
use common::*;
use fontatlas_store::{FaceKey, MAX_ROLES_PER_COMBO, NewCombo, Store, create_user_pre_combo_roles_for_test};

fn key(n: u16) -> FaceKey {
    let mut h = [0u8; 32];
    h[..2].copy_from_slice(&n.to_le_bytes());
    FaceKey { hash: h, face_index: 0 }
}

fn setup() -> (tempfile::TempDir, Store, i64) {
    let d = tmp();
    let mut s = open_store(d.path());
    let c = s.create_case("C", None, None).unwrap();
    let combo = s.create_combo(c, None, None).unwrap();
    (d, s, combo)
}

fn code<T: std::fmt::Debug>(r: Result<T, fontatlas_store::StoreError>) -> &'static str {
    r.unwrap_err().code()
}

#[test]
fn custom_role_is_created_by_set_slot_and_keeps_order() {
    let (_d, mut s, combo) = setup();
    s.set_combo_slot(combo, "body", Some(&key(1)), None, None).unwrap();
    s.set_combo_slot(combo, "Label", Some(&key(2)), None, None).unwrap();
    let r = s.set_combo_slot(combo, "Numerals", Some(&key(3)), None, None).unwrap();
    assert_eq!(r.roles, vec!["body", "Label", "Numerals"]);
    assert_eq!(r.slots.iter().map(|x| x.role.as_str()).collect::<Vec<_>>(), vec!["body", "Label", "Numerals"]);
    // preset thêm sau vẫn đứng theo vị trí preset (display < body)
    let r = s.set_combo_slot(combo, "display", Some(&key(4)), None, None).unwrap();
    assert_eq!(r.roles[0], "display");
}

#[test]
fn role_names_are_trimmed_limited_and_unique_case_insensitively() {
    let (_d, mut s, combo) = setup();
    let r = s.add_combo_role(combo, "  Tiêu đề phụ  ").unwrap();
    assert_eq!(r.roles, vec!["Tiêu đề phụ"], "trim");
    assert_eq!(code(s.add_combo_role(combo, "")), "role_name_invalid");
    assert_eq!(code(s.add_combo_role(combo, "   ")), "role_name_invalid");
    assert_eq!(code(s.add_combo_role(combo, &"x".repeat(41))), "role_name_invalid");
    s.add_combo_role(combo, &"x".repeat(40)).unwrap();
    assert_eq!(code(s.add_combo_role(combo, "a\nb")), "role_name_invalid");
    // trùng không phân biệt hoa thường, kể cả có dấu và trùng khoá preset
    assert_eq!(code(s.add_combo_role(combo, "TIÊU ĐỀ PHỤ")), "role_duplicate");
    s.set_combo_slot(combo, "body", None, None, None).unwrap();
    assert_eq!(code(s.add_combo_role(combo, "BODY")), "role_duplicate");
    // setComboSlot với tên khác hoa thường dùng lại vai đã có, không tạo vai thứ hai
    let r = s.set_combo_slot(combo, "BODY", Some(&key(1)), None, None).unwrap();
    assert_eq!(r.roles.iter().filter(|x| x.eq_ignore_ascii_case("body")).count(), 1);
    assert_eq!(r.slots[0].role, "body", "giữ chữ đã lưu");
}

#[test]
fn clearing_a_font_keeps_the_role_as_a_placeholder_and_remove_drops_it() {
    let (_d, mut s, combo) = setup();
    s.set_combo_slot(combo, "Quote 2", Some(&key(7)), None, None).unwrap();
    let r = s.set_combo_slot(combo, "Quote 2", None, None, None).unwrap();
    assert_eq!((r.slots.len(), r.roles.clone()), (0, vec!["Quote 2".to_string()]));
    let r = s.add_combo_role(combo, "Rỗng").unwrap();
    assert_eq!(r.roles, vec!["Quote 2", "Rỗng"]);
    assert!(r.slots.is_empty());
    let r = s.remove_combo_role(combo, "quote 2").unwrap();
    assert_eq!(r.roles, vec!["Rỗng"]);
    assert_eq!(code(s.remove_combo_role(combo, "không có")), "role_not_found");
    // font vẫn ở trong Case sau khi bỏ vai
    let case_id = s.combo(combo).unwrap().case_id;
    assert!(s.case_faces(case_id).unwrap().iter().any(|f| f.key == key(7)));
}

#[test]
fn rename_keeps_font_note_and_position() {
    let (_d, mut s, combo) = setup();
    s.set_combo_slot(combo, "A", Some(&key(1)), Some("ghi chú"), Some(r#"{"w":1}"#)).unwrap();
    s.add_combo_role(combo, "B").unwrap();
    let r = s.rename_combo_role(combo, "a", "Đầu trang").unwrap();
    assert_eq!(r.roles, vec!["Đầu trang", "B"]);
    assert_eq!((r.slots[0].role.as_str(), r.slots[0].key, r.slots[0].note.as_deref()), ("Đầu trang", key(1), Some("ghi chú")));
    assert_eq!(code(s.rename_combo_role(combo, "Đầu trang", "b")), "role_duplicate");
    assert_eq!(code(s.rename_combo_role(combo, "Đầu trang", " ")), "role_name_invalid");
    assert_eq!(code(s.rename_combo_role(combo, "ma", "X")), "role_not_found");
    // đổi chỉ hoa/thường của chính nó thì được
    let r = s.rename_combo_role(combo, "B", "b").unwrap();
    assert_eq!(r.roles, vec!["Đầu trang", "b"]);
}

#[test]
fn reorder_needs_exactly_the_existing_set() {
    let (_d, mut s, combo) = setup();
    for n in ["Một", "Hai", "Ba"] {
        s.add_combo_role(combo, n).unwrap();
    }
    s.set_combo_slot(combo, "Hai", Some(&key(2)), None, None).unwrap();
    let r = s.reorder_combo_roles(combo, &["ba".into(), "HAI".into(), "Một".into()]).unwrap();
    assert_eq!(r.roles, vec!["Ba", "Hai", "Một"], "so khớp không phân biệt hoa thường, giữ chữ đã lưu");
    assert_eq!(r.slots[0].role, "Hai");
    for bad in [vec!["Ba", "Hai"], vec!["Ba", "Hai", "Hai"], vec!["Ba", "Hai", "Một", "Bốn"], vec!["Ba", "Hai", "X"]] {
        let bad: Vec<String> = bad.into_iter().map(String::from).collect();
        assert_eq!(code(s.reorder_combo_roles(combo, &bad)), "role_set_mismatch", "{bad:?}");
    }
    assert_eq!(s.combo(combo).unwrap().roles, vec!["Ba", "Hai", "Một"], "lỗi không đổi gì");
    // thứ tự bền qua đọc lại
    assert_eq!(s.combo(combo).unwrap().slots[0].key, key(2));
}

#[test]
fn role_limit_per_combo() {
    let (_d, mut s, combo) = setup();
    for i in 0..MAX_ROLES_PER_COMBO {
        s.add_combo_role(combo, &format!("Vai {i}")).unwrap();
    }
    assert_eq!(code(s.add_combo_role(combo, "Thừa")), "role_limit");
    assert_eq!(code(s.set_combo_slot(combo, "Thừa", Some(&key(1)), None, None)), "role_limit");
    assert_eq!(s.combo(combo).unwrap().roles.len(), MAX_ROLES_PER_COMBO, "không để lại vai dở dang");
}

#[test]
fn duplicate_and_create_combos_carry_roles() {
    let (_d, mut s, combo) = setup();
    s.set_combo_slot(combo, "Label", Some(&key(1)), None, None).unwrap();
    s.add_combo_role(combo, "Trống").unwrap();
    let case_id = s.combo(combo).unwrap().case_id;
    let dup = s.create_combo(case_id, Some("Bản sao"), Some(combo)).unwrap();
    let d = s.combo(dup).unwrap();
    assert_eq!((d.roles.clone(), d.slots.len()), (vec!["Label".to_string(), "Trống".to_string()], 1), "nhân bản cả vai rỗng");
    let ids = s.create_combos(case_id, &[NewCombo { name: None, slots: vec![("Caption 2".into(), key(2)), ("display".into(), key(3))] }]).unwrap();
    assert_eq!(s.combo(ids[0]).unwrap().roles, vec!["display", "Caption 2"]);
    let dupname = s.create_combos(case_id, &[NewCombo { name: None, slots: vec![("A".into(), key(2)), ("a".into(), key(3))] }]);
    assert_eq!(dupname.unwrap_err().code(), "role_duplicate");
}

#[test]
fn removing_a_font_from_the_case_leaves_the_role_empty() {
    let (_d, mut s, combo) = setup();
    s.set_combo_slot(combo, "Label", Some(&key(1)), None, None).unwrap();
    let case_id = s.combo(combo).unwrap().case_id;
    s.remove_from_case(case_id, &[key(1)]).unwrap();
    let c = s.combo(combo).unwrap();
    assert_eq!((c.slots.len(), c.roles), (0, vec!["Label".to_string()]));
}

#[test]
fn migration_v10_to_v11_round_trips_legacy_slots() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_pre_combo_roles_for_test(&user);
    let (h1, h2) = (vec![1u8; 32], vec![2u8; 32]);
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        c.execute("INSERT INTO cases(id,name,settings,view_state,created_at,updated_at,opened_at) VALUES (1,'Cũ','{}','{}',1,1,1)", []).unwrap();
        c.execute("INSERT INTO combos(id,case_id,idx,name,is_main,created_at,updated_at) VALUES (1,1,1,'Combo 1',1,1,1)", []).unwrap();
        for (role, ord, h, note) in [("display", 0, &h1, Some("neo")), ("body", 3, &h2, None), ("quote", 6, &h1, None)] {
            c.execute(
                "INSERT INTO combo_slots(combo_id,role,ord,hash,face_index,note,style) VALUES (1,?1,?2,?3,0,?4,'{\"weight\":700}')",
                rusqlite::params![role, ord, h, note],
            )
            .unwrap();
        }
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.user_backup.is_some(), "migration có sao lưu");
    assert_eq!(count(&s, "PRAGMA u.user_version"), fontatlas_store::USER_VERSION as i64);
    let combo = s.combo(1).unwrap();
    assert_eq!(combo.roles, vec!["display", "body", "quote"]);
    let got: Vec<(String, i64, Option<String>, String)> = combo.slots.iter().map(|x| (x.role.clone(), x.ord, x.note.clone(), x.style.clone())).collect();
    assert_eq!(
        got,
        vec![
            ("display".to_string(), 0, Some("neo".to_string()), r#"{"weight":700}"#.to_string()),
            ("body".to_string(), 3, None, r#"{"weight":700}"#.to_string()),
            ("quote".to_string(), 6, None, r#"{"weight":700}"#.to_string()),
        ]
    );
    assert_eq!(combo.slots[0].key.hash.to_vec(), h1);
    // sau migration: bảng mới nhận vai tuỳ ý và vai rỗng, và ghi lại được như thường
    let mut s = s;
    s.set_combo_slot(1, "Label", Some(&FaceKey { hash: [3u8; 32], face_index: 1 }), None, None).unwrap();
    s.add_combo_role(1, "Trống").unwrap();
    let again = s.combo(1).unwrap();
    assert_eq!(again.roles, vec!["display", "body", "quote", "Label", "Trống"]);
    drop(s);
    let (s2, _) = Store::open(d.path()).unwrap();
    assert_eq!(s2.combo(1).unwrap().roles, again.roles, "round-trip qua đóng/mở lại");
}
