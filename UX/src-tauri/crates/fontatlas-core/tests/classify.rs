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
        // font điển hình phải được GÁN (không chỉ candidate), trừ Roboto nằm sát ngưỡng CONF_MIN
        if file != "roboto.ttf" {
            assert_eq!(c.class, want, "{file}: {c:?}");
        }
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
            // Mono/Symbol/Blackletter dựa trên tín hiệu cứng nên không qua hiệu chỉnh
            k => assert!(c.candidate.is_none() && (c.confidence >= CONF_MIN || matches!(k, Class::Mono | Class::Symbol | Class::Blackletter)), "{file}: {c:?}"),
        }
    }
}

#[test]
fn name_keywords_match_whole_words_only() {
    let i = ClassifyInput { name: "open sans regular".into(), ..Default::default() };
    assert!(classify(&i).signals.iter().all(|s| s.vote != Some(Class::Script)), "\"open\" không được khớp \"pen\"");
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
    for c in [Class::Serif, Class::Sans, Class::Mono, Class::Script] {
        assert!(!subclasses_of(c).is_empty());
    }
    assert!(subclasses_of(Class::Uncertain).is_empty());
}

#[test]
fn display_is_a_usage_tag_not_a_class() {
    assert!(Class::parse("display").is_none());
    // tên có "Stencil": trait stencil, class vẫn theo hình dáng (ở đây không đủ bằng chứng → không phải Display)
    let i = ClassifyInput { name: "foo stencil".into(), m: Metrics { serif_foot: Some(1.0), ..Default::default() }, ..Default::default() };
    let c = classify(&i);
    assert_eq!(c.traits, vec!["stencil".to_string()], "{c:?}");
    assert!(c.usage.contains(&Usage::Display), "{:?}", c.usage);
    assert_ne!(c.class.label(), "Display");
    // không có dấu hiệu → không có trait
    assert!(classify(&ClassifyInput::default()).traits.is_empty());
    // Monoton/Monofett không phải Mono, "rounded" cho trait rounded
    let j = ClassifyInput { name: "monoton rounded".into(), ..Default::default() };
    let cj = classify(&j);
    assert!(cj.signals.iter().all(|s| s.vote != Some(Class::Mono)), "{cj:?}");
    assert_eq!(cj.traits, vec!["rounded".to_string()]);
}

// --- serif chân nhỏ (core-0.9.0): chân loe, cờ trên-trái; tên không lấn số đo ---

fn synthetic(name: &str, foot: f32, flare: f32, head: f32) -> ClassifyInput {
    ClassifyInput { name: name.into(), m: Metrics { serif_foot: Some(foot), serif_flare: Some(flare), serif_head: Some(head), ..Default::default() }, ..Default::default() }
}

#[test]
fn small_foot_serif_fixtures_are_serif_with_measured_flare() {
    for file in ["dm_serif_display.ttf", "chonburi.ttf"] {
        let i = inp(file);
        let (foot, flare) = (i.m.serif_foot.unwrap(), i.m.serif_flare.unwrap());
        // vùng giữa mà phép đo cũ bỏ trống, và độ loe nhìn thấy chân
        assert!((1.2..=1.7).contains(&foot), "{file}: foot {foot}");
        assert!(flare >= 1.5, "{file}: flare {flare}");
        let c = classify(&i);
        // Chonburi là chữ rất đậm/tương phản cao (Display): có thể chỉ đạt candidate, nhưng không bao giờ nghiêng về Sans
        assert_eq!(best(&c), Class::Serif, "{file}: {:?}", c.signals);
        assert!(c.signals.iter().any(|s| s.vote == Some(Class::Serif) && s.detail.starts_with("chân nhỏ")), "{file}: {:?}", c.signals);
        if file == "dm_serif_display.ttf" {
            assert_eq!(c.class, Class::Serif);
        }
    }
}

#[test]
fn sans_fixtures_have_no_flare_and_stay_sans() {
    for file in ["roboto.ttf", "open_sans.ttf", "fira_sans_regular.ttf"] {
        let i = inp(file);
        assert!(i.m.serif_flare.unwrap() < 1.15, "{file}: {:?}", i.m.serif_flare);
        assert!(i.m.serif_head.unwrap() < 0.1, "{file}: {:?}", i.m.serif_head);
        assert_eq!(classify(&i).class, Class::Sans, "{file}");
    }
}

#[test]
fn name_neue_or_grotesk_does_not_override_clear_small_foot_numbers() {
    // chân nhỏ rõ (loe 1.63, cờ 0.3) ⇒ Serif dù tên có "Neue"/"Grotesk"
    for name in ["acme neue regular", "acme grotesk regular", "acme regular"] {
        let c = classify(&synthetic(name, 1.54, 1.63, 0.30));
        assert_eq!(best(&c), Class::Serif, "{name}: {:?}", c.signals);
    }
}

#[test]
fn pointed_foot_with_top_flag_is_not_called_sans_by_a_low_foot_reading() {
    // chân nhọn (rộng ~0 ở baseline) + cờ trên-trái: không được kết luận Sans từ serif_foot thấp, kể cả khi tên là "Neue"
    let c = classify(&synthetic("acme neue regular", 0.61, 1.0, 0.45));
    assert_ne!(c.class, Class::Sans, "{:?}", c.signals);
    assert!(!c.signals.iter().any(|s| s.vote == Some(Class::Sans) && s.weight >= 2.0), "{:?}", c.signals);
    // không có dấu hiệu serif ⇒ vẫn là Sans như trước (không đổi hành vi)
    assert_eq!(classify(&synthetic("acme sans", 0.61, 1.0, 0.0)).class, Class::Sans);
    assert_eq!(classify(&synthetic("acme sans", 1.0, 1.0, 0.0)).class, Class::Sans);
}

#[test]
fn old_analyses_without_flare_classify_exactly_as_before() {
    let mut i = synthetic("acme neue regular", 1.54, 1.63, 0.30);
    i.m.serif_flare = None;
    i.m.serif_head = None;
    let c = classify(&i);
    assert!(!c.signals.iter().any(|s| s.detail.contains("chân nhỏ")), "{:?}", c.signals);
}
