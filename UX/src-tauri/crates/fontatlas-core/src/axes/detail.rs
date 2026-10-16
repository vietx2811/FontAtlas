//! Trục chi tiết mới: aperture 'e' (thiết kế lại), a/g một hay hai tầng, khoảng chữ.
//! Đầu nét (terminal) hoãn sang plan sau — xem docs/team/plans/plan1-report.md.
use crate::geom::{GlyphCtx, vline};
use crate::metric::{FailReason, Metric, Quality};

/// Khe hở nhỏ nhất (theo chiều dọc) giữa thanh ngang 'e' và đầu nét dưới, ÷ x-height.
/// Thanh ngang = đoạn mực thứ 2 trên đường dọc qua tâm 'e'. Quét x từ 55% đến 95% bề rộng;
/// khe = đáy thanh ngang − đỉnh đoạn mực cao nhất nằm dưới nó (hoặc đáy bbox nếu không có).
pub fn aperture_e_xh(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "e_min_vertical_gap_bar_to_terminal";
    let Some(g) = ctx.glyph('e', true) else { return Metric::failed(FailReason::MissingGlyph, M) };
    let center = vline(&g, g.bbox.center().x);
    let Some(bar) = (center.len() >= 3).then(|| center[1]) else {
        return Metric::failed(FailReason::Degenerate, M);
    };
    let gap = (55..=95)
        .step_by(5)
        .map(|p| g.bbox.x0 + g.bbox.width() * p as f64 / 100.0)
        .map(|x| {
            let below = vline(&g, x).into_iter().filter(|r| r.1 <= bar.0 + 1.0).map(|r| r.1).fold(g.bbox.y0, f64::max);
            bar.0 - below
        })
        .filter(|v| *v > 0.0)
        .fold(f64::INFINITY, f64::min);
    // Chưa kiểm định với nhãn tay → Estimated (spec 4.4).
    Metric::from_opt(gap.is_finite().then(|| gap / xh), Quality::Estimated, FailReason::Degenerate, M)
}

/// Số đoạn mực trên đường dọc qua tâm bbox: 'a' hai tầng cắt ≥ 3 (đáy, đỉnh bát, móc trên).
pub fn a_storeys(ctx: &GlyphCtx) -> Metric {
    const M: &str = "a_center_vertical_runs";
    let Some(g) = ctx.glyph('a', true) else { return Metric::failed(FailReason::MissingGlyph, M) };
    let n = vline(&g, g.bbox.center().x).len();
    Metric::measured(if n >= 3 { 2.0 } else { 1.0 }, M)
}

/// 'g' hai tầng (binocular) cắt ≥ 4 đoạn: bát trên (2), cuống nối, vòng dưới.
pub fn g_storeys(ctx: &GlyphCtx) -> Metric {
    const M: &str = "g_center_vertical_runs";
    let Some(g) = ctx.glyph('g', true) else { return Metric::failed(FailReason::MissingGlyph, M) };
    let n = vline(&g, g.bbox.center().x).len();
    Metric::measured(if n >= 4 { 2.0 } else { 1.0 }, M)
}

/// Khoảng trắng hai bên trung bình (advance − bề rộng bbox) của a–z, ÷ x-height.
pub fn spacing_lc_xh(ctx: &GlyphCtx, xh: f64) -> Metric {
    const M: &str = "mean_sidebearings_a_z_over_xh";
    let v: Vec<f64> = ('a'..='z').filter_map(|c| ctx.glyph(c, false)).map(|g| (g.adv - g.bbox.width()).max(0.0)).collect();
    Metric::from_opt((!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64 / xh), Quality::Measured, FailReason::MissingGlyph, M)
}

/// Hệ số biến thiên advance a–z: monospace ≈ 0.
pub fn width_cv(ctx: &GlyphCtx) -> Metric {
    const M: &str = "cv_advance_a_z";
    let v: Vec<f64> = ('a'..='z').filter_map(|c| ctx.glyph(c, false)).map(|g| g.adv).collect();
    if v.len() < 5 {
        return Metric::failed(FailReason::MissingGlyph, M);
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64;
    Metric::measured(var.sqrt() / mean, M)
}
