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

#[test]
fn s07_flagged_as_informed_by_pairing_tune() {
    let s = load_similar(&eval_dir().join("similar_sets.csv")).unwrap();
    let flagged: Vec<_> = s
        .iter()
        .filter(|x| !x.informed_by.is_empty())
        .map(|x| x.id.as_str())
        .collect();
    assert_eq!(flagged, ["S07"]);
}

#[test]
fn groups_linked_by_superfamily_table_are_flagged() {
    // Bảng superfamily dựng từ pairing tune: nhóm TEST có query và must_include KHÁC họ nhưng chung nhóm bảng → phải gắn informed_by.
    let table = include_str!("../../fontatlas-index/data/superfamilies-v1.csv");
    let group_of = |fam: &str| -> Vec<String> {
        table
            .lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with("group,"))
            .filter_map(|l| {
                let mut it = l.split(',');
                let (g, f) = (it.next()?, it.next()?);
                (f.trim().eq_ignore_ascii_case(fam)).then(|| g.trim().to_string())
            })
            .collect()
    };
    for s in load_similar(&eval_dir().join("similar_sets.csv")).unwrap() {
        let q = group_of(&s.query.0);
        let linked = s.split == "test"
            && s.must_include.iter().any(|(f, _)| {
                !f.eq_ignore_ascii_case(&s.query.0) && group_of(f).iter().any(|g| q.contains(g))
            });
        assert!(
            !linked || !s.informed_by.is_empty(),
            "{} liên kết qua bảng superfamily mà không gắn informed_by",
            s.id
        );
    }
}
