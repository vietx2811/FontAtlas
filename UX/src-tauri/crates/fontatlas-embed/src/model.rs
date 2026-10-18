//! Tải model về máy từ file safetensors chính thức: chỉ lấy header + đoạn `vision_model.*` bằng HTTP Range
//! (server không hỗ trợ Range → stream cả file, chỉ ghi phần cần, kiểm sha toàn file), ghép thành file
//! safetensors cục bộ tất định; ghi `.part`, kiểm sha từng đoạn + file cục bộ, fsync, đổi tên nguyên tử,
//! sidecar `.ok`, khoá chống chạy chồng.
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::EmbedError;

/// Thời gian tối đa chờ một chunk dữ liệu (không phải cả file).
pub const READ_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK: usize = 64 << 10;

/// File safetensors chính thức trên máy chủ (đã ghim revision). Vị trí tính bằng byte tuyệt đối trong file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub size: u64,
    pub sha256: String,
    /// Độ dài JSON header (không gồm 8 byte độ dài).
    pub header_len: u64,
    /// sha256 của `[0, 8 + header_len)`.
    pub header_sha256: String,
    pub vision_offset: u64,
    pub vision_len: u64,
    pub vision_sha256: String,
}

/// `file`/`sha256`/`size`: file safetensors cục bộ chỉ chứa `vision_model.*` (ghép tất định từ nguồn).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub file: String,
    pub sha256: String,
    pub size: u64,
    pub license: String,
    pub notice_files: Vec<String>,
    pub source: Source,
}

pub const VISION_PREFIX: &str = "vision_model.";
const MAX_HEADER: u64 = 64 << 20;

fn sha_hex(h: Sha256) -> String {
    format!("{:x}", h.finalize())
}

fn bad(m: &str) -> EmbedError {
    EmbedError::Model(format!("safetensors: {m}"))
}

/// Thứ tự trường cố định (khớp file đã ghim: data_offsets, dtype, shape).
#[derive(Serialize)]
struct TensorEntry {
    data_offsets: [u64; 2],
    dtype: String,
    shape: Vec<u64>,
}

#[derive(Serialize)]
struct HeaderOut {
    #[serde(rename = "__metadata__", skip_serializing_if = "Option::is_none")]
    metadata: Option<std::collections::BTreeMap<String, String>>,
    #[serde(flatten)]
    tensors: std::collections::BTreeMap<String, TensorEntry>,
}

/// Từ JSON header của file nguồn → header của file cục bộ (đã kèm 8 byte độ dài, đệm space chia hết 8)
/// và đoạn dữ liệu `[start, end)` (tương đối với vùng dữ liệu) chứa mọi tensor `vision_model.*`.
/// Đoạn phải liền, không lẫn tensor khác.
pub fn vision_header(json: &[u8]) -> Result<(Vec<u8>, u64, u64), EmbedError> {
    let v: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(json).map_err(|e| bad(&e.to_string()))?;
    let offs = |x: &serde_json::Value| -> Option<(u64, u64)> {
        let o = x.get("data_offsets")?.as_array()?;
        Some((o.first()?.as_u64()?, o.get(1)?.as_u64()?))
    };
    let mut vis = Vec::new();
    let mut other = Vec::new();
    for (k, x) in &v {
        if k == "__metadata__" {
            continue;
        }
        let (a, b) = offs(x).filter(|(a, b)| a <= b).ok_or_else(|| bad(&format!("offset của {k}")))?;
        if k.starts_with(VISION_PREFIX) { vis.push((k, a, b)) } else { other.push((a, b)) }
    }
    let start = vis.iter().map(|t| t.1).min().ok_or_else(|| bad("không có tensor vision_model.*"))?;
    let end = vis.iter().map(|t| t.2).max().unwrap_or(start);
    if vis.iter().map(|t| t.2 - t.1).sum::<u64>() != end - start || other.iter().any(|&(a, b)| a < end && b > start) {
        return Err(bad("tensor vision không nằm liền một đoạn"));
    }
    // Ghi tường minh (struct + BTreeMap) để byte header không phụ thuộc feature `preserve_order` của serde_json:
    // sha file cục bộ được ghim trong manifest.
    let mut tensors = std::collections::BTreeMap::new();
    for (k, a, b) in vis {
        let x = &v[k.as_str()];
        let dtype = x.get("dtype").and_then(|d| d.as_str()).ok_or_else(|| bad(&format!("dtype của {k}")))?;
        let shape: Vec<u64> = x
            .get("shape")
            .and_then(|s| s.as_array())
            .and_then(|s| s.iter().map(|d| d.as_u64()).collect())
            .ok_or_else(|| bad(&format!("shape của {k}")))?;
        tensors.insert(k.clone(), TensorEntry { data_offsets: [a - start, b - start], dtype: dtype.to_string(), shape });
    }
    let metadata: Option<std::collections::BTreeMap<String, String>> = match v.get("__metadata__") {
        None => None,
        Some(m) => Some(serde_json::from_value(m.clone()).map_err(|e| bad(&format!("__metadata__: {e}")))?),
    };
    let out = HeaderOut { metadata, tensors };
    let mut h = serde_json::to_vec(&out).map_err(|e| bad(&e.to_string()))?;
    while !(8 + h.len()).is_multiple_of(8) {
        h.push(b' ');
    }
    let mut bytes = (h.len() as u64).to_le_bytes().to_vec();
    bytes.extend(h);
    Ok((bytes, start, end))
}

