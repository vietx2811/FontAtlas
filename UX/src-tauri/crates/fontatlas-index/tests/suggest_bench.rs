//! Bench nền cho gợi ý Desk (G4–G6): thời gian `pair_candidates_gated` và `similar_families`
//! ở N = 2k/5k/10k face. Không phải test đúng/sai (không có ngưỡng thời gian, tránh test phụ thuộc máy):
//!   cargo test --release -p fontatlas-index --test suggest_bench -- --ignored --nocapture
mod common;
use common::*;
use fontatlas_index::{FaceProfile, GroupWeights, Role, SimilarOptions};
use std::time::Instant;

fn corpus(n: usize) -> Vec<FaceProfile> {
    let mut v = synthetic_50k(20261112);
    v.truncate(n);
    for (i, p) in v.iter_mut().enumerate() {
        // ~4 face mỗi họ, vài face thiếu tiếng Việt, một phần có embedding: gần thư viện thật hơn 1 face/họ.
        p.family = format!("Fam{}", i / 4);
        p.weight_class = Some([300, 400, 700, 900][i % 4]);
        if i % 7 == 3 {
            p.vi_missing = 1 + i % 50;
        }
        if i % 3 != 0 {
            p.embedding = Some((0..8).map(|j| (((i * 31 + j * 17) % 97) as f32) / 97.0 - 0.5).collect());
        }
    }
    v
}

fn pct(mut v: Vec<f64>, q: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn time<F: FnMut()>(mut f: F) -> (f64, f64) {
    f(); // làm nóng
    let xs: Vec<f64> = (0..15)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    (pct(xs.clone(), 0.5), pct(xs, 0.95))
}

#[test]
#[ignore = "bench: chạy tay với --release --nocapture"]
fn suggest_bench() {
    for n in [2_000usize, 5_000, 10_000] {
        let ix = index_of(corpus(n));
        let q = ix.profiles()[18].key.clone();
        for (label, vi) in [("pair heading→body", false), ("pair heading→body vi", true)] {
            let (p50, p95) = time(|| {
                ix.pair_candidates_gated(&q, Role::Heading, Role::Body, vi, 60, &|_| true, None).unwrap();
            });
            println!("N={n:>6} {label:<24} p50={p50:7.2}ms p95={p95:7.2}ms");
        }
        let o = SimilarOptions { weights: GroupWeights::default(), embed_weight: None, k: 0 };
        let (p50, p95) = time(|| {
            ix.similar_families(&q, &o, &|_| true, 0, 60).unwrap();
        });
        println!("N={n:>6} {:<24} p50={p50:7.2}ms p95={p95:7.2}ms", "similar_families");
    }
}
