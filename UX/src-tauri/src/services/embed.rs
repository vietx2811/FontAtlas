//! Hàng đợi embedding: mở/đóng worker (một worker một lúc), trạng thái, ETA, tạm dừng, thoát gọn.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fontatlas_embed::{EmbedWorker, Progress, Whitening, WorkerConfig, raw_version};

use crate::dto::events::EmbedEvent;
use crate::dto::library::FaceKeyDto;
use crate::dto::model::{EmbedFailureDto, EmbedStatusDto};
use crate::error::CmdError;
use crate::index_bundle::{RebuildWhy, current_model_version, request_rebuild};
use crate::services::model;
use crate::state::AppState;

/// ms/face tham chiếu trước lô đầu (UX 03): Metal 28, CPU 30.
const DEFAULT_MS_METAL: f32 = 28.0;
const DEFAULT_MS_CPU: f32 = 30.0;
/// Dựng lại Index sau mỗi chừng này face xong (có debounce ở `request_rebuild`).
const REBUILD_EVERY: u64 = 512;

#[derive(Default)]
struct Stats {
    /// ms/face của 5 lô gần nhất.
    batches: VecDeque<f32>,
    last: Option<Instant>,
    device: Option<&'static str>,
    load_error: Option<String>,
    model_version: Option<String>,
    done_run: u64,
}

#[derive(Default)]
struct Inner {
    worker: Mutex<Option<EmbedWorker>>,
    stats: Mutex<Stats>,
    paused: AtomicBool,
    running: AtomicBool,
    /// Chỉ một `ensure_started` chạy một lúc (tránh hai worker khi tải model xong cùng lúc quét xong).
    start_guard: Mutex<()>,
}

#[derive(Default)]
pub struct EmbedSupervisor {
    inner: Arc<Inner>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl EmbedSupervisor {
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    pub fn pause(&self) {
        self.inner.paused.store(true, Ordering::SeqCst);
        if let Some(w) = lock(&self.inner.worker).as_ref() {
            w.pause();
        }
    }

    pub fn resume(&self) {
        self.inner.paused.store(false, Ordering::SeqCst);
        if let Some(w) = lock(&self.inner.worker).as_ref() {
            w.resume();
        }
    }

    /// Chỉ đặt cờ huỷ (không chờ): dùng khi tắt app để cả ba hàng đợi cùng dừng song song.
    pub fn request_stop(&self) {
        if let Some(w) = lock(&self.inner.worker).as_ref() {
            w.cancel();
        }
    }

    /// Huỷ worker hiện tại và chờ nó dừng.
    pub fn stop(&self) {
        self.stop_within(Duration::from_secs(30));
    }

    /// Thoát app: huỷ và chờ ≤ 5 s.
    pub fn shutdown(&self) {
        self.stop_within(Duration::from_secs(5));
    }

    fn stop_within(&self, limit: Duration) {
        let w = lock(&self.inner.worker).take();
        let Some(w) = w else { return };
        w.cancel();
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = std::thread::Builder::new().name("fontatlas-embed-join".into()).spawn(move || {
            let _ = tx.send(w.join());
        });
        let _ = rx.recv_timeout(limit);
        self.inner.running.store(false, Ordering::SeqCst);
    }

    pub fn load_error(&self) -> Option<String> {
        lock(&self.inner.stats).load_error.clone()
    }
}

/// Mở worker nếu: embedding bật, model `Ready`, chưa có worker chạy. Không có model → `Ok(false)`, không lỗi.
pub fn ensure_started(state: &Arc<AppState>) -> Result<bool, CmdError> {
    let _start = lock(&state.embed.inner.start_guard);
    let enabled = state.with_user(|u| Ok(u.embed_enabled()?))?;
    let Some(path) = model::ready_path(state) else { return Ok(false) };
    if !enabled {
        return Ok(false);
    }
    let sup = &state.embed;
    // worker cũ đã xong thì join trước khi mở mới
    let old = {
        let mut g = lock(&sup.inner.worker);
        if sup.inner.running.load(Ordering::SeqCst) && g.is_some() {
            return Ok(true);
        }
        g.take()
    };
    if let Some(w) = old {
        let _ = w.join();
    }
    let (encoder, device) = match (state.deps.encoder)(&path) {
        Ok(x) => x,
        Err(e) => {
            lock(&sup.inner.stats).load_error = Some(e.code().to_string());
            emit_status(state);
            return Ok(false);
        }
    };
    let w = Whitening::bundled();
    let mv = current_model_version_for(state);
    {
        let mut s = lock(&sup.inner.stats);
        *s = Stats { device: Some(device), model_version: Some(mv), ..Default::default() };
    }
    let cfg = WorkerConfig {
        store_dir: state.dirs.data.clone(),
        raw_version: raw_version(&state.deps.manifest.sha256),
        whitening: Some(w),
        render_threads: std::thread::available_parallelism().map(|n| n.get().saturating_sub(1).max(1)).unwrap_or(2),
    };
    let st = Arc::clone(state);
    let started = EmbedWorker::start(cfg, encoder, Box::new(move |p| on_progress(&st, p)));
    match started {
        Ok(worker) => {
            sup.inner.running.store(true, Ordering::SeqCst);
            if sup.inner.paused.load(Ordering::SeqCst) {
                worker.pause();
            }
            *lock(&sup.inner.worker) = Some(worker);
            Ok(true)
        }
        Err(e) => {
            lock(&sup.inner.stats).load_error = Some(e.code().to_string());
            emit_status(state);
            Ok(false)
        }
    }
}

fn current_model_version_for(state: &AppState) -> String {
    let w = Whitening::bundled();
    fontatlas_embed::model_version(&raw_version(&state.deps.manifest.sha256), &w.version, &w.sha256)
}

fn on_progress(state: &Arc<AppState>, p: Progress) {
    let sup = &state.embed.inner;
    match p {
        Progress::Started { .. } => {
            lock(&sup.stats).last = Some(Instant::now());
        }
        Progress::Batch { done, failed, skipped } => {
            let n = (done + failed + skipped).max(1) as f32;
            let crossed = {
                let mut s = lock(&sup.stats);
                let now = Instant::now();
                if let Some(t) = s.last {
                    let ms = now.duration_since(t).as_secs_f32() * 1000.0 / n;
                    if s.batches.len() == 5 {
                        s.batches.pop_front();
                    }
                    s.batches.push_back(ms);
                }
                s.last = Some(now);
                let before = s.done_run;
                s.done_run += done as u64;
                before / REBUILD_EVERY != s.done_run / REBUILD_EVERY
            };
            if crossed {
                request_rebuild(state, RebuildWhy::EmbedProgress);
            }
        }
        Progress::Paused => sup.paused.store(true, Ordering::SeqCst),
        Progress::Resumed => sup.paused.store(false, Ordering::SeqCst),
        Progress::Cancelled => sup.running.store(false, Ordering::SeqCst),
        Progress::Finished { total_done } => {
            sup.running.store(false, Ordering::SeqCst);
            request_rebuild(state, RebuildWhy::QueueEmpty);
            state.hubs.embed.emit(EmbedEvent::Finished { total_done: total_done as u64 });
        }
        Progress::Error(_) => {
            lock(&sup.stats).load_error = Some("inference".into());
            sup.running.store(false, Ordering::SeqCst);
        }
    }
    emit_status(state);
}

fn emit_status(state: &Arc<AppState>) {
    if let Ok(status) = embed_status(state) {
        state.hubs.embed.emit(EmbedEvent::Status { status });
    }
}

/// Đếm công việc embedding từ `pipeline_state` (bền qua các lần chạy).
fn counts(state: &AppState) -> (u64, u64, u64) {
    state
        .with_store(|s| {
            let q = |sql: &str| -> u64 { s.conn().query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0).max(0) as u64 };
            Ok((
                q("SELECT count(*) FROM pipeline_state WHERE stage='embedding' AND status IN ('pending','running')"),
                q("SELECT count(*) FROM pipeline_state WHERE stage='embedding' AND status='done'"),
                q("SELECT count(*) FROM pipeline_state WHERE stage='embedding' AND status='failed'"),
            ))
        })
        .unwrap_or((0, 0, 0))
}

