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
    if tilt > 90.0 {
        tilt -= 180.0;
    }
    if tilt < -90.0 {
        tilt += 180.0;
    }
    Some((tmax / tmin, tilt))
}

pub fn contrast_o(ctx: &GlyphCtx) -> Metric {
    const M: &str = "o_stroke_width_max_min";
    match ctx.glyph('o', true) {
        None => Metric::failed(FailReason::MissingGlyph, M),
        Some(o) => contrast_metric(ctx, &o, M),
    }
}

/// Perpendicular stroke-width contrast; radial (width-corrected) fallback when the glyph
/// is not a plain two-contour ring (stencil, overlapping VF contours).
pub(crate) fn contrast_metric(ctx: &GlyphCtx, o: &GlyphShape, method: &'static str) -> Metric {
    match contrast_perpendicular(o) {
        Some(c) => Metric::from_opt(Some(c), Quality::Measured, FailReason::Degenerate, method),
        None => Metric::from_opt(
            contrast_narrow_aware(o),
            contrast_quality(ctx, o),
            FailReason::Degenerate,
            method,
        ),
    }
}


/// Polylines of each closed contour (flattened, tolerance in font units).
pub(crate) fn contour_polys(path: &kurbo::BezPath, tol: f64) -> Vec<Vec<Point>> {
    let mut out: Vec<Vec<Point>> = Vec::new();
    let mut cur: Vec<Point> = Vec::new();
    kurbo::flatten(path.iter(), tol, |el| match el {
        kurbo::PathEl::MoveTo(p) => {
            if cur.len() >= 3 { out.push(std::mem::take(&mut cur)); } else { cur.clear(); }
            cur.push(p);
        }
        kurbo::PathEl::LineTo(p) => cur.push(p),
        kurbo::PathEl::ClosePath => {
            if cur.len() >= 3 { out.push(std::mem::take(&mut cur)); } else { cur.clear(); }
        }
        _ => {}
    });
    if cur.len() >= 3 { out.push(cur); }
    out
}

fn poly_area(p: &[Point]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i].x * p[(i + 1) % n].y - p[(i + 1) % n].x * p[i].y).sum::<f64>() / 2.0
}

fn seg_dist(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.hypot2().max(1e-12)).clamp(0.0, 1.0);
    (p - (a + ab * t)).hypot()
}

/// Typographic stroke contrast of an o: local stroke width = distance from the outer
/// contour to the nearest point of the counter (perpendicular to the stroke on smooth
/// curves); contrast = thickest / thinnest, each taken as the mean of the extreme 3%
/// of outer samples to resist single-node noise. Independent of o width and stress.
pub(crate) fn contrast_perpendicular(o: &GlyphShape) -> Option<f64> {
    let mut polys = contour_polys(&o.path, 0.25);
    if polys.len() != 2 {
        return None;
    }
    polys.sort_by(|a, b| poly_area(b).abs().total_cmp(&poly_area(a).abs()));
    let (outer, inner) = (&polys[0], &polys[1]);
    // a real counter sits inside the outer contour (stencil o = two side-by-side halves)
    let bb = |p: &[Point]| p.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |b, q| (b.0.min(q.x), b.1.min(q.y), b.2.max(q.x), b.3.max(q.y)));
    let (ob, ib) = (bb(outer), bb(inner));
    if !(ib.0 > ob.0 && ib.1 > ob.1 && ib.2 < ob.2 && ib.3 < ob.3) {
        return None;
    }
    // resample outer evenly by arc length
    let per: f64 = (0..outer.len()).map(|i| (outer[(i + 1) % outer.len()] - outer[i]).hypot()).sum();
    let n = 720usize;
    let step = per / n as f64;
    let mut samples = Vec::with_capacity(n);
    let (mut acc, mut i, mut target) = (0.0, 0usize, 0.0);
    while samples.len() < n && i < outer.len() * 2 {
        let (a, b) = (outer[i % outer.len()], outer[(i + 1) % outer.len()]);
        let len = (b - a).hypot();
        while target <= acc + len && samples.len() < n {
            let t = if len > 0.0 { (target - acc) / len } else { 0.0 };
            samples.push(a + (b - a) * t);
            target += step;
        }
        acc += len;
        i += 1;
    }
    let mut w: Vec<f64> = samples
        .iter()
        .map(|&p| {
            (0..inner.len())
                .map(|j| seg_dist(p, inner[j], inner[(j + 1) % inner.len()]))
                .fold(f64::MAX, f64::min)
        })
        .collect();
    w.sort_by(f64::total_cmp);
    let k = (w.len() * 3 / 100).max(1);
    let thin = w[..k].iter().sum::<f64>() / k as f64;
    let thick = w[w.len() - k..].iter().sum::<f64>() / k as f64;
    (thin > 0.0).then(|| thick / thin)
}

