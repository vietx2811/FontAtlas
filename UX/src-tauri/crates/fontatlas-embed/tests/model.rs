use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use fontatlas_embed::{
    DownloadProgress, EmbedError, Manifest, ModelStatus, ModelStore, disk_free, make_manifest,
    sha256_file,
};

/// Server tải giả ở 127.0.0.1:0; dừng khi drop.
struct Srv {
    server: Arc<tiny_http::Server>,
    count: Arc<AtomicUsize>,
    handle: Option<JoinHandle<()>>,
}

impl Srv {
    fn requests(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl Drop for Srv {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Đọc chậm: trả từng chunk, ngủ `delay` giữa các chunk.
struct Slow {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    delay: Duration,
}

impl Read for Slow {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        if self.pos > 0 {
            std::thread::sleep(self.delay);
        }
        let n = buf.len().min(self.chunk).min(self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// safetensors F32 theo đúng thứ tự cho trước (offset liền nhau), header đệm space cho chia hết 8.
fn fake_safetensors(tensors: &[(String, Vec<usize>, Vec<f32>)]) -> Vec<u8> {
    let mut hdr = serde_json::Map::new();
    hdr.insert("__metadata__".into(), serde_json::json!({"format": "pt"}));
    let mut data = Vec::new();
    for (k, shape, v) in tensors {
        let a = data.len();
        data.extend(v.iter().flat_map(|x| x.to_le_bytes()));
        hdr.insert(
            k.clone(),
            serde_json::json!({"dtype": "F32", "shape": shape, "data_offsets": [a, data.len()]}),
        );
    }
    let mut h = serde_json::to_vec(&serde_json::Value::Object(hdr)).unwrap();
    while !(8 + h.len()).is_multiple_of(8) {
        h.push(b' ');
    }
    let mut out = (h.len() as u64).to_le_bytes().to_vec();
    out.extend(h);
    out.extend(data);
    out
}

fn vals(n: usize, seed: f32) -> Vec<f32> {
    (0..n).map(|i| (i as f32 * 0.37 + seed).sin()).collect()
}

/// Giống file HF: tháp text trước, vision sau (vision liền một đoạn).
fn fake_model() -> Vec<u8> {
    fake_safetensors(&[
        (
            "text_model.embeddings.weight".into(),
            vec![64, 32],
            vals(64 * 32, 1.0),
        ),
        (
            "text_model.head.weight".into(),
            vec![32, 32],
            vals(32 * 32, 2.0),
        ),
        (
            "vision_model.embeddings.patch_embedding.weight".into(),
            vec![8, 3, 4, 4],
            vals(8 * 48, 3.0),
        ),
        (
            "vision_model.head.probe".into(),
            vec![1, 1, 8],
            vals(8, 4.0),
        ),
        (
            "vision_model.post_layernorm.weight".into(),
            vec![8],
            vals(8, 5.0),
        ),
    ])
}

fn serve<F>(f: F) -> (Srv, String)
where
    F: Fn(tiny_http::Request) + Send + 'static,
{
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let url = format!(
        "http://{}/model.safetensors",
        server.server_addr().to_ip().unwrap()
    );
    let count = Arc::new(AtomicUsize::new(0));
    let (s2, c2) = (server.clone(), count.clone());
    let handle = std::thread::spawn(move || {
        for rq in s2.incoming_requests() {
            c2.fetch_add(1, Ordering::SeqCst);
            f(rq);
        }
    });
    (
        Srv {
            server,
            count,
            handle: Some(handle),
        },
        url,
    )
}

fn range_of(rq: &tiny_http::Request) -> Option<(usize, usize)> {
    let h = rq.headers().iter().find(|h| h.field.equiv("Range"))?;
    let (a, b) = h.value.as_str().strip_prefix("bytes=")?.split_once('-')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Server hỗ trợ Range (206 + Content-Range). `tweak` sửa phần thân trả về (để giả lỗi).
fn serve_range(file: Vec<u8>, tweak: fn(&mut Vec<u8>, usize)) -> (Srv, String) {
    serve(move |rq| {
        let total = file.len();
        match range_of(&rq) {
            Some((a, b)) => {
                let mut part = file[a..=b].to_vec();
                tweak(&mut part, a);
                let cr = format!("bytes {a}-{b}/{total}");
                let h =
                    tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
                let _ = rq.respond(
                    tiny_http::Response::from_data(part)
                        .with_status_code(206)
                        .with_header(h),
                );
            }
            None => {
                let _ = rq.respond(tiny_http::Response::from_data(file.clone()));
            }
        }
    })
}

fn no_tweak(_: &mut Vec<u8>, _: usize) {}

/// Server bỏ qua Range: luôn trả 200 cả file.
fn serve_full(file: Vec<u8>) -> (Srv, String) {
    serve(move |rq| {
        let _ = rq.respond(tiny_http::Response::from_data(file.clone()));
    })
}

/// Server Range trả chậm từng chunk.
fn serve_range_slow(file: Vec<u8>, chunk: usize, delay: Duration) -> (Srv, String) {
    serve(move |rq| {
        let (a, b) = range_of(&rq).unwrap_or((0, file.len() - 1));
        let cr = format!("bytes {a}-{b}/{}", file.len());
        let h = tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
        let r = Slow {
            data: file[a..=b].to_vec(),
            pos: 0,
            chunk,
            delay,
        };
        let _ = rq.respond(tiny_http::Response::new(
            206.into(),
            vec![h],
            r,
            Some(b - a + 1),
            None,
        ));
    })
}

fn serve_status(code: u16) -> (Srv, String) {
    serve(move |rq| {
        let _ = rq.respond(tiny_http::Response::empty(code));
    })
}

/// Manifest tính từ file nguồn (giống examples/make_manifest.rs).
fn manifest_for(file: &[u8], url: &str) -> Manifest {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("src.safetensors");
    std::fs::write(&p, file).unwrap();
    make_manifest(&p, url, "vision.safetensors", "test", vec![]).unwrap()
}

fn store(d: &tempfile::TempDir, m: Manifest) -> ModelStore {
    // Fixtures are sized for 4 MiB ranges; production default is larger.
    ModelStore::new(d.path().into(), m).with_range_chunk_for_test(4 << 20)
}

fn ensure(st: &ModelStore) -> Result<std::path::PathBuf, EmbedError> {
    st.ensure(&|_, _| {}, &AtomicBool::new(false))
}

fn tensors_of(p: &std::path::Path) -> std::collections::BTreeMap<String, Vec<f32>> {
    candle_core::safetensors::load(p, &candle_core::Device::Cpu)
        .unwrap()
        .into_iter()
        .map(|(k, t)| (k, t.flatten_all().unwrap().to_vec1::<f32>().unwrap()))
        .collect()
}

#[test]
fn range_download_extracts_vision_only() {
    let src = fake_model();
    let (srv, url) = serve_range(src.clone(), no_tweak);
    let d = tempfile::tempdir().unwrap();
    let m = manifest_for(&src, &url);
    let st = store(&d, m.clone());
    assert!(matches!(st.status(), ModelStatus::Missing));
    let p = ensure(&st).unwrap();
    assert_eq!(srv.requests(), 2); // header + đoạn vision
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    assert!(std::fs::metadata(&p).unwrap().len() < src.len() as u64);
    let got = tensors_of(&p);
    assert!(
        got.keys().all(|k| k.starts_with("vision_model.")) && got.len() == 3,
        "{:?}",
        got.keys()
    );
    let d2 = tempfile::tempdir().unwrap();
    std::fs::write(d2.path().join("s"), &src).unwrap();
    let want = tensors_of(&d2.path().join("s"));
    for (k, v) in &got {
        assert_eq!(v, &want[k], "{k}");
    }
    assert!(matches!(st.status(), ModelStatus::Ready(_)));
}

#[test]
fn server_without_range_falls_back_to_full_file() {
    let src = fake_model();
    let (srv, url) = serve_full(src.clone());
    let d = tempfile::tempdir().unwrap();
    let m = manifest_for(&src, &url);
    let p = ensure(&store(&d, m.clone())).unwrap();
    assert_eq!(srv.requests(), 1);
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
}

#[test]
fn full_file_with_wrong_source_sha_rejected() {
    let src = fake_model();
    let (_s, url) = serve_full(src.clone());
    let d = tempfile::tempdir().unwrap();
    let mut m = manifest_for(&src, &url);
    m.source.sha256 = "00".repeat(32);
    let st = store(&d, m);
    assert!(matches!(
        ensure(&st),
        Err(EmbedError::ChecksumMismatch { .. })
    ));
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn bad_segment_checksum_rejected() {
    let src = fake_model();
    #[allow(clippy::ptr_arg)] // cùng kiểu với `grow` (fn con trỏ)
    fn flip_last(b: &mut Vec<u8>, _: usize) {
        let n = b.len();
        b[n - 1] ^= 0xff;
    }
    let (_s, url) = serve_range(src.clone(), flip_last);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&src, &url));
    assert!(matches!(
        ensure(&st),
        Err(EmbedError::ChecksumMismatch { .. })
    ));
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn wrong_content_range_rejected() {
    let src = fake_model();
    let (_s, url) = serve(move |rq| {
        let (a, b) = range_of(&rq).unwrap();
        let cr = format!("bytes {}-{}/{}", a + 1, b + 1, src.len()); // lệch 1 byte
        let h = tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
        let _ = rq.respond(
            tiny_http::Response::from_data(src[a..=b].to_vec())
                .with_status_code(206)
                .with_header(h),
        );
    });
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&fake_model(), &url));
    assert!(ensure(&st).is_err());
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn oversize_range_body_aborted() {
    let src = fake_model();
    fn grow(b: &mut Vec<u8>, _: usize) {
        b.extend(std::iter::repeat_n(0u8, 4096));
    }
    let (_s, url) = serve_range(src.clone(), grow);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&src, &url));
    assert!(matches!(
        ensure(&st),
        Err(EmbedError::ChecksumMismatch { .. })
    ));
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn interrupted_download_is_cancelled_and_partial() {
    // vision lớn (~2 MB) để có thời gian huỷ giữa chừng
    let src = fake_safetensors(&[
        ("text_model.w".into(), vec![1024], vals(1024, 1.0)),
        ("vision_model.big".into(), vec![1 << 19], vals(1 << 19, 2.0)),
    ]);
    let (_s, url) = serve_range_slow(src.clone(), 64 << 10, Duration::from_millis(20));
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&src, &url));
    let cancel = AtomicBool::new(false);
    let seen = AtomicU64::new(0);
    let r = std::thread::scope(|sc| {
        sc.spawn(|| {
            while seen.load(Ordering::SeqCst) < (1 << 20) {
                std::thread::sleep(Duration::from_millis(5));
            }
            cancel.store(true, Ordering::SeqCst);
        });
        st.ensure(&|done, _| seen.store(done, Ordering::SeqCst), &cancel)
    });
    assert!(matches!(r, Err(EmbedError::Cancelled)), "{r:?}");
    // huỷ thì giữ .part để tải tiếp
    assert!(matches!(st.status(), ModelStatus::Partial { bytes, .. } if bytes > 0));
}

#[test]
fn tampered_ready_file_is_corrupt() {
    let src = fake_model();
    let (_s, url) = serve_range(src.clone(), no_tweak);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&src, &url));
    let p = ensure(&st).unwrap();
    let mut x = std::fs::read(&p).unwrap();
    let n = x.len();
    x[n - 2] ^= 0xff;
    std::thread::sleep(Duration::from_millis(20)); // mtime khác
    std::fs::write(&p, x).unwrap();
    assert!(matches!(st.status(), ModelStatus::Corrupt));
    ensure(&st).unwrap(); // tải lại được
    assert!(matches!(st.status(), ModelStatus::Ready(_)));
}

