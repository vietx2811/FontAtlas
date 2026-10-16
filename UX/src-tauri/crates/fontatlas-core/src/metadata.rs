//! Metadata: name, OS/2, head, post, fvar, định dạng outline, OT features, giấy phép.
use read_fonts::{FontRef, TableProvider, types::Tag};
use serde::Serialize;
use skrifa::{MetadataProvider, string::StringId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseClass {
    Ofl,
    Apache,
    Other,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutlineFormat {
    Glyf,
    Cff,
    Cff2,
    None,
}

#[derive(Debug, Clone, Serialize)]
pub struct AxisInfo {
    pub tag: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Metadata {
    pub family: Option<String>,
    pub subfamily: Option<String>,
    pub typographic_family: Option<String>,
    pub typographic_subfamily: Option<String>,
    pub full_name: Option<String>,
    pub postscript_name: Option<String>,
    pub version: Option<String>,
    pub designer: Option<String>,
    pub manufacturer: Option<String>,
    pub license_description: Option<String>,
    pub license_url: Option<String>,
    pub license_class: LicenseClass,
    pub units_per_em: u16,
    pub weight_class: Option<u16>,
    pub width_class: Option<u16>,
    pub fs_type: Option<u16>,
    pub panose: Option<[u8; 10]>,
    pub italic_angle: f32,
    pub outline_format: OutlineFormat,
    pub color_tables: Vec<String>,
    pub num_glyphs: u16,
    pub axes: Vec<AxisInfo>,
    pub named_instances: Vec<String>,
    pub gsub_features: Vec<String>,
    pub gpos_features: Vec<String>,
}

pub fn classify_license(description: Option<&str>, url: Option<&str>) -> LicenseClass {
    if description.is_none() && url.is_none() {
        return LicenseClass::Unknown;
    }
    let s = format!("{} {}", description.unwrap_or(""), url.unwrap_or("")).to_lowercase();
    if s.contains("open font license") || s.contains("openfontlicense") || s.contains("scripts.sil.org/ofl") {
        LicenseClass::Ofl
    } else if s.contains("apache license") || s.contains("apache.org/licenses") {
        LicenseClass::Apache
    } else {
        LicenseClass::Other
    }
}

fn name(font: &FontRef, id: StringId) -> Option<String> {
    font.localized_strings(id).english_or_first().map(|s| s.to_string())
}

fn has(font: &FontRef, tag: &[u8; 4]) -> bool {
    font.table_data(Tag::new(tag)).is_some()
}

pub(crate) fn outline_format(font: &FontRef) -> OutlineFormat {
    if has(font, b"glyf") {
        OutlineFormat::Glyf
    } else if has(font, b"CFF2") {
        OutlineFormat::Cff2
    } else if has(font, b"CFF ") {
        OutlineFormat::Cff
    } else {
        OutlineFormat::None
    }
}

pub(crate) fn feature_tags(font: &FontRef) -> (Vec<String>, Vec<String>) {
    let mut gsub = Vec::new();
    let mut gpos = Vec::new();
    if let Ok(fl) = font.gsub().and_then(|t| t.feature_list()) {
        gsub.extend(fl.feature_records().iter().map(|r| r.feature_tag().to_string()));
    }
    if let Ok(fl) = font.gpos().and_then(|t| t.feature_list()) {
        gpos.extend(fl.feature_records().iter().map(|r| r.feature_tag().to_string()));
    }
    for v in [&mut gsub, &mut gpos] {
        v.sort();
        v.dedup();
    }
    (gsub, gpos)
}

pub(crate) fn metadata(font: &FontRef) -> Metadata {
    let os2 = font.os2().ok();
    let license_description = name(font, StringId::LICENSE_DESCRIPTION);
    let license_url = name(font, StringId::LICENSE_URL);
    let (gsub_features, gpos_features) = feature_tags(font);
    Metadata {
        family: name(font, StringId::FAMILY_NAME),
        subfamily: name(font, StringId::SUBFAMILY_NAME),
        typographic_family: name(font, StringId::TYPOGRAPHIC_FAMILY_NAME),
        typographic_subfamily: name(font, StringId::TYPOGRAPHIC_SUBFAMILY_NAME),
        full_name: name(font, StringId::FULL_NAME),
        postscript_name: name(font, StringId::POSTSCRIPT_NAME),
        version: name(font, StringId::VERSION_STRING),
        designer: name(font, StringId::DESIGNER),
        manufacturer: name(font, StringId::MANUFACTURER),
        license_class: classify_license(license_description.as_deref(), license_url.as_deref()),
        license_description,
        license_url,
        units_per_em: font.head().map(|h| h.units_per_em()).unwrap_or(0),
        weight_class: os2.as_ref().map(|o| o.us_weight_class()),
        width_class: os2.as_ref().map(|o| o.us_width_class()),
        fs_type: os2.as_ref().map(|o| o.fs_type()),
        panose: os2.as_ref().and_then(|o| o.panose_10().try_into().ok()),
        italic_angle: font.post().map(|p| p.italic_angle().to_f64() as f32).unwrap_or(0.0),
        outline_format: outline_format(font),
        color_tables: [b"COLR", b"CBDT", b"sbix", b"SVG "]
            .iter()
            .filter(|t| has(font, t))
            .map(|t| String::from_utf8_lossy(*t).trim().to_string())
            .collect(),
        num_glyphs: font.maxp().map(|m| m.num_glyphs()).unwrap_or(0),
        axes: font
            .axes()
            .iter()
            .map(|a| AxisInfo { tag: a.tag().to_string(), min: a.min_value(), default: a.default_value(), max: a.max_value() })
            .collect(),
        named_instances: font
            .named_instances()
            .iter()
            .filter_map(|i| name(font, i.subfamily_name_id()))
            .collect(),
        gsub_features,
        gpos_features,
    }
}
