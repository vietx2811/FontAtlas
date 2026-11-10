//! Taxonomy v2 (docs/team/research/2026-11-11-font-classification-taxonomy.md mục 4): bảng nhãn chuẩn
//! `docs/team/eval/taxonomy_v2_labels.csv`, ánh xạ classifier cũ → mới, và báo cáo ma trận nhầm lẫn.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::csvio::EvalError;

pub const CLASSES: [&str; 8] = [
    "serif",
    "slab",
    "sans",
    "script",
    "handwriting",
    "mono",
    "display",
    "blackletter",
];

/// Subclass hợp lệ của từng class (mục 4.2–4.8; scotch/fat_face là trait, không phải subclass).
pub fn subclasses(class: &str) -> &'static [&'static str] {
    match class {
        "serif" => &[
            "venetian_oldstyle",
            "garalde",
            "transitional",
            "didone",
            "glyphic",
            "wedge",
        ],
        "slab" => &["clarendon", "egyptian", "humanist_slab"],
        "sans" => &[
            "grotesque",
            "neo_grotesque",
            "geometric",
            "humanist",
            "flared",
            "rounded",
            "squarish",
        ],
        "script" => &[
            "formal_copperplate",
            "brush",
            "monoline_script",
            "signature",
            "upright_script",
            "calligraphic_unjoined",
        ],
        "handwriting" => &["casual_print", "marker", "childlike"],
        "mono" => &["sans_mono", "serif_mono", "coding", "typewriter"],
        "display" => &["stencil", "inline_outline", "shaded_3d", "pixel_bitmap"],
        _ => &[],
    }
}

/// Ô (class, subclass) chưa đủ 5 họ OFL: không có nguồn nhãn đáng tin, đừng dùng làm cổng (README eval giải thích từng ô).
pub const KNOWN_GAPS: [(&str, &str); 3] = [
    ("serif", "wedge"),
    ("mono", "serif_mono"),
    ("mono", "typewriter"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    GfTag,
    Hand,
}

/// Thứ tự tăng dần theo độ tin cậy; `Weak` không dùng làm cổng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    Weak,
    Visual,
    Gf,
}

