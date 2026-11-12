//! G4: điểm "hợp vai" và "phong cách pair". Hàm thuần trên số đo của analyzer, không I/O.
//!
//! Không có điểm số trong kết quả: chỉ mức `Fit` (good/ok/weak/unknown) cùng mã lý do `Reason { code, params }`
//! (frontend dịch vi/en). Điểm 0..1 chỉ tồn tại nội bộ để quyết định mức. Số đo thiếu thì bỏ tiêu chí đó; thiếu quá
//! nửa trọng số thì `Unknown` (không bao giờ điền mặc định). Mô hình và ngưỡng khởi điểm:
//! `docs/team/research/2026-11-12-g4-role-fit-and-pair-styles.md` (ngưỡng là giả thuyết, cần hiệu chuẩn bằng eval).
use serde::{Deserialize, Serialize};

use crate::classify::Class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Không đủ số đo để kết luận.
    Unknown,
    Weak,
    Ok,
    Good,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Body,
    Caption,
    Heading,
    Display,
    Quote,
    Label,
    Numerals,
}

/// Mã lý do + tham số số, frontend dịch.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reason {
    pub code: &'static str,
    pub params: Vec<(&'static str, f32)>,
}

impl Reason {
    fn new(code: &'static str) -> Self {
        Reason { code, params: vec![] }
    }
    fn with(code: &'static str, k: &'static str, v: f32) -> Self {
        Reason { code, params: vec![(k, v)] }
    }
}

/// Số đo một face (dựng từ `Anatomy` + phân loại + phủ tiếng Việt). `None` = chưa đo được.
#[derive(Debug, Clone)]
pub struct Traits {
    pub class: Class,
    pub subclass: Option<String>,
    pub x_height_em: Option<f32>,
    /// x-height ÷ cap-height (≥ 0.95: chữ hoa kiểu Bebas).
    pub x_height_cap: Option<f32>,
    pub aperture: Option<f32>,
    pub aperture_e: Option<f32>,
    pub contrast: Option<f32>,
    pub stem: Option<f32>,
    pub spacing: Option<f32>,
    pub width: Option<f32>,
    pub stress_deg: Option<f32>,
    pub a_storeys: Option<f32>,
    pub g_storeys: Option<f32>,
    /// Số ký tự tiếng Việt còn thiếu.
    pub vi_missing: Option<u32>,
    /// Có số tabular (`tnum`).
    pub has_tnum: Option<bool>,
}

