//! Hàng đợi nền: claim lô từ store → render song song → encode → whiten → ghi một lần cho cả lô.
//! Tạm dừng / huỷ qua cờ nguyên tử; huỷ giữa lúc encode trả face về `pending`, không ghi gì.
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use fontatlas_core::{FontFile, standard_location};
use fontatlas_store::{EmbedJob, FaceKey, JobResult, Store};
use rayon::prelude::*;

use crate::{BATCH, CANVAS, DIM, EmbedError, Encode, RAW_DIM, Whitening, render_input, to_tensor};

const PARK: Duration = Duration::from_millis(50);

pub struct WorkerConfig {
    pub store_dir: PathBuf,
    pub raw_version: String,
    pub whitening: Option<Whitening>,
    pub render_threads: usize,
}

#[derive(Debug, Clone)]
pub enum Progress {
    Started { pending: usize },
    Batch { done: usize, failed: usize, skipped: usize },
    Paused,
    Resumed,
    Cancelled,
    Finished { total_done: usize },
    /// Lỗi hệ thống làm dừng worker; `code` = `EmbedError::code()` (không còn gộp hết thành "inference").
    Error { code: &'static str, message: String },
}

pub struct EmbedWorker {
    handle: JoinHandle<Result<(), EmbedError>>,
    pause: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    model_version: Option<String>,
}

impl EmbedWorker {
    pub fn start(
        cfg: WorkerConfig,
        encoder: Box<dyn Encode>,
        on_progress: Box<dyn Fn(Progress) + Send>,
    ) -> Result<EmbedWorker, EmbedError> {
        let store = Store::open(&cfg.store_dir)?.0;
        if !store.embed_enabled()? {
            return Err(EmbedError::Disabled);
        }
        if let Some(w) = &cfg.whitening
            && (w.raw_dim != RAW_DIM || w.dim != DIM)
        {
            return Err(EmbedError::Model(format!("whitening {}→{} không khớp {RAW_DIM}→{DIM}", w.raw_dim, w.dim)));
        }
        let model_version = cfg
            .whitening
            .as_ref()
            .map(|w| crate::model_version(&cfg.raw_version, &w.version, &w.sha256));
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(cfg.render_threads.max(1))
            .build()
            .map_err(|e| EmbedError::Render(e.to_string()))?;
        let pause = Arc::new(AtomicBool::new(false));
        let cancel = Arc::new(AtomicBool::new(false));
        let mut run = Run {
            store,
            cfg,
            mv: model_version.clone().unwrap_or_default(),
            encoder,
            emit: on_progress,
            pool,
            pause: pause.clone(),
            cancel: cancel.clone(),
        };
        let handle = std::thread::Builder::new()
            .name("fontatlas-embed".into())
            .spawn(move || {
                let r = run.run();
                if let Err(e) = &r {
                    (run.emit)(Progress::Error { code: e.code(), message: e.to_string() });
                }
                r
            })?;
        Ok(EmbedWorker { handle, pause, cancel, model_version })
    }

    pub fn pause(&self) {
        self.pause.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.pause.store(false, Ordering::SeqCst);
        self.handle.thread().unpark();
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.handle.thread().unpark();
    }

    pub fn join(self) -> Result<(), EmbedError> {
        self.handle
            .join()
            .unwrap_or_else(|_| Err(EmbedError::Model("luồng embedding panic".into())))
    }

