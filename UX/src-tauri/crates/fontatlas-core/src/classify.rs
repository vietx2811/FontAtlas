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
            "blackletter" => Class::Blackletter,
            "symbol" => Class::Symbol,
            "uncertain" => Class::Uncertain,
            _ => return None,
        })
    }
}

impl Class {
    /// Tên snake_case dùng trong DTO/lọc (khớp serde).
    pub fn snake(self) -> &'static str {
        match self {
            Class::Serif => "serif",
            Class::Sans => "sans",
            Class::Mono => "mono",
            Class::Script => "script",
            Class::Blackletter => "blackletter",
            Class::Symbol => "symbol",
            Class::Uncertain => "uncertain",
        }
    }
}

/// Subclass hợp lệ của từng class (dùng kiểm tra ghi đè của người dùng).
pub fn subclasses_of(c: Class) -> &'static [&'static str] {
    match c {
        Class::Serif => &["humanist_oldstyle", "transitional", "didone", "slab", "glyphic"],
        Class::Sans => &["grotesque", "neo_grotesque", "geometric", "humanist"],
        Class::Mono => &["serif_mono", "sans_mono"],
        Class::Script => &["formal", "casual_brush", "handwritten"],
        Class::Blackletter | Class::Symbol | Class::Uncertain => &[],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Usage {
    Text,
    Display,
    Ui,
}

/// Một bằng chứng: nguồn, mô tả, phiếu bầu cho class nào (`None` = chỉ ủng hộ tag usage Display / traits) và trọng số.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    /// "measure" | "panose" | "os2_class" | "name" | "cmap" | "post"
    pub source: String,
    pub detail: String,
    pub vote: Option<Class>,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classification {
    pub class: Class,
    /// snake_case, `None` khi chưa đủ chắc (class vẫn có thể chắc).
    pub subclass: Option<String>,
    /// 0–1.
    pub confidence: f32,
    /// Tag hình thái độc lập với class: stencil | inline_outline | rounded | fat_face | pixel | decorative.
    #[serde(default)]
    pub traits: Vec<String>,
    /// Lớp tốt nhất máy đoán khi `class = Uncertain` vì độ tin cậy dưới `CONF_MIN` (để UI hiện "có lẽ là …").
    #[serde(default)]
    pub candidate: Option<Class>,
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
    /// Mặt in nghiêng/oblique (italic_angle ≠ 0 hoặc tên có Italic/Oblique): độ nghiêng khi đó là kiểu chữ, không phải dấu hiệu Script.
    pub italic: bool,
    pub m: Metrics,
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub serif_foot: Option<f32>,
    /// Cực đại bề rộng chân trong dải chân ÷ thân (core ≥ 0.9.0; None với bản phân tích cũ).
    pub serif_flare: Option<f32>,
    /// Cờ serif trên-trái của i/l/n ÷ thân (core ≥ 0.9.0).
    pub serif_head: Option<f32>,
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
    pub connect: Option<f32>,
    pub bounce: Option<f32>,
    pub terminal: Option<f32>,
    pub aperture_e: Option<f32>,
    pub counter: Option<f32>,
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
        let italic = meta.get("italic_angle").and_then(Value::as_f64).is_some_and(|a| a.abs() > 0.5)
            || name.split(|c: char| !c.is_alphanumeric()).any(|t| matches!(t, "italic" | "oblique" | "slanted"));
        ClassifyInput {
            name,
            panose,
            family_class,
            is_fixed_pitch: meta.get("is_fixed_pitch").and_then(Value::as_bool).unwrap_or(false),
            symbol_cmap: meta.get("symbol_cmap").and_then(Value::as_bool).unwrap_or(false),
            latin_letters: meta.get("latin_letters").and_then(Value::as_u64).map(|v| v as u8),
            opsz_max,
            italic,
            m: Metrics {
                serif_foot: g("serif_foot"),
                serif_flare: g("serif_flare"),
                serif_head: g("serif_head"),
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
                connect: g("connect"),
                bounce: g("bounce"),
                terminal: g("terminal"),
                aperture_e: g("aperture_e"),
                counter: g("counter"),
            },
        }
    }
}

/// Từ khoá khớp nguyên từ; hậu tố `*` = khớp tiền tố (Monoton/Monofett không khớp "mono", Romanesco không khớp "roman").
const SYMBOL_KW: &[&str] = &["dingbat*", "symbol*", "wingding*", "webding*", "emoji", "icons", "pictogram*", "ornament*"];
const BLACKLETTER_KW: &[&str] = &["blackletter", "fraktur", "textura", "old english", "unifraktur", "gotisch*", "schwabacher", "rotunda", "germania"];
const MONO_KW: &[&str] = &["mono", "monospace", "monospaced", "typewriter", "courier", "consolas", "code", "terminal"];
const SCRIPT_KW: &[&str] = &["script", "brush", "calligra*", "cursive", "handwrit*", "signature", "marker", "lettering", "copperplate", "spencer"];
const DISPLAY_KW: &[&str] = &["stencil", "inline", "outline", "shadow", "fat face", "poster", "decorative", "ornate", "rounded", "blobby", "pixel", "wacky"];
const SERIF_KW: &[&str] = &["serif", "garamond", "caslon", "baskerville", "didot", "bodoni", "slab", "roman", "times", "georgia", "palatino", "antiqua"];
const SANS_KW: &[&str] = &["sans", "grotesk", "grotesque", "gothic", "helvet*", "arial", "neue"];

/// Tên đã tách từ một lần (family + full + postscript, chữ thường).
struct Nm<'a> {
    raw: &'a str,
    toks: Vec<&'a str>,
}

