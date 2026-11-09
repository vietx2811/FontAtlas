mod common;
use common::*;
use fontatlas_index::{AxisId, GroupWeights, IndexError, Q, Role, SimilarOptions, SimilarSource};

#[test]
fn same_family_ranks_first() {
    let ix = index_of(vec![
        synth_full("A", "Regular", 0.15),
        synth_full("A", "Bold", 0.24),
        synth_with("B", &[(AxisId::Contrast, 4.0)]),
        synth_with("C", &[(AxisId::Width, 1.4)]),
    ]);
    let hits = ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            3,
            &|_| true,
        )
        .unwrap();
    assert_eq!(hits[0].key, key_of("A", "Bold"));
}

#[test]
fn similar_excludes_self() {
    let ix = index_of(vec![
        synth_full("A", "Regular", 0.15),
        synth_full("B", "Regular", 0.16),
    ]);
    assert!(
        ix.similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            10,
            &|_| true
        )
        .unwrap()
        .iter()
        .all(|h| h.key != key_of("A", "Regular"))
    );
}

#[test]
fn vf_family_has_regular_and_bold() {
    // một face VF duy nhất (wght 100–900) → họ có Regular và Bold
    let mut v = synth_full("Public Sans", "Regular", 0.15);
    v.wght_range = Some((100.0, 900.0));
    let ix = index_of(vec![v]);
    let f = ix.family_facts("Public Sans");
    assert!(f.has_regular && f.has_bold);
    // face tĩnh chỉ Regular → thiếu Bold
    let ix2 = index_of(vec![synth_full("Static", "Regular", 0.15)]);
    assert!(!ix2.family_facts("Static").has_bold);
}

#[test]
fn mismatched_or_zero_embedding_is_ignored() {
    let mut a = synth_full("A", "Regular", 0.15);
    let mut b = synth_full("B", "Regular", 0.16);
    let mut c = synth_full("C", "Regular", 0.17);
    a.embedding = Some(vec![1.0, 0.0]);
    b.embedding = Some(vec![1.0, 0.0, 0.0]);
    c.embedding = Some(vec![0.0, 0.0]);
    let ix = index_of(vec![a, b, c]);
    let h = ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            2,
            &|_| true,
        )
        .unwrap();
    assert!(
        h.iter()
            .all(|x| x.source == SimilarSource::Axes && x.distance.is_finite())
    );
}

#[test]
fn group_weight_zero_ignores_group() {
    let q = synth_full("Q", "Regular", 0.15);
    let x = synth_with("X", &[(AxisId::SlantDeg, 12.0)]); // chỉ khác ở nhóm Detail
    let y = synth_with("Y", &[(AxisId::SerifFoot, 2.5)]); // chỉ khác ở nhóm Detail
    let ix = index_of(vec![q, x, y]);
    let w = GroupWeights {
        detail: 0.0,
        ..Default::default()
    };
    let h = ix
        .similar(&key_of("Q", "Regular"), &w, 2, &|_| true)
        .unwrap();
    assert!((h[0].distance - h[1].distance).abs() < 1e-6);
    assert!(h[0].distance < 1e-6);
}

#[test]
fn explanation_same_and_differ_measured_only() {
    let mut a = synth_full("A", "Regular", 0.15);
    let mut b = synth_full("B", "Regular", 0.15);
    for p in [&mut a, &mut b] {
        set_q(p, AxisId::StressDeg, Q::Estimated);
    }
    let ix = index_of(vec![a, b]);
    let h = &ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            1,
            &|_| true,
        )
        .unwrap()[0];
    assert!(!h.same.contains(&AxisId::StressDeg) && !h.differ.contains(&AxisId::StressDeg));
}

