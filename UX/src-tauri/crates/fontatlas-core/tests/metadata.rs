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
    assert!(m.units_per_em >= 16);
    assert_eq!(m.outline_format, OutlineFormat::Glyf);
    assert!(m.num_glyphs > 100);
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
