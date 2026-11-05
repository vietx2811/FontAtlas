//! Đọc hai bộ CSV đánh giá (docs/team/eval): nhóm Similar và phán xét cặp.
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use fontatlas_index::Role;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("csv {path}: {msg}")]
    Csv { path: String, msg: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimilarSet {
    pub id: String,
    pub query: (String, String),
    pub must_include: Vec<(String, String)>,
    pub must_not: Vec<(String, String)>,
    pub k: usize,
    pub group_type: String,
    pub split: String,
    /// Nhóm đã bị thông tin từ phần tune khác (vd. bảng superfamily dựng từ pairing tune) chạm tới; rỗng = sạch.
    pub informed_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Expected {
    Good,
    Clash,
    Bad,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairRow {
    pub id: String,
    pub a: (String, String),
    pub role_a: Role,
    pub b: (String, String),
    pub role_b: Role,
    pub expected: Expected,
    pub owner_verdict: Option<Expected>,
    pub split: String,
}

type Rec = HashMap<String, String>;

fn read(path: &Path) -> Result<Vec<Rec>, EvalError> {
    let err = |m: String| EvalError::Csv {
        path: path.display().to_string(),
        msg: m,
    };
    let mut rd = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .flexible(false)
        .from_path(path)
        .map_err(|e| err(e.to_string()))?;
    let headers = rd.headers().map_err(|e| err(e.to_string()))?.clone();
    let mut out = Vec::new();
    for r in rd.records() {
        let r = r.map_err(|e| err(e.to_string()))?;
        out.push(
            headers
                .iter()
                .zip(r.iter())
                .map(|(h, v)| (h.to_string(), v.trim().to_string()))
                .collect(),
        );
    }
    Ok(out)
}

fn field<'a>(r: &'a Rec, k: &str, path: &Path) -> Result<&'a str, EvalError> {
    r.get(k).map(String::as_str).ok_or_else(|| EvalError::Csv {
        path: path.display().to_string(),
        msg: format!("thiếu cột {k}"),
    })
}

/// "Family|Style" (Style mặc định Regular).
fn face(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut it = s.splitn(2, '|');
    let fam = it.next()?.trim().to_string();
    let style = it
        .next()
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .unwrap_or("Regular");
    Some((fam, style.to_string()))
}

fn faces(s: &str) -> Vec<(String, String)> {
    s.split(';').filter_map(face).collect()
}

pub fn load_similar(path: &Path) -> Result<Vec<SimilarSet>, EvalError> {
    read(path)?
        .iter()
        .map(|r| {
            let bad = |m: &str| EvalError::Csv {
                path: path.display().to_string(),
                msg: m.to_string(),
            };
            Ok(SimilarSet {
                id: field(r, "id", path)?.to_string(),
                query: face(field(r, "query", path)?).ok_or_else(|| bad("query rỗng"))?,
                must_include: faces(field(r, "must_include", path)?),
                must_not: faces(field(r, "must_not", path)?),
                k: field(r, "k", path)?
                    .parse()
                    .map_err(|_| bad("k không phải số"))?,
                group_type: field(r, "group_type", path)?.to_string(),
                split: field(r, "split", path)?.to_string(),
                informed_by: r.get("informed_by").cloned().unwrap_or_default(),
            })
        })
        .collect()
}

fn role(s: &str) -> Option<Role> {
    match s.to_ascii_lowercase().as_str() {
        "heading" => Some(Role::Heading),
        "body" => Some(Role::Body),
        "code" => Some(Role::Code),
        _ => None,
    }
}

fn expected(s: &str) -> Option<Expected> {
    match s.to_ascii_lowercase().as_str() {
        "good" => Some(Expected::Good),
        "clash" => Some(Expected::Clash),
        "bad" => Some(Expected::Bad),
        "?" => Some(Expected::Unknown),
        _ => None,
    }
}

pub fn load_pairs(path: &Path) -> Result<Vec<PairRow>, EvalError> {
    read(path)?
        .iter()
        .map(|r| {
            let bad = |m: String| EvalError::Csv {
                path: path.display().to_string(),
                msg: m,
            };
            let id = field(r, "id", path)?.to_string();
            let style = |k: &str| -> Result<String, EvalError> {
                let s = field(r, k, path)?;
                Ok(if s.is_empty() {
                    "Regular".into()
                } else {
                    s.into()
                })
            };
            Ok(PairRow {
                a: (field(r, "family_a", path)?.into(), style("style_a")?),
                role_a: role(field(r, "role_a", path)?)
                    .ok_or_else(|| bad(format!("{id}: role_a")))?,
                b: (field(r, "family_b", path)?.into(), style("style_b")?),
                role_b: role(field(r, "role_b", path)?)
                    .ok_or_else(|| bad(format!("{id}: role_b")))?,
                expected: expected(field(r, "expected", path)?)
                    .ok_or_else(|| bad(format!("{id}: expected")))?,
                // rỗng hoặc "skip" → không có phán xét của chủ app.
                owner_verdict: expected(field(r, "owner_verdict", path)?)
                    .filter(|e| *e != Expected::Unknown),
                split: field(r, "split", path)?.to_string(),
                id,
            })
        })
        .collect()
}

/// Family xuất hiện trong test/test-vi mà không có trong tune (để chặn rò rỉ).
pub fn test_only_families(rows: &[PairRow]) -> BTreeSet<String> {
    let fams = |pred: &dyn Fn(&str) -> bool| -> BTreeSet<String> {
        rows.iter()
            .filter(|r| pred(&r.split))
            .flat_map(|r| [r.a.0.clone(), r.b.0.clone()])
            .collect()
    };
    let tune = fams(&|s| s == "tune");
    fams(&|s| s == "test" || s == "test-vi")
        .difference(&tune)
        .cloned()
        .collect()
}
