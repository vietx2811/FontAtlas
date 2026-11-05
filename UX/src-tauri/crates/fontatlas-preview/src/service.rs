//! Dịch vụ preview: render theo yêu cầu, cache, thế hệ do backend sở hữu, lô theo viewport.
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use fontatlas_core::FontFile;
use rayon::prelude::*;

use crate::{PreviewCache, PreviewError, cache_key, check_px, pool_for, render_face};

/// Tra đường dẫn file theo (hash, face). Plan 6 cài bằng Store.
pub trait FaceLocator: Send + Sync {
    fn locate(&self, hash_hex: &str, face_index: u32) -> Option<PathBuf>;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    pub hash_hex: String,
    pub face_index: u32,
    pub px: u32,
    pub text: String,
    pub generation: Option<u64>,
}

pub struct PreviewService<L: FaceLocator> {
    cache: Mutex<PreviewCache>,
    locator: L,
    generation: AtomicU64,
    renders: AtomicU64,
    reads: AtomicU64,
}

impl<L: FaceLocator> PreviewService<L> {
    pub fn new(cache: PreviewCache, locator: L) -> Self {
        PreviewService {
            cache: Mutex::new(cache),
            locator,
            generation: AtomicU64::new(0),
            renders: AtomicU64::new(0),
            reads: AtomicU64::new(0),
        }
    }

    /// (tổng byte hai vùng, số mục).
    pub fn cache_stats(&self) -> Result<(u64, u64), PreviewError> {
        let c = self.cache.lock().map_err(|_| PreviewError::Encode("khoá cache hỏng".into()))?;
        Ok((c.total_bytes(crate::Pool::Default) + c.total_bytes(crate::Pool::Custom), c.len()?))
    }

    pub fn clear_cache(&self) -> Result<(), PreviewError> {
        self.cache.lock().map_err(|_| PreviewError::Encode("khoá cache hỏng".into()))?.clear()
    }

    /// Xin thế hệ mới (khi chữ mẫu hoặc cỡ đổi). Chỉ tăng.
    pub fn bump_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
    pub fn locator(&self) -> &L {
        &self.locator
    }
    pub fn render_count(&self) -> u64 {
        self.renders.load(Ordering::Relaxed)
    }
    pub fn file_reads(&self) -> u64 {
        self.reads.load(Ordering::Relaxed)
    }

    /// Kiểm thế hệ + tham số rồi tra cache. `Ok(Some)` = trúng.
    fn precheck(&self, req: &PreviewRequest) -> Result<Option<Vec<u8>>, PreviewError> {
        if req.generation.is_some_and(|g| g < self.generation()) {
            return Err(PreviewError::StaleGeneration);
        }
        check_px(req.px)?;
        if req.hash_hex.len() != 64 || !req.hash_hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(PreviewError::BadRequest(format!("hash: {}", req.hash_hex)));
        }
        let key = cache_key(&req.hash_hex, req.face_index, req.px, &req.text);
        // Lỗi cache (DB hỏng, mutex poison) → coi như miss.
        let hit = self.cache.lock().ok().and_then(|mut c| match c.get(&key) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("preview cache get: {e}");
                None
            }
        });
        Ok(hit)
    }

    fn open_checked(&self, req: &PreviewRequest) -> Result<FontFile, PreviewError> {
        let path = self.locator.locate(&req.hash_hex, req.face_index).ok_or(PreviewError::NotFound)?;
        self.reads.fetch_add(1, Ordering::Relaxed);
        let file = FontFile::open(&path).map_err(|e| if e.code() == "io" { PreviewError::NotFound } else { e.into() })?;
        if file.hash_hex() != req.hash_hex {
            return Err(PreviewError::StaleFile);
        }
        Ok(file)
    }

    fn render_from(&self, file: &FontFile, req: &PreviewRequest) -> Result<Vec<u8>, PreviewError> {
        let faces = file.faces()?;
        let face = faces.iter().find(|f| f.index() == req.face_index).ok_or(PreviewError::NotFound)?;
        let r = render_face(face, &req.text, req.px)?;
        self.renders.fetch_add(1, Ordering::Relaxed);
        let key = cache_key(&req.hash_hex, req.face_index, req.px, &req.text);
        if let Ok(mut c) = self.cache.lock() {
            if let Err(e) = c.put(&key, pool_for(&req.text), &r.png) {
                eprintln!("preview cache put: {e}");
            }
        }
        Ok(r.png)
    }

    pub fn get(&self, req: &PreviewRequest) -> Result<Vec<u8>, PreviewError> {
        if let Some(png) = self.precheck(req)? {
            return Ok(png);
        }
        let file = self.open_checked(req)?;
        self.render_from(&file, req)
    }

    /// Dựng trước cho viewport: mỗi file đọc một lần, song song theo file; kết quả theo thứ tự đầu vào.
    pub fn prewarm(&self, reqs: &[PreviewRequest]) -> Vec<Result<(), PreviewError>> {
        let mut out: Vec<Option<Result<(), PreviewError>>> = Vec::with_capacity(reqs.len());
        let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, r) in reqs.iter().enumerate() {
            match self.precheck(r) {
                Ok(Some(_)) => out.push(Some(Ok(()))),
                Ok(None) => {
                    out.push(None);
                    groups.entry(r.hash_hex.as_str()).or_default().push(i);
                }
                Err(e) => out.push(Some(Err(e))),
            }
        }
        let done: Vec<(usize, Result<(), PreviewError>)> = groups
            .into_par_iter()
            .flat_map_iter(|(_, idxs)| {
                let file = self.open_checked(&reqs[idxs[0]]);
                idxs.into_iter()
                    .map(|i| {
                        let r = match &file {
                            Ok(f) => self.render_from(f, &reqs[i]).map(|_| ()),
                            Err(e) => Err(clone_err(e)),
                        };
                        (i, r)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        for (i, r) in done {
            out[i] = Some(r);
        }
        out.into_iter().map(|r| r.unwrap_or(Err(PreviewError::NotFound))).collect()
    }
}

/// Lỗi mở file dùng chung cho cả nhóm (PreviewError không Clone vì bọc rusqlite/CoreError).
fn clone_err(e: &PreviewError) -> PreviewError {
    match e {
        PreviewError::NotFound => PreviewError::NotFound,
        PreviewError::StaleFile => PreviewError::StaleFile,
        // Lỗi đọc/parse font (422) giữ đúng loại; các lỗi khác không xảy ra ở bước mở file.
        other => PreviewError::Core(fontatlas_core::CoreError::Parse(other.to_string())),
    }
}
