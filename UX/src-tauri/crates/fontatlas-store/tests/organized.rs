//! B7: sổ Organized Library — ứng viên chép, ghi bản chép, di chuyển có hoàn tác, vị trí file đi theo.
mod common;
use common::*;
use fontatlas_store::{OrganizedFile, SourceKind};
use std::sync::atomic::AtomicBool;

#[test]
fn candidates_prefer_local_originals_outside_the_library_and_flag_files_already_inside() {
    let d = tmp();
    let a = d.path().join("A");
    let lib = d.path().join("Lib/Library");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), a.join("roboto.ttf")).unwrap();
    std::fs::copy(core_fixture("open_sans.ttf"), a.join("open_sans.ttf")).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), lib.join("Roboto.ttf")).unwrap(); // đã có bản trong thư viện
    let mut s = open_store(&d.path().join("db"));
    for (n, p) in [("A", &a), ("Lib", &lib)] {
        let id = s.add_source(SourceKind::Directory, n, Some(p)).unwrap();
        s.rescan_source_with(id, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    }
    let c = s.organize_candidates(&lib.to_string_lossy()).unwrap();
    assert_eq!(c.len(), 2, "hai font khác nhau (theo hash)");
    let roboto = c.iter().find(|x| x.src_path.ends_with("A/roboto.ttf")).unwrap();
    assert!(roboto.in_library && !roboto.cloud_only && !roboto.face_ids.is_empty());
    let open = c.iter().find(|x| x.src_path.ends_with("open_sans.ttf")).unwrap();
    assert!(!open.in_library);
    assert!(!open.family.is_empty());
}

#[test]
fn record_move_and_undo_keep_the_ledger_and_file_locations_in_step() {
    let d = tmp();
    let a = d.path().join("A");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), a.join("roboto.ttf")).unwrap();
    let mut s = open_store(&d.path().join("db"));
    let id = s.add_source(SourceKind::Directory, "A", Some(&a)).unwrap();
    s.rescan_source_with(id, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    let hash = s.organize_candidates("/nope").unwrap()[0].hash;
    let from = a.join("roboto.ttf").to_string_lossy().to_string();
    let to = a.join("Sans/Roboto/roboto.ttf").to_string_lossy().to_string();
    let f = OrganizedFile { hash, dest_path: from.clone(), src_path: "/orig/roboto.ttf".into(), class_key: "uncertain".into(), family: "Roboto".into(), copied_at: 1 };
    s.organized_record(&f).unwrap();
    assert!(s.organized_dest_taken(&from).unwrap());

    let batch = s.organized_next_batch().unwrap();
    assert_eq!(batch, 1);
    s.organized_apply_move(batch, &hash, &from, &to, "sans", "Roboto").unwrap();
    assert_eq!(s.organized_file(&hash).unwrap().unwrap().dest_path, to);
    assert_eq!(count(&s, &format!("SELECT count(*) FROM file_locations WHERE path = '{to}'")), 1, "vị trí file đi theo");
    assert_eq!(s.organized_last_batch().unwrap(), Some(1));

    let moves = s.organized_moves(batch).unwrap();
    assert_eq!(moves.len(), 1);
    s.organized_mark_undone(&moves[0], "uncertain", "Roboto").unwrap();
    assert_eq!(s.organized_file(&hash).unwrap().unwrap().dest_path, from);
    assert_eq!(count(&s, &format!("SELECT count(*) FROM file_locations WHERE path = '{from}'")), 1);
    assert_eq!(s.organized_last_batch().unwrap(), None, "đã hoàn tác hết");
    assert_eq!(s.organized_next_batch().unwrap(), 2);
}
