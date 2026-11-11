//! Nối nguồn font request của hệ thống (`fontatlas-fontreq`) vào app: mở sau broker, đóng trước broker khi thoát.
//! Photoshop/After Effects hỏi font còn thiếu → thư viện bật session và trả file. Công tắc là setting
//! `auto_activation_enabled` (đọc mỗi request, không cần dừng/mở nguồn). Windows/Linux: không làm gì.
use std::sync::Arc;

use crate::state::AppState;

#[cfg(target_os = "macos")]
pub fn start(state: &Arc<AppState>) {
    use fontatlas_broker::Backend;
    use fontatlas_fontreq::{FontRequestSource, Observer, PidAppNamer, Responder};
    use std::time::Duration;

    if !state.activation.supported() || !state.system_hooks.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    // Nguồn này là của CẢ phiên đăng nhập (mọi app hỏi font đều tới). Bản chạy với thư mục dữ liệu riêng
    // (FONTATLAS_DATA_DIR: QA, test, thử máy) không được nhận thay thư viện thật; ép mở bằng FONTATLAS_FONTREQ=1.
    if std::env::var_os("FONTATLAS_DATA_DIR").is_some() && std::env::var_os("FONTATLAS_FONTREQ").is_none() {
        return;
    }
    let backend = crate::services::broker::AppBackend::new(state);
    let (b_ensure, b_on) = (backend.clone(), backend);
    let responder = Arc::new(Responder::new(move |req| b_ensure.ensure(req), move || b_on.enabled(), PidAppNamer));
    // Ghi thời gian trả lời: app hỏi chờ trong lúc này (Photoshop chờ bao lâu chưa đo; xem docs L2).
    // Probe tự kiểm sức khoẻ chỉ được GHI NHẬN là đã tới (không log, không nhật ký kích hoạt).
    let weak = Arc::downgrade(state);
    let observer: Arc<Observer> = Arc::new(move |pid, req, answer, took| {
        let name = req.ps.as_deref().or(req.family.as_deref()).unwrap_or("");
        if fontatlas_fontreq::is_probe_name(name) {
            if let Some(s) = weak.upgrade() {
                note_probe_seen(&s, name);
            }
            return;
        }
        if took > Duration::from_secs(2) {
            log::warn!("font request pid {pid} \"{name}\" trả lời chậm: {took:?}");
        } else {
            log::debug!("font request pid {pid} \"{name}\" → {} ({took:?})", if matches!(answer, fontatlas_fontreq::Answer::Nothing) { "không có" } else { "file" });
        }
    });
    start_with(state, RETRY_EVERY, move || {
        FontRequestSource::start(responder.clone(), Some(observer.clone()))
            .map(|s| Box::new(s) as Box<dyn std::any::Any + Send>)
            .map_err(|e| (e.code(), e.to_string()))
    });
    spawn_periodic_probe(state);
}

/// Tự kiểm sức khoẻ định kỳ: lần đầu ít giây sau khi mở, rồi mỗi `PROBE_EVERY` (và khi người dùng mở Settings → `auto_probe`).
pub const PROBE_EVERY: std::time::Duration = std::time::Duration::from_secs(300);

