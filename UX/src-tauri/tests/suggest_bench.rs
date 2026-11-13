//! Đo thời gian tầng service của gợi ý Desk trên thư viện lớn dựng từ 6 font mẫu (không dùng font thật của người dùng).
//! Mỗi bản sao thêm vài byte cuối tệp (bảng font bỏ qua phần thừa) để mỗi face có hash riêng.
//!   FA_BENCH_N=5000 cargo test --release -p app --test suggest_bench -- --ignored --nocapture
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use app_lib::dto::library::*;
use app_lib::dto::suggest::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, library, sources, suggest, suggest_pair};
use app_lib::state::{AppDirs, AppState};

const SIX: [&str; 6] = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf", "public_sans_vf.ttf"];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn pct(mut v: Vec<f64>, q: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn time<F: FnMut()>(mut f: F) -> (f64, f64) {
    f();
    let xs: Vec<f64> = (0..9)
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
fn service_level_suggest_bench() {
    let n: usize = std::env::var("FA_BENCH_N").ok().and_then(|s| s.parse().ok()).unwrap_or(5000);
    let d = tempfile::tempdir().unwrap();
    let st: Arc<AppState> = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = d.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..n {
        let name = SIX[i % 6];
        let mut bytes = std::fs::read(fixture(name)).unwrap();
        bytes.extend_from_slice(&(i as u64).to_le_bytes());
        let (stem, ext) = name.split_once('.').unwrap();
        std::fs::write(dir.join(format!("{stem}_{i}.{ext}")), bytes).unwrap();
    }
    let t = Instant::now();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(3600)));
    assert!(wait_rebuilt(&st, Duration::from_secs(600)));
    println!("dựng thư viện {n} face: {:.1}s", t.elapsed().as_secs_f64());

    let q = FaceQueryDto { text: Some("Playfair Display".into()), limit: 1, ..Default::default() };
    let r = library::query_faces(&st, &q).unwrap().items.into_iter().next().unwrap();
    let a = FaceKeyDto { hash_hex: r.hash_hex.clone(), face_index: r.face_index };
    let body = RoleRefDto { name: "body".into(), profile: None, size: None };

    let (p50, p95) = time(|| {
        suggest::role_fit_faces(&st, &body, &RoleFitListOptsDto { limit: 24, ..Default::default() }).unwrap();
    });
    println!("N={n} role_fit_faces(body, 24)        p50={p50:8.1}ms p95={p95:8.1}ms");
    let req = SuggestPartnersReqDto { limit: 24, ..Default::default() };
    let (p50, p95) = time(|| {
        suggest_pair::suggest_partners(&st, &a, &req).unwrap();
    });
    println!("N={n} suggest_partners(any style, 24) p50={p50:8.1}ms p95={p95:8.1}ms");
    let dreq = DeskSuggestReqDto { source: "similar".into(), anchor: Some(a.clone()), ..Default::default() };
    let (p50, p95) = time(|| {
        suggest_pair::desk_suggest(&st, &dreq).unwrap();
    });
    println!("N={n} desk_suggest(similar)           p50={p50:8.1}ms p95={p95:8.1}ms");
    let creq = SuggestCombosReqDto { k: 6, seed: 7, ..Default::default() };
    let (p50, p95) = time(|| {
        suggest_pair::suggest_combos(&st, &creq).unwrap();
    });
    println!("N={n} suggest_combos(k=6)             p50={p50:8.1}ms p95={p95:8.1}ms");
}