/// Below this o width/height, radial rays cut the side strokes obliquely and inflate
/// contrast (synthetic o, true 2.00: aspect 0.73 → 2.00, 0.53 → 2.26, 0.40 → 2.92).
pub(crate) const NARROW_O_ASPECT: f64 = 0.7;

/// Contrast with narrow-o correction: stretch x so the o is round, measure, then divide
/// by the stretch (horizontal thickness scales by k, vertical does not). Exact on
/// synthetic axis-aligned o; unchanged for o with aspect >= NARROW_O_ASPECT.
pub(crate) fn contrast_narrow_aware(o: &GlyphShape) -> Option<f64> {
    let (w, h) = (o.bbox.width(), o.bbox.height());
    if w <= 0.0 || h <= 0.0 || w / h >= NARROW_O_ASPECT {
        return contrast_on(o).map(|v| v.0);
    }
    let k = h / w;
    let path = kurbo::Affine::scale_non_uniform(k, 1.0) * o.path.clone();
    let g = GlyphShape { bbox: path.bounding_box(), path, adv: o.adv * k };
    contrast_on(&g).map(|(c, _)| c / k)
}

/// Narrow o is now corrected by `contrast_narrow_aware` (validated on synthetic o down to
/// aspect 0.30 and on Google Fonts condensed/regular siblings: mean error 7.2% → 4.1%).
/// Below 0.30 the correction is untested, so confidence drops to Estimated.
pub(crate) fn contrast_quality(_ctx: &GlyphCtx, o: &GlyphShape) -> Quality {
    if o.bbox.width() / o.bbox.height() < 0.3 {
        Quality::Estimated
    } else {
        Quality::Measured
    }
}

