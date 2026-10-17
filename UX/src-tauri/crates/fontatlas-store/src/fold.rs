//! Gập dấu tiếng Việt cho tìm kiếm.
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// NFD → bỏ dấu → đ→d → chữ thường → gộp khoảng trắng. FTS5 không tự gập `đ`.
pub fn vi_fold(s: &str) -> String {
    let folded: String = s
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| if c == 'đ' || c == 'Đ' { 'd' } else { c })
        .flat_map(char::to_lowercase)
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cách khớp tên: FTS trigram (≥ 3 ký tự sau gập) hoặc LIKE tiền tố từ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TextMatch {
    Fts(String),
    Prefix { start: String, word: String },
}

pub(crate) fn text_match(query: &str) -> Option<TextMatch> {
    let f: String = vi_fold(query).chars().filter(|c| *c != '"').collect();
    let f = f.trim();
    if f.is_empty() {
        return None;
    }
    if f.chars().count() >= 3 {
        return Some(TextMatch::Fts(format!("\"{f}\"")));
    }
    let e = f.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    Some(TextMatch::Prefix { start: format!("{e}%"), word: format!("% {e}%") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_match_picks_fts_or_prefix() {
        assert_eq!(text_match("Đẹp"), Some(TextMatch::Fts("\"dep\"".into())));
        assert_eq!(
            text_match("PL"),
            Some(TextMatch::Prefix { start: "pl%".into(), word: "% pl%".into() })
        );
        assert_eq!(text_match("   "), None);
    }

    #[test]
    fn text_match_neutralises_syntax() {
        // dấu " bị bỏ, % _ \ được escape cho LIKE ... ESCAPE '\'
        assert_eq!(text_match("a\"b*c"), Some(TextMatch::Fts("\"ab*c\"".into())));
        assert_eq!(
            text_match("_%"),
            Some(TextMatch::Prefix { start: "\\_\\%%".into(), word: "% \\_\\%%".into() })
        );
    }
}