#[test]
fn http_error_has_status_separate_from_disk_io() {
    let (_s, url) = serve_status(404);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&fake_model(), &url));
    assert!(matches!(
        ensure(&st),
        Err(EmbedError::Http { status: 404, .. })
    ));
}

#[test]
fn concurrent_ensure_downloads_once() {
    let src = fake_model();
    let (srv, url) = serve_range(src.clone(), no_tweak);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&src, &url));
    std::thread::scope(|sc| {
        for _ in 0..3 {
            sc.spawn(|| ensure(&st).unwrap());
        }
    });
    assert_eq!(srv.requests(), 2);
}

#[test]
fn downloaded_tiny_siglip_loads_in_candle() {
    // tiny_siglip + tháp text giả đặt trước, giống bố cục file HF
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny_siglip.safetensors");
    let mut ts: Vec<(String, Vec<usize>, Vec<f32>)> =
        vec![("text_model.x".into(), vec![256], vals(256, 9.0))];
    for (k, t) in candle_core::safetensors::load(&p, &candle_core::Device::Cpu)
        .unwrap()
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>()
    {
        ts.push((
            k,
            t.dims().to_vec(),
            t.flatten_all().unwrap().to_vec1().unwrap(),
        ));
    }
    let src = fake_safetensors(&ts);
    let (_s, url) = serve_range(src.clone(), no_tweak);
    let d = tempfile::tempdir().unwrap();
    let local = ensure(&store(&d, manifest_for(&src, &url))).unwrap();
    let cfg = candle_transformers::models::siglip::VisionConfig {
        hidden_size: 32,
        intermediate_size: 64,
        num_hidden_layers: 2,
        num_attention_heads: 4,
        num_channels: 3,
        image_size: 224,
        patch_size: 16,
        hidden_act: candle_nn::Activation::GeluPytorchTanh,
        layer_norm_eps: 1e-6,
    };
    let mut e =
        fontatlas_embed::CandleEncoder::load_with(&local, fontatlas_embed::Backend::Cpu, &cfg)
            .unwrap();
    use fontatlas_embed::Encode;
    let v = e
        .encode(&[vec![0.1; 3 * 224 * 224]], &AtomicBool::new(false))
        .unwrap();
    assert_eq!(v[0].len(), 32);
}

