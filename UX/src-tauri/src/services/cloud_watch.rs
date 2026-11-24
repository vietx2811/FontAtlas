//! A2 + B8: theo dõi MỌI thư mục nguồn (ổ cloud File Provider/iCloud, ổ ngoài, thư mục cục bộ) bằng FSEvents
//! (crate `notify`, chỉ backend macOS). Công tắc "Tự phát hiện thay đổi" (khoá `cloud_watch`, tên cũ giữ cho tương thích) mặc định bật.
//!
//! Sự kiện chỉ để ĐÁNH THỨC: không mang dữ liệu, không quyết định gì. Sau khi yên lặng một lúc (hoặc tối đa `max_wait`)
//! xếp MỘT lượt quét lại cho đúng nguồn đó bằng `ScanManager::enqueue_follow_up`; quét (so size/mtime/cờ dataless) vẫn là
//! nguồn sự thật. Không đọc, không ghi, không đổi cờ trên file của người dùng.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fontatlas_store::{FONT_EXTENSIONS, SourceKind};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::state::AppState;

/// Yên lặng bao lâu thì quét (gom các đợt đồng bộ dài).
pub const QUIET: Duration = Duration::from_secs(2);
/// Khi lô sự kiện có stub `.icloud` hoặc file dataless (đang tải từ cloud): các file về rời rạc cách nhau vài giây, quét cả
/// nguồn sau mỗi file là phí và làm UI nháy. Chờ lâu hơn (`QUIET × CLOUD_QUIET_FACTOR` = 8 s) để gom cả đợt.
pub const CLOUD_QUIET_FACTOR: u32 = 4;
/// Nhưng không để một nguồn liên tục có sự kiện chờ quá lâu mới được quét.
pub const MAX_WAIT: Duration = Duration::from_secs(30);
/// "Cơn bão" quét: một nguồn đã tới hạn ≥ `STORM_FIRES` lần trong `STORM_WINDOW` thì quiet giãn dần (×2 rồi ×4, tối đa 8 s với QUIET 2 s;
/// `MAX_WAIT` giữ nguyên). Yên `STORM_WINDOW` thì về `QUIET`. Áp cho mọi nguồn (không riêng cloud): thay đổi lẻ không bị ảnh hưởng.
pub const STORM_WINDOW: Duration = Duration::from_secs(60);
pub const STORM_FIRES: usize = 3;

/// Sự kiện này có đáng đánh thức không? Chỉ nhìn tên (không chạm đĩa): font, stub iCloud, hoặc thư mục (không đuôi).
/// Bỏ file/thư mục ẩn (trừ stub `.Tên.ttf.icloud`), file tạm/đang tải dở (`*.sb-*`, `~$*`, `~`, `.tmp`, `.temp`, `.part`, `.partial`,
/// `.crdownload`, `.download`, `.opdownload`, `.aria2`, `.swp`).
///
/// `roots`: đường dẫn nguồn như người dùng thêm VÀ dạng đã `canonicalize` (FSEvents báo đường dẫn thật, vd `/private/var`).
/// Chỉ xét phần BÊN DƯỚI gốc: thư mục ẩn nằm phía trên nguồn (vd `~/.cache/…`) không làm nguồn bị bỏ qua.
pub fn is_relevant(path: &Path, roots: &[PathBuf]) -> bool {
    let rel = roots.iter().find_map(|r| path.strip_prefix(r).ok()).unwrap_or_else(|| path.file_name().map(Path::new).unwrap_or(path));
    let comps: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let Some(last) = comps.last() else { return true }; // chính thư mục gốc (đổi tên/xoá)
    let is_stub = last.starts_with('.') && last.to_ascii_lowercase().ends_with(".icloud");
    if comps[..comps.len() - 1].iter().any(|c| c.starts_with('.')) {
        return false;
    }
    if last.starts_with('.') && !is_stub {
        return false;
    }
    let l = last.to_ascii_lowercase();
    // file tạm/đang tải dở: chung (.tmp, .part…) và đặc trưng từng hãng cloud (Drive `.tmp.driveupload`/`.tmp.drivedownload`,
    // Dropbox `.dropbox`/`.dropbox.attrs`/`.dropbox.cache`, OneDrive `~tmp`/`.~tmp`, Box `.boxsync`/`.boxtmp`)
    const PARTIAL: [&str; 20] = [
        ".tmp", ".temp", ".part", ".partial", ".crdownload", ".download", ".opdownload", ".aria2", ".swp", "~", ".tmp.driveupload",
        ".tmp.drivedownload", ".dropbox", ".dropbox.attrs", ".dropbox.cache", "~tmp", ".boxsync", ".boxtmp", ".sync", ".lock",
    ];
    if l.contains(".sb-") || l.starts_with("~$") || PARTIAL.iter().any(|x| l.ends_with(x)) {
        return false;
    }
    if is_stub {
        return true;
    }
    match Path::new(last).extension().and_then(|e| e.to_str()) {
        Some(ext) => FONT_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(ext)),
        None => true, // thư mục: tạo/xoá/đổi tên cả thư mục chứa font
    }
}

