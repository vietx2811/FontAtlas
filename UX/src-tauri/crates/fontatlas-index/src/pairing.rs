//! Đánh giá một cặp: nhãn trục, va chạm (trục / toàn cục R7 / khung R8), vai trò Nội dung (R9),
//! tiếng Việt (H1), cổng An toàn / Có kiểm soát / Thử nghiệm. Không có điểm tổng.
use std::cmp::Reverse;

use serde::{Deserialize, Serialize};

use crate::pair_axes::{
    AxisGroup, AxisLabel, PairAxis, Relation, Thresholds, label_axes, same_size,
};
use crate::profile::{AxisId, FaceProfile, Role};
use crate::reference::ReferenceTable;
use crate::superfamily::{SuperfamilyTable, same_designer};

pub struct PairContext<'a> {
    pub thresholds: &'a Thresholds,
    pub reference: &'a ReferenceTable,
    pub superfamilies: &'a SuperfamilyTable,
    /// Dự án tiếng Việt → H1.
    pub project_vi: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FamilyFacts {
    pub has_regular: bool,
    pub has_bold: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    Safe,
    Controlled,
    Experimental,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Block {
    MissingVietnamese { side: Side, missing: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyWarning {
    HighContrast,
    WeightOutOfRange,
    MissingRegularOrBold,
    /// Chỉ cảnh báo, không chặn cổng.
    LowXHeight,
}

impl BodyWarning {
    fn blocks(self) -> bool {
        !matches!(self, BodyWarning::LowXHeight)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClashKind {
    Axis(PairAxis),
    /// R7: khác họ nhưng gần như giống hệt.
    Global,
    /// R8: cùng "da thịt", khác "khung xương".
    Skeleton,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairAssessment {
    pub labels: Vec<AxisLabel>,
    pub clashes: Vec<ClashKind>,
    pub related: bool,
    pub same_designer: bool,
    /// Nội bộ (R1/R7), không phải điểm, không hiển thị.
    pub global_distance: Option<f32>,
    pub gate: Gate,
    pub block: Option<Block>,
    pub body_warnings: Vec<BodyWarning>,
    pub heading_overflow: bool,
    pub insufficient_data: bool,
    /// Stem mục tiêu cho tiêu đề khi độ đậm ở vùng gợi ý (R5).
    pub weight_hint: Option<f32>,
}

/// Ngưỡng tương phản: body không nên vượt.
const BODY_MAX_CONTRAST: f32 = 3.5;
const BODY_STEM: (f32, f32) = (0.10, 0.22);
const BODY_MIN_XHEIGHT: f32 = 0.43;

fn tau_c(ax: PairAxis, t: &Thresholds) -> Option<f32> {
    match ax {
        PairAxis::XHeight => Some(t.x_height.1),
        PairAxis::Weight => Some(t.weight.1),
        PairAxis::Contrast => Some(t.contrast.1),
        PairAxis::Width => Some(t.width.1),
        PairAxis::Aperture => Some(t.aperture.1),
        PairAxis::Slant => Some(t.slant.1),
        PairAxis::Serif | PairAxis::Stress => None,
    }
}

fn global_from_labels(labels: &[AxisLabel], ra: Role, rb: Role, t: &Thresholds) -> Option<f32> {
    let mut z2: Vec<f32> = Vec::new();
    for l in labels {
        if l.relation == Relation::Unsure {
            continue;
        }
        match l.axis {
            PairAxis::Serif => z2.push(if l.relation == Relation::Contrast {
                4.0
            } else {
                0.0
            }),
            PairAxis::Stress => {}
            PairAxis::XHeight if !same_size(ra, rb) => {}
            ax => {
                if let (Some(d), Some(c)) = (l.delta, tau_c(ax, t)) {
                    let z = d / c;
                    if z.is_finite() {
                        z2.push(z * z);
                    }
                }
            }
        }
    }
    if z2.len() < 3 {
        return None;
    }
    let d = (z2.iter().sum::<f32>() / z2.len() as f32).sqrt();
    d.is_finite().then_some(d)
}

/// R1: khoảng cách toàn cục √(mean z²), z = δ/τ_c. Không phải điểm, không hiển thị.
pub fn global_distance(
    a: &FaceProfile,
    ra: Role,
    b: &FaceProfile,
    rb: Role,
    t: &Thresholds,
    r: &ReferenceTable,
) -> Option<f32> {
    global_from_labels(&label_axes(a, ra, b, rb, t, r), ra, rb, t)
}

fn body_warnings(p: &FaceProfile, facts: Option<&FamilyFacts>) -> Vec<BodyWarning> {
    let v = |a: AxisId| p.get(a).value.filter(|x| x.is_finite());
    let mut w = Vec::new();
    if v(AxisId::Contrast).is_some_and(|c| c > BODY_MAX_CONTRAST) {
        w.push(BodyWarning::HighContrast);
    }
    if v(AxisId::StemNXh).is_some_and(|s| !(BODY_STEM.0..=BODY_STEM.1).contains(&s)) {
        w.push(BodyWarning::WeightOutOfRange);
    }
    if facts.is_some_and(|f| !(f.has_regular && f.has_bold)) {
        w.push(BodyWarning::MissingRegularOrBold);
    }
    if v(AxisId::XHeightEm).is_some_and(|x| x < BODY_MIN_XHEIGHT) {
        w.push(BodyWarning::LowXHeight);
    }
    w
}

pub fn assess(
    a: &FaceProfile,
    ra: Role,
    b: &FaceProfile,
    rb: Role,
    body_facts: Option<&FamilyFacts>,
    ctx: &PairContext,
) -> PairAssessment {
    let t = ctx.thresholds;
    let labels = label_axes(a, ra, b, rb, t, ctx.reference);
    let rel = |ax: PairAxis| labels.iter().find(|l| l.axis == ax).map(|l| l.relation);
    let related = ctx.superfamilies.related(a, b);
    let same_des = same_designer(a, b);
    let global_distance = global_from_labels(&labels, ra, rb, t);

    // Va chạm theo trục.
    let mut clashes: Vec<ClashKind> = labels
        .iter()
        .filter(|l| l.relation == Relation::Clash)
        .map(|l| ClashKind::Axis(l.axis))
        .collect();
    let expressive: Vec<Relation> = labels
        .iter()
        .filter(|l| l.axis.group() == AxisGroup::Expressive)
        .map(|l| l.relation)
        .collect();
    let any_expr_determined = expressive.iter().any(|r| *r != Relation::Unsure);
    let any_expr_contrast = expressive.contains(&Relation::Contrast);
    // R7: gần như giống hệt mà khác họ, và không có tương phản biểu cảm nào.
    if !related
        && any_expr_determined
        && !any_expr_contrast
        && global_distance.is_some_and(|d| d < ctx.reference.d_fam * 1.1)
    {
        clashes.push(ClashKind::Global);
    }
    // R8: biểu cảm hoà hợp nhưng khung (cấu trúc) tương phản.
    let expr_calm = expressive.iter().all(|r| {
        matches!(
            r,
            Relation::Harmony | Relation::Unsure | Relation::WeightHint
        )
    });
    let struct_contrast = labels
        .iter()
        .any(|l| l.axis.group() == AxisGroup::Structure && l.relation == Relation::Contrast);
    if !related && any_expr_determined && expr_calm && struct_contrast {
        clashes.push(ClashKind::Skeleton);
    }

    // Vai trò Nội dung.
    let mut warnings: Vec<BodyWarning> = Vec::new();
    for (p, r) in [(a, ra), (b, rb)] {
        if r == Role::Body {
            for w in body_warnings(p, body_facts) {
                if !warnings.contains(&w) {
                    warnings.push(w);
                }
            }
        }
    }
    let heading_overflow = [(a, ra), (b, rb)]
        .iter()
        .any(|(p, r)| *r == Role::Heading && p.vi_overflow == Some(true));
    let determined = labels
        .iter()
        .filter(|l| l.relation != Relation::Unsure)
        .count();
    let insufficient_data = determined < 4;
    let weight_hint = (rel(PairAxis::Weight) == Some(Relation::WeightHint))
        .then(|| {
            let body = if rb == Role::Body {
                b
            } else if ra == Role::Body {
                a
            } else {
                return None;
            };
            body.get(AxisId::StemNXh).value.map(|s| s * 0.40f32.exp())
        })
        .flatten();

    // H1 trước mọi luật khác.
    let block = if !ctx.project_vi {
        None
    } else if a.vi_missing > 0 {
        Some(Block::MissingVietnamese {
            side: Side::A,
            missing: a.vi_missing,
        })
    } else if b.vi_missing > 0 {
        Some(Block::MissingVietnamese {
            side: Side::B,
            missing: b.vi_missing,
        })
    } else {
        None
    };
    let blocking_warning = warnings.iter().any(|w| w.blocks());
    let gate = if block.is_some() {
        Gate::Blocked
    } else if insufficient_data {
        Gate::Experimental
    } else {
        let structure_calm = labels
            .iter()
            .filter(|l| l.axis.group() == AxisGroup::Structure)
            .all(|l| matches!(l.relation, Relation::Harmony | Relation::Unsure));
        let controlled_clash = clashes.len() <= 1
            && clashes.iter().all(|c| {
                matches!(
                    c,
                    ClashKind::Axis(PairAxis::Width | PairAxis::Aperture | PairAxis::Stress)
                )
            });
        if related || (clashes.is_empty() && structure_calm && !blocking_warning) {
            Gate::Safe
        } else if controlled_clash && any_expr_contrast && !blocking_warning {
            Gate::Controlled
        } else {
            Gate::Experimental
        }
    };

    PairAssessment {
        labels,
        clashes,
        related,
        same_designer: same_des,
        global_distance,
        gate,
        block,
        body_warnings: warnings,
        heading_overflow,
        insufficient_data,
        weight_hint,
    }
}

/// Khoá xếp hạng: (cổng, số va chạm, −trục cấu trúc hoà hợp, không liên quan,
/// −tương phản biểu cảm, −cos ảnh, tên).
pub type RankKey = (
    Gate,
    usize,
    Reverse<usize>,
    bool,
    Reverse<i32>,
    Reverse<i32>,
    String,
);

/// Khoá xếp hạng từ điển (tăng dần là tốt hơn). Không có điểm tổng.
pub fn rank_key(x: &PairAssessment, b_name: &str, embed_cos: Option<f32>) -> RankKey {
    let struct_harmony = x
        .labels
        .iter()
        .filter(|l| l.axis.group() == AxisGroup::Structure && l.relation == Relation::Harmony)
        .count();
    let t = Thresholds::v1();
    let max_expr = x
        .labels
        .iter()
        .filter(|l| l.axis.group() == AxisGroup::Expressive && l.relation == Relation::Contrast)
        .filter_map(|l| Some(l.delta? / tau_c(l.axis, &t)?))
        .filter(|v| v.is_finite())
        .fold(None, |m: Option<f32>, v| Some(m.map_or(v, |m| m.max(v))));
    let max_expr = max_expr.map_or(i32::MIN, |v| (v * 100.0).round() as i32);
    let cos = embed_cos
        .filter(|c| c.is_finite())
        .map_or(i32::MIN, |c| (c * 1000.0).round() as i32);
    (
        x.gate,
        x.clashes.len(),
        Reverse(struct_harmony),
        !(x.related || x.same_designer),
        Reverse(max_expr),
        Reverse(cos),
        b_name.to_string(),
    )
}
