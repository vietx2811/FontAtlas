mod common;
use common::*;
use fontatlas_index::{AxisId, GroupWeights, IndexError, Q, Role, SimilarSource};

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
        .profiles_for_test()
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
    let mut o = SimilarOptions { weights: GroupWeights::default(), embed_weight: Some(0.0), k: 3 };
    let hits = ix.similar_with(&q, &o, &|_| true).unwrap();
    assert!(!hits.is_empty() && hits.iter().all(|h| h.source == SimilarSource::Axes));
    o.embed_weight = None;
    let hits = ix.similar_with(&q, &o, &|_| true).unwrap();
    assert!(hits.iter().any(|h| h.source == SimilarSource::AxesAndImage));
}
