//! Đo hình học trên outline (đơn vị font, không raster).
//! Cắt outline bằng đường thẳng (`intersect_line`), xác định đoạn "mực" bằng winding tại
//! trung điểm — bền với contour chồng nhau của variable font.
use kurbo::{Affine, BezPath, Line, ParamCurve, Point, Rect, Shape};
use read_fonts::{FontRef, TableProvider};
use skrifa::{
    GlyphId, MetadataProvider,
    instance::{Location, LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};

struct KPen(BezPath);
impl OutlinePen for KPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((x as f64, y as f64));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x as f64, y as f64));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.quad_to((cx as f64, cy as f64), (x as f64, y as f64));
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        self.0.curve_to(
            (a as f64, b as f64),
            (c as f64, d as f64),
            (x as f64, y as f64),
        );
    }
    fn close(&mut self) {
        self.0.close_path();
    }
}

pub struct GlyphShape {
    pub path: BezPath,
    pub bbox: Rect,
    pub adv: f64,
}

pub struct GlyphCtx<'a> {
    font: FontRef<'a>,
    loc: Location,
    /// tan(italicAngle) để "dựng thẳng" chữ nghiêng trước khi đo.
    deskew: f64,
    char_filter: Option<fn(char) -> bool>,
}

impl<'a> GlyphCtx<'a> {
    pub fn new(font: FontRef<'a>, loc: Location) -> Self {
        let ia = font
            .post()
            .map(|p| p.italic_angle().to_f64())
            .unwrap_or(0.0);
        let mut ctx = GlyphCtx {
            font,
            loc,
            deskew: ia.to_radians().tan(),
            char_filter: None,
        };
        // Audit 2026-11-06: post.italicAngle=0 for Roboto Italic (-9.8°)
        // and Avenir Next Condensed Medium Italic (-9.8°). Upright tolerance was ±2°;
        // use >=3° measured stem slant only for declared italic/oblique faces.
        // Upright scripts do not have a uniform shear: preserve their a/g topology heuristic.
        if ia.abs() < 0.5
            && ctx.declared_slanted()
            && !ctx.font.charmap().is_symbol()
            && let Some(slant) = ctx
                .stem_slant_deg()
                .filter(|v| v.is_finite() && v.abs() >= 3.0)
        {
            ctx.deskew = slant.to_radians().tan();
        }
        ctx
    }

    /// Ẩn bớt ký tự (dùng để test nhánh font chỉ có chữ hoa).
    pub fn with_char_filter(mut self, f: fn(char) -> bool) -> Self {
        self.char_filter = Some(f);
        self
    }

