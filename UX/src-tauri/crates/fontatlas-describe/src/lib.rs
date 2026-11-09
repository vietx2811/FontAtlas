//! Từ điển mô tả → bộ lọc trục (README từ điển §4). Không LLM, không chấm "đúng/sai":
//! `evaluate` chỉ cho thứ tự hiển thị.
use std::collections::{BTreeMap, HashMap};

use fontatlas_store::vi_fold;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const BUNDLED: &str = include_str!("../data/descriptor-dictionary.json");

#[derive(Debug, thiserror::Error)]
pub enum DescribeError {
    #[error("từ điển không đọc được: {0}")]
    Parse(String),
    #[error("từ điển sai ở mục {id}: {why}")]
    Invalid { id: String, why: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strength {
    Required,
    Preferred,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AxisFilter {
    pub axis: String,
    pub range: Option<(f32, f32)>,
    pub equals: Option<String>,
    pub strength: Strength,
    pub weight: f32,
    pub term_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchedTerm {
    pub id: String,
    pub text: String,
    pub group: String,
    pub subjectivity: String,
    pub culture: String,
    pub rationale_vi: String,
    pub rationale_en: String,
    pub explanation_banned: bool,
}

/// Hai bộ lọc cùng trục không giao nhau: app không tự chọn. `range_*` là `None` với trục dạng lớp.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub axis: String,
    pub term_a: String,
    pub term_b: String,
    pub range_a: Option<(f32, f32)>,
    pub range_b: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HardConstraint {
    /// `vi_status in [full, precomposed_only]`
    ViSupported,
    /// `missing = 0`
    ViComplete,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parsed {
    pub dictionary_version: String,
    pub matched: Vec<MatchedTerm>,
    pub filters: Vec<AxisFilter>,
    pub hard: Vec<HardConstraint>,
    pub conflicts: Vec<Conflict>,
    /// (id từ khoá, lý do không ánh xạ được)
    pub unmappable: Vec<(String, String)>,
    /// (id từ khoá, phần còn thiếu)
    pub partial: Vec<(String, String)>,
    pub unknown_words: Vec<String>,
}

/// Một font nhìn từ bộ lọc. `None` → bỏ qua filter đó (README §4.4).
pub trait FaceView {
    fn percentile(&self, axis: &str) -> Option<f32>;
    fn class(&self, axis: &str) -> Option<String>;
    fn vi_missing(&self) -> usize;
    fn supports_vi(&self) -> bool;
}

#[derive(Deserialize)]
struct RawDict {
    dictionary_version: String,
    axes: BTreeMap<String, Value>,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawText {
    vi: String,
    en: String,
}

#[derive(Deserialize)]
struct RawTerms {
    vi: Vec<String>,
    en: Vec<String>,
}

#[derive(Deserialize)]
struct RawFilter {
    axis: String,
    range: Option<[f32; 2]>,
    equals: Option<Value>,
    strength: Strength,
    weight: Option<f32>,
}

#[derive(Deserialize)]
struct RawEntry {
    id: String,
    terms: RawTerms,
    group: String,
    filters: Vec<RawFilter>,
    #[serde(default)]
    hard_constraints: Vec<String>,
    rationale: RawText,
    subjectivity: String,
    culture: String,
    #[serde(default)]
    unmappable: Option<String>,
    #[serde(default)]
    unmappable_partial: Option<String>,
    #[serde(default)]
    explanation_banned: bool,
}

struct Entry {
    meta: MatchedTerm,
    filters: Vec<AxisFilter>,
    hard: Vec<HardConstraint>,
    unmappable: Option<String>,
    partial: Option<String>,
}

#[derive(Clone)]
pub struct Dictionary {
    version: String,
    entries: Vec<std::sync::Arc<Entry>>,
    /// (các từ đã gập dấu, chỉ số mục)
    phrases: Vec<(Vec<String>, usize)>,
    max_words: usize,
}

fn bad(id: &str, why: impl Into<String>) -> DescribeError {
    DescribeError::Invalid { id: id.into(), why: why.into() }
}

impl Dictionary {
    pub fn bundled() -> Dictionary {
        match Self::from_json(BUNDLED) {
            Ok(d) => d,
            Err(e) => panic!("descriptor-dictionary.json hỏng: {e}"),
        }
    }

    pub fn from_json(s: &str) -> Result<Dictionary, DescribeError> {
        let raw: RawDict = serde_json::from_str(s).map_err(|e| DescribeError::Parse(e.to_string()))?;
        let mut entries = Vec::new();
        let mut phrases = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (idx, e) in raw.entries.into_iter().enumerate() {
            if !seen.insert(e.id.clone()) {
                return Err(bad(&e.id, "id trùng"));
            }
            let mut filters = Vec::new();
            for f in &e.filters {
                if !raw.axes.contains_key(&f.axis) {
                    return Err(bad(&e.id, format!("trục lạ {}", f.axis)));
                }
                let range = match f.range {
                    Some([lo, hi]) => {
                        if !(0.0 <= lo && lo < hi && hi <= 1.0) {
                            return Err(bad(&e.id, format!("range {lo}..{hi} ngoài 0 ≤ lo < hi ≤ 1")));
                        }
                        Some((lo, hi))
                    }
                    None => None,
                };
                let equals = match &f.equals {
                    None => None,
                    Some(Value::String(s)) => Some(s.clone()),
                    Some(Value::Number(n)) => Some(n.to_string()),
                    Some(_) => return Err(bad(&e.id, "equals phải là chuỗi hoặc số")),
                };
                if range.is_none() && equals.is_none() {
                    return Err(bad(&e.id, "filter không có range lẫn equals"));
                }
                if f.strength == Strength::Preferred && f.weight.is_none() {
                    return Err(bad(&e.id, "preferred thiếu weight"));
                }
                filters.push(AxisFilter {
                    axis: f.axis.clone(),
                    range,
                    equals,
                    strength: f.strength,
                    weight: f.weight.unwrap_or(1.0),
                    term_id: e.id.clone(),
                });
            }
            let mut hard = Vec::new();
            // một chuỗi có thể ghép cả hai ràng buộc ("… và missing = 0", mục menu_body)
            for h in &e.hard_constraints {
                let (sup, comp) = (h.contains("vi_status in [full, precomposed_only]"), h.contains("missing = 0"));
                if !sup && !comp {
                    return Err(bad(&e.id, format!("ràng buộc lạ: {h}")));
                }
                for (on, c) in [(sup, HardConstraint::ViSupported), (comp, HardConstraint::ViComplete)] {
                    if on && !hard.contains(&c) {
                        hard.push(c);
                    }
                }
            }
            for t in e.terms.vi.iter().chain(&e.terms.en) {
                let words: Vec<String> = vi_fold(t).split(' ').filter(|w| !w.is_empty()).map(String::from).collect();
                if !words.is_empty() {
                    phrases.push((words, idx));
                }
            }
            entries.push(std::sync::Arc::new(Entry {
                meta: MatchedTerm {
                    id: e.id,
                    text: String::new(),
                    group: e.group,
                    subjectivity: e.subjectivity,
                    culture: e.culture,
                    rationale_vi: e.rationale.vi,
                    rationale_en: e.rationale.en,
                    explanation_banned: e.explanation_banned,
                },
                filters,
                hard,
                unmappable: e.unmappable,
                partial: e.unmappable_partial,
            }));
        }
        let max_words = phrases.iter().map(|p| p.0.len()).max().unwrap_or(1);
        Ok(Dictionary { version: raw.dictionary_version, entries, phrases, max_words })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Có mục id này không.
    pub fn has_id(&self, id: &str) -> bool {
        self.entries.iter().any(|e| e.meta.id == id)
    }

    /// Như `parse` nhưng chọn mục theo id (không qua văn bản): dùng cho Brief starter, nơi ánh xạ chip → term
    /// là bảng cố định. `strengths` ghi đè độ chặt của MỌI bộ lọc của term đó (vd thanh trượt kéo xa → Required).
    /// Id lạ → vào `unknown_words`. Thứ tự `ids` quyết định thứ tự `matched`/`filters`; id trùng chỉ tính một lần.
    pub fn parse_ids(&self, ids: &[(&str, Option<Strength>)]) -> Parsed {
        let mut out = Parsed { dictionary_version: self.version.clone(), ..Default::default() };
        let mut all: Vec<AxisFilter> = Vec::new();
        let mut seen: Vec<&str> = Vec::new();
        for (id, strength) in ids {
            if seen.contains(id) {
                continue;
            }
            seen.push(id);
            let Some(en) = self.entries.iter().find(|e| e.meta.id == *id) else {
                out.unknown_words.push((*id).to_string());
                continue;
            };
            let mut m = en.meta.clone();
            m.text = (*id).to_string();
            out.matched.push(m);
            all.extend(en.filters.iter().cloned().map(|mut f| {
                if let Some(st) = strength {
                    f.strength = *st;
                }
                f
            }));
            for h in &en.hard {
                if !out.hard.contains(h) {
                    out.hard.push(*h);
                }
            }
            if let Some(r) = &en.unmappable {
                out.unmappable.push((en.meta.id.clone(), r.clone()));
            }
            if let Some(r) = &en.partial {
                out.partial.push((en.meta.id.clone(), r.clone()));
            }
        }
        merge(all, &mut out);
        out
    }

    pub fn parse(&self, text: &str) -> Parsed {
        let mut out = Parsed { dictionary_version: self.version.clone(), ..Default::default() };
        // từng token gập riêng để giữ tương ứng với chữ gốc
        let toks: Vec<(String, String)> = text
            .split_whitespace()
            .map(|t| (t.to_string(), vi_fold(t)))
            .filter(|(_, f)| !f.is_empty())
            .collect();
        let index: HashMap<&[String], usize> = self.phrases.iter().map(|(w, i)| (w.as_slice(), *i)).collect();
        let folded: Vec<String> = toks.iter().map(|t| t.1.clone()).collect();
        let mut i = 0;
        let mut hits: Vec<usize> = Vec::new();
        while i < toks.len() {
            let longest = (1..=self.max_words.min(toks.len() - i))
                .rev()
                .find_map(|n| index.get(&folded[i..i + n]).map(|&e| (n, e)));
            match longest {
                Some((n, e)) => {
                    let mut m = self.entries[e].meta.clone();
                    m.text = toks[i..i + n].iter().map(|t| t.0.as_str()).collect::<Vec<_>>().join(" ");
                    if !hits.contains(&e) {
                        hits.push(e);
                        out.matched.push(m);
                    }
                    i += n;
                }
                None => {
                    out.unknown_words.push(toks[i].0.clone());
                    i += 1;
                }
            }
        }
        let mut all: Vec<AxisFilter> = Vec::new();
        for &e in &hits {
            let en = &self.entries[e];
            all.extend(en.filters.iter().cloned());
            for h in &en.hard {
                if !out.hard.contains(h) {
                    out.hard.push(*h);
                }
            }
            if let Some(r) = &en.unmappable {
                out.unmappable.push((en.meta.id.clone(), r.clone()));
            }
            if let Some(r) = &en.partial {
                out.partial.push((en.meta.id.clone(), r.clone()));
            }
        }
        merge(all, &mut out);
        out
    }
}

/// Gộp bộ lọc cùng trục: giao khoảng, `Required` thắng, `weight` lấy max; rỗng → mâu thuẫn, bỏ cả trục.
fn merge(all: Vec<AxisFilter>, out: &mut Parsed) {
    let mut order: Vec<String> = Vec::new();
    let mut by_axis: HashMap<String, Vec<AxisFilter>> = HashMap::new();
    for f in all {
        if !by_axis.contains_key(&f.axis) {
            order.push(f.axis.clone());
        }
        by_axis.entry(f.axis.clone()).or_default().push(f);
    }
    for axis in order {
        let list = by_axis.remove(&axis).unwrap_or_default();
        let mut it = list.into_iter();
        let Some(mut cur) = it.next() else { continue };
        let mut conflicted = false;
        for next in it {
            let merged = match (cur.range, next.range, &cur.equals, &next.equals) {
                (Some((a0, a1)), Some((b0, b1)), _, _) => {
                    let (lo, hi) = (a0.max(b0), a1.min(b1));
                    (lo < hi).then_some((Some((lo, hi)), None))
                }
                (None, None, Some(x), Some(y)) => (x == y).then(|| (None, Some(x.clone()))),
                _ => None,
            };
            match merged {
                Some((range, equals)) => {
                    cur.range = range;
                    cur.equals = equals;
                    if next.strength == Strength::Required {
                        cur.strength = Strength::Required;
                    }
                    cur.weight = cur.weight.max(next.weight);
                }
                None => {
                    out.conflicts.push(Conflict {
                        axis: axis.clone(),
                        term_a: cur.term_id.clone(),
                        term_b: next.term_id.clone(),
                        range_a: cur.range,
                        range_b: next.range,
                    });
                    conflicted = true;
                    break;
                }
            }
        }
        if !conflicted {
            out.filters.push(cur);
        }
    }
}

/// `None` = loại (trượt ràng buộc cứng hoặc `Required`). `Some(điểm)` chỉ để xếp thứ tự hiển thị:
/// `Preferred` thoả thì `+weight`, ngoài khoảng thì `−weight × khoảng cách`; trục thiếu giá trị thì bỏ qua.
pub fn evaluate(filters: &[AxisFilter], hard: &[HardConstraint], v: &dyn FaceView) -> Option<f32> {
    for h in hard {
        let ok = match h {
            HardConstraint::ViSupported => v.supports_vi(),
            HardConstraint::ViComplete => v.vi_missing() == 0,
        };
        if !ok {
            return None;
        }
    }
    let mut score = 0.0;
    for f in filters {
        let (ok, dist) = if let Some((lo, hi)) = f.range {
            let Some(p) = v.percentile(&f.axis) else { continue };
            let d = (lo - p).max(p - hi).max(0.0);
            (d == 0.0, d)
        } else if let Some(eq) = &f.equals {
            let Some(c) = v.class(&f.axis) else { continue };
            (&c == eq, 1.0)
        } else {
            continue;
        };
        match (ok, f.strength) {
            (true, _) => score += f.weight,
            (false, Strength::Required) => return None,
            (false, Strength::Preferred) => score -= f.weight * dist,
        }
    }
    Some(score)
}
