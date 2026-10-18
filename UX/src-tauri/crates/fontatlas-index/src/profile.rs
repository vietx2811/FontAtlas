//! Hồ sơ một face cho Similar/Pairing: 16 trục (giá trị + chất lượng + method) và vài trường metadata.
use fontatlas_core::FaceReport;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 16 trục, thứ tự cũng là thứ tự vector Similar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisId {
    XHeightEm,
    XHeightCap,
    AscenderEm,
    StemNXh,
    Darkness,
    Contrast,
    StressDeg,
    Aperture,
    ApertureE,
    Width,
    MeanAdvance,
    WidthCv,
    Spacing,
    SlantDeg,
    SerifFoot,
    Counter,
}

impl AxisId {
    pub const ALL: [AxisId; 16] = [
        AxisId::XHeightEm,
        AxisId::XHeightCap,
        AxisId::AscenderEm,
        AxisId::StemNXh,
        AxisId::Darkness,
        AxisId::Contrast,
        AxisId::StressDeg,
        AxisId::Aperture,
        AxisId::ApertureE,
        AxisId::Width,
        AxisId::MeanAdvance,
        AxisId::WidthCv,
        AxisId::Spacing,
        AxisId::SlantDeg,
        AxisId::SerifFoot,
        AxisId::Counter,
    ];

    /// Tên trường tương ứng trong `Anatomy` của core.
    pub fn key(self) -> &'static str {
        match self {
            AxisId::XHeightEm => "x_height_em",
            AxisId::XHeightCap => "x_height_cap",
            AxisId::AscenderEm => "ascender_em",
            AxisId::StemNXh => "stem",
            AxisId::Darkness => "darkness",
            AxisId::Contrast => "contrast",
            AxisId::StressDeg => "stress_deg",
            AxisId::Aperture => "aperture",
            AxisId::ApertureE => "aperture_e",
            AxisId::Width => "width",
            AxisId::MeanAdvance => "mean_advance",
            AxisId::WidthCv => "width_cv",
            AxisId::Spacing => "spacing",
            AxisId::SlantDeg => "slant_deg",
            AxisId::SerifFoot => "serif_foot",
            AxisId::Counter => "counter",
        }
    }

    /// Method của nhánh chữ thường trong core.
    pub fn default_method(self) -> &'static str {
        match self {
            AxisId::XHeightEm => "x_bbox_top",
            AxisId::XHeightCap => "x_over_H",
            AxisId::AscenderEm => "d_bbox_top",
            AxisId::StemNXh => "n_stem_at_35pct_xh",
            AxisId::Darkness => "ink_area_n_o",
            AxisId::Contrast => "o_radial_thickness_max_min",
            AxisId::StressDeg => "o_harmonic_fit",
            AxisId::Aperture => "c_angular_opening",
            AxisId::ApertureE => "e_min_vertical_gap_bar_to_terminal",
            AxisId::Width => "o_bbox_width_over_xh",
            AxisId::MeanAdvance => "mean_advance_a_z_over_xh",
            AxisId::WidthCv => "cv_advance_a_z",
            AxisId::Spacing => "mean_sidebearings_a_z_over_xh",
            AxisId::SlantDeg => "l_or_I_midline_offset",
            AxisId::SerifFoot => "baseline_foot_over_stem_median_iln",
            AxisId::Counter => "o_inner_over_outer_area",
        }
    }

    pub fn index(self) -> usize {
        AxisId::ALL.iter().position(|a| *a == self).unwrap_or(0)
    }
}

/// Chất lượng đo rút gọn (lý do Failed không cần cho ghép cặp).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Q {
    Measured,
    Estimated,
    Failed,
}

