mod common;
use common::*;
use fontatlas_core::{FailReason, FontFile, MeasureAt, Quality, measure_with_filter};

#[test]
fn samples_have_core_axes_measured() {
    for s in SAMPLES {
        let f = FontFile::open(&fixture(s)).unwrap();
        let a = f.faces().unwrap()[0].measure(MeasureAt::Standard);
        for (name, m) in [
            ("x_height_em", a.x_height_em),
            ("stem", a.stem),
            ("contrast", a.contrast),
            ("width", a.width),
        ] {
            assert_eq!(m.quality, Quality::Measured, "{s} {name}");
        }
        assert!(a.location.is_empty(), "{s}: font tĩnh không có location");
    }
}

#[test]
fn caps_only_font_falls_back_without_panic() {
    let f = FontFile::open(&fixture("playfair_display.ttf")).unwrap();
    let face = &f.faces().unwrap()[0];
    let a = measure_with_filter(
        face.font(),
        MeasureAt::Standard,
        Some(|c: char| !c.is_lowercase()),
    );
    assert_eq!(
        a.x_height_em.quality,
        Quality::Failed(FailReason::NoLatinLowercase)
    );
    assert_eq!(
        a.aperture.quality,
        Quality::Failed(FailReason::NoLatinLowercase)
    );
    assert_eq!(a.cap_height_em.quality, Quality::Measured);
    assert_eq!(a.contrast.quality, Quality::Measured, "đo trên O");
    assert!(
        a.contrast.value.unwrap() > 3.0,
        "Playfair O vẫn tương phản cao"
    );
}

#[test]
fn variable_font_measured_at_standard_location() {
    let Some(p) = system_font(&[
        "/System/Library/Fonts/NewYork.ttf",
        "/System/Library/Fonts/SFNS.ttf",
    ]) else {
        return;
    };
    let f = FontFile::open(&p).unwrap();
    let face = &f.faces().unwrap()[0];
    let std = face.measure(MeasureAt::Standard);
    assert!(
        std.location
            .iter()
            .any(|(t, v)| t == "wght" && (*v - 400.0).abs() < 1.0),
        "{:?}",
        std.location
    );
    let c = std.contrast.value.unwrap();
    assert!(c < 4.0, "đo ở vị trí chuẩn không ra số cực đoan: {c}");
}

#[test]
fn non_latin_or_no_outline_does_not_panic() {
    // Không dùng Hiragino/PingFang: font CJK có kèm Latin nên x-height đo được (đúng).
    for p in [
        "/System/Library/Fonts/Apple Color Emoji.ttc",
        "/System/Library/Fonts/GeezaPro.ttc",
        "/System/Library/Fonts/Supplemental/NISC18030.ttf",
    ] {
        let Some(p) = system_font(&[p]) else { continue };
        let f = FontFile::open(&p).unwrap();
        let face = &f.faces().unwrap()[0];
        let a = face.measure(MeasureAt::Standard);
        assert!(matches!(a.x_height_em.quality, Quality::Failed(_)), "{p:?}");
        assert!(!face.vietnamese().supports_vietnamese, "{p:?}");
        assert!(face.metadata().family.is_some(), "{p:?}: metadata vẫn có");
        if p.ends_with("NISC18030.ttf") {
            assert_eq!(
                a.x_height_em.quality,
                Quality::Failed(FailReason::NoOutlines),
                "font bitmap"
            );
        }
    }
}

#[test]
fn standard_location_is_public_and_consistent() {
    for name in SAMPLES {
        let f = fontatlas_core::FontFile::from_bytes(fixture_bytes(name)).unwrap();
        let faces = f.faces().unwrap();
        let _ = fontatlas_core::standard_location(faces[0].font());
        let a = faces[0].measure(fontatlas_core::MeasureAt::Standard);
        assert_eq!(a.location.is_empty(), faces[0].metadata().axes.is_empty(), "{name}");
    }
}
