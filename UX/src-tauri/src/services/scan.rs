//! Hàng đợi quét: một luồng, mỗi nguồn một lần (single-flight), tạm dừng/tiếp tục/huỷ, tiến độ qua hub.
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fontatlas_store::{SourceKind, Store};

use crate::dto::events::{ScanEvent, ScanTrigger};
use crate::dto::library::{RescanStatsDto, ScanProgressDto, ScanStatusDto};
use crate::error::CmdError;
use crate::dto::tasks::{TaskKind, TaskOutcome, TaskState};
use crate::services::tasks::TaskSpec;
use crate::services::{access, boot};
use crate::state::AppState;

#[derive(Default)]
struct Queue {
    queue: VecDeque<i64>,
    /// Ai xếp lượt đang chờ của từng nguồn (chỉ id trong `queue`). Xếp thêm bởi nguồn mạnh hơn thì nâng lên (User > Launch > Watcher).
    triggers: HashMap<i64, ScanTrigger>,
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
    /// Vị trí vắng lần đầu (xem `Store::absent`): giữ ở đây vì luồng quét thoát khi hết việc và mở kết nối mới ở lượt sau.
    absent: Mutex<HashMap<String, (i64, i64)>>,
    /// Có lượt quét nào từ lần hết việc trước làm đổi thư viện (font mới/đổi/mất, lỗi, huỷ giữa chừng)? Chỉ khi đó mới dựng lại Index.
    dirty: AtomicBool,
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
        self.enqueue_with(state, ids, false, ScanTrigger::User);
    }

    /// Như `enqueue`, ghi rõ ai kích (FE dùng để quyết định có hiện "đang quét" không).
    pub fn enqueue_as(&self, state: &Arc<AppState>, ids: &[i64], trigger: ScanTrigger) {
        self.enqueue_with(state, ids, false, trigger);
    }

    /// Như `enqueue` nhưng id đang được quét VẪN được xếp thêm một lượt SAU lượt hiện tại (lượt hiện tại có thể đã đi qua
    /// phần vừa thay đổi, vd file vừa tải về máy). Id đã nằm sẵn trong hàng thì không xếp trùng. Trả `false` khi đang tắt app.
    pub fn enqueue_follow_up(&self, state: &Arc<AppState>, ids: &[i64]) -> bool {
        self.enqueue_with(state, ids, true, ScanTrigger::User)
    }

    /// `enqueue_follow_up` với người kích rõ ràng (watcher dùng `Watcher`).
    pub fn enqueue_follow_up_as(&self, state: &Arc<AppState>, ids: &[i64], trigger: ScanTrigger) -> bool {
        self.enqueue_with(state, ids, true, trigger)
    }

    fn enqueue_with(&self, state: &Arc<AppState>, ids: &[i64], follow_up: bool, trigger: ScanTrigger) -> bool {
        if self.inner.shutdown.load(Ordering::SeqCst) {
            return false;
        }
        let mut spawn = false;
        let mut fresh = Vec::new();
        let mut promoted = Vec::new();
        {
            let mut q = lock(&self.inner.q);
            for &id in ids {
                if q.queue.contains(&id) {
                    // đã chờ sẵn: không xếp trùng, nhưng người dùng xếp thì lượt đó thành của người dùng
                    let e = q.triggers.entry(id).or_insert(trigger);
                    if trigger.rank() > e.rank() {
                        *e = trigger;
                        promoted.push(id);
                    }
                } else if follow_up || q.current != Some(id) {
                    q.queue.push_back(id);
                    q.triggers.insert(id, trigger);
                    state.hubs.scan.emit(ScanEvent::Queued { source_id: id, trigger });
                    fresh.push(id);
                }
            }
            if !q.queue.is_empty() && !q.running {
                q.running = true;
                spawn = true;
            }
        }
        // khay "Tác vụ": ngoài khoá hàng đợi (tra tên nguồn chạm user.db)
        for id in fresh {
            task_queued(state, id, trigger);
        }
        for id in promoted {
            if let Some(t) = state.tasks.id_of(&scan_key(id)) {
                state.tasks.set_trigger(&t, trigger);
            }
        }
        if spawn {
            let (st, inner) = (Arc::clone(state), Arc::clone(&self.inner));
            let h = std::thread::Builder::new().name("fontatlas-scan".into()).spawn(move || worker(st, inner));
            if let Ok(h) = h {
                *lock(&self.inner.worker) = Some(h);
            }
        }
        true
    }

    /// Nguồn đang là lượt quét hiện tại (không tính hàng đợi).
    pub fn is_current(&self, id: i64) -> bool {
        lock(&self.inner.q).current == Some(id)
    }

    /// Nguồn đang nằm trong hàng chờ (chưa chạy).
    pub fn is_queued(&self, id: i64) -> bool {
        lock(&self.inner.q).queue.contains(&id)
    }

    pub fn enqueue_all_directories(&self, state: &Arc<AppState>) -> Result<(), CmdError> {
        self.enqueue_all_directories_as(state, ScanTrigger::User)
    }

    pub fn enqueue_all_directories_as(&self, state: &Arc<AppState>, trigger: ScanTrigger) -> Result<(), CmdError> {
        let ids = directory_source_ids(state)?;
        self.enqueue_as(state, &ids, trigger);
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

    /// App đang tắt (các lượt nền không nên xếp thêm việc).
    pub fn is_shutting_down(&self) -> bool {
        self.inner.shutdown.load(Ordering::SeqCst)
    }

    /// Đặt cờ tắt + huỷ (không chờ).
    pub fn request_shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::SeqCst);
        self.inner.cancel.store(true, Ordering::SeqCst);
        let mut q = lock(&self.inner.q);
        q.queue.clear();
        q.triggers.clear();
    }

    /// Bỏ nguồn khỏi hàng đợi; đang quét nguồn đó thì huỷ và chờ xong (≤ 10 s). `false` = chưa dừng kịp.
    pub fn forget(&self, id: i64) -> bool {
        let running = {
            let mut q = lock(&self.inner.q);
            q.queue.retain(|x| *x != id);
            q.triggers.remove(&id);
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

    /// Huỷ MỘT nguồn theo yêu cầu của khay "Tác vụ": đang chờ → bỏ khỏi hàng; đang quét → đặt cờ huỷ (dừng ở ranh giới lô).
    /// Trả ngay, không chờ. `false` = nguồn không còn trong hàng lẫn đang quét.
    pub fn cancel_source(&self, id: i64) -> bool {
        let mut q = lock(&self.inner.q);
        if q.current == Some(id) {
            self.inner.cancel.store(true, Ordering::SeqCst);
            return true;
        }
        let before = q.queue.len();
        q.queue.retain(|x| *x != id);
        q.triggers.remove(&id);
        q.queue.len() != before
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

    /// Đang quét hoặc còn nguồn chờ quét.
    pub fn is_busy(&self) -> bool {
        let q = lock(&self.inner.q);
        q.running || !q.queue.is_empty()
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
                    let trigger = q.triggers.remove(&id).unwrap_or(ScanTrigger::User);
                    inner.cancel.store(false, Ordering::SeqCst);
                    Some((id, trigger))
                }
                _ => {
                    q.current = None;
                    None
                }
            }
        };
        let Some((id, trigger)) = next else {
            // hết việc: báo Idle và chạy các móc; `running` giữ true tới khi móc xong để `wait_idle` chờ đủ.
            // Đang tắt app thì bỏ móc (không mở worker embed hay đăng ký lại font sau `on_exit`).
            state.hubs.scan.emit(ScanEvent::Idle);
            // việc quét còn sót trong khay (bị `forget`/tắt app lấy mất khỏi hàng) không được kẹt lại
            state.tasks.retain_kind(TaskKind::Scan, |_| false, TaskOutcome::Cancelled);
            if !inner.shutdown.load(Ordering::SeqCst) {
                after_idle(&state, inner.dirty.swap(false, Ordering::SeqCst));
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
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_job(&state, &inner, &mut store, id, trigger)));
        if r.is_err() {
            state
                .hubs
                .scan
                .emit(ScanEvent::Failed { source_id: id, error: CmdError::internal("luồng quét gặp lỗi không lường trước") });
            fail_task(&state, id);
        }
        lock(&inner.q).current = None;
        inner.idle.notify_all(); // `forget` chờ nguồn đang quét dừng
    }
}

