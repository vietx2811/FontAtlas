//! "Tải về máy": đưa các font còn là placeholder trên ổ cloud của MỘT nguồn về máy, có tiến độ, huỷ được, giới hạn đồng thời
//! (dùng lại `CloudGate` của xem trước), xong thì quét lại nguồn để font hiện ra. Việc đọc thật nằm ở `fontatlas_activate`
//! (đọc 1 byte để hệ điều hành tải; stub iCloud kiểu cũ thì xin `startDownloadingUbiquitousItemAtURL`).
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use fontatlas_activate::{EnsureLocalError, FileProbe};

use crate::dto::library::{CloudDownloadProgressDto, CloudDownloadReportDto};
use crate::error::CmdError;
use crate::services::cloud::{CLOUD_QUEUE_WAIT, CloudGate, MAX_CLOUD_DOWNLOADS, Permit};
use crate::state::AppState;

/// Hạn chót cho MỘT font khi người dùng chủ động bấm tải (lâu hơn hạn xem trước 30 s: việc chạy nền, mạng chậm vẫn đáng chờ).
pub const BULK_FILE_TIMEOUT: Duration = Duration::from_secs(120);

/// Cờ huỷ của các lần tải đang chạy, theo `source_id`.
#[derive(Default)]
pub struct CloudJobs(Mutex<HashMap<i64, Arc<AtomicBool>>>);

impl CloudJobs {
    /// Đăng ký lần tải cho nguồn; `None` nếu nguồn đã có lần tải đang chạy.
    fn begin(&self, id: i64) -> Option<(Arc<AtomicBool>, JobGuard<'_>)> {
        let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if m.contains_key(&id) {
            return None;
        }
        let flag = Arc::new(AtomicBool::new(false));
        m.insert(id, Arc::clone(&flag));
        Some((flag, JobGuard { jobs: self, id }))
    }

    /// Huỷ lần tải của nguồn; true nếu có lần đang chạy.
    pub fn cancel(&self, id: i64) -> bool {
        match self.0.lock().unwrap_or_else(|e| e.into_inner()).get(&id) {
            Some(f) => {
                f.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn is_running(&self, id: i64) -> bool {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&id)
    }
}

struct JobGuard<'a> {
    jobs: &'a CloudJobs,
    id: i64,
}

impl Drop for JobGuard<'_> {
    fn drop(&mut self) {
        self.jobs.0.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.id);
    }
}

/// Một font: `Ok` hoặc mã lỗi ổn định.
type FetchResult = Result<(), String>;
/// Tải một font, giữ `Permit` tới hết lần đọc thật.
pub type Fetch<'a> = dyn Fn(&Path, Permit) -> Result<(), EnsureLocalError> + Sync + 'a;