#[test]
fn embedding_switches_source() {
    let mut a = synth_full("A", "Regular", 0.15);
    let mut b = synth_full("B", "Regular", 0.16);
    a.embedding = Some(vec![1.0, 0.0]);
    b.embedding = Some(vec![0.0, 1.0]);
    let c = synth_full("C", "Regular", 0.17); // không có embedding
    let ix = index_of(vec![a, b, c]);
    let h = ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            2,
            &|_| true,
        )
        .unwrap();
    let src = |f: &str| {
        h.iter()
            .find(|x| x.key == key_of(f, "Regular"))
            .unwrap()
            .source
    };
    assert_eq!(src("B"), SimilarSource::AxesAndImage);
    assert_eq!(src("C"), SimilarSource::Axes);
}

#[test]
fn non_latin_not_comparable() {
    let ix = index_of(vec![
        synth("CJK", &[]),
        synth_full("B", "Regular", 0.15),
        synth_full("C", "Regular", 0.16),
    ]);
    let cjk = ix
        .profiles()
        .iter()
        .find(|p| p.family == "CJK")
        .unwrap()
        .key
        .clone();
    assert!(matches!(
        ix.similar(&cjk, &GroupWeights::default(), 5, &|_| true),
        Err(IndexError::NotComparable(_))
    ));
    // và CJK không xuất hiện trong kết quả của người khác
    assert!(
        ix.similar(
            &key_of("B", "Regular"),
            &GroupWeights::default(),
            5,
            &|_| true
        )
        .unwrap()
        .iter()
        .all(|h| h.key != cjk)
    );
}

#[test]
fn self_excluded_from_pairs() {
    let ix = index_of(vec![
        synth_full("A", "Regular", 0.15),
        synth_full("B", "Regular", 0.16),
    ]);
    assert!(
        ix.pair_candidates(
            &key_of("A", "Regular"),
            Role::Heading,
            Role::Body,
            false,
            10
        )
        .unwrap()
        .iter()
        .all(|(k, _)| *k != key_of("A", "Regular"))
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "chỉ đo ở --release")]
fn s6_query_50k_under_50ms() {
    let ix = index_of(synthetic_50k(20261016));
    let t = std::time::Instant::now();
    let h = ix
        .similar(
            &key_of("F0", "Regular"),
            &GroupWeights::default(),
            20,
            &|_| true,
        )
        .unwrap();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    eprintln!("S6: {ms:.1} ms");
    assert_eq!(h.len(), 20);
    assert!(ms < 50.0, "{ms} ms");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "chỉ đo ở --release")]
fn s6_pair_candidates_50k_under_50ms() {
    let ix = index_of(synthetic_50k(20261016)); // cùng bộ sinh như test trên, tách thành helper
    let t = std::time::Instant::now();
    let v = ix
        .pair_candidates(
            &key_of("F0", "Regular"),
            Role::Heading,
            Role::Body,
            false,
            20,
        )
        .unwrap();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    eprintln!("S6: {ms:.1} ms");
    assert_eq!(v.len(), 20);
    assert!(ms < 50.0, "{ms} ms");
}

#[test]
fn similar_with_zero_image_weight_is_axes() {
    use fontatlas_index::SimilarOptions;
    let ps: Vec<_> = (0..6)
        .map(|i| {
            let mut p = synth_full(&format!("F{i}"), "Regular", 0.15 + i as f32 * 0.01);
            p.embedding = Some(lcg_unit(16, i));
            p
        })
        .collect();
    let ix = index_of(ps);
    let q = key_of("F0", "Regular");
    let mut o = SimilarOptions {
        weights: GroupWeights::default(),
        embed_weight: Some(0.0),
        k: 3,
    };
    let hits = ix.similar_with(&q, &o, &|_| true).unwrap();
    assert!(!hits.is_empty() && hits.iter().all(|h| h.source == SimilarSource::Axes));
    o.embed_weight = None;
    let hits = ix.similar_with(&q, &o, &|_| true).unwrap();
    assert!(hits.iter().any(|h| h.source == SimilarSource::AxesAndImage));
}

