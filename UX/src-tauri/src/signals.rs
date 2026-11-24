//! SIGTERM / SIGINT → thoát gọn.
//!
//! Trình xử lý tín hiệu CHỈ đặt một cờ nguyên tử (async-signal-safe), không làm việc nặng. Một luồng theo dõi đọc cờ rồi gọi
//! `AppHandle::exit(0)`; vòng lặp chính thoát bình thường và `RunEvent::Exit` chạy `lifecycle::on_exit` (gỡ font session,
//! giữ persistent). Tín hiệu thứ hai trong lúc đang thoát trả về hành vi mặc định (dừng ngay): thoát gọn kẹt thì người dùng vẫn dừng được app.
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static STOP: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn on_signal(sig: libc::c_int) {
    if STOP.swap(true, Ordering::SeqCst) {
        // Lần thứ hai: gỡ trình xử lý để tín hiệu kế tiếp dừng tiến trình (`signal` là async-signal-safe).
        unsafe { libc::signal(sig, libc::SIG_DFL) };
    }
}

/// Đã nhận SIGTERM/SIGINT chưa.
pub fn requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// Cài trình xử lý cho SIGTERM và SIGINT.
pub fn install() {
    #[cfg(unix)]
    for sig in [libc::SIGTERM, libc::SIGINT] {
        unsafe { libc::signal(sig, on_signal as *const () as libc::sighandler_t) };
    }
}

/// Luồng nền: thấy cờ thì xin app thoát (chạy `RunEvent::Exit`). Thăm dò 100 ms, không tốn đáng kể.
pub fn watch<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    let _ = std::thread::Builder::new().name("fa-signal-watch".into()).spawn(move || {
        while !requested() {
            std::thread::sleep(Duration::from_millis(100));
        }
        app.exit(0);
    });
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn sigterm_only_sets_the_flag_and_the_process_survives() {
        install();
        assert!(!requested());
        unsafe { libc::raise(libc::SIGTERM) };
        // `raise` gọi trình xử lý đồng bộ trong luồng này.
        assert!(requested());
        STOP.store(false, Ordering::SeqCst);
        unsafe { libc::raise(libc::SIGINT) };
        assert!(requested());
        STOP.store(false, Ordering::SeqCst);
        for sig in [libc::SIGTERM, libc::SIGINT] {
            unsafe { libc::signal(sig, libc::SIG_DFL) };
        }
    }
}