/// Lõi thuần (không biết AppState): tải `paths` bằng tối đa `gate.max` luồng, báo tiến độ ở luồng gọi, dừng nhận việc mới khi `cancel`.
pub fn run(
    paths: &[PathBuf],
    gate: &Arc<CloudGate>,
    workers: usize,
    cancel: &AtomicBool,
    fetch: &Fetch<'_>,
    progress: &mut dyn FnMut(CloudDownloadProgressDto),
) -> CloudDownloadReportDto {
    let total = paths.len() as u32;
    let (mut downloaded, mut failed) = (0u32, 0u32);
    let mut first_failure_code: Option<String> = None;
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel::<FetchResult>();
    std::thread::scope(|sc| {
        for _ in 0..workers.max(1).min(paths.len().max(1)) {
            let tx = tx.clone();
            let next = &next;
            sc.spawn(move || {
                loop {
                    if cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(path) = paths.get(i) else { break };
                    let r = match gate.acquire(CLOUD_QUEUE_WAIT) {
                        None => Err("cloud_busy".to_string()),
                        Some(permit) => fetch(path, permit).map_err(|e| e.code().to_string()),
                    };
                    if tx.send(r).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        // luồng gọi nhận kết quả tới khi mọi worker xong
        for r in rx {
            match r {
                Ok(()) => downloaded += 1,
                Err(code) => {
                    failed += 1;
                    first_failure_code.get_or_insert(code);
                }
            }
            progress(CloudDownloadProgressDto { done: downloaded + failed, total, failed });
        }
    });
    CloudDownloadReportDto { total, downloaded, failed, cancelled: cancel.load(Ordering::SeqCst) && downloaded + failed < total, rescan_queued: false, first_failure_code }
}

/// Tải thật một font: file dataless → đọc để kích tải; stub iCloud kiểu cũ → xin hệ thống tải rồi chờ stub biến thành file.
pub fn materialize(path: &Path, timeout: Duration, permit: Permit) -> Result<(), EnsureLocalError> {
    materialize_by(path, timeout, permit, fontatlas_activate::ensure_local_holding::<Permit>, request_ubiquitous_download, fontatlas_activate::probe_file)
}

pub(crate) fn materialize_by(
    path: &Path,
    timeout: Duration,
    permit: Permit,
    ensure: impl Fn(&Path, Duration, Permit) -> Result<(), EnsureLocalError>,
    start_download: impl Fn(&Path) -> Result<(), String>,
    probe: impl Fn(&Path) -> FileProbe,
) -> Result<(), EnsureLocalError> {
    let began = Instant::now();
    match ensure(path, timeout, permit) {
        Err(EnsureLocalError::PlaceholderStub) => {}
        other => return other,
    }
    // chỉ còn stub `.Tên.ttf.icloud`: không có file thật để đọc
    start_download(path).map_err(EnsureLocalError::Io)?;
    loop {
        match probe(path) {
            FileProbe::Ready => return Ok(()),
            FileProbe::Missing => return Err(EnsureLocalError::Missing),
            FileProbe::NotDownloaded => {}
        }
        if began.elapsed() >= timeout {
            return Err(EnsureLocalError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// macOS: `NSFileManager.startDownloadingUbiquitousItemAtURL:error:` (Finder "Download Now"). Nơi khác: không hỗ trợ.
#[cfg(target_os = "macos")]
fn request_ubiquitous_download(path: &Path) -> Result<(), String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let p = path.to_str().ok_or("đường dẫn không phải UTF-8")?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(p));
    NSFileManager::defaultManager().startDownloadingUbiquitousItemAtURL_error(&url).map_err(|e| e.localizedDescription().to_string())
}

#[cfg(not(target_os = "macos"))]
fn request_ubiquitous_download(_path: &Path) -> Result<(), String> {
    Err("tải font iCloud kiểu cũ chỉ hỗ trợ macOS".into())
}

/// Nguồn của `source_id` → danh sách font chưa tải. Lỗi: `not_found` (id lạ), `source_unavailable` (thư mục không còn).
fn pending(state: &AppState, source_id: i64) -> Result<Vec<PathBuf>, CmdError> {
    let src = state.with_user(|u| Ok(u.source(source_id)?))?;
    let path = src.path.as_deref().filter(|p| crate::services::sources::root_available(p)).ok_or_else(|| {
        CmdError::new("source_unavailable", format!("thư mục của nguồn “{}” hiện không truy cập được", src.name))
            .with_detail(serde_json::json!({ "sourceId": source_id, "name": src.name }))
    })?;
    Ok(fontatlas_store::cloud_placeholders(Path::new(path)))
}

/// "Tải về máy" cho một nguồn. Trả báo cáo; xong (không phải huỷ trơn) thì xếp quét lại nguồn.
pub fn download_source(
    state: &Arc<AppState>,
    source_id: i64,
    progress: &mut dyn FnMut(CloudDownloadProgressDto),
) -> Result<CloudDownloadReportDto, CmdError> {
    download_source_with(state, source_id, &|p, permit| materialize(p, BULK_FILE_TIMEOUT, permit), progress)
}

/// Như `download_source` nhưng thay được cách tải (test không cần ổ cloud thật).
pub fn download_source_with(
    state: &Arc<AppState>,
    source_id: i64,
    fetch: &Fetch<'_>,
    progress: &mut dyn FnMut(CloudDownloadProgressDto),
) -> Result<CloudDownloadReportDto, CmdError> {
    let Some((cancel, _guard)) = state.cloud_jobs.begin(source_id) else {
        return Err(CmdError::busy("nguồn này đang tải về máy"));
    };
    let paths = pending(state, source_id)?;
    progress(CloudDownloadProgressDto { done: 0, total: paths.len() as u32, failed: 0 });
    let gate = CloudGate::new(MAX_CLOUD_DOWNLOADS);
    let mut report = run(&paths, &gate, MAX_CLOUD_DOWNLOADS, &cancel, fetch, progress);
    // có font mới về máy (kể cả khi huỷ giữa chừng) → quét lại đúng nguồn này để chúng hiện ra
    if report.downloaded > 0 {
        state.scans.enqueue(state, &[source_id]);
        report.rescan_queued = true;
    }
    Ok(report)
}

/// Huỷ lần "Tải về máy" của nguồn; false nếu không có lần nào đang chạy.
pub fn cancel_download(state: &AppState, source_id: i64) -> bool {
    state.cloud_jobs.cancel(source_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn paths(n: usize) -> Vec<PathBuf> {
        (0..n).map(|i| PathBuf::from(format!("/cloud/f{i}.ttf"))).collect()
    }
    fn gate(n: usize) -> Arc<CloudGate> {
        CloudGate::new(n)
    }

    #[test]
    fn downloads_everything_reports_progress_and_never_exceeds_the_limit() {
        let (now, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let (n2, p2) = (Arc::clone(&now), Arc::clone(&peak));
        let fetch = move |_: &Path, _permit: Permit| {
            let cur = n2.fetch_add(1, Ordering::SeqCst) + 1;
            p2.fetch_max(cur, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(25));
            n2.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        };
        let g = gate(4);
        let mut seen = Vec::new();
        let r = run(&paths(20), &g, 4, &AtomicBool::new(false), &fetch, &mut |p| seen.push(p));
        assert_eq!((r.total, r.downloaded, r.failed, r.cancelled), (20, 20, 0, false));
        assert!(r.first_failure_code.is_none());
        assert_eq!(seen.len(), 20);
        assert!(seen.windows(2).all(|w| w[0].done < w[1].done), "tiến độ tăng dần");
        assert_eq!(seen.last().unwrap().done, 20);
        assert!(peak.load(Ordering::SeqCst) <= 4 && peak.load(Ordering::SeqCst) >= 2, "song song nhưng không vượt 4: {peak:?}");
        assert_eq!(g.in_flight(), 0, "nhả hết chỗ");
    }

    #[test]
    fn one_failure_does_not_stop_the_others_and_keeps_the_first_code() {
        let fetch = |p: &Path, _permit: Permit| {
            if p.ends_with("f3.ttf") {
                Err(EnsureLocalError::Timeout)
            } else if p.ends_with("f5.ttf") {
                Err(EnsureLocalError::Io("offline".into()))
            } else {
                Ok(())
            }
        };
        let r = run(&paths(8), &gate(1), 1, &AtomicBool::new(false), &fetch, &mut |_| {});
        assert_eq!((r.downloaded, r.failed), (6, 2));
        assert_eq!(r.first_failure_code.as_deref(), Some("cloud_timeout"), "1 luồng → lỗi đầu là f3");
        assert!(!r.cancelled);
    }

    #[test]
    fn cancel_stops_taking_new_files_and_reports_cancelled() {
        let cancel = Arc::new(AtomicBool::new(false));
        let c2 = Arc::clone(&cancel);
        let calls = Arc::new(AtomicUsize::new(0));
        let k = Arc::clone(&calls);
        let fetch = move |_: &Path, _permit: Permit| {
            if k.fetch_add(1, Ordering::SeqCst) + 1 == 3 {
                c2.store(true, Ordering::SeqCst); // người dùng bấm Huỷ giữa chừng
            }
            Ok(())
        };
        let r = run(&paths(50), &gate(1), 1, &cancel, &fetch, &mut |_| {});
        assert!(r.cancelled);
        assert_eq!(r.downloaded, 3, "file đang xử lý chạy nốt, không nhận thêm");
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert!(r.downloaded + r.failed < r.total);
    }

    #[test]
    fn empty_list_is_a_clean_zero_report() {
        let r = run(&[], &gate(4), 4, &AtomicBool::new(false), &|_: &Path, _p: Permit| Ok(()), &mut |_| {});
        assert_eq!((r.total, r.downloaded, r.failed, r.cancelled), (0, 0, 0, false));
    }

    #[test]
    fn jobs_allow_one_run_per_source_and_cancel_flags_it() {
        let jobs = CloudJobs::default();
        let (flag, guard) = jobs.begin(7).unwrap();
        assert!(jobs.begin(7).is_none(), "cùng nguồn không chạy hai lần");
        assert!(jobs.begin(8).is_some(), "nguồn khác thì được");
        assert!(jobs.is_running(7));
        assert!(!flag.load(Ordering::SeqCst));
        assert!(jobs.cancel(7));
        assert!(flag.load(Ordering::SeqCst));
        assert!(!jobs.cancel(99), "không có lần nào đang chạy");
        drop(guard);
        assert!(!jobs.is_running(7), "guard nhả khi xong (kể cả lỗi)");
        assert!(jobs.begin(7).is_some());
    }

    fn permit() -> Permit {
        CloudGate::new(1).acquire(Duration::from_secs(1)).unwrap()
    }

    #[test]
    fn a_dataless_file_goes_through_ensure_only() {
        let started = AtomicBool::new(false);
        let r = materialize_by(Path::new("/x.ttf"), Duration::from_secs(1), permit(), |_, _, _| Ok(()), |_| { started.store(true, Ordering::SeqCst); Ok(()) }, |_| FileProbe::Ready);
        assert!(r.is_ok());
        assert!(!started.load(Ordering::SeqCst), "không xin tải iCloud khi đọc là đủ");
    }

    #[test]
    fn an_icloud_stub_asks_the_system_then_waits_for_the_real_file() {
        let started = AtomicUsize::new(0);
        let polls = AtomicUsize::new(0);
        let r = materialize_by(
            Path::new("/x.ttf"),
            Duration::from_secs(5),
            permit(),
            |_, _, _| Err(EnsureLocalError::PlaceholderStub),
            |_| { started.fetch_add(1, Ordering::SeqCst); Ok(()) },
            |_| if polls.fetch_add(1, Ordering::SeqCst) < 2 { FileProbe::NotDownloaded } else { FileProbe::Ready },
        );
        assert!(r.is_ok());
        assert_eq!(started.load(Ordering::SeqCst), 1);
        assert!(polls.load(Ordering::SeqCst) >= 3);
    }

    #[test]
    fn an_icloud_stub_that_never_arrives_times_out_and_a_refused_request_is_io() {
        let t = materialize_by(Path::new("/x.ttf"), Duration::from_millis(250), permit(), |_, _, _| Err(EnsureLocalError::PlaceholderStub), |_| Ok(()), |_| FileProbe::NotDownloaded);
        assert_eq!(t, Err(EnsureLocalError::Timeout));
        let io = materialize_by(Path::new("/x.ttf"), Duration::from_secs(1), permit(), |_, _, _| Err(EnsureLocalError::PlaceholderStub), |_| Err("not signed in".into()), |_| FileProbe::NotDownloaded);
        assert_eq!(io, Err(EnsureLocalError::Io("not signed in".into())));
        let other = materialize_by(Path::new("/x.ttf"), Duration::from_secs(1), permit(), |_, _, _| Err(EnsureLocalError::Timeout), |_| panic!("không xin tải"), |_| FileProbe::Ready);
        assert_eq!(other, Err(EnsureLocalError::Timeout), "lỗi khác giữ nguyên");
    }
}
