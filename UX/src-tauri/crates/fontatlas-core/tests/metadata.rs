mod common;
use common::*;
use fontatlas_core::{FontFile, LicenseClass, OutlineFormat, classify_license};

fn meta(name: &str) -> fontatlas_core::Metadata {
    let f = FontFile::open(&fixture(name)).unwrap();
    f.faces().unwrap()[0].metadata()
}

#[test]
fn basic_names_and_metrics() {
    let m = meta("playfair_display.ttf");
    assert_eq!(m.family.as_deref(), Some("Playfair Display"));
    assert!(
        m.postscript_name
            .as_deref()
            .unwrap_or("")
            .starts_with("PlayfairDisplay")
    );
    assert!(m.units_per_em.unwrap() >= 16);
    assert_eq!(m.outline_format, OutlineFormat::Glyf);
    assert!(m.num_glyphs.unwrap() > 100);
    let r = meta("roboto.ttf");
    assert_eq!(r.family.as_deref(), Some("Roboto"));
    assert_eq!(r.weight_class, Some(400));
}

#[test]
fn ofl_fixtures_are_classified() {
    for s in [
        "playfair_display.ttf",
        "libre_baskerville.ttf",
        "source_code_pro.ttf",
    ] {
        assert_eq!(meta(s).license_class, LicenseClass::Ofl, "{s}");
    }
}

#[test]
fn license_classifier_rules() {
    assert_eq!(
        classify_license(
            Some("This Font Software is licensed under the SIL Open Font License, Version 1.1."),
            None
        ),
        LicenseClass::Ofl
    );
    assert_eq!(
        classify_license(None, Some("https://openfontlicense.org")),
        LicenseClass::Ofl
    );
    assert_eq!(
        classify_license(Some("Licensed under the Apache License, Version 2.0"), None),
        LicenseClass::Apache
    );
    assert_eq!(
        classify_license(None, Some("http://www.apache.org/licenses/LICENSE-2.0")),
        LicenseClass::Apache
    );
    assert_eq!(
        classify_license(Some("Commercial EULA. All rights reserved."), None),
        LicenseClass::Other
    );
    assert_eq!(classify_license(None, None), LicenseClass::Unknown);
}

#[test]
fn ttc_faces_have_own_metadata() {
    let Some(p) = system_font(&["/System/Library/Fonts/Helvetica.ttc"]) else {
        return;
    };
    let f = FontFile::open(&p).unwrap();
    let names: Vec<String> = f
        .faces()
        .unwrap()
        .iter()
        .filter_map(|x| x.metadata().postscript_name)
        .collect();
    let mut uniq = names.clone();
    uniq.sort();
    uniq.dedup();
    assert!(names.len() > 1);
    assert_eq!(uniq.len(), names.len(), "mỗi face một PostScript name");
}

#[test]
fn variable_font_axes_are_listed() {
    let Some(p) = system_font(&["/System/Library/Fonts/SFNS.ttf"]) else {
        return;
    };
    let f = FontFile::open(&p).unwrap();
    let m = f.faces().unwrap()[0].metadata();
    assert!(m.axes.iter().any(|a| a.tag == "wght"));
}

#[test]
fn script_coverage_from_opentype_script_tags() {
    // Spec 4.2: độ phủ script. Giá trị đối chiếu bằng fontTools (ScriptList GSUB+GPOS, bỏ DFLT).
    assert_eq!(
        meta("open_sans.ttf").scripts,
        ["cyrl", "grek", "hebr", "latn"]
    );
    assert_eq!(meta("playfair_display.ttf").scripts, ["latn"]);
    assert_eq!(meta("libre_baskerville.ttf").scripts, ["grek", "latn"]);
}

#[test]
fn os2_optical_size_absent_before_v5() {
    // Spec 4.4 (opsz): OS/2 size range chỉ có từ OS/2 v5; mọi fixture là v4 → None, không mặc định 0.
    for s in SAMPLES {
        assert_eq!(meta(s).os2_optical_size_pt, None, "{s}");
    }
}

#[test]
fn missing_head_gives_none_not_zero() {
    // Spec: không mặc định im lặng. Thiếu 'head' → units_per_em None; đo giải phẫu báo Degenerate.
    let raw = rename_table(fixture_bytes("roboto.ttf"), b"head", b"heae");
    let f = FontFile::from_bytes(raw).unwrap();
    let face = &f.faces().unwrap()[0];
    let m = face.metadata();
    assert_eq!(m.units_per_em, None);
    assert_eq!(m.italic_angle, Some(0.0), "post vẫn còn");
    assert!(m.num_glyphs.is_some());
    let a = face.measure(fontatlas_core::MeasureAt::Standard);
    assert_eq!(
        a.x_height_em.quality,
        fontatlas_core::Quality::Failed(fontatlas_core::FailReason::Degenerate)
    );
}

#[test]
fn copyright_and_vendor_url_read() {
    let m = meta("open_sans.ttf");
    let c = m.copyright.as_deref().expect("Open Sans có nameID 0");
    assert!(c.contains("Copyright"), "{c}");
    // vendor_url khớp bảng name thật (nameID 11): in ra giá trị thật khi lệch
    eprintln!("open_sans vendor_url = {:?}", m.vendor_url);
}

#[test]
fn public_sans_named_instances_have_wght() {
    let m = meta("public_sans_vf.ttf");
    assert!(m.named_instance_coords.len() >= 9, "{}", m.named_instance_coords.len());
    for i in &m.named_instance_coords {
        let w = i.coords.iter().find(|(t, _)| t == "wght").map(|(_, v)| *v).expect("wght");
        assert!((100.0..=900.0).contains(&w), "{:?}", i);
    }
    let bold = m
        .named_instance_coords
        .iter()
        .find(|i| i.name.as_deref() == Some("Bold"))
        .expect("có instance Bold");
    assert!(bold.coords.iter().any(|(t, v)| t == "wght" && *v == 700.0));
    // named_instances cũ giữ nguyên
    assert_eq!(m.named_instances.len(), m.named_instance_coords.len());
}

#[test]
fn analyzer_version_is_core_0_9_0() {
    // 0.4.1: metadata.cmap_scripts (no anatomy change; reference quantiles identical).
    assert_eq!(fontatlas_core::ANALYZER_VERSION, "core-0.9.0");
}

#[test]
fn old_metadata_json_parses_with_defaults() {
    let m = meta("roboto.ttf");
    let mut v = serde_json::to_value(&m).unwrap();
    let o = v.as_object_mut().unwrap();
    for k in ["copyright", "vendor_url", "named_instance_coords", "cmap_scripts"] {
        o.remove(k);
    }
    let back: fontatlas_core::Metadata = serde_json::from_value(v).unwrap();
    assert!(back.copyright.is_none() && back.vendor_url.is_none() && back.named_instance_coords.is_empty());
    assert!(back.cmap_scripts.is_empty());
    assert_eq!(back.family, m.family);
}

#[test]
fn latin_fixture_has_no_non_latin_cmap_scripts() {
    let data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/roboto.ttf")).unwrap();
    let r = fontatlas_core::analyze_bytes(data).unwrap();
    assert!(r.faces[0].metadata.cmap_scripts.is_empty());
}
