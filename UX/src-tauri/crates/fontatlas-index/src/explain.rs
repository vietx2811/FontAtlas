//! Lời giải thích song ngữ Việt/Anh, sinh từ mẫu câu; điền số đo thật (không phải phân vị).
//! Trục "chưa chắc" không bao giờ sinh câu về hoà hợp / tương phản / va chạm.
use serde::{Deserialize, Serialize};

use crate::pair_axes::{AxisLabel, PairAxis, Relation};
use crate::pairing::{Block, BodyWarning, ClashKind, Gate, PairAssessment, Side};
use crate::profile::{AxisId, FaceProfile};
use crate::similar::SimilarHit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    Vi,
    En,
}

/// 2 chữ số thập phân; tiếng Việt dùng dấu phẩy.
pub fn fmt_num(v: f32, lang: Lang) -> String {
    let s = format!("{v:.2}");
    match lang {
        Lang::Vi => s.replace('.', ","),
        Lang::En => s,
    }
}

pub fn pair_axis_name(a: PairAxis, lang: Lang) -> &'static str {
    match (a, lang) {
        (PairAxis::XHeight, _) => "x-height",
        (PairAxis::Weight, Lang::Vi) => "độ đậm",
        (PairAxis::Weight, Lang::En) => "weight",
        (PairAxis::Contrast, Lang::Vi) => "tương phản nét",
        (PairAxis::Contrast, Lang::En) => "stroke contrast",
        (PairAxis::Serif, Lang::Vi) => "chân serif",
        (PairAxis::Serif, Lang::En) => "serif",
        (PairAxis::Width, Lang::Vi) => "độ rộng",
        (PairAxis::Width, Lang::En) => "width",
        (PairAxis::Aperture, Lang::Vi) => "độ mở",
        (PairAxis::Aperture, Lang::En) => "aperture",
        (PairAxis::Stress, Lang::Vi) => "trục nhấn",
        (PairAxis::Stress, Lang::En) => "stress",
        (PairAxis::Slant, Lang::Vi) => "độ nghiêng",
        (PairAxis::Slant, Lang::En) => "slant",
    }
}

pub fn axis_name(a: AxisId, lang: Lang) -> &'static str {
    match (a, lang) {
        (AxisId::XHeightEm, Lang::Vi) => "x-height / em",
        (AxisId::XHeightEm, Lang::En) => "x-height / em",
        (AxisId::XHeightCap, Lang::Vi) => "x-height / chữ hoa",
        (AxisId::XHeightCap, Lang::En) => "x-height / cap height",
        (AxisId::AscenderEm, Lang::Vi) => "phần trên chữ (ascender)",
        (AxisId::AscenderEm, Lang::En) => "ascender",
        (AxisId::StemNXh, Lang::Vi) => "độ đậm thân chữ",
        (AxisId::StemNXh, Lang::En) => "stem weight",
        (AxisId::Darkness, Lang::Vi) => "độ phủ mực",
        (AxisId::Darkness, Lang::En) => "ink coverage",
        (AxisId::Contrast, Lang::Vi) => "tương phản nét",
        (AxisId::Contrast, Lang::En) => "stroke contrast",
        (AxisId::StressDeg, Lang::Vi) => "trục nhấn",
        (AxisId::StressDeg, Lang::En) => "stress angle",
        (AxisId::Aperture, Lang::Vi) => "độ mở chữ c",
        (AxisId::Aperture, Lang::En) => "aperture of c",
        (AxisId::ApertureE, Lang::Vi) => "độ mở chữ e",
        (AxisId::ApertureE, Lang::En) => "aperture of e",
        (AxisId::Width, Lang::Vi) => "độ rộng chữ o",
        (AxisId::Width, Lang::En) => "width of o",
        (AxisId::MeanAdvance, Lang::Vi) => "độ rộng trung bình",
        (AxisId::MeanAdvance, Lang::En) => "average width",
        (AxisId::WidthCv, Lang::Vi) => "độ đều bề rộng",
        (AxisId::WidthCv, Lang::En) => "width variation",
        (AxisId::Spacing, Lang::Vi) => "khoảng chữ",
        (AxisId::Spacing, Lang::En) => "spacing",
        (AxisId::SlantDeg, Lang::Vi) => "độ nghiêng",
        (AxisId::SlantDeg, Lang::En) => "slant",
        (AxisId::SerifFoot, Lang::Vi) => "chân serif",
        (AxisId::SerifFoot, Lang::En) => "serif",
        (AxisId::Counter, Lang::Vi) => "lòng chữ o",
        (AxisId::Counter, Lang::En) => "counter of o",
    }
}