/// Chỉ các loại sự kiện thay đổi cấu trúc hoặc nội dung; bỏ truy cập thuần.
fn is_change(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}

/// Sự kiện này thuộc về việc tải từ cloud? Stub `.Tên.ttf.icloud` (đổi tên/xoá khi tải xong) hoặc file font đang là placeholder dataless.
/// Chỉ `stat` (không đọc nội dung, không ghi gì).
pub fn is_cloud_event(path: &Path) -> bool {
    // nguồn nằm trên ổ cloud (File Provider: OneDrive/Google Drive/Dropbox/Box…, iCloud Drive): dùng chung bộ nhận diện với cột Folder
    if fontatlas_store::cloud_provider_of(path).is_some() {
        return true;
    }
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()) else { return false };
    if name.starts_with('.') && name.ends_with(".icloud") {
        return true;
    }
    let is_font = path.extension().and_then(|e| e.to_str()).is_some_and(|e| FONT_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)));
    is_font && fontatlas_store::is_dataless(path)
}

/// Bộ gom sự kiện theo nguồn (thuần, kiểm bằng giờ giả).
#[derive(Debug, Default)]
pub struct Debounce {
    pending: HashMap<i64, (Instant, Instant)>, // id → (lần đầu, lần cuối)
    /// Nguồn có sự kiện cloud trong lô đang chờ: dùng `slow_quiet` thay vì `quiet`.
    slow: HashSet<i64>,
    /// Các lần nguồn tới hạn gần đây (để giãn quiet khi quét dồn dập).
    fires: HashMap<i64, Vec<Instant>>,
}

