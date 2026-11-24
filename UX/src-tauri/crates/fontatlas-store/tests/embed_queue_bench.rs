//! D14: đo hàng đợi embedding trên thư viện tổng hợp (chạy tay, không có ngưỡng thời gian):
//!   cargo test -p fontatlas-store --release --test embed_queue_bench -- --ignored --nocapture
//! Claim theo lô BATCH = 4 như worker thật; dữ liệu nằm trong thư mục tạm.
mod common;
use common::*;
use std::time::Instant;

const RAW: &str = "bench-raw";
/// Bằng `fontatlas_embed::BATCH`.
const BATCH: usize = 4;

/// Chi phí SQLite của bước xếp lại f32 trong `similar_families` (k = cả thư viện → đọc vector của MỌI ứng viên).
#[test]
#[ignore = "bench: chạy tay với --release --nocapture"]
fn rerank_fetch_of_every_candidate() {
    use fontatlas_store::{EmbedReader, FaceKey};
    for n in [5_000usize, 20_000] {
        let (d, s) = synthetic_library(n);
        s.conn()
            .execute_batch(
                "INSERT INTO embeddings(hash,face_index,model_version,source,dim,dtype,vec,created_at)
                 SELECT hash, 0, 'mv', 'local', 128, 'f32', randomblob(512), 0 FROM files;",
            )
            .unwrap();
        let keys: Vec<FaceKey> = {
            let mut st = s.conn().prepare("SELECT hash FROM files").unwrap();
            st.query_map([], |r| r.get::<_, Vec<u8>>(0))
                .unwrap()
                .map(|h| FaceKey { hash: h.unwrap().try_into().unwrap(), face_index: 0 })
                .collect()
        };
        let r = EmbedReader::open(&d.path().join("db")).unwrap();
        let mut ms: Vec<f64> = (0..15)
            .map(|_| {
                let t = Instant::now();
                let got = r.embeddings_f32("mv", &keys).unwrap();
                assert_eq!(got.iter().flatten().count(), keys.len());
                t.elapsed().as_secs_f64() * 1e3
            })
            .collect();
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("N={n:>6} fetch {} vector: p50={:.1}ms p95={:.1}ms", keys.len(), ms[7], ms[13]);
    }
}

#[test]
#[ignore = "bench: chạy tay với --release --nocapture"]
fn claim_all_in_batches() {
    for n in [2_000usize, 5_000, 10_000, 20_000] {
        let (_d, mut s) = synthetic_library(n);
        let t = Instant::now();
        let st = s.enqueue_embeddings(RAW).unwrap();
        let enq = t.elapsed();
        let t = Instant::now();
        let (mut claimed, mut calls) = (0usize, 0usize);
        loop {
            let j = s.claim_embedding_batch(RAW, BATCH).unwrap();
            calls += 1;
            if j.is_empty() {
                break;
            }
            claimed += j.len();
        }
        let el = t.elapsed();
        println!(
            "N={n:>6} pending={:>6} enqueue={:>8.1?} claim_all={:>9.1?} ({calls} lần gọi, {} lần quét, {claimed} face)",
            st.pending,
            enq,
            el,
            s.embed_claim_scans()
        );
    }
}
