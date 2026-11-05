//! Plan 4 Task 10: một đường embedding qua FaceProfile.embedding — int8 trong Index, xếp lại f32.
mod common;
use std::collections::HashMap;
use std::time::Instant;

use common::*;
use fontatlas_index::{
    AxisId, FaceKey, FaceProfile, GroupWeights, MAX_EMBED_WEIGHT, RERANK_CANDIDATES, Role,
    SimilarSource,
};

const _: () = assert!(RERANK_CANDIDATES >= 20 && MAX_EMBED_WEIGHT < 1.0);

fn unit(seed: u64) -> Vec<f32> {
    lcg_unit(128, seed)
}

fn with_emb(mut p: FaceProfile, seed: u64) -> FaceProfile {
    p.embedding = Some(unit(seed));
    p
}

fn exact_of(ps: &[FaceProfile]) -> HashMap<FaceKey, Vec<f32>> {
    ps.iter()
        .filter_map(|p| Some((p.key.clone(), p.embedding.clone()?)))
        .collect()
}

fn keys(h: &[fontatlas_index::SimilarHit]) -> Vec<FaceKey> {
    h.iter().map(|x| x.key.clone()).collect()
}

#[test]
fn int8_then_exact_rerank_equals_f32_ranking() {
    // mọi trục giống nhau → thứ hạng chỉ do ảnh
    let ps: Vec<FaceProfile> = (0..2000)
        .map(|i| with_emb(synth_full(&format!("F{i}"), "Regular", 0.15), i))
        .collect();
    let exact = exact_of(&ps);
    let mut ix = index_of(ps);
    ix.set_exact_source(Box::new(MemExact(exact.clone())));
    let q = key_of("F0", "Regular");
    let got = keys(
        &ix.similar(&q, &GroupWeights::default(), 20, &|_| true)
            .unwrap(),
    );
    let mut truth: Vec<(f32, FaceKey)> = exact
        .iter()
        .filter(|(k, _)| **k != q)
        .map(|(k, v)| (dot(v, &exact[&q]), k.clone()))
        .collect();
    truth.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    assert_eq!(
        got,
        truth.into_iter().take(20).map(|x| x.1).collect::<Vec<_>>()
    );
}

#[test]
fn int8_alone_close_to_f32_ranking() {
    let ps: Vec<FaceProfile> = (0..2000)
        .map(|i| with_emb(synth_full(&format!("F{i}"), "Regular", 0.15), i))
        .collect();
    let exact = exact_of(&ps);
    let ix = index_of(ps); // không có nguồn f32
    let q = key_of("F0", "Regular");
    let got = keys(
        &ix.similar(&q, &GroupWeights::default(), 20, &|_| true)
            .unwrap(),
    );
    let mut truth: Vec<(f32, FaceKey)> = exact
        .iter()
        .filter(|(k, _)| **k != q)
        .map(|(k, v)| (dot(v, &exact[&q]), k.clone()))
        .collect();
    truth.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let top: Vec<FaceKey> = truth.into_iter().take(20).map(|x| x.1).collect();
    let overlap = got.iter().filter(|k| top.contains(k)).count();
    assert!(overlap >= 18, "int8 top-20 trùng {overlap}/20 với f32");
}

#[test]
fn faces_without_image_use_axes_only() {
    let a = with_emb(synth_full("A", "Regular", 0.15), 1);
    let ix = index_of(vec![a, synth_full("B", "Regular", 0.16)]);
    let h = ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            1,
            &|_| true,
        )
        .unwrap();
    assert_eq!(h[0].source, SimilarSource::Axes);
}

#[test]
fn wrong_dim_or_zero_embedding_ignored() {
    let mut a = synth_full("A", "Regular", 0.15);
    a.embedding = Some(vec![0.0; 128]);
    let mut b = synth_full("B", "Regular", 0.16);
    b.embedding = Some(vec![1.0; 64]);
    let h = index_of(vec![a, b])
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            1,
            &|_| true,
        )
        .unwrap();
    assert!(h[0].distance.is_finite() && h[0].source == SimilarSource::Axes);
}