fn run_job(state: &Arc<AppState>, inner: &Arc<Inner>, store: &mut Result<Store, fontatlas_store::StoreError>, id: i64, trigger: ScanTrigger) {
    let s = match store.as_mut() {
        Ok(s) => s,
        Err(e) => {
            state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: CmdError::new("store_unavailable", e.to_string()) });
            fail_task(state, id);
            return;
        }
    };
    let root = match state.with_user(|u| Ok(u.source(id)?)) {
        Ok(src) => src.path.map(std::path::PathBuf::from),
        Err(e) => {
            state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: e });
            fail_task(state, id);
            return;
        }
    };
    let Some(root) = root else {
        state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: CmdError::not_found("nguồn không có thư mục") });
        fail_task(state, id);
        return;
    };
    // macOS có thể treo ở opendir khi chờ cấp quyền: thử mở thư mục gốc có giới hạn thời gian trước khi quét.
    let probe = access::probe_dir(&root, access::timeout());
    if let Some(error) = access::to_error(&root, &probe) {
        state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error });
        fail_task(state, id);
        return;
    }
    let task = task_running(state, id, trigger, &root);
    s.set_absent(std::mem::take(&mut *lock(&inner.absent)));
    let r = s.rescan_root_with(
        id,
        &root,
        &mut |p| {
            while inner.paused.load(Ordering::SeqCst) && !inner.cancel.load(Ordering::SeqCst) && !inner.shutdown.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(50));
            }
            state.hubs.scan.emit(ScanEvent::Progress { source_id: id, progress: ScanProgressDto::from(p), trigger });
            state.tasks.progress(&task, p.done as u64, (p.total > 0).then_some(p.total as u64));
            if p.faces_written > 0 {
                state.hubs.scan.emit(ScanEvent::FacesAdded { source_id: id, count: p.faces_written as u32, trigger });
            }
        },
        &inner.cancel,
    );
    *lock(&inner.absent) = s.take_absent();
    match r {
        Ok(stats) => {
            let changed = changed_library(&stats);
            if changed {
                inner.dirty.store(true, Ordering::SeqCst);
            }
            let pending_absent = stats.pending_absent;
            let dto = RescanStatsDto::from(&stats);
            lock(&inner.last).insert(id, dto.clone());
            let outcome = if stats.status == fontatlas_store::RescanStatus::Cancelled { TaskOutcome::Cancelled } else { TaskOutcome::Done };
            state.hubs.scan.emit(ScanEvent::Finished { source_id: id, stats: dto, changed, trigger });
            state.tasks.finish(&task, outcome, None);
            if pending_absent > 0 {
                schedule_verify(state, id);
            }
        }
        Err(e) => {
            inner.dirty.store(true, Ordering::SeqCst);
            state.hubs.scan.emit(ScanEvent::Failed { source_id: id, error: e.into() });
            state.tasks.finish(&task, TaskOutcome::Failed, Some("task.error.scan"));
        }
    }
}

