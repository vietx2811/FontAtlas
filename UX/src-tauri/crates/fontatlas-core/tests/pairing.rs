//! G4: điểm hợp vai + phong cách pair. Hàm thuần trên số đo; không điểm số trong kết quả, chỉ mức + mã lý do.
use fontatlas_core::classify::Class;
use fontatlas_core::pairing::{
    Fit, PairCtx, PairStyle, Role, Traits, family_root, pair_style, role_fit, role_from_name,
};

fn roboto_like() -> Traits {
    Traits {
        class: Class::Sans,
        subclass: Some("neo_grotesque".into()),
        x_height_em: Some(0.53),
        x_height_cap: Some(0.74),
        aperture: Some(90.0),
        aperture_e: Some(0.25),
        contrast: Some(1.3),
        stem: Some(0.16),
        spacing: Some(0.17),
        width: Some(0.92),
        vi_missing: Some(0),
        has_tnum: Some(true),
        ..Traits::default()
    }
}

fn didone_display() -> Traits {
    Traits {
        class: Class::Serif,
        subclass: Some("didone".into()),
        x_height_em: Some(0.46),
        x_height_cap: Some(0.68),
        aperture: Some(55.0),
        aperture_e: Some(0.20),
        contrast: Some(9.0),
        stem: Some(0.10),
        spacing: Some(0.07),
        width: Some(0.95),
        vi_missing: Some(0),
        ..Traits::default()
    }
}

fn old_serif() -> Traits {
    Traits {
        class: Class::Serif,
        subclass: Some("garalde".into()),
        x_height_em: Some(0.49),
        x_height_cap: Some(0.70),
        aperture: Some(80.0),
        aperture_e: Some(0.28),
        contrast: Some(2.6),
        stem: Some(0.17),
        spacing: Some(0.10),
        width: Some(1.0),
        vi_missing: Some(0),
        ..Traits::default()
    }
}

fn codes(r: &fontatlas_core::pairing::RoleFit) -> Vec<&'static str> {
    r.reasons.iter().map(|x| x.code).collect()
}

#[test]
fn a_readable_sans_is_good_for_body() {
    let r = role_fit(&roboto_like(), Role::Body);
    assert_eq!(r.fit, Fit::Good, "{:?}", r.reasons);
}

#[test]
fn high_contrast_didone_is_good_display_but_not_good_body() {
    let d = didone_display();
    assert_eq!(role_fit(&d, Role::Display).fit, Fit::Good);
    let b = role_fit(&d, Role::Body);
    assert!(b.fit == Fit::Weak || b.fit == Fit::Ok, "{:?}", b.fit);
    assert!(codes(&b).contains(&"contrast_high"), "{:?}", codes(&b));
}

#[test]
fn missing_vietnamese_caps_text_roles_but_only_notes_display() {
    let mut t = roboto_like();
    t.vi_missing = Some(12);
    let body = role_fit(&t, Role::Body);
    assert_eq!(body.fit, Fit::Weak);
    assert!(codes(&body).contains(&"vi_missing"));
    let mut d = didone_display();
    d.vi_missing = Some(12);
    let disp = role_fit(&d, Role::Display);
    assert_ne!(disp.fit, Fit::Weak);
    assert!(codes(&disp).contains(&"vi_missing"));
}

#[test]
fn script_and_blackletter_are_weak_for_body() {
    let mut t = roboto_like();
    t.class = Class::Script;
    assert_eq!(role_fit(&t, Role::Body).fit, Fit::Weak);
    t.class = Class::Blackletter;
    assert_eq!(role_fit(&t, Role::Body).fit, Fit::Weak);
}

#[test]
fn script_is_good_for_display_when_vietnamese_is_complete() {
    let mut t = roboto_like();
    t.class = Class::Script;
    assert_eq!(role_fit(&t, Role::Display).fit, Fit::Good);
}

#[test]
fn caption_needs_more_than_body() {
    let mut t = roboto_like();
    t.x_height_em = Some(0.47);
    t.stem = Some(0.08);
    assert_eq!(role_fit(&t, Role::Body).fit, Fit::Good);
    assert_ne!(role_fit(&t, Role::Caption).fit, Fit::Good);
}

