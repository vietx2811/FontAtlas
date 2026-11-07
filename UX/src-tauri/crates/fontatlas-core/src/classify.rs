//! Phân loại font hai tầng (class → subclass) kèm tag dùng (Text/Display/UI), giải thích bằng `signals[]`.
//!
//! Tín hiệu theo thứ tự tin cậy: số đo có sẵn → Panose → OS/2 `sFamilyClass` → từ khoá trong tên → cmap
//! (Symbol). Tín hiệu mâu thuẫn thì trả `Uncertain` (không đoán). Hàm thuần: nhận JSON đã lưu của
//! Metadata/Anatomy nên app tính lại được mà không phải phân tích lại font.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Serif,
    Sans,
    Mono,
    Script,
    Display,
    Blackletter,
    Symbol,
    Uncertain,
}

impl Class {
    pub fn label(self) -> &'static str {
        match self {
            Class::Serif => "Serif",
            Class::Sans => "Sans",
            Class::Mono => "Mono",
            Class::Script => "Script",
            Class::Display => "Display",
            Class::Blackletter => "Blackletter",
            Class::Symbol => "Symbol",
            Class::Uncertain => "Uncertain",
        }
    }

    pub fn parse(s: &str) -> Option<Class> {
        Some(match s.to_ascii_lowercase().as_str() {
            "serif" => Class::Serif,
            "sans" => Class::Sans,
            "mono" => Class::Mono,
            "script" => Class::Script,
            "display" => Class::Display,
            "blackletter" => Class::Blackletter,
            "symbol" => Class::Symbol,
            "uncertain" => Class::Uncertain,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Usage {
    Text,
    Display,
    Ui,
}

/// Một bằng chứng: nguồn, mô tả, phiếu bầu cho class nào và trọng số.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    /// "measure" | "panose" | "os2_class" | "name" | "cmap" | "post"
    pub source: String,
    pub detail: String,
    pub vote: Class,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classification {
    pub class: Class,
    /// snake_case, `None` khi chưa đủ chắc (class vẫn có thể chắc).
    pub subclass: Option<String>,
    /// 0–1.
    pub confidence: f32,
    pub signals: Vec<Signal>,
    pub usage: Vec<Usage>,
}

/// Đầu vào tối thiểu; dựng từ JSON đã lưu.
#[derive(Debug, Clone, Default)]
pub struct ClassifyInput {
    /// family + full name + postscript, chữ thường.
    pub name: String,
    pub panose: Option<[u8; 10]>,
    pub family_class: Option<(u8, u8)>,
    pub is_fixed_pitch: bool,
    pub symbol_cmap: bool,
    pub latin_letters: Option<u8>,
    pub opsz_max: Option<f32>,
    pub m: Metrics,
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub serif_foot: Option<f32>,
    pub contrast: Option<f32>,
    pub stress_abs: Option<f32>,
    pub slant_abs: Option<f32>,
    pub aperture: Option<f32>,
    pub width: Option<f32>,
    pub width_cv: Option<f32>,
    pub spacing: Option<f32>,
    pub stem: Option<f32>,
    pub x_height_em: Option<f32>,
    pub a_storeys: Option<f32>,
    pub g_storeys: Option<f32>,
}

fn metric(an: &Value, k: &str) -> Option<f32> {
    let m = an.get(k)?;
    if m.get("quality")?.get("kind")?.as_str()? == "failed" {
        return None;
    }
    m.get("value")?.as_f64().map(|v| v as f32)
}

impl ClassifyInput {
    /// Từ JSON `metadata_json` và `anatomy_json` đã lưu trong thư viện (`Metadata`/`Anatomy` của core).
    pub fn from_json(meta: &Value, anatomy: &Value) -> Self {
        let s = |k: &str| meta.get(k).and_then(Value::as_str).unwrap_or("");
        let fam = if s("typographic_family").is_empty() { s("family") } else { s("typographic_family") };
        let name = format!("{} {} {}", fam, s("full_name"), s("postscript_name")).to_lowercase();
        let panose = meta
            .get("panose")
            .and_then(Value::as_array)
            .filter(|a| a.len() == 10)
            .and_then(|a| a.iter().map(|x| x.as_u64().map(|v| v as u8)).collect::<Option<Vec<u8>>>())
            .and_then(|v| <[u8; 10]>::try_from(v).ok());
        let family_class = meta
            .get("family_class")
            .and_then(Value::as_array)
            .and_then(|a| Some((a.first()?.as_u64()? as u8, a.get(1)?.as_u64()? as u8)));
        let opsz_max = meta.get("axes").and_then(Value::as_array).and_then(|a| {
            a.iter().find(|x| x.get("tag").and_then(Value::as_str) == Some("opsz"))?.get("max")?.as_f64().map(|v| v as f32)
        });
        let g = |k: &str| metric(anatomy, k);
        ClassifyInput {
            name,
            panose,
            family_class,
            is_fixed_pitch: meta.get("is_fixed_pitch").and_then(Value::as_bool).unwrap_or(false),
            symbol_cmap: meta.get("symbol_cmap").and_then(Value::as_bool).unwrap_or(false),
            latin_letters: meta.get("latin_letters").and_then(Value::as_u64).map(|v| v as u8),
            opsz_max,
            m: Metrics {
                serif_foot: g("serif_foot"),
                contrast: g("contrast"),
                stress_abs: g("stress_deg").map(f32::abs),
                slant_abs: g("slant_deg").map(f32::abs),
                aperture: g("aperture"),
                width: g("width"),
                width_cv: g("width_cv"),
                spacing: g("spacing"),
                stem: g("stem"),
                x_height_em: g("x_height_em"),
                a_storeys: g("a_storeys"),
                g_storeys: g("g_storeys"),
            },
        }
    }
}

const SYMBOL_KW: &[&str] = &["dingbat", "symbol", "wingding", "webding", "emoji", "icons", "pictogram", "ornament"];
const BLACKLETTER_KW: &[&str] = &["blackletter", "fraktur", "textura", "old english", "unifraktur", "gotisch", "schwabacher", "rotunda", "germania"];
const MONO_KW: &[&str] = &["mono", "typewriter", "courier", "consolas", "code", "terminal"];
const SCRIPT_KW: &[&str] = &["script", "brush", "calligra", "cursive", "handwrit", "signature", "marker", "lettering", "copperplate", "spencer"];
const DISPLAY_KW: &[&str] = &["stencil", "inline", "outline", "shadow", "fat face", "display", "poster", "decorative", "ornate", "rounded", "blobby", "pixel", "wacky"];
const SERIF_KW: &[&str] = &["serif", "garamond", "caslon", "baskerville", "didot", "bodoni", "slab", "roman", "times", "georgia", "palatino", "antiqua"];
const SANS_KW: &[&str] = &["sans", "grotesk", "grotesque", "gothic", "helvet", "arial", "neue"];

/// Khớp theo từ (không khớp chuỗi con giữa từ: "open" không chứa "pen"). Từ khoá một từ khớp khi trùng
/// hoặc là tiền tố của một từ (≥ 4 ký tự); cụm nhiều từ khớp theo chuỗi con.
fn has_kw(name: &str, kws: &[&str]) -> Option<String> {
    let tokens: Vec<&str> = name.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
    kws.iter()
        .find(|k| {
            if k.contains(' ') {
                name.contains(**k)
            } else {
                tokens.iter().any(|t| t == *k || (k.len() >= 4 && t.starts_with(**k)))
            }
        })
        .map(|k| (*k).to_string())
}

struct Votes {
    list: Vec<Signal>,
}

impl Votes {
    fn add(&mut self, source: &str, detail: impl Into<String>, vote: Class, weight: f32) {
        self.list.push(Signal { source: source.into(), detail: detail.into(), vote, weight });
    }

    fn score(&self, c: Class) -> f32 {
        self.list.iter().filter(|s| s.vote == c).map(|s| s.weight).sum()
    }
}

pub fn classify(i: &ClassifyInput) -> Classification {
    let mut v = Votes { list: Vec::new() };
    let m = &i.m;
    let p = i.panose;
    // --- Symbol (cmap, Panose kind 5, tên)
    if i.symbol_cmap {
        v.add("cmap", "chỉ có subtable Windows Symbol (3,0)", Class::Symbol, 3.0);
    }
    if p.is_some_and(|p| p[0] == 5) {
        v.add("panose", "family kind = 5 (Latin Symbol)", Class::Symbol, 2.0);
    }
    if let Some(k) = has_kw(&i.name, SYMBOL_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Symbol, 2.0);
    }
    if i.latin_letters.is_some_and(|n| n < 10) && !v.list.is_empty() {
        v.add("cmap", "gần như không có chữ cái Latin", Class::Symbol, 1.0);
    }
    // --- Blackletter
    if let Some(k) = has_kw(&i.name, BLACKLETTER_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Blackletter, 3.0);
    }
    // --- Mono
    if i.is_fixed_pitch {
        v.add("post", "post.isFixedPitch", Class::Mono, 2.0);
    }
    if m.width_cv.is_some_and(|x| x.abs() < 0.005) {
        v.add("measure", "bề rộng a–z đều nhau (width_cv ≈ 0)", Class::Mono, 2.0);
    }
    if p.is_some_and(|p| p[3] == 9) {
        v.add("panose", "proportion = monospaced", Class::Mono, 1.5);
    }
    if let Some(k) = has_kw(&i.name, MONO_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Mono, 1.0);
    }
    // --- Script
    if p.is_some_and(|p| p[0] == 3) {
        v.add("panose", "family kind = 3 (Latin Hand Written)", Class::Script, 2.5);
    }
    if i.family_class.is_some_and(|c| c.0 == 10) {
        v.add("os2_class", "sFamilyClass = 10 (Scripts)", Class::Script, 2.0);
    }
    if let Some(k) = has_kw(&i.name, SCRIPT_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Script, 1.5);
    }
    if m.slant_abs.is_some_and(|s| s >= 10.0) && m.spacing.is_some_and(|s| s < 0.06) {
        v.add("measure", "nghiêng mạnh, sidebearing rất nhỏ (chữ nối)", Class::Script, 1.0);
    }
    // --- Display
    if p.is_some_and(|p| p[0] == 4) {
        v.add("panose", "family kind = 4 (Latin Decorative)", Class::Display, 2.0);
    }
    if i.family_class.is_some_and(|c| c.0 == 9) {
        v.add("os2_class", "sFamilyClass = 9 (Ornamentals)", Class::Display, 1.5);
    }
    if let Some(k) = has_kw(&i.name, DISPLAY_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Display, 1.5);
    }
    // --- Dấu hiệu "không phải chữ văn bản" từ số đo (font GF hầu như không khai Panose/sFamilyClass)
    let cue = |v: &mut Votes, cond: bool, what: String, vote: Class, w: f32| {
        if cond {
            v.add("measure", what, vote, w);
        }
    };
    if let Some(x) = m.spacing {
        cue(&mut v, x < 0.04, format!("sidebearing rất nhỏ ({x:.2})"), Class::Script, 1.5);
        cue(&mut v, (0.04..0.06).contains(&x), format!("sidebearing nhỏ ({x:.2})"), Class::Script, 0.7);
    }
    if let Some(x) = m.slant_abs {
        cue(&mut v, x >= 5.0, format!("nghiêng {x:.0}°"), Class::Script, 1.5);
        cue(&mut v, (3.0..5.0).contains(&x), format!("nghiêng nhẹ {x:.0}°"), Class::Script, 0.5);
    }
    if let Some(x) = m.x_height_em {
        cue(&mut v, x <= 0.38, format!("x-height thấp {x:.2}"), Class::Script, 0.8);
        cue(&mut v, x >= 0.62, format!("x-height rất cao {x:.2}"), Class::Display, 0.8);
    }
    if let Some(x) = m.contrast {
        cue(&mut v, x >= 6.0, format!("contrast rất cao {x:.1}"), Class::Display, 1.0);
    }
    if let Some(x) = m.stem {
        cue(&mut v, x >= 0.30, format!("nét rất đậm {x:.2}"), Class::Display, 0.8);
        cue(&mut v, x <= 0.08, format!("nét rất mảnh {x:.2}"), Class::Display, 0.8);
    }
    if let Some(x) = m.width {
        cue(&mut v, x <= 0.65, format!("rất hẹp {x:.2}"), Class::Display, 0.8);
        cue(&mut v, x >= 1.2, format!("rất rộng {x:.2}"), Class::Display, 0.6);
    }
    if let Some(x) = m.serif_foot {
        cue(&mut v, x < 0.9, format!("serif_foot bất thường {x:.2}"), Class::Display, 0.7);
    }
    if let Some(x) = m.width_cv {
        cue(&mut v, x < 0.18 && !i.is_fixed_pitch, format!("bề rộng chữ đều bất thường {x:.2}"), Class::Display, 0.8);
    }
    if let Some(x) = m.aperture {
        cue(&mut v, x < 45.0, format!("aperture rất khép {x:.0}°"), Class::Display, 0.6);
    }
    if let Some(x) = m.stress_abs {
        cue(&mut v, x >= 18.0, format!("trục nhấn lệch {x:.0}°"), Class::Display, 0.7);
    }
    // --- Serif / Sans
    if let Some(sf) = m.serif_foot {
        if sf > 1.6 {
            v.add("measure", format!("serif_foot {sf:.2} > 1,6"), Class::Serif, 2.0);
        } else if sf < 1.2 {
            v.add("measure", format!("serif_foot {sf:.2} < 1,2"), Class::Sans, 2.0);
        }
    }
    if let Some(p) = p.filter(|p| p[0] == 2) {
        match p[1] {
            2..=10 | 14 => v.add("panose", format!("serif style {}", p[1]), Class::Serif, 1.5),
            11..=13 | 15 => v.add("panose", format!("serif style {} (sans)", p[1]), Class::Sans, 1.5),
            _ => {}
        }
    }
    if let Some((c, _)) = i.family_class {
        match c {
            1..=5 | 7 => v.add("os2_class", format!("sFamilyClass = {c} (serif)"), Class::Serif, 1.5),
            8 => v.add("os2_class", "sFamilyClass = 8 (Sans Serif)", Class::Sans, 1.5),
            _ => {}
        }
    }
    if let Some(k) = has_kw(&i.name, SERIF_KW).filter(|k| k != "serif" || !i.name.contains("sans")) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Serif, 0.8);
    }
    if let Some(k) = has_kw(&i.name, SANS_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Sans, 0.8);
    }

    let class = decide(&v);
    let (class, confidence) = class;
    let subclass = if class == Class::Uncertain { None } else { subclass_of(class, i, &v) };
    let usage = usage_of(class, i);
    Classification { class, subclass, confidence, signals: v.list, usage }
}

