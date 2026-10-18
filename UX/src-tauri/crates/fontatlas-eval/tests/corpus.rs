mod common;
use common::*;
use fontatlas_eval::Corpus;
use fontatlas_index::AxisId;

#[test]
fn corpus_expands_variable_weights() {
    let c = Corpus::load(&[core_fixture_dir()], true);
    let r = c.resolve("Public Sans", "Regular").unwrap();
    let b = c.resolve("Public Sans", "Bold").unwrap();
    assert_eq!(b.weight_class, Some(700));
    assert!(b.get(AxisId::StemNXh).value.unwrap() > r.get(AxisId::StemNXh).value.unwrap());
}