/// Mẫu câu theo (trục, quan hệ). `{a}`, `{b}` là số đo thật.
fn axis_template(ax: PairAxis, rel: Relation, lang: Lang) -> Option<&'static str> {
    use Lang::{En, Vi};
    use PairAxis as P;
    use Relation as R;
    Some(match (ax, rel, lang) {
        (P::XHeight, R::Harmony, Vi) => {
            "x-height gần nhau ({a} và {b}) — đọc liền mạch khi đặt cạnh nhau"
        }
        (P::XHeight, R::Harmony, En) => {
            "Similar x-heights ({a} vs {b}) — reads evenly side by side"
        }
        (P::XHeight, R::Contrast, Vi) => {
            "x-height chênh rõ ({a} và {b}) — tạo nhịp khác biệt có chủ ý"
        }
        (P::XHeight, R::Contrast, En) => {
            "Clearly different x-heights ({a} vs {b}) — a deliberate change of rhythm"
        }
        (P::XHeight, R::Clash, Vi) => {
            "x-height chênh nửa vời ({a} và {b}) — dòng chữ trông như lệch cỡ"
        }
        (P::XHeight, R::Clash, En) => {
            "x-heights clash ({a} vs {b}) — lines look set at mismatched sizes"
        }

        (P::Weight, R::Harmony, Vi) => "độ đậm gần nhau ({a} và {b}) — hai font đi cùng nhịp",
        (P::Weight, R::Harmony, En) => "Close weight ({a} vs {b}) — the two move at the same pace",
        (P::Weight, R::Contrast, Vi) => {
            "độ đậm chênh rõ ({a} và {b}) — thứ bậc tiêu đề và nội dung dễ thấy"
        }
        (P::Weight, R::Contrast, En) => {
            "Clear weight difference ({a} vs {b}) — hierarchy is easy to see"
        }
        (P::Weight, R::WeightHint, Vi) => {
            "độ đậm chỉ hơi chênh ({a} và {b}) — nên đẩy tiêu đề đậm hơn cho rõ thứ bậc"
        }
        (P::Weight, R::WeightHint, En) => {
            "Slight weight difference ({a} vs {b}) — push the heading bolder for clearer hierarchy"
        }

        (P::Contrast, R::Harmony, Vi) => {
            "tương phản nét gần nhau ({a} và {b}) — cùng một kiểu nét bút"
        }
        (P::Contrast, R::Harmony, En) => {
            "Similar stroke contrast ({a} vs {b}) — the same kind of pen"
        }
        (P::Contrast, R::Contrast, Vi) => {
            "tương phản nét khác hẳn ({a} và {b}) — một bên nét đều, một bên thanh đậm rõ"
        }
        (P::Contrast, R::Contrast, En) => {
            "Very different stroke contrast ({a} vs {b}) — one even, one with marked thick-thin"
        }
        (P::Contrast, R::Clash, Vi) => {
            "tương phản nét chênh nửa vời ({a} và {b}) — không đủ giống, cũng không đủ khác"
        }
        (P::Contrast, R::Clash, En) => {
            "stroke contrast clashes ({a} vs {b}) — neither alike nor different enough"
        }
        (P::Contrast, R::ContrastWithSerif, Vi) => {
            "tương phản nét khác ({a} và {b}) đi cùng khác biệt chân serif — tương phản có chủ ý"
        }
        (P::Contrast, R::ContrastWithSerif, En) => {
            "Different stroke contrast ({a} vs {b}) together with a serif change — an intended contrast"
        }

        (P::Serif, R::Harmony, Vi) => "chân serif cùng kiểu ({a} và {b})",
        (P::Serif, R::Harmony, En) => "Same serif treatment ({a} vs {b})",
        (P::Serif, R::Contrast, Vi) => "chân serif khác loại ({a} và {b}) — một có chân, một không",
        (P::Serif, R::Contrast, En) => {
            "Different serif treatment ({a} vs {b}) — one with serifs, one without"
        }
        (P::Serif, R::Clash, Vi) => "chân serif ở mức lưng chừng ({a} và {b}) — khó đọc ra ý đồ",
        (P::Serif, R::Clash, En) => {
            "serif treatment clashes ({a} vs {b}) — the intent is hard to read"
        }

        (P::Width, R::Harmony, Vi) => "độ rộng gần nhau ({a} và {b})",
        (P::Width, R::Harmony, En) => "Similar width ({a} vs {b})",
        (P::Width, R::Contrast, Vi) => "độ rộng khác rõ ({a} và {b}) — một hẹp, một rộng",
        (P::Width, R::Contrast, En) => {
            "Clearly different width ({a} vs {b}) — one narrow, one wide"
        }
        (P::Width, R::Clash, Vi) => {
            "độ rộng chênh vừa phải ({a} và {b}) — dễ trông như lỗi dàn trang"
        }
        (P::Width, R::Clash, En) => "width clashes ({a} vs {b}) — can look like a layout mistake",

        (P::Aperture, R::Harmony, Vi) => "độ mở gần nhau ({a} và {b})",
        (P::Aperture, R::Harmony, En) => "Similar aperture ({a} vs {b})",
        (P::Aperture, R::Contrast, Vi) => {
            "độ mở khác rõ ({a} và {b}) — một chữ thoáng, một chữ khép"
        }
        (P::Aperture, R::Contrast, En) => {
            "Clearly different aperture ({a} vs {b}) — one open, one closed"
        }
        (P::Aperture, R::Clash, Vi) => {
            "độ mở chênh nửa vời ({a} và {b}) — nhịp chữ thường không khớp"
        }
        (P::Aperture, R::Clash, En) => {
            "aperture clashes ({a} vs {b}) — the lowercase rhythm does not match"
        }

        (P::Stress, R::Harmony, Vi) => "trục nhấn gần nhau ({a} và {b})",
        (P::Stress, R::Harmony, En) => "Similar stress ({a} vs {b})",
        (P::Stress, R::Contrast, Vi) => {
            "trục nhấn khác rõ ({a} và {b}) — một đứng, một nghiêng kiểu bút"
        }
        (P::Stress, R::Contrast, En) => {
            "Clearly different stress ({a} vs {b}) — one upright, one pen-angled"
        }
        (P::Stress, R::Clash, Vi) => "trục nhấn chênh nửa vời ({a} và {b})",
        (P::Stress, R::Clash, En) => "stress clashes ({a} vs {b})",

        (P::Slant, R::Harmony, Vi) => "độ nghiêng gần nhau ({a} và {b})",
        (P::Slant, R::Harmony, En) => "Similar slant ({a} vs {b})",
        (P::Slant, R::Contrast, Vi) => "độ nghiêng khác rõ ({a} và {b}) — một đứng, một nghiêng",
        (P::Slant, R::Contrast, En) => {
            "Clearly different slant ({a} vs {b}) — one upright, one slanted"
        }
        (P::Slant, R::Clash, Vi) => {
            "độ nghiêng chênh nửa vời ({a} và {b}) — trông như nghiêng nhầm"
        }
        (P::Slant, R::Clash, En) => "slant clashes ({a} vs {b}) — looks accidentally tilted",
        _ => return None,
    })
}

