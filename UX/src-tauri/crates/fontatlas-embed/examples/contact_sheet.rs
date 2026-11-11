//! Công cụ QA nhãn (chỉ dùng tay, không vào CI): `cargo run --release -p fontatlas-embed --example contact_sheet -- <out.png> <text> <cols> <file>...`
//! Mỗi ô 340×150: <text> vẽ một dòng, vừa chiều rộng ô. In ra "ô N = file" theo thứ tự trái→phải, trên→dưới.
use read_fonts::FontRef;
use skrifa::{
    GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

const CW: u32 = 340;
const CH: u32 = 150;

struct Pen<'a> {
    pb: &'a mut PathBuilder,
    x0: f32,
    base: f32,
    s: f32,
}
impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x0 + x * self.s, self.base - y * self.s)
    }
}
impl OutlinePen for Pen<'_> {
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

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (out, text, cols) = (&a[0], &a[1], a[2].parse::<u32>().unwrap());
    let files = &a[3..];
    let rows = (files.len() as u32).div_ceil(cols);
    let mut sheet = Pixmap::new(cols * CW, rows * CH).unwrap();
    sheet.fill(tiny_skia::Color::WHITE);
    for (i, f) in files.iter().enumerate() {
        println!("ô {} = {}", i + 1, f);
        let Ok(bytes) = std::fs::read(f) else { continue };
        let Ok(font) = FontRef::from_index(&bytes, 0) else { continue };
        let loc = LocationRef::default();
        let m = font.metrics(Size::unscaled(), loc);
        let gm = font.glyph_metrics(Size::unscaled(), loc);
        let cmap = font.charmap();
        let glyphs: Vec<(GlyphId, f32)> = text
            .chars()
            .map(|c| {
                let g = cmap.map(c).unwrap_or(GlyphId::new(0));
                (g, gm.advance_width(g).unwrap_or(0.0))
            })
            .collect();
        let w: f32 = glyphs.iter().map(|g| g.1).sum();
        let lh = m.ascent - m.descent;
        let s = ((CW as f32 - 20.0) / w).min((CH as f32 - 20.0) / lh);
        let (cx, cy) = ((i as u32 % cols) as f32 * CW as f32, (i as u32 / cols) as f32 * CH as f32);
        let base = cy + (CH as f32 - lh * s) / 2.0 + m.ascent * s;
        let mut x = cx + (CW as f32 - w * s) / 2.0;
        let mut pb = PathBuilder::new();
        let outlines = font.outline_glyphs();
        for (g, adv) in &glyphs {
            if let Some(og) = outlines.get(*g) {
                let mut pen = Pen { pb: &mut pb, x0: x, base, s };
                let _ = og.draw(DrawSettings::unhinted(Size::unscaled(), loc), &mut pen);
            }
            x += adv * s;
        }
        if let Some(p) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(0, 0, 0, 255);
            paint.anti_alias = true;
            sheet.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
        }
        // viền ô + số thứ tự (chấm theo số: vạch nhỏ ở góc trái trên để đếm)
        let mut border = PathBuilder::new();
        border.move_to(cx, cy);
        border.line_to(cx + CW as f32, cy);
        border.line_to(cx + CW as f32, cy + CH as f32);
        border.line_to(cx, cy + CH as f32);
        border.close();
        if let Some(p) = border.finish() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(190, 190, 190, 255);
            let st = tiny_skia::Stroke { width: 1.0, ..Default::default() };
            sheet.stroke_path(&p, &paint, &st, Transform::identity(), None);
        }
    }
    let file = std::fs::File::create(out).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), sheet.width(), sheet.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(sheet.data()).unwrap();
}
