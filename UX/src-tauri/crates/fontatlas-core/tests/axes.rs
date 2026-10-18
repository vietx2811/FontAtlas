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
            assert!(
                (v.x_height - os2).abs() / os2 < 0.02,
                "{s}: {} vs {os2}",
                v.x_height
            );
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
    let Some(p) = system_font(&["/System/Library/Fonts/Helvetica.ttc"]) else {
        return;
    };
    let file = FontFile::open(&p).unwrap();
    let mut by_name = std::collections::HashMap::new();
    for face in file.faces().unwrap() {
        if let Some(ps) = face.metadata().postscript_name {
            let ctx = GlyphCtx::new(face.font().clone(), Default::default());
            let xh = vertical(&ctx).unwrap().x_height;
            by_name.insert(ps, stem_n_xh(&ctx, xh).value.unwrap());
        }
    }
    let (l, r, b) = (
        by_name["Helvetica-Light"],
        by_name["Helvetica"],
        by_name["Helvetica-Bold"],
    );
    assert!(l < r && r < b, "{l} {r} {b}");
}

use fontatlas_core::FailReason;
use fontatlas_core::axes::stroke::*;

fn val(m: fontatlas_core::Metric) -> f32 {
    m.value.unwrap_or_else(|| panic!("{:?}", m))
}

#[test]
fn contrast_orders_didone_oldstyle_sans() {
    let c = |s: &str| with_ctx(&fixture(s), 0, |ctx| val(contrast_o(ctx)));
    let (pf, lb, rb, os) = (
        c("playfair_display.ttf"),
        c("libre_baskerville.ttf"),
        c("roboto.ttf"),
        c("open_sans.ttf"),
    );
    assert!(pf > 4.0 && lb > 1.8 && pf > lb, "pf {pf} lb {lb}");
    assert!(rb < 1.5 && os < 1.5, "sans {rb} {os}");
    let h = |s: &str| with_ctx(&fixture(s), 0, |ctx| val(contrast_h(ctx)));
    assert!(h("playfair_display.ttf") > h("roboto.ttf"));
}

#[test]
fn stress_only_with_contrast_and_estimated() {
    with_ctx(&fixture("roboto.ttf"), 0, |ctx| {
        assert_eq!(
            stress_deg(ctx).quality,
            Quality::Failed(FailReason::LowContrast)
        );
    });
    with_ctx(&fixture("playfair_display.ttf"), 0, |ctx| {
        let s = stress_deg(ctx);
        assert_eq!(s.quality, Quality::Estimated);
        assert!(s.value.unwrap().abs() <= 90.0);
    });
}

#[test]
fn aperture_c_humanist_open_grotesque_closed() {
    let a = |s: &str| with_ctx(&fixture(s), 0, |ctx| val(aperture_c_deg(ctx)));
    assert!(a("open_sans.ttf") > a("roboto.ttf") + 30.0);
}

#[test]
fn serif_foot_slant_counter_width() {
    for s in SAMPLES {
        with_ctx(&fixture(s), 0, |ctx| {
            let xh = fontatlas_core::axes::vertical::vertical(ctx)
                .unwrap()
                .x_height;
            let foot = val(serif_foot(ctx, xh));
            let serif = matches!(s, "playfair_display.ttf" | "libre_baskerville.ttf");
            if serif {
                assert!(foot > 1.6, "{s} {foot}")
            } else if s != "source_code_pro.ttf" {
                assert!(foot < 1.25, "{s} {foot}")
            }
            assert!(val(slant_deg(ctx)).abs() < 1.5, "{s} đứng");
            let co = val(counter_o(ctx));
            assert!(co > 0.0 && co < 1.0, "{s}");
            assert!(val(width_o_xh(ctx, xh)) > 0.5 && val(mean_adv_lc_xh(ctx, xh)) > 0.5);
        });
    }
}

#[test]
fn oblique_slant_is_measured() {
    let Some(p) = system_font(&["/System/Library/Fonts/Helvetica.ttc"]) else {
        return;
    };
    let file = FontFile::open(&p).unwrap();
    let face = file
        .faces()
        .unwrap()
        .into_iter()
        .find(|f| f.metadata().postscript_name.as_deref() == Some("Helvetica-Oblique"))
        .unwrap();
    let ctx = GlyphCtx::new(face.font().clone(), Default::default());
    let s = val(slant_deg(&ctx));
    assert!((s + 12.0).abs() < 1.5, "Helvetica Oblique −12°: {s}");
}

use fontatlas_core::axes::detail::*;

#[test]
fn storeys() {
    let a = |s: &str| with_ctx(&fixture(s), 0, |ctx| val(a_storeys(ctx)));
    let g = |s: &str| with_ctx(&fixture(s), 0, |ctx| val(g_storeys(ctx)));
    assert_eq!(a("roboto.ttf"), 2.0);
    assert_eq!(g("roboto.ttf"), 1.0);
    assert_eq!(g("libre_baskerville.ttf"), 2.0);
}

#[test]
fn futura_single_storey_a() {
    let Some(p) = system_font(&["/System/Library/Fonts/Supplemental/Futura.ttc"]) else {
        return;
    };
    with_ctx(&p, 0, |ctx| assert_eq!(val(a_storeys(ctx)), 1.0));
}

#[test]
fn monospace_has_no_width_variation() {
    let cv = |s: &str| with_ctx(&fixture(s), 0, |ctx| val(width_cv(ctx)));
    assert!(cv("source_code_pro.ttf") < 0.02);
    assert!(cv("roboto.ttf") > 0.1);
}

#[test]
fn spacing_positive() {
    for s in SAMPLES {
        with_ctx(&fixture(s), 0, |ctx| {
            let xh = fontatlas_core::axes::vertical::vertical(ctx)
                .unwrap()
                .x_height;
            let v = val(spacing_lc_xh(ctx, xh));
            assert!(v > 0.0 && v < 0.6, "{s} {v}");
        });
    }
}

#[test]
fn aperture_e_estimated_and_humanist_wider() {
    let e = |s: &str| {
        with_ctx(&fixture(s), 0, |ctx| {
            let xh = fontatlas_core::axes::vertical::vertical(ctx)
                .unwrap()
                .x_height;
            aperture_e_xh(ctx, xh)
        })
    };
    let (os, rb) = (e("open_sans.ttf"), e("roboto.ttf"));
    assert_eq!(os.quality, Quality::Estimated);
    assert!(
        os.value.unwrap() > rb.value.unwrap(),
        "Open Sans mở hơn Roboto: {os:?} {rb:?}"
    );
}

#[test]
fn terminal_normal_ball_above_sans() {
    let t = |s: &str| with_ctx(&fixture(s), 0, |ctx| val(terminal_c_normal(ctx)));
    let pf = t("playfair_display.ttf");
    for s in ["roboto.ttf", "open_sans.ttf", "source_code_pro.ttf"] {
        assert!(pf > t(s), "playfair {pf} vs {s} {}", t(s));
    }
    assert!(t("open_sans.ttf") < 1.6);
}

#[test]
fn terminal_normal_is_estimated() {
    let m = with_ctx(&fixture("roboto.ttf"), 0, |ctx| terminal_c_normal(ctx));
    assert_eq!(m.quality, Quality::Estimated);
    assert_eq!(m.method, "c_upper_terminal_normal_over_thin");
}