impl Quality {
    pub fn gates(self) -> bool {
        self != Quality::Weak
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub id: String,
    pub family: String,
    pub class: String,
    pub subclass: Option<String>,
    pub secondary: Option<String>,
    pub license: String,
    pub gf_dir: String,
    pub source: Source,
    pub quality: Quality,
    pub boundary: Option<String>,
    pub gf_tags: Vec<String>,
}

/// Đáp án theo họ (gộp các dòng cùng họ).
#[derive(Debug, Clone, PartialEq)]
pub struct Truth {
    pub family: String,
    pub gf_dir: String,
    pub class: String,
    /// (subclass, độ tin cậy của nhãn đó).
    pub subs: Vec<(String, Quality)>,
    pub secondary: Option<String>,
    /// Tốt nhất trong các dòng của họ.
    pub quality: Quality,
    pub boundary: Option<String>,
}

/// Dự đoán đã quy về taxonomy v2: class có thể là một trong CLASSES, "symbol" hoặc "uncertain".
#[derive(Debug, Clone, PartialEq)]
pub struct Pred {
    pub class: String,
    pub subclass: Option<String>,
}

fn bad(path: &Path, msg: String) -> EvalError {
    EvalError::Csv {
        path: path.display().to_string(),
        msg,
    }
}

pub fn load_labels(path: &Path) -> Result<Vec<Label>, EvalError> {
    let mut rd = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_path(path)
        .map_err(|e| bad(path, e.to_string()))?;
    let headers = rd.headers().map_err(|e| bad(path, e.to_string()))?.clone();
    let mut out = Vec::new();
    for r in rd.records() {
        let r = r.map_err(|e| bad(path, e.to_string()))?;
        let get = |k: &str| -> Result<String, EvalError> {
            let i = headers
                .iter()
                .position(|h| h == k)
                .ok_or_else(|| bad(path, format!("thiếu cột {k}")))?;
            Ok(r.get(i).unwrap_or("").trim().to_string())
        };
        let opt = |s: String| (!s.is_empty()).then_some(s);
        let source = match get("source")?.as_str() {
            "GF tag" => Source::GfTag,
            "tay" => Source::Hand,
            o => return Err(bad(path, format!("source không hợp lệ: {o}"))),
        };
        let quality = match get("quality")?.as_str() {
            "gf" => Quality::Gf,
            "visual" => Quality::Visual,
            "weak" => Quality::Weak,
            o => return Err(bad(path, format!("quality không hợp lệ: {o}"))),
        };
        out.push(Label {
            id: get("id")?,
            family: get("family")?,
            class: get("class")?,
            subclass: opt(get("subclass")?),
            secondary: opt(get("secondary")?),
            license: get("license")?,
            gf_dir: get("gf_dir")?,
            source,
            quality,
            boundary: opt(get("boundary")?),
            gf_tags: get("gf_tags")?
                .split(';')
                .filter(|t| !t.is_empty())
                .map(String::from)
                .collect(),
        });
    }
    Ok(out)
}

/// Gộp theo họ; một họ phải giữ đúng một class.
pub fn truths(labels: &[Label]) -> Result<Vec<Truth>, EvalError> {
    let mut by: BTreeMap<&str, Truth> = BTreeMap::new();
    let mut order: Vec<&str> = Vec::new();
    for l in labels {
        match by.get_mut(l.family.as_str()) {
            None => {
                order.push(&l.family);
                by.insert(
                    &l.family,
                    Truth {
                        family: l.family.clone(),
                        gf_dir: l.gf_dir.clone(),
                        class: l.class.clone(),
                        subs: l.subclass.iter().map(|s| (s.clone(), l.quality)).collect(),
                        secondary: l.secondary.clone(),
                        quality: l.quality,
                        boundary: l.boundary.clone(),
                    },
                );
            }
            Some(t) => {
                if t.class != l.class {
                    return Err(EvalError::Csv {
                        path: "taxonomy_v2_labels.csv".into(),
                        msg: format!(
                            "{}: hai class khác nhau ({} / {})",
                            l.family, t.class, l.class
                        ),
                    });
                }
                if let Some(s) = &l.subclass {
                    t.subs.push((s.clone(), l.quality));
                }
                t.quality = t.quality.max(l.quality);
                t.secondary = t.secondary.take().or_else(|| l.secondary.clone());
                t.boundary = t.boundary.take().or_else(|| l.boundary.clone());
            }
        }
    }
    Ok(order
        .into_iter()
        .map(|f| by.remove(f).expect("đã chèn"))
        .collect())
}

/// Quy kết quả của classifier cũ (7 class, trait stencil/inline_outline/pixel) về taxonomy v2 để lấy baseline.
/// `class` là `Class::snake()`. Trait cấu trúc thắng class cũ vì taxonomy v2 xếp chúng vào Display.
pub fn map_old_classifier(class: &str, subclass: Option<&str>, traits: &[String]) -> Pred {
    let has = |t: &str| traits.iter().any(|x| x == t);
    let p = |c: &str, s: Option<&str>| Pred {
        class: c.into(),
        subclass: s.map(Into::into),
    };
    for (t, s) in [
        ("stencil", "stencil"),
        ("inline_outline", "inline_outline"),
        ("pixel", "pixel_bitmap"),
    ] {
        if has(t) {
            return p("display", Some(s));
        }
    }
    match (class, subclass) {
        ("serif", Some("slab")) => p("slab", None),
        ("serif", Some("humanist_oldstyle")) => p("serif", Some("venetian_oldstyle")),
        ("serif", s) => p("serif", s),
        ("sans", None) if has("rounded") => p("sans", Some("rounded")),
        ("sans", s) => p("sans", s),
        ("mono", s) => p("mono", s),
        ("script", Some("formal")) => p("script", Some("formal_copperplate")),
        ("script", Some("casual_brush")) => p("script", Some("brush")),
        ("script", Some("handwritten")) => p("handwriting", Some("casual_print")),
        ("script", s) => p("script", s),
        (c, _) => p(c, None),
    }
}

fn pct(x: usize, d: usize) -> f64 {
    if d == 0 {
        0.0
    } else {
        100.0 * x as f64 / d as f64
    }
}

/// Nhãn dự đoán để hiển thị trong cột "nhầm sang": cùng class → subclass; khác class → class/subclass.
fn confusion_label(t: &Truth, p: &Pred) -> String {
    match (&p.subclass, p.class == t.class) {
        (Some(s), true) => s.clone(),
        (None, true) => "—".into(),
        (Some(s), false) => format!("{}/{s}", p.class),
        (None, false) => p.class.clone(),
    }
}

/// Báo cáo markdown: ma trận class (chỉ họ quality gf/visual), bảng subclass, ma trận subclass từng class,
/// fixture ranh giới, và một dòng cho nhãn weak (không cổng).
pub fn report(rows: &[(Truth, Pred)]) -> String {
    let mut md = String::new();
    let gate: Vec<&(Truth, Pred)> = rows.iter().filter(|r| r.0.quality.gates()).collect();
    let cols: Vec<&str> = CLASSES
        .iter()
        .copied()
        .chain(["symbol", "uncertain"])
        .collect();

    let (mut ok, mut wrong, mut unc) = (0, 0, 0);
    let mut cm: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for (t, p) in &gate {
        *cm.entry((t.class.as_str(), p.class.as_str())).or_default() += 1;
        if p.class == "uncertain" {
            unc += 1;
        } else if p.class == t.class {
            ok += 1;
        } else {
            wrong += 1;
        }
    }
    md.push_str(&format!(
        "### Class — {} họ (nhãn gf/visual)\n\n| đúng | sai | Uncertain | đúng/tổng | đúng/đã gán |\n|---|---|---|---|---|\n| {ok} | {wrong} | {unc} | {:.1}% | {:.1}% |\n\nHàng = đáp án, cột = dự đoán.\n\n| đáp án \\ dự đoán | {} |\n|---|{}\n",
        gate.len(),
        pct(ok, gate.len()),
        pct(ok, ok + wrong),
        cols.join(" | "),
        "---|".repeat(cols.len())
    ));
    for c in CLASSES {
        let n: usize = cols
            .iter()
            .map(|p| cm.get(&(c, *p)).copied().unwrap_or(0))
            .sum();
        if n == 0 {
            continue;
        }
        let cells: Vec<String> = cols
            .iter()
            .map(|p| cm.get(&(c, *p)).copied().unwrap_or(0).to_string())
            .collect();
        md.push_str(&format!("| {c} | {} |\n", cells.join(" | ")));
    }

    // subclass: mỗi (họ, subclass gating) là một mục
    let mut sub: BTreeMap<(String, String), Vec<(&Truth, &Pred)>> = BTreeMap::new();
    for (t, p) in &gate {
        for (s, q) in &t.subs {
            if q.gates() {
                sub.entry((t.class.clone(), s.clone()))
                    .or_default()
                    .push((t, p));
            }
        }
    }
    md.push_str("\n### Subclass (nhãn gf/visual; mỗi họ tính theo từng subclass của nó)\n\n| class/subclass | số họ | đúng | nhầm sang |\n|---|---|---|---|\n");
    for ((c, s), v) in &sub {
        let right = v
            .iter()
            .filter(|(t, p)| p.class == t.class && p.subclass.as_deref() == Some(s))
            .count();
        let mut conf: BTreeMap<String, usize> = BTreeMap::new();
        for (t, p) in v {
            if !(p.class == t.class && p.subclass.as_deref() == Some(s)) {
                *conf.entry(confusion_label(t, p)).or_default() += 1;
            }
        }
        let mut conf: Vec<_> = conf.into_iter().collect();
        conf.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let txt: Vec<String> = conf.iter().map(|(k, n)| format!("{k}×{n}")).collect();
        md.push_str(&format!(
            "| {c}/{s} | {} | {right} | {} |\n",
            v.len(),
            txt.join(", ")
        ));
    }

    // ma trận subclass đầy đủ cho từng class có subclass
    for c in CLASSES {
        let rows_c: Vec<_> = sub.iter().filter(|((cc, _), _)| cc == c).collect();
        if rows_c.is_empty() {
            continue;
        }
        let mut labels: BTreeSet<String> = BTreeSet::new();
        let mut cell: BTreeMap<(String, String), usize> = BTreeMap::new();
        for ((_, s), v) in &rows_c {
            for (t, p) in v.iter() {
                let l = confusion_label(t, p);
                labels.insert(l.clone());
                *cell.entry((s.clone(), l)).or_default() += 1;
            }
        }
        let labels: Vec<String> = labels.into_iter().collect();
        md.push_str(&format!(
            "\n#### {c}: đáp án subclass × dự đoán\n\n| đáp án \\ dự đoán | {} |\n|---|{}\n",
            labels.join(" | "),
            "---|".repeat(labels.len())
        ));
        for ((_, s), _) in &rows_c {
            let cells: Vec<String> = labels
                .iter()
                .map(|l| {
                    cell.get(&(s.clone(), l.clone()))
                        .copied()
                        .unwrap_or(0)
                        .to_string()
                })
                .collect();
            md.push_str(&format!("| {s} | {} |\n", cells.join(" | ")));
        }
    }

    // fixture ranh giới: tính cả họ weak
    let bd: Vec<&(Truth, Pred)> = rows.iter().filter(|r| r.0.boundary.is_some()).collect();
    if !bd.is_empty() {
        md.push_str("\n### Fixture ranh giới (class đúng?)\n\n| nhóm | họ | class đáp án | class dự đoán | đúng |\n|---|---|---|---|---|\n");
        let mut bd = bd;
        bd.sort_by(|a, b| {
            (a.0.boundary.as_deref(), &a.0.family).cmp(&(b.0.boundary.as_deref(), &b.0.family))
        });
        for (t, p) in bd {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                t.boundary.as_deref().unwrap_or(""),
                t.family,
                t.class,
                p.class,
                if p.class == t.class { "✓" } else { "✗" }
            ));
        }
    }

    let weak: Vec<&(Truth, Pred)> = rows.iter().filter(|r| !r.0.quality.gates()).collect();
    if !weak.is_empty() {
        let cls = weak.iter().filter(|(t, p)| p.class == t.class).count();
        let subs: Vec<_> = weak.iter().filter(|(t, _)| !t.subs.is_empty()).collect();
        let sok = subs
            .iter()
            .filter(|(t, p)| {
                p.class == t.class
                    && t.subs
                        .iter()
                        .any(|(s, _)| Some(s.as_str()) == p.subclass.as_deref())
            })
            .count();
        md.push_str(&format!(
            "\n### Nhãn weak (không cổng, chỉ tham khảo)\n\n| nhãn | số họ | class đúng | subclass đúng |\n|---|---|---|---|\n| weak | {} | {cls} | {sok}/{} |\n",
            weak.len(),
            subs.len()
        ));
    }
    md
}
