// SPIKE — code bỏ đi. Sinh preview (PNG alpha-mask + SVG path) cho mọi face trong thư mục font.
// gen <out_dir> <text> [px] [--no-write]   (dirs mặc định: /System/Library/Fonts, /Library/Fonts)
use rayon::prelude::*;
use serde::Serialize;
use skrifa::{
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
    raw::FileRef,
    string::StringId,
    MetadataProvider,
};
use std::{fmt::Write as _, io::Write as _, path::PathBuf, sync::atomic::*, time::Instant};

const W: u32 = 640; // 2x của vùng preview 320 css px
const H: u32 = 112;

#[derive(Serialize)]
struct Entry { id: usize, path: String, index: u32, family: String, style: String, bytes: u64, missing: usize }

struct Pen { pb: tiny_skia::PathBuilder, svg: String, dx: f32, base: f32 }
impl Pen {
    fn p(&self, x: f32, y: f32) -> (f32, f32) { (x + self.dx, self.base - y) }
}
impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) { let (x, y) = self.p(x, y); self.pb.move_to(x, y); let _ = write!(self.svg, "M{:.1} {:.1}", x / 2.0, y / 2.0); }
    fn line_to(&mut self, x: f32, y: f32) { let (x, y) = self.p(x, y); self.pb.line_to(x, y); let _ = write!(self.svg, "L{:.1} {:.1}", x / 2.0, y / 2.0); }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        let (a, b) = self.p(a, b); let (x, y) = self.p(x, y); self.pb.quad_to(a, b, x, y);
        let _ = write!(self.svg, "Q{:.1} {:.1} {:.1} {:.1}", a / 2.0, b / 2.0, x / 2.0, y / 2.0);
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        let (a, b) = self.p(a, b); let (c, d) = self.p(c, d); let (x, y) = self.p(x, y); self.pb.cubic_to(a, b, c, d, x, y);
        let _ = write!(self.svg, "C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}", a / 2.0, b / 2.0, c / 2.0, d / 2.0, x / 2.0, y / 2.0);
    }
    fn close(&mut self) { self.pb.close(); self.svg.push('Z'); }
}

