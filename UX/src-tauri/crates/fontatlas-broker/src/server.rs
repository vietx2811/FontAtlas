//! Máy chủ HTTP localhost. Mỗi yêu cầu qua đủ cổng: Host → Origin → token → giới hạn tần suất → route → thân.
//! Yêu cầu CHƯA xác thực chỉ được trừ vào một xô nhỏ riêng, nên trang web/ứng dụng lạ không hút cạn xô của app thật.
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use tiny_http::{Header, Method, Request, Response, StatusCode};

use crate::config::{BrokerFile, new_token};
use crate::ensure::{EnsureError, EnsureRequest, EnsureResponse};
use crate::limit::TokenBucket;
use crate::PROTOCOL_VERSION;

/// Thân yêu cầu tối đa (byte).
const MAX_BODY: u64 = 128 * 1024;
const WORKERS: usize = 4;

/// Phần thật sự làm việc; server chỉ lo giao vận.
pub trait Backend: Send + Sync + 'static {
    /// Công tắc người dùng; tắt → 503.
    fn enabled(&self) -> bool {
        true
    }
    fn ensure(&self, req: &EnsureRequest) -> Result<EnsureResponse, EnsureError>;
}

pub struct ServerConfig {
    /// Thư mục ghi `broker.json`.
    pub dir: PathBuf,
    /// Origin được phép ngoài `null` và `file://` (panel CEP chạy từ file nên gửi `Origin: null`).
    pub extra_origins: Vec<String>,
    /// Xô token: dung lượng và số yêu cầu/giây.
    pub burst: u32,
    pub per_sec: u32,
    /// Xô riêng cho yêu cầu chưa xác thực (sai/thiếu token, preflight): nhỏ để kẻ lạ không làm gì được ngoài việc tự nhận 429.
    pub anon_burst: u32,
    pub anon_per_sec: u32,
}

impl ServerConfig {
    pub fn new(dir: &Path) -> Self {
        Self { dir: dir.to_path_buf(), extra_origins: vec![], burst: 60, per_sec: 30, anon_burst: 20, anon_per_sec: 5 }
    }
}

pub struct Server {
    addr: SocketAddr,
    token: String,
    dir: PathBuf,
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

struct Shared {
    token: String,
    port: u16,
    backend: Arc<dyn Backend>,
    bucket: TokenBucket,
    anon: TokenBucket,
    extra_origins: Vec<String>,
}

impl Server {
    /// Nghe `127.0.0.1:0`, sinh token, ghi `broker.json` (0600).
    pub fn start(cfg: ServerConfig, backend: Arc<dyn Backend>) -> std::io::Result<Server> {
        let http = tiny_http::Server::http("127.0.0.1:0").map_err(|e| std::io::Error::other(e.to_string()))?;
        let addr = http.server_addr().to_ip().ok_or_else(|| std::io::Error::other("không lấy được địa chỉ nghe"))?;
        let token = new_token()?;
        BrokerFile { v: PROTOCOL_VERSION, port: addr.port(), token: token.clone(), pid: std::process::id() }.write(&cfg.dir)?;
        let shared = Arc::new(Shared {
            token: token.clone(),
            port: addr.port(),
            backend,
            bucket: TokenBucket::new(cfg.burst, cfg.per_sec),
            anon: TokenBucket::new(cfg.anon_burst, cfg.anon_per_sec),
            extra_origins: cfg.extra_origins,
        });
        let http = Arc::new(http);
        let stop = Arc::new(AtomicBool::new(false));
        let workers = (0..WORKERS)
            .map(|_| {
                let (http, shared, stop) = (http.clone(), shared.clone(), stop.clone());
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        match http.recv_timeout(Duration::from_millis(200)) {
                            Ok(Some(req)) => handle(&shared, req),
                            Ok(None) => {}
                            Err(_) => break,
                        }
                    }
                })
            })
            .collect();
        Ok(Server { addr, token, dir: cfg.dir, stop, workers })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Có đang trả lời không (GET /v1/ping qua socket thật, hạn 1,5 s)? 200 hoặc 503 (đang tắt bằng công tắc) đều là "sống".
    /// Dùng cho trạng thái Settings: số luồng còn sống không đáng tin nếu có worker chết.
    pub fn probe(&self) -> bool {
        use std::io::Write;
        let t = Duration::from_millis(1500);
        let Ok(mut s) = std::net::TcpStream::connect_timeout(&self.addr, t) else { return false };
        let _ = s.set_read_timeout(Some(t));
        let _ = s.set_write_timeout(Some(t));
        let port = self.addr.port();
        let q = format!("GET /v1/ping HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n\r\n", self.token);
        if s.write_all(q.as_bytes()).is_err() {
            return false;
        }
        let mut head = [0u8; 16];
        let mut got = 0;
        while got < head.len() {
            match std::io::Read::read(&mut s, &mut head[got..]) {
                Ok(0) | Err(_) => break,
                Ok(n) => got += n,
            }
        }
        let line = String::from_utf8_lossy(&head[..got]);
        line.starts_with("HTTP/1.1 200") || line.starts_with("HTTP/1.1 503")
    }

    /// Dừng nhận yêu cầu và xoá `broker.json` của mình.
    pub fn shutdown(mut self) {
        self.stop_inner();
    }

    fn stop_inner(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
        BrokerFile::remove_if_mine(&self.dir, std::process::id());
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("header hợp lệ")
}

fn get_header(req: &Request, name: &str) -> Option<String> {
    req.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name)).map(|h| h.value.as_str().to_string())
}

