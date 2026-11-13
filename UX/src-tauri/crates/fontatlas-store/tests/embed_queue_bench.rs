//! D14: đo hàng đợi embedding trên thư viện tổng hợp (chạy tay, không có ngưỡng thời gian):
//!   cargo test -p fontatlas-store --release --test embed_queue_bench -- --ignored --nocapture
//! Claim theo lô BATCH = 4 như worker thật; dữ liệu nằm trong thư mục tạm.
mod common;
use common::*;
use std::time::Instant;

const RAW: &str = "bench-raw";
/// Bằng `fontatlas_embed::BATCH`.
const BATCH: usize = 4;

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
