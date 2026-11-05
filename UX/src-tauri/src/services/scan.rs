//! Hàng đợi quét: một luồng, mỗi nguồn một lần (single-flight), tạm dừng/tiếp tục/huỷ, tiến độ qua hub.
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fontatlas_store::{SourceKind, Store};

use crate::dto::events::ScanEvent;
use crate::dto::library::{RescanStatsDto, ScanProgressDto, ScanStatusDto};
use crate::error::CmdError;
use crate::services::boot;
use crate::state::AppState;

#[derive(Default)]
struct Queue {
    queue: VecDeque<i64>,
    current: Option<i64>,
    running: bool,
}

#[derive(Default)]
struct Inner {
    q: Mutex<Queue>,
    idle: Condvar,
    paused: AtomicBool,
    cancel: AtomicBool,
    shutdown: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
    last: Mutex<HashMap<i64, RescanStatsDto>>,
}

#[derive(Default)]
pub struct ScanManager {
    inner: Arc<Inner>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl ScanManager {
    /// Thêm vào hàng đợi (bỏ id đang chạy hoặc đã có trong hàng). Không chặn.
    pub fn enqueue(&self, state: &Arc<AppState>, ids: &[i64]) {
        if self.inner.shutdown.load(Ordering::SeqCst) {
            return;
        }
        let mut spawn = false;
        {
            let mut q = lock(&self.inner.q);
            for &id in ids {
                if q.current != Some(id) && !q.queue.contains(&id) {
                    q.queue.push_back(id);
                    state.hubs.scan.emit(ScanEvent::Queued { source_id: id });
                }
            }
            if !q.queue.is_empty() && !q.running {
                q.running = true;
                spawn = true;
            }
        }
        if spawn {
            let (st, inner) = (Arc::clone(state), Arc::clone(&self.inner));
            let h = std::thread::Builder::new().name("fontatlas-scan".into()).spawn(move || worker(st, inner));
            if let Ok(h) = h {
                *lock(&self.inner.worker) = Some(h);
            }
        }
    }

    pub fn enqueue_all_directories(&self, state: &Arc<AppState>) -> Result<(), CmdError> {
        let ids = directory_source_ids(state)?;
        self.enqueue(state, &ids);
        Ok(())
    }

    pub fn pause(&self, state: &AppState) {
        if !self.inner.paused.swap(true, Ordering::SeqCst) {
            state.hubs.scan.emit(ScanEvent::Paused);
        }
    }

    pub fn resume(&self, state: &AppState) {
        if self.inner.paused.swap(false, Ordering::SeqCst) {
            state.hubs.scan.emit(ScanEvent::Resumed);
        }
    }

    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    /// Đặt cờ tắt + huỷ (không chờ).
    pub fn request_shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::SeqCst);
        self.inner.cancel.store(true, Ordering::SeqCst);
        lock(&self.inner.q).queue.clear();
    }

    /// Bỏ nguồn khỏi hàng đợi; đang quét nguồn đó thì huỷ và chờ xong (≤ 10 s). `false` = chưa dừng kịp.
    pub fn forget(&self, id: i64) -> bool {
        let running = {
            let mut q = lock(&self.inner.q);
            q.queue.retain(|x| *x != id);
            q.current == Some(id)
        };
        if !running {
            return true;
        }
        self.inner.cancel.store(true, Ordering::SeqCst);
        let q = lock(&self.inner.q);
        let (_g, r) = self
            .inner
            .idle
            .wait_timeout_while(q, Duration::from_secs(10), |q| q.current == Some(id))
            .unwrap_or_else(|e| e.into_inner());
        !r.timed_out()
    }

    /// Huỷ nguồn đang quét (các nguồn còn trong hàng vẫn chạy).
    pub fn cancel_current(&self) {
        self.inner.cancel.store(true, Ordering::SeqCst);
    }

    /// Huỷ và chờ luồng quét kết thúc (≤ 5 s).
    pub fn shutdown(&self) {
        self.request_shutdown();
        let h = lock(&self.inner.worker).take();
        if let Some(h) = h {
            let t = Instant::now();
            while !h.is_finished() && t.elapsed() < Duration::from_secs(5) {
                std::thread::sleep(Duration::from_millis(20));
            }
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }

    /// Chờ tới khi hàng đợi rỗng và luồng quét nghỉ.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let q = lock(&self.inner.q);
        let (_guard, r) = self
            .inner
            .idle
            .wait_timeout_while(q, timeout, |q| q.running || !q.queue.is_empty())
            .unwrap_or_else(|e| e.into_inner());
        !r.timed_out()
    }

    pub fn status(&self) -> ScanStatusDto {
        let q = lock(&self.inner.q);
        ScanStatusDto {
            running: q.running,
            paused: self.is_paused(),
            current_source_id: q.current,
            queued: q.queue.iter().copied().collect(),
        }
    }

    pub fn is_scanning(&self, id: i64) -> bool {
        let q = lock(&self.inner.q);
        q.current == Some(id) || q.queue.contains(&id)
    }

    pub fn last_scan(&self, id: i64) -> Option<RescanStatsDto> {
        lock(&self.inner.last).get(&id).cloned()
    }
}

