//! G4: điểm hợp vai + phong cách pair. Hàm thuần trên số đo; không điểm số trong kết quả, chỉ mức + mã lý do.
use fontatlas_core::classify::Class;
use fontatlas_core::pairing::{
    Fit, PairCtx, PairStyle, Role, Traits, ViPolicy, family_root, pair_style, role_fit, role_fit_with, role_from_name,
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

// Chính sách tiếng Việt (chủ app đồng ý 2026-11-15): chỉ hạ mức khi Case/Desk bật "Đủ tiếng Việt" (`ViPolicy::Require`);
// không bật thì chấm theo hình dạng và chỉ ghi chú `missing_vietnamese` (UI hiện NoteMark "!").

#[test]
fn without_require_vietnamese_shape_decides_and_a_note_is_added() {
    let mut t = roboto_like();
    t.vi_missing = Some(12);
    for role in [Role::Body, Role::Caption, Role::Label] {
        let strict = role_fit_with(&t, role, ViPolicy::Require);
        let loose = role_fit_with(&t, role, ViPolicy::Note);
        assert_eq!(strict.fit, Fit::Weak, "{role:?} đòi đủ tiếng Việt vẫn hạ xuống Weak");
        assert!(codes(&strict).contains(&"vi_missing"));
        // cùng số đo nhưng đủ tiếng Việt: mức theo hình dạng
        let mut full = t.clone();
        full.vi_missing = Some(0);
        assert_eq!(loose.fit, role_fit(&full, role).fit, "{role:?}: mức không còn bị hạ");
        assert!(loose.fit > Fit::Weak, "{role:?}: {:?}", loose.fit);
        assert_eq!(codes(&loose).first(), Some(&"missing_vietnamese"), "ghi chú đứng đầu: {:?}", codes(&loose));
        assert!(!codes(&loose).contains(&"vi_missing"));
        let n = loose.reasons[0].params.iter().find(|(k, _)| *k == "count").map(|(_, v)| *v);
        assert_eq!(n, Some(12.0));
    }
}

#[test]
fn without_require_vietnamese_display_roles_are_not_capped_at_ok_either() {
    let mut d = didone_display();
    d.vi_missing = Some(40);
    let mut full = d.clone();
    full.vi_missing = Some(0);
    let loose = role_fit_with(&d, Role::Display, ViPolicy::Note);
    assert_eq!(loose.fit, role_fit(&full, Role::Display).fit);
    assert_eq!(role_fit_with(&d, Role::Display, ViPolicy::Require).fit, Fit::Ok.min(loose.fit));
    assert!(codes(&loose).contains(&"missing_vietnamese"));
}

#[test]
fn no_note_when_vietnamese_is_complete_or_unknown() {
    let t = roboto_like(); // vi_missing = Some(0)
    assert!(!codes(&role_fit_with(&t, Role::Body, ViPolicy::Note)).contains(&"missing_vietnamese"));
    let mut u = roboto_like();
    u.vi_missing = None; // chưa đo: không đoán
    assert!(!codes(&role_fit_with(&u, Role::Body, ViPolicy::Note)).contains(&"missing_vietnamese"));
    assert_eq!(role_fit_with(&t, Role::Body, ViPolicy::Note).fit, role_fit(&t, Role::Body).fit);
}

#[test]
fn numerals_never_get_a_vietnamese_note_because_digits_do_not_need_it() {
    let mut t = roboto_like();
    t.vi_missing = Some(30);
    assert!(!codes(&role_fit_with(&t, Role::Numerals, ViPolicy::Note)).contains(&"missing_vietnamese"));
    assert!(!codes(&role_fit_with(&t, Role::Numerals, ViPolicy::Require)).contains(&"vi_missing"));
}

#[test]
fn note_policy_keeps_the_other_gates() {
    let mut t = roboto_like();
    t.vi_missing = Some(12);
    t.class = Class::Script; // lớp không hợp chữ đọc: vẫn hạ xuống Weak dù không đòi tiếng Việt
    let r = role_fit_with(&t, Role::Body, ViPolicy::Note);
    assert_eq!(r.fit, Fit::Weak);
    assert!(codes(&r).contains(&"class_unfit"));
    assert!(codes(&r).contains(&"missing_vietnamese"));
}

// ---- Bộ mù 2026-11-16 (docs/team/eval/pairing/blind/RESULTS-2026-11-16.md): P1–P4. Ngưỡng "lệch xa" = p1/p99 của chữ đọc
// (1259 họ Sans+Serif OFL, tập eval 1722 họ), KHÔNG lấy từ thẻ nào của bộ mù.

const ALL_ROLES: [Role; 7] = [Role::Body, Role::Caption, Role::Heading, Role::Display, Role::Quote, Role::Label, Role::Numerals];

fn far_param(r: &fontatlas_core::pairing::RoleFit) -> bool {
    r.reasons.iter().any(|x| x.params.iter().any(|(k, _)| *k == "far"))
}

#[test]
fn p1_implausible_x_height_is_unknown_for_every_role() {
    // font "Guides" (đường kẻ bị đo vào bbox): x-height > 0,8 em
    let mut t = roboto_like();
    t.x_height_em = Some(1.075);
    for role in ALL_ROLES {
        let r = role_fit(&t, role);
        assert_eq!(r.fit, Fit::Unknown, "{role:?}");
        assert_eq!(codes(&r).first(), Some(&"measure_doubtful"), "{role:?}: {:?}", codes(&r));
    }
    // biên: đúng 0,8 chưa đáng ngờ; chưa đo thì không đoán
    t.x_height_em = Some(0.8);
    assert_ne!(role_fit(&t, Role::Body).fit, Fit::Unknown);
    t.x_height_em = None;
    assert!(!codes(&role_fit(&t, Role::Body)).contains(&"measure_doubtful"));
    // đúng cả với ViPolicy::Note
    t.x_height_em = Some(0.9);
    assert_eq!(role_fit_with(&t, Role::Display, ViPolicy::Note).fit, Fit::Unknown);
}

#[test]
fn p2_one_far_outlier_caps_text_roles_at_ok_two_at_weak() {
    // Lexend Peta/Tera: spacing 0,47–0,51 (p99 chữ đọc 0,294)
    let mut one = roboto_like();
    one.spacing = Some(0.47);
    for role in [Role::Body, Role::Caption] {
        let r = role_fit(&one, role);
        assert_eq!(r.fit, Fit::Ok, "{role:?}: {:?}", codes(&r));
        assert!(codes(&r).contains(&"spacing_loose"));
        assert!(far_param(&r), "{role:?}: lý do lệch xa mang tham số far");
    }
    let mut two = one.clone();
    two.width = Some(1.6); // vượt p99 width 1,379
    assert_eq!(role_fit(&two, Role::Body).fit, Fit::Weak);
    assert_eq!(role_fit(&two, Role::Caption).fit, Fit::Weak);
    // Label: width là tiêu chí của Label; một tiêu chí lệch xa → ok, hai → weak
    let mut label = roboto_like();
    label.width = Some(1.6);
    assert_eq!(role_fit(&label, Role::Label).fit, Fit::Ok);
    label.stem = Some(0.6); // vượt p99 stem 0,478
    assert_eq!(role_fit(&label, Role::Label).fit, Fit::Weak);
}

#[test]
fn p2_outside_the_band_but_not_far_is_not_capped() {
    // spacing 0,25 nằm giữa p90 (0,208) và p99 (0,294): lệch dải nhưng không "xa" → vẫn good như trước
    let mut t = roboto_like();
    t.spacing = Some(0.25);
    assert_eq!(role_fit(&t, Role::Body).fit, Fit::Good);
    assert!(!far_param(&role_fit(&t, Role::Body)));
}

#[test]
fn p2_body_and_caption_now_score_width() {
    let mut t = roboto_like();
    t.width = Some(1.25); // quá rộng nhưng chưa tới p99 (1,379): điểm width thấp, hiện lý do
    for role in [Role::Body, Role::Caption] {
        assert!(codes(&role_fit(&t, role)).contains(&"width_wide"), "{role:?}");
    }
    t.width = Some(0.92); // dải thường: không có lý do width xấu
    assert!(!codes(&role_fit(&t, Role::Body)).contains(&"width_wide"));
}

#[test]
fn p2_width_only_ever_lowers_a_text_role_never_raises_it() {
    // Cormorant-like: x-height thấp, tương phản cao; width bình thường (điểm width = 1). Thêm tiêu chí width KHÔNG được
    // làm điểm trung bình tăng lên so với khi width chưa đo (bản đầu của P2 đã nâng 47 họ Body và 64 họ Caption ở tập 2014 họ).
    // Body: điểm trung bình khi chưa có width ≈ 0,730 (ok); nếu width điểm 1 được tính vào trung bình thì ≈ 0,753 (good).
    let mut t = roboto_like();
    t.x_height_em = Some(0.42);
    t.contrast = Some(4.5);
    t.aperture = Some(120.0);
    let mut with_width = t.clone();
    with_width.width = Some(0.92);
    let mut no_width = t.clone();
    no_width.width = None;
    assert_eq!(role_fit(&no_width, Role::Body).fit, Fit::Ok);
    assert_eq!(role_fit(&with_width, Role::Body).fit, Fit::Ok, "width bình thường không được nâng Body từ ok lên good");
    for role in [Role::Body, Role::Caption] {
        assert!(role_fit(&with_width, role).fit <= role_fit(&no_width, role).fit, "{role:?}: width làm mức tăng");
    }
}

#[test]
fn p4_heading_far_outlier_caps_at_ok_and_more_far_criteria_fall_only_by_the_mean() {
    let mut t = roboto_like();
    assert_eq!(role_fit(&t, Role::Heading).fit, Fit::Good);
    // một tiêu chí lệch xa (Handjet: contrast 13,5; p99 chữ đọc 8,4): trần ok
    t.contrast = Some(13.5);
    let r = role_fit(&t, Role::Heading);
    assert_eq!(r.fit, Fit::Ok, "{:?}", codes(&r));
    assert!(codes(&r).contains(&"contrast_high") && far_param(&r));
    // hai tiêu chí lệch xa: luật Heading KHÔNG phạt thêm (trần vẫn là ok); mức cuối do điểm trung bình quyết định.
    // Ở đây stem và contrast (trọng số 2 + 2) đều 0, spacing (1) đạt: trung bình 1/5 = 0,2 → weak.
    t.stem = Some(0.6);
    let r2 = role_fit(&t, Role::Heading);
    assert_eq!(r2.fit, Fit::Weak, "{:?}", codes(&r2));
    assert!(far_param(&r2));
}

#[test]
fn p1_unknown_still_reports_the_vietnamese_note_and_class_gate_in_order() {
    // font "Guides": số đo vô lý nhưng roleFit đơn lẻ vẫn phải nói thiếu tiếng Việt và lớp không hợp
    let mut t = roboto_like();
    t.x_height_em = Some(1.075);
    t.vi_missing = Some(12);
    t.class = Class::Script;
    let note = role_fit_with(&t, Role::Body, ViPolicy::Note);
    assert_eq!(note.fit, Fit::Unknown);
    assert_eq!(codes(&note), ["measure_doubtful", "missing_vietnamese", "class_unfit"]);
    let strict = role_fit_with(&t, Role::Body, ViPolicy::Require);
    assert_eq!(strict.fit, Fit::Unknown);
    assert_eq!(codes(&strict), ["measure_doubtful", "vi_missing", "class_unfit"]);
    // Numerals: không có ghi chú tiếng Việt (chữ số không cần dấu)
    assert_eq!(codes(&role_fit_with(&t, Role::Numerals, ViPolicy::Note)), ["measure_doubtful"]);
}

#[test]
fn reason_order_gates_first_then_far_after_truncate_to_three() {
    // class biểu cảm + thiếu tiếng Việt (Note) + spacing lệch xa: hai cổng rồi lý do far, không bị truncate(3) cắt mất
    let mut t = roboto_like();
    t.class = Class::Script;
    t.vi_missing = Some(12);
    t.spacing = Some(0.47);
    let r = role_fit_with(&t, Role::Body, ViPolicy::Note);
    assert_eq!(codes(&r), ["missing_vietnamese", "class_unfit", "spacing_loose"]);
    assert!(r.reasons[2].params.iter().any(|(k, _)| *k == "far"), "lý do thứ ba là lý do lệch xa");
    assert_eq!(r.fit, Fit::Weak);
}

#[test]
fn far_never_applies_to_display_or_numerals() {
    // Display cần cá tính, Numerals chỉ có hai tiêu chí: không có trần far và không có tham số far trên lý do
    let mut d = didone_display();
    d.spacing = Some(0.5);
    d.stem = Some(0.6);
    d.contrast = Some(13.0);
    let rd = role_fit(&d, Role::Display);
    assert!(!far_param(&rd), "{:?}", codes(&rd));
    assert_eq!(rd.fit, Fit::Good, "trần far không áp cho Display: {:?}", codes(&rd));
    let mut n = roboto_like();
    n.spacing = Some(0.5);
    n.stem = Some(0.6);
    let rn = role_fit(&n, Role::Numerals);
    assert!(!far_param(&rn), "{:?}", codes(&rn));
}

#[test]
fn p3_a_far_reason_never_comes_with_good() {
    // lệch từng trục tới mức xa, mọi vai: nếu lý do mang `far` thì mức không bao giờ là Good
    type Mutate = fn(&mut Traits);
    let far: [(&str, Mutate); 7] = [
        ("x_height", |t| t.x_height_em = Some(0.76)),
        ("aperture", |t| t.aperture = Some(140.0)),
        ("aperture_e", |t| t.aperture_e = Some(0.45)),
        ("contrast", |t| t.contrast = Some(9.0)),
        ("spacing", |t| t.spacing = Some(0.5)),
        ("stem", |t| t.stem = Some(0.6)),
        ("width", |t| t.width = Some(1.6)),
    ];
    let mut flagged = 0;
    for (axis, apply) in far {
        let mut t = roboto_like();
        apply(&mut t);
        for role in ALL_ROLES {
            let r = role_fit(&t, role);
            if far_param(&r) {
                flagged += 1;
                assert_ne!(r.fit, Fit::Good, "{axis} {role:?}: {:?}", codes(&r));
            }
        }
    }
    assert!(flagged >= 7, "ít nhất mỗi trục phải làm một vai có lý do far (đang {flagged})");
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
fn an_uncertain_class_falls_back_to_the_classifier_candidate() {
    // Oswald: class Uncertain nhưng candidate Sans ⇒ vẫn dùng được cho phong cách tương phản serif + sans.
    let oswald = Traits { class: Class::Uncertain, candidate: Some(Class::Sans), x_height_em: Some(0.50), ..Traits::default() };
    let r = pair_style(&old_serif(), &oswald, &PairCtx::default(), PairStyle::Contrast);
    assert_ne!(r.fit, Fit::Unknown);
    assert!(r.reasons.iter().any(|x| x.code == "serif_vs_sans"), "{:?}", r.reasons);
    // Không có candidate ⇒ vẫn Unknown (không đoán)
    let blank = Traits { class: Class::Uncertain, ..Traits::default() };
    assert_eq!(pair_style(&old_serif(), &blank, &PairCtx::default(), PairStyle::Contrast).fit, Fit::Unknown);
    // Pacifico: Uncertain + candidate Script ⇒ không làm chữ thân
    let pac = Traits { class: Class::Uncertain, candidate: Some(Class::Script), ..roboto_like() };
    assert_eq!(role_fit(&pac, Role::Body).fit, Fit::Weak);
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