pub fn contrast_h(ctx: &GlyphCtx) -> Metric {
    const M: &str = "H_stem_over_bar";
    let Some(g) = ctx.glyph('H', true) else {
        return Metric::failed(FailReason::MissingGlyph, M);
    };
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
    let Some(o) = ctx.glyph('o', true) else {
        return Metric::failed(FailReason::MissingGlyph, M);
    };
    // Gate on width-corrected contrast so a narrow low-contrast o (Bebas: raw 1.94,
    // corrected 1.27) does not pass as a stressed design.
    match contrast_on(&o).map(|(c, t)| {
        let c = contrast_perpendicular(&o).or_else(|| contrast_narrow_aware(&o)).unwrap_or(c);
        (c, t)
    }) {
        None => Metric::failed(FailReason::Degenerate, M),
        // Audit: regular sans contrast 1.08..1.35; transitional minimum 1.89.
        // 1.7 leaves a margin on both groups; this is a confidence gate, not a classifier.
        Some((c, _)) if c < 1.7 => Metric::failed(FailReason::LowContrast, M),
        // Narrow o can flip the fitted stress to ±90° (Bebas/DIN in audit).
        Some((_, tilt)) if tilt.abs() > 45.0 => Metric::failed(FailReason::Degenerate, M),
        // Harmonic stress remains Estimated; canonicalize -0.0 after rounding.
        Some((_, tilt)) => Metric::estimated(tilt.round() + 0.0, M),
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
    (open > 0).then_some(open as f64)
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
    let Some(g) = ctx.glyph('c', true) else {
        return Metric::failed(FailReason::MissingGlyph, M);
    };
    let v = gap_origin(&g, g.bbox.center().y).and_then(|o| angular_aperture(&g, o));
    Metric::from_opt(v, Quality::Measured, FailReason::Degenerate, M)
}

pub fn width_o_xh(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "o_bbox_width_over_xh";
    Metric::from_opt(
        ctx.glyph('o', true).map(|g| g.bbox.width() / xh),
        Quality::Measured,
        FailReason::MissingGlyph,
        M,
    )
}

pub fn mean_adv_lc_xh(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "mean_advance_a_z_over_xh";
    let adv: Vec<f64> = ('a'..='z')
        .filter_map(|c| ctx.glyph(c, false))
        .map(|g| g.adv)
        .collect();
    let v = (!adv.is_empty()).then(|| adv.iter().sum::<f64>() / adv.len() as f64 / xh);
    Metric::from_opt(v, Quality::Measured, FailReason::MissingGlyph, M)
}

/// Độ nghiêng đo thật (không dựng thẳng) trên 'l' hoặc 'I'; âm = nghiêng phải như italic.
pub fn slant_deg(ctx: &GlyphCtx) -> Metric {
    const M: &str = "I_T_stem_midline";
    let v = ctx.stem_slant_deg();
    Metric::from_opt(
        v.map(|v| (v * 10.0).round() / 10.0 + 0.0),
        Quality::Measured,
        FailReason::MissingGlyph,
        M,
    )
}

/// Dải chiều cao (tỷ lệ x-height tính từ đáy glyph) mà chân được đo: đủ cao cho chân có bracket/flare,
/// đủ thấp để không chạm vào thân/vai chữ.
/// Đáy = `bbox.y0` của glyph (không phải baseline 0): chân nhọn/đuôi nằm DƯỚI baseline (Annam Neue: -20) cần được đo từ điểm thấp nhất,
/// còn chân phẳng thì y0 = 0.
const FOOT_BAND: (f64, f64) = (0.0125, 0.15);
const FOOT_STEPS: usize = 12;

/// Độ loe ĐỐI XỨNG lớn nhất của nét đầu tiên trong dải chân ÷ thân ở 35% xh. Sans ≈ 1, serif chân nhỏ ≈ 1.3–1.6,
/// slab/serif rõ ≥ 1.6. Lấy cực đại trong dải (không lấy một dòng quét sát baseline): chân nhọn/bracket
/// có bề rộng gần 0 ngay tại baseline nhưng loe ra trong vài % x-height tiếp theo. Chỉ phần loe có ở CẢ HAI bên
/// thân được tính (`thân + 2 × min(trái, phải)`): nét vào/ra một phía của chữ viết tay không phải chân serif.
pub(crate) fn foot_ratio(g: &GlyphShape, xh: f64) -> Option<f64> {
    let (sa, sb) = hline(g, xh * 0.35).first().copied()?;
    let stem = sb - sa;
    if stem <= 0.0 {
        return None;
    }
    let peak = (0..FOOT_STEPS)
        .filter_map(|k| {
            let f = FOOT_BAND.0 + (FOOT_BAND.1 - FOOT_BAND.0) * k as f64 / (FOOT_STEPS - 1) as f64;
            let (a, b) = hline(g, g.bbox.y0 + xh * f).first().copied()?;
            Some(stem + 2.0 * (sa - a).max(0.0).min((b - sb).max(0.0)))
        })
        .fold(None, |m: Option<f64>, w| Some(m.map_or(w, |m| m.max(w))))?;
    let r = peak / stem;
    r.is_finite().then_some(r)
}

/// Bề rộng mực sát baseline ÷ thân ở 35% xh, trung vị trên 'i','l','n'. Sans ≈ 1, serif ≥ 1.6.
/// Chỉ một dòng quét ở 2.5% xh: chân nhọn/bracket (rộng ~0 ngay tại baseline) bị đo thấp, xem `serif_flare`.
pub fn serif_foot(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "baseline_foot_over_stem_median_iln";
    let mut foot: Vec<f64> = ['i', 'l', 'n']
        .iter()
        .filter_map(|c| ctx.glyph(*c, true))
        .filter_map(|g| Some(first_stem(&g, g.bbox.y0 + xh * 0.025)? / first_stem(&g, xh * 0.35)?))
        .filter(|v| v.is_finite())
        .collect();
    foot.sort_by(f64::total_cmp);
    Metric::from_opt(foot.get(foot.len() / 2).copied(), foot_quality(ctx), FailReason::MissingGlyph, M)
}

fn foot_quality(ctx: &GlyphCtx) -> Quality {
    if ctx.is_slanted() { Quality::Estimated } else { Quality::Measured }
}

/// Độ loe của chân: trung vị `foot_ratio` (cực đại trong dải chân) trên 'i','l','n'. Bổ sung cho `serif_foot`.
pub fn serif_flare(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "peak_foot_band_over_stem_median_iln";
    let mut v: Vec<f64> = ['i', 'l', 'n'].iter().filter_map(|c| ctx.glyph(*c, true)).filter_map(|g| foot_ratio(&g, xh)).collect();
    v.sort_by(f64::total_cmp);
    Metric::from_opt(v.get(v.len() / 2).copied(), foot_quality(ctx), FailReason::MissingGlyph, M)
}

/// Độ dài cờ serif phía trên-trái của thân (i, l, n) ÷ bề dày thân: phần mực nhô sang trái thân trong dải
/// 3–15% x-height ngay dưới đỉnh. Sans ≈ 0; serif/semi-serif (kể cả chân nhọn không có slab) > 0.25.
pub(crate) fn head_ratio(g: &GlyphShape, top: f64, xh: f64) -> Option<f64> {
    let (sa, sb) = hline(g, xh * 0.35).first().copied()?;
    let stem = sb - sa;
    if stem <= 0.0 {
        return None;
    }
    let ext = [0.03, 0.06, 0.09, 0.12, 0.15]
        .iter()
        .filter_map(|f| hline(g, top - xh * f).first().map(|(a, _)| (sa - a).max(0.0)))
        .fold(0.0_f64, f64::max);
    Some(ext / stem)
}

/// Cờ serif phía trên: trung vị `head_ratio` trên 'i','l','n' (đỉnh = x-height; 'l' = đỉnh glyph).
pub fn serif_head(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "top_left_flag_over_stem_median_iln";
    let mut v: Vec<f64> = ['i', 'l', 'n']
        .iter()
        .filter_map(|c| ctx.glyph(*c, true).map(|g| (*c, g)))
        .filter_map(|(c, g)| head_ratio(&g, if c == 'l' { g.bbox.y1 } else { xh }, xh))
        .collect();
    v.sort_by(f64::total_cmp);
    Metric::from_opt(v.get(v.len() / 2).copied(), foot_quality(ctx), FailReason::MissingGlyph, M)
}

/// Diện tích lòng 'o' ÷ diện tích bao ngoài (tích phân scanline).
pub fn counter_o(ctx: &GlyphCtx) -> Metric {
    const M: &str = "o_inner_over_outer_area";
    let Some(g) = ctx.glyph('o', true) else {
        return Metric::failed(FailReason::MissingGlyph, M);
    };
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

#[cfg(test)]
mod synthetic_stress {
    use super::*;
    use kurbo::{Affine, BezPath, Ellipse};

    /// Thân thẳng (rộng `stem`, cao `h`) ghép chân tuỳ ý: danh sách (độ cao, nửa-bề-rộng thêm hai bên).
    fn stem_with_foot(stem: f64, h: f64, foot: &[(f64, f64)]) -> GlyphShape {
        let c = 300.0;
        let mut right: Vec<(f64, f64)> = vec![(0.0, stem / 2.0 + foot.first().map_or(0.0, |f| f.1))];
        right.extend(foot.iter().map(|&(y, e)| (y, stem / 2.0 + e)));
        right.push((h, stem / 2.0));
        let mut p = BezPath::new();
        p.move_to((c - right[0].1, right[0].0));
        for &(y, hw) in &right {
            p.line_to((c + hw, y));
        }
        for &(y, hw) in right.iter().rev() {
            p.line_to((c - hw, y));
        }
        p.close_path();
        GlyphShape { bbox: p.bounding_box(), path: p, adv: 600.0 }
    }

    /// Phép đo cũ: một dòng quét ở 2.5% xh trên đáy glyph.
    fn old_single_scanline(g: &GlyphShape, xh: f64) -> f64 {
        first_stem(g, g.bbox.y0 + xh * 0.025).unwrap() / first_stem(g, xh * 0.35).unwrap()
    }

    #[test]
    fn foot_ratio_sans_stem_is_exactly_one() {
        let g = stem_with_foot(100.0, 500.0, &[]);
        assert!((foot_ratio(&g, 500.0).unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn foot_ratio_slab_and_bracket_serif_are_clearly_above_sans() {
        // slab: chân hình chữ nhật rộng gấp 2.4 lần thân, cao 8% xh
        let slab = stem_with_foot(100.0, 500.0, &[(40.0, 70.0), (41.0, 0.0)]);
        assert!(foot_ratio(&slab, 500.0).unwrap() > 1.6, "{:?}", foot_ratio(&slab, 500.0));
        // bracket loe dần: 0 → +30 mỗi bên ở 10% xh → ≈ 1.6
        let bracket = stem_with_foot(100.0, 500.0, &[(50.0, 30.0), (100.0, 0.0)]);
        let r = foot_ratio(&bracket, 500.0).unwrap();
        assert!((1.3..=1.7).contains(&r), "{r}");
    }

    #[test]
    fn one_sided_entry_stroke_is_not_a_serif_foot() {
        // nét vào của chữ viết tay: chỉ loe sang trái (Playwrite…), không đối xứng như chân serif
        let xh = 500.0;
        let mut p = BezPath::new();
        let (c, st) = (300.0, 50.0);
        p.move_to((c - st - 120.0, 0.0));
        p.line_to((c + st, 0.0));
        p.line_to((c + st, 500.0));
        p.line_to((c - st, 500.0));
        p.line_to((c - st, xh * 0.1));
        p.close_path();
        let g = GlyphShape { bbox: p.bounding_box(), path: p, adv: 600.0 };
        let r = foot_ratio(&g, xh).unwrap();
        assert!((r - 1.0).abs() < 0.05, "{r}");
    }

    #[test]
    fn pointed_foot_is_not_mistaken_for_a_stem_narrower_than_the_stem() {
        // chân nhọn: đáy gần như một điểm (rộng ~4), loe tới 1.5× thân ở 8% xh rồi thu về thân ở 16%
        // (tái hiện lỗi: Annam Neue/Noirist đo 0.61/0.26 vì chỉ nhìn dòng quét sát baseline)
        let xh = 500.0;
        let mut p = BezPath::new();
        let (c, st) = (300.0, 50.0);
        p.move_to((c - 2.0, 0.0));
        p.line_to((c + 2.0, 0.0));
        p.line_to((c + st * 1.5, xh * 0.08));
        p.line_to((c + st, xh * 0.16));
        p.line_to((c + st, 500.0));
        p.line_to((c - st, 500.0));
        p.line_to((c - st, xh * 0.16));
        p.line_to((c - st * 1.5, xh * 0.08));
        p.close_path();
        let g = GlyphShape { bbox: p.bounding_box(), path: p, adv: 600.0 };
        assert!(old_single_scanline(&g, xh) < 0.6, "lỗi cũ còn tái hiện được: {}", old_single_scanline(&g, xh));
        let r = foot_ratio(&g, xh).unwrap();
        assert!((1.4..=1.6).contains(&r), "{r}");
    }

    /// o tổng hợp: viền ngoài tròn đều, counter là ellipse quay `deg` (dương = nghiêng trái).
    /// Điểm mảnh nhất nằm ở hai đầu trục dài của counter → trục nhấn đúng bằng `deg`.
    fn synthetic_o(deg: f64, inner_rx: f64) -> GlyphShape {
        synthetic_o_w(deg, 300.0, inner_rx)
    }

    fn synthetic_o_w(deg: f64, outer_rx: f64, inner_rx: f64) -> GlyphShape {
        let outer = Ellipse::new((0.0, 0.0), (outer_rx, 300.0), 0.0);
        let inner = Ellipse::new((0.0, 0.0), (inner_rx, 270.0), deg.to_radians());
        let mut path = BezPath::from_vec(outer.path_elements(0.1).collect());
        let hole = BezPath::from_vec(inner.path_elements(0.1).collect()).reverse_subpaths();
        path.extend(hole);
        let path = Affine::translate((outer_rx, 300.0)) * path;
        GlyphShape { bbox: path.bounding_box(), path, adv: 2.0 * outer_rx }
    }

    #[test]
    fn harmonic_fit_recovers_known_stress_angle() {
        for inner_rx in [150.0, 200.0] {
            for deg in [0.0, 10.0, 20.0, 30.0, -15.0] {
                let (c, tilt) = contrast_on(&synthetic_o(deg, inner_rx)).expect("measurable");
                // độ dày ngang 300-rx, dọc 30 → contrast thật (300-rx)/30
                let truth_c = (300.0 - inner_rx) / 30.0;
                assert!((tilt - deg).abs() < 0.5, "rx={inner_rx} truth={deg} got {tilt}");
                assert!((c - truth_c).abs() / truth_c < 0.02, "contrast {c} vs {truth_c}");
                let p = contrast_perpendicular(&synthetic_o(deg, inner_rx)).unwrap();
                assert!((p - truth_c).abs() / truth_c < 0.03, "perpendicular {p} vs {truth_c}");
            }
        }
    }

    #[test]
    fn narrow_o_contrast_is_corrected() {
        // nét ngang 60, nét dọc 30 → contrast thật 2.0 ở mọi độ rộng
        for outer_rx in [300.0, 220.0, 160.0, 120.0, 90.0] {
            let g = synthetic_o_w(0.0, outer_rx, outer_rx - 60.0);
            let raw = contrast_on(&g).unwrap().0;
            let c = contrast_narrow_aware(&g).unwrap();
            eprintln!("aspect={:.2} truth=2.00 raw={raw:.2} corrected={c:.2}", outer_rx / 300.0);
            assert!((c - 2.0).abs() < 0.04, "aspect {} got {c}", outer_rx / 300.0);
            let p = contrast_perpendicular(&g).unwrap();
            assert!((p - 2.0).abs() < 0.06, "perpendicular aspect {} got {p}", outer_rx / 300.0);
        }
    }
}
