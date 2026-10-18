mod common;
use common::*;
use fontatlas_eval::{load_pairs, load_similar, test_only_families};

#[test]
fn loads_draft_csvs() {
    let (sp, pp) = (
        eval_dir().join("similar_sets.csv"),
        eval_dir().join("pairing_judgements.csv"),
    );
    let s = load_similar(&sp).unwrap();
    assert_eq!(s.len(), data_lines(&sp));
    assert!(s.iter().all(|x| !x.split.is_empty()));
    let p = load_pairs(&pp).unwrap();
    assert_eq!(p.len(), data_lines(&pp));
    for sp in ["tune", "test", "test-vi"] {
        assert!(p.iter().any(|r| r.split == sp), "{sp}");
    }
}

#[test]
fn superfamily_table_has_no_test_leak() {
    let p = load_pairs(&eval_dir().join("pairing_judgements.csv")).unwrap();
    let leak = test_only_families(&p);
    let table = include_str!("../../fontatlas-index/data/superfamilies-v1.csv");
    for line in table
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("group,"))
    {
        let fam = line.split(',').nth(1).unwrap();
        assert!(
            !leak.contains(fam),
            "bảng superfamily chứa family chỉ có trong tập kiểm: {fam}"
        );
    }
}
