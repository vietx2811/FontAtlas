//! Dữ liệu ngôn ngữ từ Hyperglot (Apache-2.0, xem `resources/NOTICE-hyperglot.md`) và định nghĩa mức hỗ trợ.
//! Dữ liệu đã chuyển sẵn bởi `tools/langdata/build.py` (không cần mạng lúc build).
//!
//! Mức hỗ trợ của một font với ngôn ngữ L (đo trên cmap thật, do bên gọi cung cấp tập grapheme thiếu):
//! - **Full**: có 100% `base` và 100% `marks`.
//! - **Near**: có ≥ 90% `base`, đủ toàn bộ `marks` và đủ các "chữ cái riêng" của L.
//!   Chữ cái riêng: grapheme `base` ngoài Basic Latin (script Latin), hoặc xuất hiện ở < 50% số ngôn ngữ cùng
//!   script (script khác). Áp dụng cho mọi ngôn ngữ.
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde::Deserialize;

const RAW: &str = include_str!("../data/langdata.json");
/// Ngưỡng `Near` trên độ phủ `base`.
pub const NEAR_RATIO: f32 = 0.90;

#[derive(Debug, Deserialize)]
struct Raw {
    hyperglot_sha: String,
    hyperglot_version: String,
    converter: u32,
    langs: Vec<RawLang>,
}

#[derive(Debug, Deserialize)]
struct RawLang {
    id: String,
    bcp47: String,
    name_en: String,
    autonym: String,
    script: String,
    base: String,
    marks: String,
}

#[derive(Debug, Clone)]
pub struct LangEntry {
    /// Khoá ổn định: mã ISO 639-3 (`vie`); orthography thứ hai của cùng ngôn ngữ: `srp-Latin`.
    pub id: String,
    /// Mã ngôn ngữ cho `Intl.DisplayNames` (ISO 639-3 của ngôn ngữ).
    pub bcp47: String,
    pub name_en: String,
    pub autonym: String,
    pub script: String,
    pub base: Vec<String>,
    pub marks: Vec<String>,
    /// `core[i]` = `base[i]` là chữ cái riêng của ngôn ngữ.
    core: Vec<bool>,
}

