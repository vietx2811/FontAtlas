mod common;
use common::*;
use fontatlas_core::{Container, CoreError, FontFile};

#[test]
fn opens_ttf_and_hashes_raw_bytes() {
    let raw = fixture_bytes("roboto.ttf");
    let f = FontFile::open(&fixture("roboto.ttf")).unwrap();
    assert_eq!(f.container(), Container::Sfnt);
    assert_eq!(f.hash(), &blake3::hash(&raw));
    assert_eq!(f.hash_hex().len(), 64);
}

#[test]
fn rejects_garbage_and_tiny_input() {
    let e = FontFile::from_bytes(b"hello world, not a font".to_vec()).unwrap_err();
    assert_eq!(e.code(), "not_a_font");
    let e = FontFile::from_bytes(vec![0, 1, 0]).unwrap_err();
    assert_eq!(e.code(), "not_a_font");
}

#[test]
fn missing_file_is_io_error() {
    let e = FontFile::open(std::path::Path::new("/nonexistent/x.ttf")).unwrap_err();
    assert!(matches!(e, CoreError::Io(_)));
    assert_eq!(e.code(), "io");
}

#[test]
fn decompresses_woff1_and_woff2_but_hashes_original() {
    for (name, kind) in [
        ("open_sans.woff", Container::Woff1),
        ("open_sans.woff2", Container::Woff2),
    ] {
        let raw = fixture_bytes(name);
        let f = FontFile::open(&fixture(name)).unwrap();
        assert_eq!(f.container(), kind, "{name}");
        assert_eq!(f.hash(), &blake3::hash(&raw), "{name}: băm file gốc");
        assert_eq!(
            &f.data()[0..4],
            &[0, 1, 0, 0],
            "{name}: dữ liệu đã giải nén là SFNT"
        );
    }
}

#[test]
fn corrupt_woff2_is_woff_decode() {
    let mut raw = fixture_bytes("open_sans.woff2");
    raw.truncate(200);
    let e = FontFile::from_bytes(raw).unwrap_err();
    assert_eq!(e.code(), "woff_decode");
}

#[test]
fn single_font_has_one_face_index_zero() {
    let f = FontFile::open(&fixture("roboto.ttf")).unwrap();
    let faces = f.faces().unwrap();
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].index(), 0);
}

#[test]
fn samples_validate_ok() {
    for s in SAMPLES {
        FontFile::open(&fixture(s))
            .unwrap()
            .validate()
            .unwrap_or_else(|e| panic!("{s}: {e}"));
    }
}

#[test]
fn truncated_file_is_detected() {
    let mut raw = fixture_bytes("roboto.ttf");
    raw.truncate(5000);
    let f = FontFile::from_bytes(raw).unwrap();
    let e = f.validate().unwrap_err();
    assert_eq!(e.code(), "truncated", "{e}");
}

#[test]
fn missing_head_is_detected() {
    let raw = rename_table(fixture_bytes("roboto.ttf"), b"head", b"heae");
    let f = FontFile::from_bytes(raw).unwrap();
    let e = f.validate().unwrap_err();
    assert_eq!(e.code(), "missing_table");
    assert!(e.to_string().contains("head"));
}

#[test]
fn ttc_faces_have_distinct_indices() {
    let Some(p) = system_font(&["/System/Library/Fonts/Helvetica.ttc"]) else {
        return;
    };
    let f = FontFile::open(&p).unwrap();
    assert_eq!(f.container(), Container::Collection);
    f.validate().unwrap();
    let idx: Vec<u32> = f.faces().unwrap().iter().map(|x| x.index()).collect();
    assert!(idx.len() > 1);
    assert_eq!(idx, (0..idx.len() as u32).collect::<Vec<_>>());
}
