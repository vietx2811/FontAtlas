//! Tải model về máy: ghi `.part`, kiểm size + sha256, fsync, đổi tên nguyên tử, sidecar `.ok`, khoá chống chạy chồng.
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::EmbedError;

/// Thời gian tối đa chờ một chunk dữ liệu (không phải cả file).
pub const READ_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK: usize = 64 << 10;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub file: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub license: String,
    pub notice_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStatus {
    Missing,
    Partial,
    Ready(PathBuf),
    Corrupt,
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
        let io = |e: ureq::Error| EmbedError::Io(std::io::Error::other(e.to_string()));
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(READ_TIMEOUT))
            .build()
            .into();
        let resp = agent.get(&self.m.url).call().map_err(io)?;
        if resp.status().as_u16() != 200 {
            return Err(EmbedError::Io(std::io::Error::other(format!("http {}", resp.status()))));
        }
        let limit = self.m.size.saturating_add(1);
        let mut reader = resp.into_body().into_with_config().limit(limit.max(1)).reader();
        // Đọc ở luồng phụ: luồng chính chờ từng chunk có timeout và kiểm cờ huỷ.
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
        let mut out = File::create(part)?;
        let mut h = Sha256::new();
        let (mut done, mut last_mb) = (0u64, 0u64);
        let mut idle_since = Instant::now();
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(EmbedError::Cancelled);
            }
            let chunk = match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(v)) => v,
                Ok(Err(e)) if done >= self.m.size => {
                    // vượt limit của ureq (size+1) → coi là quá cỡ
                    let _ = e;
                    return Err(self.mismatch("oversize"));
                }
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
            if done > self.m.size {
                return Err(self.mismatch("oversize"));
            }
            h.update(&chunk);
            out.write_all(&chunk)?;
            if done / (1 << 20) > last_mb {
                last_mb = done / (1 << 20);
                progress(done, self.m.size);
            }
        }
        out.sync_all()?;
        let got = format!("{:x}", h.finalize());
        if done != self.m.size || got != self.m.sha256 {
            return Err(EmbedError::ChecksumMismatch { expected: self.m.sha256.clone(), got });
        }
        progress(done, self.m.size);
        Ok(())
    }

    fn mismatch(&self, why: &str) -> EmbedError {
        EmbedError::ChecksumMismatch { expected: self.m.sha256.clone(), got: why.into() }
    }
}
