//! Tương phản nét, trục nhấn, aperture 'c', độ rộng, độ nghiêng, chân serif, counter.
use kurbo::{Point, Shape, Vec2};

use crate::geom::{GlyphCtx, GlyphShape, first_stem, hline, ink_runs, vline};
use crate::metric::{FailReason, Metric, Quality};

/// Quét tia qua tâm mỗi 2°, độ dày theo hướng → (max/min, độ nghiêng trục nhấn).
/// Trục nhấn: khớp điều hoà bậc 2 t(θ) ≈ a + b·cos 2(θ−φ); 0 = đứng, dương = nghiêng trái.
pub(crate) fn contrast_on(g: &GlyphShape) -> Option<(f64, f64)> {
    let c = g.bbox.center();
    let r = g.bbox.width().max(g.bbox.height());
    let mut samples = Vec::new();
    for deg in (0..180).step_by(2) {
        let th = (deg as f64).to_radians();
        let d = Vec2::new(th.cos(), th.sin()) * r;
        let runs = ink_runs(&g.path, c - d, c + d);
        if runs.len() < 2 {
            continue;
        }
        let last = runs[runs.len() - 1];
        let t = ((runs[0].1 - runs[0].0) + (last.1 - last.0)) / 2.0;
        samples.push((deg as f64, t));
    }
    if samples.len() < 45 {
        return None;
    }
    let tmin = samples.iter().map(|s| s.1).fold(f64::MAX, f64::min);
    let tmax = samples.iter().map(|s| s.1).fold(0.0, f64::max);
    if tmin <= 0.0 {
        return None;
    }
    let (mut sx, mut sy) = (0.0, 0.0);
    for (d, t) in &samples {
        let a = 2.0 * d.to_radians();
        sx += t * a.cos();
        sy += t * a.sin();
    }
    let mut tilt = sy.atan2(sx).to_degrees() / 2.0;
    if tilt > 90.0 { tilt -= 180.0; }
    if tilt < -90.0 { tilt += 180.0; }
    Some((tmax / tmin, tilt))
}

pub fn contrast_o(ctx: &GlyphCtx) -> Metric {
    const M: &str = "o_radial_thickness_max_min";
    match ctx.glyph('o', true) {
        None => Metric::failed(FailReason::MissingGlyph, M),
        Some(o) => Metric::from_opt(contrast_on(&o).map(|v| v.0), Quality::Measured, FailReason::Degenerate, M),
    }
}

pub fn contrast_h(ctx: &GlyphCtx) -> Metric {
    const M: &str = "H_stem_over_bar";
    let Some(g) = ctx.glyph('H', true) else { return Metric::failed(FailReason::MissingGlyph, M) };
    let v = (|| {
        let stem = first_stem(&g, g.bbox.y1 * 0.2)?;
        let bar = vline(&g, g.bbox.center().x)
            .into_iter()
            .find(|(a, b)| *a > g.bbox.y1 * 0.2 && *b < g.bbox.y1 * 0.8)
            .map(|(a, b)| b - a)?;
        Some(stem / bar)
    })();
    Metric::from_opt(v, Quality::Measured, FailReason::Degenerate, M)
}

pub fn stress_deg(ctx: &GlyphCtx) -> Metric {
    const M: &str = "o_harmonic_fit";
    let Some(o) = ctx.glyph('o', true) else { return Metric::failed(FailReason::MissingGlyph, M) };
    match contrast_on(&o) {
        None => Metric::failed(FailReason::Degenerate, M),
        Some((c, _)) if c < 1.3 => Metric::failed(FailReason::LowContrast, M),
        // Chưa qua kiểm định (spike: Baskerville ra 0°) → luôn Estimated.
        Some((_, tilt)) => Metric::estimated(tilt.round(), M),
    }
}

/// Số độ (0–360) tia từ `origin` thoát ra ngoài không chạm mực.
pub(crate) fn angular_aperture(g: &GlyphShape, origin: Point) -> Option<f64> {
    if g.path.winding(origin) != 0 {
        return None;
    }
    let r = 3.0 * g.bbox.width().max(g.bbox.height());
    let open = (0..360)
        .filter(|deg| {
            let th = (*deg as f64).to_radians();
            ink_runs(&g.path, origin, origin + Vec2::new(th.cos(), th.sin()) * r).is_empty()
        })
        .count();
    Some(open as f64)
}