impl<'a> Nm<'a> {
    fn new(raw: &'a str) -> Self {
        Nm { raw, toks: raw.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect() }
    }
}

fn has_kw(n: &Nm, kws: &[&str]) -> Option<String> {
    kws.iter()
        .find(|k| match k.strip_suffix('*') {
            Some(p) => n.toks.iter().any(|t| t.starts_with(p)),
            None if k.contains(' ') => n.raw.contains(**k),
            None => n.toks.iter().any(|t| t == *k),
        })
        .map(|k| k.trim_end_matches('*').to_string())
}

struct Votes {
    list: Vec<Signal>,
}

impl Votes {
    fn add(&mut self, source: &str, detail: impl Into<String>, vote: impl Into<Option<Class>>, weight: f32) {
        self.list.push(Signal { source: source.into(), detail: detail.into(), vote: vote.into(), weight });
    }

    /// Bằng chứng "nổi bật / dùng cho cỡ lớn" (không bỏ phiếu cho class nào).
    fn display_ev(&self) -> f32 {
        self.list.iter().filter(|s| s.vote.is_none()).map(|s| s.weight).sum()
    }

    fn score(&self, c: Class) -> f32 {
        self.list.iter().filter(|s| s.vote == Some(c)).map(|s| s.weight).sum()
    }
}

