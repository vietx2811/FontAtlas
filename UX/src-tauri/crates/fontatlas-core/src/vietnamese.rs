//! Phủ tiếng Việt: dựng sẵn (cmap) vs ghép dấu (dấu kết hợp + GPOS mark/mkmk, GSUB ccmp),
//! và độ cao dấu chồng trên chữ hoa so với ascender.
use read_fonts::{FontRef, TableProvider, tables::glyf::Glyph, types::GlyphId};
use serde::Serialize;
use skrifa::MetadataProvider;

use crate::geom::{GlyphCtx, GlyphShape, first_stem, ink_area};
use crate::metric::{FailReason, Metric, Quality};

/// 67 chữ thường có dấu/đặc thù (12 nguyên âm cơ sở × thanh + ă â ê ô ơ ư đ).
const VI_LOWER: &str = "àáảãạăằắẳẵặâầấẩẫậđèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵ";
/// Dấu kết hợp: huyền, sắc, ngã, hỏi, nặng.
const VI_MARKS: [u32; 5] = [0x0300, 0x0301, 0x0303, 0x0309, 0x0323];
const VI_BASES: &str = "aăâeêioôơuưyAĂÂEÊIOÔƠUƯY";
/// Chữ hoa mang dấu chồng (mũ/trăng/móc + thanh) — cao nhất trong bộ.
const VI_STACKED_UPPER: &str = "ẤẦẨẪẬẮẰẲẴẶẾỀỂỄỆỐỒỔỖỘỚỜỞỠỢỨỪỬỮỰ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViStatus {
    Full,
    PrecomposedOnly,
    MarksOnly,
    NoneOrPartial,
}

#[derive(Debug, Clone, Serialize)]
pub struct VietnameseCoverage {
    pub status: ViStatus,
    /// Đủ dựng sẵn — quyết định hiển thị đúng văn bản NFC thông thường.
    pub supports_vietnamese: bool,
    pub required_total: usize,
    pub missing: Vec<char>,
    pub precomposed_present: usize,
    pub precomposed_composite: usize,
    pub combining_marks_missing: Vec<u32>,
    pub bases_missing: Vec<char>,
    pub gpos_mark: bool,
    pub gpos_mkmk: bool,
    pub gsub_ccmp: bool,
    pub dong_sign: bool,
    /// Đỉnh cao nhất của chữ hoa có dấu chồng, tính theo em.
    pub stacked_marks_top_em: Option<f32>,
    /// Đỉnh dấu chồng vượt hhea.ascender → dễ va dòng khi leading chặt.
    pub stacked_marks_overflow: Option<bool>,
    /// Cách đặt dấu và độ đậm dấu (Estimated cho tới khi kiểm định).
    pub marks: ViMarks,
}

/// Vị trí dấu thanh so với dấu mũ trên 'ấ' (sắc) và 'ầ' (huyền).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StackStyle {
    /// Cả sắc và huyền nằm bên phải mũ.
    Right,
    /// Sắc bên phải, huyền bên trái.
    BothSides,
    /// Cả hai chồng thẳng lên trên mũ.
    Stacked,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ViMarks {
    pub stack_style: Option<StackStyle>,
    pub stack_style_quality: Quality,
    pub mark_weight_ratio: Metric,
}

pub fn vi_required_chars() -> Vec<char> {
    let mut v: Vec<char> = VI_LOWER.chars().collect();
    v.extend(VI_LOWER.chars().flat_map(|c| c.to_uppercase()));
    v.extend('a'..='z');
    v.extend('A'..='Z');
    v
}

fn is_composite(font: &FontRef, gid: GlyphId) -> bool {
    let (Ok(glyf), Ok(loca)) = (font.glyf(), font.loca(None)) else {
        return false;
    };
    matches!(loca.get_glyf(gid, &glyf), Ok(Some(Glyph::Composite(_))))
}

/// Coverage is Unicode semantics, unlike a renderer's symbol cmap fallback.
/// OpenType cmap: prefer full repertoire, then BMP; never Windows encoding 0.
fn unicode_cmap<'a>(font: &FontRef<'a>) -> Option<read_fonts::tables::cmap::CmapSubtable<'a>> {
    use read_fonts::tables::cmap::PlatformId;
    let cmap = font.cmap().ok()?;
    for (platform, encoding) in [
        (PlatformId::Windows, 10),
        (PlatformId::Unicode, 6),
        (PlatformId::Unicode, 4),
        (PlatformId::Windows, 1),
        (PlatformId::Unicode, 3),
        (PlatformId::Unicode, 2),
        (PlatformId::Unicode, 1),
        (PlatformId::Unicode, 0),
    ] {
        for record in cmap.encoding_records() {
            if record.platform_id() == platform
                && record.encoding_id() == encoding
                && let Ok(subtable) = record.subtable(cmap.offset_data())
            {
                return Some(subtable);
            }
        }
    }
    None
}