#[test]
fn range_survives_redirect_like_hf_cdn() {
    // HF: /resolve/... → 302 tới CDN; header Range phải đi theo sau redirect.
    let src = fake_model();
    let (srv, cdn) = serve_range(src.clone(), no_tweak);
    let (redir, url) = serve(move |rq| {
        let h = tiny_http::Header::from_bytes(&b"Location"[..], cdn.as_bytes()).unwrap();
        let _ = rq.respond(tiny_http::Response::empty(302).with_header(h));
    });
    let d = tempfile::tempdir().unwrap();
    let m = manifest_for(&src, &url);
    let p = ensure(&store(&d, m.clone())).unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    // Redirect resolved once (header probe); the vision range goes straight to the CDN.
    assert_eq!((redir.requests(), srv.requests()), (1, 2));
}

#[test]
fn offline_extract_equals_downloaded_file() {
    let src = fake_model();
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("src"), &src).unwrap();
    fontatlas_embed::extract_vision_file(&d.path().join("src"), &d.path().join("out")).unwrap();
    assert_eq!(
        sha256_file(&d.path().join("out")).unwrap(),
        manifest_for(&src, "http://x").sha256
    );
}

#[test]
fn vision_header_bytes_are_pinned() {
    // Thứ tự khoá trong JSON nguồn kiểu HF (dtype, shape, data_offsets); đầu ra phải cố định từng byte,
    // kể cả khi serde_json bật preserve_order (sha file cục bộ được ghim trong manifest).
    let src = br#"{"__metadata__":{"format":"pt"},"text_model.t":{"dtype":"F32","shape":[1],"data_offsets":[0,4]},"vision_model.b":{"dtype":"F32","shape":[1],"data_offsets":[12,16]},"vision_model.a":{"dtype":"F32","shape":[2],"data_offsets":[4,12]}}"#;
    let (h, a, b) = fontatlas_embed::vision_header(src).unwrap();
    assert_eq!((a, b), (4, 16));
    let json = r#"{"__metadata__":{"format":"pt"},"vision_model.a":{"data_offsets":[0,8],"dtype":"F32","shape":[2]},"vision_model.b":{"data_offsets":[8,12],"dtype":"F32","shape":[1]}}"#;
    let mut want = json.as_bytes().to_vec();
    while !(8 + want.len()).is_multiple_of(8) {
        want.push(b' ');
    }
    let mut full = (want.len() as u64).to_le_bytes().to_vec();
    full.extend(want);
    assert_eq!(
        String::from_utf8_lossy(&h[8..]),
        String::from_utf8_lossy(&full[8..])
    );
    assert_eq!(h, full);
}

type Log = Arc<std::sync::Mutex<Vec<(usize, usize)>>>;

/// Như `serve_range_slow` nhưng ghi lại từng Range yêu cầu.
fn serve_logged(file: Vec<u8>, chunk: usize, delay: Duration) -> (Srv, String, Log) {
    let log: Log = Arc::default();
    let l2 = log.clone();
    let (srv, url) = serve(move |rq| {
        let (a, b) = range_of(&rq).unwrap_or((0, file.len() - 1));
        l2.lock().unwrap().push((a, b));
        let cr = format!("bytes {a}-{b}/{}", file.len());
        let h = tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
        let r = Slow {
            data: file[a..=b].to_vec(),
            pos: 0,
            chunk,
            delay,
        };
        let _ = rq.respond(tiny_http::Response::new(
            206.into(),
            vec![h],
            r,
            Some(b - a + 1),
            None,
        ));
    });
    (srv, url, log)
}

fn big_model(seed: f32) -> Vec<u8> {
    fake_safetensors(&[
        ("text_model.w".into(), vec![1024], vals(1024, seed)),
        (
            "vision_model.big".into(),
            vec![1 << 19],
            vals(1 << 19, seed + 1.0),
        ),
    ])
}

