//! Metadata: name, OS/2, head, post, fvar, định dạng outline, OT features, giấy phép.
use read_fonts::{FontRef, TableProvider, types::Tag};
use serde::{Deserialize, Serialize};
use skrifa::{MetadataProvider, string::StringId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseClass {
    Ofl,
    Apache,
    Other,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutlineFormat {
    Glyf,
    Cff,
    Cff2,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisInfo {
    pub tag: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
}

/// Named instance kèm toạ độ user-space theo thứ tự trục `fvar`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedInstanceInfo {
    pub name: Option<String>,
    pub coords: Vec<(String, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub units_per_em: Option<u16>,
    pub weight_class: Option<u16>,
    pub width_class: Option<u16>,
    pub fs_type: Option<u16>,
    pub panose: Option<[u8; 10]>,
    /// OS/2 sFamilyClass: (lớp IBM, lớp con). `None` khi không có OS/2 hoặc bằng 0 (không phân loại).
    #[serde(default)]
    pub family_class: Option<(u8, u8)>,
    /// OS/2 achVendID (4 ký tự, đã bỏ khoảng trắng cuối).
    #[serde(default)]
    pub vendor_id: Option<String>,
    /// `post.isFixedPitch != 0`.
    #[serde(default)]
    pub is_fixed_pitch: bool,
    /// cmap chỉ có subtable Windows Symbol (3,0), không có subtable Unicode.
    #[serde(default)]
    pub symbol_cmap: bool,
    /// Số chữ cái ASCII (a–z, A–Z; tối đa 52) có glyph trong cmap.
    #[serde(default)]
    pub latin_letters: u8,
    pub italic_angle: Option<f32>,
    pub outline_format: OutlineFormat,
    pub color_tables: Vec<String>,
    pub num_glyphs: Option<u16>,
    pub axes: Vec<AxisInfo>,
    pub named_instances: Vec<String>,
    #[serde(default)]
    pub named_instance_coords: Vec<NamedInstanceInfo>,
    /// nameID 0
    #[serde(default)]
    pub copyright: Option<String>,
    /// nameID 11
    #[serde(default)]
    pub vendor_url: Option<String>,
    pub gsub_features: Vec<String>,
    pub gpos_features: Vec<String>,
    /// Tag script OpenType (GSUB + GPOS ScriptList, bỏ DFLT), đã sort.
    pub scripts: Vec<String>,
    /// Non-Latin script blocks with >= CMAP_SCRIPT_MIN mapped code points (cmap, not layout),
    /// for fonts whose layout is AAT-only and has no OpenType script list (Thonburi, Sangam MN).
    #[serde(default)]
    pub cmap_scripts: Vec<String>,
    /// Dải optical size (pt) từ OS/2 v5; None nếu OS/2 < v5.
    pub os2_optical_size_pt: Option<(f32, f32)>,
    /// Dữ liệu D2 cho Compare (năm, nhóm feature, kerning, cấu trúc style, fsType, loại license).
    #[serde(default)]
    pub extras: crate::extras::FontExtras,
}

pub fn classify_license(description: Option<&str>, url: Option<&str>) -> LicenseClass {
    if description.is_none() && url.is_none() {
        return LicenseClass::Unknown;
    }
    let s = format!("{} {}", description.unwrap_or(""), url.unwrap_or("")).to_lowercase();
    if s.contains("open font license")
        || s.contains("openfontlicense")
        || s.contains("scripts.sil.org/ofl")
    {
        LicenseClass::Ofl
    } else if s.contains("apache license") || s.contains("apache.org/licenses") {
        LicenseClass::Apache
    } else {
        LicenseClass::Other
    }
}

fn name(font: &FontRef, id: StringId) -> Option<String> {
    font.localized_strings(id)
        .english_or_first()
        .map(|s| s.to_string())
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

/// Minimum mapped code points in a block to count the script as covered (whole alphabets,
/// not a few borrowed signs). Thai alone is 87; Cherokee 85; Malayalam ~100.
const CMAP_SCRIPT_MIN: usize = 40;
const CMAP_BLOCKS: &[(&str, u32, u32)] = &[
    ("deva", 0x0900, 0x097F), ("beng", 0x0980, 0x09FF), ("guru", 0x0A00, 0x0A7F),
    ("gujr", 0x0A80, 0x0AFF), ("orya", 0x0B00, 0x0B7F), ("taml", 0x0B80, 0x0BFF),
    ("telu", 0x0C00, 0x0C7F), ("knda", 0x0C80, 0x0CFF), ("mlym", 0x0D00, 0x0D7F),
    ("sinh", 0x0D80, 0x0DFF), ("thai", 0x0E00, 0x0E7F), ("lao ", 0x0E80, 0x0EFF),
    ("tibt", 0x0F00, 0x0FFF), ("mymr", 0x1000, 0x109F), ("ethi", 0x1200, 0x137F),
    ("cher", 0x13A0, 0x13FF), ("cans", 0x1400, 0x167F), ("khmr", 0x1780, 0x17FF),
    ("mong", 0x1800, 0x18AF), ("kana", 0x3040, 0x30FF), ("hani", 0x4E00, 0x9FFF),
    ("hang", 0xAC00, 0xD7AF),
];

fn cmap_scripts(font: &FontRef) -> Vec<String> {
    let mut counts = vec![0usize; CMAP_BLOCKS.len()];
    for (cp, _) in font.charmap().mappings() {
        if let Some(i) = CMAP_BLOCKS.iter().position(|&(_, lo, hi)| cp >= lo && cp <= hi) {
            counts[i] += 1;
        }
    }
    CMAP_BLOCKS
        .iter()
        .zip(counts)
        .filter(|(_, n)| *n >= CMAP_SCRIPT_MIN)
        .map(|((tag, _, _), _)| tag.to_string())
        .collect()
}

/// cmap có subtable Windows Symbol (3,0) và không có subtable Unicode nào (Webdings, Wingdings…).
fn symbol_only_cmap(font: &FontRef) -> bool {
    use read_fonts::tables::cmap::PlatformId;
    let Ok(cmap) = font.cmap() else { return false };
    let (mut symbol, mut unicode) = (false, false);
    for r in cmap.encoding_records() {
        match (r.platform_id(), r.encoding_id()) {
            (PlatformId::Windows, 0) => symbol = true,
            (PlatformId::Unicode, _) | (PlatformId::Windows, 1 | 10) => unicode = true,
            _ => {}
        }
    }
    symbol && !unicode
}

fn script_tags(font: &FontRef) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(sl) = font.gsub().and_then(|t| t.script_list()) {
        v.extend(
            sl.script_records()
                .iter()
                .map(|r| r.script_tag().to_string()),
        );
    }
    if let Ok(sl) = font.gpos().and_then(|t| t.script_list()) {
        v.extend(
            sl.script_records()
                .iter()
                .map(|r| r.script_tag().to_string()),
        );
    }
    v.retain(|t| t != "DFLT");
    v.sort();
    v.dedup();
    v
}

pub(crate) fn feature_tags(font: &FontRef) -> (Vec<String>, Vec<String>) {
    let mut gsub = Vec::new();
    let mut gpos = Vec::new();
    if let Ok(fl) = font.gsub().and_then(|t| t.feature_list()) {
        gsub.extend(
            fl.feature_records()
                .iter()
                .map(|r| r.feature_tag().to_string()),
        );
    }
    if let Ok(fl) = font.gpos().and_then(|t| t.feature_list()) {
        gpos.extend(
            fl.feature_records()
                .iter()
                .map(|r| r.feature_tag().to_string()),
        );
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
    let copyright = name(font, StringId::COPYRIGHT_NOTICE);
    let all_features: Vec<String> = gsub_features.iter().chain(gpos_features.iter()).cloned().collect();
    let hint = format!(
        "{} {}",
        name(font, StringId::FULL_NAME).unwrap_or_default(),
        name(font, StringId::SUBFAMILY_NAME).unwrap_or_default()
    );
    let extras = crate::extras::compute(
        font,
        &all_features,
        copyright.as_deref(),
        (license_description.as_deref(), license_url.as_deref()),
        &hint,
    );
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
        units_per_em: font.head().ok().map(|h| h.units_per_em()),
        weight_class: os2.as_ref().map(|o| o.us_weight_class()),
        width_class: os2.as_ref().map(|o| o.us_width_class()),
        fs_type: os2.as_ref().map(|o| o.fs_type()),
        panose: os2.as_ref().and_then(|o| o.panose_10().try_into().ok()),
        family_class: os2.as_ref().and_then(|o| {
            let v = o.s_family_class();
            (v != 0).then_some(((v >> 8) as u8, (v & 0xff) as u8))
        }),
        vendor_id: os2.as_ref().map(|o| o.ach_vend_id().to_string().trim_matches(|c: char| c.is_whitespace() || c.is_control()).to_string()).filter(|s| !s.is_empty()),
        is_fixed_pitch: font.post().is_ok_and(|p| p.is_fixed_pitch() != 0),
        symbol_cmap: symbol_only_cmap(font),
        latin_letters: ('a'..='z').chain('A'..='Z').filter(|c| font.charmap().map(*c).is_some()).count() as u8,
        italic_angle: font.post().ok().map(|p| p.italic_angle().to_f64() as f32),
        outline_format: outline_format(font),
        color_tables: [b"COLR", b"CBDT", b"sbix", b"SVG "]
            .iter()
            .filter(|t| has(font, t))
            .map(|t| String::from_utf8_lossy(*t).trim().to_string())
            .collect(),
        num_glyphs: font.maxp().ok().map(|m| m.num_glyphs()),
        axes: font
            .axes()
            .iter()
            .map(|a| AxisInfo {
                tag: a.tag().to_string(),
                min: a.min_value(),
                default: a.default_value(),
                max: a.max_value(),
            })
            .collect(),
        named_instances: font
            .named_instances()
            .iter()
            .filter_map(|i| name(font, i.subfamily_name_id()))
            .collect(),
        named_instance_coords: font
            .named_instances()
            .iter()
            .map(|i| NamedInstanceInfo {
                name: name(font, i.subfamily_name_id()),
                coords: font
                    .axes()
                    .iter()
                    .map(|a| a.tag().to_string())
                    .zip(i.user_coords())
                    .collect(),
            })
            .collect(),
        copyright,
        vendor_url: name(font, StringId::VENDOR_URL),
        gsub_features,
        gpos_features,
        scripts: script_tags(font),
        cmap_scripts: cmap_scripts(font),
        // OS/2 lưu theo TWIP (1/20 pt).
        os2_optical_size_pt: os2.as_ref().and_then(|o| {
            Some((
                o.us_lower_optical_point_size()? as f32 / 20.0,
                o.us_upper_optical_point_size()? as f32 / 20.0,
            ))
        }),
        extras,
    }
}
