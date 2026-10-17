mod common;
use common::*;
use fontatlas_index::{
    AxisId, Block, BodyWarning, ClashKind, Gate, PREFIX_STOP_ALWAYS,
    PREFIX_STOP_UNLESS_SAME_MANUFACTURER, PairAxis, PairContext, Q, Relation, Role, Side,
    SuperfamilyTable, Thresholds, assess, rank_key,
};

fn ctx(vi: bool) -> PairContext<'static> {
    // leak để có 'static trong test
    PairContext {
        thresholds: Box::leak(Box::new(Thresholds::v1())),
        reference: Box::leak(Box::new(test_reference())),
        superfamilies: Box::leak(Box::new(SuperfamilyTable::bundled())),
        project_vi: vi,
    }
}

#[test]
fn same_family_is_safe() {
    let a = synth_full("Inter", "Regular", 0.15);
    let b = synth_full("Inter", "Bold", 0.26);
    assert_eq!(
        assess(
            &a,
            Role::Heading,
            &b,
            Role::Body,
            Some(&FACTS_OK),
            &ctx(false)
        )
        .gate,
        Gate::Safe
    );
}

#[test]
fn csv_superfamily_is_related() {
    let a = synth_full("Source Serif 4", "Regular", 0.15);
    let b = synth_full("Source Sans 3", "Regular", 0.15);
    let x = assess(
        &a,
        Role::Heading,
        &b,
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(x.related);
    assert_eq!(x.gate, Gate::Safe);
}

#[test]
fn near_identical_other_family_is_global_clash() {
    // giống Helvetica + Arial: mọi trục gần như bằng nhau, khác họ
    let a = synth_full("Helvetica", "Regular", 0.168);
    let b = synth_full("Arial", "Regular", 0.169);
    let x = assess(
        &a,
        Role::Heading,
        &b,
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(x.clashes.contains(&ClashKind::Global));
    assert_eq!(x.gate, Gate::Experimental);
}

#[test]
fn same_flesh_different_skeleton_is_clash() {
    // biểu cảm hoà hợp, aperture tương phản (Source Sans + Helvetica trong font matrix)
    let a = synth_with("Source Sans 3", &[(AxisId::Aperture, 150.0)]);
    let b = synth_with("Helvetica", &[(AxisId::Aperture, 50.0)]);
    assert!(
        assess(
            &a,
            Role::Heading,
            &b,
            Role::Body,
            Some(&FACTS_OK),
            &ctx(false)
        )
        .clashes
        .contains(&ClashKind::Skeleton)
    );
}

#[test]
fn prefix_stoplist_and_manufacturer() {
    let t = SuperfamilyTable::from_csv("group,family,source\n").unwrap(); // bảng rỗng: chỉ quy tắc metadata
    let mk = |f: &str, m: Option<&str>| {
        let mut p = synth_full(f, "Regular", 0.15);
        p.manufacturer = m.map(String::from);
        p
    };
    assert!(!t.related(&mk("Libre Baskerville", None), &mk("Libre Franklin", None))); // luôn chặn
    assert!(!t.related(&mk("Red Hat Display", None), &mk("Red Rose", None))); // luôn chặn
    assert!(!t.related(&mk("Noto Serif", Some("A")), &mk("Noto Sans", Some("B")))); // khác manufacturer
    assert!(t.related(
        &mk("Noto Serif", Some("Google")),
        &mk("Noto Sans", Some("google"))
    )); // cùng manufacturer
    assert!(t.related(&mk("Gowun Batang", None), &mk("Gowun Dodum", None))); // tiền tố thường
    assert_eq!(PREFIX_STOP_ALWAYS.len(), 6);
    assert_eq!(PREFIX_STOP_UNLESS_SAME_MANUFACTURER.len(), 3);
}

#[test]
fn global_and_skeleton_need_a_determined_expressive_axis() {
    // mọi trục biểu cảm Unsure (Failed), cấu trúc gần nhau / aperture tương phản → không có Global, không có Skeleton
    let mk = |f: &str, ap: f32| {
        synth(
            f,
            &[
                (AxisId::XHeightEm, 0.5, Q::Measured, M_XH),
                (AxisId::Width, 1.0, Q::Measured, M_WIDTH),
                (AxisId::Aperture, ap, Q::Measured, "c_angular_opening"),
            ],
        )
    };
    let x = assess(
        &mk("Aa", 100.0),
        Role::Body,
        &mk("Bb", 100.0),
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(!x.clashes.contains(&ClashKind::Global));
    let y = assess(
        &mk("Aa", 160.0),
        Role::Body,
        &mk("Bb", 40.0),
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(!y.clashes.contains(&ClashKind::Skeleton));
}

#[test]
fn rank_puts_gate_first() {
    let mut safe = blank_assessment();
    safe.gate = Gate::Safe;
    safe.clashes = vec![ClashKind::Axis(PairAxis::Width)];
    let mut exp = blank_assessment();
    exp.gate = Gate::Experimental;
    let mut v = [("exp", &exp), ("safe", &safe)];
    v.sort_by_key(|(n, x)| rank_key(x, n, None));
    assert_eq!(v[0].0, "safe");
}

#[test]
fn intentional_contrast_suppresses_global_clash() {
    // R7: Playfair + Gelasio — D nhỏ nhưng contrast tương phản rõ
    let a = synth_with("Playfair Display", &[(AxisId::Contrast, 6.0)]);
    let b = synth_with("Gelasio", &[(AxisId::Contrast, 2.6)]);
    assert!(
        !assess(
            &a,
            Role::Heading,
            &b,
            Role::Body,
            Some(&FACTS_OK),
            &ctx(false)
        )
        .clashes
        .contains(&ClashKind::Global)
    );
}

#[test]
fn h1_blocks_before_everything() {
    let mut a = synth_full("Josefin Slab", "Regular", 0.15);
    a.vi_missing = 102;
    let b = synth_full("Josefin Sans", "Regular", 0.15);
    let x = assess(
        &a,
        Role::Heading,
        &b,
        Role::Body,
        Some(&FACTS_OK),
        &ctx(true),
    );
    assert_eq!(x.gate, Gate::Blocked);
    assert!(matches!(
        x.block,
        Some(Block::MissingVietnamese {
            side: Side::A,
            missing: 102
        })
    ));
    // dự án không phải tiếng Việt → không chặn
    assert_ne!(
        assess(
            &a,
            Role::Heading,
            &b,
            Role::Body,
            Some(&FACTS_OK),
            &ctx(false)
        )
        .gate,
        Gate::Blocked
    );
}

#[test]
fn body_warnings_gate_but_low_xheight_only_warns() {
    let h = synth_full("H", "Regular", 0.15);
    let mut b = synth_full("B", "Regular", 0.15);
    set_value(&mut b, AxisId::XHeightEm, 0.40);
    let x = assess(
        &h,
        Role::Heading,
        &b,
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(x.body_warnings.contains(&BodyWarning::LowXHeight));
    set_value(&mut b, AxisId::Contrast, 5.0);
    let y = assess(
        &h,
        Role::Heading,
        &b,
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(y.body_warnings.contains(&BodyWarning::HighContrast));
    assert!(!matches!(y.gate, Gate::Safe | Gate::Controlled));
}

#[test]
fn mostly_unsure_never_safe() {
    let a = synth("A", &[(AxisId::XHeightEm, 0.5, Q::Measured, M_XH)]); // 7/8 trục Unsure
    let b = synth("A", &[(AxisId::XHeightEm, 0.5, Q::Measured, M_XH)]); // cùng family!
    let x = assess(
        &a,
        Role::Heading,
        &b,
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(x.insufficient_data);
    assert_eq!(x.gate, Gate::Experimental);
}

#[test]
fn non_latin_face_is_insufficient() {
    let cjk = synth("Hiragino Sans", &[]); // mọi trục Failed
    let x = assess(
        &cjk,
        Role::Heading,
        &synth_full("Inter", "Regular", 0.15),
        Role::Body,
        Some(&FACTS_OK),
        &ctx(false),
    );
    assert!(x.insufficient_data);
    assert_eq!(x.gate, Gate::Experimental);
}

#[test]
fn rank_prefers_fewer_clashes_then_structure_harmony() {
    let mk = |clashes: usize, struct_h: usize| {
        let mut x = blank_assessment(); // mọi nhãn Unsure, không clash, related = false
        for (i, ax) in [
            PairAxis::XHeight,
            PairAxis::Width,
            PairAxis::Aperture,
            PairAxis::Stress,
        ]
        .iter()
        .enumerate()
        {
            if i < struct_h {
                set_label(&mut x, *ax, Relation::Harmony);
            }
        }
        x.clashes = (0..clashes)
            .map(|_| ClashKind::Axis(PairAxis::Width))
            .collect();
        x.gate = Gate::Experimental; // cùng cổng để thử các khoá sau
        x
    };
    let (one_clash, two_h, four_h) = (mk(1, 4), mk(0, 2), mk(0, 4));
    let mut v = [("c", &one_clash), ("b", &two_h), ("a", &four_h)];
    v.sort_by_key(|(n, x)| rank_key(x, n, None));
    assert_eq!(v.iter().map(|p| p.0).collect::<Vec<_>>(), ["a", "b", "c"]);
}
