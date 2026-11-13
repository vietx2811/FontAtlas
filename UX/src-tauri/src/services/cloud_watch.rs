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
/// Nhưng không để một nguồn liên tục có sự kiện chờ quá lâu mới được quét.
pub const MAX_WAIT: Duration = Duration::from_secs(30);

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
    const PARTIAL: [&str; 10] = [".tmp", ".temp", ".part", ".partial", ".crdownload", ".download", ".opdownload", ".aria2", ".swp", "~"];
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

/// Bộ gom sự kiện theo nguồn (thuần, kiểm bằng giờ giả).
#[derive(Debug, Default)]
pub struct Debounce {
    pending: HashMap<i64, (Instant, Instant)>, // id → (lần đầu, lần cuối)
}

impl Debounce {
    pub fn push(&mut self, id: i64, now: Instant) {
        self.pending.entry(id).and_modify(|e| e.1 = now).or_insert((now, now));
    }
    /// Các id đã tới hạn (yên lặng ≥ `quiet` hoặc chờ ≥ `max_wait`); đồng thời bỏ chúng khỏi hàng.
    pub fn take_due(&mut self, now: Instant, quiet: Duration, max_wait: Duration) -> Vec<i64> {
        let due: Vec<i64> = self
            .pending
            .iter()
            .filter(|(_, (first, last))| now.duration_since(*last) >= quiet || now.duration_since(*first) >= max_wait)
            .map(|(id, _)| *id)
            .collect();
        for id in &due {
            self.pending.remove(id);
        }
        let mut due = due;
        due.sort_unstable();
        due
    }
    /// Còn bao lâu tới hạn gần nhất (để ngủ đúng lúc); `None` khi không có gì chờ.
    pub fn next_wait(&self, now: Instant, quiet: Duration, max_wait: Duration) -> Option<Duration> {
        self.pending
            .values()
            .map(|(first, last)| (*last + quiet).min(*first + max_wait).saturating_duration_since(now))
            .min()
    }
    pub fn forget(&mut self, id: i64) {
        self.pending.remove(&id);
    }
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

enum Msg {
    Changed(i64),
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
}

#[derive(Default)]
struct Inner {
    watches: HashMap<i64, Watch>,
    tx: Option<Sender<Msg>>,
    worker: Option<JoinHandle<()>>,
}

impl Default for CloudWatch {
    fn default() -> Self {
        Self::with_timing(QUIET, MAX_WAIT)
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl CloudWatch {
    pub fn with_timing(quiet: Duration, max_wait: Duration) -> Self {
        CloudWatch { quiet, max_wait, inner: Mutex::new(Inner::default()) }
    }

    /// Các nguồn đang được theo dõi (cho test/chẩn đoán).
    pub fn watching(&self) -> Vec<i64> {
        let mut v: Vec<i64> = lock(&self.inner).watches.keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// Đồng bộ danh sách watcher với nguồn hiện tại: MỌI nguồn thư mục đang bật và đang truy cập được có watcher; nguồn bị
    /// tắt/xoá/ngắt kết nối (ổ rút) thì dừng. Gọi lại bao nhiêu lần cũng được; ổ cắm lại thì lần `sync` sau (thêm/gỡ nguồn,
    /// mở app, đổi công tắc) theo dõi lại.
    pub fn sync(&self, state: &Arc<AppState>) {
        // công tắc "Tự phát hiện thay đổi trên ổ cloud" (mặc định bật); tắt = không nguồn nào được theo dõi
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
            let handler = move |res: notify::Result<notify::Event>| {
                if let Ok(ev) = res
                    && is_change(&ev.kind)
                    && (ev.paths.is_empty() || ev.paths.iter().any(|p| is_relevant(p, &roots)))
                {
                    let _ = tx.send(Msg::Changed(id));
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
        g.worker = Some(std::thread::spawn(move || {
            let mut deb = Debounce::default();
            loop {
                let now = Instant::now();
                let msg = match deb.next_wait(now, quiet, max_wait) {
                    Some(w) => rx.recv_timeout(w),
                    None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                };
                match msg {
                    Ok(Msg::Changed(id)) => deb.push(id, Instant::now()),
                    Ok(Msg::Forget(id)) => deb.forget(id),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let due = deb.take_due(Instant::now(), quiet, max_wait);
                if due.is_empty() {
                    continue;
                }
                let Some(state) = weak.upgrade() else { break };
                state.scans.enqueue_follow_up(&state, &due);
            }
        }));
        g.tx = Some(tx.clone());
        tx
    }

    /// Dừng mọi watcher và luồng gom (khi tắt app).
    pub fn shutdown(&self) {
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