#[test]
fn expressive_faces_the_classifier_leaves_uncertain_still_read_as_display() {
    // Số đo thật: Bebas Neue (chữ hoa, class Sans) và Pacifico (script, class Uncertain).
    let bebas = Traits {
        class: Class::Sans,
        x_height_em: Some(0.70),
        x_height_cap: Some(1.0),
        contrast: Some(1.1),
        stem: Some(0.14),
        aperture_e: Some(0.31),
        spacing: Some(0.076),
        vi_missing: Some(0),
        ..Traits::default()
    };
    assert_eq!(role_fit(&bebas, Role::Display).fit, Fit::Good);
    let b = role_fit(&bebas, Role::Body);
    assert_eq!(b.fit, Fit::Weak);
    assert!(codes(&b).contains(&"class_unfit"));
    let pacifico = Traits {
        class: Class::Uncertain,
        x_height_em: Some(0.47),
        x_height_cap: Some(0.54),
        contrast: Some(2.7),
        stem: Some(0.358),
        aperture_e: Some(0.027),
        spacing: Some(0.0),
        vi_missing: Some(0),
        ..Traits::default()
    };
    assert_eq!(role_fit(&pacifico, Role::Display).fit, Fit::Good);
    assert_ne!(role_fit(&pacifico, Role::Body).fit, Fit::Good);
}

#[test]
fn numerals_require_tabular_figures() {
    let mut t = roboto_like();
    t.has_tnum = Some(false);
    let r = role_fit(&t, Role::Numerals);
    assert!(matches!(r.fit, Fit::Weak | Fit::Ok));
    assert!(codes(&r).contains(&"tnum_missing"));
}

#[test]
fn no_measures_gives_unknown_not_a_guess() {
    let t = Traits { class: Class::Sans, ..Traits::default() };
    assert_eq!(role_fit(&t, Role::Body).fit, Fit::Unknown);
}

#[test]
fn reasons_are_at_most_three() {
    let mut t = didone_display();
    t.vi_missing = Some(3);
    assert!(role_fit(&t, Role::Body).reasons.len() <= 3);
}

#[test]
fn custom_role_names_map_through_a_synonym_table() {
    assert_eq!(role_from_name("Subhead"), Some(Role::Heading));
    assert_eq!(role_from_name("  Giá "), Some(Role::Numerals));
    assert_eq!(role_from_name("eyebrow"), Some(Role::Label));
    assert_eq!(role_from_name("Chú thích"), Some(Role::Caption));
    assert_eq!(role_from_name("Hero"), Some(Role::Display));
    assert_eq!(role_from_name("Dự án 7"), None);
}

#[test]
fn family_root_strips_role_words_so_superfamilies_match() {
    assert_eq!(family_root("Source Sans 3"), family_root("Source Serif 4"));
    assert_eq!(family_root("IBM Plex Sans"), family_root("IBM Plex Serif"));
    assert_eq!(family_root("Merriweather"), family_root("Merriweather Sans"));
    assert_ne!(family_root("Lora"), family_root("Inter"));
}

#[test]
fn contrast_style_needs_one_serif_and_one_sans_with_close_x_height() {
    let ctx = PairCtx::default();
    let ok = pair_style(&old_serif(), &roboto_like(), &ctx, PairStyle::Contrast);
    assert_eq!(ok.fit, Fit::Good, "{:?}", ok.reasons);
    let two_sans = pair_style(&roboto_like(), &roboto_like(), &ctx, PairStyle::Contrast);
    assert_eq!(two_sans.fit, Fit::Weak);
    let mut big = roboto_like();
    big.x_height_em = Some(0.58);
    let mism = pair_style(&old_serif(), &big, &ctx, PairStyle::Contrast);
    assert_ne!(mism.fit, Fit::Good);
    assert!(mism.warnings.iter().any(|w| w.code == "xheight_mismatch"));
}

#[test]
fn superfamily_style_follows_the_family_root() {
    let same = PairCtx { same_family_root: true, ..PairCtx::default() };
    assert_eq!(
        pair_style(&old_serif(), &roboto_like(), &same, PairStyle::Superfamily).fit,
        Fit::Good
    );
    let other = PairCtx::default();
    assert_eq!(
        pair_style(&old_serif(), &roboto_like(), &other, PairStyle::Superfamily).fit,
        Fit::Weak
    );
}

#[test]
fn same_structure_is_good_for_near_identical_construction_across_classes() {
    let mut sans = roboto_like();
    let serif = old_serif();
    sans.x_height_em = serif.x_height_em;
    sans.contrast = Some(2.5);
    sans.aperture_e = serif.aperture_e;
    sans.stem = serif.stem;
    let r = pair_style(&serif, &sans, &PairCtx::default(), PairStyle::SameStructure);
    assert_eq!(r.fit, Fit::Good, "{:?}", r.reasons);
    let far = pair_style(&didone_display(), &roboto_like(), &PairCtx::default(), PairStyle::SameStructure);
    assert_eq!(far.fit, Fit::Weak);
}

