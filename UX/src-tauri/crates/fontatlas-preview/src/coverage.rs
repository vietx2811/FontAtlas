//! Phủ ký tự: grapheme nào font KHÔNG có glyph (cmap thật, không đoán bằng hình vẽ).
use read_fonts::FontRef;
use skrifa::MetadataProvider;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Ký tự điều khiển cách chữ (ZWJ, bộ chọn biến thể): không cần glyph riêng.
fn is_joiner_or_selector(c: char) -> bool {
    c == '\u{200D}' || c == '\u{200C}' || ('\u{FE00}'..='\u{FE0F}').contains(&c) || ('\u{E0100}'..='\u{E01EF}').contains(&c)
}

/// Grapheme thiếu glyph. Bỏ qua khoảng trắng và ký tự điều khiển (không bao giờ "thiếu").
/// Quy tắc cho grapheme có dấu kết hợp (tách NFD thành chữ nền + dấu):
/// - chữ nền không có trong cmap → thiếu;
/// - dạng dựng sẵn (NFC, một ký tự) có trong cmap → có;
/// - không thì font phải ghép được: mọi dấu kết hợp cũng phải có trong cmap, thiếu một dấu → thiếu.
pub fn missing_graphemes(font: &FontRef<'_>, graphemes: &[String]) -> Vec<String> {
    let cmap = font.charmap();
    let has = |c: char| cmap.map(c).is_some();
    graphemes
        .iter()
        .filter(|g| {
            if g.chars().all(|c| c.is_whitespace() || c.is_control()) {
                return false;
            }
            let nfc: Vec<char> = g.nfc().collect();
            if nfc.len() == 1 && has(nfc[0]) {
                return false;
            }
            let decomposed: Vec<char> = g.nfd().filter(|&c| !is_joiner_or_selector(c)).collect();
            let missing_base = decomposed.iter().any(|&c| !is_combining_mark(c) && !c.is_whitespace() && !c.is_control() && !has(c));
            let missing_mark = decomposed.iter().any(|&c| is_combining_mark(c) && !has(c));
            missing_base || missing_mark
        })
        .cloned()
        .collect()
}