    pub fn font(&self) -> &FontRef<'a> {
        &self.font
    }

    /// Raw stem angle without deskew/rounding; shared by fallback deskew and the public metric.
    pub(crate) fn stem_slant_deg(&self) -> Option<f64> {
        let g = self.glyph('l', false).or_else(|| self.glyph('I', false))?;
        let (y1, y2) = (
            g.bbox.y0 + g.bbox.height() * 0.25,
            g.bbox.y0 + g.bbox.height() * 0.75,
        );
        let m1 = hline(&g, y1).first().map(|(a, b)| (a + b) / 2.0)?;
        let m2 = hline(&g, y2).first().map(|(a, b)| (a + b) / 2.0)?;
        Some(-((m2 - m1) / (y2 - y1)).atan().to_degrees())
    }

    fn declared_slanted(&self) -> bool {
        use read_fonts::tables::{head::MacStyle, os2::SelectionFlags};
        self.font.os2().is_ok_and(|o| {
            o.fs_selection()
                .intersects(SelectionFlags::ITALIC | SelectionFlags::OBLIQUE)
        }) || self
            .font
            .head()
            .is_ok_and(|h| h.mac_style().contains(MacStyle::ITALIC))
            // Avenir Next Condensed Heavy/Ultra Light Italic: italicAngle 0, no flags, -10° stems.
            || [
                skrifa::string::StringId::SUBFAMILY_NAME,
                skrifa::string::StringId::TYPOGRAPHIC_SUBFAMILY_NAME,
                skrifa::string::StringId::POSTSCRIPT_NAME,
            ]
            .into_iter()
            .any(|id| {
                use skrifa::MetadataProvider;
                self.font.localized_strings(id).english_or_first().is_some_and(|s| {
                    let s = s.to_string();
                    s.contains("Italic") || s.contains("Oblique")
                })
            })
    }

    pub(crate) fn is_slanted(&self) -> bool {
        self.deskew.atan().to_degrees().abs() >= 0.5 || self.declared_slanted()
    }

    pub fn glyph(&self, ch: char, upright: bool) -> Option<GlyphShape> {
        if self.char_filter.is_some_and(|f| !f(ch)) {
            return None;
        }
        let gid: GlyphId = self.font.charmap().map(ch)?;
        let og = self.font.outline_glyphs().get(gid)?;
        let mut pen = KPen(BezPath::new());
        let lr: LocationRef = (&self.loc).into();
        og.draw(DrawSettings::unhinted(Size::unscaled(), lr), &mut pen)
            .ok()?;
        let mut path = pen.0;
        if path.elements().is_empty() {
            return None;
        }
        if upright && self.deskew != 0.0 {
            path.apply_affine(Affine::new([1.0, 0.0, self.deskew, 1.0, 0.0, 0.0]));
        }
        let bbox = path.bounding_box();
        if !(bbox.width() > 0.0 && bbox.height() > 0.0) {
            return None;
        }
        let adv = self
            .font
            .glyph_metrics(Size::unscaled(), lr)
            .advance_width(gid)? as f64;
        Some(GlyphShape { path, bbox, adv })
    }
}

/// Các đoạn mực dọc theo đoạn thẳng p0→p1, tính theo khoảng cách từ p0.
pub fn ink_runs(path: &BezPath, p0: Point, p1: Point) -> Vec<(f64, f64)> {
    let line = Line::new(p0, p1);
    let len = (p1 - p0).hypot();
    let mut ts: Vec<f64> = path
        .segments()
        .flat_map(|s| s.intersect_line(line).into_iter().map(|i| i.line_t))
        .filter(|t| t.is_finite())
        .collect();
    ts.push(0.0);
    ts.push(1.0);
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let mut runs: Vec<(f64, f64)> = Vec::new();
    for w in ts.windows(2) {
        let mid = line.eval((w[0] + w[1]) * 0.5);
        if path.winding(mid) != 0 {
            let (s, e) = (w[0] * len, w[1] * len);
            match runs.last_mut() {
                Some(last) if (last.1 - s).abs() < 1e-6 => last.1 = e,
                _ => runs.push((s, e)),
            }
        }
    }
    runs
}

pub(crate) fn hline(g: &GlyphShape, y: f64) -> Vec<(f64, f64)> {
    let x0 = g.bbox.x0 - 10.0;
    ink_runs(&g.path, Point::new(x0, y), Point::new(g.bbox.x1 + 10.0, y))
        .into_iter()
        .map(|(a, b)| (a + x0, b + x0))
        .collect()
}

pub(crate) fn vline(g: &GlyphShape, x: f64) -> Vec<(f64, f64)> {
    let y0 = g.bbox.y0 - 10.0;
    ink_runs(&g.path, Point::new(x, y0), Point::new(x, g.bbox.y1 + 10.0))
        .into_iter()
        .map(|(a, b)| (a + y0, b + y0))
        .collect()
}

/// Diện tích mực bằng tích phân scanline (bền với overlap; không dùng signed area).
pub(crate) fn ink_area(g: &GlyphShape, y0: f64, y1: f64, n: usize) -> f64 {
    let dy = (y1 - y0) / n as f64;
    (0..n)
        .map(|i| {
            hline(g, y0 + dy * (i as f64 + 0.5))
                .iter()
                .map(|(a, b)| b - a)
                .sum::<f64>()
                * dy
        })
        .sum()
}

/// Bề dày nét đầu tiên (từ trái) trên dòng quét y.
pub(crate) fn first_stem(g: &GlyphShape, y: f64) -> Option<f64> {
    hline(g, y).first().map(|(a, b)| b - a)
}
