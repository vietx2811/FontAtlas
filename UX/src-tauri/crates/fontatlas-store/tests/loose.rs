//! G8: nguồn `loose` (font lẻ), `import_files`, `faces_at`, `refresh_file_source`; migration nới `sources.kind`.
mod common;
use common::*;
use fontatlas_store::{FileImportOutcome, SourceKind};
use std::sync::atomic::AtomicBool;

#[test]
fn loose_and_derived_kinds_are_accepted_singletons_and_old_sources_survive_the_migration() {
    let d = tmp();
    let mut s = open_store(&d.path().join("db"));
    let dir = d.path().join("F");
    std::fs::create_dir_all(&dir).unwrap();
    let a = s.add_source(SourceKind::Directory, "F", Some(&dir)).unwrap();
    let l1 = s.add_source(SourceKind::Loose, "Font lẻ", None).unwrap();
    let l2 = s.add_source(SourceKind::Loose, "Font lẻ", None).unwrap();
    let dv = s.add_source(SourceKind::Derived, "Bản sao", None).unwrap();
    assert_eq!(l1, l2, "một nguồn loose duy nhất");
    assert!(a != l1 && l1 != dv);
    let kinds: Vec<_> = s.sources().unwrap().iter().map(|x| x.kind).collect();
    assert_eq!(kinds, vec![SourceKind::Directory, SourceKind::Loose, SourceKind::Derived]);
    assert_eq!(count(&s, "SELECT count(*) FROM pragma_table_info('sources', 'u') WHERE name='priority'"), 1, "cột priority còn");
}

#[test]
fn single_files_outside_any_source_import_in_place_and_follow_the_file() {
    let d = tmp();
    let mut s = open_store(&d.path().join("db"));
    let loose = s.add_source(SourceKind::Loose, "Font lẻ", None).unwrap();
    let dl = d.path().join("Downloads");
    std::fs::create_dir_all(&dl).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), dl.join("roboto.ttf")).unwrap();
    std::fs::write(dl.join("notes.txt"), b"x").unwrap();
    let paths = vec![dl.join("roboto.ttf"), dl.join("notes.txt"), dl.join("ghost.ttf")];
    let out = s.import_files(loose, &paths).unwrap();
    let of = |name: &str| out.iter().find(|(p, _)| p.ends_with(name)).unwrap().1;
    assert_eq!(of("roboto.ttf"), FileImportOutcome::Imported);
    assert_eq!(of("notes.txt"), FileImportOutcome::Missing);
    assert_eq!(of("ghost.ttf"), FileImportOutcome::Missing);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE source_id=1 OR source_id>0"), 1, "chỉ file font được nhập, KHÔNG nhập cả thư mục");
    assert!(std::fs::read(dl.join("notes.txt")).is_ok());

    let key = |s: &fontatlas_store::Store| s.faces_at(&[dl.join("roboto.ttf").to_string_lossy().into()], &[]).unwrap();
    assert!(!key(&s).is_empty());
    // nhập lần hai: đã biết, không đổi gì
    assert_eq!(s.import_files(loose, &paths[..1]).unwrap()[0].1, FileImportOutcome::AlreadyKnown);

    // xoá file → refresh đánh missing, face rời khỏi faces_at; trả file → trở lại
    std::fs::remove_file(dl.join("roboto.ttf")).unwrap();
    assert_eq!(s.refresh_file_source(loose).unwrap(), 1);
    assert!(key(&s).is_empty());
    std::fs::copy(core_fixture("roboto.ttf"), dl.join("roboto.ttf")).unwrap();
    assert_eq!(s.refresh_file_source(loose).unwrap(), 1);
    assert!(!key(&s).is_empty());
    assert_eq!(s.refresh_file_source(loose).unwrap(), 0, "không đổi thì không ghi");
}

#[test]
fn a_file_already_in_a_directory_source_is_never_stolen_by_the_loose_source() {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), fonts.join("roboto.ttf")).unwrap();
    let mut s = open_store(&d.path().join("db"));
    let dir_src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    s.rescan_source_with(dir_src, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    let loose = s.add_source(SourceKind::Loose, "Font lẻ", None).unwrap();
    let out = s.import_files(loose, &[fonts.join("roboto.ttf")]).unwrap();
    assert_eq!(out[0].1, FileImportOutcome::AlreadyKnown);
    assert_eq!(count(&s, &format!("SELECT count(*) FROM file_locations WHERE source_id = {dir_src}")), 1);
}

#[test]
fn faces_at_matches_a_folder_by_whole_path_components_only() {
    let d = tmp();
    let a = d.path().join("Fonts");
    let b = d.path().join("Fonts2");
    for p in [&a, &b] {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::copy(core_fixture("roboto.ttf"), a.join("roboto.ttf")).unwrap();
    std::fs::copy(core_fixture("open_sans.ttf"), b.join("open_sans.ttf")).unwrap();
    let mut s = open_store(&d.path().join("db"));
    for (n, p) in [("A", &a), ("B", &b)] {
        let id = s.add_source(SourceKind::Directory, n, Some(p)).unwrap();
        s.rescan_source_with(id, &mut |_| {}, &AtomicBool::new(false)).unwrap();
    }
    let in_a = s.faces_at(&[], &[a.to_string_lossy().into()]).unwrap();
    assert!(!in_a.is_empty() && in_a.iter().all(|(p, _)| p.starts_with(&*a.to_string_lossy())));
    assert!(in_a.iter().all(|(p, _)| p.contains("/Fonts/")), "Fonts2 không bị khớp nhầm");
}