fn image_only_index() -> (fontatlas_index::Index, Vec<fontatlas_index::FaceProfile>) {
    // Q: mọi trục Failed nhưng có ảnh; A..D: có ảnh; E: không ảnh
    let mk = |name: &str, seed: Option<u64>| {
        let mut p = synth(name, &[]);
        p.key = key_of(name, "Regular");
        p.embedding = seed.map(|s| lcg_unit(32, s));
        p
    };
    let ps = vec![
        mk("Q", Some(1)),
        mk("A", Some(1)),
        mk("B", Some(2)),
        mk("C", Some(3)),
        mk("E", None),
    ];
    (index_of(ps.clone()), ps)
}

#[test]
fn image_only_for_non_comparable_query() {
    let (ix, _) = image_only_index();
    let q = key_of("Q", "Regular");
    assert!(matches!(
        ix.similar(&q, &GroupWeights::default(), 5, &|_| true),
        Err(IndexError::NotComparable(_))
    ));
    let hits = ix.similar_image_only(&q, 5, &|_| true).unwrap();
    assert_eq!(hits.len(), 3);
    assert!(
        hits.iter()
            .all(|h| h.source == SimilarSource::Image && h.same.is_empty() && h.differ.is_empty())
    );
    assert!(hits.windows(2).all(|w| w[0].distance <= w[1].distance));
    assert_eq!(hits[0].key, key_of("A", "Regular")); // cùng vector với Q → cos 1
    assert!(hits[0].distance.abs() < 1e-3, "{}", hits[0].distance);
}

#[test]
fn image_only_without_embedding_is_not_comparable() {
    let (ix, _) = image_only_index();
    assert!(matches!(
        ix.similar_image_only(&key_of("E", "Regular"), 5, &|_| true),
        Err(IndexError::NotComparable(_))
    ));
}

#[test]
fn image_only_excludes_self_and_faces_without_image() {
    let (ix, _) = image_only_index();
    let hits = ix
        .similar_image_only(&key_of("Q", "Regular"), 10, &|_| true)
        .unwrap();
    assert!(
        hits.iter()
            .all(|h| h.key != key_of("Q", "Regular") && h.key != key_of("E", "Regular"))
    );
    let only_b = ix
        .similar_image_only(&key_of("Q", "Regular"), 10, &|p| p.family == "B")
        .unwrap();
    assert_eq!(only_b.len(), 1);
}

fn fam_style(family: &str, style: &str, stem: f32, contrast: f32) -> fontatlas_index::FaceProfile {
    let mut p = synth_full(family, style, stem);
    set_value(&mut p, AxisId::Contrast, contrast);
    p
}

fn grouped_index() -> fontatlas_index::Index {
    let mut ps = vec![
        fam_style("Query", "Regular", 0.15, 1.2),
        fam_style("Query", "Bold", 0.26, 1.2),
    ];
    // A gần nhất (3 kiểu), B xa hơn (2 kiểu), C xa nhất (1 kiểu)
    for (i, st) in ["Light", "Regular", "Bold"].iter().enumerate() {
        ps.push(fam_style("A", st, 0.15 + i as f32 * 0.02, 1.25));
    }
    for (i, st) in ["Regular", "Bold"].iter().enumerate() {
        ps.push(fam_style("B", st, 0.15 + i as f32 * 0.02, 2.0));
    }
    ps.push(fam_style("C", "Regular", 0.15, 4.0));
    index_of(ps)
}