/// Chọn class: các nhóm "mạnh" (Symbol, Blackletter, Mono, Script, Display) cần đủ điểm; còn lại Serif/Sans.
/// Tổng điểm Script+Display tối thiểu để coi là font không dùng cho chữ văn bản.
const ODD_MIN: f32 = 1.6;
const ODD_MARGIN: f32 = 1.0;

fn decide(v: &Votes) -> (Class, f32) {
    for (c, min) in [(Class::Symbol, 2.0), (Class::Blackletter, 2.5), (Class::Mono, 2.0)] {
        if v.score(c) >= min {
            return (c, (v.score(c) / (v.score(c) + 1.0)).min(0.99));
        }
    }
    let script = v.score(Class::Script);
    let display = v.score(Class::Display);
    let (serif, sans) = (v.score(Class::Serif), v.score(Class::Sans));
    let text = serif.max(sans);
    let odd = script + display;
    // Script: bằng chứng chữ viết/nối mạnh hơn bằng chứng Display
    // bằng chứng "chữ văn bản" (serif_foot rõ, Panose, sFamilyClass) cạnh tranh với "không phải chữ văn bản": nhập nhằng → Uncertain
    let contested = odd < text * ODD_MARGIN && text >= 3.0;
    if script >= 1.5 && script >= display && odd >= ODD_MIN {
        return if contested { (Class::Uncertain, 0.0) } else { (Class::Script, (odd / (odd + 1.5)).min(0.99)) };
    }
    if odd >= ODD_MIN {
        return if contested { (Class::Uncertain, 0.0) } else { (Class::Display, (odd / (odd + 1.5)).min(0.99)) };
    }
    let (hi, lo, class) = if serif >= sans { (serif, sans, Class::Serif) } else { (sans, serif, Class::Sans) };
    if hi < 1.5 {
        return (Class::Uncertain, 0.0);
    }
    // số đo đã nói rõ ngược lại với phần còn lại (≥ 2 điểm) → mâu thuẫn thật, không đoán
    if lo >= 2.0 || (lo > 0.0 && lo >= hi * 0.65) {
        return (Class::Uncertain, 0.0);
    }
    (class, ((hi - lo) / (hi + 1.0)).clamp(0.05, 0.99))
}

