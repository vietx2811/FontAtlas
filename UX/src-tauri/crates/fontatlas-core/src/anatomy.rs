//! Gom mọi trục thành Anatomy; chọn vị trí đo cho variable font; nhánh chữ hoa / không outline.
use read_fonts::{FontRef, TableProvider};
use serde::Serialize;
use skrifa::{MetadataProvider, Tag, instance::Location};

use crate::axes::{detail, stroke, vertical};
use crate::geom::GlyphCtx;
use crate::metadata::{OutlineFormat, outline_format};
use crate::metric::{FailReason, Metric, Quality};

#[derive(Debug, Clone, PartialEq)]
pub enum MeasureAt {
    /// wght=400, wdth=100, opsz=14 (kẹp vào khoảng trục), trục khác = mặc định.
    Standard,
    Default,
    Location(Vec<(String, f32)>),
}

#[derive(Debug, Clone, Serialize)]
pub struct Anatomy {
    /// Vị trí đã đo (rỗng với font tĩnh).
    pub location: Vec<(String, f32)>,
    pub x_height_em: Metric,
    pub cap_height_em: Metric,
    pub x_height_cap: Metric,
    pub ascender_em: Metric,
    pub descender_em: Metric,
    pub stem: Metric,
    pub darkness: Metric,
    pub contrast: Metric,
    pub contrast_h: Metric,
    pub stress_deg: Metric,
    pub aperture: Metric,
    pub aperture_e: Metric,
    pub width: Metric,
    pub mean_advance: Metric,
    pub width_cv: Metric,
    pub spacing: Metric,
    pub slant_deg: Metric,
    pub serif_foot: Metric,
    pub counter: Metric,
    pub a_storeys: Metric,
    pub g_storeys: Metric,
}

fn settings(font: &FontRef, at: &MeasureAt) -> Vec<(String, f32)> {
    let axes = font.axes();
    if axes.is_empty() {
        return Vec::new();
    }
    match at {
        MeasureAt::Default => Vec::new(),
        MeasureAt::Location(v) => v.clone(),
        MeasureAt::Standard => [("wght", 400.0f32), ("wdth", 100.0), ("opsz", 14.0)]
            .into_iter()
            .filter_map(|(t, v)| {
                let a = axes.get_by_tag(Tag::new(t.as_bytes().try_into().ok()?))?;
                Some((t.to_string(), v.clamp(a.min_value(), a.max_value())))
            })
            .collect(),
    }
}

fn location(font: &FontRef, set: &[(String, f32)]) -> Location {
    let pairs: Vec<(Tag, f32)> = set
        .iter()
        .filter_map(|(t, v)| Some((Tag::new(t.as_bytes().try_into().ok()?), *v)))
        .collect();
    font.axes().location(pairs)
}

/// Vị trí đo chuẩn (wght=400, wdth=100, opsz=14, có kẹp) — dùng chung cho Anatomy và embedding.
pub fn standard_location(font: &FontRef) -> Location {
    location(font, &settings(font, &MeasureAt::Standard))
}

