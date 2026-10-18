//! Chỉ số S4 (Similar) và S5 (Pairing: good / clash / bad riêng, H1 dòng riêng).
use std::collections::HashMap;

use fontatlas_index::{FaceKey, Gate, GroupWeights, Index};

use crate::corpus::Corpus;
use crate::csvio::{Expected, PairRow, SimilarSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Hit,
    Miss,
    MustNotViolated,
    NoData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimilarReport {
    pub rows: Vec<(String, Outcome)>,
    /// Cùng họ tự động: face cùng họ đầu tiên ở hạng 1 / trong top 5.
    pub same_family_r1: f32,
    pub same_family_r5: f32,
    pub queries: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LabelCounts {
    pub n: usize,
    /// Chỉ có nghĩa với good: Safe | Controlled.
    pub pass: usize,
    /// Có ≥ 1 ClashKind (không tính Blocked).
    pub flagged: usize,
    /// clash/bad lọt ra Safe | Controlled.
    pub leaked: usize,
    /// H1.
    pub blocked: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PairCounts {
    pub good: LabelCounts,
    pub clash: LabelCounts,
    pub bad: LabelCounts,
    pub no_data: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelSource {
    Expected,
    Owner,
}

/// "Family|*" khớp mọi face của family; còn lại khớp đúng face đã resolve.
/// Hàm khớp một face (theo key) với một mục "Family|Style" của CSV.
type Matcher<'a> = Box<dyn Fn(&FaceKey) -> bool + 'a>;

fn matcher<'a>(c: &'a Corpus, f: &(String, String)) -> Option<Matcher<'a>> {
    if f.1 == "*" {
        let keys: Vec<FaceKey> = c.family(&f.0).iter().map(|p| p.key.clone()).collect();
        if keys.is_empty() {
            return None;
        }
        Some(Box::new(move |k| keys.contains(k)))
    } else {
        let key = c.resolve(&f.0, &f.1)?.key.clone();
        Some(Box::new(move |k| *k == key))
    }
}

pub fn similar_report(ix: &Index, c: &Corpus, sets: &[SimilarSet]) -> SimilarReport {
    let w = GroupWeights::default();
    let rows = sets
        .iter()
        .map(|s| {
            let out = (|| {
                let q = c.resolve(&s.query.0, &s.query.1)?;
                let must: Vec<_> = s
                    .must_include
                    .iter()
                    .map(|f| matcher(c, f))
                    .collect::<Option<_>>()?;
                // must_not không có trong máy → không thể vi phạm.
                let must_not: Vec<_> = s.must_not.iter().filter_map(|f| matcher(c, f)).collect();
                let hits = ix.similar(&q.key, &w, s.k, &|_| true).ok()?;
                let keys: Vec<&FaceKey> = hits.iter().map(|h| &h.key).collect();
                if must_not.iter().any(|m| keys.iter().any(|k| m(k))) {
                    return Some(Outcome::MustNotViolated);
                }
                Some(if must.iter().all(|m| keys.iter().any(|k| m(k))) {
                    Outcome::Hit
                } else {
                    Outcome::Miss
                })
            })();
            (s.id.clone(), out.unwrap_or(Outcome::NoData))
        })
        .collect();

    // Cùng họ tự động.
    let mut fam_count: HashMap<String, usize> = HashMap::new();
    for p in &c.profiles {
        *fam_count.entry(p.family.to_lowercase()).or_default() += 1;
    }
    let (mut q, mut r1, mut r5) = (0usize, 0usize, 0usize);
    for p in &c.profiles {
        let fam = p.family.to_lowercase();
        if fam_count.get(&fam).copied().unwrap_or(0) < 2 {
            continue;
        }
        let Ok(hits) = ix.similar(&p.key, &w, 5, &|_| true) else {
            continue;
        };
        q += 1;
        let pos = hits.iter().position(|h| {
            ix.profile(&h.key)
                .is_some_and(|x| x.family.to_lowercase() == fam)
        });
        match pos {
            Some(0) => {
                r1 += 1;
                r5 += 1;
            }
            Some(_) => r5 += 1,
            None => {}
        }
    }
    let ratio = |x: usize| if q == 0 { 0.0 } else { x as f32 / q as f32 };
    SimilarReport {
        rows,
        same_family_r1: ratio(r1),
        same_family_r5: ratio(r5),
        queries: q,
    }
}

pub fn pairing_report(
    ix: &Index,
    c: &Corpus,
    rows: &[PairRow],
    split: &str,
    src: LabelSource,
) -> PairCounts {
    let project_vi = split == "test-vi";
    let mut n = PairCounts::default();
    for r in rows.iter().filter(|r| r.split == split) {
        let label = match src {
            LabelSource::Expected => Some(r.expected),
            LabelSource::Owner => r.owner_verdict,
        };
        let Some(label) = label.filter(|l| *l != Expected::Unknown) else {
            continue;
        };
        let x = (|| {
            let a = c.resolve(&r.a.0, &r.a.1)?;
            let b = c.resolve(&r.b.0, &r.b.1)?;
            ix.assess_pair(&a.key, r.role_a, &b.key, r.role_b, project_vi)
                .ok()
        })();
        let Some(x) = x else {
            n.no_data += 1;
            continue;
        };
        let lc = match label {
            Expected::Good => &mut n.good,
            Expected::Clash => &mut n.clash,
            Expected::Bad => &mut n.bad,
            Expected::Unknown => continue,
        };
        lc.n += 1;
        if x.gate == Gate::Blocked {
            lc.blocked += 1;
            continue;
        }
        if !x.clashes.is_empty() {
            lc.flagged += 1;
        }
        let passes = matches!(x.gate, Gate::Safe | Gate::Controlled);
        match label {
            Expected::Good if passes => lc.pass += 1,
            Expected::Clash | Expected::Bad if passes => lc.leaked += 1,
            _ => {}
        }
    }
    n
}

fn pct(a: usize, b: usize) -> String {
    if b == 0 {
        "—".into()
    } else {
        format!("{a}/{b} ({:.0}%)", 100.0 * a as f32 / b as f32)
    }
}

/// Bảng markdown: một khối Similar, rồi mỗi (split, nguồn nhãn) một khối Pairing.
pub fn markdown(sim: &SimilarReport, pairs: &[(String, LabelSource, PairCounts)]) -> String {
    let mut s = String::new();
    let count = |o: Outcome| sim.rows.iter().filter(|r| r.1 == o).count();
    let judged = sim.rows.len() - count(Outcome::NoData);
    s.push_str("### Similar (split test)\n\n| chỉ số | giá trị |\n|---|---|\n");
    s.push_str(&format!(
        "| nhóm đạt (Hit) | {} |\n",
        pct(count(Outcome::Hit), judged)
    ));
    s.push_str(&format!("| trượt (Miss) | {} |\n", count(Outcome::Miss)));
    s.push_str(&format!(
        "| vi phạm must_not | {} |\n",
        count(Outcome::MustNotViolated)
    ));
    s.push_str(&format!("| NO_DATA | {} |\n", count(Outcome::NoData)));
    s.push_str(&format!(
        "| cùng họ R@1 / R@5 ({} truy vấn) | {:.1}% / {:.1}% |\n\n",
        sim.queries,
        sim.same_family_r1 * 100.0,
        sim.same_family_r5 * 100.0
    ));
    s.push_str("| bad/clash/good chi tiết | |\n|---|---|\n");
    for (id, o) in &sim.rows {
        s.push_str(&format!("| {id} | {o:?} |\n"));
    }
    for (split, src, c) in pairs {
        if *src == LabelSource::Owner && c.good.n + c.clash.n + c.bad.n == 0 {
            continue;
        }
        s.push_str(&format!(
            "\n### Pairing — split `{split}`, nhãn {src:?}\n\n"
        ));
        s.push_str("| nhãn | n | pass (Safe/Controlled) | flagged (có va chạm) | leaked (lọt Safe/Controlled) | blocked (H1) |\n|---|---|---|---|---|---|\n");
        for (name, l) in [("good", c.good), ("clash", c.clash), ("bad", c.bad)] {
            let pass = if name == "good" {
                pct(l.pass, l.n)
            } else {
                "—".into()
            };
            let flagged = if name == "good" {
                l.flagged.to_string()
            } else {
                pct(l.flagged, l.n)
            };
            let leaked = if name == "good" {
                "—".into()
            } else {
                pct(l.leaked, l.n)
            };
            s.push_str(&format!(
                "| {name} | {} | {pass} | {flagged} | {leaked} | {} |\n",
                l.n, l.blocked
            ));
        }
        s.push_str(&format!("\nNO_DATA: {}\n", c.no_data));
    }
    s
}