pub fn directory_source_ids(state: &AppState) -> Result<Vec<i64>, CmdError> {
    state.with_user(|u| {
        Ok(u.sources()?
            .into_iter()
            .filter(|s| s.kind == SourceKind::Directory && s.enabled && s.path.is_some())
            .map(|s| s.id)
            .collect())
    })
}

fn worker(state: Arc<AppState>, inner: Arc<Inner>) {
    // kết nối chỉ library.db: lô ingest không khoá user.db (yêu thích trong lúc quét không phải chờ)
    let mut store = Store::open_library_only(&state.dirs.data);
    loop {
        let next = {
            let mut q = lock(&inner.q);
            match q.queue.pop_front() {
                Some(id) if !inner.shutdown.load(Ordering::SeqCst) => {
                    q.current = Some(id);
                    inner.cancel.store(false, Ordering::SeqCst);
                    Some(id)
                }
                _ => {
                    q.current = None;
                    None
                }
            }
        };
        let Some(id) = next else {
            // hết việc: báo Idle và chạy các móc; `running` giữ true tới khi móc xong để `wait_idle` chờ đủ.
            // Đang tắt app thì bỏ móc (không mở worker embed hay đăng ký lại font sau `on_exit`).
            state.hubs.scan.emit(ScanEvent::Idle);
            if !inner.shutdown.load(Ordering::SeqCst) {
                after_idle(&state);
            }
            let mut q = lock(&inner.q);
            if q.queue.is_empty() || inner.shutdown.load(Ordering::SeqCst) {
                q.running = false;
                inner.idle.notify_all();
                break;
            }
            continue;
        };
        // panic trong quét/callback không được làm kẹt `running`: coi như job lỗi và đi tiếp
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_job(&state, &inner, &mut store, id)));
        if r.is_err() {
            state
                .hubs
                .scan
                .emit(ScanEvent::Failed { source_id: id, error: CmdError::internal("luồng quét gặp lỗi không lường trước") });
        }
        lock(&inner.q).current = None;
        inner.idle.notify_all(); // `forget` chờ nguồn đang quét dừng
    }
}

fn run_job(state: &Arc<AppState>, inner: &Arc<Inner>, store: &mut Result<Store, fontatlas_store::StoreError>, id: i64) {
    let s = match store.as_mut() {
        Ok(s) => s,
        Err(e) => {
            state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: CmdError::new("store_unavailable", e.to_string()) });
            return;
        }
    };
    let root = match state.with_user(|u| Ok(u.source(id)?)) {
        Ok(src) => src.path.map(std::path::PathBuf::from),
        Err(e) => {
            state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: e });
            return;
        }
    };
    let Some(root) = root else {
        state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: CmdError::not_found("nguồn không có thư mục") });
        return;
    };
    let r = s.rescan_root_with(
        id,
        &root,
        &mut |p| {
            while inner.paused.load(Ordering::SeqCst) && !inner.cancel.load(Ordering::SeqCst) && !inner.shutdown.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(50));
            }
            state.hubs.scan.emit(ScanEvent::Progress { source_id: id, progress: ScanProgressDto::from(p) });
            if p.faces_written > 0 {
                state.hubs.scan.emit(ScanEvent::FacesAdded { source_id: id, count: p.faces_written as u32 });
            }
        },
        &inner.cancel,
    );
    match r {
        Ok(stats) => {
            let dto = RescanStatsDto::from(&stats);
            lock(&inner.last).insert(id, dto.clone());
            state.hubs.scan.emit(ScanEvent::Finished { source_id: id, stats: dto });
        }
        Err(e) => state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: e.into() }),
    }
}

/// Việc nối tiếp khi hàng đợi quét về rỗng (mỗi task sau thêm móc của mình).
fn after_idle(state: &Arc<AppState>) {
    boot::resolve_legacy_if_needed(state);
    boot::mark_analyzer_seen_if_current(state);
    crate::services::activation::reconcile_now(state, false);
    let _ = crate::services::embed::ensure_started(state);
    crate::index_bundle::request_rebuild(state, crate::index_bundle::RebuildWhy::ScanIdle);
}
