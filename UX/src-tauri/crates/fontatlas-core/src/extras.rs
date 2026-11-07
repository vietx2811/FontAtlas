//! Dữ liệu D2 cho Compare: năm, nhóm OpenType feature (kèm tên ssXX), số kerning pair, cấu trúc style, diễn giải fsType,
//! phân loại license theo từ khoá. Hàm thuần (trừ `compute` đọc font); lưu trong `Metadata.extras`.
use read_fonts::{FontRef, TableProvider, types::Tag};
use serde::{Deserialize, Serialize};
use skrifa::{MetadataProvider, string::StringId};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Years {
    pub created: Option<u16>,
    pub modified: Option<u16>,
    pub copyright: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StylisticSet {
    pub tag: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OtGroup {
    pub group: String,
    pub tags: Vec<String>,
    pub sets: Vec<StylisticSet>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kerning {
    /// "gpos" | "kern" | "none"
    pub source: String,
    /// Cặp glyph khai tường minh (PairPos format 1, hoặc bảng kern).
    pub pairs: u32,
    /// Số ô ma trận lớp CÓ giá trị khác 0 (PairPos format 2). Mỗi ô đại diện nhiều cặp glyph — khác đơn vị với `pairs`, không cộng hai số.
    pub class_pairs: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleStructure {
    /// "upright" | "italic" | "oblique"
    pub italic_kind: String,
    pub ital_axis: bool,
    pub slnt_axis: bool,
    pub wght_range: Option<[f32; 2]>,
    pub wdth_range: Option<[f32; 2]>,
    pub opsz_range: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Embedding {
    pub fs_type: Option<u16>,
    /// "installable" | "restricted" | "preview_print" | "editable" | "unknown"
    pub level: String,
    /// "no_subsetting" | "bitmap_only"
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontExtras {
    pub years: Years,
    pub ot_groups: Vec<OtGroup>,
    pub kerning: Kerning,
    pub style: StyleStructure,
    pub embedding: Embedding,
    /// "ofl" | "apache" | "ufl" | "eula" | "other" | "unknown"
    pub license_kind: String,
}

/// Năm từ `LONGDATETIME` (giây từ 1904-01-01 UTC). `None` khi 0 hoặc ngoài 1950–2100.
/// Số phiên bản và ngày sửa từ bảng `head` (đọc nhẹ, không phân tích glyph) — dùng lúc cần so phiên bản.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeadInfo {
    /// head.fontRevision (vd 2.137); `None` khi 0/thiếu.
    pub revision: Option<f32>,
    /// head.modified (unix giây UTC); `None` khi 0/ngoài khoảng.
    pub modified: Option<i64>,
}

/// Đọc `head` của face `face_index` thẳng từ tệp: chỉ bảng thư mục sfnt + 54 byte của `head` (không đọc cả tệp, không
/// băm, không giải nén). Hỗ trợ TTF/OTF và TTC/OTC (theo `face_index`); WOFF/WOFF2 → `None` (bảng nén).
/// Người gọi phải kiểm tệp có sẵn trên đĩa (không phải placeholder iCloud) TRƯỚC khi gọi.
pub fn head_info(path: &std::path::Path, face_index: u32) -> Option<HeadInfo> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let read_at = |f: &mut std::fs::File, off: u64, n: usize| -> Option<Vec<u8>> {
        if off.checked_add(n as u64)? > len {
            return None;
        }
        f.seek(SeekFrom::Start(off)).ok()?;
        let mut b = vec![0u8; n];
        f.read_exact(&mut b).ok()?;
        Some(b)
    };
    let be32 = |b: &[u8], i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let magic = read_at(&mut f, 0, 4)?;
    let sfnt_off: u64 = match &magic[..] {
        b"ttcf" => {
            let h = read_at(&mut f, 0, 12)?;
            let n = be32(&h, 8);
            if face_index >= n || n > 4096 {
                return None;
            }
            let o = read_at(&mut f, 12 + 4 * u64::from(face_index), 4)?;
            u64::from(be32(&o, 0))
        }
        [0, 1, 0, 0] | b"OTTO" | b"true" | b"typ1" if face_index == 0 => 0,
        _ => return None, // WOFF/WOFF2 hoặc không phải sfnt
    };
    let dir = read_at(&mut f, sfnt_off, 12)?;
    let num_tables = usize::from(u16::from_be_bytes([dir[4], dir[5]]));
    if num_tables == 0 || num_tables > 4096 {
        return None;
    }
    let recs = read_at(&mut f, sfnt_off + 12, num_tables * 16)?;
    let rec = recs.chunks_exact(16).find(|r| &r[0..4] == b"head")?;
    let (off, tlen) = (u64::from(be32(rec, 8)), be32(rec, 12));
    if tlen < 54 {
        return None;
    }
    let head = read_at(&mut f, off, 54)?;
    let revision = be32(&head, 4) as i32 as f64 / 65536.0;
    let modified = i64::from_be_bytes(head[28..36].try_into().ok()?);
    Some(HeadInfo {
        revision: Some(revision as f32).filter(|r| r.is_finite() && *r > 0.0),
        modified: unix_from_longdatetime(modified),
    })
}

/// `LONGDATETIME` (giây từ 1904-01-01) → unix giây; `None` khi 0 hoặc trước 1970 hoặc sau 2100.
pub fn unix_from_longdatetime(secs: i64) -> Option<i64> {
    let u = secs.checked_sub(2_082_844_800)?;
    (secs > 0 && (0..=4_102_444_800).contains(&u)).then_some(u)
}

pub fn year_from_longdatetime(secs: i64) -> Option<u16> {
    if secs <= 0 {
        return None;
    }
    // 1904-01-01 → 1970-01-01 = 24107 ngày
    let days = secs.div_euclid(86_400) - 24_107;
    // civil_from_days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (1950..=2100).contains(&y).then_some(y as u16)
}

/// Các năm (1900–2099) đứng riêng thành số 4 chữ số, kèm vị trí byte.
fn years_in(s: &str) -> Vec<(usize, u16)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let j = (i..b.len()).find(|k| !b[*k].is_ascii_digit()).unwrap_or(b.len());
            if j - i == 4
                && let Ok(y) = s[i..j].parse::<u16>()
                && (1900..=2099).contains(&y)
            {
                out.push((i, y));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Năm bản quyền: ưu tiên số đứng ngay sau `©` / `Copyright` / `(c)` (trong 24 byte), nếu không thì năm sớm nhất tìm thấy.
pub fn copyright_year(s: &str) -> Option<u16> {
    let years = years_in(s);
    let lower = s.to_lowercase();
    // vị trí byte trong `lower` có thể lệch với `s` khi có ký tự đổi độ dài khi hạ chữ: chỉ dùng khi độ dài bằng nhau
    if lower.len() == s.len() {
        let marked = ["©", "copyright", "(c)"]
            .iter()
            .flat_map(|m| lower.match_indices(m).map(move |(p, m)| p + m.len()))
            .filter_map(|end| years.iter().find(|(p, _)| *p >= end && *p <= end + 24).map(|(_, y)| *y))
            .min();
        if marked.is_some() {
            return marked;
        }
    }
    years.iter().map(|(_, y)| *y).min()
}

/// OS/2 fsType: bit 1 restricted, 2 preview&print, 3 editable, 8 no subsetting, 9 bitmap only; 0 = installable.
pub fn decode_fs_type(fs: Option<u16>) -> Embedding {
    let Some(v) = fs else { return Embedding { fs_type: None, level: "unknown".into(), flags: vec![] } };
    // thấp nhất trong 3 bit đầu thắng (đa số font chỉ đặt một bit); bit 1 (restricted) là hạn chế nhất
    let level = if v & 0x0002 != 0 {
        "restricted"
    } else if v & 0x0004 != 0 {
        "preview_print"
    } else if v & 0x0008 != 0 {
        "editable"
    } else {
        "installable"
    };
    let mut flags = Vec::new();
    if v & 0x0100 != 0 {
        flags.push("no_subsetting".to_string());
    }
    if v & 0x0200 != 0 {
        flags.push("bitmap_only".to_string());
    }
    Embedding { fs_type: Some(v), level: level.into(), flags }
}

/// Phân loại license theo từ khoá (chỉ để ghi chú, không chặn).
pub fn license_kind(description: Option<&str>, url: Option<&str>) -> &'static str {
    if description.is_none_or(|s| s.trim().is_empty()) && url.is_none_or(|s| s.trim().is_empty()) {
        return "unknown";
    }
    let s = format!("{} {}", description.unwrap_or(""), url.unwrap_or("")).to_lowercase();
    if s.contains("open font license") || s.contains("openfontlicense") || s.contains("scripts.sil.org/ofl") {
        "ofl"
    } else if s.contains("apache license") || s.contains("apache.org/licenses") {
        "apache"
    } else if s.contains("ubuntu font licen") || s.contains("ubuntu.com/legal/font") {
        "ufl"
    } else if s.contains("eula")
        || s.contains("end user license")
        || s.contains("end-user license")
        || s.contains("all rights reserved")
        || s.contains("commercial")
    {
        "eula"
    } else {
        "other"
    }
}

const GROUPS: &[(&str, &[&str])] = &[
    ("figures", &["lnum", "onum", "tnum", "pnum", "zero"]),
    ("small_caps", &["smcp", "c2sc"]),
    ("fractions", &["frac", "afrc", "numr", "dnom"]),
    ("case", &["case", "cpsp"]),
    ("ligatures", &["liga", "dlig", "clig", "hlig", "rlig"]),
    ("swash", &["swsh", "cswh"]),
];

fn is_numbered(tag: &str, prefix: &str, max: u32) -> bool {
    tag.strip_prefix(prefix).filter(|n| n.len() == 2).and_then(|n| n.parse::<u32>().ok()).is_some_and(|n| (1..=max).contains(&n))
}

/// Nhóm tag thành nhóm dễ đọc. `ss_names` = tên ssXX đã đọc từ bảng `name`.
pub fn ot_groups(features: &[String], ss_names: &[(String, Option<String>)]) -> Vec<OtGroup> {
    let mut out = Vec::new();
    for (g, tags) in GROUPS {
        let mut t: Vec<String> = features.iter().filter(|f| tags.contains(&f.as_str())).cloned().collect();
        t.sort();
        t.dedup();
        if !t.is_empty() {
            out.push(OtGroup { group: (*g).into(), tags: t, sets: vec![] });
        }
    }
    let mut ss: Vec<String> = features.iter().filter(|f| is_numbered(f, "ss", 20)).cloned().collect();
    ss.sort();
    ss.dedup();
    if !ss.is_empty() {
        let sets = ss.iter().map(|t| StylisticSet { tag: t.clone(), name: ss_names.iter().find(|(k, _)| k == t).and_then(|(_, n)| n.clone()) }).collect();
        out.push(OtGroup { group: "stylistic_sets".into(), tags: ss, sets: sets_sorted(sets) });
    }
    let mut cv: Vec<String> = features.iter().filter(|f| is_numbered(f, "cv", 99)).cloned().collect();
    cv.sort();
    cv.dedup();
    if !cv.is_empty() {
        out.push(OtGroup { group: "character_variants".into(), tags: cv, sets: vec![] });
    }
    out
}

fn sets_sorted(mut v: Vec<StylisticSet>) -> Vec<StylisticSet> {
    v.sort_by(|a, b| a.tag.cmp(&b.tag));
    v
}

fn ss_names(font: &FontRef) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let Ok(fl) = font.gsub().and_then(|t| t.feature_list()) else { return out };
    let data = fl.offset_data();
    for r in fl.feature_records() {
        let tag = r.feature_tag().to_string();
        if !is_numbered(&tag, "ss", 20) || out.iter().any(|(t, _): &(String, Option<String>)| *t == tag) {
            continue;
        }
        let name = r
            .feature(data)
            .ok()
            .and_then(|f| f.feature_params())
            .and_then(|p| p.ok())
            .and_then(|p| match p {
                read_fonts::tables::layout::FeatureParams::StylisticSet(s) => Some(s.ui_name_id()),
                _ => None,
            })
            .and_then(|id| font.localized_strings(StringId::new(id.to_u16())).english_or_first())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty());
        out.push((tag, name));
    }
    out
}

/// Chặn trên số ô ma trận lớp duyệt cho mỗi font (font CJK có ma trận rất lớn).
const MAX_CLASS_CELLS: u32 = 4_000_000;

fn kerning(font: &FontRef) -> Kerning {
    use read_fonts::tables::gpos::{PairPos, PositionSubtables};
    let mut k = Kerning { source: "none".into(), ..Default::default() };
    if let Ok(gpos) = font.gpos()
        && let (Ok(fl), Ok(ll)) = (gpos.feature_list(), gpos.lookup_list())
    {
        let data = fl.offset_data();
        let mut seen = std::collections::BTreeSet::new();
        for r in fl.feature_records() {
            if r.feature_tag() != Tag::new(b"kern") {
                continue;
            }
            let Ok(f) = r.feature(data) else { continue };
            for idx in f.lookup_list_indices() {
                seen.insert(idx.get());
            }
        }
        // font không gắn lookup vào feature `kern` (hiếm): đếm mọi lookup PairPos
        if seen.is_empty() {
            seen.extend(0..ll.lookups().len() as u16);
        }
        for idx in seen {
            let Ok(lookup) = ll.lookups().get(idx as usize) else { continue };
            let Ok(PositionSubtables::Pair(subs)) = lookup.subtables() else { continue };
            for sub in subs.iter().flatten() {
                match sub {
                    PairPos::Format1(p) => {
                        for set in p.pair_sets().iter().flatten() {
                            k.pairs = k.pairs.saturating_add(u32::from(set.pair_value_count()));
                        }
                    }
                    PairPos::Format2(p) => {
                        // chỉ đếm ô ma trận lớp có giá trị khác 0 (ma trận đầy đủ gồm cả lớp 0 và ô 0 — không phải "cặp"); chặn trên
                        let mut budget = MAX_CLASS_CELLS;
                        'cells: for c1 in p.class1_records().iter().flatten() {
                            for c2 in c1.class2_records().iter().flatten() {
                                if budget == 0 {
                                    break 'cells;
                                }
                                budget -= 1;
                                let nz = |v: &read_fonts::tables::gpos::ValueRecord| {
                                    [v.x_placement(), v.y_placement(), v.x_advance(), v.y_advance()].iter().any(|x| x.is_some_and(|x| x != 0))
                                };
                                if nz(&c2.value_record1) || nz(&c2.value_record2) {
                                    k.class_pairs = k.class_pairs.saturating_add(1);
                                }
                            }
                        }
                    }
                }
            }
        }
        if k.pairs > 0 || k.class_pairs > 0 {
            k.source = "gpos".into();
            return k;
        }
    }
    if let Ok(kern) = font.kern() {
        use read_fonts::tables::kern::SubtableKind;
        for sub in kern.subtables().flatten() {
            if let Ok(SubtableKind::Format0(s0)) = sub.kind() {
                k.pairs = k.pairs.saturating_add(s0.pairs().len() as u32);
            }
        }
        if k.pairs > 0 {
            k.source = "kern".into();
        }
    }
    k
}

fn style_structure(font: &FontRef, name_hint: &str) -> StyleStructure {
    let axes = font.axes();
    let range = |tag: &[u8; 4]| axes.iter().find(|a| a.tag() == Tag::new(tag)).map(|a| [a.min_value(), a.max_value()]);
    let ital_axis = axes.iter().any(|a| a.tag() == Tag::new(b"ital"));
    let slnt_axis = axes.iter().any(|a| a.tag() == Tag::new(b"slnt"));
    let os2 = font.os2().ok();
    let sel = os2.as_ref().map(|o| o.fs_selection().bits()).unwrap_or(0);
    let angle = font.post().ok().map(|p| p.italic_angle().to_f64()).unwrap_or(0.0);
    let n = name_hint.to_lowercase();
    let oblique = sel & 0x0200 != 0 || n.contains("oblique") || n.contains("slanted");
    let italic = sel & 0x0001 != 0 || n.contains("italic") || angle.abs() > 0.5;
    let kind = if oblique {
        "oblique"
    } else if ital_axis || italic {
        "italic"
    } else if slnt_axis && angle.abs() > 0.5 {
        "oblique"
    } else {
        "upright"
    };
    StyleStructure { italic_kind: kind.into(), ital_axis, slnt_axis, wght_range: range(b"wght"), wdth_range: range(b"wdth"), opsz_range: range(b"opsz") }
}

pub(crate) fn compute(font: &FontRef, features: &[String], copyright: Option<&str>, license: (Option<&str>, Option<&str>), name_hint: &str) -> FontExtras {
    let head = font.head().ok();
    let years = Years {
        created: head.as_ref().and_then(|h| year_from_longdatetime(h.created().as_secs())),
        modified: head.as_ref().and_then(|h| year_from_longdatetime(h.modified().as_secs())),
        copyright: copyright.and_then(copyright_year),
    };
    let fs = font.os2().ok().map(|o| o.fs_type());
    FontExtras {
        years,
        ot_groups: ot_groups(features, &ss_names(font)),
        kerning: kerning(font),
        style: style_structure(font, name_hint),
        embedding: decode_fs_type(fs),
        license_kind: license_kind(license.0, license.1).into(),
    }
}