impl Debounce {
    pub fn push(&mut self, id: i64, now: Instant) {
        self.pending.entry(id).and_modify(|e| e.1 = now).or_insert((now, now));
    }
    /// Như `push`, đánh dấu lô này có sự kiện cloud (chờ `slow_quiet` thay vì `quiet`).
    pub fn push_cloud(&mut self, id: i64, now: Instant) {
        self.push(id, now);
        self.slow.insert(id);
    }
    fn quiet_of(&self, id: i64, quiet: Duration, slow_quiet: Duration, now: Instant) -> Duration {
        let recent = self.fires.get(&id).map_or(0, |v| v.iter().filter(|t| now.saturating_duration_since(**t) < STORM_WINDOW).count());
        let storm = match recent {
            n if n < STORM_FIRES => 1,
            n if n == STORM_FIRES => 2,
            _ => 4,
        };
        let base = quiet * storm;
        if self.slow.contains(&id) { slow_quiet.max(base) } else { base }
    }
    /// Các id đã tới hạn (yên lặng ≥ `quiet` hoặc chờ ≥ `max_wait`); đồng thời bỏ chúng khỏi hàng.
    pub fn take_due(&mut self, now: Instant, quiet: Duration, max_wait: Duration) -> Vec<i64> {
        self.take_due_with(now, quiet, quiet, max_wait)
    }
    pub fn take_due_with(&mut self, now: Instant, quiet: Duration, slow_quiet: Duration, max_wait: Duration) -> Vec<i64> {
        let due: Vec<i64> = self
            .pending
            .iter()
            .filter(|(id, (first, last))| now.duration_since(*last) >= self.quiet_of(**id, quiet, slow_quiet, now) || now.duration_since(*first) >= max_wait)
            .map(|(id, _)| *id)
            .collect();
        for id in &due {
            self.pending.remove(id);
            self.slow.remove(id);
            let v = self.fires.entry(*id).or_default();
            v.retain(|t| now.saturating_duration_since(*t) < STORM_WINDOW);
            v.push(now);
        }
        let mut due = due;
        due.sort_unstable();
        due
    }
    /// Còn bao lâu tới hạn gần nhất (để ngủ đúng lúc); `None` khi không có gì chờ.
    pub fn next_wait(&self, now: Instant, quiet: Duration, max_wait: Duration) -> Option<Duration> {
        self.next_wait_with(now, quiet, quiet, max_wait)
    }
    pub fn next_wait_with(&self, now: Instant, quiet: Duration, slow_quiet: Duration, max_wait: Duration) -> Option<Duration> {
        self.pending
            .iter()
            .map(|(id, (first, last))| (*last + self.quiet_of(*id, quiet, slow_quiet, now)).min(*first + max_wait).saturating_duration_since(now))
            .min()
    }
    pub fn forget(&mut self, id: i64) {
        self.pending.remove(&id);
        self.slow.remove(&id);
        self.fires.remove(&id);
    }
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

enum Msg {
    /// (nguồn, sự kiện này thuộc việc tải cloud)
    Changed(i64, bool),
    Forget(i64),
}

struct Watch {
    root: PathBuf,
    _watcher: RecommendedWatcher,
}

/// Các luồng theo dõi: mỗi nguồn cloud đang bật một watcher; một luồng gom và xếp quét.
pub struct CloudWatch {
    quiet: Duration,
    max_wait: Duration,
    inner: Mutex<Inner>,
    /// `sync_in_background`: có luồng đồng bộ đang chạy / có yêu cầu mới đến khi nó đang chạy.
    bg_running: std::sync::atomic::AtomicBool,
    bg_again: std::sync::atomic::AtomicBool,
    closing: std::sync::atomic::AtomicBool,
    /// Có dựng watcher FSEvents thật không. `CloudWatch::default()` (trạng thái trong `AppState::new`, dùng bởi mọi test) = TẮT để thư viện
    /// tạm của test không dựng luồng FSEvents (chậm khi máy bận); app thật bật ở `lib.rs` bằng `enable_real_watchers`. `with_timing` = BẬT.
    real: std::sync::atomic::AtomicBool,
}

#[derive(Default)]
struct Inner {
    watches: HashMap<i64, Watch>,
    tx: Option<Sender<Msg>>,
    worker: Option<JoinHandle<()>>,
}

impl Default for CloudWatch {
    fn default() -> Self {
        let w = Self::with_timing(QUIET, MAX_WAIT);
        w.real.store(false, std::sync::atomic::Ordering::SeqCst);
        w
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl CloudWatch {
    pub fn with_timing(quiet: Duration, max_wait: Duration) -> Self {
        CloudWatch {
            quiet,
            max_wait,
            inner: Mutex::new(Inner::default()),
            bg_running: Default::default(),
            bg_again: Default::default(),
            closing: Default::default(),
            real: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Bật watcher thật (app thật gọi một lần lúc khởi tạo; test của watcher dùng `with_timing` hoặc gọi hàm này).
    pub fn enable_real_watchers(&self) {
        self.real.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Các nguồn đang được theo dõi (cho test/chẩn đoán).
    pub fn watching(&self) -> Vec<i64> {
        let mut v: Vec<i64> = lock(&self.inner).watches.keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// Như `sync` nhưng chạy ở luồng nền, không chặn người gọi: dựng một luồng FSEvents có thể mất nhiều giây khi máy bận (đã đo 13 s),
    /// không được làm treo "Thêm nguồn"/mở app. Nhiều yêu cầu dồn lại thành tối đa một lượt chạy nối tiếp.
    pub fn sync_in_background(&self, state: &Arc<AppState>) {
        use std::sync::atomic::Ordering::SeqCst;
        if self.closing.load(SeqCst) || !self.real.load(SeqCst) {
            return;
        }
        self.bg_again.store(true, SeqCst);
        if self.bg_running.swap(true, SeqCst) {
            return; // luồng đang chạy sẽ thấy cờ `again`
        }
        let st = Arc::clone(state);
        let spawned = std::thread::Builder::new().name("fontatlas-watch-sync".into()).spawn(move || {
            let w = &st.cloud_watch;
            while w.bg_again.swap(false, SeqCst) && !w.closing.load(SeqCst) {
                w.sync(&st);
            }
            w.bg_running.store(false, SeqCst);
            // yêu cầu lọt vào giữa lúc kết thúc
            if w.bg_again.load(SeqCst) && !w.closing.load(SeqCst) {
                w.sync_in_background(&st);
            }
        });
        if spawned.is_err() {
            self.bg_running.store(false, SeqCst);
        }
    }

    /// Đồng bộ danh sách watcher với nguồn hiện tại: MỌI nguồn thư mục đang bật và đang truy cập được có watcher; nguồn bị
    /// tắt/xoá/ngắt kết nối (ổ rút) thì dừng. Gọi lại bao nhiêu lần cũng được; ổ cắm lại thì lần `sync` sau (thêm/gỡ nguồn,
    /// mở app, đổi công tắc) theo dõi lại.
    pub fn sync(&self, state: &Arc<AppState>) {
        // công tắc "Tự phát hiện thay đổi trên ổ cloud" (mặc định bật); tắt = không nguồn nào được theo dõi
        if !self.real.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let enabled = state.with_user(|u| Ok(u.setting(crate::services::keys::CLOUD_WATCH)?)).ok().flatten().is_none_or(|v| v != "0" && v != "false");
        let want: HashMap<i64, PathBuf> = state
            .with_user(|u| Ok(u.sources()?))
            .unwrap_or_default()
            .into_iter()
            .filter(|s| enabled && s.kind == SourceKind::Directory && s.enabled)
            .filter_map(|s| s.path.map(|p| (s.id, PathBuf::from(p))))
            .filter(|(_, p)| crate::services::sources::root_available(&p.to_string_lossy()))
            .collect();
        let mut g = lock(&self.inner);
        let stale: Vec<i64> = g.watches.iter().filter(|(id, w)| want.get(id) != Some(&w.root)).map(|(id, _)| *id).collect();
        for id in stale {
            g.watches.remove(&id); // Drop dừng luồng FSEvents
            if let Some(tx) = &g.tx {
                let _ = tx.send(Msg::Forget(id));
            }
        }
        let have: HashSet<i64> = g.watches.keys().copied().collect();
        for (id, root) in want {
            if have.contains(&id) {
                continue;
            }
            let tx = self.ensure_worker(&mut g, state);
            let roots: Vec<PathBuf> = std::iter::once(root.clone()).chain(root.canonicalize().ok()).collect();
            let cloud_root = roots.iter().any(|r| fontatlas_store::cloud_provider_of(r).is_some());
            let handler = move |res: notify::Result<notify::Event>| {
                if let Ok(ev) = res
                    && is_change(&ev.kind)
                    && (ev.paths.is_empty() || ev.paths.iter().any(|p| is_relevant(p, &roots)))
                {
                    let cloud = cloud_root || ev.paths.iter().any(|p| is_relevant(p, &roots) && is_cloud_event(p));
                    let _ = tx.send(Msg::Changed(id, cloud));
                }
            };
            match notify::recommended_watcher(handler).and_then(|mut w| w.watch(&root, RecursiveMode::Recursive).map(|_| w)) {
                Ok(w) => {
                    g.watches.insert(id, Watch { root, _watcher: w });
                }
                Err(e) => log::warn!("cloud_watch: không theo dõi được nguồn {id} ({}): {e}", root.display()),
            }
        }
    }

    fn ensure_worker(&self, g: &mut Inner, state: &Arc<AppState>) -> Sender<Msg> {
        if let Some(tx) = &g.tx {
            return tx.clone();
        }
        let (tx, rx) = mpsc::channel::<Msg>();
        let weak: Weak<AppState> = Arc::downgrade(state);
        let (quiet, max_wait) = (self.quiet, self.max_wait);
        let slow_quiet = quiet * CLOUD_QUIET_FACTOR;
        g.worker = Some(std::thread::spawn(move || {
            let mut deb = Debounce::default();
            loop {
                let now = Instant::now();
                let msg = match deb.next_wait_with(now, quiet, slow_quiet, max_wait) {
                    Some(w) => rx.recv_timeout(w),
                    None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                };
                match msg {
                    Ok(Msg::Changed(id, true)) => deb.push_cloud(id, Instant::now()),
                    Ok(Msg::Changed(id, false)) => deb.push(id, Instant::now()),
                    Ok(Msg::Forget(id)) => deb.forget(id),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let due = deb.take_due_with(Instant::now(), quiet, slow_quiet, max_wait);
                if due.is_empty() {
                    continue;
                }
                let Some(state) = weak.upgrade() else { break };
                state.scans.enqueue_follow_up_as(&state, &due, crate::dto::events::ScanTrigger::Watcher);
            }
        }));
        g.tx = Some(tx.clone());
        tx
    }

    /// Dừng mọi watcher và luồng gom (khi tắt app).
    pub fn shutdown(&self) {
        self.closing.store(true, std::sync::atomic::Ordering::SeqCst);
        let (worker, watches) = {
            let mut g = lock(&self.inner);
            g.tx = None;
            (g.worker.take(), std::mem::take(&mut g.watches))
        };
        // watcher giữ bản sao của kênh gửi: phải thả TRƯỚC khi chờ luồng gom, kẻo nó chờ mãi
        drop(watches);
        if let Some(h) = worker {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q: Duration = Duration::from_secs(2);
    const M: Duration = Duration::from_secs(30);

    #[test]
    fn debounce_waits_for_quiet_and_merges_a_burst_into_one() {
        let t0 = Instant::now();
        let mut d = Debounce::default();
        for i in 0..100 {
            d.push(7, t0 + Duration::from_millis(i * 10)); // 100 sự kiện trong 1 s
        }
        assert!(d.take_due(t0 + Duration::from_secs(2), Q, M).is_empty(), "chưa yên lặng đủ lâu");
        assert_eq!(d.take_due(t0 + Duration::from_millis(990 + 2000), Q, M), vec![7]);
        assert!(d.is_empty());
        assert!(d.take_due(t0 + Duration::from_secs(60), Q, M).is_empty(), "một cụm = đúng một lượt");
    }

    #[test]
    fn debounce_caps_the_wait_for_a_source_that_never_goes_quiet() {
        let t0 = Instant::now();
        let mut d = Debounce::default();
        for s in 0..40 {
            d.push(1, t0 + Duration::from_secs(s)); // sự kiện mỗi giây, không bao giờ yên lặng 2 s
        }
        assert!(d.take_due(t0 + Duration::from_secs(29), Q, M).is_empty());
        assert_eq!(d.take_due(t0 + Duration::from_secs(30), Q, M), vec![1]);
    }

    #[test]
    fn debounce_keeps_sources_apart_and_reports_next_wait() {
        let t0 = Instant::now();
        let mut d = Debounce::default();
        d.push(1, t0);
        d.push(2, t0 + Duration::from_secs(1));
        assert_eq!(d.next_wait(t0, Q, M), Some(Q));
        assert_eq!(d.take_due(t0 + Q, Q, M), vec![1]);
        assert_eq!(d.take_due(t0 + Duration::from_secs(3), Q, M), vec![2]);
        assert_eq!(d.next_wait(t0, Q, M), None);
        d.push(3, t0);
        d.forget(3);
        assert!(d.is_empty());
    }

    #[test]
    fn a_source_that_keeps_firing_backs_off_to_four_times_quiet_then_recovers() {
        const Q: Duration = Duration::from_secs(2);
        const M: Duration = Duration::from_secs(30);
        let t0 = Instant::now();
        let mut d = Debounce::default();
        // 3 lần đầu: quiet thường 2 s
        let mut now = t0;
        for _ in 0..3 {
            d.push(1, now);
            now += Q;
            assert_eq!(d.take_due(now, Q, M), vec![1]);
        }
        // lần 4 (đã 3 lần trong 60 s): quiet 4 s
        d.push(1, now);
        assert!(d.take_due(now + Duration::from_secs(3), Q, M).is_empty());
        assert_eq!(d.take_due(now + Duration::from_secs(4), Q, M), vec![1]);
        // lần 5: 8 s, nhưng không quá MAX_WAIT
        let now2 = now + Duration::from_secs(4);
        d.push(1, now2);
        assert!(d.take_due(now2 + Duration::from_secs(7), Q, M).is_empty());
        assert_eq!(d.take_due(now2 + Duration::from_secs(8), Q, M), vec![1]);
        // yên đủ STORM_WINDOW: về 2 s
        let later = now2 + Duration::from_secs(8) + STORM_WINDOW;
        d.push(1, later);
        assert_eq!(d.take_due(later + Q, Q, M), vec![1]);
        // nguồn khác không bị ảnh hưởng
        d.push(2, later);
        assert_eq!(d.take_due(later + Q, Q, M), vec![2]);
    }

    /// Case C của QA: 30 file cách 4 s trên nguồn local. Mô phỏng giờ giả qua vòng lặp thật của Debounce.
    #[test]
    fn thirty_events_four_seconds_apart_make_far_fewer_than_thirty_scans_but_a_lone_event_still_waits_only_quiet() {
        const Q: Duration = Duration::from_secs(2);
        const M: Duration = Duration::from_secs(30);
        let t0 = Instant::now();
        let mut d = Debounce::default();
        let mut fires = 0;
        let mut t = Duration::ZERO;
        let end = Duration::from_secs(4 * 30 + 40);
        while t <= end {
            if t.as_secs().is_multiple_of(4) && t.as_secs() < 4 * 30 {
                d.push(1, t0 + t);
            }
            if !d.take_due(t0 + t, Q, M).is_empty() {
                fires += 1;
            }
            t += Duration::from_millis(500);
        }
        assert!(fires <= 8, "30 file cách 4 s: {fires} lượt quét (trước backoff: 30)");
        assert!(fires >= 3, "vẫn phải quét đều, không bỏ đói: {fires}");

        // ghi/xoá lẻ trên nguồn yên: tới hạn đúng QUIET (≤ 2 s), kể cả sau cơn bão khi đã yên đủ STORM_WINDOW
        let mut lone = Debounce::default();
        lone.push(7, t0);
        assert!(lone.take_due(t0 + Q - Duration::from_millis(1), Q, M).is_empty());
        assert_eq!(lone.take_due(t0 + Q, Q, M), vec![7]);
        let later = t0 + Q + STORM_WINDOW + Duration::from_secs(1);
        lone.push(7, later);
        assert_eq!(lone.take_due(later + Q, Q, M), vec![7], "sau 60 s yên lại 2 s");
        // hai lần lẻ cách xa nhau không kích hoạt backoff
        lone.push(7, later + Duration::from_secs(20));
        assert_eq!(lone.take_due(later + Duration::from_secs(22), Q, M), vec![7]);
    }

    #[test]
    fn cloud_batches_wait_four_times_longer_but_never_past_max_wait() {
        let t0 = Instant::now();
        let mut d = Debounce::default();
        let slow = Q * CLOUD_QUIET_FACTOR; // 8 s
        d.push_cloud(1, t0);
        d.push(2, t0); // nguồn khác, không cloud
        assert_eq!(d.take_due_with(t0 + Q, Q, slow, M), vec![2], "nguồn thường vẫn 2 s");
        assert!(d.take_due_with(t0 + Duration::from_secs(7), Q, slow, M).is_empty(), "lô cloud chưa tới 8 s");
        assert_eq!(d.next_wait_with(t0 + Duration::from_secs(7), Q, slow, M), Some(Duration::from_secs(1)));
        assert_eq!(d.take_due_with(t0 + slow, Q, slow, M), vec![1]);
        // sự kiện cloud liên tục: trần 30 s vẫn áp dụng
        for s in 0..40 {
            d.push_cloud(3, t0 + Duration::from_secs(s));
        }
        assert!(d.take_due_with(t0 + Duration::from_secs(29), Q, slow, M).is_empty());
        assert_eq!(d.take_due_with(t0 + Duration::from_secs(30), Q, slow, M), vec![3]);
        // lượt kế của nguồn đó (không còn sự kiện cloud) quay về 2 s
        d.push(3, t0 + Duration::from_secs(40));
        assert_eq!(d.take_due_with(t0 + Duration::from_secs(42), Q, slow, M), vec![3]);
    }

    #[test]
    fn every_cloud_provider_path_counts_as_cloud_and_vendor_temp_files_are_filtered() {
        for p in [
            "/Users/a/Library/CloudStorage/OneDrive-Personal/Fonts/a.ttf",
            "/Users/a/Library/CloudStorage/GoogleDrive-x@y.com/My Drive/a.otf",
            "/Users/a/Library/CloudStorage/Dropbox/Fonts/a.ttf",
            "/Users/a/Library/CloudStorage/Box-Box/a.ttf",
            "/Users/a/Library/Mobile Documents/com~apple~CloudDocs/Fonts/a.ttf",
        ] {
            assert!(is_cloud_event(Path::new(p)), "{p}");
        }
        assert!(!is_cloud_event(Path::new("/Users/a/Fonts/a.ttf")), "thư mục cục bộ không phải cloud");
        // file tạm từng hãng: không đáng đánh thức
        let root = vec![PathBuf::from("/r")];
        for tmp in [
            "/r/a.ttf.tmp.driveupload", "/r/a.otf.tmp.drivedownload", "/r/a.ttf.dropbox", "/r/a.ttf.dropbox.attrs", "/r/a.ttf.dropbox.cache",
            "/r/a.ttf~tmp", "/r/a.ttf.~tmp", "/r/a.ttf.boxsync", "/r/a.ttf.boxtmp", "/r/.dropbox.cache/a.ttf", "/r/a.ttf.lock", "/r/a.ttf.sync",
        ] {
            assert!(!is_relevant(Path::new(tmp), &root), "{tmp}");
        }
        assert!(is_relevant(Path::new("/r/a.ttf"), &root));
    }

    #[test]
    fn icloud_stub_events_count_as_cloud_events_plain_fonts_do_not() {
        assert!(is_cloud_event(Path::new("/r/.Name.ttf.icloud")));
        assert!(!is_cloud_event(Path::new("/r/a-file-that-does-not-exist.ttf")));
        assert!(!is_cloud_event(Path::new("/r/readme.txt")));
    }

    #[test]
    fn relevance_filter_keeps_fonts_stubs_and_folders_only() {
        let root = vec![PathBuf::from("/r")];
        for ok in ["/r/a.ttf", "/r/sub/B.OTF", "/r/sub", "/r/.Name.ttf.icloud", "/r/x/.Name.otf.icloud", "/r"] {
            assert!(is_relevant(Path::new(ok), &root), "{ok}");
        }
        for no in [
            "/r/a.txt", "/r/.DS_Store", "/r/.hidden/a.ttf", "/r/a.ttf.sb-82369e39-LTUTFh", "/r/~$a.ttf", "/r/a.ttf.part", "/r/readme.md",
            "/r/a.ttf.download", "/r/a.otf.crdownload", "/r/a.ttf.partial", "/r/a.ttf~", "/r/a.ttf.swp", "/r/a.ttf.aria2", "/r/a.ttf.temp",
        ] {
            assert!(!is_relevant(Path::new(no), &root), "{no}");
        }
        // gốc nằm dưới thư mục ẩn / qua symlink (/var → /private/var): chỉ xét phần dưới gốc
        let hidden = vec![PathBuf::from("/h/.cache/Fonts"), PathBuf::from("/private/h/.cache/Fonts")];
        assert!(is_relevant(Path::new("/private/h/.cache/Fonts/a.ttf"), &hidden));
        assert!(!is_relevant(Path::new("/private/h/.cache/Fonts/.x/a.ttf"), &hidden));
    }
}