/// Huỷ ngay khi đã nhận ≥ 1 MiB vision.
fn cancel_at_1mib(st: &ModelStore) -> Result<std::path::PathBuf, EmbedError> {
    let cancel = AtomicBool::new(false);
    st.ensure_resumable(
        &|p| {
            if p.done >= (1 << 20) {
                cancel.store(true, Ordering::SeqCst)
            }
        },
        &cancel,
    )
}

#[test]
fn cancel_keeps_part_and_resumes() {
    let src = big_model(1.0);
    let (_s, url, log) = serve_logged(src.clone(), 64 << 10, Duration::from_millis(3));
    let d = tempfile::tempdir().unwrap();
    let m = manifest_for(&src, &url);
    let st = store(&d, m.clone());
    assert!(matches!(cancel_at_1mib(&st), Err(EmbedError::Cancelled)));
    let ModelStatus::Partial { bytes, total } = st.status() else {
        panic!("{:?}", st.status())
    };
    assert_eq!(total, m.size);
    let nh = m.size - m.source.vision_len; // header cục bộ
    assert!(bytes > nh && bytes < total, "{bytes} {nh} {total}");
    log.lock().unwrap().clear();
    let events = std::sync::Mutex::new(Vec::<DownloadProgress>::new());
    let p = st
        .ensure_resumable(&|e| events.lock().unwrap().push(e), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    let reqs = log.lock().unwrap().clone();
    // 1: header (nhỏ) để khôi phục sha; 2: phần vision còn lại bắt đầu đúng offset đã có
    assert_eq!(reqs.len(), 2, "{reqs:?}");
    assert_eq!(reqs[1].0 as u64, m.source.vision_offset + (bytes - nh));
    assert!(events.lock().unwrap().iter().all(|e| e.resumed));
    assert!(matches!(st.status(), ModelStatus::Ready(_)));
    assert!(
        !d.path()
            .join(format!("{}.part.json", st.file_name()))
            .exists()
    );
}

#[test]
fn manifest_change_restarts() {
    let (src1, src2) = (big_model(1.0), big_model(5.0));
    let (_s1, url1, _l1) = serve_logged(src1.clone(), 64 << 10, Duration::from_millis(3));
    let (_s2, url2, log2) = serve_logged(src2.clone(), 64 << 10, Duration::from_millis(0));
    let d = tempfile::tempdir().unwrap();
    assert!(matches!(
        cancel_at_1mib(&store(&d, manifest_for(&src1, &url1))),
        Err(EmbedError::Cancelled)
    ));
    let m2 = manifest_for(&src2, &url2);
    let st2 = store(&d, m2.clone());
    assert!(
        matches!(st2.status(), ModelStatus::Partial { bytes: 0, .. }),
        "manifest mới → .part cũ không dùng được"
    );
    let p = ensure(&st2).unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m2.sha256);
    let reqs = log2.lock().unwrap().clone();
    assert_eq!(reqs[1].0 as u64, m2.source.vision_offset, "{reqs:?}");
}

#[test]
fn checksum_mismatch_reported() {
    let src = fake_model();
    #[allow(clippy::ptr_arg)]
    fn flip(b: &mut Vec<u8>, _: usize) {
        let n = b.len();
        b[n - 1] ^= 0xff;
    }
    let (_s, url) = serve_range(src.clone(), flip);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&src, &url));
    assert!(matches!(
        ensure(&st),
        Err(EmbedError::ChecksumMismatch { .. })
    ));
    assert!(!d.path().join(format!("{}.part", st.file_name())).exists());
    assert!(
        !d.path()
            .join(format!("{}.part.json", st.file_name()))
            .exists()
    );
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn progress_reports_speed() {
    let src = big_model(1.0);
    let (_s, url, _l) = serve_logged(src.clone(), 128 << 10, Duration::from_millis(2));
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&src, &url));
    let events = std::sync::Mutex::new(Vec::<DownloadProgress>::new());
    st.ensure_resumable(&|e| events.lock().unwrap().push(e), &AtomicBool::new(false))
        .unwrap();
    let ev = events.lock().unwrap();
    let last = ev.last().unwrap();
    assert!(
        last.bytes_per_sec > 0.0 && last.done == last.total,
        "{last:?}"
    );
    assert!(ev.iter().all(|e| !e.resumed));
}

