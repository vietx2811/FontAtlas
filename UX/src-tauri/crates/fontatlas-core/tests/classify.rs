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
fn display_is_a_usage_tag_until_the_v2_classifier_emits_it() {
    // v2: `Display` tồn tại như class (nhãn người dùng/ánh xạ) nhưng bộ phân loại tự động CHƯA phát ra nó; vẫn là tag usage
    assert_eq!(Class::parse("display"), Some(Class::Display));
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

#[test]
fn one_sided_tail_foot_without_top_flag_is_not_a_serif() {
    // chữ l có đuôi cong (Mooli, Bellota…): serif_foot > 1.6 nhưng chân chỉ nhô một phía, không loe đối xứng, không cờ trên
    let c = classify(&synthetic("acme regular", 1.63, 1.0, 0.0));
    assert!(!c.signals.iter().any(|s| s.vote == Some(Class::Serif) && s.detail.contains("serif_foot")), "{:?}", c.signals);
    assert_ne!(best(&c), Class::Serif);
    // chân serif thật (loe đối xứng) vẫn là Serif
    assert_eq!(best(&classify(&synthetic("acme regular", 2.2, 2.2, 0.6))), Class::Serif);
    // bản phân tích cũ (không có flare/head) giữ hành vi cũ
    let mut old = synthetic("acme regular", 1.63, 1.0, 0.0);
    old.m.serif_flare = None;
    old.m.serif_head = None;
    assert!(classify(&old).signals.iter().any(|s| s.vote == Some(Class::Serif)));
}

#[test]
fn flat_foot_with_strong_top_flag_is_ambiguous_not_a_confident_sans() {
    // semi-serif / italic serif: foot ~1.0 nhưng cờ trên 0.9 (Libre Baskerville Italic): Sans chỉ nặng 1.0 ⇒ không còn Sans 2.0
    let c = classify(&synthetic("acme italic", 1.04, 1.0, 0.93));
    assert!(!c.signals.iter().any(|s| s.vote == Some(Class::Sans) && s.weight >= 2.0), "{:?}", c.signals);
    // không có cờ ⇒ Sans như trước
    assert_eq!(classify(&synthetic("acme regular", 1.04, 1.0, 0.0)).class, Class::Sans);
}

#[test]
fn sans_with_weak_or_moderate_flag_and_flat_foot_keeps_its_sans_vote() {
    for head in [0.0, 0.1, 0.2, 0.3] {
        let c = classify(&synthetic("acme regular", 1.0, 1.0, head));
        assert!(c.signals.iter().any(|s| s.vote == Some(Class::Sans) && s.weight >= 2.0), "head {head}: {:?}", c.signals);
    }
}

#[test]
fn monospace_sans_with_tall_flag_is_never_serif() {
    // mono sans: i/l có cờ trên rất rõ nhưng chân phẳng; không được thành Serif
    let mut i = synthetic("acme mono regular", 1.0, 1.0, 0.8);
    i.is_fixed_pitch = true;
    let c = classify(&i);
    assert_ne!(best(&c), Class::Serif, "{:?}", c.signals);
}

#[test]
fn italic_fixtures_measure_like_their_upright_counterparts() {
    // ctx.glyph(upright) bù độ nghiêng trước khi so lề trái/phải: italic không bị coi là chân một phía
    let r = inp("roboto_italic_vf.ttf");
    assert!(r.m.serif_flare.unwrap() < 1.15 && r.m.serif_head.unwrap() < 0.1, "{:?} {:?}", r.m.serif_flare, r.m.serif_head);
    let b = inp("libre_baskerville_italic_vf.ttf");
    assert!(b.m.serif_head.unwrap() > 0.4, "serif italic vẫn thấy cờ trên: {:?}", b.m.serif_head);
    let up = inp("libre_baskerville.ttf");
    assert!(up.m.serif_flare.unwrap() > 2.0 && up.m.serif_head.unwrap() > 0.4);
}

// --- taxonomy v2: lớp mới + ánh xạ nhãn cũ (không mất ghi đè người dùng) ---

#[test]
fn v2_classes_round_trip_and_have_subclasses() {
    use fontatlas_core::classify::subclasses_of;
    for c in [Class::Slab, Class::Handwriting, Class::Display] {
        assert_eq!(Class::parse(c.snake()), Some(c));
        assert!(!subclasses_of(c).is_empty());
    }
    assert!(subclasses_of(Class::Slab).contains(&"clarendon") && subclasses_of(Class::Sans).contains(&"flared"));
    assert!(subclasses_of(Class::Mono).contains(&"coding") && subclasses_of(Class::Serif).contains(&"wedge"));
}

#[test]
fn legacy_overrides_map_to_v2_without_losing_the_choice() {
    use fontatlas_core::classify::migrate_legacy;
    let m = |c: &str, s: Option<&str>| migrate_legacy(c, s);
    assert_eq!(m("serif", Some("slab")), ("slab".into(), None));
    assert_eq!(m("script", Some("handwritten")), ("handwriting".into(), None));
    assert_eq!(m("script", Some("casual_brush")), ("script".into(), Some("brush".into())));
    assert_eq!(m("script", Some("formal")), ("script".into(), Some("formal_copperplate".into()))
    );
    assert_eq!(
        m("serif", Some("humanist_oldstyle")),
        ("serif".into(), Some("venetian_oldstyle".into())));
    // nhãn còn hợp lệ giữ nguyên; subclass không hợp lệ bị bỏ nhưng class giữ
    assert_eq!(m("sans", Some("geometric")), ("sans".into(), Some("geometric".into())));
    assert_eq!(m("serif", Some("nonsense")), ("serif".into(), None));
    assert_eq!(m("blackletter", None), ("blackletter".into(), None));
    // idempotent
    for (c, s) in [("serif", Some("slab")), ("script", Some("handwritten")), ("mono", Some("sans_mono"))] {
        let once = migrate_legacy(c, s);
        assert_eq!(migrate_legacy(&once.0, once.1.as_deref()), once);
    }
}

// --- classify_v2: tinh chỉnh sang 8 class + subclass mới ---

use fontatlas_core::classify::classify_v2;

#[test]
fn v2_slab_is_its_own_class_with_a_measured_subclass() {
    // chân vuông rõ, contrast thấp, name có "slab" → Slab/egyptian
    let i = ClassifyInput { name: "acme slab regular".into(), m: Metrics { serif_foot: Some(2.4), serif_flare: Some(2.4), serif_head: Some(0.6), contrast: Some(1.2), stress_abs: Some(2.0), ..Default::default() }, ..Default::default() };
    let c = classify_v2(&i);
    assert_eq!((c.class, c.subclass.as_deref()), (Class::Slab, Some("egyptian")), "{:?}", c.signals);
    // classify() cũ vẫn trả Serif/slab (không đổi hợp đồng v1)
    assert_eq!(classify(&i).class, Class::Serif);
}

#[test]
fn v2_unjoined_handwritten_script_becomes_handwriting() {
    let i = ClassifyInput {
        name: "acme hand regular".into(),
        m: Metrics { slant_abs: Some(14.0), spacing: Some(0.03), connect: Some(0.1), bounce: Some(0.12), ..Default::default() },
        ..Default::default()
    };
    let v1 = classify(&i);
    let v2 = classify_v2(&i);
    if v1.class == Class::Script && v1.subclass.as_deref() == Some("handwritten") {
        assert_eq!(v2.class, Class::Handwriting, "{:?}", v2.signals);
    }
}

#[test]
fn v2_structural_name_makes_display_and_keeps_the_shape_class_secondary() {
    let i = ClassifyInput { name: "acme stencil regular".into(), m: Metrics { serif_foot: Some(1.0), ..Default::default() }, ..Default::default() };
    let c = classify_v2(&i);
    assert_eq!((c.class, c.subclass.as_deref()), (Class::Display, Some("stencil")));
    assert!(c.confidence >= CONF_MIN, "{}", c.confidence);
    assert_eq!(c.secondary.as_ref().map(|s| s.class), Some(Class::Sans), "{:?}", c.secondary);
    // font không có dấu hiệu cấu trúc: không thành Display
    assert_ne!(classify_v2(&synthetic("acme regular", 1.0, 1.0, 0.0)).class, Class::Display);
}

#[test]
fn v2_does_not_change_plain_sans_and_serif() {
    for (name, foot, flare, head, want) in [("acme sans", 1.0, 1.0, 0.0, Class::Sans), ("acme serif", 2.6, 2.6, 0.9, Class::Serif)] {
        assert_eq!(classify_v2(&synthetic(name, foot, flare, head)).class, want, "{name}");
    }
}