#[cfg(target_os = "macos")]
fn spawn_periodic_probe(state: &Arc<AppState>) {
    use std::sync::atomic::Ordering;
    let weak = Arc::downgrade(state);
    let _ = std::thread::Builder::new().name("fontatlas-fontreq-probe".into()).spawn(move || {
        let mut wait = std::time::Duration::from_secs(8);
        loop {
            let until = std::time::Instant::now() + wait;
            while std::time::Instant::now() < until {
                std::thread::sleep(std::time::Duration::from_millis(250));
                match weak.upgrade() {
                    Some(s) if !s.fontreq_stop.load(Ordering::SeqCst) => {}
                    _ => return,
                }
            }
            let Some(s) = weak.upgrade() else { return };
            if s.fontreq_stop.load(Ordering::SeqCst) {
                return;
            }
            probe(&s, PROBE_WAIT, spawn_probe_child);
            wait = PROBE_EVERY;
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn spawn_periodic_probe(_state: &Arc<AppState>) {}

/// Cờ dòng lệnh: tiến trình con chỉ hỏi một font rồi thoát (không khởi động Tauri). Xem `run_probe_child_if_asked`.
pub const PROBE_FLAG: &str = "--fontatlas-font-probe";

/// Chờ tối đa ngần này để request của probe tới nguồn; quá hạn = coi như bị bỏ đói.
pub const PROBE_WAIT: std::time::Duration = std::time::Duration::from_secs(4);

/// Chạy ở tiến trình con (gọi từ `main`): nếu có cờ probe thì hỏi font đó và trả `true` (main thoát ngay).
pub fn run_probe_child_if_asked() -> bool {
    let mut a = std::env::args().skip(1);
    if a.next().as_deref() != Some(PROBE_FLAG) {
        return false;
    }
    #[cfg(target_os = "macos")]
    if let Some(name) = a.next().filter(|n| fontatlas_fontreq::is_probe_name(n)) {
        let _ = fontatlas_fontreq::ask_font(&name);
    }
    true
}

#[cfg(target_os = "macos")]
fn spawn_probe_child(name: &str) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = std::process::Command::new(exe)
        .arg(PROBE_FLAG)
        .arg(name)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    // Dọn tiến trình con: thoát tự nhiên thường dưới 1 s; kẹt thì giết sau 15 s (chỉ PID của chính ta).
    let _ = std::thread::Builder::new().name("fontatlas-probe-reaper".into()).spawn(move || {
        let t = std::time::Instant::now();
        while t.elapsed() < std::time::Duration::from_secs(15) {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = child.kill();
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn spawn_probe_child(_name: &str) -> Result<(), String> {
    Err("không hỗ trợ".into())
}

/// Dùng cho tests/app: kết quả một lần tự kiểm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// Request từ tiến trình khác tới được nguồn của ta.
    Reached,
    /// Nguồn đang mở nhưng không nhận gì: tiến trình khác đang nhận font request trước ta (`fontreq_error = "starved"`).
    Starved,
    /// Nguồn chưa mở (không có gì để kiểm; lỗi mở nằm ở `fontreq_error` khác) hoặc không chạy được tiến trình con.
    Unknown,
}

#[doc(hidden)]
pub fn note_probe_seen(state: &AppState, name: &str) {
    if let Ok(mut v) = state.fontreq_seen.lock() {
        v.push(name.to_string());
        let n = v.len();
        if n > 8 {
            v.drain(..n - 8);
        }
    }
}

/// Tự kiểm: một TIẾN TRÌNH KHÁC (`spawn`) hỏi một font ngẫu nhiên không tồn tại; nguồn của ta có nhận được không?
/// (Hỏi từ chính tiến trình này không dùng được: đã đo, request cùng tiến trình luôn tới nguồn của nó dù ai khác giữ nguồn.)
/// Không nhận trong `wait` → `fontreq_error = "starved"`; nhận → xoá "starved". Tên probe không bao giờ vào nhật ký/log.
pub fn probe<F>(state: &AppState, wait: std::time::Duration, spawn: F) -> ProbeOutcome
where
    F: FnOnce(&str) -> Result<(), String>,
{
    if state.fontreq.lock().map(|g| g.is_none()).unwrap_or(true) {
        return ProbeOutcome::Unknown;
    }
    let name = fontatlas_fontreq::new_probe_name();
    if let Err(e) = spawn(&name) {
        log::debug!("không chạy được probe font request: {e}");
        return ProbeOutcome::Unknown;
    }
    let t = std::time::Instant::now();
    let reached = loop {
        if state.fontreq_seen.lock().map(|v| v.contains(&name)).unwrap_or(false) {
            break true;
        }
        if t.elapsed() >= wait {
            break false;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let Ok(mut e) = state.fontreq_error.lock() else { return ProbeOutcome::Unknown };
    if reached {
        if e.as_deref() == Some("starved") {
            *e = None;
        }
        ProbeOutcome::Reached
    } else {
        if e.is_none() {
            log::warn!("nguồn font request đang mở nhưng không nhận được request thử: có app khác nhận font request trước Font Atlas");
        }
        *e = Some("starved".into());
        ProbeOutcome::Starved
    }
}

/// Tự kiểm ngay (lệnh `auto_probe`, nút "Kiểm tra lại").
pub fn probe_now(state: &AppState) -> ProbeOutcome {
    probe(state, PROBE_WAIT, spawn_probe_child)
}

/// Giữa hai lần thử lại khi mở nguồn không được. Nguồn là tài nguyên của cả phiên đăng nhập: có thể đang bận (máy tải nặng,
/// dịch vụ font chưa sẵn sàng sau khi đăng nhập) và rảnh lại sau đó, nên không bỏ cuộc sau lần đầu.
pub const RETRY_EVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// Mở nguồn bằng `open`; lỗi thì ghi mã vào `state.fontreq_error` (Settings đọc qua `auto_status`) và thử lại mỗi `retry`
/// trên một luồng nền cho tới khi mở được hoặc app thoát. Tách khỏi CoreText để test đường lỗi.
#[doc(hidden)]
pub fn start_with<F>(state: &Arc<AppState>, retry: std::time::Duration, open: F)
where
    F: Fn() -> Result<Box<dyn std::any::Any + Send>, (&'static str, String)> + Send + 'static,
{
    use std::sync::atomic::Ordering;
    if attempt(state, &open, retry) {
        return;
    }
    let weak = Arc::downgrade(state);
    let _ = std::thread::Builder::new().name("fontatlas-fontreq-retry".into()).spawn(move || loop {
        // ngủ từng nhịp ngắn để thoát app không phải chờ cả khoảng thử lại
        let until = std::time::Instant::now() + retry;
        while std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(50).min(retry));
            match weak.upgrade() {
                Some(s) if !s.fontreq_stop.load(Ordering::SeqCst) => {}
                _ => return,
            }
        }
        let Some(s) = weak.upgrade() else { return };
        if s.fontreq_stop.load(Ordering::SeqCst) || attempt(&s, &open, retry) {
            return;
        }
    });
}

fn attempt<F>(state: &AppState, open: &F, retry: std::time::Duration) -> bool
where
    F: Fn() -> Result<Box<dyn std::any::Any + Send>, (&'static str, String)>,
{
    let Ok(mut slot) = state.fontreq.lock() else { return true };
    if slot.is_some() {
        return true;
    }
    match open() {
        Ok(src) => {
            log::info!("nguồn font request đã mở");
            *slot = Some(src);
            if let Ok(mut e) = state.fontreq_error.lock() {
                *e = None;
            }
            true
        }
        Err((code, msg)) => {
            log::warn!("không mở được nguồn font request ({code}): {msg}; thử lại sau {retry:?}");
            if let Ok(mut e) = state.fontreq_error.lock() {
                *e = Some(code.to_string());
            }
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn start(_state: &Arc<AppState>) {}

/// Đóng nguồn (chờ luồng dừng, tối đa một nhịp runloop ~0,25 s).
pub fn stop(state: &AppState) {
    state.fontreq_stop.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut slot) = state.fontreq.lock() {
        slot.take();
    }
}