    /// `None` khi không có whitening (chỉ ghi vector thô).
    pub fn model_version(&self) -> Option<&str> {
        self.model_version.as_deref()
    }
}

struct Run {
    store: Store,
    cfg: WorkerConfig,
    mv: String,
    encoder: Box<dyn Encode>,
    emit: Box<dyn Fn(Progress) + Send>,
    pool: rayon::ThreadPool,
    pause: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

const TENSOR_LEN: usize = 3 * (CANVAS * CANVAS) as usize;

impl Run {
    fn run(&mut self) -> Result<(), EmbedError> {
        let raw = self.cfg.raw_version.clone();
        self.store.reset_running_embeddings()?;
        self.store.requeue_missing_embeddings(&raw)?;
        let st = self.store.enqueue_embeddings(&raw)?;
        self.rewhiten(&raw)?;
        (self.emit)(Progress::Started { pending: st.pending });
        let mut total_done = 0;
        loop {
            if self.pause.load(Ordering::SeqCst) && !self.cancel.load(Ordering::SeqCst) {
                (self.emit)(Progress::Paused);
                while self.pause.load(Ordering::SeqCst) && !self.cancel.load(Ordering::SeqCst) {
                    std::thread::park_timeout(PARK);
                }
                if !self.cancel.load(Ordering::SeqCst) {
                    (self.emit)(Progress::Resumed);
                }
            }
            if self.cancel.load(Ordering::SeqCst) {
                (self.emit)(Progress::Cancelled);
                return Ok(());
            }
            let mut jobs = self.store.claim_embedding_batch(&raw, BATCH)?;
            if jobs.is_empty() {
                // Face mới có thể đã vào thư viện trong lúc chạy.
                self.store.enqueue_embeddings(&raw)?;
                jobs = self.store.claim_embedding_batch(&raw, BATCH)?;
            }
            if jobs.is_empty() {
                if self.cfg.whitening.is_some() {
                    self.rewhiten(&raw)?;
                    self.store.gc_embeddings(&[&raw, &self.mv])?;
                }
                (self.emit)(Progress::Finished { total_done });
                return Ok(());
            }
            let keys: Vec<FaceKey> = jobs.iter().map(|j| j.key).collect();
            let r = self.batch(&raw, &jobs);
            if r.is_err() {
                // Lỗi hệ thống (encoder/store): trả lô về `pending`, không đánh lỗi từng face.
                let _ = self.store.release_embeddings(&raw, &keys);
            }
            match r? {
                Some(done) => total_done += done,
                None => {
                    self.store.release_embeddings(&raw, &keys)?;
                    (self.emit)(Progress::Cancelled);
                    return Ok(());
                }
            }
        }
    }

    /// Face có vector thô nhưng thiếu vector whitening hiện tại (whitening mới, hoặc lần trước chạy không whitening):
    /// whiten lại từ vector thô, không chạy model.
    fn rewhiten(&mut self, raw: &str) -> Result<(), EmbedError> {
        if let Some(w) = &self.cfg.whitening {
            self.store.rewhiten(raw, &self.mv, &|v| w.apply(v))?;
        }
        Ok(())
    }

