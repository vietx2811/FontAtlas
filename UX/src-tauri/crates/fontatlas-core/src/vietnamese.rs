//! Phủ tiếng Việt: dựng sẵn (cmap) vs ghép dấu (dấu kết hợp + GPOS mark/mkmk, GSUB ccmp),
//! và độ cao dấu chồng trên chữ hoa so với ascender.
use read_fonts::{FontRef, TableProvider, tables::glyf::Glyph, types::GlyphId};
use serde::Serialize;
use skrifa::MetadataProvider;

use crate::geom::GlyphCtx;

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

pub(crate) fn coverage(font: &FontRef) -> VietnameseCoverage {
    let cmap = font.charmap();
    let chars = vi_required_chars();
    let missing: Vec<char> = chars
        .iter()
        .copied()
        .filter(|c| cmap.map(*c).is_none())
        .collect();
    let present: Vec<GlyphId> = chars
        .iter()
        .filter(|c| !c.is_ascii())
        .filter_map(|c| cmap.map(*c))
        .collect();
    let precomposed_composite = present.iter().filter(|g| is_composite(font, **g)).count();
    let combining_marks_missing: Vec<u32> = VI_MARKS
        .iter()
        .copied()
        .filter(|m| char::from_u32(*m).and_then(|c| cmap.map(c)).is_none())
        .collect();
    let bases_missing: Vec<char> = VI_BASES
        .chars()
        .filter(|c| cmap.map(*c).is_none())
        .collect();
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
        dong_sign: cmap.map('₫').is_some(),
        stacked_marks_top_em,
        stacked_marks_overflow,
    }
}

fn stacked_marks(font: &FontRef) -> (Option<f32>, Option<bool>) {
    let upm = font.head().map(|h| h.units_per_em() as f64).unwrap_or(0.0);
    if upm <= 0.0 {
        return (None, None);
    }
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
