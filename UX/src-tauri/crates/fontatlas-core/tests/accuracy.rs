mod common;
use common::*;
use fontatlas_core::axes::stroke::*;
use fontatlas_core::geom::{GlyphCtx, ink_runs};
use fontatlas_core::{FailReason, FontFile, MeasureAt, Quality};
use kurbo::Point;
use read_fonts::TableProvider;

fn table(data: &[u8], tag: &[u8; 4]) -> usize {
    let n = u16::from_be_bytes(data[4..6].try_into().unwrap()) as usize;
    (0..n)
        .find_map(|i| {
            let p = 12 + i * 16;
            (&data[p..p + 4] == tag)
                .then(|| u32::from_be_bytes(data[p + 8..p + 12].try_into().unwrap()) as usize)
        })
        .unwrap()
}
fn stem_tilt(ctx: &GlyphCtx, upright: bool) -> f64 {
    let g = ctx.glyph('l', upright).unwrap();
    let b = g.bbox;
    let mid = |fraction| {
        let y = b.y0 + b.height() * fraction;
        let x = b.x0 - 10.0;
        let r = ink_runs(&g.path, Point::new(x, y), Point::new(b.x1 + 10.0, y));
        x + (r[0].0 + r[0].1) / 2.0
    };
    -((mid(0.75) - mid(0.25)) / (b.height() * 0.5))
        .atan()
        .to_degrees()
}
#[test]
fn zero_post_angle_uses_measured_oblique_stem_to_deskew() {
    let f = FontFile::open(&fixture("roboto_italic_vf.ttf")).unwrap();
    let faces = f.faces().unwrap();
    let face = &faces[0];
    assert!(face.font().post().unwrap().italic_angle().to_f64().abs() < 0.5);
    let ctx = GlyphCtx::new(
        face.font().clone(),
        fontatlas_core::standard_location(face.font()),
    );
    assert!((stem_tilt(&ctx, false) + 9.8).abs() < 0.5);
    assert!(
        stem_tilt(&ctx, true).abs() < 0.05,
        "measured fallback must actually straighten outline"
    );
}
#[test]
fn upright_outline_is_preserved() {
    let f = FontFile::open(&fixture("roboto.ttf")).unwrap();
    let faces = f.faces().unwrap();
    let ctx = GlyphCtx::new(faces[0].font().clone(), Default::default());
    assert_eq!(
        ctx.glyph('o', true).unwrap().path,
        ctx.glyph('o', false).unwrap().path
    );
}
#[test]
fn sans_above_old_stress_threshold_is_now_low_contrast() {
    let f = FontFile::open(&fixture("fira_sans_regular.ttf")).unwrap();
    let faces = f.faces().unwrap();
    let ctx = GlyphCtx::new(faces[0].font().clone(), Default::default());
    let c = contrast_o(&ctx).value.unwrap();
    assert!((1.3..1.7).contains(&c), "{c}");
    assert_eq!(
        stress_deg(&ctx).quality,
        Quality::Failed(FailReason::LowContrast)
    );
    let f = FontFile::open(&fixture("libre_baskerville.ttf")).unwrap();
    assert_eq!(
        f.faces().unwrap()[0]
            .measure(MeasureAt::Standard)
            .stress_deg
            .quality,
        Quality::Estimated,
        "preserve clearly contrasted serif"
    );
}
#[test]
fn condensed_contrast_is_width_corrected_and_stress_is_failed() {
    // Raw radial ratio was 1.94 / 1.91; perpendicular stroke width gives 1.10 / 1.35,
    // matching H stem/bar (1.10 / 1.40) for these low-contrast sans.
    for (name, lo, hi) in [("bebas_neue.ttf", 1.0, 1.25), ("oswald_vf.ttf", 1.25, 1.50)] {
        let f = FontFile::open(&fixture(name)).unwrap();
        let a = f.faces().unwrap()[0].measure(MeasureAt::Standard);
        assert_eq!(a.contrast.quality, Quality::Measured, "{name}");
        let c = a.contrast.value.unwrap();
        assert!((lo..hi).contains(&c), "{name}: {c}");
        assert!(matches!(a.stress_deg.quality, Quality::Failed(_)), "{name}");
        assert!(a.stress_deg.value.is_none());
    }
}
#[test]
fn italic_serif_foot_is_estimated_even_after_deskew() {
    for name in ["roboto_italic_vf.ttf", "libre_baskerville_italic_vf.ttf"] {
        let f = FontFile::open(&fixture(name)).unwrap();
        let a = f.faces().unwrap()[0].measure(MeasureAt::Standard);
        assert_eq!(a.serif_foot.quality, Quality::Estimated, "{name}");
        assert!(a.serif_foot.value.is_some());
    }
    let f = FontFile::open(&fixture("libre_baskerville.ttf")).unwrap();
    assert_eq!(
        f.faces().unwrap()[0]
            .measure(MeasureAt::Standard)
            .serif_foot
            .quality,
        Quality::Measured
    );
}
#[test]
fn zero_stress_is_positive_zero_and_closed_decorative_c_is_degenerate() {
    let f = FontFile::open(&fixture("playfair_display.ttf")).unwrap();
    let a = f.faces().unwrap()[0].measure(MeasureAt::Standard);
    assert_eq!(a.stress_deg.value, Some(0.0));
    assert!(!a.stress_deg.value.unwrap().is_sign_negative());
    let f = FontFile::open(&fixture("miltonian.ttf")).unwrap();
    let a = f.faces().unwrap()[0].measure(MeasureAt::Standard);
    assert_eq!(a.aperture.quality, Quality::Failed(FailReason::Degenerate));
    assert!(a.aperture.value.is_none());
}
#[test]
fn symbol_cmap_cannot_claim_vietnamese_while_explicit_unicode_can() {
    for mixed in [false, true] {
        let mut bytes = fixture_bytes("open_sans.ttf");
        let off = table(&bytes, b"cmap");
        let n = u16::from_be_bytes(bytes[off + 2..off + 4].try_into().unwrap()) as usize;
        for i in 0..n {
            if mixed && i > 0 {
                break;
            }
            let p = off + 4 + i * 8;
            bytes[p..p + 2].copy_from_slice(&3u16.to_be_bytes());
            bytes[p + 2..p + 4].copy_from_slice(&0u16.to_be_bytes());
        }
        let f = FontFile::from_bytes(bytes).unwrap();
        let faces = f.faces().unwrap();
        let font = faces[0].font();
        use skrifa::MetadataProvider;
        assert!(font.charmap().is_symbol());
        let v = faces[0].vietnamese();
        if mixed {
            assert!(
                v.supports_vietnamese,
                "explicit Unicode subtable remains authoritative"
            );
        } else {
            assert_eq!(v.missing.len(), 186);
            assert_eq!(v.precomposed_present, 0);
            assert!(!v.supports_vietnamese && !v.dong_sign);
        }
        assert!(v.stacked_marks_top_em.is_none());
        assert!(v.marks.mark_weight_ratio.value.is_none());
    }
}