pub(crate) fn coverage(font: &FontRef) -> VietnameseCoverage {
    let unicode = unicode_cmap(font);
    let map = |c: char| {
        unicode
            .as_ref()
            .and_then(|m| m.map_codepoint(c))
            .filter(|g| g.to_u32() != 0)
    };
    let chars = vi_required_chars();
    let missing: Vec<char> = chars
        .iter()
        .copied()
        .filter(|c| map(*c).is_none())
        .collect();
    let present: Vec<GlyphId> = chars
        .iter()
        .filter(|c| !c.is_ascii())
        .filter_map(|c| map(*c))
        .collect();
    let precomposed_composite = present.iter().filter(|g| is_composite(font, **g)).count();
    let combining_marks_missing: Vec<u32> = VI_MARKS
        .iter()
        .copied()
        .filter(|m| char::from_u32(*m).and_then(&map).is_none())
        .collect();
    let bases_missing: Vec<char> = VI_BASES.chars().filter(|c| map(*c).is_none()).collect();
    let (gsub, gpos) = crate::metadata::feature_tags(font);
    let gpos_mark = gpos.iter().any(|t| t == "mark");
    let gpos_mkmk = gpos.iter().any(|t| t == "mkmk");
    let gsub_ccmp = gsub.iter().any(|t| t == "ccmp");
    let precomposed_ok = missing.is_empty();
    // NFD hiển thị được khi đủ chữ cơ sở + 5 dấu + GPOS mark.
    let decomposed_ok = bases_missing.is_empty() && combining_marks_missing.is_empty() && gpos_mark;
    let status = match (precomposed_ok, decomposed_ok) {
        (true, true) => ViStatus::Full,
        (true, false) => ViStatus::PrecomposedOnly,
        (false, true) => ViStatus::MarksOnly,
        (false, false) => ViStatus::NoneOrPartial,
    };
    let (stacked_marks_top_em, stacked_marks_overflow) = stacked_marks(font);
    VietnameseCoverage {
        status,
        supports_vietnamese: precomposed_ok,
        required_total: chars.len(),
        missing,
        precomposed_present: present.len(),
        precomposed_composite,
        combining_marks_missing,
        bases_missing,
        gpos_mark,
        gpos_mkmk,
        gsub_ccmp,
        dong_sign: map('₫').is_some(),
        stacked_marks_top_em,
        stacked_marks_overflow,
        marks: vi_marks(font),
    }
}

const MARK_RATIO_METHOD: &str = "acute_area_over_diag_div_n_stem";

/// bbox từng contour nằm trên x-height (đáy > 1,05·xh) — coi là dấu.
fn mark_contours(g: &GlyphShape, xh: f64) -> Vec<kurbo::Rect> {
    let mut out = Vec::new();
    let mut cur = kurbo::BezPath::new();
    let mut flush = |p: &mut kurbo::BezPath| {
        if !p.elements().is_empty() {
            let r = kurbo::Shape::bounding_box(&*p);
            if r.y0 > 1.05 * xh && r.width() > 0.0 && r.height() > 0.0 {
                out.push(r);
            }
        }
        *p = kurbo::BezPath::new();
    };
    for el in g.path.elements() {
        if matches!(el, kurbo::PathEl::MoveTo(_)) {
            flush(&mut cur);
        }
        cur.push(*el);
    }
    flush(&mut cur);
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Right,
    Left,
    Stacked,
}

/// Vị trí dấu thanh so với dấu mũ trong một glyph; Err = lý do thất bại.
fn tone_side(ctx: &GlyphCtx, ch: char, xh: f64) -> Result<Side, FailReason> {
    let g = ctx.glyph(ch, true).ok_or(FailReason::MissingGlyph)?;
    let marks = mark_contours(&g, xh);
    if marks.len() < 2 {
        return Err(FailReason::Degenerate);
    }
    // Mũ = contour dấu rộng nhất; thanh = contour còn lại có tâm xa tâm mũ nhất.
    let (hi, hat) = marks
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.width().total_cmp(&b.1.width()))
        .ok_or(FailReason::Degenerate)?;
    let hc = hat.center();
    let tone = marks
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != hi)
        .map(|(_, r)| r)
        .max_by(|a, b| {
            (a.center() - hc)
                .hypot()
                .total_cmp(&(b.center() - hc).hypot())
        })
        .ok_or(FailReason::Degenerate)?;
    let tc = tone.center();
    if tone.y0 >= hat.y1 - 0.1 * hat.height() && (tc.x - hc.x).abs() < 0.5 * hat.width() {
        Ok(Side::Stacked)
    } else if tc.x > hc.x {
        Ok(Side::Right)
    } else {
        Ok(Side::Left)
    }
}

