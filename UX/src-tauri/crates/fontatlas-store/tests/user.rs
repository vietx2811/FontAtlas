mod common;
use common::*;
use fontatlas_store::{FaceKey, SourceKind};

fn key(n: u8, i: u32) -> FaceKey {
    FaceKey { hash: [n; 32], face_index: i }
}

#[test]
fn source_dedupes_by_kind_and_path() {
    let d = tmp();
    let p = d.path().join("Fonts");
    std::fs::create_dir_all(&p).unwrap();
    let mut s = open_store(&d.path().join("db"));
    let a = s.add_source(SourceKind::Directory, "Fonts", Some(&p)).unwrap();
    let b = s.add_source(SourceKind::Directory, "Tên khác", Some(&p)).unwrap();
    assert_eq!(a, b);
    assert_eq!(s.sources().unwrap().len(), 1);
    assert_eq!(s.source(a).unwrap().kind, SourceKind::Directory);
    assert_eq!(s.source(999).unwrap_err().code(), "not_found");
}

#[test]
fn pathless_source_dedupes() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.add_source(SourceKind::GoogleFonts, "G", None).unwrap();
    let b = s.add_source(SourceKind::GoogleFonts, "G2", None).unwrap();
    assert_eq!(a, b);
    let c = s.add_source(SourceKind::AdobeFonts, "A", None).unwrap();
    assert_ne!(a, c);
}

#[test]
fn nested_sources_are_blocked_both_ways() {
    let d = tmp();
    let root = d.path().join("Thư viện");
    let child = root.join("Serif");
    let sibling = d.path().join("Thư viện2"); // cùng tiền tố chuỗi nhưng không lồng
    for p in [&child, &sibling] {
        std::fs::create_dir_all(p).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let parent = s.add_source(SourceKind::Directory, "Cha", Some(&root)).unwrap();
    let e = s.add_source(SourceKind::Directory, "Con", Some(&child)).unwrap_err();
    assert_eq!(e.code(), "nested_source");
    assert!(matches!(e, fontatlas_store::StoreError::NestedSource { existing_id } if existing_id == parent));
    assert!(s.add_source(SourceKind::Directory, "Anh em", Some(&sibling)).is_ok());
    let e = s.add_source(SourceKind::System, "Cùng path khác kind", Some(&root)).unwrap_err();
    assert_eq!(e.code(), "nested_source");

    let d2 = tmp();
    let root2 = d2.path().join("A");
    let child2 = root2.join("B");
    std::fs::create_dir_all(&child2).unwrap();
    let mut s2 = open_store(&d2.path().join("db"));
    let c = s2.add_source(SourceKind::Directory, "Con", Some(&child2)).unwrap();
    let e = s2.add_source(SourceKind::Directory, "Cha", Some(&root2)).unwrap_err();
    assert!(matches!(e, fontatlas_store::StoreError::NestedSource { existing_id } if existing_id == c));
}

#[test]
fn nested_check_uses_nfc_for_vietnamese_paths() {
    use unicode_normalization::UnicodeNormalization;
    let d = tmp();
    let root = d.path().join("Phông");
    std::fs::create_dir_all(root.join("Mới")).unwrap();
    let mut s = open_store(&d.path().join("db"));
    s.add_source(SourceKind::Directory, "Cha", Some(&root)).unwrap();
    let child_nfd: String = root.join("Mới").to_string_lossy().nfd().collect();
    let e = s.add_source(SourceKind::Directory, "Con", Some(std::path::Path::new(&child_nfd))).unwrap_err();
    assert_eq!(e.code(), "nested_source");
}

#[test]
fn favorites_are_idempotent_and_survive_library_rebuild() {
    let d = tmp();
    {
        let mut s = open_store(d.path());
        s.set_favorite(&key(1, 0), true).unwrap();
        s.set_favorite(&key(1, 0), true).unwrap();
        s.set_favorite(&key(2, 1), true).unwrap();
        s.set_favorite(&key(2, 1), false).unwrap();
        assert_eq!(s.favorites().unwrap(), vec![key(1, 0)]);
    }
    for f in ["library.db", "library.db-wal", "library.db-shm"] {
        let _ = std::fs::remove_file(d.path().join(f));
    }
    let s = open_store(d.path());
    assert_eq!(s.favorites().unwrap(), vec![key(1, 0)]);
}

#[test]
fn collections_crud_and_cascade() {
    let d = tmp();
    let mut s = open_store(d.path());
    let c = s.create_collection("Menu quán phở", Some("#c0392b")).unwrap();
    assert_eq!(s.add_to_collection(c, &[key(1, 0), key(2, 0)]).unwrap(), 2);
    assert_eq!(s.add_to_collection(c, &[key(1, 0)]).unwrap(), 0);
    s.rename_collection(c, "Menu").unwrap();
    assert_eq!(s.rename_collection(777, "x").unwrap_err().code(), "not_found");
    let all = s.collections().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].name.as_str(), all[0].item_count), ("Menu", 2));
    assert_eq!(s.remove_from_collection(c, &[key(2, 0)]).unwrap(), 1);
    s.delete_collection(c).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM u.collection_items"), 0);
}