fn kw(i: &ClassifyInput, kws: &[&str]) -> bool {
    has_kw(&i.name, kws).is_some()
}

fn subclass_of(class: Class, i: &ClassifyInput, v: &Votes) -> Option<String> {
    let m = &i.m;
    let sfc = i.family_class;
    let s = |x: &str| Some(x.to_string());
    match class {
        Class::Serif => {
            if kw(i, &["slab", "rockwell", "clarendon", "egyptian", "memphis"]) || sfc.is_some_and(|c| c.0 == 5) {
                return s("slab");
            }
            if kw(i, &["glyphic", "wedge", "trajan", "optima", "marcellus", "cinzel"]) {
                return s("glyphic");
            }
            if kw(i, &["fat face", "fatface"]) {
                return s("fat_face");
            }
            let (c, st) = (m.contrast?, m.stress_abs);
            if c < 1.7 && m.serif_foot.is_some_and(|f| f > 1.6) {
                return s("slab");
            }
            if sfc.is_some_and(|x| x.0 == 3) || (c >= 3.2 && st.is_some_and(|x| x <= 4.0)) {
                return s("didone");
            }
            if sfc.is_some_and(|x| x.0 == 1) || st.is_some_and(|x| x >= 7.0) {
                return s("humanist_oldstyle");
            }
            if sfc.is_some_and(|x| x.0 == 2) || st.is_some_and(|x| x < 7.0) {
                return s("transitional");
            }
            None
        }
        Class::Sans => {
            if let Some((8, sub)) = sfc {
                match sub {
                    2 => return s("humanist"),
                    3 | 4 => return s("geometric"),
                    1 | 5 => return s("neo_grotesque"),
                    _ => {}
                }
            }
            let a = m.a_storeys?;
            let ap = m.aperture?;
            // một tầng 'a' hoặc chữ tròn rộng, aperture khép: geometric
            if a < 1.5 || (m.width.is_some_and(|w| w >= 1.0) && ap < 90.0) {
                return s("geometric");
            }
            // aperture rất mở (nét cuối nghiêng, đầu chữ mở): humanist
            if ap >= 94.0 {
                return s("humanist");
            }
            let _ = v;
            if ap < 70.0 { s("grotesque") } else { s("neo_grotesque") }
        }
        Class::Mono => {
            let sf = m.serif_foot?;
            if sf > 1.6 {
                s("serif_mono")
            } else if sf < 1.2 {
                s("sans_mono")
            } else {
                None
            }
        }
        Class::Script => {
            if kw(i, &["brush", "marker", "paint", "casual", "felt"]) {
                return s("casual_brush");
            }
            if kw(i, &["copperplate", "spencer", "formal", "calligra", "chancery", "roundhand"]) || m.contrast.is_some_and(|c| c > 3.0) {
                return s("formal");
            }
            s("handwritten")
        }
        Class::Display => {
            if kw(i, &["stencil"]) {
                s("stencil")
            } else if kw(i, &["inline", "outline", "shadow"]) {
                s("inline_outline")
            } else if kw(i, &["rounded", "round "]) {
                s("rounded")
            } else if kw(i, &["fat face", "fatface"]) {
                s("fat_face")
            } else {
                s("decorative")
            }
        }
        _ => None,
    }
}

fn usage_of(class: Class, i: &ClassifyInput) -> Vec<Usage> {
    let m = &i.m;
    let mut out = Vec::new();
    match class {
        Class::Script | Class::Display | Class::Blackletter => out.push(Usage::Display),
        Class::Serif | Class::Sans | Class::Mono => {
            let contrast = m.contrast.unwrap_or(1.5);
            if contrast >= 4.0 || i.opsz_max.is_some_and(|o| o >= 72.0) {
                out.push(Usage::Display);
            }
            if contrast < 4.0 && m.x_height_em.is_none_or(|x| x >= 0.44) && m.spacing.is_none_or(|s| s >= 0.08) {
                out.push(Usage::Text);
            }
            if class != Class::Serif && contrast < 1.6 && m.x_height_em.is_some_and(|x| x >= 0.5) {
                out.push(Usage::Ui);
            }
        }
        _ => {}
    }
    out
}