pub(crate) fn vi_marks(font: &FontRef) -> ViMarks {
    // GlyphCtx uses the renderer mapping; do not measure symbol glyphs as Vietnamese marks.
    if font.charmap().is_symbol() {
        return ViMarks {
            stack_style: None,
            stack_style_quality: Quality::Failed(FailReason::MissingGlyph),
            mark_weight_ratio: Metric::failed(FailReason::MissingGlyph, MARK_RATIO_METHOD),
        };
    }
    let ctx = GlyphCtx::new(font.clone(), crate::anatomy::standard_location(font));
    let Some(xh) = ctx.glyph('x', true).map(|g| g.bbox.y1).filter(|v| *v > 0.0) else {
        let f = FailReason::NoLatinLowercase;
        return ViMarks {
            stack_style: None,
            stack_style_quality: Quality::Failed(f),
            mark_weight_ratio: Metric::failed(f, MARK_RATIO_METHOD),
        };
    };
    let (stack_style, stack_style_quality) = match (
        tone_side(&ctx, '\u{1EA5}', xh),
        tone_side(&ctx, '\u{1EA7}', xh),
    ) {
        (Ok(acute), Ok(grave)) => {
            let s = match (acute, grave) {
                (Side::Right, Side::Right) => StackStyle::Right,
                (Side::Right, Side::Left) => StackStyle::BothSides,
                (Side::Stacked, Side::Stacked) => StackStyle::Stacked,
                _ => StackStyle::Other,
            };
            // Chưa kiểm định với nhãn tay → Estimated.
            (Some(s), Quality::Estimated)
        }
        (Err(e), _) | (_, Err(e)) => (None, Quality::Failed(e)),
    };
    ViMarks {
        stack_style,
        stack_style_quality,
        mark_weight_ratio: mark_weight_ratio(&ctx, xh),
    }
}

/// Bề dày dấu sắc trên 'á' (diện tích mực ÷ đường chéo bbox dấu) ÷ thân 'n' ở 35% xh.
fn mark_weight_ratio(ctx: &GlyphCtx, xh: f64) -> Metric {
    let (Some(a), Some(n)) = (ctx.glyph('\u{E1}', true), ctx.glyph('n', true)) else {
        return Metric::failed(FailReason::MissingGlyph, MARK_RATIO_METHOD);
    };
    let marks = mark_contours(&a, xh);
    let Some(r) = marks.into_iter().reduce(|p, q| p.union(q)) else {
        return Metric::failed(FailReason::Degenerate, MARK_RATIO_METHOD);
    };
    let diag = r.width().hypot(r.height());
    let v = first_stem(&n, xh * 0.35)
        .filter(|s| *s > 0.0 && diag > 0.0)
        .map(|stem| ink_area(&a, r.y0, r.y1, 64) / diag / stem);
    Metric::from_opt(
        v,
        Quality::Estimated,
        FailReason::Degenerate,
        MARK_RATIO_METHOD,
    )
}

fn stacked_marks(font: &FontRef) -> (Option<f32>, Option<bool>) {
    if font.charmap().is_symbol() {
        return (None, None);
    }
    // Thiếu head hoặc upm = 0 → không đo được (không mặc định im lặng).
    let Some(upm) = font
        .head()
        .ok()
        .map(|h| h.units_per_em() as f64)
        .filter(|u| *u > 0.0)
    else {
        return (None, None);
    };
    let ctx = GlyphCtx::new(font.clone(), Default::default());
    let top = VI_STACKED_UPPER
        .chars()
        .filter_map(|c| ctx.glyph(c, false))
        .map(|g| g.bbox.y1)
        .max_by(f64::total_cmp);
    let Some(top) = top else { return (None, None) };
    let overflow = font.hhea().ok().map(|h| top > h.ascender().to_i16() as f64);
    (Some((top / upm) as f32), overflow)
}