#[test]
fn similar_families_group_exclude_own_and_count() {
    let ix = grouped_index();
    let q = key_of("Query", "Regular");
    let o = SimilarOptions {
        weights: GroupWeights::default(),
        embed_weight: Some(0.0),
        k: 0,
    };
    let g = ix.similar_families(&q, &o, &|_| true, 0, 10).unwrap();
    let fams: Vec<&str> = g.iter().map(|x| x.family.as_str()).collect();
    assert_eq!(
        fams,
        ["A", "B", "C"],
        "mỗi họ một mục, không có họ của font đang chọn"
    );
    assert_eq!(g.iter().map(|x| x.count).collect::<Vec<_>>(), [3, 2, 1]);
    assert_eq!(g[0].faces.len(), 3);
    // đại diện = face gần nhất của họ
    let rep_dist = g[0].representative.distance;
    let all = ix
        .similar(&q, &GroupWeights::default(), 20, &|p| p.family == "A")
        .unwrap();
    assert!((all[0].distance - rep_dist).abs() < 1e-6);
    assert_eq!(all[0].key, g[0].representative.key);
}

#[test]
fn similar_families_page_by_family_not_by_face() {
    let ix = grouped_index();
    let q = key_of("Query", "Regular");
    let o = SimilarOptions {
        weights: GroupWeights::default(),
        embed_weight: Some(0.0),
        k: 0,
    };
    let p1 = ix.similar_families(&q, &o, &|_| true, 0, 2).unwrap();
    let p2 = ix.similar_families(&q, &o, &|_| true, 2, 2).unwrap();
    assert_eq!(
        p1.iter().map(|x| x.family.as_str()).collect::<Vec<_>>(),
        ["A", "B"]
    );
    assert_eq!(
        p2.iter().map(|x| x.family.as_str()).collect::<Vec<_>>(),
        ["C"]
    );
    assert!(
        ix.similar_families(&q, &o, &|_| true, 9, 2)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn similar_families_filter_limits_member_list() {
    let ix = grouped_index();
    let q = key_of("Query", "Regular");
    let o = SimilarOptions {
        weights: GroupWeights::default(),
        embed_weight: Some(0.0),
        k: 0,
    };
    let g = ix
        .similar_families(&q, &o, &|p| p.style != "Bold", 0, 10)
        .unwrap();
    assert_eq!(g[0].family, "A");
    assert_eq!(g[0].count, 2);
}

#[test]
fn similar_family_large_family_cannot_starve_other_families() {
    let mut ps = vec![fam_style("Query", "Regular", 0.15, 1.2)];
    for i in 0..3200 {
        ps.push(fam_style("Large", &format!("Style{i}"), 0.15, 1.25));
    }
    for i in 0..5 {
        ps.push(fam_style(
            &format!("Other{i}"),
            "Regular",
            0.15,
            2.0 + i as f32,
        ));
    }
    let ix = index_of(ps);
    let o = SimilarOptions {
        weights: GroupWeights::default(),
        embed_weight: Some(0.0),
        k: 0,
    };
    let q = key_of("Query", "Regular");
    let all = ix.similar_families(&q, &o, &|_| true, 0, 20).unwrap();
    assert_eq!(all.len(), 6);
    let mut paged = Vec::new();
    for off in (0..6).step_by(2) {
        paged.extend(ix.similar_families(&q, &o, &|_| true, off, 2).unwrap());
    }
    assert_eq!(all, paged);
    assert!(all[0].faces.windows(2).all(|p| p[0] < p[1]));
    assert_eq!(
        ix.similar_families(&q, &o, &|_| true, usize::MAX, 2)
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn similar_families_image_only_and_disabled_image() {
    let (ix, _) = image_only_index();
    let mut o = SimilarOptions {
        weights: GroupWeights::default(),
        embed_weight: None,
        k: 0,
    };
    let q = key_of("Q", "Regular");
    let g = ix.similar_families(&q, &o, &|_| true, 0, 20).unwrap();
    assert_eq!(g.len(), 3);
    assert!(
        g.iter()
            .all(|x| x.representative.source == SimilarSource::Image && x.family != "Q")
    );
    o.embed_weight = Some(0.0);
    assert!(matches!(
        ix.similar_families(&q, &o, &|_| true, 0, 20),
        Err(IndexError::NotComparable(_))
    ));
}
