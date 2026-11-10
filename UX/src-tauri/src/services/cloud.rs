//! Tải font từ ổ cloud để xem trước: cổng giới hạn đồng thời + hạn chót, dùng cho preview và FontFace.
use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use fontatlas_activate::EnsureLocalError;
use fontatlas_preview::PreviewError;

/// Số font cloud được tải cùng lúc để xem trước.
pub const MAX_CLOUD_DOWNLOADS: usize = 4;
/// Hạn chót cho một font (gồm cả thời gian chờ tới lượt).
pub const CLOUD_PREVIEW_TIMEOUT: Duration = Duration::from_secs(30);

/// Cổng đếm: tối đa `max` người giữ giấy phép; ai không vào kịp hạn chót thì bỏ.
pub struct CloudGate {
    max: usize,
    busy: Mutex<usize>,
    freed: Condvar,
}

pub struct Permit<'a>(&'a CloudGate);

impl CloudGate {
    pub fn new(max: usize) -> Self {
        CloudGate { max: max.max(1), busy: Mutex::new(0), freed: Condvar::new() }
    }

    /// Chờ tới lượt tối đa `wait`; None nếu hết hạn.
    pub fn acquire(&self, wait: Duration) -> Option<Permit<'_>> {
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
        Some(Permit(self))
    }

    pub fn in_flight(&self) -> usize {
        *self.busy.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut busy = self.0.busy.lock().unwrap_or_else(|e| e.into_inner());
        *busy = busy.saturating_sub(1);
        self.0.freed.notify_one();
    }
}

type Probe = Box<dyn Fn(&Path) -> bool + Send + Sync>;
type Ensure = Box<dyn Fn(&Path, Duration) -> Result<(), EnsureLocalError> + Send + Sync>;

/// Đưa file cloud về máy ngay trước khi đọc. File đã ở máy: đi thẳng, không qua cổng.
pub struct CloudDownloader {
    gate: CloudGate,
    total: Duration,
    is_dataless: Probe,
    ensure: Ensure,
}

impl Default for CloudDownloader {
    fn default() -> Self {
        CloudDownloader {
            gate: CloudGate::new(MAX_CLOUD_DOWNLOADS),
            total: CLOUD_PREVIEW_TIMEOUT,
            is_dataless: Box::new(|p| std::fs::metadata(p).map(|m| fontatlas_store::os_placeholder(p, &m)).unwrap_or(false)),
            ensure: Box::new(fontatlas_activate::ensure_local),
        }
    }
}

impl CloudDownloader {
    /// Thay cách thăm dò / tải / hạn chót (test không cần ổ cloud thật).
    pub fn with(
        max: usize,
        total: Duration,
        is_dataless: impl Fn(&Path) -> bool + Send + Sync + 'static,
        ensure: impl Fn(&Path, Duration) -> Result<(), EnsureLocalError> + Send + Sync + 'static,
    ) -> Self {
        CloudDownloader { gate: CloudGate::new(max), total, is_dataless: Box::new(is_dataless), ensure: Box::new(ensure) }
    }

    pub fn in_flight(&self) -> usize {
        self.gate.in_flight()
    }