/// Đọc dòng byte của file nguồn (từ đầu, hoặc header rồi nhảy tới đoạn vision) và ghi file cục bộ.
struct Extract<'a> {
    expect: Option<&'a Source>,
    out: Option<&'a mut File>,
    pos: u64,
    hdr: Vec<u8>,
    hdr_total: Option<u64>,
    header_sha: Option<String>,
    span: Option<(u64, u64)>,
    full: Sha256,
    vision: Sha256,
    local: Sha256,
    local_len: u64,
}

struct Extracted {
    size: u64,
    full_sha: String,
    header_len: u64,
    header_sha: String,
    vision_offset: u64,
    vision_len: u64,
    vision_sha: String,
    local_sha: String,
    local_len: u64,
}

impl<'a> Extract<'a> {
    fn new(expect: Option<&'a Source>, out: Option<&'a mut File>) -> Self {
        Extract {
            expect,
            out,
            pos: 0,
            hdr: Vec::new(),
            hdr_total: None,
            header_sha: None,
            span: None,
            full: Sha256::new(),
            vision: Sha256::new(),
            local: Sha256::new(),
            local_len: 0,
        }
    }

    fn mismatch(&self, what: &str, got: String) -> EmbedError {
        EmbedError::ChecksumMismatch { expected: what.into(), got }
    }

    fn on_header(&mut self) -> Result<(), EmbedError> {
        let mut h = Sha256::new();
        h.update(&self.hdr);
        let hs = sha_hex(h);
        if let Some(e) = self.expect
            && (hs != e.header_sha256 || self.hdr.len() as u64 != 8 + e.header_len)
        {
            return Err(self.mismatch("header", hs));
        }
        let (nh, a, b) = vision_header(&self.hdr[8..])?;
        let base = self.hdr.len() as u64;
        let span = (base + a, base + b);
        if let Some(e) = self.expect
            && span != (e.vision_offset, e.vision_offset + e.vision_len)
        {
            return Err(self.mismatch("vision span", format!("{span:?}")));
        }
        if let Some(o) = self.out.as_mut() {
            o.write_all(&nh)?;
        }
        self.local.update(&nh);
        self.local_len += nh.len() as u64;
        self.header_sha = Some(hs);
        self.span = Some(span);
        Ok(())
    }

    /// Nhảy tới vị trí tuyệt đối `pos` (đường Range); sha toàn file không còn ý nghĩa.
    fn seek(&mut self, pos: u64) {
        self.pos = pos;
    }

