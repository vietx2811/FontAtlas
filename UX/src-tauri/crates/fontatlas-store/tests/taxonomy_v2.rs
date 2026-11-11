//! Migration taxonomy v2: ghi đè phân loại cũ chuyển sang class mới, không mất, có backup, khớp `migrate_legacy`.
mod common;
use common::*;
use fontatlas_core::classify::{Class, migrate_legacy, subclasses_of};
use fontatlas_store::{FaceKey, Store, create_user_pre_taxonomy_for_test};

#[test]
fn legacy_overrides_migrate_like_migrate_legacy_with_backup_and_no_loss() {
    let d = tmp();
    let user = d.path().join(fontatlas_store::USER_FILE);
    create_user_pre_taxonomy_for_test(&user);
    // mọi (class, subclass) v1 hợp lệ, mỗi cái một face
    let mut cases: Vec<(String, Option<String>)> = Vec::new();
    for c in ["serif", "sans", "mono", "script", "blackletter", "symbol"] {
        cases.push((c.into(), None));
        if let Some(k) = Class::parse(c) {
            for s in subclasses_of(k) {
                cases.push((c.into(), Some((*s).into())));
            }
        }
    }
    // cặp lạ: subclass không thuộc class (SQL phải bỏ subclass như `migrate_legacy`, giữ class)
    for (c, s) in [("sans", "slab"), ("blackletter", "wedge"), ("serif", "handwritten"), ("mono", "egyptian"), ("script", "garalde")] {
        cases.push((c.into(), Some(s.into())));
    }
    {
        let c = rusqlite::Connection::open(&user).unwrap();
        for (i, (cl, sub)) in cases.iter().enumerate() {
            c.execute(
                "INSERT INTO classification_overrides(hash, face_index, class, subclass, updated_at) VALUES (?1,0,?2,?3,?4)",
                rusqlite::params![&[i as u8; 32][..], cl, sub, 1000 + i as i64],
            )
            .unwrap();
        }
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.user_backup.is_some(), "phải có backup trước khi đổi");
    let rows = s.classification_overrides().unwrap();
    assert_eq!(rows.len(), cases.len(), "không mất ghi đè nào");
    for (i, (cl, sub)) in cases.iter().enumerate() {
        let want = migrate_legacy(cl, sub.as_deref());
        let got = s.classification_override(&FaceKey { hash: [i as u8; 32], face_index: 0 }).unwrap().expect("còn ghi đè");
        assert_eq!((got.class.clone(), got.subclass.clone()), want, "{cl}/{sub:?}");
        assert_eq!(got.updated_at, 1000 + i as i64, "giữ nguyên thời điểm");
    }
    // lớp mới ghi được (CHECK đã mở rộng)
    let mut s = s;
    let k = FaceKey { hash: [200; 32], face_index: 0 };
    for (c, sub) in [("slab", Some("clarendon")), ("handwriting", Some("marker")), ("display", Some("stencil"))] {
        s.set_classification_override(&k, c, sub).unwrap();
        assert_eq!(s.classification_override(&k).unwrap().unwrap().class, c);
    }
}
