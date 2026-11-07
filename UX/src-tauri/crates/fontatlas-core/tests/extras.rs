mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_core::extras::*;

fn extras(file: &str) -> (fontatlas_core::Metadata, FontExtras) {
    let m = FontFile::open(&fixture(file)).unwrap().faces().unwrap()[0].metadata();
    let e = m.extras.clone();
    (m, e)
}

#[test]
fn years_from_longdatetime_and_copyright() {
    // 2020-01-01T00:00:00Z = 1577836800 (unix) + 2082844800 (1904→1970)
    assert_eq!(year_from_longdatetime(1_577_836_800 + 2_082_844_800), Some(2020));
    // đúng ranh giới năm
    assert_eq!(year_from_longdatetime(1_577_836_799 + 2_082_844_800), Some(2019));
    assert_eq!(year_from_longdatetime(0), None);
    assert_eq!(copyright_year("Copyright 2011 Google Inc. 2013, 1850"), Some(2011));
    assert_eq!(copyright_year("(c) 20112 no, 18999 y"), None);
    assert_eq!(copyright_year("© 2008–2020"), Some(2008));
    // ưu tiên số ngay sau © / Copyright, không phải năm sớm nhất ở chỗ khác (số phiên bản, tên dự án…)
    assert_eq!(copyright_year("Rev 1999 build. Copyright (c) 2015 The Authors"), Some(2015));
    assert_eq!(copyright_year("Project Q2 1985 notes; © 2011 Foo"), Some(2011));
}

#[test]
fn fs_type_is_decoded() {
    assert_eq!(decode_fs_type(Some(0)).level, "installable");
    assert_eq!(decode_fs_type(Some(2)).level, "restricted");
    assert_eq!(decode_fs_type(Some(4)).level, "preview_print");
    assert_eq!(decode_fs_type(Some(8)).level, "editable");
    let e = decode_fs_type(Some(8 | 0x100 | 0x200));
    assert_eq!(e.flags, vec!["no_subsetting".to_string(), "bitmap_only".to_string()]);
    assert_eq!(decode_fs_type(None).level, "unknown");
}

#[test]
fn license_kind_by_keywords() {
    assert_eq!(license_kind(Some("This Font Software is licensed under the SIL Open Font License"), None), "ofl");
    assert_eq!(license_kind(None, Some("https://www.apache.org/licenses/LICENSE-2.0")), "apache");
    assert_eq!(license_kind(Some("Ubuntu Font Licence 1.0"), None), "ufl");
    assert_eq!(license_kind(Some("Licensed under the Foundry EULA"), None), "eula");
    assert_eq!(license_kind(Some("Free for personal use"), None), "other");
    assert_eq!(license_kind(None, None), "unknown");
    assert_eq!(license_kind(Some("  "), None), "unknown");
}

#[test]
fn feature_groups_are_readable_and_ss_named() {
    let feats: Vec<String> = ["liga", "dlig", "smcp", "ss01", "ss03", "ss21", "onum", "frac", "cv05", "kern", "case"].iter().map(|s| s.to_string()).collect();
    let g = ot_groups(&feats, &[("ss01".into(), Some("Alt a".into()))]);
    let by = |n: &str| g.iter().find(|x| x.group == n);
    assert_eq!(by("ligatures").unwrap().tags, ["dlig", "liga"]);
    assert_eq!(by("figures").unwrap().tags, ["onum"]);
    assert_eq!(by("small_caps").unwrap().tags, ["smcp"]);
    let ss = by("stylistic_sets").unwrap();
    assert_eq!(ss.tags, ["ss01", "ss03"]); // ss21 ngoài ss01–20
    assert_eq!(ss.sets[0].name.as_deref(), Some("Alt a"));
    assert_eq!(ss.sets[1].name, None);
    assert_eq!(by("character_variants").unwrap().tags, ["cv05"]);
    assert!(by("swash").is_none() && by("fractions").is_some());
}

#[test]
fn fixtures_carry_d2_data() {
    let (m, e) = extras("roboto.ttf");
    assert_eq!(m.vendor_id.as_deref(), Some("GOOG"));
    assert_eq!(e.years.created, Some(2008));
    assert_eq!(e.years.copyright, Some(2011));
    assert_eq!(e.kerning.source, "gpos");
    assert!(e.kerning.pairs > 0 && e.kerning.class_pairs > 0);
    assert_eq!(e.style.italic_kind, "upright");
    assert_eq!(e.embedding.level, "installable");
    assert_eq!(e.license_kind, "ofl");
    // vendor id không chứa NUL
    let (m, _) = extras("playfair_display.ttf");
    assert!(m.vendor_id.as_deref().unwrap().chars().all(|c| !c.is_control()), "{:?}", m.vendor_id);
    // italic thật + tên stylistic set đọc được từ bảng name
    let (_, ital) = extras("libre_baskerville_italic_vf.ttf");
    assert_eq!(ital.style.italic_kind, "italic");
    assert!(ital.style.wght_range.is_some());
    let ss = ital.ot_groups.iter().find(|g| g.group == "stylistic_sets").unwrap();
    assert!(ss.sets.iter().any(|s| s.name.is_some()), "{ss:?}");
    // nhóm figures/small caps có trong Roboto Italic VF
    let (_, ri) = extras("roboto_italic_vf.ttf");
    assert!(ri.ot_groups.iter().any(|g| g.group == "small_caps"));
}

#[test]
fn revision_and_modified_timestamp_are_read_lazily_from_head() {
    assert_eq!(unix_from_longdatetime(0), None);
    assert_eq!(unix_from_longdatetime(2_082_844_800), Some(0));
    assert_eq!(unix_from_longdatetime(1_577_836_800 + 2_082_844_800), Some(1_577_836_800));
    let h = head_info(&fixture("roboto.ttf"), 0).expect("head");
    assert!(h.revision.is_some_and(|r| r > 1.0), "{h:?}");
    let ts = h.modified.expect("head.modified");
    // khớp năm đã lưu trong extras.years.modified
    let (_, e) = extras("roboto.ttf");
    assert_eq!(year_from_longdatetime(ts + 2_082_844_800), e.years.modified);
    // face không tồn tại / tệp không tồn tại
    assert_eq!(head_info(&fixture("roboto.ttf"), 9), None);
    assert_eq!(head_info(&fixture("khong-co.ttf"), 0), None);
}
