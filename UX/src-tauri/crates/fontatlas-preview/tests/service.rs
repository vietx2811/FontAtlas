mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_preview::*;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

struct MapLocator(HashMap<(String, u32), PathBuf>);
impl FaceLocator for MapLocator {
    fn locate(&self, h: &str, i: u32) -> Option<PathBuf> { self.0.get(&(h.to_string(), i)).cloned() }
}
fn hash_of(p: &std::path::Path) -> String { FontFile::open(p).unwrap().hash_hex() }
fn svc_with(budgets: Budgets, entries: &[(PathBuf, u32)]) -> (tempfile::TempDir, PreviewService<MapLocator>) {
    let d = tempfile::tempdir().unwrap();
    let cache = PreviewCache::open(&d.path().join("p.db"), budgets).unwrap();
    let map = entries.iter().map(|(p, i)| ((hash_of(p), *i), p.clone())).collect();
    (d, PreviewService::new(cache, MapLocator(map)))
}
fn svc(entries: &[(PathBuf, u32)]) -> (tempfile::TempDir, PreviewService<MapLocator>) { svc_with(Budgets::DEFAULT, entries) }
fn req(p: &std::path::Path, i: u32, g: Option<u64>) -> PreviewRequest {
    PreviewRequest { hash_hex: hash_of(p), face_index: i, px: 48, text: "Ăn phở".into(), generation: g }
}

#[test]
fn get_renders_once_then_hits_cache() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0)]);
    let a = s.get(&req(&p, 0, None)).unwrap();
    let b = s.get(&req(&p, 0, None)).unwrap();
    assert_eq!(a, b);
    assert_eq!(s.render_count(), 1);
    assert_eq!(&a[1..4], b"PNG");
}

#[test]
fn bump_generation_is_monotonic() {
    let (_d, s) = svc(&[]);
    assert_eq!(s.generation(), 0);
    assert_eq!(s.bump_generation(), 1);
    assert_eq!(s.bump_generation(), 2);
    assert_eq!(s.generation(), 2);
}

#[test]
fn stale_generation_rejected_before_render() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0)]);
    for _ in 0..5 { s.bump_generation(); }
    let e = s.get(&req(&p, 0, Some(4))).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("stale_generation", 410));
    assert_eq!(s.render_count(), 0);
    assert!(s.get(&req(&p, 0, Some(5))).is_ok());
    assert!(s.get(&req(&p, 0, Some(6))).is_ok(), "thế hệ mới hơn (đã xin nhưng chưa thấy) vẫn hợp lệ");
    assert!(s.get(&req(&p, 0, None)).is_ok(), "không có g → không kiểm thế hệ");
}

#[test]
fn unknown_face_is_not_found() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[]);
    assert_eq!(s.get(&req(&p, 0, None)).unwrap_err().code(), "not_found");
    let (_d2, s2) = svc(&[(p.clone(), 7)]); // locator trả path nhưng file chỉ có face 0
    let mut r = req(&p, 0, None);
    r.face_index = 7;
    assert_eq!(s2.get(&r).unwrap_err().code(), "not_found");
}

#[test]
fn changed_file_is_stale_file() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("f.ttf");
    std::fs::copy(core_fixture("roboto.ttf"), &path).unwrap();
    let (_d, s) = svc(&[(path.clone(), 0)]);
    let r = req(&path, 0, None);
    std::fs::copy(core_fixture("open_sans.ttf"), &path).unwrap(); // file đổi sau khi quét
    let e = s.get(&r).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("stale_file", 409));
}

#[test]
fn prewarm_reads_each_file_once_and_fills_cache() {
    let p = fixture("pair.ttc");
    let (_d, s) = svc(&[(p.clone(), 0), (p.clone(), 1)]);
    let rs = s.prewarm(&[req(&p, 0, None), req(&p, 1, None)]);
    assert!(!rs.is_empty() && rs.iter().all(|r| r.is_ok()));
    assert_eq!((s.file_reads(), s.render_count()), (1, 2));
    s.get(&req(&p, 1, None)).unwrap();
    assert_eq!(s.render_count(), 2, "đã có trong cache");
}

