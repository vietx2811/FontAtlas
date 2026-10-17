mod common;
use common::*;
use fontatlas_index::{
    Block, Gate, Lang, PairAxis, Relation, Side, all_templates, explain_pair, fmt_num,
    pair_axis_name,
};

#[test]
fn banned_words_absent() {
    for t in all_templates() {
        let l = t.to_lowercase();
        for w in [
            "elegant",
            "modern",
            "generic",
            "thanh lịch",
            "hiện đại",
            "chung chung",
        ] {
            assert!(!l.contains(w), "{w} trong: {t}");
        }
    }
}

#[test]
fn vi_uses_decimal_comma() {
    assert_eq!(fmt_num(0.53, Lang::Vi), "0,53");
    assert_eq!(fmt_num(0.53, Lang::En), "0.53");
}

#[test]
fn no_clash_sentence_for_unsure() {
    let mut x = all_harmony_assessment(); // 8 nhãn Harmony, gate Safe
    set_label(&mut x, PairAxis::Stress, Relation::Unsure);
    let v = explain_pair(
        &x,
        &synth_full("A", "Regular", 0.15),
        &synth_full("B", "Regular", 0.15),
        Lang::Vi,
    );
    assert!(
        v.iter()
            .all(|s| !s.contains("va chạm") && !s.contains("lệch")),
        "{v:?}"
    );
    assert_eq!(
        v.iter()
            .filter(|s| s.starts_with("Chưa đủ số đo tin cậy"))
            .count(),
        1
    );
    let e = explain_pair(
        &x,
        &synth_full("A", "Regular", 0.15),
        &synth_full("B", "Regular", 0.15),
        Lang::En,
    );
    assert!(e.iter().all(|s| !s.to_lowercase().contains("clash")));
}

#[test]
fn every_label_has_sentence_in_both_langs() {
    let (a, b) = (
        synth_full("A", "Regular", 0.15),
        synth_full("B", "Regular", 0.15),
    );
    for ax in PairAxis::ALL {
        for rel in [
            Relation::Harmony,
            Relation::Contrast,
            Relation::Clash,
            Relation::WeightHint,
            Relation::ContrastWithSerif,
        ] {
            if !valid_combo(ax, rel) {
                continue;
            } // Weight không có Clash; WeightHint chỉ cho Weight; ContrastWithSerif chỉ cho Contrast
            let mut x = blank_assessment();
            set_label_values(&mut x, ax, rel, 0.53, 0.51);
            for (l, n1, n2) in [(Lang::Vi, "0,53", "0,51"), (Lang::En, "0.53", "0.51")] {
                let v = explain_pair(&x, &a, &b, l);
                // câu của ĐÚNG nhãn này: chứa tên trục và cả hai số đo
                let name = pair_axis_name(ax, l);
                assert!(
                    v.iter()
                        .any(|s| s.contains(name) && s.contains(n1) && s.contains(n2)),
                    "{ax:?} {rel:?} {l:?}: {v:?}"
                );
            }
        }
    }
}

#[test]
fn numbers_are_raw_measurements() {
    let mut x = blank_assessment();
    set_label_values(&mut x, PairAxis::XHeight, Relation::Harmony, 0.53, 0.51);
    let v = explain_pair(
        &x,
        &synth_full("A", "Regular", 0.15),
        &synth_full("B", "Regular", 0.15),
        Lang::Vi,
    );
    assert!(
        v.iter().any(|s| s.contains("0,53") && s.contains("0,51")),
        "{v:?}"
    );
}

#[test]
fn blocked_first() {
    let mut x = all_harmony_assessment();
    x.gate = Gate::Blocked;
    x.block = Some(Block::MissingVietnamese {
        side: Side::A,
        missing: 68,
    });
    let v = explain_pair(
        &x,
        &synth_full("Libre Baskerville", "Regular", 0.15),
        &synth_full("B", "Regular", 0.15),
        Lang::Vi,
    );
    assert!(v[0].contains("thiếu 68 ký tự tiếng Việt"), "{v:?}");
}
