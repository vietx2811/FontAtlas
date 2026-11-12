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
use crate::services::cloud::{CLOUD_QUEUE_WAIT, CloudGate, MAX_CLOUD_DOWNLOADS, Permit, global_gate};
use crate::state::AppState;

/// Hạn chót cho MỘT font khi người dùng chủ động bấm tải (lâu hơn hạn xem trước 30 s: việc chạy nền, mạng chậm vẫn đáng chờ).
pub const BULK_FILE_TIMEOUT: Duration = Duration::from_secs(120);

/// Số luồng tải của MỘT lần \"Tải về máy\": chừa 1 chỗ của cổng chung cho xem trước (tổng toàn app vẫn ≤ `MAX_CLOUD_DOWNLOADS`).
pub const BULK_WORKERS: usize = MAX_CLOUD_DOWNLOADS - 1;

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
/// Tải một font, giữ `Permit` tới hết lần đọc thật; dừng sớm với `Cancelled` khi cờ huỷ bật.
pub type Fetch<'a> = dyn Fn(&Path, Permit, &AtomicBool) -> Result<(), EnsureLocalError> + Sync + 'a;

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
                    let r = match gate.acquire_unless(CLOUD_QUEUE_WAIT, cancel) {
                        None if cancel.load(Ordering::SeqCst) => break,
                        None => Err("cloud_busy".to_string()),
                        Some(permit) => match fetch(path, permit, cancel) {
                            Err(EnsureLocalError::Cancelled) => break, // huỷ giữa chừng: không phải lỗi của font này
                            other => other.map_err(|e| e.code().to_string()),
                        },
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
pub fn materialize(path: &Path, timeout: Duration, permit: Permit, cancel: &AtomicBool) -> Result<(), EnsureLocalError> {
    materialize_by(
        path,
        timeout,
        permit,
        cancel,
        |p, t, hold, c| fontatlas_activate::ensure_local_holding_cancellable(p, t, hold, c),
        request_ubiquitous_download,
        fontatlas_activate::probe_file,
    )
}

/// `permit` được giữ tới khi hàm trả về — kể cả vòng xin tải/chờ của stub iCloud — để số lần tải thật đồng thời không vượt cổng.
pub(crate) fn materialize_by(
    path: &Path,
    timeout: Duration,
    permit: Permit,
    cancel: &AtomicBool,
    ensure: impl Fn(&Path, Duration, Arc<Permit>, &AtomicBool) -> Result<(), EnsureLocalError>,
    start_download: impl Fn(&Path) -> Result<(), String>,
    probe: impl Fn(&Path) -> FileProbe,
) -> Result<(), EnsureLocalError> {
    let began = Instant::now();
    let permit = Arc::new(permit);
    match ensure(path, timeout, Arc::clone(&permit), cancel) {
        Err(EnsureLocalError::PlaceholderStub) => {}
        other => return other,
    }
    // chỉ còn stub `.Tên.ttf.icloud`: không có file thật để đọc. `permit` vẫn trong tay tới hết vòng chờ bên dưới.
    if cancel.load(Ordering::SeqCst) {
        return Err(EnsureLocalError::Cancelled);
    }
    start_download(path).map_err(EnsureLocalError::Io)?;
    loop {
        match probe(path) {
            FileProbe::Ready => return Ok(()),
            FileProbe::Missing => return Err(EnsureLocalError::Missing),
            FileProbe::NotDownloaded => {}
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(EnsureLocalError::Cancelled);
        }
        if began.elapsed() >= timeout {
            return Err(EnsureLocalError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(50));
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
    download_source_with(state, source_id, &|p, permit, cancel| materialize(p, BULK_FILE_TIMEOUT, permit, cancel), progress)
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
    // cổng CHUNG của cả app: nhiều nguồn tải cùng lúc hay đang xem trước cũng không vượt MAX_CLOUD_DOWNLOADS lần tải thật
    let mut report = run(&paths, global_gate(), BULK_WORKERS, &cancel, fetch, progress);
    // có font mới về máy (kể cả khi huỷ giữa chừng) → quét lại đúng nguồn này để chúng hiện ra; nguồn đang quét thì xếp
    // thêm một lượt sau lượt hiện tại (lượt đó có thể đã đi qua chỗ vừa đổi). `rescan_queued` chỉ true khi thật sự đã xếp.
    if report.downloaded > 0 {
        report.rescan_queued = state.scans.enqueue_follow_up(state, &[source_id]);
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
    fn permit() -> Permit {
        CloudGate::new(1).acquire(Duration::from_secs(1)).unwrap()
    }
    const OK: fn(&Path, Permit, &AtomicBool) -> Result<(), EnsureLocalError> = |_, _, _| Ok(());

    #[test]
    fn downloads_everything_reports_progress_and_never_exceeds_the_limit() {
        // hai lượt tải BẮT BUỘC chồng nhau (latch, không dựa vào sleep): mỗi lượt chờ tới khi thấy ≥ 2 lượt đang chạy
        let (now, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let (n2, p2) = (Arc::clone(&now), Arc::clone(&peak));
        let fetch = move |_: &Path, _permit: Permit, _c: &AtomicBool| {
            let cur = n2.fetch_add(1, Ordering::SeqCst) + 1;
            p2.fetch_max(cur, Ordering::SeqCst);
            let t = Instant::now();
            while p2.load(Ordering::SeqCst) < 2 && t.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(1));
            }
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
        assert!(peak.load(Ordering::SeqCst) >= 2, "có song song");
        assert!(peak.load(Ordering::SeqCst) <= 4, "không vượt giới hạn: {peak:?}");
        assert_eq!(g.in_flight(), 0, "nhả hết chỗ");
    }

    #[test]
    fn the_limit_is_global_two_runs_sharing_one_gate_never_exceed_it() {
        let g = gate(3);
        let (now, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let mk = || {
            let (n, p) = (Arc::clone(&now), Arc::clone(&peak));
            move |_: &Path, _permit: Permit, _c: &AtomicBool| {
                let cur = n.fetch_add(1, Ordering::SeqCst) + 1;
                p.fetch_max(cur, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(15));
                n.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            }
        };
        std::thread::scope(|sc| {
            for _ in 0..2 {
                let (g, f) = (&g, mk());
                sc.spawn(move || run(&paths(12), g, 3, &AtomicBool::new(false), &f, &mut |_| {}));
            }
        });
        assert!(peak.load(Ordering::SeqCst) <= 3, "hai lần chạy x 3 luồng vẫn ≤ 3 lần tải thật: {peak:?}");
        assert_eq!(g.in_flight(), 0);
    }

    #[test]
    fn one_failure_does_not_stop_the_others_and_keeps_the_first_code() {
        let fetch = |p: &Path, _permit: Permit, _c: &AtomicBool| {
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
        let fetch = move |_: &Path, _permit: Permit, _c: &AtomicBool| {
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
    fn a_fetch_that_reports_cancelled_is_neither_a_download_nor_a_failure() {
        let fetch = |_: &Path, _permit: Permit, _c: &AtomicBool| Err(EnsureLocalError::Cancelled);
        let r = run(&paths(5), &gate(2), 2, &AtomicBool::new(false), &fetch, &mut |_| {});
        assert_eq!((r.downloaded, r.failed, r.first_failure_code), (0, 0, None), "huỷ giữa chừng không bị tính là font lỗi");
    }

    #[test]
    fn cancel_while_waiting_for_a_slot_returns_fast_not_after_the_queue_wait() {
        let g = gate(1);
        let _held = g.acquire(Duration::from_secs(1)).unwrap(); // cổng đầy, giữ suốt test
        let cancel = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&cancel);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            c.store(true, Ordering::SeqCst);
        });
        let t = Instant::now();
        let r = run(&paths(4), &g, 2, &cancel, &OK, &mut |_| {});
        assert!(t.elapsed() < Duration::from_secs(3), "không chờ hết CLOUD_QUEUE_WAIT (60 s): {:?}", t.elapsed());
        assert!(r.cancelled);
        assert_eq!((r.downloaded, r.failed), (0, 0), "chờ chỗ mà bị huỷ không phải lỗi cloud_busy");
    }

    #[test]
    fn empty_list_is_a_clean_zero_report() {
        let r = run(&[], &gate(4), 4, &AtomicBool::new(false), &OK, &mut |_| {});
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

    type Ens = fn(&Path, Duration, Arc<Permit>, &AtomicBool) -> Result<(), EnsureLocalError>;
    const ENS_OK: Ens = |_, _, _, _| Ok(());
    const ENS_STUB: Ens = |_, _, _, _| Err(EnsureLocalError::PlaceholderStub);

    #[test]
    fn a_dataless_file_goes_through_ensure_only() {
        let started = AtomicBool::new(false);
        let r = materialize_by(Path::new("/x.ttf"), Duration::from_secs(1), permit(), &AtomicBool::new(false), ENS_OK, |_| { started.store(true, Ordering::SeqCst); Ok(()) }, |_| FileProbe::Ready);
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
            &AtomicBool::new(false),
            ENS_STUB,
            |_| { started.fetch_add(1, Ordering::SeqCst); Ok(()) },
            |_| if polls.fetch_add(1, Ordering::SeqCst) < 2 { FileProbe::NotDownloaded } else { FileProbe::Ready },
        );
        assert!(r.is_ok());
        assert_eq!(started.load(Ordering::SeqCst), 1);
        assert!(polls.load(Ordering::SeqCst) >= 3);
    }

    #[test]
    fn the_permit_is_held_for_the_whole_stub_request_and_poll_loop() {
        let g = gate(1);
        let p = g.acquire(Duration::from_secs(1)).unwrap();
        let during = Arc::new(Mutex::new(Vec::<usize>::new()));
        let (g2, d2) = (Arc::clone(&g), Arc::clone(&during));
        let polls = AtomicUsize::new(0);
        let r = materialize_by(
            Path::new("/x.ttf"),
            Duration::from_secs(5),
            p,
            &AtomicBool::new(false),
            ENS_STUB,
            |_| { d2.lock().unwrap().push(g2.in_flight()); Ok(()) },
            |_| { during.lock().unwrap().push(g.in_flight()); if polls.fetch_add(1, Ordering::SeqCst) < 2 { FileProbe::NotDownloaded } else { FileProbe::Ready } },
        );
        assert!(r.is_ok());
        let seen = during.lock().unwrap().clone();
        assert!(seen.len() >= 4 && seen.iter().all(|n| *n == 1), "chỗ vẫn bị chiếm trong lúc xin tải và chờ: {seen:?}");
        assert_eq!(g.in_flight(), 0, "xong mới nhả");
    }

    #[test]
    fn cancel_during_the_stub_poll_loop_stops_it_within_a_second() {
        let cancel = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&cancel);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            c.store(true, Ordering::SeqCst);
        });
        let t = Instant::now();
        let r = materialize_by(Path::new("/x.ttf"), Duration::from_secs(120), permit(), &cancel, ENS_STUB, |_| Ok(()), |_| FileProbe::NotDownloaded);
        assert_eq!(r, Err(EnsureLocalError::Cancelled));
        assert!(t.elapsed() < Duration::from_secs(1), "{:?}", t.elapsed());
        // đã huỷ trước khi xin tải: không xin hệ thống tải
        let asked = AtomicBool::new(false);
        let r = materialize_by(Path::new("/x.ttf"), Duration::from_secs(5), permit(), &AtomicBool::new(true), ENS_STUB, |_| { asked.store(true, Ordering::SeqCst); Ok(()) }, |_| FileProbe::NotDownloaded);
        assert_eq!(r, Err(EnsureLocalError::Cancelled));
        assert!(!asked.load(Ordering::SeqCst));
    }

    #[test]
    fn an_icloud_stub_that_never_arrives_times_out_and_a_refused_request_is_io() {
        let no = AtomicBool::new(false);
        let t = materialize_by(Path::new("/x.ttf"), Duration::from_millis(250), permit(), &no, ENS_STUB, |_| Ok(()), |_| FileProbe::NotDownloaded);
        assert_eq!(t, Err(EnsureLocalError::Timeout));
        let io = materialize_by(Path::new("/x.ttf"), Duration::from_secs(1), permit(), &no, ENS_STUB, |_| Err("not signed in".into()), |_| FileProbe::NotDownloaded);
        assert_eq!(io, Err(EnsureLocalError::Io("not signed in".into())));
        let other = materialize_by(Path::new("/x.ttf"), Duration::from_secs(1), permit(), &no, |_, _, _, _| Err(EnsureLocalError::Timeout), |_| panic!("không xin tải"), |_| FileProbe::Ready);
        assert_eq!(other, Err(EnsureLocalError::Timeout), "lỗi khác giữ nguyên");
    }
}
