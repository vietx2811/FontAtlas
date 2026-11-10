//! Tải font từ ổ cloud để xem trước: cổng giới hạn đồng thời, hạn chót, dùng chung một lần tải cho cùng một file.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use fontatlas_activate::EnsureLocalError;
use fontatlas_preview::PreviewError;

/// Số font cloud được tải thật cùng lúc để xem trước.
pub const MAX_CLOUD_DOWNLOADS: usize = 4;
/// Hạn chót cho MỘT lần tải, tính từ lúc đã có chỗ (không gồm thời gian xếp hàng chờ chỗ).
pub const CLOUD_PREVIEW_TIMEOUT: Duration = Duration::from_secs(30);
/// Chờ chỗ tối đa bao lâu; quá thì trả `CloudBusy` (503) để giao diện xếp lại thay vì báo hết giờ oan.
pub const CLOUD_QUEUE_WAIT: Duration = Duration::from_secs(60);

/// Cổng đếm: tối đa `max` người giữ giấy phép; ai không vào kịp thì bỏ.
pub struct CloudGate {
    max: usize,
    busy: Mutex<usize>,
    freed: Condvar,
}

/// Giấy phép sở hữu (không mượn): chuyển được vào luồng tải và chỉ nhả khi luồng đó xong.
pub struct Permit {
    gate: Arc<CloudGate>,
}

impl CloudGate {
    pub fn new(max: usize) -> Arc<Self> {
        Arc::new(CloudGate { max: max.max(1), busy: Mutex::new(0), freed: Condvar::new() })
    }

    /// Chờ tới lượt tối đa `wait`; None nếu hết hạn.
    pub fn acquire(self: &Arc<Self>, wait: Duration) -> Option<Permit> {
        let deadline = Instant::now() + wait;
        let mut busy = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        while *busy >= self.max {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            let (g, _) = self.freed.wait_timeout(busy, left).unwrap_or_else(|e| e.into_inner());
            busy = g;
        }
        *busy += 1;
        Some(Permit { gate: Arc::clone(self) })
    }

    pub fn in_flight(&self) -> usize {
        *self.busy.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut busy = self.gate.busy.lock().unwrap_or_else(|e| e.into_inner());
        *busy = busy.saturating_sub(1);
        self.gate.freed.notify_one();
    }
}

/// Kết quả một lần tải, chia sẻ cho mọi người cùng chờ một file.
#[derive(Clone)]
enum Outcome {
    Ok,
    Timeout,
    Missing,
    Busy,
    Unavailable(String),
}

impl Outcome {
    fn into_result(self) -> Result<(), PreviewError> {
        match self {
            Outcome::Ok => Ok(()),
            Outcome::Timeout => Err(PreviewError::CloudTimeout),
            Outcome::Missing => Err(PreviewError::NotFound),
            Outcome::Busy => Err(PreviewError::CloudBusy),
            Outcome::Unavailable(m) => Err(PreviewError::CloudUnavailable(m)),
        }
    }
}

struct Flight {
    done: Mutex<Option<Outcome>>,
    finished: Condvar,
}

type Probe = Box<dyn Fn(&Path) -> bool + Send + Sync>;
/// Tải một file; `Permit` phải được chuyển vào luồng đọc để nhả đúng lúc (xem `ensure_local_holding`).
type Ensure = Box<dyn Fn(&Path, Duration, Permit) -> Result<(), EnsureLocalError> + Send + Sync>;
type OnLocal = Box<dyn Fn(&Path) + Send + Sync>;

/// Đưa file cloud về máy ngay trước khi đọc. File đã ở máy: đi thẳng, không qua cổng.
pub struct CloudDownloader {
    gate: Arc<CloudGate>,
    total: Duration,
    queue_wait: Duration,
    is_dataless: Probe,
    ensure: Ensure,
    on_local: OnLocal,
    inflight: Mutex<HashMap<PathBuf, Arc<Flight>>>,
}

impl Default for CloudDownloader {
    fn default() -> Self {
        CloudDownloader::with(
            MAX_CLOUD_DOWNLOADS,
            CLOUD_PREVIEW_TIMEOUT,
            |p| fontatlas_store::is_dataless(p),
            |p, left, permit| fontatlas_activate::ensure_local_holding(p, left, permit),
        )
    }
}