#[test]
fn tags_dedupe_by_folded_name_and_keep_first_spelling() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.tag("Cổ điển").unwrap();
    let b = s.tag("co dien").unwrap();
    let c = s.tag("  CỔ ĐIỂN ").unwrap();
    assert_eq!((a, a), (b, c));
    assert_eq!(s.tag("   ").unwrap_err().code(), "not_found");
    assert_eq!(s.tag_faces(a, &[key(1, 0), key(1, 1)]).unwrap(), 2);
    let tags = s.tags().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!((tags[0].name.as_str(), tags[0].face_count), ("Cổ điển", 2));
    assert_eq!(s.tags_of(&key(1, 1)).unwrap()[0].id, a);
    assert_eq!(s.untag_faces(a, &[key(1, 1)]).unwrap(), 1);
}

#[test]
fn rename_tag_keeps_assignments_and_never_merges_silently() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.tag("Cổ điển").unwrap();
    let b = s.tag("Hiện đại").unwrap();
    s.tag_faces(a, &[key(1, 0), key(2, 0)]).unwrap();
    let t = s.rename_tag(a, "  Retro ").unwrap();
    assert_eq!((t.id, t.name.as_str(), t.face_count), (a, "Retro", 2), "giữ nguyên face_tags, trim tên");
    assert_eq!(s.tags_of(&key(2, 0)).unwrap()[0].name, "Retro");
    // đổi hoa/thường/dấu của chính nó hợp lệ
    assert_eq!(s.rename_tag(b, "HIEN DAI").unwrap().name, "HIEN DAI");
    // trùng tên (không phân biệt hoa/thường/dấu) với tag KHÁC → conflict, không đổi gì
    assert_eq!(s.rename_tag(a, "hien dai").unwrap_err().code(), "conflict");
    assert_eq!(s.tags().unwrap().iter().find(|t| t.id == a).unwrap().name, "Retro");
    assert_eq!(s.rename_tag(a, "   ").unwrap_err().code(), "invalid_arg");
    assert_eq!(s.rename_tag(999, "X").unwrap_err().code(), "not_found");
}

#[test]
fn delete_tag_cascades_to_assignments_and_reports_the_count() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.tag("a").unwrap();
    let b = s.tag("b").unwrap();
    s.tag_faces(a, &[key(1, 0), key(1, 1), key(2, 0)]).unwrap();
    s.tag_faces(b, &[key(1, 0)]).unwrap();
    assert_eq!(s.delete_tag(a).unwrap(), 3, "số face bị bỏ thẻ");
    assert_eq!(s.tags().unwrap().len(), 1);
    assert_eq!(s.tags_of(&key(1, 0)).unwrap().iter().map(|t| t.id).collect::<Vec<_>>(), [b], "thẻ khác còn nguyên");
    assert_eq!(s.delete_tag(a).unwrap_err().code(), "not_found");
    assert_eq!(count(&s, "SELECT count(*) FROM u.face_tags WHERE tag_id NOT IN (SELECT tag_id FROM u.tags)"), 0, "không mồ côi");
}

#[test]
fn rename_tag_normalizes_nfc_and_pho_vs_pho_conflict_is_intentional() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.tag("Retro").unwrap();
    let other = s.tag("Other").unwrap();
    // "Phở" gõ rời dấu (o + U+031B + U+0309) lưu thành NFC dựng sẵn
    let t = s.rename_tag(a, "  Pho\u{031B}\u{0309}  ").unwrap();
    assert_eq!(t.name, "Ph\u{1EDF}", "NFC + trim");
    assert_eq!(s.tags().unwrap().iter().find(|x| x.id == a).unwrap().name, "Ph\u{1EDF}");
    // CHỦ Ý: "Phở" và "Pho" cùng khoá gập dấu ⇒ conflict (cùng luật dedupe với tag()/create_tag)
    assert_eq!(s.rename_tag(other, "Pho").unwrap_err().code(), "conflict");
    // id lạ ⇒ not_found, dù tên mới trùng một Tag khác
    assert_eq!(s.rename_tag(9999, "Retro").unwrap_err().code(), "not_found");
    assert_eq!(s.rename_tag(9999, "Phở").unwrap_err().code(), "not_found");
}