    /// `None` = bị huỷ giữa lúc encode (chưa ghi gì). `Err` = lỗi hệ thống, lô chưa ghi.
    fn batch(&mut self, raw: &str, jobs: &[EmbedJob]) -> Result<Option<usize>, EmbedError> {
        let prepared: Vec<Result<Vec<f32>, JobResult>> = self.pool.install(|| {
            jobs.par_iter()
                .map(|j| {
                    catch_unwind(AssertUnwindSafe(|| prepare_with(j, &fontatlas_store::is_dataless))).unwrap_or_else(|_| {
                        Err(JobResult::Failed { code: "render_failed", detail: "panic khi render".into() })
                    })
                })
                .collect()
        });
        let tensors: Vec<Vec<f32>> = prepared.iter().filter_map(|p| p.as_ref().ok().cloned()).collect();
        let vecs = if tensors.is_empty() {
            Vec::new()
        } else {
            match self.encoder.encode(&tensors, &self.cancel) {
                Err(EmbedError::Cancelled) => return Ok(None),
                Err(e) => return Err(e),
                Ok(v) if v.len() != tensors.len() => {
                    return Err(EmbedError::Model(format!("encoder trả {} vector cho {} ảnh", v.len(), tensors.len())));
                }
                Ok(v) => v,
            }
        };
        let mut vecs = vecs.into_iter();
        let (mut done, mut failed, mut skipped) = (0, 0, 0);
        let mut results = Vec::with_capacity(jobs.len());
        for (j, p) in jobs.iter().zip(prepared) {
            let res = match p {
                Err(r) => r,
                Ok(_) => {
                    let v = vecs.next().unwrap_or_default();
                    match &self.cfg.whitening {
                        None => JobResult::Done { raw: v, white: None },
                        Some(w) => match w.apply(&v) {
                            Some(white) => JobResult::Done { raw: v, white: Some(white) },
                            None => JobResult::Failed { code: "inference", detail: "vector không hợp lệ".into() },
                        },
                    }
                }
            };
            match res {
                JobResult::Done { .. } => done += 1,
                JobResult::Failed { .. } => failed += 1,
                JobResult::Skipped { .. } => skipped += 1,
                JobResult::Deferred { .. } => {
                    // file là placeholder cloud mà cờ trong DB chưa biết: ghi cờ để lượt claim sau bỏ qua, không quay vòng
                    let _ = self.store.refresh_cloud_flag(&j.path);
                }
            }
            results.push((j.key, res));
        }
        self.store.finish_embeddings(raw, &self.mv, &results)?;
        (self.emit)(Progress::Batch { done, failed, skipped });
        Ok(Some(done))
    }
}

/// Đọc file, kiểm hash, render, ra tensor. Lỗi → kết quả cuối của job.
/// File còn là placeholder cloud (`is_dataless`, chỉ stat) KHÔNG được mở: đọc sẽ kéo cả file về → `Deferred`.
fn prepare_with(j: &EmbedJob, is_dataless: &dyn Fn(&std::path::Path) -> bool) -> Result<Vec<f32>, JobResult> {
    let fail = |code: &'static str, detail: String| JobResult::Failed { code, detail };
    if is_dataless(&j.path) {
        return Err(JobResult::Deferred { code: "cloud_only" });
    }
    let bytes = std::fs::read(&j.path).map_err(|e| fail("missing_file", e.to_string()))?;
    if blake3::hash(&bytes).as_bytes() != &j.key.hash {
        return Err(fail("stale_file", "file đã đổi sau khi quét".into()));
    }
    let file = FontFile::from_bytes(bytes).map_err(|e| fail("render_failed", e.to_string()))?;
    let faces = file.faces().map_err(|e| fail("render_failed", e.to_string()))?;
    let face = faces
        .iter()
        .find(|f| f.index() == j.key.face_index)
        .ok_or_else(|| fail("render_failed", format!("không có face {}", j.key.face_index)))?;
    let loc = standard_location(face.font());
    let img = match render_input(face.font(), &loc) {
        Ok(i) => i,
        Err(EmbedError::NoGlyphs) => return Err(JobResult::Skipped { code: "no_glyphs" }),
        Err(e) => return Err(fail("render_failed", e.to_string())),
    };
    let mut t = vec![0f32; TENSOR_LEN];
    to_tensor(&img, &mut t);
    Ok(t)
}

#[cfg(test)]
mod cloud_tests {
    use super::*;

    fn job(path: &str) -> EmbedJob {
        EmbedJob { key: FaceKey { hash: [0; 32], face_index: 0 }, path: path.into() }
    }

    #[test]
    fn dataless_file_is_deferred_before_any_read() {
        // đường dẫn không tồn tại: nếu code mở file thì sẽ ra Failed(missing_file), không phải Deferred
        match prepare_with(&job("/khong/ton/tai.ttf"), &|_| true) {
            Err(JobResult::Deferred { code }) => assert_eq!(code, "cloud_only"),
            other => panic!("{:?}", other.map(|v| v.len())),
        }
    }

    #[test]
    fn ordinary_file_is_still_read() {
        match prepare_with(&job("/khong/ton/tai.ttf"), &|_| false) {
            Err(JobResult::Failed { code, .. }) => assert_eq!(code, "missing_file"),
            other => panic!("{:?}", other.map(|v| v.len())),
        }
    }
}
