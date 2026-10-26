//! Plan 4 Task 10: eval nhúng ảnh mọi face của corpus (kể cả instance VF) và đo cùng họ có lọc split.
mod common;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

use common::*;
use fontatlas_embed::{EmbedError, Encode};
use fontatlas_eval::{Corpus, embed_corpus, image_only_same_family, same_family};
use fontatlas_index::{AxisId, FaceKey};

/// Encoder giả: 16 giá trị = trung bình 16 dải ngang của kênh 0 (ảnh khác → vector khác).
struct Bands;
impl Encode for Bands {
    fn raw_dim(&self) -> usize {
        16
    }
    fn encode(&mut self, t: &[Vec<f32>], _: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(t.iter()
            .map(|x| {
                (0..16)
                    .map(|b| {
                        let rows = &x[b * 14 * 224..(b + 1) * 14 * 224];
                        rows.iter().sum::<f32>() / rows.len() as f32 + 1.0 + b as f32 * 1e-3
                    })
                    .collect()
            })
            .collect())
    }
}

#[test]
fn corpus_records_origin_for_every_profile() {
    let c = Corpus::load(&[core_fixture_dir()], true);
    for p in &c.profiles {
        let o = c.origin(&p.key).unwrap_or_else(|| panic!("{:?}", p.key));
        assert!(o.path.exists());
        if p.family == "Public Sans" {
            let w = o
                .loc
                .as_ref()
                .unwrap()
                .iter()
                .find(|x| x.0 == "wght")
                .unwrap()
                .1;
            assert_eq!(w, p.weight_class.unwrap() as f32);
        }
    }
}

#[test]
fn embed_corpus_covers_every_face_and_vf_instances_differ() {
    let c = Corpus::load(&[core_fixture_dir()], true);
    let (embs, skipped) = embed_corpus(&c, &mut Bands, &|v| Some(v.to_vec())).unwrap();
    assert_eq!(embs.len() + skipped, c.profiles.len());
    assert_eq!(skipped, 0, "fixture đều có chữ Latin");
    let reg = c.resolve("Public Sans", "Regular").unwrap();
    let bold = c.resolve("Public Sans", "Bold").unwrap();
    assert_ne!(
        embs[&reg.key], embs[&bold.key],
        "instance VF phải render ở đúng độ đậm"
    );
}

#[test]
fn same_family_filters_queries_and_gallery() {
    let c = Corpus {
        profiles: vec![
            synth_full("A", "Regular", 0.15),
            synth_full("A", "Bold", 0.24),
            synth_with("B", &[(AxisId::Contrast, 4.0)]),
            synth_with("C", &[(AxisId::Width, 1.4)]),
        ],
        ..Default::default()
    };
    let ix = index_with_test_reference(&c);
    let all = same_family(&ix, &c, &|_| true, &|_| true);
    assert_eq!((all.queries, all.r1), (2, 1.0));
    // query chỉ họ A Regular; gallery bỏ A Bold → họ A không còn ≥ 2 face trong gallery → không có truy vấn
    let q = same_family(
        &ix,
        &c,
        &|p| p.style == "Regular" && p.family == "A",
        &|p| p.style != "Bold",
    );
    assert_eq!(q.queries, 0);
    let q = same_family(
        &ix,
        &c,
        &|p| p.family == "A" && p.style == "Regular",
        &|_| true,
    );
    assert_eq!((q.queries, q.keys.len()), (1, 1));
}

#[test]
fn image_only_uses_same_queries_and_counts_missing_image_as_miss() {
    let c = Corpus {
        profiles: vec![
            synth_full("A", "Regular", 0.15),
            synth_full("A", "Bold", 0.24),
            synth_full("B", "Regular", 0.15),
            synth_full("B", "Bold", 0.24),
        ],
        ..Default::default()
    };
    let e = |x: f32| vec![x, 1.0 - x];
    let embs: HashMap<FaceKey, Vec<f32>> = [
        (key_of("A", "Regular"), e(0.9)),
        (key_of("A", "Bold"), e(0.8)),
        (key_of("B", "Regular"), e(0.1)),
    ]
    .into_iter()
    .collect();
    let queries = vec![key_of("A", "Regular"), key_of("B", "Bold")];
    let r = image_only_same_family(&c, &embs, &queries, &|_| true);
    assert_eq!((r.queries, r.r1, r.r5), (2, 0.5, 0.5)); // B Bold không có ảnh → trượt
}
