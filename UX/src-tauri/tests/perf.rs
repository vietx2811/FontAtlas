//! Đo thời gian trên thư viện thật (ignored). Chạy:
//! `FONTATLAS_PERF_DIR=/đường/dẫn/google-fonts cargo test -p app --release --test perf -- --ignored --nocapture`
use std::time::{Duration, Instant};

use app_lib::dto::analysis::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{analysis, boot, describe, library, sources};
use app_lib::state::{AppDirs, AppState};

fn timed<T>(label: &str, n: u32, mut f: impl FnMut() -> T) -> T {
    let t = Instant::now();
    let mut out = f();
    for _ in 1..n {
        out = f();
    }
    eprintln!("{label}: {:.1} ms/lần (n={n})", t.elapsed().as_secs_f64() * 1000.0 / n as f64);
    out
}

#[test]
#[ignore]
fn real_library_timings() {
    let Ok(dir) = std::env::var("FONTATLAS_PERF_DIR") else {
        eprintln!("đặt FONTATLAS_PERF_DIR để đo");
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&d.path().join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let t = Instant::now();
    sources::add_source(&st, &dir).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(1800)));
    assert!(wait_rebuilt(&st, Duration::from_secs(600)));
    let total = library::query_faces(&st, &FaceQueryDto { limit: 1, ..Default::default() }).unwrap().total;
    eprintln!("quét + phân tích + dựng Index: {:.1} s, {total} face", t.elapsed().as_secs_f64());
    timed("dựng lại Index (reader riêng)", 1, || app_lib::index_bundle::rebuild_index(&st).unwrap());
    timed("query_faces (trang 100, không lọc)", 20, || library::query_faces(&st, &FaceQueryDto { limit: 100, ..Default::default() }).unwrap());
    timed("query_faces text='sans'", 20, || library::query_faces(&st, &FaceQueryDto { text: Some("sans".into()), limit: 100, ..Default::default() }).unwrap());
    let desc = describe::describe(&st, "ấm tối giản");
    let q = FaceQueryDto { sort: SortKeyDto::Match, axis_filters: desc.filters, limit: 100, ..Default::default() };
    let p = timed("query_faces có lọc trục (describe 'ấm tối giản')", 10, || library::query_faces(&st, &q).unwrap());
    eprintln!("  → {} kết quả", p.total);
    let face = {
        let r = library::query_faces(&st, &FaceQueryDto { text: Some("roboto".into()), limit: 1, ..Default::default() }).unwrap();
        let r = r.items.first().cloned().unwrap_or_else(|| library::query_faces(&st, &FaceQueryDto { limit: 1, ..Default::default() }).unwrap().items[0].clone());
        FaceKeyDto { hash_hex: r.hash_hex, face_index: r.face_index }
    };
    let h = timed("similar k=12", 10, || analysis::similar(&st, &face, &SimilarOptsDto::default()).unwrap());
    eprintln!("  → {} kết quả", h.len());
    let o = PairOptsDto { role: PairRoleDto::Heading, mode: None, project_vi: true, k: 20, collection_id: None, lang: LangDto::Vi };
    let c = timed("pair_candidates k=20 (projectVi)", 5, || analysis::pair_candidates(&st, &face, &o).unwrap());
    eprintln!("  → {} ứng viên", c.len());
    timed("get_face", 10, || analysis::get_face(&st, &face, LangDto::Vi).unwrap());
}