/// Thư viện có ảnh cho một phần font; query không có ảnh → kết quả y hệt Index không có ảnh nào.
#[test]
fn query_without_image_identical_to_axes_only_index() {
    let base: Vec<FaceProfile> = (0..60)
        .map(|i| synth_with(&format!("F{i}"), &[(AxisId::Width, 0.8 + i as f32 * 0.01)]))
        .collect();
    let mixed: Vec<FaceProfile> = base
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, p)| {
            if i > 0 && i % 2 == 0 {
                with_emb(p, i as u64)
            } else {
                p
            }
        })
        .collect();
    let q = key_of("F0", "Regular");
    let w = GroupWeights::default();
    let plain = index_of(base).similar(&q, &w, 10, &|_| true).unwrap();
    let mut ix = index_of(mixed.clone());
    ix.set_exact_source(Box::new(MemExact(exact_of(&mixed))));
    assert_eq!(ix.similar(&q, &w, 10, &|_| true).unwrap(), plain);
}

/// "Giống ở / Khác ở" chỉ từ trục: ảnh làm đổi thứ hạng, không đổi lời giải thích của từng cặp.
#[test]
fn explanation_comes_from_axes_not_image() {
    let ps: Vec<FaceProfile> = (0..30)
        .map(|i| {
            synth_with(
                &format!("F{i}"),
                &[(AxisId::Contrast, 1.0 + i as f32 * 0.1)],
            )
        })
        .collect();
    let with: Vec<FaceProfile> = ps
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, p)| with_emb(p, i as u64))
        .collect();
    let q = key_of("F0", "Regular");
    let w = GroupWeights::default();
    let a = index_of(ps).similar(&q, &w, 29, &|_| true).unwrap();
    let b = index_of(with).similar(&q, &w, 29, &|_| true).unwrap();
    for hb in &b {
        assert_eq!(hb.source, SimilarSource::AxesAndImage);
        let ha = a.iter().find(|x| x.key == hb.key).unwrap();
        assert_eq!(
            (&ha.same, &ha.differ),
            (&hb.same, &hb.differ),
            "{:?}",
            hb.key
        );
    }
}

#[test]
fn embed_weight_capped_so_axes_never_vanish() {
    let mut ix = index_of(vec![
        with_emb(synth_full("A", "Regular", 0.15), 1),
        with_emb(synth_full("B", "Regular", 0.30), 2),
    ]);
    ix.set_embed_weight(5.0);
    assert_eq!(ix.embed_weight(), MAX_EMBED_WEIGHT);
    ix.set_embed_weight(f32::NAN);
    assert_eq!(ix.embed_weight(), fontatlas_index::EMBED_WEIGHT);
    ix.set_embed_weight(0.0);
    // w = 0: khoảng cách = khoảng cách trục thuần
    let d0 = ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            1,
            &|_| true,
        )
        .unwrap()[0]
        .distance;
    let plain = index_of(vec![
        synth_full("A", "Regular", 0.15),
        synth_full("B", "Regular", 0.30),
    ])
    .similar(
        &key_of("A", "Regular"),
        &GroupWeights::default(),
        1,
        &|_| true,
    )
    .unwrap()[0]
        .distance;
    assert!((d0 - plain).abs() < 1e-6, "{d0} vs {plain}");
}

#[test]
fn f32_dropped_from_profiles_after_index_build() {
    let ix = index_of(vec![
        with_emb(synth_full("A", "Regular", 0.15), 1),
        synth_full("B", "Regular", 0.16),
    ]);
    assert!(
        ix.profile(&key_of("A", "Regular"))
            .unwrap()
            .embedding
            .is_none()
    );
    assert!(ix.has_image(&key_of("A", "Regular")));
    assert!(!ix.has_image(&key_of("B", "Regular")));
}

