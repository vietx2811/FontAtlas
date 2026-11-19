//! TaskHub: nơi duy nhất mọi việc nền đăng ký để khay "Tác vụ" hiển thị và huỷ (thiết kế: docs/design/taskhub-be-beta5.md).
//!
//! Hub CHỈ để hiển thị + huỷ: không kích bump thư viện (việc đó vẫn theo `ScanEvent::Finished.changed`). Không giữ lịch sử:
//! xong/huỷ/lỗi → `Removed` rồi biến mất. Khoá riêng, không bao giờ chạm `AppState.store`.
//!
//! Thứ tự: `rev` cấp và sự kiện phát đều trong cùng một khoá nên người nhận thấy đúng thứ tự (sink là gửi vào kênh, không gọi lại hub).
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::dto::events::ScanTrigger;
use crate::dto::tasks::{TaskArg, TaskDto, TaskEvent, TaskKind, TaskListDto, TaskOutcome, TaskState, TaskUnit};
use crate::hub::ProgressHub;

/// Tiến độ phát tối đa bấy nhiêu lần/giây cho mỗi việc (đổi state luôn phát ngay).
pub const PROGRESS_MIN_GAP: Duration = Duration::from_millis(250);

pub type CancelFn = Arc<dyn Fn() + Send + Sync>;

/// Mô tả một việc khi đăng ký.
pub struct TaskSpec {
    pub kind: TaskKind,
    pub trigger: ScanTrigger,
    pub title_key: String,
    pub title_args: BTreeMap<String, TaskArg>,
    pub unit: TaskUnit,
    pub total: Option<u64>,
    pub cancel_confirm_key: Option<String>,
    pub group_key: Option<String>,
    /// `Some` = huỷ được; hàm chỉ ĐẶT cờ/xoá khỏi hàng, trả ngay.
    pub cancel: Option<CancelFn>,
}

impl TaskSpec {
    pub fn new(kind: TaskKind, title_key: &str) -> Self {
        TaskSpec {
            kind,
            trigger: ScanTrigger::User,
            title_key: title_key.to_string(),
            title_args: BTreeMap::new(),
            unit: TaskUnit::Items,
            total: None,
            cancel_confirm_key: None,
            group_key: None,
            cancel: None,
        }
    }
    pub fn arg(mut self, k: &str, v: impl Into<TaskArg>) -> Self {
        self.title_args.insert(k.to_string(), v.into());
        self
    }
    pub fn trigger(mut self, t: ScanTrigger) -> Self {
        self.trigger = t;
        self
    }
    pub fn unit(mut self, u: TaskUnit) -> Self {
        self.unit = u;
        self
    }
    pub fn total(mut self, t: Option<u64>) -> Self {
        self.total = t;
        self
    }
    pub fn group(mut self, g: &str) -> Self {
        self.group_key = Some(g.to_string());
        self
    }
    pub fn cancellable(mut self, f: CancelFn) -> Self {
        self.cancel = Some(f);
        self
    }
    pub fn confirm_cancel(mut self, key: &str) -> Self {
        self.cancel_confirm_key = Some(key.to_string());
        self
    }
}

struct Entry {
    dto: TaskDto,
    cancel: Option<CancelFn>,
    key: Option<String>,
    last_emit: Instant,
}

#[derive(Default)]
struct Inner {
    tasks: BTreeMap<String, Entry>,
    /// khoá ngoài (vd `scan:3`) → id việc đang sống
    keys: HashMap<String, String>,
}