/// Lượt quét thất bại trước khi bắt đầu: gỡ việc (nếu đã đăng ký) với kết quả lỗi.
fn fail_task(state: &Arc<AppState>, id: i64) {
    if let Some(t) = state.tasks.id_of(&scan_key(id)) {
        state.tasks.finish(&t, TaskOutcome::Failed, Some("task.error.scan"));
    }
}

fn scan_key(id: i64) -> String {
    format!("scan:{id}")
}

/// Tên ngắn cho tiêu đề (tên nguồn hoặc thư mục cuối); không bao giờ đường dẫn đầy đủ.
fn short_name(path: &std::path::Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "…".to_string())
}

fn scan_spec(state: &Arc<AppState>, id: i64, trigger: ScanTrigger, name: Option<String>) -> TaskSpec {
    let weak = Arc::downgrade(state);
    let mut spec = TaskSpec::new(TaskKind::Scan, "task.scan")
        .trigger(trigger)
        .group(&format!("scan:{id}"))
        .cancellable(Arc::new(move || {
            if let Some(st) = weak.upgrade() {
                let was_current = st.scans.is_current(id);
                st.scans.cancel_source(id);
                // chưa chạy: đã bỏ khỏi hàng, gỡ khỏi khay ngay (đang chạy thì `run_job` gỡ khi dừng ở ranh giới lô)
                if !was_current && let Some(t) = st.tasks.id_of(&scan_key(id)) {
                    st.tasks.finish(&t, TaskOutcome::Cancelled, None);
                }
            }
        }));
    if let Some(n) = name {
        spec = spec.arg("source", n);
    }
    spec
}