#[test]
fn same_era_uses_the_subclass_table() {
    let mut garalde = old_serif();
    let mut venetian = old_serif();
    garalde.subclass = Some("garalde".into());
    venetian.subclass = Some("venetian_oldstyle".into());
    assert_eq!(pair_style(&garalde, &venetian, &PairCtx::default(), PairStyle::SameEra).fit, Fit::Good);
    let neo = roboto_like();
    assert_eq!(pair_style(&garalde, &neo, &PairCtx::default(), PairStyle::SameEra).fit, Fit::Weak);
    let mut none = roboto_like();
    none.subclass = None;
    assert_eq!(pair_style(&garalde, &none, &PairCtx::default(), PairStyle::SameEra).fit, Fit::Unknown);
}

#[test]
fn display_vs_neutral_wants_one_expressive_and_one_neutral() {
    let ctx = PairCtx::default();
    let good = pair_style(&didone_display(), &roboto_like(), &ctx, PairStyle::DisplayVsNeutral);
    assert_eq!(good.fit, Fit::Good, "{:?}", good.reasons);
    let both = pair_style(&didone_display(), &didone_display(), &ctx, PairStyle::DisplayVsNeutral);
    assert_eq!(both.fit, Fit::Weak);
    assert!(both.reasons.iter().any(|r| r.code == "both_expressive"));
}

#[test]
fn harmony_uses_image_similarity_and_is_unknown_without_it() {
    let a = old_serif();
    let b = roboto_like();
    let none = pair_style(&a, &b, &PairCtx::default(), PairStyle::Harmony);
    assert_eq!(none.fit, Fit::Unknown);
    let close = PairCtx { embed_cos: Some(0.9), ..PairCtx::default() };
    assert_eq!(pair_style(&a, &b, &close, PairStyle::Harmony).fit, Fit::Good);
    let far = PairCtx { embed_cos: Some(0.4), ..PairCtx::default() };
    assert_eq!(pair_style(&a, &b, &far, PairStyle::Harmony).fit, Fit::Weak);
}

#[test]
fn nearly_the_same_but_not_the_same_family_warns() {
    let ctx = PairCtx { embed_cos: Some(0.7), ..PairCtx::default() };
    let r = pair_style(&old_serif(), &roboto_like(), &ctx, PairStyle::Contrast);
    assert!(r.warnings.iter().any(|w| w.code == "too_close_not_same"));
    let same = PairCtx { embed_cos: Some(0.7), same_family_root: true, ..PairCtx::default() };
    let r2 = pair_style(&old_serif(), &roboto_like(), &same, PairStyle::Contrast);
    assert!(!r2.warnings.iter().any(|w| w.code == "too_close_not_same"));
}

#[test]
fn traits_from_a_real_face_classify_and_measure() {
    let raw = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/dm_serif_display.ttf")).unwrap();
    let rep = fontatlas_core::analyze_bytes(raw).unwrap();
    let t = fontatlas_core::pairing::traits_from_face(&rep.faces[0]).unwrap();
    assert_eq!(t.class, Class::Serif);
    assert!(t.x_height_em.is_some() && t.contrast.is_some());
    assert!(t.vi_missing.is_some());
    // Hạng Good chỉ khi đủ tiếng Việt; thiếu thì trần Ok kèm mã vi_missing (đúng thiết kế).
    let d = role_fit(&t, Role::Display);
    let expect = if t.vi_missing == Some(0) { Fit::Good } else { Fit::Ok };
    assert_eq!(d.fit, expect, "vi_missing={:?} {:?}", t.vi_missing, d.reasons);
    assert_eq!(d.reasons.iter().any(|r| r.code == "vi_missing"), t.vi_missing != Some(0));
}

#[test]
fn style_ids_round_trip_as_snake_case() {
    for (s, id) in [
        (PairStyle::Contrast, "contrast"),
        (PairStyle::Superfamily, "superfamily"),
        (PairStyle::SameStructure, "same_structure"),
        (PairStyle::SameEra, "same_era"),
        (PairStyle::DisplayVsNeutral, "display_vs_neutral"),
        (PairStyle::Harmony, "harmony"),
    ] {
        assert_eq!(s.id(), id);
        assert_eq!(PairStyle::from_id(id), Some(s));
    }
    assert_eq!(PairStyle::from_id("rule_breaking"), None);
}
