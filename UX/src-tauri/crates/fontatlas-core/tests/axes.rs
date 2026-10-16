mod common;
use common::*;
use fontatlas_core::axes::vertical::{darkness_no, stem_n_xh, vertical};
use fontatlas_core::geom::GlyphCtx;
use fontatlas_core::{FontFile, Quality};
use read_fonts::TableProvider;

pub fn with_ctx<R>(path: &std::path::Path, face: usize, f: impl FnOnce(&GlyphCtx) -> R) -> R {
    let file = FontFile::open(path).unwrap();
    let faces = file.faces().unwrap();
    let ctx = GlyphCtx::new(faces[face].font().clone(), Default::default());
    f(&ctx)
}

#[test]
fn x_height_matches_os2_within_2_percent() {
    for s in SAMPLES {
        with_ctx(&fixture(s), 0, |ctx| {
            let v = vertical(ctx).expect(s);
            let os2 = ctx.font().os2().unwrap().sx_height().unwrap() as f64;
            assert!((v.x_height - os2).abs() / os2 < 0.02, "{s}: {} vs {os2}", v.x_height);
            assert!(v.cap_height.unwrap() > v.x_height);
            assert!(v.descender.unwrap() < 0.0);
        });
    }
}

#[test]
fn weight_is_sane_and_measured() {
    for s in SAMPLES {
        with_ctx(&fixture(s), 0, |ctx| {
            let xh = vertical(ctx).unwrap().x_height;
            let st = stem_n_xh(ctx, xh);
            assert_eq!(st.quality, Quality::Measured, "{s}");
            let v = st.value.unwrap();
            assert!((0.05..0.4).contains(&v), "{s}: stem {v}");
            let d = darkness_no(ctx, xh).value.unwrap();
            assert!((0.1..0.8).contains(&d), "{s}: darkness {d}");
        });
    }
}

#[test]
fn helvetica_weights_increase() {
    // Spike: Helvetica Light/Regular/Bold = 0.116 / 0.168 / 0.261.
    let Some(p) = system_font(&["/System/Library/Fonts/Helvetica.ttc"]) else { return };
    let file = FontFile::open(&p).unwrap();
    let mut by_name = std::collections::HashMap::new();
    for face in file.faces().unwrap() {
        if let Some(ps) = face.metadata().postscript_name {
            let ctx = GlyphCtx::new(face.font().clone(), Default::default());
            let xh = vertical(&ctx).unwrap().x_height;
            by_name.insert(ps, stem_n_xh(&ctx, xh).value.unwrap());
        }
    }
    let (l, r, b) = (by_name["Helvetica-Light"], by_name["Helvetica"], by_name["Helvetica-Bold"]);
    assert!(l < r && r < b, "{l} {r} {b}");
}