const UNSURE_VI: &str = "Chưa đủ số đo tin cậy ở: {list}";
const UNSURE_EN: &str = "Not enough reliable measurements for: {list}";
const GLOBAL_VI: &str = "Hai font khác họ nhưng gần nhau hơn cả hai độ đậm trong cùng một họ — chọn một, hoặc chọn font khác hẳn";
const GLOBAL_EN: &str = "Different families, yet closer than two weights of one family — pick one, or pick something clearly different";
const SKELETON_VI: &str =
    "Bề mặt nét giống nhau nhưng khung chữ khác ({axis}) — mắt thấy lệch mà khó gọi tên";
const SKELETON_EN: &str =
    "Same surface, different skeleton ({axis}) — the mismatch is felt but hard to name";
const H1_VI: &str = "{font} thiếu {n} ký tự tiếng Việt; chữ thiếu sẽ bị lấy từ font khác";
const H1_EN: &str =
    "{font} is missing {n} Vietnamese characters; they will fall back to another font";
const HIGH_CONTRAST_VI: &str = "Font nội dung có tương phản nét cao ({v}) — mỏi mắt ở cỡ nhỏ";
const HIGH_CONTRAST_EN: &str =
    "The body font has high stroke contrast ({v}) — tiring at small sizes";
const WEIGHT_RANGE_VI: &str = "Độ đậm font nội dung ({v}) nằm ngoài khoảng dễ đọc 0,10–0,22";
const WEIGHT_RANGE_EN: &str = "Body font weight ({v}) is outside the readable range 0.10–0.22";
const MISSING_RB_VI: &str = "Họ font nội dung thiếu Regular hoặc Bold";
const MISSING_RB_EN: &str = "The body family lacks a Regular or a Bold";
const LOW_XH_VI: &str = "x-height của font nội dung thấp ({v}) — nên tăng cỡ chữ";
const LOW_XH_EN: &str = "The body font has a low x-height ({v}) — set it a little larger";
const OVERFLOW_VI: &str = "Dấu chồng của font tiêu đề vượt ascender — cần nới khoảng dòng";
const OVERFLOW_EN: &str =
    "Stacked Vietnamese marks in the heading font exceed the ascender — loosen the leading";
