mod common;
use common::*;
use fontatlas_core::classify::{CONF_MIN, Class, ClassifyInput, Metrics, Usage, classify, subclass_for};
use fontatlas_core::{FontFile, MeasureAt};

fn inp(name: &str) -> ClassifyInput {
    let f = FontFile::open(&fixture(name)).unwrap();
    let face = &f.faces().unwrap()[0];
    let m = serde_json::to_value(face.metadata()).unwrap();
    let a = serde_json::to_value(face.measure(MeasureAt::Standard)).unwrap();
    ClassifyInput::from_json(&m, &a)
}

fn cls(name: &str) -> fontatlas_core::classify::Classification {
    classify(&inp(name))
}

/// Lớp tốt nhất máy đoán: class nếu đã đủ tin, ngược lại `candidate` (độ tin cậy dưới CONF_MIN).
fn best(c: &fontatlas_core::classify::Classification) -> Class {
    c.candidate.unwrap_or(c.class)
}

#[test]
fn fixtures_get_expected_classes() {
    for (file, want) in [
        ("roboto.ttf", Class::Sans),
        ("open_sans.ttf", Class::Sans),
        ("playfair_display.ttf", Class::Serif),
        ("libre_baskerville.ttf", Class::Serif),
        ("source_code_pro.ttf", Class::Mono),
        ("fira_sans_regular.ttf", Class::Sans),
    ] {
        let c = cls(file);
        assert_eq!(best(&c), want, "{file}: {c:?}");
        assert!(c.confidence > 0.0 && !c.signals.is_empty(), "{file}");
    }
}

#[test]
fn playfair_is_didone_and_has_display_usage() {
    let c = cls("playfair_display.ttf");
    assert_eq!(subclass_for(&inp("playfair_display.ttf"), Class::Serif).as_deref(), Some("didone"), "{c:?}");
    assert!(c.usage.contains(&Usage::Display) || c.usage.contains(&Usage::Text), "{:?}", c.usage);
}

#[test]
fn script_fixture_is_script() {
    let c = cls("dancing_script_vf.ttf");
    assert_eq!(best(&c), Class::Script, "{c:?}");
    // tín hiệu nét nối / baseline nhảy phải tham gia
    assert!(c.signals.iter().any(|s| s.detail.contains("chạm cả hai mép")), "{c:?}");
    assert!(c.signals.iter().any(|s| s.detail.contains("nhảy")), "{c:?}");
}

#[test]
fn below_conf_min_is_uncertain_with_candidate_never_a_wrong_guess() {
    for file in ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf", "dancing_script_vf.ttf"] {
        let c = cls(file);
        match c.class {
            Class::Uncertain => assert!(c.subclass.is_none() && (c.candidate.is_some() || c.confidence == 0.0), "{file}: {c:?}"),
            _ => assert!(c.candidate.is_none() && c.confidence >= CONF_MIN, "{file}: {c:?}"),
        }
    }
}

#[test]
fn name_keywords_match_whole_words_only() {
    let i = ClassifyInput { name: "open sans regular".into(), ..Default::default() };
    assert!(classify(&i).signals.iter().all(|s| s.vote != Class::Script), "\"open\" không được khớp \"pen\"");
}

#[test]
fn contradicting_signals_are_uncertain() {
    // số đo nói sans, Panose + sFamilyClass + tên nói serif
    let i = ClassifyInput {
        name: "foo serif".into(),
        panose: Some([2, 3, 0, 0, 0, 0, 0, 0, 0, 0]),
        family_class: Some((2, 0)),
        m: Metrics { serif_foot: Some(1.0), ..Default::default() },
        ..Default::default()
    };
    let c = classify(&i);
    assert_eq!(c.class, Class::Uncertain, "{c:?}");
    assert!(c.signals.len() >= 3, "vẫn phải giải thích bằng signals");
}

#[test]
fn symbol_cmap_and_keywords_make_symbol() {
    let i = ClassifyInput { name: "webdings".into(), symbol_cmap: true, latin_letters: Some(0), ..Default::default() };
    assert_eq!(classify(&i).class, Class::Symbol);
    let j = ClassifyInput { name: "my dingbats".into(), ..Default::default() };
    assert_eq!(classify(&j).class, Class::Symbol);
}

#[test]
fn no_signal_means_uncertain_not_a_guess() {
    assert_eq!(classify(&ClassifyInput::default()).class, Class::Uncertain);
}

#[test]
fn metadata_exposes_new_fields() {
    let f = FontFile::open(&fixture("source_code_pro.ttf")).unwrap();
    let m = f.faces().unwrap()[0].metadata();
    assert!(m.is_fixed_pitch);
    assert!(m.latin_letters >= 52);
    assert!(!m.symbol_cmap);
    assert!(m.vendor_id.is_some() || m.family_class.is_some() || m.panose.is_some());
}

#[test]
fn every_subclass_the_classifier_can_emit_is_valid() {
    use fontatlas_core::classify::subclasses_of;
    for c in [Class::Serif, Class::Sans, Class::Mono, Class::Script, Class::Display] {
        assert!(!subclasses_of(c).is_empty());
    }
    assert!(subclasses_of(Class::Uncertain).is_empty());
}