#[test]
#[ignore = "local audit only; never redistribute system fonts"]
fn macos_oblique_condensed_and_symbol_regressions() {
    for name in [
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Avenir Next Condensed.ttc",
        "/System/Library/Fonts/Supplemental/DIN Condensed Bold.ttf",
        "/System/Library/Fonts/Supplemental/Webdings.ttf",
    ] {
        let Some(path) = system_font(&[name]) else {
            continue;
        };
        let f = FontFile::open(&path).unwrap();
        for face in f.faces().unwrap() {
            let m = face.metadata();
            let a = face.measure(MeasureAt::Standard);
            if m.postscript_name
                .as_deref()
                .is_some_and(|n| n.contains("Oblique") || n.contains("Italic"))
            {
                assert_eq!(a.serif_foot.quality, Quality::Estimated, "{:?}", m.postscript_name);
                let ctx = GlyphCtx::new(
                    face.font().clone(),
                    fontatlas_core::standard_location(face.font()),
                );
                assert!(stem_tilt(&ctx, true).abs() < 3.0);
            }
            if name.contains("DIN Condensed") {
                assert_eq!(a.contrast.quality, Quality::Measured);
                assert!(matches!(a.stress_deg.quality, Quality::Failed(_)));
            }
            if name.contains("Webdings") {
                assert_eq!(face.vietnamese().missing.len(), 186);
            }
            eprintln!(
                "{} #{} {:?}: stress={:?}, contrast={:?}, serif={:?}",
                name,
                face.index(),
                m.full_name,
                a.stress_deg,
                a.contrast,
                a.serif_foot
            );
        }
    }
}

#[test]
fn upright_script_is_not_deskewed_from_a_curved_l() {
    let f = FontFile::open(&fixture("dancing_script_vf.ttf")).unwrap();
    let faces = f.faces().unwrap();
    let face = &faces[0];
    let ctx = GlyphCtx::new(
        face.font().clone(),
        fontatlas_core::standard_location(face.font()),
    );
    assert!(
        slant_deg(&ctx).value.unwrap().abs() >= 3.0,
        "a curved l alone must not opt a script into deskew"
    );
    assert_eq!(
        ctx.glyph('g', true).unwrap().path,
        ctx.glyph('g', false).unwrap().path
    );
    let a = face.measure(MeasureAt::Standard);
    assert_eq!(
        a.g_storeys.value,
        Some(1.0),
        "preserve audited script heuristic until independently validated"
    );
}