pub fn measure_with_filter(
    font: &FontRef,
    at: MeasureAt,
    filter: Option<fn(char) -> bool>,
) -> Anatomy {
    let set = settings(font, &at);
    let fail_all = |r: FailReason| all_failed(set.clone(), r);
    if outline_format(font) == OutlineFormat::None {
        return fail_all(FailReason::NoOutlines);
    }
    let mut ctx = GlyphCtx::new(font.clone(), location(font, &set));
    if let Some(f) = filter {
        ctx = ctx.with_char_filter(f);
    }
    // Thiếu head hoặc upm = 0 → không đo được (không mặc định im lặng).
    let Some(upm) = font
        .head()
        .ok()
        .map(|h| h.units_per_em() as f64)
        .filter(|u| *u > 0.0)
    else {
        return fail_all(FailReason::Degenerate);
    };
    let Some(v) = vertical::vertical(&ctx) else {
        return caps_only(&ctx, upm, set);
    };
    let xh = v.x_height;
    let em = |x: Option<f64>, m| {
        Metric::from_opt(
            x.map(|x| x / upm),
            Quality::Measured,
            FailReason::MissingGlyph,
            m,
        )
    };
    Anatomy {
        x_height_em: Metric::measured(xh / upm, "x_bbox_top"),
        cap_height_em: em(v.cap_height, "H_bbox_top"),
        x_height_cap: Metric::from_opt(
            v.cap_height.map(|c| xh / c),
            Quality::Measured,
            FailReason::MissingGlyph,
            "x_over_H",
        ),
        ascender_em: em(v.ascender, "d_bbox_top"),
        descender_em: em(v.descender, "p_bbox_bottom"),
        stem: vertical::stem_n_xh(&ctx, xh),
        darkness: vertical::darkness_no(&ctx, xh),
        contrast: stroke::contrast_o(&ctx),
        contrast_h: stroke::contrast_h(&ctx),
        stress_deg: stroke::stress_deg(&ctx),
        aperture: stroke::aperture_c_deg(&ctx),
        aperture_e: detail::aperture_e_xh(&ctx, xh),
        width: stroke::width_o_xh(&ctx, xh),
        mean_advance: stroke::mean_adv_lc_xh(&ctx, xh),
        width_cv: detail::width_cv(&ctx),
        spacing: detail::spacing_lc_xh(&ctx, xh),
        slant_deg: stroke::slant_deg(&ctx),
        serif_foot: stroke::serif_foot(&ctx, xh),
        counter: stroke::counter_o(&ctx),
        a_storeys: detail::a_storeys(&ctx),
        g_storeys: detail::g_storeys(&ctx),
        location: set,
    }
}

/// Font không có chữ thường: đo cap-height, contrast trên 'O', độ đậm trên thân 'H'.
fn caps_only(ctx: &GlyphCtx, upm: f64, set: Vec<(String, f32)>) -> Anatomy {
    let mut a = all_failed(set, FailReason::NoLatinLowercase);
    if let Some(h) = ctx.glyph('H', true) {
        a.cap_height_em = Metric::measured(h.bbox.y1 / upm, "H_bbox_top");
        a.stem = Metric::from_opt(
            crate::geom::first_stem(&h, h.bbox.y1 * 0.35).map(|s| s / h.bbox.y1),
            Quality::Measured,
            FailReason::Degenerate,
            "H_stem_over_cap",
        );
    }
    if let Some(o) = ctx.glyph('O', true) {
        a.contrast = Metric::from_opt(
            stroke::contrast_on(&o).map(|v| v.0),
            Quality::Measured,
            FailReason::Degenerate,
            "O_radial_thickness_max_min",
        );
    }
    a.contrast_h = stroke::contrast_h(ctx);
    a.slant_deg = stroke::slant_deg(ctx);
    a
}

fn all_failed(location: Vec<(String, f32)>, r: FailReason) -> Anatomy {
    let f = |m| Metric::failed(r, m);
    Anatomy {
        location,
        x_height_em: f("x_bbox_top"),
        cap_height_em: f("H_bbox_top"),
        x_height_cap: f("x_over_H"),
        ascender_em: f("d_bbox_top"),
        descender_em: f("p_bbox_bottom"),
        stem: f("n_stem_at_35pct_xh"),
        darkness: f("ink_area_n_o"),
        contrast: f("o_radial_thickness_max_min"),
        contrast_h: f("H_stem_over_bar"),
        stress_deg: f("o_harmonic_fit"),
        aperture: f("c_angular_opening"),
        aperture_e: f("e_min_vertical_gap_bar_to_terminal"),
        width: f("o_bbox_width_over_xh"),
        mean_advance: f("mean_advance_a_z_over_xh"),
        width_cv: f("cv_advance_a_z"),
        spacing: f("mean_sidebearings_a_z_over_xh"),
        slant_deg: f("l_or_I_midline_offset"),
        serif_foot: f("baseline_foot_over_stem_median_iln"),
        counter: f("o_inner_over_outer_area"),
        a_storeys: f("a_center_vertical_runs"),
        g_storeys: f("g_center_vertical_runs"),
    }
}