pub fn embed_status(state: &AppState) -> Result<EmbedStatusDto, CmdError> {
    let enabled = state.with_user(|u| Ok(u.embed_enabled()?))?;
    let (pending, done, failed) = counts(state);
    let sup = &state.embed.inner;
    let s = lock(&sup.stats);
    let ms_per_face = if s.batches.is_empty() {
        s.device.map(|d| if d == "metal" { DEFAULT_MS_METAL } else { DEFAULT_MS_CPU })
    } else {
        Some(s.batches.iter().sum::<f32>() / s.batches.len() as f32)
    };
    let eta_ms = ms_per_face.filter(|_| pending > 0).map(|m| (pending as f32 * m) as u64);
    Ok(EmbedStatusDto {
        enabled,
        running: sup.running.load(Ordering::SeqCst),
        paused: sup.paused.load(Ordering::SeqCst),
        pending,
        done,
        failed,
        device: s.device.map(String::from),
        ms_per_face,
        eta_ms,
        model_version: s.model_version.clone().or_else(|| Some(current_model_version()).filter(|_| model::ready_path(state).is_some())),
        load_error: s.load_error.clone(),
    })
}

pub fn embed_failures(state: &AppState) -> Result<Vec<EmbedFailureDto>, CmdError> {
    let rows = state.with_store(|s| Ok(s.problem_files()?))?;
    Ok(rows
        .into_iter()
        .filter(|p| p.stage == "embedding")
        .filter_map(|p| {
            Some(EmbedFailureDto {
                face: FaceKeyDto { hash_hex: p.hash_hex, face_index: p.face_index? },
                code: p.code,
                detail: p.detail,
            })
        })
        .collect())
}

/// Tạm dừng cả quét lẫn embed (B4).
pub fn pause_all(state: &AppState) {
    state.scans.pause(state);
    state.embed.pause();
}

pub fn resume_all(state: &AppState) {
    state.scans.resume(state);
    state.embed.resume();
}