#[test]
fn server_ignoring_range_restarts_with_resumed_false() {
    let src = big_model(1.0);
    let (_s, url, _l) = serve_logged(src.clone(), 64 << 10, Duration::from_millis(3));
    let d = tempfile::tempdir().unwrap();
    let m = manifest_for(&src, &url);
    assert!(matches!(
        cancel_at_1mib(&store(&d, m.clone())),
        Err(EmbedError::Cancelled)
    ));
    let (_f, url_full) = serve_full(src.clone());
    let mut m2 = m.clone();
    m2.source.url = url_full;
    let st2 = store(&d, m2);
    assert!(matches!(st2.status(), ModelStatus::Partial { bytes, .. } if bytes > 0));
    let events = std::sync::Mutex::new(Vec::<DownloadProgress>::new());
    let p = st2
        .ensure_resumable(&|e| events.lock().unwrap().push(e), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    let ev = events.lock().unwrap();
    assert!(
        !ev.is_empty() && ev.iter().all(|e| !e.resumed),
        "{:?}",
        ev.first()
    );
}

#[test]
fn disk_free_positive() {
    let d = tempfile::tempdir().unwrap();
    assert!(disk_free(d.path()).unwrap() > 0);
    assert!(disk_free(&d.path().join("chua/co/thu/muc")).unwrap() > 0);
}

// Raw local HTTP fixture: one explicitly owned thread per connection, joined on drop.
// Connection: close makes cancelled bodies observable as EOF rather than pooled idle sockets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    WrongRange,
    IgnoreRange,
    TruncateOnce,
    Stall,
    StallAlways,
    Forbid403Once,
}
struct ConcurrentSrv {
    url: String,
    stop: Arc<AtomicBool>,
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    requests: Arc<AtomicUsize>,
    finished: Log,
    thread: Option<JoinHandle<()>>,
}
impl Drop for ConcurrentSrv {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.thread.take() {
            h.join().unwrap();
        }
    }
}
impl ConcurrentSrv {
    fn wait_idle(&self) {
        let t = std::time::Instant::now();
        while self.active.load(Ordering::SeqCst) > 0 && t.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            self.active.load(Ordering::SeqCst),
            0,
            "no active transfer after cancellation/failure"
        );
    }
}
fn concurrent_server(file: Vec<u8>, vision_offset: usize, fault: Fault) -> ConcurrentSrv {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/model", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let finished: Log = Arc::default();
    let (s, act, max, reqs, done) = (
        stop.clone(),
        active.clone(),
        peak.clone(),
        requests.clone(),
        finished.clone(),
    );
    let file = Arc::new(file);
    let fired = Arc::new(AtomicBool::new(false));
    let thread = std::thread::spawn(move || {
        let mut handles = Vec::new();
        while !s.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let (act, max, reqs, done, file, fired, stop) = (
                        act.clone(),
                        max.clone(),
                        reqs.clone(),
                        done.clone(),
                        file.clone(),
                        fired.clone(),
                        s.clone(),
                    );
                    handles.push(std::thread::spawn(move || {
                        stream.set_nonblocking(false).unwrap();
                        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                        stream.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
                        let mut header = Vec::new(); let mut ch = [0u8; 1];
                        while !header.ends_with(b"\r\n\r\n") {
                            if header.len() > 32000 || stream.read_exact(&mut ch).is_err() { return; }
                            header.push(ch[0]);
                        }
                        reqs.fetch_add(1, Ordering::SeqCst);
                        struct Guard(Arc<AtomicUsize>); impl Drop for Guard { fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); } }
                        let n = act.fetch_add(1, Ordering::SeqCst) + 1; max.fetch_max(n, Ordering::SeqCst); let _g = Guard(act);
                        let text = String::from_utf8(header).unwrap();
                        let range = text.lines().find_map(|l| {
                            let (key, value) = l.split_once(':')?;
                            if !key.eq_ignore_ascii_case("range") { return None; }
                            let (a,b) = value.trim().strip_prefix("bytes=")?.split_once('-')?;
                            Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))
                        });
                        let (a,b) = range.unwrap_or((0,file.len()-1));
                        let child = range.is_some() && a >= vision_offset + (4 << 20);
                        let special = child && fault != Fault::None && (fault == Fault::StallAlways || !fired.swap(true, Ordering::SeqCst));
                        if special && fault == Fault::Forbid403Once {
                            let _ = write!(stream,"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                            return;
                        }
                        if special && fault == Fault::IgnoreRange {
                            let _ = write!(stream,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",file.len());
                            let _ = stream.flush(); let _ = stream.read(&mut ch); return;
                        }
                        let (status, cr) = if range.is_some() {
                            (206, format!("Content-Range: bytes {}-{}/{}\r\n", if special && fault == Fault::WrongRange { a+1 } else { a },b,file.len()))
                        } else { (200,String::new()) };
                        if write!(stream,"HTTP/1.1 {status} OK\r\n{cr}Content-Length: {}\r\nConnection: close\r\n\r\n", b-a+1).is_err() { return; }
                        if special && matches!(fault,Fault::Stall | Fault::StallAlways) { let _ = stream.flush(); let _ = stream.read(&mut ch); return; }
                        let end = if special && fault == Fault::TruncateOnce { a + ((b-a)/2) } else { b+1 };
                        // Delay the first parallel chunk so later chunks finish first.
                        if child && a == vision_offset + (4<<20) { std::thread::sleep(Duration::from_millis(150)); }
                        for c in file[a..end].chunks(64<<10) {
                            if stop.load(Ordering::SeqCst) || stream.write_all(c).is_err() { return; }
                            if child { std::thread::sleep(Duration::from_millis(2)); }
                        }
                        done.lock().unwrap().push((a,b));
                    }));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(_) => break,
            }
        }
        for h in handles {
            h.join().unwrap();
        }
    });
    ConcurrentSrv {
        url,
        stop,
        active,
        peak,
        requests,
        finished,
        thread: Some(thread),
    }
}
fn parallel_model() -> Vec<u8> {
    fake_safetensors(&[
        ("text_model.w".into(), vec![8], vec![0.0; 8]),
        ("vision_model.big".into(), vec![4 << 20], vals(4 << 20, 2.0)),
    ])
}

#[test]
fn parallel_ranges_are_bounded_ordered_and_sha_verified() {
    let src = parallel_model();
    let mut m = manifest_for(&src, "");
    let s = concurrent_server(src, m.source.vision_offset as usize, Fault::None);
    m.source.url = s.url.clone();
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, m.clone());
    let t = std::time::Instant::now();
    let p = ensure(&st).unwrap();
    eprintln!(
        "localhost 16MiB bounded transfer: {:?}, peak={}",
        t.elapsed(),
        s.peak.load(Ordering::SeqCst)
    );
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    s.wait_idle();
    assert_eq!(s.peak.load(Ordering::SeqCst), 3);
    let finished = s.finished.lock().unwrap();
    let first = m.source.vision_offset as usize + (4 << 20);
    assert!(
        finished.iter().position(|x| x.0 == first).unwrap()
            > finished
                .iter()
                .position(|x| x.0 == first + (4 << 20))
                .unwrap(),
        "fixture must complete out of order"
    );
}

#[test]
fn parallel_worker_bad_range_and_200_fallback() {
    for fault in [Fault::WrongRange, Fault::IgnoreRange] {
        let src = parallel_model();
        let mut m = manifest_for(&src, "");
        let s = concurrent_server(src, m.source.vision_offset as usize, fault);
        m.source.url = s.url.clone();
        let d = tempfile::tempdir().unwrap();
        let st = store(&d, m.clone());
        let r = ensure(&st);
        if fault == Fault::WrongRange {
            assert!(matches!(
                r,
                Err(EmbedError::Network {
                    kind: fontatlas_embed::NetworkErrorKind::Other,
                    ..
                })
            ));
            assert!(!matches!(st.status(), ModelStatus::Ready(_)));
        } else {
            assert_eq!(sha256_file(&r.unwrap()).unwrap(), m.sha256);
        }
        s.wait_idle();
        assert!(
            s.peak.load(Ordering::SeqCst) <= 3,
            "{}",
            s.peak.load(Ordering::SeqCst)
        );
    }
}