const REVERSED_VI: &str = "Tiêu đề đang mảnh hơn nội dung — thứ bậc bị đảo";
const REVERSED_EN: &str = "The heading is lighter than the body — the hierarchy is inverted";
const HINT_VI: &str = "Gợi ý: độ đậm thân chữ của tiêu đề khoảng {v}";
const HINT_EN: &str = "Suggestion: a heading stem weight around {v}";
const INSUFFICIENT_VI: &str = "Chưa đủ dữ liệu để xếp cặp này cao hơn mức Thử nghiệm";
const INSUFFICIENT_EN: &str = "Not enough data to rate this pair above Experimental";

/// Mọi mẫu câu (để test từ cấm).
pub fn all_templates() -> Vec<&'static str> {
    let mut v = vec![
        UNSURE_VI,
        UNSURE_EN,
        GLOBAL_VI,
        GLOBAL_EN,
        SKELETON_VI,
        SKELETON_EN,
        H1_VI,
        H1_EN,
        HIGH_CONTRAST_VI,
        HIGH_CONTRAST_EN,
        WEIGHT_RANGE_VI,
        WEIGHT_RANGE_EN,
        MISSING_RB_VI,
        MISSING_RB_EN,
        LOW_XH_VI,
        LOW_XH_EN,
        OVERFLOW_VI,
        OVERFLOW_EN,
        REVERSED_VI,
        REVERSED_EN,
        HINT_VI,
        HINT_EN,
        INSUFFICIENT_VI,
        INSUFFICIENT_EN,
    ];
    let rels = [
        Relation::Harmony,
        Relation::Contrast,
        Relation::Clash,
        Relation::WeightHint,
        Relation::ContrastWithSerif,
    ];
    for ax in PairAxis::ALL {
        for r in rels {
            for l in [Lang::Vi, Lang::En] {
                v.extend(axis_template(ax, r, l));
            }
        }
    }
    for a in AxisId::ALL {
        v.push(axis_name(a, Lang::Vi));
        v.push(axis_name(a, Lang::En));
    }
    v
}

fn pick(lang: Lang, vi: &'static str, en: &'static str) -> &'static str {
    match lang {
        Lang::Vi => vi,
        Lang::En => en,
    }
}

fn num(v: Option<f32>, lang: Lang) -> String {
    v.filter(|x| x.is_finite())
        .map_or_else(|| "?".into(), |x| fmt_num(x, lang))
}

fn axis_sentence(l: &AxisLabel, lang: Lang) -> Option<String> {
    let t = axis_template(l.axis, l.relation, lang)?;
    Some(
        t.replace("{a}", &num(l.a, lang))
            .replace("{b}", &num(l.b, lang)),
    )
}

fn body_value(w: BodyWarning, a: &FaceProfile, b: &FaceProfile) -> Option<f32> {
    let axis = match w {
        BodyWarning::HighContrast => AxisId::Contrast,
        BodyWarning::WeightOutOfRange => AxisId::StemNXh,
        BodyWarning::LowXHeight => AxisId::XHeightEm,
        BodyWarning::MissingRegularOrBold => return None,
    };
    // Không biết bên nào là Body ở đây → lấy giá trị vượt ngưỡng nếu có.
    [a, b]
        .iter()
        .filter_map(|p| p.get(axis).value)
        .find(|v| match w {
            BodyWarning::HighContrast => *v > 3.5,
            BodyWarning::WeightOutOfRange => !(0.10..=0.22).contains(v),
            BodyWarning::LowXHeight => *v < 0.43,
            BodyWarning::MissingRegularOrBold => false,
        })
}