/// Một lượt quét vừa được xếp: đăng ký việc "chờ" (đã có việc cùng nguồn thì giữ nguyên).
fn task_queued(state: &Arc<AppState>, id: i64, trigger: ScanTrigger) {
    if state.tasks.id_of(&scan_key(id)).is_some() {
        return;
    }
    let name = state.with_user(|u| Ok(u.source(id)?)).ok().and_then(|s| s.path).map(|p| short_name(std::path::Path::new(&p)));
    state.tasks.begin(Some(&scan_key(id)), scan_spec(state, id, trigger, name), TaskState::Queued);
}

/// Lượt quét bắt đầu chạy: chuyển việc sang "đang chạy" (chưa có thì tạo, vd lượt đầu không đi qua `enqueue`).
fn task_running(state: &Arc<AppState>, id: i64, trigger: ScanTrigger, root: &std::path::Path) -> String {
    let key = scan_key(id);
    let tid = state.tasks.begin(Some(&key), scan_spec(state, id, trigger, Some(short_name(root))), TaskState::Running);
    state.tasks.set_arg(&tid, "source", short_name(root));
    state.tasks.set_trigger(&tid, trigger);
    state.tasks.running(&tid);
    tid
}

/// Sau bao lâu quét xác nhận một nguồn còn vị trí vắng lần đầu (xem `Store::absent`). Đủ dài để stub↔file thật/ghi tạm rồi đổi
/// tên xong, đủ ngắn để xoá thật biến mất khỏi lưới ngay sau đó.
pub const VERIFY_DELAY: Duration = Duration::from_secs(5);

/// Xếp một lượt quét xác nhận (trigger `watcher`, không nháy UI) sau `VERIFY_DELAY`. Không giữ AppState sống và không chạy khi đang tắt.
fn schedule_verify(state: &Arc<AppState>, id: i64) {
    let weak = Arc::downgrade(state);
    let _ = std::thread::Builder::new().name("fontatlas-scan-verify".into()).spawn(move || {
        std::thread::sleep(VERIFY_DELAY);
        if let Some(st) = weak.upgrade()
            && !st.scans.is_shutting_down()
        {
            st.scans.enqueue_follow_up_as(&st, &[id], ScanTrigger::Watcher);
        }
    });
}

/// Lượt quét có làm đổi thư viện không? Lượt "không có gì mới" (vd quét lại do watcher báo, mọi file không đổi) KHÔNG được
/// kéo theo dựng lại Index (đắt với thư viện lớn).
fn changed_library(s: &fontatlas_store::RescanStats) -> bool {
    s.status != fontatlas_store::RescanStatus::Ok
        || s.analyzed + s.relinked.saturating_sub(s.touched) + s.failed + s.missing + s.purged > 0
}

/// Việc nối tiếp khi hàng đợi quét về rỗng (mỗi task sau thêm móc của mình).
fn after_idle(state: &Arc<AppState>, library_changed: bool) {
    boot::resolve_legacy_if_needed(state);
    boot::mark_analyzer_seen_if_current(state);
    crate::services::activation::reconcile_now(state, false);
    let _ = crate::services::embed::ensure_started(state);
    state.langs.bump();
    crate::services::langs::ensure_started(state);
    if library_changed {
        crate::index_bundle::request_rebuild(state, crate::index_bundle::RebuildWhy::ScanIdle);
    }
}
