mod common;
use common::*;
use fontatlas_eval::{
    Corpus, Expected, LabelSource, Outcome, PairRow, SimilarSet, pairing_report, similar_report,
};
use fontatlas_index::Role;

fn row(id: &str, a: &str, b: &str, exp: Expected, split: &str) -> PairRow {
    PairRow {
        id: id.into(),
        a: (a.into(), "Regular".into()),
        role_a: Role::Heading,
        b: (b.into(), "Regular".into()),
        role_b: Role::Body,
        expected: exp,
        owner_verdict: None,
        split: split.into(),
    }
}

#[test]
fn owner_verdict_rows_counted_only_when_filled() {
    let c = Corpus::load(&[core_fixture_dir()], false);
    let ix = index_with_test_reference(&c);
    let mut a = row(
        "P1",
        "Playfair Display",
        "Open Sans",
        Expected::Good,
        "test",
    );
    a.owner_verdict = Some(Expected::Clash);
    let b = row("P2", "Playfair Display", "Roboto", Expected::Good, "test"); // owner trống
    let n = pairing_report(&ix, &c, &[a, b], "test", LabelSource::Owner);
    assert_eq!((n.good.n, n.clash.n), (0, 1));
}

#[test]
fn blocked_is_not_flagged() {
    let c = Corpus::load(&[core_fixture_dir()], false);
    let ix = index_with_test_reference(&c);
    let r = row(
        "V1",
        "Libre Baskerville",
        "Roboto",
        Expected::Bad,
        "test-vi",
    );
    let n = pairing_report(&ix, &c, &[r], "test-vi", LabelSource::Expected);
    assert_eq!(n.bad.blocked, 1);
    assert_eq!(n.bad.flagged, 0); // H1 là dòng riêng, không tính là "có nhãn va chạm"
}

#[test]
fn unresolvable_row_is_no_data_not_error() {
    let c = Corpus::load(&[core_fixture_dir()], false);
    let ix = index_with_test_reference(&c);
    let n = pairing_report(
        &ix,
        &c,
        &[row("P1", "Futura", "Souvenir", Expected::Good, "test")],
        "test",
        LabelSource::Expected,
    );
    assert_eq!((n.no_data, n.good.n), (1, 0));
}

#[test]
fn only_requested_split_is_counted() {
    let c = Corpus::load(&[core_fixture_dir()], false);
    let ix = index_with_test_reference(&c);
    let rows = [
        row("P1", "Roboto", "Open Sans", Expected::Good, "tune"),
        row("P2", "Playfair Display", "Roboto", Expected::Good, "tune"),
        row(
            "P3",
            "Playfair Display",
            "Open Sans",
            Expected::Good,
            "test",
        ),
    ];
    assert_eq!(
        pairing_report(&ix, &c, &rows, "test", LabelSource::Expected)
            .good
            .n,
        1
    );
}

#[test]
fn must_not_violation_beats_hit() {
    let c = Corpus::load(&[core_fixture_dir()], false);
    let ix = index_with_test_reference(&c);
    // k = số profile khác → mọi font khác đều nằm trong top-k (thư mục fixture có cả woff/woff2 và VF)
    let set = SimilarSet {
        id: "S".into(),
        query: ("Roboto".into(), "Regular".into()),
        must_include: vec![("Open Sans".into(), "*".into())],
        must_not: vec![("Playfair Display".into(), "*".into())],
        k: c.profiles.len() - 1,
        group_type: "t".into(),
        split: "test".into(),
    };
    assert!(matches!(
        similar_report(&ix, &c, &[set]).rows[0].1,
        Outcome::MustNotViolated
    ));
}