impl CloudDownloader {
    /// Thay cách thăm dò / tải / hạn chót (test không cần ổ cloud thật).
    pub fn with(
        max: usize,
        total: Duration,
        is_dataless: impl Fn(&Path) -> bool + Send + Sync + 'static,
        ensure: impl Fn(&Path, Duration, Permit) -> Result<(), EnsureLocalError> + Send + Sync + 'static,
    ) -> Self {
        CloudDownloader {
            gate: CloudGate::new(max),
            total,
            queue_wait: CLOUD_QUEUE_WAIT,
            is_dataless: Box::new(is_dataless),
            ensure: Box::new(ensure),
            on_local: Box::new(|_| {}),
            inflight: Mutex::new(HashMap::new()),
        }
    }

    /// Gọi đúng một lần sau khi một font vừa được tải về thành công (ghi cờ `cloud_only` ngay, không đợi lần quét sau).
    pub fn on_local(mut self, f: impl Fn(&Path) + Send + Sync + 'static) -> Self {
        self.on_local = Box::new(f);
        self
    }

    pub fn queue_wait(mut self, d: Duration) -> Self {
        self.queue_wait = d;
        self
    }

    pub fn in_flight(&self) -> usize {
        self.gate.in_flight()
    }

    /// `allow_cloud` = công tắc "Tự tải font trên cloud để xem trước". Tắt mà file thật sự là placeholder (kể cả khi cờ
    /// trong DB cũ/chưa biết) → `CloudDisabled`, không tải gì.
    pub fn before_read(&self, path: &Path, allow_cloud: bool) -> Result<(), PreviewError> {
        if !(self.is_dataless)(path) {
            return Ok(());
        }
        if !allow_cloud {
            return Err(PreviewError::CloudDisabled);
        }
        let (flight, leader) = {
            let mut map = self.inflight.lock().unwrap_or_else(|e| e.into_inner());
            match map.get(path) {
                Some(f) => (Arc::clone(f), false),
                None => {
                    let f = Arc::new(Flight { done: Mutex::new(None), finished: Condvar::new() });
                    map.insert(path.to_path_buf(), Arc::clone(&f));
                    (f, true)
                }
            }
        };
        if leader {
            let outcome = self.download(path);
            *flight.done.lock().unwrap_or_else(|e| e.into_inner()) = Some(outcome);
            self.inflight.lock().unwrap_or_else(|e| e.into_inner()).remove(path);
            flight.finished.notify_all();
        }
        // người đến sau cùng file chờ kết quả của lần tải đang chạy (không xin thêm chỗ, không tải lần nữa)
        let deadline = Instant::now() + self.queue_wait + self.total;
        let mut done = flight.done.lock().unwrap_or_else(|e| e.into_inner());
        while done.is_none() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(PreviewError::CloudTimeout);
            }
            done = flight.finished.wait_timeout(done, left).unwrap_or_else(|e| e.into_inner()).0;
        }
        done.clone().unwrap_or(Outcome::Timeout).into_result()
    }

    fn download(&self, path: &Path) -> Outcome {
        // chờ chỗ KHÔNG ăn vào hạn tải: cuộn nhanh làm hàng dài thì báo bận (xếp lại được), không báo hết giờ oan
        let Some(permit) = self.gate.acquire(self.queue_wait) else { return Outcome::Busy };
        match (self.ensure)(path, self.total, permit) {
            Ok(()) => {
                (self.on_local)(path);
                Outcome::Ok
            }
            Err(EnsureLocalError::Timeout) => Outcome::Timeout,
            Err(EnsureLocalError::Missing) => Outcome::Missing,
            Err(e) => Outcome::Unavailable(e.message()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn p(s: &str) -> &Path {
        Path::new(s)
    }
    /// Tải giả chạy ngay trong luồng gọi; giữ permit tới hết hàm.
    fn instant(r: Result<(), EnsureLocalError>) -> impl Fn(&Path, Duration, Permit) -> Result<(), EnsureLocalError> + Send + Sync + 'static {
        move |_, _, _permit| r.clone()
    }

    #[test]
    fn gate_never_exceeds_its_limit_under_contention() {
        let gate = CloudGate::new(4);
        let (now, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let (g, n, pk) = (Arc::clone(&gate), Arc::clone(&now), Arc::clone(&peak));
                std::thread::spawn(move || {
                    let permit = g.acquire(Duration::from_secs(10)).expect("phải tới lượt");
                    let cur = n.fetch_add(1, Ordering::SeqCst) + 1;
                    pk.fetch_max(cur, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(30));
                    n.fetch_sub(1, Ordering::SeqCst);
                    drop(permit);
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 4, "đạt đúng giới hạn, không vượt");
        assert_eq!(gate.in_flight(), 0);
    }

    #[test]
    fn saturated_gate_gives_up_at_the_deadline() {
        let gate = CloudGate::new(1);
        let _held = gate.acquire(Duration::from_secs(1)).unwrap();
        let t = Instant::now();
        assert!(gate.acquire(Duration::from_millis(120)).is_none());
        assert!(t.elapsed() >= Duration::from_millis(100) && t.elapsed() < Duration::from_secs(2));
        assert_eq!(gate.in_flight(), 1, "người bỏ cuộc không giữ chỗ");
    }

    #[test]
    fn local_file_skips_the_gate_and_the_download_even_when_the_switch_is_off() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let d = CloudDownloader::with(1, Duration::from_secs(1), |_| false, move |_, _, _| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert!(d.before_read(p("/x.ttf"), true).is_ok());
        assert!(d.before_read(p("/x.ttf"), false).is_ok(), "file ở máy: công tắc tắt không cản đọc");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn switch_off_refuses_a_real_placeholder_even_if_the_db_flag_was_stale() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let d = CloudDownloader::with(4, Duration::from_secs(1), |_| true, move |_, _, _| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let e = d.before_read(p("/x.ttf"), false).unwrap_err();
        assert_eq!(e.code(), "cloud_disabled");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "tắt thì không được tải");
        assert!(d.before_read(p("/x.ttf"), true).is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn download_budget_starts_after_the_permit_not_before() {
        // 1 chỗ; lần đầu giữ chỗ ~250 ms; lần hai chờ chỗ nhưng vẫn nhận ĐỦ hạn 30 s cho riêng nó
        let lefts = Arc::new(Mutex::new(Vec::<Duration>::new()));
        let l = Arc::clone(&lefts);
        let d = Arc::new(CloudDownloader::with(1, Duration::from_secs(30), |_| true, move |_, left, _permit| {
            l.lock().unwrap().push(left);
            std::thread::sleep(Duration::from_millis(250));
            Ok(())
        }));
        let d1 = Arc::clone(&d);
        let first = std::thread::spawn(move || d1.before_read(p("/a.ttf"), true));
        std::thread::sleep(Duration::from_millis(40));
        assert!(d.before_read(p("/b.ttf"), true).is_ok());
        first.join().unwrap().unwrap();
        let lefts = lefts.lock().unwrap();
        assert_eq!(lefts.len(), 2);
        assert!(lefts.iter().all(|x| *x == Duration::from_secs(30)), "mỗi lần tải nhận đủ hạn: {lefts:?}");
    }

    #[test]
    fn waiting_too_long_for_a_slot_is_busy_not_a_timeout() {
        let d = Arc::new(
            CloudDownloader::with(1, Duration::from_secs(30), |_| true, |_, _, _permit| {
                std::thread::sleep(Duration::from_millis(400));
                Ok(())
            })
            .queue_wait(Duration::from_millis(80)),
        );
        let d1 = Arc::clone(&d);
        let first = std::thread::spawn(move || d1.before_read(p("/a.ttf"), true));
        std::thread::sleep(Duration::from_millis(40));
        let t = Instant::now();
        let e = d.before_read(p("/b.ttf"), true).unwrap_err();
        assert_eq!(e.code(), "cloud_busy");
        assert!(t.elapsed() < Duration::from_secs(1), "{:?}", t.elapsed());
        first.join().unwrap().unwrap();
    }

    #[test]
    fn the_permit_stays_taken_while_a_stuck_read_lingers_after_the_caller_timed_out() {
        // ensure trả Timeout ngay nhưng luồng đọc (giữ permit) còn chạy 300 ms: chỗ chưa được nhả
        let d = CloudDownloader::with(1, Duration::from_secs(30), |_| true, |_, _, permit| {
            std::thread::spawn(move || {
                let _hold = permit;
                std::thread::sleep(Duration::from_millis(300));
            });
            Err(EnsureLocalError::Timeout)
        })
        .queue_wait(Duration::from_millis(50));
        assert_eq!(d.before_read(p("/a.ttf"), true).unwrap_err().code(), "cloud_timeout");
        assert_eq!(d.in_flight(), 1, "tải thật vẫn đang chạy → vẫn chiếm 1 chỗ");
        assert_eq!(d.before_read(p("/b.ttf"), true).unwrap_err().code(), "cloud_busy", "không mở thêm lần tải thứ 2 khi hết chỗ");
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(d.in_flight(), 0);
    }

    #[test]
    fn concurrent_requests_for_the_same_file_share_one_download() {
        let calls = Arc::new(AtomicUsize::new(0));
        let locals = Arc::new(AtomicUsize::new(0));
        let (c, lc) = (Arc::clone(&calls), Arc::clone(&locals));
        let d = Arc::new(
            CloudDownloader::with(4, Duration::from_secs(5), |_| true, move |_, _, _permit| {
                c.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(150));
                Ok(())
            })
            .on_local(move |_| {
                lc.fetch_add(1, Ordering::SeqCst);
            }),
        );
        let ts: Vec<_> = (0..6).map(|_| { let d = Arc::clone(&d); std::thread::spawn(move || d.before_read(p("/same.ttf"), true)) }).collect();
        for t in ts {
            assert!(t.join().unwrap().is_ok());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "một file, một lần tải");
        assert_eq!(locals.load(Ordering::SeqCst), 1);
        assert_eq!(d.in_flight(), 0);
        // xong rồi thì lần sau (file bị giải phóng lại) tải lại được
        assert!(d.before_read(p("/same.ttf"), true).is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn different_files_download_in_parallel_up_to_the_limit() {
        let (now, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let (n, pk) = (Arc::clone(&now), Arc::clone(&peak));
        let d = Arc::new(CloudDownloader::with(4, Duration::from_secs(10), |_| true, move |_, _, _permit| {
            let cur = n.fetch_add(1, Ordering::SeqCst) + 1;
            pk.fetch_max(cur, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(40));
            n.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        }));
        let ts: Vec<_> = (0..12)
            .map(|i| { let d = Arc::clone(&d); std::thread::spawn(move || d.before_read(Path::new(&format!("/f{i}.ttf")), true)) })
            .collect();
        for t in ts {
            assert!(t.join().unwrap().is_ok());
        }
        assert_eq!(peak.load(Ordering::SeqCst), 4);
        assert_eq!(d.in_flight(), 0);
    }

    #[test]
    fn on_local_runs_only_after_a_successful_download() {
        let seen = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let mk = |r: Result<(), EnsureLocalError>, dataless: bool| {
            let s = Arc::clone(&seen);
            CloudDownloader::with(4, Duration::from_secs(1), move |_| dataless, instant(r)).on_local(move |q| s.lock().unwrap().push(q.to_path_buf()))
        };
        mk(Ok(()), true).before_read(p("/ok.ttf"), true).unwrap();
        assert!(mk(Err(EnsureLocalError::Timeout), true).before_read(p("/slow.ttf"), true).is_err());
        mk(Ok(()), false).before_read(p("/already-local.ttf"), true).unwrap();
        assert_eq!(*seen.lock().unwrap(), vec![PathBuf::from("/ok.ttf")]);
    }

    #[test]
    fn download_errors_map_to_preview_errors() {
        let case = |e: EnsureLocalError| {
            CloudDownloader::with(4, Duration::from_secs(1), |_| true, instant(Err(e))).before_read(p("/x.ttf"), true).unwrap_err()
        };
        assert_eq!(case(EnsureLocalError::Timeout).code(), "cloud_timeout");
        assert_eq!(case(EnsureLocalError::Missing).code(), "not_found");
        assert_eq!(case(EnsureLocalError::Io("offline".into())).code(), "cloud_unavailable");
        assert_eq!(case(EnsureLocalError::StillCloudOnly).code(), "cloud_unavailable");
        assert_eq!(case(EnsureLocalError::PlaceholderStub).code(), "cloud_unavailable");
    }

    #[test]
    fn a_failed_download_is_not_remembered() {
        let ok = Arc::new(AtomicBool::new(false));
        let o = Arc::clone(&ok);
        let d = CloudDownloader::with(4, Duration::from_secs(1), |_| true, move |_, _, _permit| {
            if o.load(Ordering::SeqCst) { Ok(()) } else { Err(EnsureLocalError::Io("offline".into())) }
        });
        assert!(d.before_read(p("/x.ttf"), true).is_err());
        ok.store(true, Ordering::SeqCst);
        assert!(d.before_read(p("/x.ttf"), true).is_ok(), "mạng về là tải được, không nhớ lỗi cũ");
    }
}