/// Thu thập mọi tín hiệu (bỏ phiếu cho class hoặc chỉ ủng hộ usage Display/traits).
fn gather(i: &ClassifyInput, nm: &Nm) -> Votes {
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
    if let Some(k) = has_kw(nm, SYMBOL_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Symbol, 2.0);
    }
    if i.latin_letters.is_some_and(|n| n < 10) && !v.list.is_empty() {
        v.add("cmap", "gần như không có chữ cái Latin", Class::Symbol, 1.0);
    }
    // --- Blackletter
    if let Some(k) = has_kw(nm, BLACKLETTER_KW) {
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
    if let Some(k) = has_kw(nm, MONO_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Mono, 1.0);
    }
    // --- Script
    if p.is_some_and(|p| p[0] == 3) {
        v.add("panose", "family kind = 3 (Latin Hand Written)", Class::Script, 2.5);
    }
    if i.family_class.is_some_and(|c| c.0 == 10) {
        v.add("os2_class", "sFamilyClass = 10 (Scripts)", Class::Script, 2.0);
    }
    if let Some(k) = has_kw(nm, SCRIPT_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Script, 1.5);
    }
    if !i.italic && m.slant_abs.is_some_and(|s| s >= 10.0) && m.spacing.is_some_and(|s| s < 0.06) {
        v.add("measure", "nghiêng mạnh, sidebearing rất nhỏ (chữ nối)", Class::Script, 1.0);
    }
    // --- Display
    if p.is_some_and(|p| p[0] == 4) {
        v.add("panose", "family kind = 4 (Latin Decorative)", None, 2.0);
    }
    if i.family_class.is_some_and(|c| c.0 == 9) {
        v.add("os2_class", "sFamilyClass = 9 (Ornamentals)", None, 1.5);
    }
    if let Some(k) = has_kw(nm, DISPLAY_KW) {
        v.add("name", format!("từ khoá \"{k}\""), None, 1.5);
    }
    // --- Dấu hiệu "không phải chữ văn bản" từ số đo (font GF hầu như không khai Panose/sFamilyClass)
    fn cue(v: &mut Votes, cond: bool, what: String, vote: impl Into<Option<Class>>, w: f32) {
        if cond {
            v.add("measure", what, vote, w);
        }
    }
    if let Some(x) = m.spacing {
        cue(&mut v, x < 0.04, format!("sidebearing rất nhỏ ({x:.2})"), Class::Script, 1.5);
        cue(&mut v, (0.04..0.06).contains(&x), format!("sidebearing nhỏ ({x:.2})"), Class::Script, 0.7);
    }
    if let Some(x) = m.slant_abs.filter(|_| !i.italic) {
        cue(&mut v, x >= 5.0, format!("nghiêng {x:.0}°"), Class::Script, 1.5);
        cue(&mut v, (3.0..5.0).contains(&x), format!("nghiêng nhẹ {x:.0}°"), Class::Script, 0.5);
    }
    if let Some(x) = m.connect {
        cue(&mut v, x >= 0.8, format!("{:.0}% chữ a–z chạm cả hai mép advance (nối nét)", x * 100.0), Class::Script, 1.5);
        cue(&mut v, (0.5..0.8).contains(&x), format!("{:.0}% chữ a–z chạm cả hai mép advance", x * 100.0), Class::Script, 0.8);
    }
    if let Some(x) = m.bounce {
        cue(&mut v, x >= 0.10, format!("baseline/x-height nhảy mạnh ({x:.3})"), Class::Script, 2.0);
        cue(&mut v, (0.04..0.10).contains(&x), format!("baseline/x-height nhảy ({x:.3})"), Class::Script, 1.2);
        cue(&mut v, (0.025..0.04).contains(&x), format!("baseline/x-height hơi nhảy ({x:.3})"), Class::Script, 0.5);
    }
    if let Some(x) = m.x_height_em {
        cue(&mut v, x <= 0.38, format!("x-height thấp {x:.2}"), Class::Script, 0.8);
        cue(&mut v, x >= 0.62, format!("x-height rất cao {x:.2}"), None, 0.8);
    }
    if let Some(x) = m.contrast {
        cue(&mut v, x >= 6.0, format!("contrast rất cao {x:.1}"), None, 1.0);
    }
    if let Some(x) = m.stem {
        cue(&mut v, x >= 0.30, format!("nét rất đậm {x:.2}"), None, 0.8);
        cue(&mut v, x <= 0.08, format!("nét rất mảnh {x:.2}"), None, 0.8);
    }
    if let Some(x) = m.width {
        cue(&mut v, x <= 0.65, format!("rất hẹp {x:.2}"), None, 0.8);
        cue(&mut v, x >= 1.2, format!("rất rộng {x:.2}"), None, 0.6);
    }
    if let Some(x) = m.serif_foot {
        cue(&mut v, x < 0.9, format!("serif_foot bất thường {x:.2}"), None, 0.7);
    }
    if let Some(x) = m.width_cv {
        cue(&mut v, x < 0.18 && !i.is_fixed_pitch, format!("bề rộng chữ đều bất thường {x:.2}"), None, 0.8);
    }
    if let Some(x) = m.aperture {
        cue(&mut v, x < 45.0, format!("aperture rất khép {x:.0}°"), None, 0.6);
    }
    if let Some(x) = m.stress_abs {
        cue(&mut v, x >= 18.0, format!("trục nhấn lệch {x:.0}°"), None, 0.7);
    }
    // --- Serif / Sans
    if let Some(sf) = m.serif_foot {
        let (flare, head) = (m.serif_flare.unwrap_or(0.0), m.serif_head.unwrap_or(0.0));
        let small_foot_ev = small_foot_weight(flare, head);
        if sf > 1.6 {
            v.add("measure", format!("serif_foot {sf:.2} > 1,6"), Class::Serif, 2.0);
        } else if sf < 1.2 {
            // chân nhọn: rộng ~0 ngay tại baseline nên `serif_foot` đo thấp dù có cờ/chân loe → không kết luận Sans
            if sf < 0.9 && small_foot_ev > 0.0 {
                v.add("measure", format!("serif_foot {sf:.2} thấp nhưng có cờ {head:.2} / loe {flare:.2} (chân nhọn)"), Class::Serif, 1.0);
            } else {
                v.add("measure", format!("serif_foot {sf:.2} < 1,2"), Class::Sans, 2.0);
            }
        } else if small_foot_ev > 0.0 {
            // vùng giữa 1.2–1.6: serif chân nhỏ. Cờ trên-trái và độ loe quyết định; tên (Neue/Grotesk) chỉ nặng 0.8.
            v.add("measure", format!("chân nhỏ: serif_foot {sf:.2}, loe {flare:.2}, cờ {head:.2}"), Class::Serif, small_foot_ev);
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
    if let Some(k) = has_kw(nm, SERIF_KW).filter(|k| k != "serif" || !i.name.contains("sans")) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Serif, 0.8);
    }
    if let Some(k) = has_kw(nm, SANS_KW) {
        v.add("name", format!("từ khoá \"{k}\""), Class::Sans, 0.8);
    }

    v
}

