//! Dàn chữ (cmap + advance) và vẽ PNG alpha-mask.
use crate::PreviewError;

/// Co cho vừa: top/bottom là biên lớn nhất của metric và mực thật (dấu chồng có thể vượt ascent).
pub fn fit(ascent: f32, descent: f32, ink_top: f32, ink_bottom: f32, height: u32, vpad: f32) -> (f32, f32) {
    let top = ascent.max(ink_top);
    let bottom = descent.min(ink_bottom);
    let avail = height as f32 - 2.0 * vpad;
    let k = (avail / (top - bottom).max(1e-3)).min(1.0);
    (k, vpad + top * k) // baseline tính từ mép trên canvas (y hướng xuống)
}

/// Số glyph giữ lại: dừng trước glyph đầu tiên có mép phải vượt (width - hpad); không cắt giữa glyph.
pub fn truncate_at(right_edges: &[f32], k: f32, width: u32, hpad: f32) -> usize {
    let limit = width as f32 - hpad;
    right_edges.iter().take_while(|&&r| hpad + r * k <= limit).count()
}

pub fn require_fit(keep: usize) -> Result<usize, PreviewError> {
    if keep == 0 { Err(PreviewError::NothingFits) } else { Ok(keep) }
}

use read_fonts::FontRef;
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};

use crate::{CMAP_SAMPLE_CHARS, TextSource, canvas_size, check_px, normalize_text, pick_text};

/// Ảnh preview đã mã hoá.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub source: TextSource,
    pub text: String,
    pub truncated: bool,
}

#[derive(Clone, Copy)]
enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// Bộ thu outline: cộng dịch x, ghi biên mực từ mọi điểm (kể cả điểm điều khiển).
struct Recorder {
    dx: f32,
    segs: Vec<Seg>,
    top: f32,
    bottom: f32,
}
impl Recorder {
    fn pt(&mut self, x: f32, y: f32) -> (f32, f32) {
        self.top = self.top.max(y);
        self.bottom = self.bottom.min(y);
        (x + self.dx, y)
    }
}
impl OutlinePen for Recorder {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.pt(x, y);
        self.segs.push(Seg::Move(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.pt(x, y);
        self.segs.push(Seg::Line(x, y));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (cx, cy) = self.pt(cx, cy);
        let (x, y) = self.pt(x, y);
        self.segs.push(Seg::Quad(cx, cy, x, y));
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        let (a, b) = self.pt(a, b);
        let (c, d) = self.pt(c, d);
        let (x, y) = self.pt(x, y);
        self.segs.push(Seg::Cubic(a, b, c, d, x, y));
    }
    fn close(&mut self) {
        self.segs.push(Seg::Close);
    }
}

/// Mẫu từ cmap: ký tự in được đầu tiên theo thứ tự codepoint (bỏ control, khoảng trắng, PUA).
pub fn cmap_sample(font: &FontRef<'_>) -> String {
    font.charmap()
        .mappings()
        .filter_map(|(cp, _)| char::from_u32(cp))
        .filter(|&c| c >= '\u{21}' && !c.is_control() && !c.is_whitespace() && !('\u{E000}'..='\u{F8FF}').contains(&c))
        .take(CMAP_SAMPLE_CHARS)
        .collect()
}

pub fn render_face(face: &fontatlas_core::Face<'_>, text: &str, px: u32) -> Result<Rendered, PreviewError> {
    check_px(px)?;
    let (w, h) = canvas_size(px);
    let hpad = px as f32 / 8.0;
    let vpad = (h as f32 / 28.0).max(2.0);
    let font = face.font();
    let size = Size::new(px as f32);
    let loc = LocationRef::default();
    let cmap = font.charmap();
    let (text, source) = pick_text(&normalize_text(text), |c| cmap.map(c).is_some(), || cmap_sample(font));
    if source == TextSource::Nothing {
        return Err(PreviewError::NoOutlines);
    }

    // Dàn chữ: mỗi glyph một khoảng trong `segs` để cắt theo số glyph giữ lại.
    let outlines = font.outline_glyphs();
    let gm = font.glyph_metrics(size, loc);
    let mut rec = Recorder { dx: 0.0, segs: Vec::new(), top: f32::MIN, bottom: f32::MAX };
    let mut ends = Vec::new(); // vị trí kết thúc segs của từng glyph
    let mut right_edges = Vec::new();
    for ch in text.chars() {
        let Some(gid) = cmap.map(ch) else { continue };
        if let Some(g) = outlines.get(gid) {
            // Glyph lỗi outline thì bỏ phần vẽ, vẫn giữ advance.
            let _ = g.draw(DrawSettings::unhinted(size, loc), &mut rec);
        }
        let adv = gm.advance_width(gid).unwrap_or(px as f32 * 0.5);
        rec.dx += adv;
        ends.push(rec.segs.len());
        right_edges.push(rec.dx);
    }
    if rec.segs.is_empty() {
        return Err(PreviewError::NoOutlines);
    }

    let m = font.metrics(size, loc);
    let (k, base) = fit(m.ascent, m.descent, rec.top, rec.bottom, h, vpad);
    let keep = require_fit(truncate_at(&right_edges, k, w, hpad))?;
    let truncated = keep < right_edges.len();
    let end = ends[keep - 1];

    let tf = |x: f32, y: f32| (hpad + x * k, base - y * k);
    let mut pb = tiny_skia::PathBuilder::new();
    for s in &rec.segs[..end] {
        match *s {
            Seg::Move(x, y) => { let (x, y) = tf(x, y); pb.move_to(x, y) }
            Seg::Line(x, y) => { let (x, y) = tf(x, y); pb.line_to(x, y) }
            Seg::Quad(a, b, x, y) => {
                let ((a, b), (x, y)) = (tf(a, b), tf(x, y));
                pb.quad_to(a, b, x, y)
            }
            Seg::Cubic(a, b, c, d, x, y) => {
                let ((a, b), (c, d), (x, y)) = (tf(a, b), tf(c, d), tf(x, y));
                pb.cubic_to(a, b, c, d, x, y)
            }
            Seg::Close => pb.close(),
        }
    }
    let mut pm = tiny_skia::Pixmap::new(w, h).ok_or_else(|| PreviewError::Encode("pixmap".into()))?;
    if let Some(path) = pb.finish() {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(0, 0, 0, 255);
        paint.anti_alias = true;
        pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
    }
    // GrayscaleAlpha: gray = 0, alpha = độ phủ → FE tô màu bằng mask-image.
    let ga: Vec<u8> = pm.data().chunks_exact(4).flat_map(|p| [0u8, p[3]]).collect();
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::GrayscaleAlpha);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut wr = enc.write_header().map_err(|e| PreviewError::Encode(e.to_string()))?;
        wr.write_image_data(&ga).map_err(|e| PreviewError::Encode(e.to_string()))?;
    }
    Ok(Rendered { png: out, width: w, height: h, source, text, truncated })
}