    pub fn before_read(&self, path: &Path) -> Result<(), PreviewError> {
        if !(self.is_dataless)(path) {
            return Ok(());
        }
        let started = Instant::now();
        let _permit = self.gate.acquire(self.total).ok_or(PreviewError::CloudTimeout)?;
        let left = self.total.saturating_sub(started.elapsed());
        match (self.ensure)(path, left) {
            Ok(()) => Ok(()),
            Err(EnsureLocalError::Timeout) => Err(PreviewError::CloudTimeout),
            Err(EnsureLocalError::Missing) => Err(PreviewError::NotFound),
            Err(e) => Err(PreviewError::CloudUnavailable(e.message())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn gate_never_exceeds_its_limit_under_contention() {
        let gate = Arc::new(CloudGate::new(4));
        let (now, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let (g, n, p) = (Arc::clone(&gate), Arc::clone(&now), Arc::clone(&peak));
                std::thread::spawn(move || {
                    let permit = g.acquire(Duration::from_secs(10)).expect("phải tới lượt");
                    let cur = n.fetch_add(1, Ordering::SeqCst) + 1;
                    p.fetch_max(cur, Ordering::SeqCst);
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
        assert_eq!(gate.in_flight(), 0, "mọi giấy phép đã trả");
    }

    #[test]
    fn saturated_gate_gives_up_at_the_deadline() {
        let gate = CloudGate::new(1);
        let _held = gate.acquire(Duration::from_secs(1)).unwrap();
        let t = Instant::now();
        assert!(gate.acquire(Duration::from_millis(120)).is_none());
        assert!(t.elapsed() >= Duration::from_millis(100) && t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
        assert_eq!(gate.in_flight(), 1, "người bỏ cuộc không giữ chỗ");
    }

    #[test]
    fn local_file_skips_the_gate_and_the_download() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let d = CloudDownloader::with(1, Duration::from_secs(1), |_| false, move |_, _| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert!(d.before_read(&PathBuf::from("/x.ttf")).is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn dataless_file_is_downloaded_within_the_remaining_budget() {
        let seen = Arc::new(Mutex::new(None));
        let s = Arc::clone(&seen);
        let d = CloudDownloader::with(4, Duration::from_secs(30), |_| true, move |_, left| {
            *s.lock().unwrap() = Some(left);
            Ok(())
        });
        assert!(d.before_read(Path::new("/x.ttf")).is_ok());
        let left = seen.lock().unwrap().unwrap();
        assert!(left <= Duration::from_secs(30) && left > Duration::from_secs(25), "{left:?}");
    }

    #[test]
    fn download_errors_map_to_preview_errors() {
        let case = |e: EnsureLocalError| {
            CloudDownloader::with(4, Duration::from_secs(1), |_| true, move |_, _| Err(e.clone())).before_read(Path::new("/x.ttf")).unwrap_err()
        };
        assert_eq!(case(EnsureLocalError::Timeout).code(), "cloud_timeout");
        assert_eq!(case(EnsureLocalError::Missing).code(), "not_found");
        assert_eq!(case(EnsureLocalError::Io("offline".into())).code(), "cloud_unavailable");
        assert_eq!(case(EnsureLocalError::StillCloudOnly).code(), "cloud_unavailable");
        assert_eq!(case(EnsureLocalError::PlaceholderStub).code(), "cloud_unavailable");
    }

    #[test]
    fn at_most_four_downloads_run_at_once_and_the_rest_wait() {
        let d = Arc::new(CloudDownloader::with(4, Duration::from_secs(10), |_| true, {
            let (now, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
            move |_, _| {
                let cur = now.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(cur, Ordering::SeqCst);
                assert!(peak.load(Ordering::SeqCst) <= 4, "vượt 4 tải đồng thời");
                std::thread::sleep(Duration::from_millis(40));
                now.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            }
        }));
        let ts: Vec<_> = (0..12).map(|_| { let d = Arc::clone(&d); std::thread::spawn(move || d.before_read(Path::new("/x.ttf"))) }).collect();
        for t in ts {
            assert!(t.join().unwrap().is_ok());
        }
        assert_eq!(d.in_flight(), 0);
    }

    #[test]
    fn waiting_past_the_deadline_is_a_cloud_timeout_not_a_hang() {
        // 1 chỗ, ensure chậm 300 ms, hạn chót 100 ms → người thứ hai hết hạn khi chờ lượt
        let d = Arc::new(CloudDownloader::with(1, Duration::from_millis(100), |_| true, |_, _| {
            std::thread::sleep(Duration::from_millis(300));
            Ok(())
        }));
        let d1 = Arc::clone(&d);
        let first = std::thread::spawn(move || d1.before_read(Path::new("/a.ttf")));
        std::thread::sleep(Duration::from_millis(30));
        let t = Instant::now();
        let e = d.before_read(Path::new("/b.ttf")).unwrap_err();
        assert_eq!(e.code(), "cloud_timeout");
        assert!(t.elapsed() < Duration::from_secs(1), "{:?}", t.elapsed());
        first.join().unwrap().unwrap();
    }
}
