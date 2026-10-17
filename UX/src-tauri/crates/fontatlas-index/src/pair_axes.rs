//! Nhãn theo trục cho một cặp: hoà hợp / tương phản / va chạm / chưa chắc (rules_plan3.md §1).
//! Nhãn cứng theo ngưỡng τ_h, τ_c — chủ ý lệch spec 6.3 (Lead, 2026-10-17).
use serde::{Deserialize, Serialize};

use crate::profile::{AxisId, FaceProfile, Reading, Role};
use crate::reference::ReferenceTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairAxis {
    XHeight,
    Weight,
    Contrast,
    Serif,
    Width,
    Aperture,
    Stress,
    Slant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisGroup {
    Structure,
    Expressive,
}

impl PairAxis {
    pub const ALL: [PairAxis; 8] = [
        PairAxis::XHeight,
        PairAxis::Weight,
        PairAxis::Contrast,
        PairAxis::Serif,
        PairAxis::Width,
        PairAxis::Aperture,
        PairAxis::Stress,
        PairAxis::Slant,
    ];

    pub fn group(self) -> AxisGroup {
        match self {
            PairAxis::XHeight | PairAxis::Width | PairAxis::Aperture | PairAxis::Stress => {
                AxisGroup::Structure
            }
            PairAxis::Weight | PairAxis::Contrast | PairAxis::Serif | PairAxis::Slant => {
                AxisGroup::Expressive
            }
        }
    }

    /// Trục đo tương ứng của core.
    pub fn source(self) -> AxisId {
        match self {
            PairAxis::XHeight => AxisId::XHeightEm,
            PairAxis::Weight => AxisId::StemNXh,
            PairAxis::Contrast => AxisId::Contrast,
            PairAxis::Serif => AxisId::SerifFoot,
            PairAxis::Width => AxisId::Width,
            PairAxis::Aperture => AxisId::Aperture,
            PairAxis::Stress => AxisId::StressDeg,
            PairAxis::Slant => AxisId::SlantDeg,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Harmony,
    Contrast,
    Clash,
    Unsure,
    /// Contrast vùng va chạm nhưng đi kèm khác lớp serif (R3) → tương phản có chủ ý.
    ContrastWithSerif,
    /// Độ đậm vùng giữa (R5): gợi ý, không phải va chạm.
    WeightHint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisLabel {
    pub axis: PairAxis,
    pub relation: Relation,
    pub delta: Option<f32>,
    /// Số đo thật của hai bên (không phải phân vị).
    pub a: Option<f32>,
    pub b: Option<f32>,
    /// Tiêu đề mảnh hơn nội dung (R5).
    pub role_reversed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    pub x_height: (f32, f32),
    pub weight: (f32, f32),
    pub contrast: (f32, f32),
    pub width: (f32, f32),
    pub aperture: (f32, f32),
    pub stress: (f32, f32),
    pub slant: (f32, f32),
    pub serif_sans_max: f32,
    pub serif_min: f32,
    pub stress_min_contrast: f32,
}

impl Thresholds {
    /// Bảng ngưỡng của rules_plan3.md §1.
    pub fn v1() -> Self {
        Thresholds {
            x_height: (0.08, 0.15),
            weight: (0.10, 0.40),
            contrast: (0.20, 0.70),
            width: (0.10, 0.25),
            aperture: (0.15, 0.35),
            stress: (8.0, 25.0),
            slant: (2.0, 8.0),
            serif_sans_max: 1.2,
            serif_min: 1.6,
            stress_min_contrast: 1.3,
        }
    }
}

/// Cùng cỡ chữ: sai chỉ khi một bên là tiêu đề còn bên kia không.
pub fn same_size(ra: Role, rb: Role) -> bool {
    (ra == Role::Heading) == (rb == Role::Heading)
}

/// Hai giá trị dùng được: có số, đủ tin cậy, cùng method.
fn usable<'r>(a: &'r Reading, b: &'r Reading) -> Option<(f32, f32)> {
    let (va, vb) = (a.value?, b.value?);
    let ok = a.q.conf() * b.q.conf() >= 0.5 && a.method == b.method;
    ok.then_some((va, vb))
}

fn ln_delta(a: f32, b: f32) -> Option<f32> {
    if !(a > 0.0 && b > 0.0) {
        return None;
    }
    let d = (a / b).ln().abs();
    d.is_finite().then_some(d)
}

fn abs_delta(a: f32, b: f32) -> Option<f32> {
    let d = (a - b).abs();
    d.is_finite().then_some(d)
}

fn band(d: f32, (h, c): (f32, f32)) -> Relation {
    if d <= h {
        Relation::Harmony
    } else if d >= c {
        Relation::Contrast
    } else {
        Relation::Clash
    }
}

#[derive(PartialEq)]
enum SerifClass {
    Sans,
    Serif,
}

pub fn label_axes(
    a: &FaceProfile,
    ra: Role,
    b: &FaceProfile,
    rb: Role,
    t: &Thresholds,
    r: &ReferenceTable,
) -> Vec<AxisLabel> {
    let serif = label(PairAxis::Serif, a, ra, b, rb, t, r, None);
    PairAxis::ALL
        .iter()
        .map(|&ax| {
            if ax == PairAxis::Serif {
                serif.clone()
            } else {
                label(ax, a, ra, b, rb, t, r, Some(serif.relation))
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn label(
    ax: PairAxis,
    a: &FaceProfile,
    ra: Role,
    b: &FaceProfile,
    rb: Role,
    t: &Thresholds,
    r: &ReferenceTable,
    serif_rel: Option<Relation>,
) -> AxisLabel {
    let (ga, gb) = (a.get(ax.source()), b.get(ax.source()));
    let mut out = AxisLabel {
        axis: ax,
        relation: Relation::Unsure,
        delta: None,
        a: ga.value.filter(|v| v.is_finite()),
        b: gb.value.filter(|v| v.is_finite()),
        role_reversed: false,
    };
    let Some((va, vb)) = usable(ga, gb) else {
        return out;
    };
    let rel = match ax {
        PairAxis::XHeight => ln_delta(va, vb).map(|d| {
            out.delta = Some(d);
            // R6: chỉ so x-height khi hai vai trò cùng cỡ.
            if same_size(ra, rb) {
                band(d, t.x_height)
            } else {
                Relation::Harmony
            }
        }),
        PairAxis::Weight => ln_delta(va, vb).map(|d| {
            out.delta = Some(d);
            out.role_reversed = ra == Role::Heading && rb != Role::Heading && va < vb && d > 0.10;
            // R5: độ đậm không bao giờ là va chạm.
            match band(d, t.weight) {
                Relation::Clash => Relation::WeightHint,
                x => x,
            }
        }),
        PairAxis::Contrast => ln_delta(va, vb).map(|d| {
            out.delta = Some(d);
            match band(d, t.contrast) {
                // R3: khác lớp serif biến va chạm contrast thành tương phản có chủ ý.
                Relation::Clash if serif_rel == Some(Relation::Contrast) => {
                    Relation::ContrastWithSerif
                }
                x => x,
            }
        }),
        PairAxis::Width => ln_delta(va, vb).map(|d| {
            out.delta = Some(d);
            band(d, t.width)
        }),
        PairAxis::Serif => {
            let class = |v: f32| {
                if !v.is_finite() {
                    None
                } else if v < t.serif_sans_max {
                    Some(SerifClass::Sans)
                } else if v > t.serif_min {
                    Some(SerifClass::Serif)
                } else {
                    None // vùng [1.2, 1.6]: chưa chắc
                }
            };
            match (class(va), class(vb)) {
                (Some(x), Some(y)) => {
                    out.delta = abs_delta(va, vb);
                    Some(if x == y {
                        Relation::Harmony
                    } else {
                        Relation::Contrast
                    })
                }
                _ => None,
            }
        }
        PairAxis::Aperture => {
            let pa = r.percentile(AxisId::Aperture, &ga.method, va);
            let pb = r.percentile(AxisId::Aperture, &gb.method, vb);
            match (pa, pb) {
                (Some(pa), Some(pb)) => abs_delta(pa, pb).map(|d| {
                    out.delta = Some(d);
                    band(d, t.aperture)
                }),
                _ => None,
            }
        }
        PairAxis::Stress => {
            // Trục nhấn chỉ có nghĩa khi cả hai có tương phản.
            let ca = a.get(AxisId::Contrast).value;
            let cb = b.get(AxisId::Contrast).value;
            let both = matches!((ca, cb), (Some(x), Some(y)) if x >= t.stress_min_contrast && y >= t.stress_min_contrast);
            if both {
                abs_delta(va, vb).map(|d| {
                    out.delta = Some(d);
                    band(d, t.stress)
                })
            } else {
                None
            }
        }
        PairAxis::Slant => abs_delta(va, vb).map(|d| {
            out.delta = Some(d);
            band(d, t.slant)
        }),
    };
    match rel {
        Some(x) => out.relation = x,
        // δ không hợp lệ → chưa chắc, không bao giờ rơi vào va chạm.
        None => out.delta = None,
    }
    out
}
