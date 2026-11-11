//! Sổ "bản cũ đã gỡ để thay bằng bản của Font Atlas" (user.db): cho nút "Khôi phục bản cũ".
mod common;
use common::*;
use fontatlas_store::{ActivationScope, NewReplaced};

fn new_entry(hash: [u8; 32], path: &str, scopes: Vec<ActivationScope>) -> NewReplaced {
    NewReplaced { hash, old_path: path.into(), owner: "right_font".into(), scopes, postscript_names: vec!["A-Regular".into(), "A-Bold".into()] }
}

#[test]
fn table_exists_after_open_in_user_db_only() {
    let d = tmp();
    let s = open_store(d.path());
    assert_eq!(count(&s, "SELECT count(*) FROM u.sqlite_schema WHERE name='activation_replaced'"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM main.sqlite_schema WHERE name='activation_replaced'"), 0, "dữ liệu người dùng, không nằm trong library.db");
}

#[test]
fn journal_round_trip_and_restore_is_one_shot() {
    let d = tmp();
    let mut s = open_store(d.path());
    let id = s.add_replaced(&new_entry([7; 32], "/x/lib.rightfontlibrary/a.ttf", vec![ActivationScope::Persistent])).unwrap();
    let id2 = s.add_replaced(&new_entry([7; 32], "/y/b.ttf", vec![ActivationScope::Session, ActivationScope::Persistent])).unwrap();
    assert!(id2 > id);
    let all = s.unrestored_replaced().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].old_path, "/x/lib.rightfontlibrary/a.ttf");
    assert_eq!(all[0].scopes, vec![ActivationScope::Persistent]);
    assert_eq!(all[1].scopes, vec![ActivationScope::Session, ActivationScope::Persistent]);
    assert_eq!(all[0].postscript_names, vec!["A-Regular".to_string(), "A-Bold".to_string()]);
    assert_eq!(all[0].hash, [7; 32]);
    assert_eq!(s.get_replaced(id).unwrap().unwrap().owner, "right_font");
    assert!(s.get_replaced(999).unwrap().is_none());
    assert!(s.mark_replaced_restored(id).unwrap(), "lần đầu khôi phục được");
    assert!(!s.mark_replaced_restored(id).unwrap(), "đã khôi phục rồi thì không làm lại");
    assert_eq!(s.unrestored_replaced().unwrap().len(), 1);
    assert!(s.get_replaced(id).unwrap().unwrap().restored_at.is_some());
}

#[test]
fn undo_the_record_when_the_swap_failed() {
    let d = tempfile::tempdir().unwrap();
    let mut s = open_store(d.path());
    let id = s.add_replaced(&new_entry([1; 32], "/p.ttf", vec![ActivationScope::Session])).unwrap();
    assert!(s.delete_replaced(id).unwrap());
    assert!(s.unrestored_replaced().unwrap().is_empty());
    assert!(!s.delete_replaced(id).unwrap());
}

#[test]
fn journal_survives_a_library_rebuild() {
    let d = tmp();
    {
        let mut s = open_store(d.path());
        s.add_replaced(&new_entry([2; 32], "/q.ttf", vec![ActivationScope::Persistent])).unwrap();
    }
    for f in ["library.db", "library.db-wal", "library.db-shm"] {
        let _ = std::fs::remove_file(d.path().join(f));
    }
    let s = open_store(d.path());
    assert_eq!(s.unrestored_replaced().unwrap().len(), 1);
}
