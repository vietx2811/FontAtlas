//! Render ảnh đầu vào "render-v2": 3 dòng chuỗi mẫu, 224×224 xám, chuẩn hoá SigLIP.
use read_fonts::FontRef;
use skrifa::{
    GlyphId, MetadataProvider,
    instance::{Location, LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

use crate::{CANVAS, EmbedError, MARGIN, NORM_MEAN, NORM_STD, SAMPLE_LINES};

/// Ảnh xám 224×224, 0 = đen, 255 = trắng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gray224(pub Vec<u8>);

/// Tỉ lệ ký tự (trừ khoảng trắng) của chuỗi mẫu có trong cmap.
pub fn coverage_ratio(font: &FontRef) -> f32 {
    let cmap = font.charmap();
    let chars: Vec<char> = SAMPLE_LINES.iter().flat_map(|l| l.chars()).filter(|c| !c.is_whitespace()).collect();
    let present = chars.iter().filter(|c| cmap.map(**c).is_some()).count();
    present as f32 / chars.len() as f32
}

/// Pen của skrifa → PathBuilder của tiny-skia, đặt glyph tại (x0, baseline) với tỉ lệ s, lật trục y.
struct SkiaPen<'a> {
    pb: &'a mut PathBuilder,
    x0: f32,
    base: f32,
    s: f32,
}

impl SkiaPen<'_> {
    fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x0 + x * self.s, self.base - y * self.s)
    }
}

impl OutlinePen for SkiaPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        self.pb.move_to(a, b);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        self.pb.line_to(a, b);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (c0, c1) = self.p(cx, cy);
        let (a, b) = self.p(x, y);
        self.pb.quad_to(c0, c1, a, b);
    }
    fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
        let (a0, a1) = self.p(ax, ay);
        let (b0, b1) = self.p(bx, by);
        let (c0, c1) = self.p(x, y);
        self.pb.cubic_to(a0, a1, b0, b1, c0, c1);
    }
    fn close(&mut self) {
        self.pb.close();
    }
}

pub fn render_input(font: &FontRef, loc: &Location) -> Result<Gray224, EmbedError> {
    if coverage_ratio(font) < 0.5 {
        return Err(EmbedError::NoGlyphs);
    }
    let lr: LocationRef = loc.into();
    let cmap = font.charmap();
    let gm = font.glyph_metrics(Size::unscaled(), lr);
    let m = font.metrics(Size::unscaled(), lr);
    let lh = m.ascent - m.descent;
    // Mỗi dòng: danh sách (glyph, advance); ký tự thiếu → glyph 0 (.notdef).
    let lines: Vec<Vec<(GlyphId, f32)>> = SAMPLE_LINES
        .iter()
        .map(|l| {
            l.chars()
                .map(|c| {
                    let g = cmap.map(c).unwrap_or(GlyphId::new(0));
                    (g, gm.advance_width(g).unwrap_or(0.0))
                })
                .collect()
        })
        .collect();
    let widths: Vec<f32> = lines.iter().map(|l| l.iter().map(|g| g.1).sum()).collect();
    let max_w = widths.iter().copied().fold(0.0f32, f32::max);
    if !(lh > 0.0 && max_w > 0.0) {
        return Err(EmbedError::Render("metrics".into()));
    }
    let n = lines.len() as f32;
    let avail = CANVAS as f32 - 2.0 * MARGIN;
    let s = (avail / max_w).min(avail / (n * lh));
    let top = (CANVAS as f32 - n * lh * s) / 2.0;
    let outlines = font.outline_glyphs();
    let mut pb = PathBuilder::new();
    for (i, (line, w)) in lines.iter().zip(&widths).enumerate() {
        let base = top + i as f32 * lh * s + m.ascent * s;
        let mut x = (CANVAS as f32 - w * s) / 2.0;
        for (g, adv) in line {
            if let Some(og) = outlines.get(*g) {
                let mut pen = SkiaPen { pb: &mut pb, x0: x, base, s };
                // Glyph hỏng thì bỏ qua glyph đó, không làm hỏng cả ảnh.
                let _ = og.draw(DrawSettings::unhinted(Size::unscaled(), lr), &mut pen);
            }
            x += adv * s;
        }
    }
    let mut pm = Pixmap::new(CANVAS, CANVAS).ok_or_else(|| EmbedError::Render("pixmap".into()))?;
    pm.fill(tiny_skia::Color::WHITE);
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color_rgba8(0, 0, 0, 255);
        paint.anti_alias = true;
        pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
    Ok(Gray224(pm.data().chunks(4).map(|p| p[0]).collect()))
}

/// Ảnh xám → tensor CHW 3×224×224, (v/255 − 0.5)/0.5, R=G=B.
pub fn to_tensor(img: &Gray224, out: &mut [f32]) {
    let hw = (CANVAS * CANVAS) as usize;
    for (i, v) in img.0.iter().enumerate().take(hw) {
        let x = (*v as f32 / 255.0 - NORM_MEAN) / NORM_STD;
        for c in 0..3 {
            if let Some(o) = out.get_mut(c * hw + i) {
                *o = x;
            }
        }
    }
}