/// Pairing: cos ảnh chỉ là tie-break (rank_key bước 5), không đổi cổng.
#[test]
fn pair_candidates_break_ties_by_image_cos() {
    let q = with_emb(synth_full("Q", "Regular", 0.15), 7);
    let mut near = synth_full("Near", "Regular", 0.15);
    let mut far = synth_full("Far", "Regular", 0.15);
    near.embedding = q.embedding.clone(); // cos = 1
    far.embedding = Some(q.embedding.as_ref().unwrap().iter().map(|x| -x).collect()); // cos = −1
    let plain = index_of(vec![
        q.clone(),
        synth_full("Near", "Regular", 0.15),
        synth_full("Far", "Regular", 0.15),
    ]);
    let ix = index_of(vec![q, near, far]);
    let qk = key_of("Q", "Regular");
    let a = ix
        .pair_candidates(&qk, Role::Heading, Role::Body, false, 2)
        .unwrap();
    let b = plain
        .pair_candidates(&qk, Role::Heading, Role::Body, false, 2)
        .unwrap();
    assert_eq!(a[0].0, key_of("Near", "Regular"));
    assert_eq!(a[1].0, key_of("Far", "Regular"));
    // cổng và nhãn giữ nguyên
    for (k, x) in &a {
        let y = &b.iter().find(|z| &z.0 == k).unwrap().1;
        assert_eq!(
            (x.gate, &x.labels, &x.clashes),
            (y.gate, &y.labels, &y.clashes)
        );
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "chỉ đo ở --release")]
fn s6_similar_with_images_50k_under_50ms() {
    let mut ps = synthetic_50k(20261016);
    for (i, p) in ps.iter_mut().enumerate() {
        p.embedding = Some(unit(i as u64));
    }
    let exact = exact_of(&ps);
    let mut ix = index_of(ps);
    ix.set_exact_source(Box::new(MemExact(exact)));
    let q = key_of("F0", "Regular");
    ix.similar(&q, &GroupWeights::default(), 20, &|_| true)
        .unwrap(); // khởi động
    let t = Instant::now();
    let h = ix
        .similar(&q, &GroupWeights::default(), 20, &|_| true)
        .unwrap();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    eprintln!("S6 có ảnh: {ms:.1} ms");
    assert_eq!(h.len(), 20);
    assert!(h.iter().all(|x| x.source == SimilarSource::AxesAndImage));
    assert!(ms < 50.0, "{ms} ms");
}

/// Vector có cos đúng `c` với `q` (q, u đơn vị; u được trực giao hoá).
fn at_cos(q: &[f32], u: &[f32], c: f32) -> Vec<f32> {
    let d = dot(q, u);
    let mut o: Vec<f32> = u.iter().zip(q).map(|(x, y)| x - d * y).collect();
    let n = dot(&o, &o).sqrt();
    o.iter_mut().for_each(|x| *x /= n);
    q.iter()
        .zip(&o)
        .map(|(a, b)| c * a + (1.0 - c * c).sqrt() * b)
        .collect()
}

/// Query có ảnh: ứng viên chưa có ảnh (đang chờ embed) không được chen lên trên nhờ thang khoảng cách khác.
#[test]
fn imaged_query_unembedded_candidate_does_not_jump_ahead() {
    let q = unit(1);
    let mut ps = vec![];
    let mut a = synth_full("A", "Regular", 0.15);
    a.embedding = Some(q.clone());
    ps.push(a);
    let mut x = synth_full("X", "Regular", 0.15); // trục y hệt, ảnh khá giống (cos 0,7)
    x.embedding = Some(at_cos(&q, &unit(2), 0.7));
    ps.push(x);
    ps.push(synth_full("Y", "Regular", 0.16)); // trục gần, CHƯA có ảnh
    for i in 0..40 {
        ps.push(with_emb(
            synth_full(&format!("F{i}"), "Regular", 0.15),
            100 + i,
        )); // ảnh ngẫu nhiên, cos ≈ 0
    }
    let exact = exact_of(&ps);
    let mut ix = index_of(ps);
    ix.set_exact_source(Box::new(MemExact(exact)));
    let h = ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            3,
            &|_| true,
        )
        .unwrap();
    assert_eq!(h[0].key, key_of("X", "Regular"), "{:?}", keys(&h));
    let y = ix
        .similar(
            &key_of("A", "Regular"),
            &GroupWeights::default(),
            42,
            &|_| true,
        )
        .unwrap();
    let y = y.iter().find(|z| z.key == key_of("Y", "Regular")).unwrap();
    assert_eq!(y.source, SimilarSource::Axes);
}

/// Nguồn f32 trả vector khác số chiều của Index (vd. nhầm vector thô 768) → bỏ qua, giữ kết quả int8.
#[test]
fn exact_vectors_of_wrong_dim_are_ignored() {
    let ps: Vec<FaceProfile> = (0..200)
        .map(|i| with_emb(synth_full(&format!("F{i}"), "Regular", 0.15), i))
        .collect();
    let wrong: HashMap<FaceKey, Vec<f32>> = ps
        .iter()
        .enumerate()
        .map(|(i, p)| (p.key.clone(), lcg_unit(64, 9000 + i as u64)))
        .collect();
    let q = key_of("F0", "Regular");
    let plain = keys(
        &index_of(ps.clone())
            .similar(&q, &GroupWeights::default(), 10, &|_| true)
            .unwrap(),
    );
    let mut ix = index_of(ps);
    ix.set_exact_source(Box::new(MemExact(wrong)));
    assert_eq!(
        keys(
            &ix.similar(&q, &GroupWeights::default(), 10, &|_| true)
                .unwrap()
        ),
        plain
    );
}
