mod common;
use common::*;
use fontatlas_core::{FontFile, ViStatus, vi_required_chars};

fn vi(name: &str) -> fontatlas_core::VietnameseCoverage {
    FontFile::open(&fixture(name)).unwrap().faces().unwrap()[0].vietnamese()
}

#[test]
fn required_set_has_186_unique_chars() {
    let mut v = vi_required_chars();
    assert_eq!(v.len(), 186);
    v.sort();
    v.dedup();
    assert_eq!(v.len(), 186);
    for c in ['ạ', 'Ỵ', 'đ', 'Đ', 'ơ', 'Ư', 'ẫ'] {
        assert!(v.contains(&c), "{c}");
    }
}

#[test]
fn spike_verified_statuses() {
    let r = vi("roboto.ttf");
    assert_eq!(r.status, ViStatus::PrecomposedOnly);
    assert!(r.supports_vietnamese && r.missing.is_empty());
    for s in [
        "open_sans.ttf",
        "playfair_display.ttf",
        "source_code_pro.ttf",
    ] {
        let v = vi(s);
        assert_eq!(v.status, ViStatus::Full, "{s}");
        assert!(v.gpos_mark && v.gpos_mkmk, "{s}");
    }
    assert!(!vi("playfair_display.ttf").dong_sign);
    assert!(vi("source_code_pro.ttf").dong_sign);
}

#[test]
fn libre_baskerville_lists_missing_chars() {
    let v = vi("libre_baskerville.ttf");
    assert_eq!(v.status, ViStatus::NoneOrPartial);
    assert!(!v.supports_vietnamese);
    assert_eq!(v.missing.len(), 68);
    assert!(v.missing.contains(&'ơ') && v.missing.contains(&'ả'));
    assert!(v.combining_marks_missing.contains(&0x0309));
}

#[test]
fn stacked_marks_are_measured() {
    let r = vi("roboto.ttf");
    let top = r.stacked_marks_top_em.expect("Roboto có Ấ…");
    assert!(top > 0.8, "dấu chồng cao hơn cap-height: {top}");
    assert!(r.stacked_marks_overflow.is_some());
}

#[test]
fn regression_avenir_next_supports_vietnamese() {
    // Quy tắc Python cũ báo sai False (âm tính giả).
    let Some(p) = system_font(&["/System/Library/Fonts/Avenir Next.ttc"]) else {
        return;
    };
    let f = FontFile::open(&p).unwrap();
    // Spike (dữ liệu thật): 10/12 face đủ; riêng Heavy và Heavy Italic thiếu 94 ký tự.
    for face in f.faces().unwrap() {
        let name = face.metadata().full_name.unwrap_or_default();
        let v = face.vietnamese();
        if name.contains("Heavy") {
            assert!(!v.supports_vietnamese, "{name}");
            assert_eq!(v.missing.len(), 94, "{name}");
        } else {
            assert!(v.supports_vietnamese, "{name}: {:?}", v.missing);
        }
    }
}

use fontatlas_core::{Quality, StackStyle, analyze_path};

#[test]
fn stack_style_detected_on_vi_fixtures() {
    for f in [
        "roboto.ttf",
        "open_sans.ttf",
        "playfair_display.ttf",
        "source_code_pro.ttf",
        "public_sans_vf.ttf",
    ] {
        let r = analyze_path(&fixture(f)).unwrap();
        let m = &r.faces[0].vietnamese.marks;
        assert!(m.stack_style.is_some(), "{f}: {m:?}");
        assert_eq!(m.stack_style_quality, Quality::Estimated, "{f}");
    }
}

#[test]
fn stack_style_none_when_glyphs_missing() {
    // Libre Baskerville không có ấ/ầ → không đoán.
    let r = analyze_path(&fixture("libre_baskerville.ttf")).unwrap();
    let m = &r.faces[0].vietnamese.marks;
    assert_eq!(m.stack_style, None);
    assert!(matches!(m.stack_style_quality, Quality::Failed(_)));
}

#[test]
fn mark_weight_ratio_plausible_and_estimated() {
    for f in [
        "roboto.ttf",
        "open_sans.ttf",
        "playfair_display.ttf",
        "source_code_pro.ttf",
    ] {
        let r = analyze_path(&fixture(f)).unwrap();
        let m = r.faces[0].vietnamese.marks.mark_weight_ratio;
        let v = m.value.unwrap_or_else(|| panic!("{f}: {m:?}"));
        assert!((0.2..2.5).contains(&v), "{f}: {v}");
        assert_eq!(m.quality, Quality::Estimated);
        assert_eq!(m.method, "acute_area_over_diag_div_n_stem");
    }
}

#[test]
fn analyzer_version_bumped() {
    // một hằng số duy nhất; nơi khác đọc từ hằng này, không ghi cứng
    assert_ne!(fontatlas_core::ANALYZER_VERSION, "core-0.1.0");
    let r = analyze_path(&fixture("roboto.ttf")).unwrap();
    assert_eq!(r.analyzer_version, fontatlas_core::ANALYZER_VERSION);
}

#[test]
fn vi_marks_measured_at_standard_location() {
    // VF: đo ở wght=400 giống Anatomy; ratio phải giống khi đo lại ở Location wght=400
    let r = analyze_path(&fixture("public_sans_vf.ttf")).unwrap();
    let m = r.faces[0].vietnamese.marks.mark_weight_ratio;
    assert!(m.value.is_some(), "{m:?}");
    assert_eq!(
        r.faces[0]
            .anatomy
            .location
            .iter()
            .find(|(t, _)| t == "wght")
            .map(|x| x.1),
        Some(400.0)
    );
}