#[test]
fn prewarm_skips_cached_and_stale_and_keeps_order() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0)]);
    s.get(&req(&p, 0, None)).unwrap();
    for _ in 0..9 { s.bump_generation(); }
    let rs = s.prewarm(&[req(&p, 0, None), req(&p, 0, Some(1))]);
    assert!(rs[0].is_ok());
    assert_eq!(rs[1].as_ref().unwrap_err().code(), "stale_generation");
    assert_eq!((s.render_count(), s.file_reads()), (1, 1));
}

#[test]
fn tiny_budget_still_serves() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc_with(Budgets { default_bytes: 1, custom_bytes: 1 }, &[(p.clone(), 0)]);
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert_eq!(s.render_count(), 2, "cache không giữ được → render lại, vẫn phục vụ");
}

#[test]
fn cache_errors_degrade_to_render() {
    let p = core_fixture("roboto.ttf");
    let (d, s) = svc(&[(p.clone(), 0)]);
    // làm hỏng cache lúc đang chạy: bảng biến mất dưới chân service
    rusqlite::Connection::open(d.path().join("p.db")).unwrap().execute_batch("DROP TABLE previews;").unwrap();
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert_eq!(s.render_count(), 2);
}

#[test]
fn concurrent_get_and_prewarm() {
    let p = fixture("pair.ttc");
    let q = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0), (p.clone(), 1), (q.clone(), 0)]);
    let s = Arc::new(s);
    let reqs = vec![req(&p, 0, None), req(&p, 1, None), req(&q, 0, None)];
    let handles: Vec<_> = (0..8).map(|t| {
        let (s, reqs) = (Arc::clone(&s), reqs.clone());
        std::thread::spawn(move || {
            if t % 2 == 0 { s.prewarm(&reqs).into_iter().all(|r| r.is_ok()) }
            else { reqs.iter().map(|r| s.get(r).map(|png| png.len())).all(|r| r.is_ok()) }
        })
    }).collect();
    assert!(handles.into_iter().all(|h| h.join().unwrap()));
    let a = s.get(&reqs[1]).unwrap();
    assert_eq!(a, s.get(&reqs[1]).unwrap());
}

// ---- font trên ổ cloud: hook before_read chạy trước khi MỞ file (cache hit thì không chạm) ----

use std::sync::atomic::{AtomicUsize, Ordering};

struct Hooked { inner: MapLocator, calls: Arc<AtomicUsize>, verdict: fn() -> Result<(), PreviewError> }
impl FaceLocator for Hooked {
    fn locate(&self, h: &str, i: u32) -> Option<PathBuf> { self.inner.locate(h, i) }
    fn before_read(&self, _p: &std::path::Path) -> Result<(), PreviewError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        (self.verdict)()
    }
}
fn hooked(p: &std::path::Path, verdict: fn() -> Result<(), PreviewError>) -> (tempfile::TempDir, PreviewService<Hooked>, Arc<AtomicUsize>) {
    let d = tempfile::tempdir().unwrap();
    let cache = PreviewCache::open(&d.path().join("p.db"), Budgets::DEFAULT).unwrap();
    let map = [((hash_of(p), 0), p.to_path_buf())].into_iter().collect();
    let calls = Arc::new(AtomicUsize::new(0));
    (d, PreviewService::new(cache, Hooked { inner: MapLocator(map), calls: Arc::clone(&calls), verdict }), calls)
}

#[test]
fn before_read_runs_once_per_cache_miss_and_never_on_a_hit() {
    let p = core_fixture("roboto.ttf");
    let (_d, s, calls) = hooked(&p, || Ok(()));
    s.get(&req(&p, 0, None)).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    s.get(&req(&p, 0, None)).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1, "cache hit: không được chạm file, nên không có hook");
}

#[test]
fn before_read_failure_is_returned_and_nothing_is_read_or_cached() {
    let p = core_fixture("roboto.ttf");
    let (_d, s, calls) = hooked(&p, || Err(PreviewError::CloudTimeout));
    let e = s.get(&req(&p, 0, None)).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("cloud_timeout", 504));
    assert_eq!(s.render_count(), 0);
    let e = s.get(&req(&p, 0, None)).unwrap_err(); // không cache lỗi: lần sau thử lại
    assert_eq!(e.code(), "cloud_timeout");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn cloud_unavailable_is_502_with_its_own_code() {
    let e = PreviewError::CloudUnavailable("offline".into());
    assert_eq!((e.code(), e.http_status()), ("cloud_unavailable", 502));
    assert!(e.to_string().contains("offline"));
}
