//! Nhập font từ Figma (B6): đọc tải trọng (JSON hoặc link `fontatlas://fonts?...`) và khớp tên với thư viện.
//! Chỉ đọc Index, không ghi gì, không đoán: `family_only` không bao giờ thành `exact`; nhiều bản thì `ambiguous`.
//! Đầu vào do bên ngoài tạo ra nên kiểm định chặt (kích thước, số lượng, độ dài, UTF-8).
use std::collections::HashMap;

use base64::Engine;
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use crate::dto::figma::*;
use crate::dto::library::FaceKeyDto;
use crate::error::CmdError;
use crate::services::analysis::{bundle, dto_key};
use crate::state::AppState;

pub const IMPORT_MAX_BYTES: usize = 64 * 1024;
pub const IMPORT_MAX_FONTS: usize = 200;
const STR_MAX_CHARS: usize = 128;
/// Số face tối đa trả cho một mục.
pub const FACES_MAX: usize = 5;

fn coded(code: &str, value: &str) -> CmdError {
    CmdError::new(code, code).with_detail(serde_json::json!({ "value": value }))
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string()
}

fn short_str(v: &Value) -> Option<String> {
    let s = clean(v.as_str()?);
    (!s.is_empty() && s.chars().count() <= STR_MAX_CHARS).then_some(s)
}

/// Link `fontatlas://fonts?v=1&d=<base64url>` → JSON; văn bản khác → coi là JSON thuần.
fn payload_json(text: &str) -> Result<String, CmdError> {
    let Some(rest) = text.strip_prefix("fontatlas://") else { return Ok(text.to_string()) };
    let (host, query) = rest.split_once('?').unwrap_or((rest, ""));
    if host.trim_end_matches('/') != "fonts" {
        return Err(coded("import_format", host));
    }
    let mut d = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("v", v)) if v != "1" => return Err(coded("import_version", v)),
            Some(("d", v)) => d = Some(v),
            _ => {}
        }
    }
    let d = d.ok_or_else(|| coded("import_format", "d"))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(d.trim_end_matches('='))
        .map_err(|_| coded("import_format", "d"))?;
    String::from_utf8(bytes).map_err(|_| coded("import_format", "utf8"))
}

pub fn parse_figma_import(text: &str) -> Result<FigmaImportDto, CmdError> {
    if text.len() > IMPORT_MAX_BYTES {
        return Err(coded("import_too_large", &text.len().to_string()));
    }
    let text = text.trim();
    if text.is_empty() {
        return Err(coded("import_empty", ""));
    }
    let json = payload_json(text)?;
    if json.len() > IMPORT_MAX_BYTES {
        return Err(coded("import_too_large", &json.len().to_string()));
    }
    let v: Value = serde_json::from_str(&json).map_err(|_| coded("import_format", "json"))?;
    match v.get("v").and_then(Value::as_u64) {
        Some(1) => {}
        Some(n) => return Err(coded("import_version", &n.to_string())),
        None => return Err(coded("import_format", "v")),
    }
    let fonts = v.get("fonts").and_then(Value::as_array).ok_or_else(|| coded("import_format", "fonts"))?;
    if fonts.len() > IMPORT_MAX_FONTS {
        return Err(coded("import_too_many_fonts", &fonts.len().to_string()));
    }
    let mut items = Vec::with_capacity(fonts.len());
    for f in fonts {
        let a = f.as_array().filter(|a| a.len() >= 2).ok_or_else(|| coded("import_format", "font"))?;
        let (Some(family), Some(style)) = (short_str(&a[0]), short_str(&a[1])) else {
            return Err(coded("import_format", "font"));
        };
        let figma_missing = a.get(2).is_some_and(|m| m.as_u64() == Some(1) || m.as_bool() == Some(true));
        items.push(FigmaFontItemDto { family, style, figma_missing });
    }
    let file = v.get("file").and_then(short_str);
    Ok(FigmaImportDto { file, items })
}

/// Họ: NFC, chữ thường, gộp khoảng trắng.
fn family_key(s: &str) -> String {
    s.nfc().collect::<String>().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Style: như họ nhưng bỏ khoảng trắng và `-`/`_` ("Semi Bold" = "Semibold" = "semi-bold"). KHÔNG gộp
/// "Italic" với "Oblique" hay "Regular" với "Book": đó là hai kiểu khác nhau.
fn style_key(s: &str) -> String {
    s.nfc().collect::<String>().to_lowercase().chars().filter(|c| !c.is_whitespace() && *c != '-' && *c != '_').collect()
}

pub fn resolve_fonts(state: &AppState, items: &[FontNameDto]) -> Result<Vec<FontMatchDto>, CmdError> {
    if items.len() > IMPORT_MAX_FONTS {
        return Err(coded("import_too_many_fonts", &items.len().to_string()));
    }
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let b = bundle(state)?;
    // họ → [(style_key, style, family, key, missing)]
    type Row = (String, String, String, FaceKeyDto, bool);
    let mut fam: HashMap<String, Vec<Row>> = HashMap::new();
    for p in b.index.profiles() {
        let missing = b.meta.get(&p.key).is_some_and(|m| m.missing);
        fam.entry(family_key(&p.family)).or_default().push((style_key(&p.style), p.style.clone(), p.family.clone(), dto_key(&p.key), missing));
    }
    let face = |r: &Row| FaceMatchDto { face: r.3.clone(), family: r.2.clone(), style: r.1.clone(), missing: r.4 };
    let rank = |a: &&Row, b: &&Row| (a.4, &a.1, &a.3.hash_hex, a.3.face_index).cmp(&(b.4, &b.1, &b.3.hash_hex, b.3.face_index));
    Ok(items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let rows = fam.get(&family_key(&it.family));
            let sk = style_key(&it.style);
            let mut same: Vec<&Row> = rows.map(|r| r.iter().filter(|x| x.0 == sk).collect()).unwrap_or_default();
            let (status, mut picked) = match (rows, same.len()) {
                (None, _) => (MatchStatusDto::None, Vec::new()),
                (Some(r), 0) => (MatchStatusDto::FamilyOnly, r.iter().collect()),
                (Some(_), 1) => (MatchStatusDto::Exact, std::mem::take(&mut same)),
                (Some(_), _) => (MatchStatusDto::Ambiguous, std::mem::take(&mut same)),
            };
            picked.sort_by(rank);
            FontMatchDto { index: i as u32, status, faces: picked.into_iter().take(FACES_MAX).map(face).collect() }
        })
        .collect())
}