/// Trả về (png bytes, svg string, số ký tự thiếu glyph). Không shaping — chỉ cmap + advance.
fn render(font: &skrifa::FontRef, text: &str, px: f32) -> (Vec<u8>, String, usize) {
    let size = Size::new(px);
    let loc = LocationRef::default();
    let cmap = font.charmap();
    let gm = font.glyph_metrics(size, loc);
    let outlines = font.outline_glyphs();
    let m = font.metrics(size, loc);
    let base = ((H as f32 + m.ascent + m.descent) / 2.0).round(); // canh giữa theo ascent/descent
    let mut pen = Pen { pb: tiny_skia::PathBuilder::new(), svg: String::new(), dx: 8.0, base };
    let mut missing = 0;
    for ch in text.chars() {
        let gid = match cmap.map(ch) { Some(g) => g, None => { missing += 1; continue } };
        if let Some(g) = outlines.get(gid) { let _ = g.draw(DrawSettings::unhinted(size, loc), &mut pen); }
        pen.dx += gm.advance_width(gid).unwrap_or(px * 0.5);
        if pen.dx > W as f32 { break }
    }
    let svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\"><path d=\"{}\"/></svg>", W / 2, H / 2, pen.svg);
    let mut pm = tiny_skia::Pixmap::new(W, H).unwrap();
    if let Some(path) = pen.pb.finish() {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(0, 0, 0, 255);
        paint.anti_alias = true;
        pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
    }
    // PNG GrayscaleAlpha (gray=0, alpha=coverage) → dùng làm CSS mask-image, đổi màu theo theme.
    let ga: Vec<u8> = pm.data().chunks_exact(4).flat_map(|p| [0u8, p[3]]).collect();
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, W, H);
        enc.set_color(png::ColorType::GrayscaleAlpha);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&ga).unwrap();
    }
    (out, svg, missing)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // gen one <path> <face index> <text> <px>  → PNG ra stdout (render theo yêu cầu khi đổi chữ mẫu)
    if args[1] == "one" {
        let data = std::fs::read(&args[2]).unwrap();
        let font = FileRef::new(&data).unwrap().fonts().nth(args[3].parse().unwrap()).unwrap().unwrap();
        let (pngb, _, _) = render(&font, &args[4], args[5].parse().unwrap());
        std::io::stdout().write_all(&pngb).unwrap();
        return;
    }
    let out = PathBuf::from(&args[1]);
    let text = args.get(2).cloned().unwrap_or("Hamburgefonts Ăn phở".into());
    let px: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(64.0);
    let write = !args.iter().any(|a| a == "--no-write");
    let dirs = ["/System/Library/Fonts", "/Library/Fonts"];

    let t0 = Instant::now();
    let files: Vec<PathBuf> = dirs.iter().flat_map(|d| walkdir::WalkDir::new(d).into_iter().flatten())
        .filter(|e| e.file_type().is_file())
        .filter(|e| matches!(e.path().extension().and_then(|x| x.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(), Some("ttf" | "otf" | "ttc" | "otc")))
        .map(|e| e.into_path()).collect();
    // (path, face index) cho mọi face, kể cả trong TTC
    let faces: Vec<(PathBuf, u32)> = files.par_iter().flat_map_iter(|p| {
        let data = std::fs::read(p).unwrap_or_default();
        let n = FileRef::new(&data).map(|f| f.fonts().count()).unwrap_or(0) as u32;
        (0..n).map(move |i| (p.clone(), i)).collect::<Vec<_>>()
    }).collect();
    let t_enum = t0.elapsed();

    if write { for d in ["png", "svg"] { std::fs::create_dir_all(out.join(d)).unwrap(); } }
    let png_total = AtomicU64::new(0); let svg_total = AtomicU64::new(0); let svgz_total = AtomicU64::new(0);
    let t1 = Instant::now();
    let entries: Vec<Entry> = faces.par_iter().enumerate().filter_map(|(id, (path, idx))| {
        let data = std::fs::read(path).ok()?;
        let font = FileRef::new(&data).ok()?.fonts().nth(*idx as usize)?.ok()?;
        let (pngb, svg, missing) = render(&font, &text, px);
        png_total.fetch_add(pngb.len() as u64, Ordering::Relaxed);
        svg_total.fetch_add(svg.len() as u64, Ordering::Relaxed);
        let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(svg.as_bytes()).unwrap();
        svgz_total.fetch_add(z.finish().unwrap().len() as u64, Ordering::Relaxed);
        if write {
            std::fs::write(out.join(format!("png/{id}.png")), &pngb).ok()?;
            std::fs::write(out.join(format!("svg/{id}.svg")), &svg).ok()?;
        }
        let name = |sid| font.localized_strings(sid).english_or_first().map(|s| s.to_string()).unwrap_or_default();
        Some(Entry { id, path: path.display().to_string(), index: *idx, family: name(StringId::FAMILY_NAME), style: name(StringId::SUBFAMILY_NAME), bytes: data.len() as u64, missing })
    }).collect();
    let t_render = t1.elapsed();
    if write { std::fs::write(out.join("manifest.json"), serde_json::to_string(&entries).unwrap()).unwrap(); }
    let n = entries.len() as f64;
    let threads = rayon::current_num_threads();
    let t_ms = t_render.as_secs_f64() * 1000.0;
    eprintln!("{}", serde_json::json!({
        "files": files.len(), "faces": entries.len(), "threads": threads, "text": text, "px": px, "write": write,
        "enum_ms": t_enum.as_millis(), "render_ms": t_ms.round(), "ms_per_face_cpu": (t_ms * threads as f64 / n * 100.0).round() / 100.0,
        "png_avg_bytes": (png_total.load(Ordering::Relaxed) as f64 / n).round(),
        "svg_avg_bytes": (svg_total.load(Ordering::Relaxed) as f64 / n).round(),
        "svgz_avg_bytes": (svgz_total.load(Ordering::Relaxed) as f64 / n).round(),
        "faces_missing_glyphs": entries.iter().filter(|e| e.missing > 0).count(),
    }));
}