    fn feed(&mut self, c: &[u8]) -> Result<(), EmbedError> {
        self.full.update(c);
        let start = self.pos;
        self.pos += c.len() as u64;
        let mut off = 0usize;
        while off < c.len() {
            let abs = start + off as u64;
            let rem = c.len() - off;
            let Some((vo, ve)) = self.span else {
                let need = match self.hdr_total {
                    None => 8 - self.hdr.len(),
                    Some(t) => (t - self.hdr.len() as u64) as usize,
                };
                let take = need.min(rem);
                self.hdr.extend_from_slice(&c[off..off + take]);
                off += take;
                if self.hdr_total.is_none() && self.hdr.len() == 8 {
                    let hl = u64::from_le_bytes(self.hdr[..8].try_into().unwrap_or_default());
                    if hl == 0 || hl > MAX_HEADER {
                        return Err(bad("độ dài header"));
                    }
                    self.hdr_total = Some(8 + hl);
                }
                if self.hdr_total == Some(self.hdr.len() as u64) {
                    self.on_header()?;
                }
                continue;
            };
            if abs < vo {
                off += ((vo - abs) as usize).min(rem);
                continue;
            }
            if abs >= ve {
                break;
            }
            let take = ((ve - abs) as usize).min(rem);
            let b = &c[off..off + take];
            self.vision.update(b);
            self.local.update(b);
            if let Some(o) = self.out.as_mut() {
                o.write_all(b)?;
            }
            self.local_len += take as u64;
            off += take;
        }
        Ok(())
    }

    fn finish(self) -> Result<Extracted, EmbedError> {
        let (Some((vo, ve)), Some(hs), Some(ht)) = (self.span, self.header_sha, self.hdr_total) else {
            return Err(bad("thiếu header"));
        };
        Ok(Extracted {
            size: self.pos,
            full_sha: sha_hex(self.full),
            header_len: ht - 8,
            header_sha: hs,
            vision_offset: vo,
            vision_len: ve - vo,
            vision_sha: sha_hex(self.vision),
            local_sha: sha_hex(self.local),
            local_len: self.local_len,
        })
    }
}

/// Ghép file vision cục bộ từ file nguồn đã có trong máy (tool dev: whitenfit, đo đạc). Giống hệt file `ensure` tải về.
pub fn extract_vision_file(src: &Path, dst: &Path) -> Result<(), EmbedError> {
    let mut f = File::open(src)?;
    let mut out = File::create(dst)?;
    let mut ex = Extract::new(None, Some(&mut out));
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ex.feed(&buf[..n])?;
    }
    ex.finish()?;
    out.sync_all()?;
    Ok(())
}

