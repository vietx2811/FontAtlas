//! B5 App Font Sets: Case gắn với app. Bộ quan sát app đang chạy (NSWorkspace) + CRUD liên kết + danh sách app đã cài.
//!
//! Phát hiện bằng THĂM DÒ `NSWorkspace.runningApplications` mỗi `POLL` (không phải thông báo didLaunch/didTerminate): đã đo cả hai
//! cho bundle id + pid như nhau; thăm dò không cần run loop riêng, không bỏ sót app chạy sẵn lúc mở Font Atlas, và
//! `sync_app_sets` là đồng bộ theo trạng thái nên không cần từng sự kiện. Độ trễ phát hiện ≤ `POLL`.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fontatlas_activate::{AppSetEvent, AppSetTracker, DEFAULT_GRACE};
use fontatlas_store::CaseApp;

use crate::dto::activation::{AppSetEventDto, CaseAppDto, InstalledAppDto};
use crate::dto::events::ActivationEvent;
use crate::error::CmdError;
use crate::state::AppState;

/// Nhịp thăm dò app đang chạy.
pub const POLL: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct AppSets {
    /// Trạng thái ân hạn/tắt-lần-này, chung giữa luồng quan sát và lệnh `case_app_suppress`.
    tracker: Mutex<AppSetTracker>,
    stop: AtomicBool,
    handle: Mutex<Option<JoinHandle<()>>>,
}

/// Bundle id của mọi app đang chạy. Nền tảng khác macOS: rỗng (chưa hỗ trợ).
pub fn running_bundle_ids() -> HashSet<String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;
        let ws = NSWorkspace::sharedWorkspace();
        ws.runningApplications().iter().filter_map(|a| a.bundleIdentifier()).map(|b| b.to_string()).collect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        HashSet::new()
    }
}

/// Một nhịp: đồng bộ rồi phát sự kiện. Tách ra để test không cần luồng hay NSWorkspace.
pub fn tick(state: &AppState, running: &HashSet<String>, now: Instant, grace: Duration, tracker: &mut AppSetTracker) -> Vec<AppSetEvent> {
    let Ok(store) = state.store() else { return vec![] };
    match state.activation.sync_app_sets(store, running, now, grace, tracker) {
        Ok(events) => {
            for e in &events {
                state.hubs.activation.emit(ActivationEvent::AppSet { event: AppSetEventDto::from(e) });
            }
            events
        }
        Err(e) => {
            log::warn!("đồng bộ font theo app lỗi: {e}");
            vec![]
        }
    }
}

/// Một nhịp với trạng thái CHUNG của app (cùng bộ nhớ với `case_app_suppress`). Luồng quan sát và test dùng hàm này.
pub fn sync_once(state: &AppState, running: &HashSet<String>, now: Instant, grace: Duration) -> Vec<AppSetEvent> {
    let mut tracker = state.app_sets.tracker.lock().unwrap_or_else(|p| p.into_inner());
    tick(state, running, now, grace, &mut tracker)
}