#[test]
fn parallel_truncation_retries_from_contiguous_prefix_without_progress_rewind() {
    let src = parallel_model();
    let mut m = manifest_for(&src, "");
    let s = concurrent_server(src, m.source.vision_offset as usize, Fault::TruncateOnce);
    m.source.url = s.url.clone();
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, m.clone());
    let events = std::sync::Mutex::new(Vec::new());
    let p = st
        .ensure_resumable(&|e| events.lock().unwrap().push(e), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    s.wait_idle();
    let ev = events.lock().unwrap();
    assert!(ev.windows(2).all(|w| w[0].done <= w[1].done));
    assert!(ev.iter().all(|e| e.done <= e.total));
    assert!(ev.iter().any(|e| e.resumed));
    assert!(s.requests.load(Ordering::SeqCst) > 5);
    assert!(s.peak.load(Ordering::SeqCst) <= 3);
}

#[test]
fn parallel_cancel_and_timeout_drop_stalled_body_and_resume_exact_prefix() {
    for cancel_it in [true, false] {
        let src = parallel_model();
        let mut m = manifest_for(&src, "");
        let s = concurrent_server(src, m.source.vision_offset as usize, Fault::Stall);
        m.source.url = s.url.clone();
        let d = tempfile::tempdir().unwrap();
        let st = store(&d, m.clone()).with_read_timeout_for_test(Duration::from_millis(350));
        let cancel = Arc::new(AtomicBool::new(false));
        let watcher = if cancel_it {
            let (cancel, active, finished) = (cancel.clone(), s.active.clone(), s.finished.clone());
            let head_end = m.source.vision_offset as usize + (4 << 20);
            Some(std::thread::spawn(move || {
                // Cancel only once the first 4 MiB vision chunk has been fully served and the
                // parallel workers are running; a fixed short sleep raced on a loaded CI machine.
                let t = std::time::Instant::now();
                while t.elapsed() < Duration::from_secs(30)
                    && !(active.load(Ordering::SeqCst) >= 3
                        && finished.lock().unwrap().iter().any(|&(_, b)| b + 1 == head_end))
                {
                    std::thread::sleep(Duration::from_millis(2));
                }
                std::thread::sleep(Duration::from_millis(75));
                cancel.store(true, Ordering::SeqCst);
            }))
        } else {
            None
        };
        let r = st.ensure_resumable(&|_| {}, &cancel);
        if let Some(h) = watcher {
            h.join().unwrap();
        }
        if cancel_it {
            assert!(matches!(r, Err(EmbedError::Cancelled)));
        } else {
            assert!(r.is_ok(), "single stall is retried");
        }
        s.wait_idle();
        assert!(s.peak.load(Ordering::SeqCst) <= 3);
        if cancel_it {
            let ModelStatus::Partial { bytes, .. } = st.status() else {
                panic!("must retain contiguous prefix")
            };
            assert_eq!(bytes, m.size - m.source.vision_len + (4 << 20));
            let p = ensure(&st).unwrap();
            assert_eq!(sha256_file(&p).unwrap(), m.sha256);
            s.wait_idle();
        }
    }
}

#[test]
fn sync_model_api_inside_async_runtime_returns_error_without_panicking() {
    let d = tempfile::tempdir().unwrap();
    let m = manifest_for(&fake_model(), "http://127.0.0.1:1/model");
    let st = store(&d, m);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let r = rt.block_on(async { ensure(&st) });
    assert!(matches!(r,Err(EmbedError::Model(ref e)) if e.contains("dedicated std::thread")));
}

#[test]
fn persistent_timeout_stops_after_bounded_retries_and_leaves_no_active_worker() {
    let src = parallel_model();
    let mut m = manifest_for(&src, "");
    let s = concurrent_server(src, m.source.vision_offset as usize, Fault::StallAlways);
    m.source.url = s.url.clone();
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, m.clone()).with_read_timeout_for_test(Duration::from_millis(350));
    let t = std::time::Instant::now();
    assert!(matches!(
        ensure(&st),
        Err(EmbedError::Network {
            kind: fontatlas_embed::NetworkErrorKind::Timeout,
            ..
        })
    ));
    // Retries are bounded by the request count below; the wall-clock bound only guards
    // against an endless loop. 5 s flaked when the machine was busy (parallel builds).
    assert!(t.elapsed() < Duration::from_secs(30), "{:?}", t.elapsed());
    s.wait_idle();
    assert!(s.peak.load(Ordering::SeqCst) <= 3);
    assert!(s.requests.load(Ordering::SeqCst) <= 9);
    let ModelStatus::Partial { bytes, .. } = st.status() else {
        panic!("retain prefix")
    };
    assert_eq!(bytes, m.size - m.source.vision_len + (4 << 20));
}

#[test]
fn final_local_sha_mismatch_never_ready_or_retried_and_429_honors_server_pacing() {
    let src = fake_model();
    let (s, url) = serve_range(src.clone(), no_tweak);
    let d = tempfile::tempdir().unwrap();
    let mut m = manifest_for(&src, &url);
    m.sha256 = "00".repeat(32);
    let st = store(&d, m);
    assert!(matches!(
        ensure(&st),
        Err(EmbedError::ChecksumMismatch { .. })
    ));
    assert_eq!(s.requests(), 2);
    assert_eq!(st.status(), ModelStatus::Missing);
    let (s, url) = serve(|rq| {
        let h = tiny_http::Header::from_bytes("Retry-After", "120").unwrap();
        let _ = rq.respond(tiny_http::Response::empty(429).with_header(h));
    });
    let st = store(&d, manifest_for(&src, &url));
    assert!(
        matches!(ensure(&st),Err(EmbedError::Http { status:429, retry_after:Some(ref v) }) if v == "120")
    );
    assert_eq!(s.requests(), 1);
    assert_eq!(st.status(), ModelStatus::Missing);
}

