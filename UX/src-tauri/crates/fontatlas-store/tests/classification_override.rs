mod common;
use common::*;
use fontatlas_store::{FaceKey, Store};

fn key(n: u8) -> FaceKey {
    FaceKey { hash: [n; 32], face_index: 0 }
}

#[test]
fn override_roundtrip_upsert_and_clear() {
    let d = tmp();
    let mut s = open_store(d.path());
    assert!(s.classification_override(&key(1)).unwrap().is_none());
    s.set_classification_override(&key(1), "script", Some("formal")).unwrap();
    s.set_classification_override(&key(2), "mono", None).unwrap();
    let o = s.classification_override(&key(1)).unwrap().unwrap();
    assert_eq!((o.class.as_str(), o.subclass.as_deref()), ("script", Some("formal")));
    // ghi đè lần nữa = cập nhật
    s.set_classification_override(&key(1), "display", None).unwrap();
    let o = s.classification_override(&key(1)).unwrap().unwrap();
    assert_eq!((o.class.as_str(), o.subclass), ("display", None));
    assert_eq!(s.classification_overrides().unwrap().len(), 2);
    s.clear_classification_override(&key(1)).unwrap();
    assert!(s.classification_override(&key(1)).unwrap().is_none());
    // xoá cái không có: không lỗi
    s.clear_classification_override(&key(9)).unwrap();
}

#[test]
fn override_survives_reopen_and_lives_in_user_db() {
    let d = tmp();
    {
        let mut s = open_store(d.path());
        s.set_classification_override(&key(3), "serif", Some("slab")).unwrap();
    }
    let u = Store::open_user_only(d.path()).unwrap();
    assert_eq!(u.classification_override(&key(3)).unwrap().unwrap().class, "serif");
    assert_eq!(count(&u, "SELECT count(*) FROM u.classification_overrides"), 1);
}
