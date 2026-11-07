//! Đo thời gian tải model thật từ nguồn chính thức trong manifest (không dùng trong test/CI).
//! cargo run --release -p fontatlas-embed --example bench_download -- <thư-mục-tạm-rỗng>
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use fontatlas_embed::model::{ModelStore, bundled_manifest};

fn main() {
    let dir = std::env::args().nth(1).expect("cần thư mục tạm");
    let m = bundled_manifest();
    println!("nguồn: {}", m.source.url);
    let store = ModelStore::new(dir.into(), m);
    let t = Instant::now();
    let last = std::cell::Cell::new(0u64);
    let res = store.ensure_resumable(
        &|p| {
            let mb = p.done / (32 << 20);
            if mb != last.get() {
                last.set(mb);
                eprintln!("{:>6.1}s {:>5} / {} MB  {:.1} MB/s", t.elapsed().as_secs_f64(), p.done >> 20, p.total >> 20, p.bytes_per_sec / 1e6);
            }
        },
        &AtomicBool::new(false),
    );
    let secs = t.elapsed().as_secs_f64();
    match res {
        Ok(p) => {
            let n = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            println!("READY {} bytes trong {:.1}s = {:.1} MB/s (gồm kiểm SHA)", n, secs, n as f64 / 1e6 / secs);
        }
        Err(e) => println!("LỖI sau {:.1}s: {e}", secs),
    }
}