pub fn classify(i: &ClassifyInput) -> Classification {
    let nm = Nm::new(&i.name);
    let v = gather(i, &nm);
    let (class, confidence) = decide(&v);
    let (class, confidence, candidate) = calibrate(&v, i, class, confidence);
    let subclass = if class == Class::Uncertain { None } else { subclass_of(class, i, &nm) };
    let traits = traits_of(i, &nm);
    let usage = usage_of(candidate.unwrap_or(class), i, &v, &traits);
    Classification { class, subclass, confidence, candidate, traits, signals: v.list, usage }
}

/// Chọn class theo hình dáng: Symbol, Blackletter, Mono, Script cần đủ điểm; còn lại Serif/Sans. Display không phải class.
/// Tổng điểm Script+Display tối thiểu để coi là font không dùng cho chữ văn bản.
/// Độ loe chân (`serif_flare`) từ đây là có chân nhỏ; từ STRONG là chân nhỏ rõ.
const SMALL_FOOT_FLARE: f32 = 1.3;
const SMALL_FOOT_STRONG: f32 = 1.5;
/// Bằng chứng serif chân nhỏ từ độ loe của chân (`serif_flare`) và cờ trên-trái (`serif_head`). 0 = không có.
fn small_foot_weight(flare: f32, head: f32) -> f32 {
    if flare > 1.6 || head >= SMALL_HEAD_STRONG {
        2.0
    } else if flare >= SMALL_FOOT_STRONG || head >= SMALL_HEAD {
        1.5
    } else if flare >= SMALL_FOOT_FLARE || head >= SMALL_HEAD_WEAK {
        1.0
    } else {
        0.0
    }
}
/// Cờ trên-trái ÷ thân: từ WEAK là có dấu hiệu, từ SMALL_HEAD khá rõ, từ STRONG là rõ.
const SMALL_HEAD_WEAK: f32 = 0.15;
const SMALL_HEAD: f32 = 0.25;
const SMALL_HEAD_STRONG: f32 = 0.4;
const ODD_MIN: f32 = 1.6;
const ODD_MARGIN: f32 = 1.0;
/// Bằng chứng Display tối thiểu để thêm tag usage Display cho font chữ văn bản (Display không còn là class).
const DISPLAY_MIN: f32 = 2.0;