/// Điểm giữa khe đầu tiên sau nét trái trên dòng quét y.
pub(crate) fn gap_origin(g: &GlyphShape, y: f64) -> Option<Point> {
    let r = hline(g, y);
    let first = r.first()?;
    let end = r.get(1).map(|n| n.0).unwrap_or(g.bbox.x1);
    (end - first.1 > 1.0).then(|| Point::new((first.1 + end) / 2.0, y))
}

pub fn aperture_c_deg(ctx: &GlyphCtx) -> Metric {
    const M: &str = "c_angular_opening";
    let Some(g) = ctx.glyph('c', true) else { return Metric::failed(FailReason::MissingGlyph, M) };
    let v = gap_origin(&g, g.bbox.center().y).and_then(|o| angular_aperture(&g, o));
    Metric::from_opt(v, Quality::Measured, FailReason::Degenerate, M)
}

pub fn width_o_xh(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "o_bbox_width_over_xh";
    Metric::from_opt(ctx.glyph('o', true).map(|g| g.bbox.width() / xh), Quality::Measured, FailReason::MissingGlyph, M)
}

pub fn mean_adv_lc_xh(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "mean_advance_a_z_over_xh";
    let adv: Vec<f64> = ('a'..='z').filter_map(|c| ctx.glyph(c, false)).map(|g| g.adv).collect();
    let v = (!adv.is_empty()).then(|| adv.iter().sum::<f64>() / adv.len() as f64 / xh);
    Metric::from_opt(v, Quality::Measured, FailReason::MissingGlyph, M)
}

/// Độ nghiêng đo thật (không dựng thẳng) trên 'l' hoặc 'I'; âm = nghiêng phải như italic.
pub fn slant_deg(ctx: &GlyphCtx) -> Metric {
    const M: &str = "l_or_I_midline_offset";
    let v = ctx.glyph('l', false).or_else(|| ctx.glyph('I', false)).and_then(|g| {
        let (y1, y2) = (g.bbox.y0 + g.bbox.height() * 0.25, g.bbox.y0 + g.bbox.height() * 0.75);
        let m1 = hline(&g, y1).first().map(|(a, b)| (a + b) / 2.0)?;
        let m2 = hline(&g, y2).first().map(|(a, b)| (a + b) / 2.0)?;
        Some(-((m2 - m1) / (y2 - y1)).atan().to_degrees())
    });
    Metric::from_opt(v.map(|v| (v * 10.0).round() / 10.0), Quality::Measured, FailReason::MissingGlyph, M)
}

/// Bề rộng mực sát baseline ÷ thân ở 35% xh, trung vị trên 'i','l','n'. Sans ≈ 1, serif ≥ 1.6.
pub fn serif_foot(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "baseline_foot_over_stem_median_iln";
    let mut foot: Vec<f64> = ['i', 'l', 'n']
        .iter()
        .filter_map(|c| ctx.glyph(*c, true))
        .filter_map(|g| Some(first_stem(&g, g.bbox.y0 + xh * 0.025)? / first_stem(&g, xh * 0.35)?))
        .filter(|v| v.is_finite())
        .collect();
    foot.sort_by(f64::total_cmp);
    Metric::from_opt(foot.get(foot.len() / 2).copied(), Quality::Measured, FailReason::MissingGlyph, M)
}

/// Diện tích lòng 'o' ÷ diện tích bao ngoài (tích phân scanline).
pub fn counter_o(ctx: &GlyphCtx) -> Metric {
    const M: &str = "o_inner_over_outer_area";
    let Some(g) = ctx.glyph('o', true) else { return Metric::failed(FailReason::MissingGlyph, M) };
    let (mut inner, mut outer) = (0.0, 0.0);
    let n = 48;
    let dy = g.bbox.height() / n as f64;
    for i in 0..n {
        let r = hline(&g, g.bbox.y0 + dy * (i as f64 + 0.5));
        if let (Some(f), Some(l)) = (r.first(), r.last()) {
            outer += (l.1 - f.0) * dy;
            if r.len() >= 2 {
                inner += (r[1].0 - r[0].1) * dy;
            }
        }
    }
    let v = (outer > 0.0).then(|| inner / outer);
    Metric::from_opt(v, Quality::Measured, FailReason::Degenerate, M)
}
