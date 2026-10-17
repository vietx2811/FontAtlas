use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use fontatlas_embed::{EmbedError, Manifest, ModelStatus, ModelStore, sha256_file};
use sha2::{Digest, Sha256};

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

fn body(n: usize) -> Vec<u8> {
    let mut s: u64 = 7;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 56) as u8
        })
        .collect()
}

fn serve<F>(f: F) -> (Srv, String)
where
    F: Fn(tiny_http::Request) + Send + 'static,
{
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let url = format!("http://{}/m.onnx", server.server_addr().to_ip().unwrap());
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

fn serve_bytes(n: usize) -> (Srv, String, Vec<u8>) {
    let b = body(n);
    let b2 = b.clone();
    let (s, u) = serve(move |rq| {
        let _ = rq.respond(tiny_http::Response::from_data(b2.clone()));
    });
    (s, u, b)
}

fn serve_slow(total: usize, chunk: usize, delay: Duration) -> (Srv, String, Vec<u8>) {
    let b = body(total);
    let b2 = b.clone();
    let (s, u) = serve(move |rq| {
        let r = Slow { data: b2.clone(), pos: 0, chunk, delay };
        let _ = rq.respond(tiny_http::Response::new(200.into(), vec![], r, Some(total), None));
    });
    (s, u, b)
}

/// Gửi `actual` byte không có Content-Length (chunked) — để kiểm chặn theo size của manifest.
fn serve_oversize(actual: usize) -> (Srv, String) {
    let b = body(actual);
    serve(move |rq| {
        let _ = rq.respond(tiny_http::Response::new(200.into(), vec![], std::io::Cursor::new(b.clone()), None, None));
    })
}

fn serve_status(code: u16) -> (Srv, String) {
    serve(move |rq| {
        let _ = rq.respond(tiny_http::Response::empty(code));
    })
}

fn sha_hex(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

fn manifest(url: &str, sha: &str, size: u64) -> Manifest {
    Manifest { file: "m.onnx".into(), url: url.into(), sha256: sha.into(), size, license: "test".into(), notice_files: vec![] }
}

#[test]
fn downloads_verifies_and_marks_ready() {
    let (_s, url, b) = serve_bytes(3 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&b), b.len() as u64));
    assert!(matches!(st.status(), ModelStatus::Missing));
    let p = st.ensure(&|_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b);
    assert_eq!(sha256_file(&p).unwrap(), sha_hex(&b));
    assert!(matches!(st.status(), ModelStatus::Ready(_)));
}

#[test]
fn bad_checksum_rejected() {
    let (_s, url, b) = serve_bytes(1 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &"00".repeat(32), b.len() as u64));
    assert!(matches!(st.ensure(&|_, _| {}, &AtomicBool::new(false)), Err(EmbedError::ChecksumMismatch { .. })));
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn oversize_body_aborted() {
    let (_s, url) = serve_oversize(4 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &"00".repeat(32), 1 << 20));
    assert!(matches!(st.ensure(&|_, _| {}, &AtomicBool::new(false)), Err(EmbedError::ChecksumMismatch { .. })));
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn interrupted_download_is_cancelled_and_partial() {
    let (_s, url, b) = serve_slow(4 << 20, 64 << 10, Duration::from_millis(20));
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&b), b.len() as u64));
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
    // app chết giữa chừng: .part còn lại
    std::fs::write(d.path().join(format!("{}.part", st.file_name())), &b[..1000]).unwrap();
    assert!(matches!(st.status(), ModelStatus::Partial));
}

#[test]
fn tampered_ready_file_is_corrupt() {
    let (_s, url, b) = serve_bytes(1 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&b), b.len() as u64));
    let p = st.ensure(&|_, _| {}, &AtomicBool::new(false)).unwrap();
    let mut x = std::fs::read(&p).unwrap();
    x[10] ^= 0xff;
    std::thread::sleep(Duration::from_millis(20)); // mtime khác
    std::fs::write(&p, x).unwrap();
    assert!(matches!(st.status(), ModelStatus::Corrupt));
}

#[test]
fn http_error_is_io() {
    let (_s, url) = serve_status(404);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &"00".repeat(32), 10));
    assert!(matches!(st.ensure(&|_, _| {}, &AtomicBool::new(false)), Err(EmbedError::Io(_))));
}

#[test]
fn concurrent_ensure_downloads_once() {
    let (srv, url, b) = serve_bytes(1 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&b), b.len() as u64));
    std::thread::scope(|sc| {
        for _ in 0..3 {
            sc.spawn(|| st.ensure(&|_, _| {}, &AtomicBool::new(false)).unwrap());
        }
    });
    assert_eq!(srv.requests(), 1);
}