impl Q {
    /// Độ tin cậy dùng cho "chưa chắc": conf(a)·conf(b) < 0,5.
    pub fn conf(self) -> f32 {
        match self {
            Q::Measured => 1.0,
            Q::Estimated => 0.6,
            Q::Failed => 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub value: Option<f32>,
    pub q: Q,
    pub method: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FaceKey {
    pub hash: String,
    pub index: u32,
}

/// Vai trò chữ trong một cặp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Heading,
    Body,
    Code,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceProfile {
    pub key: FaceKey,
    pub family: String,
    pub style: String,
    pub designer: Option<String>,
    /// nameID 8 — dùng làm "vendor" cho R2.
    pub manufacturer: Option<String>,
    pub weight_class: Option<u16>,
    /// (min, max) của trục wght nếu là variable font.
    pub wght_range: Option<(f32, f32)>,
    pub named_instances: Vec<String>,
    pub open_license: bool,
    pub vi_missing: usize,
    pub vi_overflow: Option<bool>,
    /// 16 giá trị theo `AxisId::ALL`.
    pub readings: Vec<Reading>,
    /// Đầu nét (Estimated): không thuộc 16 chiều Similar, không là PairAxis ở Plan 3.
    pub terminal: Reading,
    /// Do Plan 4 cung cấp.
    pub embedding: Option<Vec<f32>>,
}

impl FaceProfile {
    /// Dựng từ báo cáo core. Đi qua cùng đường parse với `from_json` để hai đường luôn khớp.
    pub fn from_report(hash: &str, r: &FaceReport) -> Self {
        let built = (|| {
            let m = serde_json::to_value(&r.metadata).ok()?;
            let v = serde_json::to_value(&r.vietnamese).ok()?;
            let a = serde_json::to_value(&r.anatomy).ok()?;
            Self::from_json(hash, r.index, &m, &v, &a)
        })();
        built.unwrap_or_else(|| Self::unreadable(hash, r.index))
    }

    /// Hồ sơ rỗng: mọi trục Failed (không bao giờ xảy ra với dữ liệu từ core).
    fn unreadable(hash: &str, index: u32) -> Self {
        FaceProfile {
            key: FaceKey {
                hash: hash.to_string(),
                index,
            },
            family: "?".into(),
            style: "?".into(),
            designer: None,
            manufacturer: None,
            weight_class: None,
            wght_range: None,
            named_instances: Vec::new(),
            open_license: false,
            vi_missing: 0,
            vi_overflow: None,
            readings: AxisId::ALL
                .iter()
                .map(|a| Reading {
                    value: None,
                    q: Q::Failed,
                    method: a.default_method().into(),
                })
                .collect(),
            terminal: no_terminal(),
            embedding: None,
        }
    }

    pub fn get(&self, a: AxisId) -> &Reading {
        &self.readings[a.index()]
    }

    /// Dựng từ JSON nguyên văn của Metadata / VietnameseCoverage / Anatomy (Plan 2 lưu).
    /// Thiếu trường bắt buộc → None.
    pub fn from_json(
        hash: &str,
        index: u32,
        metadata: &Value,
        vietnamese: &Value,
        anatomy: &Value,
    ) -> Option<Self> {
        let s = |k: &str| metadata.get(k).and_then(Value::as_str).map(str::to_string);
        let family = s("typographic_family")
            .or_else(|| s("family"))
            .or_else(|| s("postscript_name"))
            .unwrap_or_else(|| "?".into());
        let style = s("typographic_subfamily")
            .or_else(|| s("subfamily"))
            .or_else(|| s("postscript_name"))
            .unwrap_or_else(|| "?".into());
        let license = metadata.get("license_class")?.as_str()?;
        let wght_range = metadata.get("axes")?.as_array()?.iter().find_map(|a| {
            (a.get("tag")?.as_str()? == "wght").then(|| {
                Some((
                    a.get("min")?.as_f64()? as f32,
                    a.get("max")?.as_f64()? as f32,
                ))
            })?
        });
        let named_instances = metadata
            .get("named_instances")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let weight_class = metadata
            .get("weight_class")
            .and_then(Value::as_u64)
            .and_then(|v| u16::try_from(v).ok());
        let vi_missing = vietnamese.get("missing")?.as_array()?.len();
        let vi_overflow = vietnamese
            .get("stacked_marks_overflow")
            .and_then(Value::as_bool);
        let readings = AxisId::ALL
            .iter()
            .map(|a| reading(anatomy.get(a.key())?))
            .collect::<Option<Vec<_>>>()?;
        Some(FaceProfile {
            key: FaceKey {
                hash: hash.to_string(),
                index,
            },
            family,
            style,
            designer: s("designer"),
            manufacturer: s("manufacturer"),
            weight_class,
            wght_range,
            named_instances,
            open_license: matches!(license, "ofl" | "apache"),
            vi_missing,
            vi_overflow,
            readings,
            // JSON cũ (core-0.1.0) chưa có trường này → Failed, không loại cả profile.
            terminal: anatomy
                .get("terminal")
                .and_then(reading)
                .unwrap_or_else(no_terminal),
            embedding: None,
        })
    }
}

pub const TERMINAL_METHOD: &str = "c_upper_terminal_normal_over_thin";

fn no_terminal() -> Reading {
    Reading {
        value: None,
        q: Q::Failed,
        method: TERMINAL_METHOD.into(),
    }
}

/// Một `Metric` của core ở dạng JSON: `{value, quality: {kind, reason?}, method}`.
fn reading(m: &Value) -> Option<Reading> {
    let q = match m.get("quality")?.get("kind")?.as_str()? {
        "measured" => Q::Measured,
        "estimated" => Q::Estimated,
        "failed" => Q::Failed,
        _ => return None,
    };
    let value = match m.get("value")? {
        Value::Null => None,
        v => Some(v.as_f64()? as f32),
    };
    Some(Reading {
        value,
        q,
        method: m.get("method")?.as_str()?.to_string(),
    })
}