pub fn explain_pair(
    x: &PairAssessment,
    a: &FaceProfile,
    b: &FaceProfile,
    lang: Lang,
) -> Vec<String> {
    let mut out = Vec::new();
    // 1. Chặn (H1).
    if x.gate == Gate::Blocked {
        if let Some(Block::MissingVietnamese { side, missing }) = x.block {
            let font = match side {
                Side::A => &a.family,
                Side::B => &b.family,
            };
            out.push(
                pick(lang, H1_VI, H1_EN)
                    .replace("{font}", font)
                    .replace("{n}", &missing.to_string()),
            );
        }
    }
    // 2. Va chạm: toàn cục, khung, rồi theo trục.
    if x.clashes.contains(&ClashKind::Global) {
        out.push(pick(lang, GLOBAL_VI, GLOBAL_EN).to_string());
    }
    if x.clashes.contains(&ClashKind::Skeleton) {
        let axes: Vec<&str> = x
            .labels
            .iter()
            .filter(|l| {
                l.axis.group() == crate::pair_axes::AxisGroup::Structure
                    && l.relation == Relation::Contrast
            })
            .map(|l| pair_axis_name(l.axis, lang))
            .collect();
        out.push(pick(lang, SKELETON_VI, SKELETON_EN).replace("{axis}", &axes.join(", ")));
    }
    for rels in [
        &[Relation::Clash][..],
        &[Relation::Contrast, Relation::ContrastWithSerif][..],
        &[Relation::Harmony][..],
    ] {
        for l in &x.labels {
            if rels.contains(&l.relation) {
                out.extend(axis_sentence(l, lang));
            }
        }
    }
    // 5. Cảnh báo (gồm gợi ý độ đậm vùng giữa).
    for l in &x.labels {
        if l.relation == Relation::WeightHint {
            out.extend(axis_sentence(l, lang));
        }
    }
    for w in &x.body_warnings {
        let s = match w {
            BodyWarning::HighContrast => pick(lang, HIGH_CONTRAST_VI, HIGH_CONTRAST_EN),
            BodyWarning::WeightOutOfRange => pick(lang, WEIGHT_RANGE_VI, WEIGHT_RANGE_EN),
            BodyWarning::MissingRegularOrBold => pick(lang, MISSING_RB_VI, MISSING_RB_EN),
            BodyWarning::LowXHeight => pick(lang, LOW_XH_VI, LOW_XH_EN),
        };
        out.push(s.replace("{v}", &num(body_value(*w, a, b), lang)));
    }
    if x.heading_overflow {
        out.push(pick(lang, OVERFLOW_VI, OVERFLOW_EN).to_string());
    }
    if x.labels
        .iter()
        .any(|l| l.axis == PairAxis::Weight && l.role_reversed)
    {
        out.push(pick(lang, REVERSED_VI, REVERSED_EN).to_string());
    }
    if let Some(h) = x.weight_hint {
        out.push(pick(lang, HINT_VI, HINT_EN).replace("{v}", &fmt_num(h, lang)));
    }
    if x.insufficient_data {
        out.push(pick(lang, INSUFFICIENT_VI, INSUFFICIENT_EN).to_string());
    }
    // Một câu chung cho mọi trục chưa chắc.
    let unsure: Vec<&str> = x
        .labels
        .iter()
        .filter(|l| l.relation == Relation::Unsure)
        .map(|l| pair_axis_name(l.axis, lang))
        .collect();
    if !unsure.is_empty() {
        out.push(pick(lang, UNSURE_VI, UNSURE_EN).replace("{list}", &unsure.join(", ")));
    }
    out
}

/// (Giống ở, Khác ở) cho một kết quả Similar, kèm số đo thật của hai bên.
pub fn explain_similar(
    hit: &SimilarHit,
    q: &FaceProfile,
    c: &FaceProfile,
    lang: Lang,
) -> (Vec<String>, Vec<String>) {
    let line = |a: &AxisId| {
        let sep = pick(lang, " và ", " vs ");
        format!(
            "{}: {}{}{}",
            axis_name(*a, lang),
            num(q.get(*a).value, lang),
            sep,
            num(c.get(*a).value, lang)
        )
    };
    (
        hit.same.iter().map(line).collect(),
        hit.differ.iter().map(line).collect(),
    )
}