/// Tính manifest từ file nguồn đã có trong máy (tool phát hành: examples/make_manifest.rs).
pub fn make_manifest(src: &Path, url: &str, file: &str, license: &str, notice_files: Vec<String>) -> Result<Manifest, EmbedError> {
    let mut f = File::open(src)?;
    let mut ex = Extract::new(None, None);
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ex.feed(&buf[..n])?;
    }
    let x = ex.finish()?;
    if x.vision_offset + x.vision_len > x.size {
        return Err(bad("file cụt"));
    }
    Ok(Manifest {
        file: file.into(),
        sha256: x.local_sha,
        size: x.local_len,
        license: license.into(),
        notice_files,
        source: Source {
            url: url.into(),
            size: x.size,
            sha256: x.full_sha,
            header_len: x.header_len,
            header_sha256: x.header_sha,
            vision_offset: x.vision_offset,
            vision_len: x.vision_len,
            vision_sha256: x.vision_sha,
        },
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStatus {
    Missing,
    Partial,
    Ready(PathBuf),
    Corrupt,
}

/// Manifest phát hành kèm app (nguồn: repo chính thức trên Hugging Face, đã ghim revision).
pub fn bundled_manifest() -> Manifest {
    serde_json::from_str(include_str!("../data/model-manifest.json")).expect("data/model-manifest.json hợp lệ")
}

pub struct ModelStore {
    dir: PathBuf,
    m: Manifest,
}

pub fn sha256_file(p: &Path) -> std::io::Result<String> {
    let mut f = File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

fn mtime_ns(p: &Path) -> Option<u128> {
    fs::metadata(p).ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_nanos())
}

impl ModelStore {
    pub fn new(dir: PathBuf, m: Manifest) -> Self {
        ModelStore { dir, m }
    }

    pub fn file_name(&self) -> &str {
        &self.m.file
    }

    fn path(&self, suffix: &str) -> PathBuf {
        self.dir.join(format!("{}{suffix}", self.m.file))
    }

    fn ok_line(&self, p: &Path) -> Option<String> {
        let size = fs::metadata(p).ok()?.len();
        Some(format!("{} {} {}", self.m.sha256, size, mtime_ns(p)?))
    }

    pub fn status(&self) -> ModelStatus {
        let file = self.path("");
        if file.exists() {
            let cur = self.ok_line(&file);
            if cur.is_some() && fs::read_to_string(self.path(".ok")).ok() == cur {
                return ModelStatus::Ready(file);
            }
            // .ok thiếu/lệch → băm lại
            let good = fs::metadata(&file).map(|m| m.len() == self.m.size).unwrap_or(false)
                && sha256_file(&file).map(|h| h == self.m.sha256).unwrap_or(false);
            if good {
                if let Some(l) = cur {
                    let _ = fs::write(self.path(".ok"), l);
                }
                return ModelStatus::Ready(file);
            }
            return ModelStatus::Corrupt;
        }
        if self.path(".part").exists() { ModelStatus::Partial } else { ModelStatus::Missing }
    }

    pub fn remove(&self) -> std::io::Result<()> {
        for s in ["", ".part", ".ok"] {
            match fs::remove_file(self.path(s)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }

    pub fn ensure(&self, progress: &dyn Fn(u64, u64), cancel: &AtomicBool) -> Result<PathBuf, EmbedError> {
        fs::create_dir_all(&self.dir)?;
        let lock = OpenOptions::new().create(true).truncate(false).write(true).open(self.path(".lock"))?;
        lock.lock()?; // nhả khi drop
        match self.status() {
            ModelStatus::Ready(p) => return Ok(p),
            ModelStatus::Corrupt => {
                fs::remove_file(self.path(""))?;
                let _ = fs::remove_file(self.path(".ok"));
            }
            _ => {}
        }
        let part = self.path(".part");
        let _ = fs::remove_file(&part);
        let r = self.download(&part, progress, cancel);
        if r.is_err() {
            let _ = fs::remove_file(&part);
        }
        r?;
        let file = self.path("");
        fs::rename(&part, &file)?;
        File::open(&self.dir)?.sync_all()?;
        if let Some(l) = self.ok_line(&file) {
            fs::write(self.path(".ok"), l)?;
        }
        Ok(file)
    }

    fn download(&self, part: &Path, progress: &dyn Fn(u64, u64), cancel: &AtomicBool) -> Result<(), EmbedError> {
        let src = &self.m.source;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(READ_TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        let mut out = File::create(part)?;
        let hdr_bytes = 8 + src.header_len;
        let mut ex = Extract::new(Some(src), Some(&mut out));
        let (status, cr, body) = get(&agent, &src.url, Some((0, hdr_bytes - 1)))?;
        match status {
            206 => {
                check_range(cr.as_deref(), 0, hdr_bytes - 1, src.size)?;
                let total = hdr_bytes + src.vision_len;
                pump(body, hdr_bytes, cancel, &|d| progress(d, total), &mut |c| ex.feed(c))?;
                let (a, b) = (src.vision_offset, src.vision_offset + src.vision_len - 1);
                let (status, cr, body) = get(&agent, &src.url, Some((a, b)))?;
                if status != 206 {
                    return Err(http(status));
                }
                check_range(cr.as_deref(), a, b, src.size)?;
                ex.seek(a);
                pump(body, src.vision_len, cancel, &|d| progress(hdr_bytes + d, total), &mut |c| ex.feed(c))?;
                let x = ex.finish()?;
                if x.vision_sha != src.vision_sha256 {
                    return Err(EmbedError::ChecksumMismatch { expected: src.vision_sha256.clone(), got: x.vision_sha });
                }
                self.check_local(x, &mut out)
            }
            // Server bỏ qua Range: stream cả file, chỉ ghi header + đoạn vision.
            200 => {
                pump(body, src.size, cancel, &|d| progress(d, src.size), &mut |c| ex.feed(c))?;
                let x = ex.finish()?;
                if x.size != src.size || x.full_sha != src.sha256 {
                    return Err(EmbedError::ChecksumMismatch { expected: src.sha256.clone(), got: x.full_sha });
                }
                self.check_local(x, &mut out)
            }
            s => Err(http(s)),
        }
    }

    fn check_local(&self, x: Extracted, out: &mut File) -> Result<(), EmbedError> {
        out.sync_all()?;
        if x.local_len != self.m.size || x.local_sha != self.m.sha256 {
            return Err(EmbedError::ChecksumMismatch { expected: self.m.sha256.clone(), got: x.local_sha });
        }
        Ok(())
    }
}

fn http(status: u16) -> EmbedError {
    EmbedError::Io(std::io::Error::other(format!("http {status}")))
}

type Body = ureq::Body;

fn get(agent: &ureq::Agent, url: &str, range: Option<(u64, u64)>) -> Result<(u16, Option<String>, Body), EmbedError> {
    let mut rq = agent.get(url);
    if let Some((a, b)) = range {
        rq = rq.header("Range", format!("bytes={a}-{b}"));
    }
    let resp = rq.call().map_err(|e| EmbedError::Io(std::io::Error::other(e.to_string())))?;
    let status = resp.status().as_u16();
    let cr = resp.headers().get("content-range").and_then(|v| v.to_str().ok()).map(str::to_string);
    Ok((status, cr, resp.into_body()))
}

fn check_range(cr: Option<&str>, a: u64, b: u64, total: u64) -> Result<(), EmbedError> {
    let want = format!("bytes {a}-{b}/{total}");
    if cr != Some(want.as_str()) {
        return Err(EmbedError::Io(std::io::Error::other(format!("content-range {cr:?}, cần {want}"))));
    }
    Ok(())
}

/// Đọc thân phản hồi ở luồng phụ; luồng gọi chờ từng chunk có timeout, kiểm cờ huỷ, chặn quá `expected` byte.
fn pump(
    body: Body,
    expected: u64,
    cancel: &AtomicBool,
    progress: &dyn Fn(u64),
    sink: &mut dyn FnMut(&[u8]) -> Result<(), EmbedError>,
) -> Result<(), EmbedError> {
    let mut reader = body.into_with_config().limit(expected.saturating_add(1)).reader();
    let (tx, rx) = mpsc::sync_channel::<std::io::Result<Vec<u8>>>(4);
    std::thread::spawn(move || {
        let mut buf = vec![0u8; CHUNK];
        loop {
            let r = reader.read(&mut buf).map(|n| buf[..n].to_vec());
            let end = matches!(&r, Ok(v) if v.is_empty()) || r.is_err();
            if tx.send(r).is_err() || end {
                return;
            }
        }
    });
    let oversize = || EmbedError::ChecksumMismatch { expected: format!("{expected} byte"), got: "oversize".into() };
    let (mut done, mut last_mb) = (0u64, 0u64);
    let mut idle_since = Instant::now();
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(EmbedError::Cancelled);
        }
        let chunk = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(v)) => v,
            Ok(Err(_)) if done >= expected => return Err(oversize()), // vượt limit size+1 của ureq
            Ok(Err(e)) => return Err(EmbedError::Io(e)),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if idle_since.elapsed() > READ_TIMEOUT {
                    return Err(EmbedError::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout")));
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if chunk.is_empty() {
            break;
        }
        idle_since = Instant::now();
        done += chunk.len() as u64;
        if done > expected {
            return Err(oversize());
        }
        sink(&chunk)?;
        if done / (1 << 20) > last_mb {
            last_mb = done / (1 << 20);
            progress(done);
        }
    }
    if done != expected {
        return Err(EmbedError::ChecksumMismatch { expected: format!("{expected} byte"), got: format!("{done} byte") });
    }
    progress(done);
    Ok(())
}
