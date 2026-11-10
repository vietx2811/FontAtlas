//! Handler thuần cho custom protocol `fontatlas-preview` và `fontatlas-font`.
//! Plan 6 nối vào `register_asynchronous_uri_scheme_protocol` (trong spawn_blocking).
use crate::{
    DEFAULT_SAMPLE, FaceLocator, PreviewError, PreviewRequest, PreviewService, check_px, normalize_text,
};

pub const PREVIEW_SCHEME: &str = "fontatlas-preview";
pub const FONT_SCHEME: &str = "fontatlas-font";

/// Origin được phép đọc phản hồi (CORS). Không bao giờ `*`.
#[derive(Debug, Clone)]
pub struct CorsPolicy {
    pub allowed: Vec<String>,
}
impl CorsPolicy {
    pub fn app() -> CorsPolicy {
        CorsPolicy {
            allowed: ["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]
                .map(String::from)
                .to_vec(),
        }
    }
    pub fn with(mut self, origin: &str) -> CorsPolicy {
        self.allowed.push(origin.to_string());
        self
    }
    fn allow(&self, origin: Option<&str>) -> Option<String> {
        origin.filter(|o| self.allowed.iter().any(|a| a == o)).map(String::from)
    }
}

#[derive(Debug)]
pub struct PreviewHttp {
    pub status: u16,
    pub content_type: &'static str,
    pub cache_control: &'static str,
    pub allow_origin: Option<String>,
    pub vary: &'static str,
    pub body: Vec<u8>,
}

fn bad(msg: impl Into<String>) -> PreviewError {
    PreviewError::BadRequest(msg.into())
}

fn is_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Tên file cuối đường dẫn + URL đã parse. Hỗ trợ `{scheme}://localhost/…` và `http://{scheme}.localhost/…`.
fn last_segment(uri: &str) -> Result<(url::Url, String), PreviewError> {
    let u = url::Url::parse(uri).map_err(|e| bad(format!("url: {e}")))?;
    let name = u
        .path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad("thiếu tên file"))?
        .to_string();
    Ok((u, name))
}

pub fn parse_preview_uri(uri: &str) -> Result<PreviewRequest, PreviewError> {
    let (u, name) = last_segment(uri)?;
    let stem = name.strip_suffix(".png").ok_or_else(|| bad("phải là .png"))?;
    let parts: Vec<&str> = stem.split('_').collect();
    let [hash, idx, px] = parts[..] else { return Err(bad("tên phải là hash_face_px.png")) };
    if !is_hash(hash) {
        return Err(bad("hash"));
    }
    let face_index: u32 = idx.parse().map_err(|_| bad("face_index"))?;
    let px: u32 = px.parse().map_err(|_| bad("px"))?;
    let px = check_px(px)?;
    let mut text = None;
    let mut generation = None;
    for (k, v) in u.query_pairs() {
        match k.as_ref() {
            "t" => text = Some(normalize_text(&v)),
            "g" => generation = Some(v.parse::<u64>().map_err(|_| bad("g"))?),
            _ => {} // r= chỉ để phá cache WebView
        }
    }
    Ok(PreviewRequest {
        hash_hex: hash.to_string(),
        face_index,
        px,
        text: text.unwrap_or_else(|| DEFAULT_SAMPLE.to_string()),
        generation,
    })
}

pub fn parse_font_uri(uri: &str) -> Result<(String, u32), PreviewError> {
    let (_, name) = last_segment(uri)?;
    let stem = name.strip_suffix(".font").ok_or_else(|| bad("phải là .font"))?;
    let (hash, idx) = stem.split_once('_').ok_or_else(|| bad("tên phải là hash_face.font"))?;
    if !is_hash(hash) {
        return Err(bad("hash"));
    }
    Ok((hash.to_string(), idx.parse().map_err(|_| bad("face_index"))?))
}

fn ok(content_type: &'static str, body: Vec<u8>, origin: Option<&str>, cors: &CorsPolicy) -> PreviewHttp {
    PreviewHttp {
        status: 200,
        content_type,
        cache_control: "private, max-age=86400",
        allow_origin: cors.allow(origin),
        vary: "Origin",
        body,
    }
}

fn err(e: PreviewError, origin: Option<&str>, cors: &CorsPolicy) -> PreviewHttp {
    PreviewHttp {
        status: e.http_status(),
        content_type: "text/plain; charset=utf-8",
        cache_control: "no-store",
        allow_origin: cors.allow(origin),
        vary: "Origin",
        body: e.code().as_bytes().to_vec(),
    }
}

pub fn handle_preview_uri<L: FaceLocator>(
    svc: &PreviewService<L>,
    uri: &str,
    origin: Option<&str>,
    cors: &CorsPolicy,
) -> PreviewHttp {
    match parse_preview_uri(uri).and_then(|r| svc.get(&r)) {
        Ok(png) => ok("image/png", png, origin, cors),
        Err(e) => err(e, origin, cors),
    }
}

fn font_bytes<L: FaceLocator>(locator: &L, uri: &str) -> Result<(&'static str, Vec<u8>), PreviewError> {
    let (hash, face) = parse_font_uri(uri)?;
    if face > 0 {
        return Err(PreviewError::TtcFace);
    }
    let path = locator.locate(&hash, face).ok_or(PreviewError::NotFound)?;
    locator.before_read(&path)?;
    let raw = std::fs::read(&path).map_err(|_| PreviewError::NotFound)?;
    if blake3::hash(&raw).to_hex().as_str() != hash {
        return Err(PreviewError::StaleFile);
    }
    let ct = match raw.get(0..4) {
        Some(b"ttcf") => "font/collection",
        Some(b"OTTO") => "font/otf",
        Some(b"wOFF") => "font/woff",
        Some(b"wOF2") => "font/woff2",
        _ => "font/ttf",
    };
    Ok((ct, raw))
}

pub fn handle_font_uri<L: FaceLocator>(locator: &L, uri: &str, origin: Option<&str>, cors: &CorsPolicy) -> PreviewHttp {
    match font_bytes(locator, uri) {
        Ok((ct, raw)) => ok(ct, raw, origin, cors),
        Err(e) => err(e, origin, cors),
    }
}