impl Default for Traits {
    fn default() -> Self {
        Traits {
            class: Class::Uncertain,
            subclass: None,
            x_height_em: None,
            x_height_cap: None,
            aperture: None,
            aperture_e: None,
            contrast: None,
            stem: None,
            spacing: None,
            width: None,
            stress_deg: None,
            a_storeys: None,
            g_storeys: None,
            vi_missing: None,
            has_tnum: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoleFit {
    pub fit: Fit,
    /// Tối đa 3, đã sắp theo mức quan trọng (cổng trước, rồi tiêu chí kém nhất).
    pub reasons: Vec<Reason>,
}

/// `[lo, hi]` điểm 1; ra ngoài giảm tuyến tính về 0 sau `margin`.
fn band(v: f32, lo: f32, hi: f32, margin: f32) -> f32 {
    let d = if v < lo {
        lo - v
    } else if v > hi {
        v - hi
    } else {
        0.0
    };
    (1.0 - d / margin).clamp(0.0, 1.0)
}

struct Crit {
    code: &'static str,
    weight: f32,
    score: Option<f32>,
    /// Mã khi quá thấp / quá cao (v < lo / v > hi) hoặc chung.
    low: &'static str,
    high: &'static str,
    value: Option<f32>,
    lo: f32,
}

#[allow(clippy::too_many_arguments)] // bảng khai báo tiêu chí; tuple hoá chỉ làm bảng khó đọc hơn
fn crit(
    code: &'static str,
    weight: f32,
    v: Option<f32>,
    lo: f32,
    hi: f32,
    margin: f32,
    low: &'static str,
    high: &'static str,
) -> Crit {
    Crit { code, weight, score: v.map(|x| band(x, lo, hi, margin)), low, high, value: v, lo }
}

fn expressive_class(c: Class) -> bool {
    matches!(c, Class::Display | Class::Script | Class::Blackletter | Class::Handwriting)
}

/// 0..1: độ cá tính (dùng cho Display và `display_vs_neutral`). `None` nếu không có dữ kiện.
fn distinctiveness(t: &Traits) -> Option<f32> {
    if expressive_class(t.class) {
        return Some(1.0);
    }
    // Class hay là Uncertain với font cá tính (Pacifico, Bebas): lấy thêm dấu hiệu từ số đo, cái nào mạnh nhất.
    let parts = [
        t.contrast.map(|c| (c - 2.0) / 4.0),
        t.stem.map(|s| (s - 0.22) / 0.12),
        t.aperture_e.map(|a| (0.12 - a) / 0.08),
        t.x_height_cap.map(|r| if r >= 0.95 { 0.8 } else { 0.0 }),
    ];
    parts.into_iter().flatten().reduce(f32::max).map(|v| v.clamp(0.0, 1.0))
}

/// Chữ hoa kiểu Bebas: "x-height" bằng chiều cao chữ hoa (không có chữ thường thật).
fn caps_only(t: &Traits) -> bool {
    t.x_height_cap.is_some_and(|r| r >= 0.95)
}

fn level(score: f32) -> Fit {
    if score >= 0.75 {
        Fit::Good
    } else if score >= 0.5 {
        Fit::Ok
    } else {
        Fit::Weak
    }
}

pub fn role_fit(t: &Traits, role: Role) -> RoleFit {
    let mut cs: Vec<Crit> = match role {
        Role::Body => vec![
            // Vùng đích hiệu chuẩn theo phân vị 10–90 của 1722 họ OFL (Serif+Sans, mặt Regular), docs/team/eval/pairing/README.md.
            crit("x_height", 3.0, t.x_height_em, 0.46, 0.56, 0.08, "x_height_low", "x_height_high"),
            crit("aperture", 2.0, t.aperture, 66.0, 110.0, 25.0, "aperture_low", "aperture_high"),
            crit("aperture_e", 2.0, t.aperture_e, 0.17, 0.37, 0.10, "aperture_low", "aperture_high"),
            crit("contrast", 2.0, t.contrast, 1.0, 3.5, 3.0, "contrast_low", "contrast_high"),
            crit("spacing", 1.0, t.spacing, 0.08, 0.20, 0.06, "spacing_tight", "spacing_loose"),
            crit("stem", 1.0, t.stem, 0.13, 0.20, 0.05, "stem_light", "stem_heavy"),
        ],
        Role::Caption => vec![
            crit("x_height", 3.0, t.x_height_em, 0.50, 0.60, 0.06, "x_height_low", "x_height_high"),
            crit("aperture", 2.0, t.aperture, 75.0, 110.0, 25.0, "aperture_low", "aperture_high"),
            crit("contrast", 2.0, t.contrast, 1.0, 3.0, 3.0, "contrast_low", "contrast_high"),
            crit("spacing", 2.0, t.spacing, 0.10, 0.22, 0.06, "spacing_tight", "spacing_loose"),
            crit("stem", 2.0, t.stem, 0.14, 0.22, 0.05, "stem_light", "stem_heavy"),
        ],
        Role::Heading => vec![
            crit("stem", 2.0, t.stem, 0.12, 0.30, 0.08, "stem_light", "stem_heavy"),
            crit("contrast", 2.0, t.contrast, 1.0, 6.0, 4.0, "contrast_low", "contrast_high"),
            crit("spacing", 1.0, t.spacing, 0.05, 0.22, 0.06, "spacing_tight", "spacing_loose"),
        ],
        Role::Display => vec![
            Crit {
                code: "distinctiveness",
                weight: 3.0,
                score: distinctiveness(t),
                low: "distinctiveness_low",
                high: "distinctiveness_low",
                value: distinctiveness(t),
                lo: 0.7,
            },
            crit("spacing", 1.0, t.spacing, 0.0, 0.20, 0.06, "spacing_tight", "spacing_loose"),
        ],
        Role::Quote => vec![
            crit("x_height", 1.0, t.x_height_em, 0.42, 0.56, 0.08, "x_height_low", "x_height_high"),
            crit("contrast", 2.0, t.contrast, 1.5, 5.0, 3.0, "contrast_low", "contrast_high"),
            crit("spacing", 1.0, t.spacing, 0.06, 0.18, 0.06, "spacing_tight", "spacing_loose"),
        ],
        Role::Label => vec![
            crit("aperture", 2.0, t.aperture, 66.0, 110.0, 25.0, "aperture_low", "aperture_high"),
            crit("stem", 2.0, t.stem, 0.13, 0.26, 0.06, "stem_light", "stem_heavy"),
            crit("width", 1.0, t.width, 0.70, 1.10, 0.20, "width_narrow", "width_wide"),
        ],
        Role::Numerals => vec![
            crit("stem", 2.0, t.stem, 0.12, 0.26, 0.06, "stem_light", "stem_heavy"),
            crit("spacing", 1.0, t.spacing, 0.08, 0.20, 0.06, "spacing_tight", "spacing_loose"),
        ],
    };

    let total: f32 = cs.iter().map(|c| c.weight).sum();
    let avail: f32 = cs.iter().filter(|c| c.score.is_some()).map(|c| c.weight).sum();
    let mut reasons: Vec<Reason> = vec![];

    // Cổng: mức trần + lý do đứng đầu.
    let mut cap = Fit::Good;
    if let Some(n) = t.vi_missing.filter(|n| *n > 0) {
        let c = match role {
            Role::Body | Role::Caption | Role::Label => Fit::Weak,
            Role::Heading | Role::Quote | Role::Display => Fit::Ok,
            Role::Numerals => Fit::Good,
        };
        if c < Fit::Good {
            cap = cap.min(c);
            reasons.push(Reason::with("vi_missing", "count", n as f32));
        }
    }
    let unfit = match role {
        Role::Body | Role::Caption => expressive_class(t.class) || t.class == Class::Symbol || caps_only(t),
        Role::Label => expressive_class(t.class) || t.class == Class::Symbol,
        Role::Heading | Role::Quote => t.class == Class::Symbol,
        _ => false,
    };
    if unfit {
        cap = cap.min(Fit::Weak);
        reasons.push(Reason::new("class_unfit"));
    }
    if role == Role::Numerals && t.has_tnum == Some(false) {
        cap = cap.min(Fit::Ok);
        reasons.push(Reason::new("tnum_missing"));
    }

    if avail * 2.0 < total {
        return RoleFit { fit: Fit::Unknown, reasons };
    }
    let sum: f32 = cs.iter().filter_map(|c| c.score.map(|s| s * c.weight)).sum();
    let base = level(sum / avail);
    let fit = base.min(cap);

    // Lý do tiêu chí: kém nhất trước (< 0.75); nếu không có thì hai tiêu chí tốt nhất.
    cs.retain(|c| c.score.is_some());
    cs.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    let weak: Vec<&Crit> = cs.iter().filter(|c| c.score.unwrap_or(1.0) < 0.75).collect();
    if weak.is_empty() {
        for c in cs.iter().rev().take(2) {
            reasons.push(Reason::with(ok_code(c.code), "value", c.value.unwrap_or(0.0)));
        }
    } else {
        for c in weak {
            let v = c.value.unwrap_or(0.0);
            let code = if v < c.lo { c.low } else { c.high };
            reasons.push(Reason::with(code, "value", v));
        }
    }
    reasons.truncate(3);
    RoleFit { fit, reasons }
}

fn ok_code(c: &'static str) -> &'static str {
    match c {
        "x_height" => "x_height_ok",
        "aperture" | "aperture_e" => "aperture_open",
        "contrast" => "contrast_ok",
        "spacing" => "spacing_ok",
        "stem" => "stem_ok",
        "width" => "width_ok",
        "distinctiveness" => "distinctive",
        _ => "criterion_ok",
    }
}

// ---------------------------------------------------------------- vai tuỳ ý (G3)

fn fold(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ' | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => 'e',
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ' | 'ợ' | 'ở' | 'ỡ' => 'o',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => 'u',
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'đ' => 'd',
            c => c,
        })
        .collect()
}

/// Tên vai tự do → hồ sơ vai gần nhất (bảng đồng nghĩa; không nhận ra → `None`, UI hỏi hồ sơ hoặc `fit = unknown`).
pub fn role_from_name(name: &str) -> Option<Role> {
    let f = fold(name);
    let f = f.trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ');
    let table: &[(&[&str], Role)] = &[
        (&["body", "paragraph", "text", "noi dung", "doan van", "van ban"], Role::Body),
        (&["caption", "note", "footnote", "chu thich", "ghi chu"], Role::Caption),
        (&["heading", "subhead", "subheading", "sub heading", "title", "subtitle", "tagline", "tieu de", "tieu de phu"], Role::Heading),
        (&["display", "hero", "headline", "tieu de lon"], Role::Display),
        (&["quote", "pull quote", "pullquote", "trich dan"], Role::Quote),
        (&["label", "eyebrow", "tag", "button", "nhan", "nut"], Role::Label),
        (&["numerals", "numbers", "number", "figures", "price", "gia", "so"], Role::Numerals),
    ];
    table.iter().find(|(ks, _)| ks.contains(&f)).map(|(_, r)| *r)
}

// ---------------------------------------------------------------- phong cách pair

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairStyle {
    Contrast,
    Superfamily,
    SameStructure,
    SameEra,
    DisplayVsNeutral,
    Harmony,
}

impl PairStyle {
    pub const ALL: [PairStyle; 6] = [
        PairStyle::Contrast,
        PairStyle::Superfamily,
        PairStyle::SameStructure,
        PairStyle::SameEra,
        PairStyle::DisplayVsNeutral,
        PairStyle::Harmony,
    ];
    pub fn id(self) -> &'static str {
        match self {
            PairStyle::Contrast => "contrast",
            PairStyle::Superfamily => "superfamily",
            PairStyle::SameStructure => "same_structure",
            PairStyle::SameEra => "same_era",
            PairStyle::DisplayVsNeutral => "display_vs_neutral",
            PairStyle::Harmony => "harmony",
        }
    }
    pub fn from_id(id: &str) -> Option<PairStyle> {
        PairStyle::ALL.into_iter().find(|s| s.id() == id)
    }
}

/// Dữ kiện của cặp mà số đo từng font không có.
#[derive(Debug, Clone, Default)]
pub struct PairCtx {
    /// Cùng gốc tên họ (`family_root` bằng nhau).
    pub same_family_root: bool,
    /// Cùng nhà thiết kế/nhà làm (tên hãng khớp).
    pub same_foundry: bool,
    /// Độ giống ảnh (embedding, cosine). `None` khi chưa có embedding.
    pub embed_cos: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StyleFit {
    pub fit: Fit,
    pub reasons: Vec<Reason>,
    /// Lưu ý không chặn (`xheight_mismatch`, `too_close_not_same`).
    pub warnings: Vec<Reason>,
}

/// Gốc tên họ: bỏ chữ chỉ vai/kiểu (Sans, Serif, Mono, Display, Text…) và số phiên bản.
pub fn family_root(name: &str) -> String {
    const DROP: &[&str] = &[
        "sans", "serif", "mono", "display", "text", "condensed", "narrow", "wide", "slab", "rounded", "ui", "soft",
        "headline", "caption", "std", "pro", "variable", "vf",
    ];
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !DROP.contains(w) && !w.chars().all(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn era(sub: &str) -> Option<u8> {
    Some(match sub {
        "venetian_oldstyle" | "garalde" | "humanist_oldstyle" => 0,
        "transitional" => 1,
        "didone" | "clarendon" | "egyptian" | "grotesque" => 2,
        "neo_grotesque" | "geometric" | "humanist" | "flared" | "rounded" | "squarish" | "humanist_slab" => 3,
        _ => return None,
    })
}

fn serifish(c: Class) -> bool {
    matches!(c, Class::Serif | Class::Slab)
}

pub fn pair_style(a: &Traits, b: &Traits, ctx: &PairCtx, style: PairStyle) -> StyleFit {
    let mut reasons = vec![];
    let mut warnings = vec![];
    let mut cap = Fit::Good;

    // Kiểm chung: x-height (không áp khi một bên cá tính) và vùng "gần giống mà không giống".
    if style != PairStyle::DisplayVsNeutral
        && !expressive_class(a.class)
        && !expressive_class(b.class)
        && let (Some(x), Some(y)) = (a.x_height_em, b.x_height_em)
        && (x - y).abs() > 0.041
    {
        warnings.push(Reason::with("xheight_mismatch", "delta", (x - y).abs()));
        cap = cap.min(Fit::Ok);
    }
    if style != PairStyle::Harmony
        && !ctx.same_family_root
        && let Some(c) = ctx.embed_cos
        && (0.55..0.85).contains(&c)
    {
        warnings.push(Reason::with("too_close_not_same", "cosine", c));
        cap = cap.min(Fit::Ok);
    }

    let base = match style {
        PairStyle::Contrast => {
            if a.class == Class::Uncertain || b.class == Class::Uncertain {
                Fit::Unknown
            } else if (serifish(a.class) && b.class == Class::Sans) || (serifish(b.class) && a.class == Class::Sans) {
                reasons.push(Reason::new("serif_vs_sans"));
                Fit::Good
            } else {
                reasons.push(Reason::new("class_not_contrasting"));
                Fit::Weak
            }
        }
        PairStyle::Superfamily => {
            if ctx.same_family_root {
                reasons.push(Reason::new("same_superfamily"));
                Fit::Good
            } else if ctx.same_foundry {
                reasons.push(Reason::new("same_designer"));
                Fit::Ok
            } else {
                reasons.push(Reason::new("not_same_family"));
                Fit::Weak
            }
        }
        PairStyle::SameStructure => {
            let dims: [(Option<f32>, Option<f32>, f32); 7] = [
                (a.x_height_em, b.x_height_em, 0.06),
                (a.contrast, b.contrast, 3.0),
                (a.aperture_e, b.aperture_e, 0.12),
                (a.stem, b.stem, 0.05),
                (a.stress_deg, b.stress_deg, 20.0),
                (a.a_storeys, b.a_storeys, 1.0),
                (a.g_storeys, b.g_storeys, 1.0),
            ];
            let ds: Vec<f32> = dims
                .iter()
                .filter_map(|(x, y, s)| Some(((*x)? - (*y)?).abs() / s))
                .map(|d| d.min(1.0))
                .collect();
            if ds.len() < 3 {
                Fit::Unknown
            } else {
                let mean = ds.iter().sum::<f32>() / ds.len() as f32;
                reasons.push(Reason::with("same_structure", "distance", mean));
                if mean < 0.25 {
                    Fit::Good
                } else if mean < 0.5 {
                    Fit::Ok
                } else {
                    Fit::Weak
                }
            }
        }
        PairStyle::SameEra => match (a.subclass.as_deref().and_then(era), b.subclass.as_deref().and_then(era)) {
            (Some(x), Some(y)) => {
                let d = x.abs_diff(y);
                reasons.push(Reason::with("same_era", "era", x as f32));
                match d {
                    0 => Fit::Good,
                    1 => Fit::Ok,
                    _ => Fit::Weak,
                }
            }
            _ => Fit::Unknown,
        },
        PairStyle::DisplayVsNeutral => match (distinctiveness(a), distinctiveness(b)) {
            (Some(da), Some(db)) => {
                let (ea, eb) = (da >= 0.7, db >= 0.7);
                if ea && eb {
                    reasons.push(Reason::new("both_expressive"));
                    Fit::Weak
                } else if !ea && !eb {
                    reasons.push(Reason::new("no_expressive_side"));
                    Fit::Weak
                } else {
                    let (d, n) = if ea { (a, b) } else { (b, a) };
                    reasons.push(Reason::new("display_vs_neutral"));
                    let f = role_fit(d, Role::Display).fit.min(role_fit(n, Role::Body).fit);
                    if f == Fit::Unknown { Fit::Ok } else { f.min(Fit::Good) }
                }
            }
            _ => Fit::Unknown,
        },
        PairStyle::Harmony => match ctx.embed_cos {
            None => Fit::Unknown,
            Some(c) => {
                reasons.push(Reason::with("image_close", "cosine", c));
                if ctx.same_family_root {
                    Fit::Ok
                } else if c >= 0.85 {
                    Fit::Good
                } else if c >= 0.75 {
                    Fit::Ok
                } else {
                    Fit::Weak
                }
            }
        },
    };
    let fit = if base == Fit::Unknown { base } else { base.min(cap) };
    StyleFit { fit, reasons, warnings }
}

/// Dựng `Traits` từ báo cáo phân tích một face (phân loại bằng `classify_v2`). `has_tnum` để `None`: chưa có số đo.
pub fn traits_from_face(face: &crate::FaceReport) -> Option<Traits> {
    use crate::classify::{ClassifyInput, classify_v2};
    let meta = serde_json::to_value(&face.metadata).ok()?;
    let an = serde_json::to_value(&face.anatomy).ok()?;
    let inp = ClassifyInput::from_json(&meta, &an);
    let c = classify_v2(&inp);
    let m = &inp.m;
    Some(Traits {
        class: c.class,
        subclass: c.subclass.clone(),
        x_height_em: m.x_height_em,
        x_height_cap: face.anatomy.x_height_cap.value,
        aperture: m.aperture,
        aperture_e: m.aperture_e,
        contrast: m.contrast,
        stem: m.stem,
        spacing: m.spacing,
        width: m.width,
        stress_deg: m.stress_abs,
        a_storeys: m.a_storeys,
        g_storeys: m.g_storeys,
        vi_missing: Some(face.vietnamese.missing.len() as u32),
        has_tnum: None,
    })
}