/// So sánh không rò độ dài phần khớp.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn origin_allowed(origin: &str, extra: &[String]) -> bool {
    origin == "null" || origin.starts_with("file://") || extra.iter().any(|o| o == origin)
}

fn host_ok(host: &str, port: u16) -> bool {
    host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}")
}

struct Reply {
    status: u16,
    body: String,
    cors: Option<String>,
    retry_after: bool,
    preflight: bool,
}

impl Reply {
    fn json(status: u16, body: String) -> Reply {
        Reply { status, body, cors: None, retry_after: false, preflight: false }
    }
    fn err(status: u16, code: &str, msg: &str) -> Reply {
        let body = serde_json::json!({"ok": false, "error": code, "message": msg}).to_string();
        Reply::json(status, body)
    }
}

fn send(req: Request, r: Reply) {
    let mut resp = Response::from_string(r.body).with_status_code(StatusCode(r.status));
    resp.add_header(header("Content-Type", "application/json; charset=utf-8"));
    resp.add_header(header("Cache-Control", "no-store"));
    resp.add_header(header("X-Content-Type-Options", "nosniff"));
    if let Some(o) = r.cors {
        resp.add_header(header("Access-Control-Allow-Origin", &o));
        resp.add_header(header("Vary", "Origin"));
    }
    if r.preflight {
        resp.add_header(header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"));
        resp.add_header(header("Access-Control-Allow-Headers", "Authorization, Content-Type"));
        resp.add_header(header("Access-Control-Max-Age", "600"));
    }
    if r.retry_after {
        resp.add_header(header("Retry-After", "1"));
    }
    let _ = req.respond(resp);
}

fn handle(sh: &Shared, mut req: Request) {
    // Một panic trong backend không được giết worker (4 worker chết dần thì UI vẫn báo "đang chạy").
    let reply = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| route(sh, &mut req)))
        .unwrap_or_else(|_| Reply::err(500, "internal", "lỗi nội bộ"));
    send(req, reply);
}

fn rate_limited() -> Reply {
    let mut r = Reply::err(429, "rate_limited", "quá nhiều yêu cầu, thử lại sau");
    r.retry_after = true;
    r
}

fn route(sh: &Shared, req: &mut Request) -> Reply {
    // DNS rebinding: trang web lạ trỏ tên miền của nó về 127.0.0.1 sẽ mang Host của nó.
    if !get_header(req, "Host").is_some_and(|h| host_ok(&h, sh.port)) {
        return Reply::err(403, "bad_host", "Host không hợp lệ");
    }
    let origin = get_header(req, "Origin");
    if let Some(o) = &origin {
        if !origin_allowed(o, &sh.extra_origins) {
            return Reply::err(403, "bad_origin", "Origin không được phép");
        }
    }
    let cors = origin.clone();
    let mut reply = dispatch(sh, req, origin.is_some());
    reply.cors = cors;
    reply
}

fn dispatch(sh: &Shared, req: &mut Request, has_origin: bool) -> Reply {
    let path = req.url().split('?').next().unwrap_or("").to_string();
    // Preflight của trình duyệt/CEP (Authorization khiến XHR chéo origin phải hỏi trước). Không cần token, nên đi xô riêng.
    if *req.method() == Method::Options {
        if !sh.anon.try_take() {
            return rate_limited();
        }
        let mut r = Reply::json(204, String::new());
        r.preflight = true;
        return if has_origin { r } else { Reply::err(400, "bad_request", "không phải preflight") };
    }
    // Token TRƯỚC mọi thứ khác: yêu cầu không có token đúng không đụng tới xô của app thật.
    let auth = get_header(req, "Authorization").unwrap_or_default();
    let given = auth.strip_prefix("Bearer ").unwrap_or("");
    if !ct_eq(given.as_bytes(), sh.token.as_bytes()) {
        return if sh.anon.try_take() { Reply::err(401, "unauthorized", "thiếu hoặc sai token") } else { rate_limited() };
    }
    if !sh.bucket.try_take() {
        return rate_limited();
    }
    let known = matches!(path.as_str(), "/v1/ping" | "/v1/ensure");
    if !known {
        return Reply::err(404, "not_found", "không có đường dẫn này");
    }
    let want = if path == "/v1/ping" { Method::Get } else { Method::Post };
    if *req.method() != want {
        return Reply::err(405, "method_not_allowed", "sai phương thức");
    }
    if !sh.backend.enabled() {
        return Reply::err(503, "disabled", "tự bật font đang tắt trong Font Atlas");
    }
    if path == "/v1/ping" {
        return Reply::json(200, serde_json::json!({"ok": true, "v": PROTOCOL_VERSION}).to_string());
    }
    if req.body_length().is_some_and(|n| n as u64 > MAX_BODY) {
        return Reply::err(413, "too_large", "thân yêu cầu quá lớn");
    }
    let mut body = Vec::new();
    if req.as_reader().take(MAX_BODY + 1).read_to_end(&mut body).is_err() || body.len() as u64 > MAX_BODY {
        return Reply::err(413, "too_large", "thân yêu cầu quá lớn hoặc không đọc được");
    }
    let parsed: EnsureRequest = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => return Reply::err(400, "bad_json", &format!("JSON không hợp lệ: {e}")),
    };
    match sh.backend.ensure(&parsed) {
        Ok(resp) => Reply::json(200, serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into())),
        Err(EnsureError::BadRequest(msg)) => Reply::err(400, "bad_request", &msg),
        Err(EnsureError::Internal(msg)) => Reply::err(500, "internal", &msg),
    }
}