/// Khởi động luồng quan sát (một lần). Gọi sau `reconcile` lúc mở app.
pub fn start(state: &Arc<AppState>) {
    if !state.activation.supported() {
        return;
    }
    let mut h = state.app_sets.handle.lock().unwrap_or_else(|p| p.into_inner());
    if h.is_some() {
        return;
    }
    state.app_sets.stop.store(false, Ordering::SeqCst);
    let st = Arc::clone(state);
    *h = std::thread::Builder::new()
        .name("fa-app-sets".into())
        .spawn(move || {
            while !st.app_sets.stop.load(Ordering::SeqCst) {
                let events = sync_once(&st, &running_bundle_ids(), Instant::now(), DEFAULT_GRACE);
                if events.iter().any(|e| e.changed > 0) {
                    st.hubs.activation.emit(ActivationEvent::Changed { hashes: vec![] });
                }
                // ngủ từng lát 100 ms để `stop` có hiệu lực ngay
                let t0 = Instant::now();
                while t0.elapsed() < POLL && !st.app_sets.stop.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        })
        .ok();
}

/// Dừng luồng quan sát (lúc thoát; chờ luồng xong). Không gỡ font: `on_exit` làm việc đó.
pub fn stop(state: &AppState) {
    state.app_sets.stop.store(true, Ordering::SeqCst);
    let h = state.app_sets.handle.lock().unwrap_or_else(|p| p.into_inner()).take();
    if let Some(h) = h {
        let _ = h.join();
    }
}

/// Nút "Tắt lần này" trên toast tự bật: gỡ lý do `caseapp:<id>` của Case (không đụng lý do khác) và chặn bật lại cho tới khi
/// mọi app của Case thoát rồi chạy lại.
pub fn case_app_suppress(state: &AppState, collection_id: i64) -> Result<(), CmdError> {
    if !state.activation.supported() {
        return Ok(());
    }
    let mut tracker = state.app_sets.tracker.lock().unwrap_or_else(|p| p.into_inner());
    let rep = state.activation.suppress_case_app(state.store()?, collection_id, &mut tracker)?;
    if rep.results.iter().any(|r| r.outcome == fontatlas_activate::FileOutcome::Deactivated) {
        state.hubs.activation.emit(ActivationEvent::Changed { hashes: vec![] });
    }
    Ok(())
}

pub fn case_apps(state: &AppState, collection_id: i64) -> Result<Vec<CaseAppDto>, CmdError> {
    let apps = state.with_user(|u| Ok(u.case_apps(collection_id)?))?;
    Ok(apps.into_iter().map(|a| CaseAppDto { bundle_id: a.bundle_id, name: a.app_name }).collect())
}

pub fn set_case_apps(state: &AppState, collection_id: i64, apps: &[CaseAppDto]) -> Result<(), CmdError> {
    let rows: Vec<CaseApp> = apps.iter().map(|a| CaseApp { bundle_id: a.bundle_id.clone(), app_name: a.name.clone() }).collect();
    state.with_user(|u| Ok(u.set_case_apps(collection_id, &rows)?))
}

/// App đã cài, có bundle id, theo tên. `running` lấy từ lúc quét.
pub fn installed_apps() -> Vec<InstalledAppDto> {
    let running = running_bundle_ids();
    let mut roots: Vec<PathBuf> = vec!["/Applications".into(), "/System/Applications".into()];
    if let Some(h) = std::env::var_os("HOME") {
        roots.push(Path::new(&h).join("Applications"));
    }
    scan_apps(&roots, &running)
}

/// Quét `roots` (một cấp con thư mục, vd `Adobe Illustrator 2026/Adobe Illustrator.app`). Thuần theo đĩa: test dùng thư mục giả.
pub fn scan_apps(roots: &[PathBuf], running: &HashSet<String>) -> Vec<InstalledAppDto> {
    let mut out: Vec<InstalledAppDto> = vec![];
    let mut seen = HashSet::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let mut cands = vec![];
            if p.extension().is_some_and(|x| x == "app") {
                cands.push(p);
            } else if p.is_dir()
                && let Ok(sub) = std::fs::read_dir(&p)
            {
                cands.extend(sub.flatten().map(|s| s.path()).filter(|s| s.extension().is_some_and(|x| x == "app")));
            }
            for app in cands {
                if let Some(a) = read_app(&app, running)
                    && seen.insert(a.bundle_id.clone())
                {
                    out.push(a);
                }
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

fn read_app(app: &Path, running: &HashSet<String>) -> Option<InstalledAppDto> {
    let v = plist::Value::from_file(app.join("Contents/Info.plist")).ok()?;
    let d = v.as_dictionary()?;
    let bundle_id = d.get("CFBundleIdentifier")?.as_string()?.to_string();
    if bundle_id.len() < 3 {
        return None;
    }
    let name = ["CFBundleDisplayName", "CFBundleName"]
        .iter()
        .find_map(|k| d.get(k).and_then(|x| x.as_string()).filter(|s| !s.is_empty()))
        .map(str::to_string)
        .or_else(|| app.file_stem().map(|s| s.to_string_lossy().into_owned()))?;
    let running = running.contains(&bundle_id);
    Some(InstalledAppDto { bundle_id, name, path: app.to_string_lossy().into_owned(), running })
}