#[test]
fn sync_api_rejects_tokio_blocking_context_before_filesystem_mutation() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("untouched");
    let m = manifest_for(&fake_model(), "http://127.0.0.1:1/model");
    let p = path.clone();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let r = rt.block_on(async move {
        tokio::task::spawn_blocking(move || ensure(&ModelStore::new(p, m)))
            .await
            .unwrap()
    });
    assert!(matches!(r,Err(EmbedError::Model(ref e)) if e.contains("dedicated std::thread")));
    assert!(!path.exists());
}

#[test]
fn malformed_resume_metadata_cannot_overflow_or_mix_header_denominators() {
    let src = big_model(1.0);
    let (_s, url, _log) = serve_logged(src.clone(), 64 << 10, Duration::from_millis(3));
    let d = tempfile::tempdir().unwrap();
    let m = manifest_for(&src, &url);
    let st = store(&d, m.clone());
    assert!(matches!(cancel_at_1mib(&st), Err(EmbedError::Cancelled)));
    let meta = d.path().join(format!("{}.part.json", st.file_name()));
    std::fs::write(meta,serde_json::to_vec(&serde_json::json!({"manifest_sha256":m.sha256,"local_header_len":u64::MAX,"vision_done":1})).unwrap()).unwrap();
    assert_eq!(
        st.status(),
        ModelStatus::Partial {
            bytes: 0,
            total: m.size
        }
    );
    assert_eq!(sha256_file(&ensure(&st).unwrap()).unwrap(), m.sha256);
}

#[test]
fn transient_http_5xx_retries_then_succeeds_but_4xx_never_retries() {
    for status in [500, 502, 503, 504] {
        let src = fake_model();
        let served = src.clone();
        let n = AtomicUsize::new(0);
        let (srv, url) = serve(move |rq| {
            if n.fetch_add(1, Ordering::SeqCst) == 0 {
                let _ = rq.respond(tiny_http::Response::empty(status));
                return;
            }
            let (a, b) = range_of(&rq).unwrap();
            let cr = tiny_http::Header::from_bytes(
                "Content-Range",
                format!("bytes {a}-{b}/{}", served.len()),
            )
            .unwrap();
            let _ = rq.respond(
                tiny_http::Response::from_data(served[a..=b].to_vec())
                    .with_status_code(206)
                    .with_header(cr),
            );
        });
        let d = tempfile::tempdir().unwrap();
        let m = manifest_for(&src, &url);
        let p = ensure(&store(&d, m.clone())).unwrap();
        assert_eq!(sha256_file(&p).unwrap(), m.sha256);
        assert_eq!(srv.requests(), 3);
    }
    for status in [400, 401, 403, 404, 408] {
        let (srv, url) = serve_status(status);
        let d = tempfile::tempdir().unwrap();
        let e = ensure(&store(&d, manifest_for(&fake_model(), &url))).unwrap_err();
        assert!(matches!(e,EmbedError::Http { status:s, .. } if s==status));
        assert_eq!(e.code(), "http");
        assert_eq!(srv.requests(), 1);
    }
}

#[test]
fn full_stream_fallback_starts_a_new_progress_denominator() {
    let src = parallel_model();
    let mut m = manifest_for(&src, "");
    let s = concurrent_server(src, m.source.vision_offset as usize, Fault::IgnoreRange);
    m.source.url = s.url.clone();
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, m.clone());
    let events = std::sync::Mutex::new(Vec::new());
    let p = st
        .ensure_resumable(&|e| events.lock().unwrap().push(e), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    let events = events.lock().unwrap();
    let switch = events
        .windows(2)
        .position(|w| w[0].total != w[1].total)
        .expect("Range -> full source changes total")
        + 1;
    assert!(events[switch - 1].done >= 4 << 20);
    assert!(
        events[switch].done <= 2 << 20,
        "new phase must start advancing from zero"
    );
    assert!(
        events
            .windows(2)
            .all(|w| w[0].total != w[1].total || w[0].done <= w[1].done)
    );
    assert!(
        events
            .iter()
            .all(|e| e.done <= e.total && e.bytes_per_sec.is_finite())
    );
    assert_eq!(events.last().unwrap().done, m.source.size);
    s.wait_idle();
}

#[test]
fn redirect_loop_is_reported_without_a_second_download_attempt() {
    let (srv, url) = serve(|rq| {
        let h = tiny_http::Header::from_bytes("Location", "/model.safetensors").unwrap();
        let _ = rq.respond(tiny_http::Response::empty(302).with_header(h));
    });
    let d = tempfile::tempdir().unwrap();
    let e = ensure(&store(&d, manifest_for(&fake_model(), &url))).unwrap_err();
    assert!(
        matches!(
            e,
            EmbedError::Network {
                kind: fontatlas_embed::NetworkErrorKind::Redirect,
                ..
            }
        ),
        "{e:?}"
    );
    assert_eq!(e.code(), "http_redirect");
    assert!(
        srv.requests() <= 11,
        "redirects should not be retried as a new download"
    );
}

/// Origin that 302s every request to `cdn` and counts hits.
fn redirector(cdn: String) -> (Srv, String) {
    serve(move |rq| {
        let h = tiny_http::Header::from_bytes(&b"Location"[..], cdn.as_bytes()).unwrap();
        let _ = rq.respond(tiny_http::Response::empty(302).with_header(h));
    })
}

