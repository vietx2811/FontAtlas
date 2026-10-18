use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use fontatlas_embed::{EmbedError, Manifest, ModelStatus, ModelStore, make_manifest, sha256_file};

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
        hdr.insert(k.clone(), serde_json::json!({"dtype": "F32", "shape": shape, "data_offsets": [a, data.len()]}));
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
        ("text_model.embeddings.weight".into(), vec![64, 32], vals(64 * 32, 1.0)),
        ("text_model.head.weight".into(), vec![32, 32], vals(32 * 32, 2.0)),
        ("vision_model.embeddings.patch_embedding.weight".into(), vec![8, 3, 4, 4], vals(8 * 48, 3.0)),
        ("vision_model.head.probe".into(), vec![1, 1, 8], vals(8, 4.0)),
        ("vision_model.post_layernorm.weight".into(), vec![8], vals(8, 5.0)),
    ])
}

fn serve<F>(f: F) -> (Srv, String)
where
    F: Fn(tiny_http::Request) + Send + 'static,
{
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let url = format!("http://{}/model.safetensors", server.server_addr().to_ip().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let (s2, c2) = (server.clone(), count.clone());
    let handle = std::thread::spawn(move || {
        for rq in s2.incoming_requests() {
            c2.fetch_add(1, Ordering::SeqCst);
            f(rq);
        }
    });
    (Srv { server, count, handle: Some(handle) }, url)
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
                let h = tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
                let _ = rq.respond(tiny_http::Response::from_data(part).with_status_code(206).with_header(h));
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
        let r = Slow { data: file[a..=b].to_vec(), pos: 0, chunk, delay };
        let _ = rq.respond(tiny_http::Response::new(206.into(), vec![h], r, Some(b - a + 1), None));
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
    ModelStore::new(d.path().into(), m)
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
    assert!(got.keys().all(|k| k.starts_with("vision_model.")) && got.len() == 3, "{:?}", got.keys());
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
    assert!(matches!(ensure(&st), Err(EmbedError::ChecksumMismatch { .. })));
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
    assert!(matches!(ensure(&st), Err(EmbedError::ChecksumMismatch { .. })));
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn wrong_content_range_rejected() {
    let src = fake_model();
    let (_s, url) = serve(move |rq| {
        let (a, b) = range_of(&rq).unwrap();
        let cr = format!("bytes {}-{}/{}", a + 1, b + 1, src.len()); // lệch 1 byte
        let h = tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
        let _ = rq.respond(tiny_http::Response::from_data(src[a..=b].to_vec()).with_status_code(206).with_header(h));
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
    assert!(matches!(ensure(&st), Err(EmbedError::ChecksumMismatch { .. })));
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
    assert!(matches!(st.status(), ModelStatus::Missing)); // huỷ thì xoá .part
    std::fs::write(d.path().join(format!("{}.part", st.file_name())), b"dang do").unwrap(); // app chết giữa chừng
    assert!(matches!(st.status(), ModelStatus::Partial));
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
fn http_error_is_io() {
    let (_s, url) = serve_status(404);
    let d = tempfile::tempdir().unwrap();
    let st = store(&d, manifest_for(&fake_model(), &url));
    assert!(matches!(ensure(&st), Err(EmbedError::Io(_))));
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
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny_siglip.safetensors");
    let mut ts: Vec<(String, Vec<usize>, Vec<f32>)> = vec![("text_model.x".into(), vec![256], vals(256, 9.0))];
    for (k, t) in candle_core::safetensors::load(&p, &candle_core::Device::Cpu).unwrap().into_iter().collect::<std::collections::BTreeMap<_, _>>() {
        ts.push((k, t.dims().to_vec(), t.flatten_all().unwrap().to_vec1().unwrap()));
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
    let mut e = fontatlas_embed::CandleEncoder::load_with(&local, fontatlas_embed::Backend::Cpu, &cfg).unwrap();
    use fontatlas_embed::Encode;
    let v = e.encode(&[vec![0.1; 3 * 224 * 224]], &AtomicBool::new(false)).unwrap();
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
    assert_eq!((redir.requests(), srv.requests()), (2, 2));
}
