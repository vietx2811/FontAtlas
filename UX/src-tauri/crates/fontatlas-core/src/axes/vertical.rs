//! x-height, cap, ascender, descender, độ đậm.
use crate::geom::{GlyphCtx, GlyphShape, first_stem, ink_area};
use crate::metric::{FailReason, Metric};

pub struct Vertical {
    pub x_height: f64,
    pub cap_height: Option<f64>,
    pub ascender: Option<f64>,
    pub descender: Option<f64>,
}

pub fn vertical(ctx: &GlyphCtx) -> Option<Vertical> {
    let xh = ctx
        .glyph('x', true)
        .or_else(|| ctx.glyph('z', true))?
        .bbox
        .y1;
    (xh > 0.0).then(|| Vertical {
        x_height: xh,
        cap_height: ctx.glyph('H', true).map(|g| g.bbox.y1),
        ascender: ctx.glyph('d', true).map(|g| g.bbox.y1),
        descender: ctx.glyph('p', true).map(|g| g.bbox.y0),
    })
}

/// Bề dày thân trái 'n' cắt ngang ở 35% x-height, chia x-height.
pub fn stem_n_xh(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "n_stem_at_35pct_xh";
    let Some(n) = ctx.glyph('n', true) else {
        return Metric::failed(FailReason::MissingGlyph, M);
    };
    Metric::from_opt(
        first_stem(&n, xh * 0.35).map(|s| s / xh),
        crate::Quality::Measured,
        FailReason::Degenerate,
        M,
    )
}

/// Diện tích mực 'n','o' ÷ (advance × x-height), trung bình.
pub fn darkness_no(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "ink_area_n_o";
    let dark = |g: &GlyphShape| ink_area(g, 0.0, xh, 48) / (g.adv * xh);
    let vals: Vec<f64> = ['n', 'o']
        .iter()
        .filter_map(|c| ctx.glyph(*c, true))
        .map(|g| dark(&g))
        .collect();
    if vals.is_empty() {
        return Metric::failed(FailReason::MissingGlyph, M);
    }
    Metric::measured(vals.iter().sum::<f64>() / vals.len() as f64, M)
}
