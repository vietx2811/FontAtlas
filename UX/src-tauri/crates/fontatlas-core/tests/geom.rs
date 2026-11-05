mod common;
use common::*;
use fontatlas_core::geom::{GlyphCtx, ink_runs};
use fontatlas_core::{FailReason, FontFile, Metric, Quality};
use kurbo::{BezPath, Point};

fn square(x0: f64, y0: f64, x1: f64, y1: f64, ccw: bool) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((x0, y0));
    if ccw {
        p.line_to((x1, y0));
        p.line_to((x1, y1));
        p.line_to((x0, y1));
    } else {
        p.line_to((x0, y1));
        p.line_to((x1, y1));
        p.line_to((x1, y0));
    }
    p.close_path();
    p
}

#[test]
fn single_square_one_run() {
    let p = square(0.0, 0.0, 100.0, 100.0, true);
    let r = ink_runs(&p, Point::new(-10.0, 50.0), Point::new(110.0, 50.0));
    assert_eq!(r.len(), 1);
    assert!((r[0].0 - 10.0).abs() < 1e-6 && (r[0].1 - 110.0).abs() < 1e-6);
}

#[test]
fn overlapping_contours_merge() {
    // Contour chồng nhau (VF không gỡ overlap) → winding vẫn cho một đoạn mực liền.
    let mut p = square(0.0, 0.0, 60.0, 100.0, true);
    p.extend(square(40.0, 0.0, 100.0, 100.0, true));
    let r = ink_runs(&p, Point::new(-10.0, 50.0), Point::new(110.0, 50.0));
    assert_eq!(r.len(), 1, "{r:?}");
}

#[test]
fn counter_gives_two_runs() {
    let mut p = square(0.0, 0.0, 100.0, 100.0, true);
    p.extend(square(30.0, 30.0, 70.0, 70.0, false));
    let r = ink_runs(&p, Point::new(-10.0, 50.0), Point::new(110.0, 50.0));
    assert_eq!(r.len(), 2, "{r:?}");
}

#[test]
fn glyph_ctx_draws_real_outline() {
    let f = FontFile::open(&fixture("roboto.ttf")).unwrap();
    let faces = f.faces().unwrap();
    let ctx = GlyphCtx::new(faces[0].font().clone(), Default::default());
    let o = ctx.glyph('o', true).expect("có o");
    assert!(o.bbox.height() > 0.0 && o.adv > 0.0);
    assert!(
        ctx.glyph('\u{E000}', true).is_none(),
        "ký tự không có trong cmap"
    );
    let caps_only = GlyphCtx::new(faces[0].font().clone(), Default::default())
        .with_char_filter(|c| !c.is_lowercase());
    assert!(caps_only.glyph('o', true).is_none());
    assert!(caps_only.glyph('O', true).is_some());
}

#[test]
fn metric_constructors() {
    let m = Metric::measured(1.23456, "x");
    assert_eq!(m.quality, Quality::Measured);
    assert!((m.value.unwrap() - 1.23456).abs() < 1e-5);
    let e = Metric::failed(FailReason::MissingGlyph, "x");
    assert_eq!(e.value, None);
    assert_eq!(e.quality, Quality::Failed(FailReason::MissingGlyph));
    let nan = Metric::measured(f64::NAN, "x");
    assert_eq!(
        nan.quality,
        Quality::Failed(FailReason::Degenerate),
        "NaN không bao giờ lọt ra ngoài"
    );
}