pub struct LangDb {
    pub entries: Vec<LangEntry>,
    index: HashMap<String, usize>,
    /// Hợp mọi grapheme `base` và `marks` (không trùng): một lần tính cmap/face rồi đánh giá mọi ngôn ngữ.
    pub all_graphemes: Vec<String>,
    version: String,
    pub hyperglot_sha: String,
    pub hyperglot_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LangLevel {
    /// Thứ tự: Near < Full (dùng cho "ít nhất mức này").
    Near,
    Full,
}

impl LangLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LangLevel::Near => "near",
            LangLevel::Full => "full",
        }
    }
    pub fn parse(s: &str) -> Option<LangLevel> {
        match s {
            "near" => Some(LangLevel::Near),
            "full" => Some(LangLevel::Full),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LangCoverage {
    pub base_have: u32,
    pub base_total: u32,
    pub marks_have: u32,
    pub marks_total: u32,
    pub core_ok: bool,
    pub missing: Vec<String>,
    pub missing_marks: Vec<String>,
}

impl LangCoverage {
    pub fn level(&self) -> Option<LangLevel> {
        if self.base_have == self.base_total && self.marks_have == self.marks_total {
            return Some(LangLevel::Full);
        }
        let ratio = self.base_have as f32 / self.base_total.max(1) as f32;
        if ratio >= NEAR_RATIO && self.marks_have == self.marks_total && self.core_ok {
            Some(LangLevel::Near)
        } else {
            None
        }
    }
}

impl LangEntry {
    /// `missing`: tập grapheme font KHÔNG có (từ `fontatlas_preview::missing_graphemes` trên `all_graphemes`).
    pub fn evaluate(&self, missing: &HashSet<String>) -> LangCoverage {
        let miss_base: Vec<&String> = self.base.iter().filter(|g| missing.contains(*g)).collect();
        let miss_marks: Vec<String> = self.marks.iter().filter(|g| missing.contains(*g)).cloned().collect();
        let core_ok = self
            .base
            .iter()
            .zip(&self.core)
            .all(|(g, &is_core)| !is_core || !missing.contains(g));
        LangCoverage {
            base_have: (self.base.len() - miss_base.len()) as u32,
            base_total: self.base.len() as u32,
            marks_have: (self.marks.len() - miss_marks.len()) as u32,
            marks_total: self.marks.len() as u32,
            core_ok,
            missing: miss_base.into_iter().cloned().collect(),
            missing_marks: miss_marks,
        }
    }
}

impl LangDb {
    pub fn get(&self, id: &str) -> Option<&LangEntry> {
        self.index.get(id).map(|&i| &self.entries[i])
    }
    /// `hg-<sha8>-<phiên bản bộ chuyển đổi>`: đổi giá trị này thì độ phủ đã lưu bị coi là cũ.
    pub fn version(&self) -> &str {
        &self.version
    }
}

fn build() -> LangDb {
    let raw: Raw = serde_json::from_str(RAW).expect("langdata.json hỏng");
    // chữ cái riêng ở script không phải Latin: grapheme xuất hiện ở < 50% ngôn ngữ cùng script
    let mut per_script: HashMap<&str, (usize, HashMap<&str, usize>)> = HashMap::new();
    for l in &raw.langs {
        let e = per_script.entry(l.script.as_str()).or_default();
        e.0 += 1;
        for g in l.base.split_whitespace() {
            *e.1.entry(g).or_default() += 1;
        }
    }
    let mut entries = Vec::with_capacity(raw.langs.len());
    let mut all = Vec::new();
    let mut seen = HashSet::new();
    for l in &raw.langs {
        let base: Vec<String> = l.base.split_whitespace().map(str::to_owned).collect();
        let marks: Vec<String> = l.marks.split_whitespace().map(str::to_owned).collect();
        let (n_langs, counts) = &per_script[l.script.as_str()];
        let core = base
            .iter()
            .map(|g| {
                if l.script == "Latin" {
                    !g.is_ascii()
                } else {
                    (counts[g.as_str()] as f32) < 0.5 * *n_langs as f32
                }
            })
            .collect();
        for g in base.iter().chain(&marks) {
            if seen.insert(g.clone()) {
                all.push(g.clone());
            }
        }
        entries.push(LangEntry {
            id: l.id.clone(),
            bcp47: l.bcp47.clone(),
            name_en: l.name_en.clone(),
            autonym: l.autonym.clone(),
            script: l.script.clone(),
            base,
            marks,
            core,
        });
    }
    let index = entries.iter().enumerate().map(|(i, e)| (e.id.clone(), i)).collect();
    let sha8: String = raw.hyperglot_sha.chars().take(8).collect();
    LangDb {
        entries,
        index,
        all_graphemes: all,
        version: format!("hg-{sha8}-{}", raw.converter),
        hyperglot_sha: raw.hyperglot_sha,
        hyperglot_version: raw.hyperglot_version,
    }
}

/// Cơ sở dữ liệu ngôn ngữ (giải mã một lần).
pub fn langdb() -> &'static LangDb {
    static DB: OnceLock<LangDb> = OnceLock::new();
    DB.get_or_init(build)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn miss(gs: &[&str]) -> HashSet<String> {
        gs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn database_has_expected_languages_and_version() {
        let db = langdb();
        assert!(db.entries.len() > 600, "{}", db.entries.len());
        assert!(db.version().starts_with("hg-12d0c093-"), "{}", db.version());
        let vie = db.get("vie").expect("vie");
        assert_eq!(vie.base.len(), 186);
        assert_eq!(vie.script, "Latin");
        assert_eq!(vie.autonym, "Tiếng Việt");
        assert!(vie.marks.iter().any(|m| m == "\u{0309}"), "hỏi mark");
        assert!(db.get("eng").is_some() && db.get("fra").is_some());
        // orthography thứ hai của cùng ngôn ngữ có khoá riêng, bcp47 vẫn là mã ngôn ngữ
        let sr = db.get("srp-Latin").expect("srp-Latin");
        assert_eq!(sr.bcp47, "srp");
    }

    #[test]
    fn all_graphemes_cover_every_entry_without_duplicates() {
        let db = langdb();
        let set: HashSet<&String> = db.all_graphemes.iter().collect();
        assert_eq!(set.len(), db.all_graphemes.len());
        for e in &db.entries {
            assert!(!e.base.is_empty(), "{}", e.id);
            for g in e.base.iter().chain(&e.marks) {
                assert!(set.contains(g), "{} {g}", e.id);
            }
        }
    }

    #[test]
    fn levels_full_near_and_none() {
        let db = langdb();
        let vie = db.get("vie").unwrap();
        // đủ hết
        let c = vie.evaluate(&miss(&[]));
        assert_eq!((c.base_have, c.base_total), (186, 186));
        assert_eq!(c.level(), Some(LangLevel::Full));
        // thiếu một chữ ngoài Basic Latin (chữ cái riêng) → không near dù ≥ 90%
        let c = vie.evaluate(&miss(&["ặ"]));
        assert!(c.base_have as f32 / 186.0 > 0.99);
        assert!(!c.core_ok);
        assert_eq!(c.level(), None);
        assert_eq!(c.missing, vec!["ặ".to_string()]);
        // thiếu một mark → không near
        let c = vie.evaluate(&miss(&["\u{0309}"]));
        assert_eq!(c.missing_marks, vec!["\u{0309}".to_string()]);
        assert_eq!(c.level(), None);
        // tiếng Anh: thiếu 1 chữ ASCII (không phải chữ cái riêng) ⇒ ≥ 90% và near
        let eng = db.get("eng").unwrap();
        let c = eng.evaluate(&miss(&["Q"]));
        assert!(c.core_ok);
        assert_eq!(c.level(), Some(LangLevel::Near));
        // thiếu quá 10% ⇒ không đạt
        let many: Vec<&str> = eng.base.iter().take(10).map(String::as_str).collect();
        assert_eq!(eng.evaluate(&miss(&many)).level(), None);
    }

    #[test]
    fn core_letters_for_non_latin_scripts_use_script_frequency() {
        let db = langdb();
        let rus = db.get("rus").unwrap();
        // chữ Cyrillic dùng chung (А) không phải chữ cái riêng của tiếng Nga; thiếu nó vẫn near nếu ≥ 90%
        let c = rus.evaluate(&miss(&["А"]));
        assert!(c.core_ok, "А dùng chung");
        assert_eq!(c.level(), Some(LangLevel::Near));
    }

    #[test]
    fn level_ordering_supports_at_least_semantics() {
        assert!(LangLevel::Full > LangLevel::Near);
        assert_eq!(LangLevel::parse("near"), Some(LangLevel::Near));
        assert_eq!(LangLevel::parse("x"), None);
    }
}