fn decide(v: &Votes) -> (Class, f32) {
    for (c, min) in [(Class::Symbol, 2.0), (Class::Blackletter, 2.5), (Class::Mono, 2.0)] {
        if v.score(c) >= min {
            return (c, (v.score(c) / (v.score(c) + 1.0)).min(0.99));
        }
    }
    let script = v.score(Class::Script);
    let display = v.display_ev();
    let (serif, sans) = (v.score(Class::Serif), v.score(Class::Sans));
    let text = serif.max(sans);
    let odd = script + display;
    // Script: bằng chứng chữ viết/nối mạnh hơn bằng chứng Display
    // bằng chứng "chữ văn bản" (serif_foot rõ, Panose, sFamilyClass) cạnh tranh với "không phải chữ văn bản": nhập nhằng → Uncertain
    let contested = odd < text * ODD_MARGIN && text >= 3.0;
    if script >= 1.5 && script >= display && odd >= ODD_MIN {
        return if contested { (Class::Uncertain, 0.0) } else { (Class::Script, (odd / (odd + 1.5)).min(0.99)) };
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

/// Độ tin cậy hiệu chỉnh: hồi quy logistic (L2) fit trên split `tune` của GF (973 họ), P(đúng | đặc trưng).
/// Đặc trưng: class đoán (one-hot Serif/Sans/Script/Mono), lề điểm, bằng chứng nổi bật (Display) và Script còn lại, và vài số đo "văn bản điển hình".
/// Dưới `CONF_MIN` thì trả `Uncertain` (giữ `candidate`). Sinh bởi `docs/team/eval/classify_calibrate.py`.
pub const CONF_MIN: f32 = 0.88;
/// Cộng thêm vào CONF_MIN cho mặt italic.
pub const ITALIC_EXTRA: f32 = 0.06;
const CAL_MU: [f32; 15] = [0.0000, 0.0000, 0.0000, 0.0000, 2.5125, 0.2636, 0.0791, 0.2490, 0.1315, 0.2184, 0.4320, 0.5871, 1.7687, 0.0297, 0.2388];
const CAL_SD: [f32; 15] = [1.0000, 1.0000, 1.0000, 1.0000, 1.2414, 0.5945, 0.2721, 0.0609, 0.0779, 0.0981, 0.1268, 0.5537, 0.8013, 0.0454, 0.2503];
const CAL_W: [f32; 15] = [0.0936, -0.0304, -0.0415, -0.0217, 0.6601, -0.1710, -0.1775, 0.2951, 0.0727, 0.3341, 0.0523, -0.0254, -0.0173, 0.1259, -0.1350];
const CAL_B: f32 = 2.4852;

fn calibrate(v: &Votes, i: &ClassifyInput, class: Class, conf: f32) -> (Class, f32, Option<Class>) {
    const SHAPES: [Class; 4] = [Class::Serif, Class::Sans, Class::Script, Class::Mono];
    let Some(own) = SHAPES.iter().position(|c| *c == class) else {
        return (class, conf, None); // Symbol/Blackletter/Uncertain giữ nguyên
    };
    // Tín hiệu cứng (post.isFixedPitch hoặc bề rộng a–z đều tuyệt đối) và không có bằng chứng Script/Display
    // ngược lại: giữ Mono, không để hiệu chỉnh thống kê hạ về Uncertain.
    if class == Class::Mono
        && (i.is_fixed_pitch || i.m.width_cv.is_some_and(|x| x.abs() < 0.005))
        && v.score(Class::Script) + v.display_ev() < ODD_MIN
    {
        return (class, conf, None);
    }
    let m = &i.m;
    let sc = SHAPES.map(|c| v.score(c));
    let other = sc.iter().enumerate().filter(|(k, _)| *k != own).map(|(_, x)| *x).fold(f32::MIN, f32::max);
    let g = |x: Option<f32>, a: f32, b: f32, miss: f32| x.unwrap_or(miss).clamp(a, b);
    let mut x = [0.0f32; 15];
    x[own] = 1.0;
    x[4] = (sc[own] - other).clamp(-2.0, 5.0);
    // Bằng chứng nổi bật (Display) và Script còn lại tính như "nhiễu" cho class văn bản; Script không bị phạt bởi chính nó.
    let is_script = class == Class::Script;
    x[5] = if is_script { 0.0 } else { v.display_ev().clamp(0.0, 4.0) };
    x[6] = if is_script { 0.0 } else { sc[2].clamp(0.0, 4.0) };
    x[7] = g(m.width_cv, 0.0, 0.5, 0.25);
    x[8] = g(m.spacing, 0.0, 0.4, 0.15);
    x[9] = g(m.aperture_e, 0.0, 0.4, 0.2);
    x[10] = g(m.counter, 0.0, 0.7, 0.45);
    x[11] = g(m.contrast, 1.0, 8.0, 1.5).ln();
    x[12] = g(m.terminal, 0.0, 3.0, 1.2);
    x[13] = g(m.bounce, 0.0, 0.2, 0.015);
    x[14] = g(m.connect, 0.0, 1.0, 0.2);
    let z = CAL_B + (0..15).map(|k| CAL_W[k] * (x[k] - CAL_MU[k]) / CAL_SD[k]).sum::<f32>();
    let p = 1.0 / (1.0 + (-z).exp());
    // Hiệu chỉnh fit trên mặt Regular; mặt italic (cue hình học lệch) đòi ngưỡng cao hơn. Chọn trên tune (mọi mặt).
    let min = if i.italic { CONF_MIN + ITALIC_EXTRA } else { CONF_MIN };
    if p < min { (Class::Uncertain, p, Some(class)) } else { (class, p, None) }
}

fn kw(n: &Nm, kws: &[&str]) -> bool {
    has_kw(n, kws).is_some()
}

/// Subclass cho một class đã biết (dùng cả khi class bị hạ về Uncertain vì độ tin cậy thấp — để đo riêng).
pub fn subclass_for(i: &ClassifyInput, class: Class) -> Option<String> {
    subclass_of(class, i, &Nm::new(&i.name))
}

fn subclass_of(class: Class, i: &ClassifyInput, nm: &Nm) -> Option<String> {
    let m = &i.m;
    let sfc = i.family_class;
    let s = |x: &str| Some(x.to_string());
    match class {
        Class::Serif => {
            if kw(nm, &["slab", "rockwell", "clarendon", "egyptian", "memphis"]) || sfc.is_some_and(|c| c.0 == 5) {
                return s("slab");
            }
            if kw(nm, &["glyphic", "wedge", "trajan", "optima", "marcellus", "cinzel"]) {
                return s("glyphic");
            }
            if kw(nm, &["fat face", "fatface"]) {
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
            // Luật fit trên split tune (cây quyết định + quét ngưỡng); grotesque ↔ neo_grotesque vẫn nhập nhằng.
            // 'a' một tầng: geometric.
            if a < 1.5 {
                return s("geometric");
            }
            // aperture rất mở: humanist
            if ap >= 92.0 {
                return s("humanist");
            }
            // chữ tròn rộng, contrast thấp, aperture khép: geometric
            if m.width.is_some_and(|w| w >= 0.97) && m.contrast.is_some_and(|c| c <= 1.34) {
                return s("geometric");
            }
            // đầu 'e' mở: humanist
            if m.aperture_e.is_some_and(|e| e > 0.2168) {
                return s("humanist");
            }
            if ap < 68.0 || m.terminal.is_some_and(|t| t >= 1.55) { s("grotesque") } else { s("neo_grotesque") }
        }
        Class::Mono => {
            // serif_foot đo trên font mono không đáng tin (i/l/j vốn có chân kể cả sans mono; tune: 19/19 sans mono,
            // serif_foot trung vị 2.8) nên chỉ gán serif_mono khi tên/Panose/sFamilyClass khai báo serif.
            let p = i.panose.filter(|p| p[0] == 2);
            let named_sans = i.name.contains("sans") || kw(nm, &["gothic", "grotesk"]);
            let serif = !named_sans
                && (kw(nm, &["courier", "typewriter", "mincho", "batang", "slab", "serif"])
                    || p.is_some_and(|p| matches!(p[1], 2..=10 | 14))
                    || sfc.is_some_and(|c| matches!(c.0, 1..=5 | 7)));
            if serif {
                s("serif_mono")
            } else if m.serif_foot.is_some() || named_sans || p.is_some_and(|p| matches!(p[1], 11..=13 | 15)) {
                s("sans_mono")
            } else {
                None
            }
        }
        Class::Script => {
            if kw(nm, &["brush", "marker", "paint", "casual", "felt"]) {
                return s("casual_brush");
            }
            if kw(nm, &["copperplate", "spencer", "formal", "calligra", "chancery", "roundhand"]) || m.contrast.is_some_and(|c| c > 3.0) {
                return s("formal");
            }
            s("handwritten")
        }
        _ => None,
    }
}

/// Tag dùng cho một class cho trước (kể cả class do người dùng ghi đè).
pub fn usage_for(i: &ClassifyInput, class: Class) -> Vec<Usage> {
    let nm = Nm::new(&i.name);
    let v = gather(i, &nm);
    usage_of(class, i, &v, &traits_of(i, &nm))
}

/// Tag hình thái từ tên (và Panose/sFamilyClass): độc lập với class. Chỉ các dấu hiệu rõ.
fn traits_of(i: &ClassifyInput, nm: &Nm) -> Vec<String> {
    let mut t = Vec::new();
    let mut add = |c: bool, name: &str| {
        if c {
            t.push(name.to_string());
        }
    };
    add(kw(nm, &["stencil"]), "stencil");
    add(kw(nm, &["inline", "outline", "shadow", "shaded"]), "inline_outline");
    add(kw(nm, &["rounded"]), "rounded");
    add(kw(nm, &["fat face", "fatface"]), "fat_face");
    add(kw(nm, &["pixel*", "bitmap"]), "pixel");
    add(
        kw(nm, &["decorative", "ornate", "blobby", "wacky", "woodtype", "tuscan", "deco", "nouveau", "distressed", "grunge"])
            || i.panose.is_some_and(|p| p[0] == 4)
            || i.family_class.is_some_and(|c| c.0 == 9),
        "decorative",
    );
    t
}

fn usage_of(class: Class, i: &ClassifyInput, v: &Votes, traits: &[String]) -> Vec<Usage> {
    let m = &i.m;
    let mut out = Vec::new();
    // Display là tag mục đích: Script/Blackletter, font có trait hình thái, hoặc đủ bằng chứng nổi bật
    let showy = !traits.is_empty() || v.display_ev() >= DISPLAY_MIN;
    match class {
        Class::Script | Class::Blackletter => out.push(Usage::Display),
        Class::Serif | Class::Sans | Class::Mono => {
            let contrast = m.contrast.unwrap_or(1.5);
            if showy || contrast >= 4.0 || i.opsz_max.is_some_and(|o| o >= 72.0) {
                out.push(Usage::Display);
            }
            if !showy && contrast < 4.0 && m.x_height_em.is_none_or(|x| x >= 0.44) && m.spacing.is_none_or(|s| s >= 0.08) {
                out.push(Usage::Text);
            }
            if class != Class::Serif && contrast < 1.6 && m.x_height_em.is_some_and(|x| x >= 0.5) {
                out.push(Usage::Ui);
            }
        }
        _ => {
            if showy {
                out.push(Usage::Display);
            }
        }
    }
    out
}