#[test]
fn redirect_is_resolved_once_for_all_ranges() {
    let src = parallel_model();
    let mut m = manifest_for(&src, "");
    let cdn = concurrent_server(src, m.source.vision_offset as usize, Fault::None);
    let (origin, url) = redirector(cdn.url.clone());
    m.source.url = url;
    let d = tempfile::tempdir().unwrap();
    let p = ensure(&store(&d, m.clone())).unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    cdn.wait_idle();
    assert_eq!(origin.requests(), 1, "only the probe touches the origin");
    assert!(cdn.requests.load(Ordering::SeqCst) >= 5);
}

#[test]
fn cdn_403_re_resolves_from_origin_and_continues() {
    let src = parallel_model();
    let mut m = manifest_for(&src, "");
    let cdn = concurrent_server(src, m.source.vision_offset as usize, Fault::Forbid403Once);
    let (origin, url) = redirector(cdn.url.clone());
    m.source.url = url;
    let d = tempfile::tempdir().unwrap();
    let p = ensure(&store(&d, m.clone())).unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256);
    cdn.wait_idle();
    assert_eq!(origin.requests(), 2, "probe + one re-resolve");
}

#[test]
fn sliding_window_bounds_in_flight_and_reorder_buffer() {
    let n = 8 << 20;
    let src = fake_safetensors(&[
        ("text_model.w".into(), vec![8], vec![0.0; 8]),
        ("vision_model.big".into(), vec![n], vals(n, 2.0)),
    ]);
    let mut m = manifest_for(&src, "");
    let s = concurrent_server(src, m.source.vision_offset as usize, Fault::None);
    m.source.url = s.url.clone();
    let d = tempfile::tempdir().unwrap();
    let p = ensure(&store(&d, m.clone())).unwrap();
    assert_eq!(sha256_file(&p).unwrap(), m.sha256, "fed strictly in order");
    s.wait_idle();
    assert_eq!(s.peak.load(Ordering::SeqCst), 3);
    // The first parallel chunk is delayed 150 ms. Window of 3: at most the next two chunks can
    // complete before it; a chunk 3 positions later cannot even start until it is consumed.
    let first = m.source.vision_offset as usize + (4 << 20);
    let finished = s.finished.lock().unwrap();
    let pos = finished.iter().position(|x| x.0 == first).unwrap();
    let before = finished[..pos]
        .iter()
        .filter(|x| x.0 > m.source.vision_offset as usize)
        .count();
    assert!((1..=2).contains(&before), "{before} chunks finished before the delayed one");
}

/// Server nhận kết nối rồi im lặng: không bao giờ đọc request hay trả header.
struct Silent {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Silent {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
    }
}

fn silent_server() -> Silent {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/model", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    let thread = std::thread::spawn(move || {
        let mut held = Vec::new(); // giữ kết nối mở tới khi dừng
        while !s.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((st, _)) => held.push(st),
                Err(_) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    });
    Silent { url, stop, thread: Some(thread) }
}

#[test]
fn server_that_accepts_but_never_answers_times_out_and_fails_cleanly() {
    let s = silent_server();
    let m = manifest_for(&fake_model(), &s.url);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, m).with_read_timeout_for_test(Duration::from_millis(300));
    let t = std::time::Instant::now();
    let r = ensure(&st);
    assert!(
        matches!(r, Err(EmbedError::Network { kind: fontatlas_embed::NetworkErrorKind::Timeout, .. })),
        "{r:?}"
    );
    assert!(t.elapsed() < Duration::from_secs(120), "{:?}", t.elapsed()); // chỉ chặn vòng lặp vô hạn
    assert!(matches!(st.status(), ModelStatus::Missing), "không để lại .part khi chưa có byte nào");
}

#[test]
fn cancel_while_waiting_for_headers_returns_cancelled_promptly() {
    let s = silent_server();
    let m = manifest_for(&fake_model(), &s.url);
    let d = tempfile::tempdir().unwrap();
    // timeout đọc 10 phút: chỉ cờ huỷ mới giải phóng được luồng. Luồng riêng + chờ có hạn thay cho đồng hồ ngắn:
    // CI chạy cùng build song song (tải máy > 20) từng làm một lần huỷ mất 15 s, không phải do treo.
    let st = Arc::new(store(&d, m).with_read_timeout_for_test(Duration::from_secs(600)));
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();
    let (st2, c2) = (st.clone(), cancel.clone());
    std::thread::spawn(move || {
        let _ = tx.send(st2.ensure_resumable(&|_| {}, &c2));
    });
    std::thread::sleep(Duration::from_millis(300));
    cancel.store(true, Ordering::SeqCst);
    match rx.recv_timeout(Duration::from_secs(90)) {
        Ok(r) => assert!(matches!(r, Err(EmbedError::Cancelled)), "{r:?}"),
        Err(_) => panic!("luồng tải không dừng sau khi huỷ dù timeout đọc còn 10 phút"),
    }
}

#[test]
fn cancel_while_another_process_holds_the_model_lock_returns_promptly() {
    // Tiến trình khác (app thứ hai, công cụ dòng lệnh) đang giữ khoá `.lock` của model: luồng tải không được
    // kẹt vô hạn ở bước lấy khoá, vì app chỉ có thể huỷ bằng cờ và `wait_download` sẽ báo `busy` mãi.
    let m = manifest_for(&fake_model(), "http://127.0.0.1:1/model");
    let d = tempfile::tempdir().unwrap();
    let st = Arc::new(store(&d, m));
    let held = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(d.path().join(format!("{}.lock", st.file_name())))
        .unwrap();
    held.lock().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();
    let (st2, c2) = (st.clone(), cancel.clone());
    std::thread::spawn(move || {
        let _ = tx.send(st2.ensure_resumable(&|_| {}, &c2));
    });
    std::thread::sleep(Duration::from_millis(300));
    cancel.store(true, Ordering::SeqCst);
    // chờ có hạn rộng: chỉ để phân biệt "kẹt vô hạn" với "máy đang tải nặng"
    match rx.recv_timeout(Duration::from_secs(90)) {
        Ok(r) => assert!(matches!(r, Err(EmbedError::Cancelled)), "{r:?}"),
        Err(_) => panic!("luồng tải bị chặn ở khoá file dù đã huỷ"),
    }
    drop(held);
}
