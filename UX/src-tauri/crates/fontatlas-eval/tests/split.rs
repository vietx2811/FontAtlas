mod common;
use std::collections::{BTreeMap, BTreeSet};

use common::*;
use fontatlas_eval::{SIMILAR_SPLIT_SEED, SimilarSet, load_similar, split_similar};

fn set(id: &str, ty: &str, query: &str, include: &[&str]) -> SimilarSet {
    let f = |s: &str| (s.to_string(), "Regular".to_string());
    SimilarSet {
        id: id.into(),
        query: f(query),
        must_include: include.iter().map(|s| f(s)).collect(),
        must_not: vec![f("Shared Distractor")],
        k: 10,
        group_type: ty.into(),
        split: String::new(),
    }
}

fn csv_sets() -> Vec<SimilarSet> {
    load_similar(&eval_dir().join("similar_sets.csv")).unwrap()
}

fn families(s: &SimilarSet) -> BTreeSet<String> {
    std::iter::once(&s.query)
        .chain(&s.must_include)
        .map(|(f, _)| f.to_lowercase())
        .collect()
}

#[test]
fn split_is_deterministic_and_covers_every_group() {
    let sets = csv_sets();
    let a = split_similar(&sets, SIMILAR_SPLIT_SEED);
    assert_eq!(a, split_similar(&sets, SIMILAR_SPLIT_SEED));
    assert_eq!(a.len(), sets.len());
    assert!(a.values().all(|s| *s == "tune" || *s == "test"));
}

#[test]
fn groups_sharing_a_family_land_in_the_same_split() {
    let sets = csv_sets();
    let sp = split_similar(&sets, SIMILAR_SPLIT_SEED);
    for a in &sets {
        for b in &sets {
            if !families(a).is_disjoint(&families(b)) {
                assert_eq!(sp[&a.id], sp[&b.id], "{} và {} chung họ", a.id, b.id);
            }
        }
    }
}

#[test]
fn shared_must_not_does_not_link_groups() {
    // Sáu nhóm chỉ chung "Shared Distractor" ở must_not: vẫn chia độc lập, tune = round(6/3).
    let sets: Vec<_> = (0..6)
        .map(|i| set(&format!("X{i}"), "t", &format!("Q{i}"), &[&format!("I{i}")]))
        .collect();
    let sp = split_similar(&sets, SIMILAR_SPLIT_SEED);
    assert_eq!(sp.values().filter(|s| **s == "tune").count(), 2);
}

#[test]
fn every_group_type_keeps_a_test_group_and_tune_is_about_a_third() {
    let sets = csv_sets();
    let sp = split_similar(&sets, SIMILAR_SPLIT_SEED);
    let mut by_type: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for s in &sets {
        let e = by_type.entry(&s.group_type).or_default();
        if sp[&s.id] == "tune" {
            e.0 += 1
        } else {
            e.1 += 1
        }
    }
    for (ty, (tune, test)) in &by_type {
        assert!(*test >= 1, "{ty} không còn nhóm test");
        assert!(
            *tune <= (tune + test).div_ceil(2),
            "{ty}: tune {tune} quá nhiều"
        );
    }
    let tune = sp.values().filter(|s| **s == "tune").count();
    assert!((6..=11).contains(&tune), "tune = {tune}/{}", sets.len());
}

#[test]
fn single_group_stratum_stays_test() {
    let sp = split_similar(&[set("A", "only", "Fam", &["Fam2"])], SIMILAR_SPLIT_SEED);
    assert_eq!(sp["A"], "test");
}

#[test]
fn csv_split_column_matches_rule() {
    let sets = csv_sets();
    let sp = split_similar(&sets, SIMILAR_SPLIT_SEED);
    for s in &sets {
        assert_eq!(
            s.split, sp[&s.id],
            "{}: CSV ghi {}, quy tắc ra {}",
            s.id, s.split, sp[&s.id]
        );
    }
}