#[derive(Default)]
pub struct TaskHub {
    inner: Mutex<Inner>,
    rev: AtomicU64,
    next: AtomicU64,
    pub events: ProgressHub<TaskEvent>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl TaskHub {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn bump(&self) -> u64 {
        self.rev.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Danh sách hiện tại kèm `rev` hiện tại (FE subscribe trước, rồi gọi cái này, bỏ sự kiện có rev ≤ rev trả về).
    pub fn list(&self) -> TaskListDto {
        let g = self.lock();
        TaskListDto { rev: self.rev.load(Ordering::SeqCst), tasks: g.tasks.values().map(|e| e.dto.clone()).collect() }
    }

    /// Đăng ký một việc ở trạng thái chờ. `key` (nếu có) định danh ngoài: đã có việc cùng key thì KHÔNG tạo mới, trả id cũ.
    pub fn begin(&self, key: Option<&str>, spec: TaskSpec, state: TaskState) -> String {
        let mut g = self.lock();
        if let Some(k) = key
            && let Some(id) = g.keys.get(k)
        {
            return id.clone();
        }
        let id = format!("t{}", self.next.fetch_add(1, Ordering::SeqCst) + 1);
        let dto = TaskDto {
            id: id.clone(),
            rev: self.bump(),
            kind: spec.kind,
            trigger: spec.trigger,
            title_key: spec.title_key,
            title_args: spec.title_args,
            done: 0,
            total: spec.total,
            unit: spec.unit,
            state,
            cancellable: spec.cancel.is_some(),
            cancel_confirm_key: spec.cancel_confirm_key,
            group_key: spec.group_key,
            started_at: now_ms(),
        };
        self.events.emit(TaskEvent::Upsert { task: dto.clone() });
        if let Some(k) = key {
            g.keys.insert(k.to_string(), id.clone());
        }
        g.tasks.insert(id.clone(), Entry { dto, cancel: spec.cancel, key: key.map(str::to_string), last_emit: Instant::now() });
        id
    }

    pub fn id_of(&self, key: &str) -> Option<String> {
        self.lock().keys.get(key).cloned()
    }

    fn update(&self, id: &str, force: bool, f: impl FnOnce(&mut TaskDto)) {
        let mut g = self.lock();
        let Some(e) = g.tasks.get_mut(id) else { return };
        let before = e.dto.clone();
        f(&mut e.dto);
        if e.dto == before {
            return;
        }
        let state_changed = e.dto.state != before.state || e.dto.trigger != before.trigger || (e.dto.total != before.total && before.total.is_none());
        if !(force || state_changed || e.last_emit.elapsed() >= PROGRESS_MIN_GAP) {
            return; // tiến độ dồn: bản sau sẽ mang giá trị mới nhất
        }
        e.dto.rev = self.bump();
        e.last_emit = Instant::now();
        let dto = e.dto.clone();
        self.events.emit(TaskEvent::Upsert { task: dto });
    }

    /// Chuyển sang đang chạy (không làm gì nếu đã `Cancelling`).
    pub fn running(&self, id: &str) {
        self.update(id, true, |d| {
            if d.state == TaskState::Queued {
                d.state = TaskState::Running;
            }
        });
    }

    pub fn progress(&self, id: &str, done: u64, total: Option<u64>) {
        self.update(id, false, |d| {
            d.done = done;
            d.total = total;
        });
    }

    pub fn set_trigger(&self, id: &str, trigger: ScanTrigger) {
        self.update(id, true, |d| d.trigger = trigger);
    }

    /// Tên ngắn cho tiêu đề (vd tên nguồn) biết muộn.
    pub fn set_arg(&self, id: &str, k: &str, v: impl Into<TaskArg>) {
        let v = v.into();
        self.update(id, true, |d| {
            d.title_args.insert(k.to_string(), v);
        });
    }

    /// Kết thúc việc: phát `Removed` một lần (không lưu lịch sử). Gọi lần hai là vô hại.
    pub fn finish(&self, id: &str, outcome: TaskOutcome, error_key: Option<&str>) {
        let mut g = self.lock();
        let Some(e) = g.tasks.remove(id) else { return };
        if let Some(k) = e.key {
            g.keys.remove(&k);
        }
        let rev = self.bump();
        self.events.emit(TaskEvent::Removed { id: id.to_string(), rev, outcome, error_key: error_key.map(str::to_string) });
    }

    /// Huỷ: đặt `Cancelling` (vẫn giữ done/total) rồi gọi hàm huỷ NGOÀI khoá. `false` = không có việc hoặc không huỷ được.
    pub fn cancel(&self, id: &str) -> bool {
        let f = {
            let mut g = self.lock();
            let Some(e) = g.tasks.get_mut(id) else { return false };
            let Some(f) = e.cancel.clone() else { return false };
            if e.dto.state != TaskState::Cancelling {
                e.dto.state = TaskState::Cancelling;
                e.dto.rev = self.bump();
                e.last_emit = Instant::now();
                let dto = e.dto.clone();
                self.events.emit(TaskEvent::Upsert { task: dto });
            }
            f
        };
        f();
        true
    }

    /// Mọi việc thuộc `kind` mà `keep` trả `false` bị gỡ với kết quả `outcome` (dọn sau khi hàng đợi về rỗng).
    pub fn retain_kind(&self, kind: TaskKind, keep: impl Fn(&str) -> bool, outcome: TaskOutcome) {
        let stale: Vec<String> = {
            let g = self.lock();
            g.tasks.values().filter(|e| e.dto.kind == kind && !e.key.as_deref().is_some_and(&keep)).map(|e| e.dto.id.clone()).collect()
        };
        for id in stale {
            self.finish(&id, outcome, None);
        }
    }
}

/// Giữ một việc đang chạy trong một hàm đồng bộ: thoát sớm bằng `?`/panic thì tự gỡ với kết quả LỖI; gọi `done`/`cancelled` khi
/// kết thúc bình thường.
pub struct TaskGuard<'a> {
    hub: &'a TaskHub,
    pub id: String,
    error_key: Option<&'static str>,
    finished: std::cell::Cell<bool>,
}

impl<'a> TaskGuard<'a> {
    pub fn new(hub: &'a TaskHub, id: String, error_key: &'static str) -> Self {
        Self::with_error(hub, id, Some(error_key))
    }
    /// `None` = lỗi vẫn được báo bởi chính lệnh (toast từ kết quả lệnh), khay không thêm toast thứ hai.
    pub fn with_error(hub: &'a TaskHub, id: String, error_key: Option<&'static str>) -> Self {
        TaskGuard { hub, id, error_key, finished: std::cell::Cell::new(false) }
    }
    pub fn progress(&self, done: u64, total: Option<u64>) {
        self.hub.progress(&self.id, done, total);
    }
    pub fn end(&self, outcome: TaskOutcome) {
        if !self.finished.replace(true) {
            self.hub.finish(&self.id, outcome, None);
        }
    }
}

impl Drop for TaskGuard<'_> {
    fn drop(&mut self) {
        if !self.finished.replace(true) {
            self.hub.finish(&self.id, TaskOutcome::Failed, self.error_key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn rec(h: &TaskHub) -> Arc<Mutex<Vec<TaskEvent>>> {
        let v = Arc::new(Mutex::new(Vec::new()));
        let v2 = v.clone();
        h.events.subscribe(Box::new(move |e| v2.lock().unwrap().push(e)));
        v
    }
    fn rev_of(e: &TaskEvent) -> u64 {
        match e {
            TaskEvent::Upsert { task } => task.rev,
            TaskEvent::Removed { rev, .. } => *rev,
        }
    }

    #[test]
    fn rev_grows_across_upserts_and_removals_and_list_reports_the_current_rev() {
        let h = TaskHub::default();
        let ev = rec(&h);
        let id = h.begin(Some("scan:1"), TaskSpec::new(TaskKind::Scan, "task.scan").arg("source", "Fonts"), TaskState::Queued);
        h.running(&id);
        h.progress(&id, 5, Some(10));
        let l = h.list();
        assert_eq!(l.tasks.len(), 1);
        assert_eq!(l.tasks[0].title_args.len(), 1);
        h.finish(&id, TaskOutcome::Done, None);
        let revs: Vec<u64> = ev.lock().unwrap().iter().map(rev_of).collect();
        assert!(revs.windows(2).all(|w| w[0] < w[1]), "{revs:?}");
        assert!(l.rev <= *revs.last().unwrap());
        assert!(h.list().tasks.is_empty() && h.list().rev == *revs.last().unwrap());
    }

    #[test]
    fn same_key_does_not_create_a_second_task_and_finish_is_idempotent() {
        let h = TaskHub::default();
        let ev = rec(&h);
        let a = h.begin(Some("scan:1"), TaskSpec::new(TaskKind::Scan, "t"), TaskState::Queued);
        let b = h.begin(Some("scan:1"), TaskSpec::new(TaskKind::Scan, "t"), TaskState::Queued);
        assert_eq!(a, b);
        h.finish(&a, TaskOutcome::Cancelled, None);
        h.finish(&a, TaskOutcome::Cancelled, None);
        let removed = ev.lock().unwrap().iter().filter(|e| matches!(e, TaskEvent::Removed { .. })).count();
        assert_eq!(removed, 1);
        // key được giải phóng: tạo lại được việc mới
        let c = h.begin(Some("scan:1"), TaskSpec::new(TaskKind::Scan, "t"), TaskState::Queued);
        assert_ne!(a, c);
    }

    #[test]
    fn progress_is_throttled_but_state_changes_and_the_final_value_are_never_lost() {
        let h = TaskHub::default();
        let ev = rec(&h);
        let id = h.begin(None, TaskSpec::new(TaskKind::Export, "t"), TaskState::Running);
        for i in 1..=200u64 {
            h.progress(&id, i, Some(200));
        }
        let n = ev.lock().unwrap().len();
        assert!(n < 20, "200 lần progress liên tiếp phải bị dồn: {n} sự kiện");
        // giá trị mới nhất vẫn đọc được qua list (không mất)
        assert_eq!(h.list().tasks[0].done, 200, "list luôn có giá trị mới nhất");
        h.set_trigger(&id, ScanTrigger::Watcher); // đổi quan trọng → phát ngay
        assert!(matches!(ev.lock().unwrap().last(), Some(TaskEvent::Upsert { task }) if task.trigger == ScanTrigger::Watcher));
    }

    #[test]
    fn cancel_marks_cancelling_keeps_done_total_calls_the_hook_outside_the_lock_and_is_safe_twice() {
        let h = Arc::new(TaskHub::default());
        let ev = rec(&h);
        let hit = Arc::new(AtomicBool::new(false));
        let (h2, hit2) = (h.clone(), hit.clone());
        // hook gọi lại hub (list) — chứng minh không giữ khoá khi gọi
        let f: CancelFn = Arc::new(move || {
            let _ = h2.list();
            hit2.store(true, Ordering::SeqCst);
        });
        let id = h.begin(None, TaskSpec::new(TaskKind::ResolveDuplicates, "t").cancellable(f).confirm_cancel("task.keepDone"), TaskState::Running);
        h.progress(&id, 3, Some(12));
        assert!(h.cancel(&id));
        assert!(h.cancel(&id));
        assert!(hit.load(Ordering::SeqCst));
        let t = h.list().tasks.remove(0);
        assert_eq!((t.state, t.done, t.total, t.cancel_confirm_key.as_deref()), (TaskState::Cancelling, 3, Some(12), Some("task.keepDone")));
        h.running(&id); // không kéo Cancelling về Running
        assert_eq!(h.list().tasks[0].state, TaskState::Cancelling);
        assert!(!h.cancel("t999"));
        assert!(ev.lock().unwrap().iter().any(|e| matches!(e, TaskEvent::Upsert { task } if task.state == TaskState::Cancelling)));
    }

    #[test]
    fn a_task_without_a_cancel_hook_is_not_cancellable_and_unknown_total_stays_none() {
        let h = TaskHub::default();
        let id = h.begin(None, TaskSpec::new(TaskKind::Embed, "t"), TaskState::Running);
        assert!(!h.cancel(&id));
        let t = h.list().tasks.remove(0);
        assert!(!t.cancellable && t.total.is_none());
    }

    #[test]
    fn retain_kind_removes_the_stale_ones_only() {
        let h = TaskHub::default();
        let a = h.begin(Some("scan:1"), TaskSpec::new(TaskKind::Scan, "t"), TaskState::Queued);
        let _b = h.begin(Some("scan:2"), TaskSpec::new(TaskKind::Scan, "t"), TaskState::Queued);
        let other = h.begin(None, TaskSpec::new(TaskKind::Export, "t"), TaskState::Running);
        h.retain_kind(TaskKind::Scan, |k| k == "scan:1", TaskOutcome::Cancelled);
        let ids: Vec<String> = h.list().tasks.into_iter().map(|t| t.id).collect();
        assert!(ids.contains(&a) && ids.contains(&other) && ids.len() == 2, "{ids:?}");
    }
}
