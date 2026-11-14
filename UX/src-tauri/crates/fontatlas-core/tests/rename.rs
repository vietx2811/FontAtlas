//! G7b: bản sao đổi tên của VF — chỉ bảng `name` đổi, mọi thứ khác nguyên byte, file hợp lệ.
use fontatlas_core::analyze_paths;
use fontatlas_core::rename::{RenameError, rename_vf_copy};
use read_fonts::{FontRef, TableProvider};
use std::path::PathBuf;

fn fixture(n: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(n)
}

fn table_bytes<'a>(f: &FontRef<'a>, tag: &str) -> Vec<u8> {
    use read_fonts::types::Tag;
    f.table_data(Tag::new(tag.as_bytes().try_into().unwrap())).map(|d| d.as_bytes().to_vec()).unwrap_or_default()
}

#[test]
fn oswald_vf_copy_has_new_names_unchanged_glyph_data_and_valid_checksums() {
    let orig = std::fs::read(fixture("oswald_vf.ttf")).unwrap();
    let r = rename_vf_copy(&orig).unwrap();
    assert_eq!((r.family.as_str(), r.postscript_name.as_str()), ("Oswald VF", "OswaldVF-Regular"));

    // đọc lại bằng chính bộ phân tích của app
    let tmp = std::env::temp_dir().join(format!("fa-rename-{}.ttf", std::process::id()));
    std::fs::write(&tmp, &r.bytes).unwrap();
    let rep = analyze_paths(std::slice::from_ref(&tmp)).pop().unwrap().1.unwrap();
    let _ = std::fs::remove_file(&tmp);
    let m = &rep.faces[0].metadata;
    assert_eq!(m.typographic_family.as_deref().or(m.family.as_deref()), Some("Oswald VF"));
    assert!(m.postscript_name.as_deref().is_some_and(|p| p.starts_with("OswaldVF-")), "{:?}", m.postscript_name);
    assert!(m.full_name.as_deref().is_some_and(|f| f.starts_with("Oswald VF")), "{:?}", m.full_name);
    assert!(m.axes.iter().any(|a| a.tag == "wght"), "trục biến thiên còn nguyên");
    let orig_rep = {
        let t = std::env::temp_dir().join(format!("fa-rename-o-{}.ttf", std::process::id()));
        std::fs::write(&t, &orig).unwrap();
        let rr = analyze_paths(std::slice::from_ref(&t)).pop().unwrap().1.unwrap();
        let _ = std::fs::remove_file(&t);
        rr
    };
    assert_eq!(m.named_instances.len(), orig_rep.faces[0].metadata.named_instances.len());
    assert_ne!(rep.hash_hex, orig_rep.hash_hex, "file khác byte ⇒ hash mới");

    // bảng khác nguyên byte
    let (a, b) = (FontRef::new(&orig).unwrap(), FontRef::new(&r.bytes).unwrap());
    for tag in ["glyf", "gvar", "fvar", "cmap", "hmtx", "head", "OS/2", "loca", "maxp", "HVAR", "STAT", "post"] {
        let (x, y) = (table_bytes(&a, tag), table_bytes(&b, tag));
        if tag == "head" {
            assert_eq!(x.len(), y.len());
            assert_eq!(&x[..8], &y[..8]); // checkSumAdjustment (8..12) được tính lại
            assert_eq!(&x[12..], &y[12..]);
        } else {
            assert_eq!(x, y, "bảng {tag} phải nguyên byte");
        }
    }
    assert_ne!(table_bytes(&a, "name"), table_bytes(&b, "name"));
    // PostScript name của named instance (nếu font có) cũng được đổi, và khác bản gốc
    {
        use skrifa::MetadataProvider;
        let ps_of = |bytes: &[u8]| -> Vec<String> {
            let f = skrifa::FontRef::new(bytes).unwrap();
            f.named_instances()
                .iter()
                .filter_map(|i| i.postscript_name_id())
                .filter_map(|id| f.localized_strings(id).english_or_first().map(|s| s.to_string()))
                .collect()
        };
        let (po, pn) = (ps_of(&orig), ps_of(&r.bytes));
        assert_eq!(po.len(), pn.len());
        for (o, n) in po.iter().zip(&pn) {
            assert_ne!(o, n);
            assert!(n.contains("VF"), "{o} → {n}");
        }
    }
    // toàn file cộng lại = 0xB1B0AFBA (quy tắc checkSumAdjustment)
    let mut sum = 0u32;
    for c in r.bytes.chunks(4) {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        sum = sum.wrapping_add(u32::from_be_bytes(w));
    }
    assert_eq!(sum, 0xB1B0_AFBA);
    // file gốc không bị đổi
    assert_eq!(orig, std::fs::read(fixture("oswald_vf.ttf")).unwrap());
    // áp lần hai: tên không bị thêm VF lần nữa
    let again = rename_vf_copy(&r.bytes).unwrap();
    assert_eq!((again.family.as_str(), again.postscript_name.as_str()), ("Oswald VF", "OswaldVF-Regular"));
}

#[test]
fn only_variable_sfnt_is_accepted() {
    let stat = std::fs::read(fixture("roboto.ttf")).unwrap();
    assert!(matches!(rename_vf_copy(&stat), Err(RenameError::Malformed(_))), "static không có fvar");
    let woff = std::fs::read(fixture("open_sans.woff")).unwrap();
    assert!(matches!(rename_vf_copy(&woff), Err(RenameError::Unsupported("woff"))));
    let woff2 = std::fs::read(fixture("open_sans.woff2")).unwrap();
    assert!(matches!(rename_vf_copy(&woff2), Err(RenameError::Unsupported("woff2"))));
    assert!(matches!(rename_vf_copy(b"ttcf\0\0\0\0"), Err(RenameError::Unsupported("ttc"))));
    assert!(matches!(rename_vf_copy(b"junk"), Err(RenameError::Malformed(_))));
}

#[test]
fn ps_name_rule_inserts_vf_into_the_family_part() {
    use fontatlas_core::rename::ps_name;
    assert_eq!(ps_name("Oswald-Regular"), "OswaldVF-Regular");
    assert_eq!(ps_name("OswaldVF-Regular"), "OswaldVF-Regular");
    assert_eq!(ps_name("Oswald"), "OswaldVF");
    assert_eq!(ps_name("Open-Sans-Bold"), "OpenVF-Sans-Bold");
}
