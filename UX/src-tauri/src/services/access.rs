//! Kiểm tra quyền đọc thư mục nguồn có giới hạn thời gian.
//!
//! macOS có thể treo ở `opendir` (không trả lỗi) khi đang chờ người dùng trả lời hộp thoại quyền truy cập ổ ngoài,
//! thư mục Desktop/Documents/Downloads hay thư mục đám mây. Một luồng quét treo như vậy giữ hàng đợi vô hạn.
//! Ta thử mở thư mục gốc ở một luồng riêng; quá hạn hoặc bị từ chối thì báo `Failed` kèm lý do để giao diện
//! hiện "cần quyền truy cập" (nút Cấp quyền, Thử lại). Luồng thử bị treo không thể huỷ: nó tự kết thúc khi hệ điều hành trả lời.
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::json;

use crate::error::CmdError;

#[derive(Debug, PartialEq, Eq)]
pub enum Access {
    Ok,
    Denied(String),
    TimedOut,
}

/// Thời gian chờ mặc định; `FONTATLAS_ACCESS_TIMEOUT_MS` ghi đè (test, QA).
pub fn timeout() -> Duration {
    let ms = std::env::var("FONTATLAS_ACCESS_TIMEOUT_MS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(8_000);
    Duration::from_millis(ms)
}

pub fn probe_dir(path: &Path, timeout: Duration) -> Access {
    probe_with(path, timeout, |p| std::fs::read_dir(p).map(|mut it| {
        let _ = it.next();
    }))
}

/// Chạy `op` (mở thư mục) ở luồng riêng và chờ tối đa `timeout`.
/// Lỗi khác "từ chối quyền" (không tồn tại, ổ rút…) không phải vấn đề quyền: để luồng quét xử lý như cũ.
pub fn probe_with<F>(path: &Path, timeout: Duration, op: F) -> Access
where
    F: FnOnce(&Path) -> io::Result<()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let p = path.to_path_buf();
    let spawned = std::thread::Builder::new().name("fontatlas-access-probe".into()).spawn(move || {
        let _ = tx.send(op(&p).map_err(|e| (e.kind(), e.to_string())));
    });
    if spawned.is_err() {
        return Access::Ok;
    }
    match rx.recv_timeout(timeout) {
        Ok(Ok(())) => Access::Ok,
        Ok(Err((io::ErrorKind::PermissionDenied, msg))) => Access::Denied(msg),
        Ok(Err(_)) => Access::Ok,
        Err(mpsc::RecvTimeoutError::Timeout) => Access::TimedOut,
        Err(mpsc::RecvTimeoutError::Disconnected) => Access::Ok,
    }
}

/// Lỗi gửi cho giao diện: mã `io` (hai nguồn `io` đã có trong 6a) kèm `detail {source:"app", op:"opendir", reason, path}`.
pub fn to_error(root: &Path, a: &Access) -> Option<CmdError> {
    let (reason, text) = match a {
        Access::Ok => return None,
        Access::Denied(m) => ("permission_denied", format!("macOS từ chối đọc thư mục: {m}")),
        Access::TimedOut => ("timeout", "không mở được thư mục sau vài giây (có thể đang chờ cấp quyền truy cập)".to_string()),
    };
    Some(CmdError::new("io", text).with_detail(json!({ "source": "app", "op": "opendir", "reason": reason, "path": root.to_string_lossy() })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("fa-access-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn readable_dir_is_ok() {
        let d = tmp("ok");
        assert_eq!(probe_dir(&d, Duration::from_secs(2)), Access::Ok);
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn missing_dir_is_not_an_access_problem() {
        assert_eq!(probe_dir(Path::new("/nonexistent/fontatlas/xyz"), Duration::from_secs(2)), Access::Ok);
    }

    #[test]
    fn permission_denied_is_reported() {
        let a = probe_with(Path::new("/x"), Duration::from_secs(1), |_| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
        assert!(matches!(a, Access::Denied(_)));
    }

    #[test]
    fn a_blocked_open_times_out_instead_of_hanging() {
        let t = Instant::now();
        let a = probe_with(Path::new("/x"), Duration::from_millis(60), |_| {
            std::thread::sleep(Duration::from_millis(1500));
            Ok(())
        });
        assert_eq!(a, Access::TimedOut);
        assert!(t.elapsed() < Duration::from_millis(900), "phải trả về sớm, không đợi luồng treo: {:?}", t.elapsed());
    }

    #[cfg(unix)]
    #[test]
    fn a_dir_without_read_permission_is_denied() {
        use std::os::unix::fs::PermissionsExt;
        let d = tmp("noperm");
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o000)).unwrap();
        let root = std::fs::read_dir(&d).is_ok(); // chạy bằng root thì bỏ qua
        let a = probe_dir(&d, Duration::from_secs(2));
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(&d).ok();
        if !root {
            assert!(matches!(a, Access::Denied(_)), "{a:?}");
        }
    }

    #[test]
    fn errors_carry_a_machine_readable_reason_for_the_ui() {
        assert!(to_error(Path::new("/V"), &Access::Ok).is_none());
        let e = to_error(Path::new("/Volumes/X"), &Access::TimedOut).unwrap();
        assert_eq!(e.code, "io");
        let d = e.detail.unwrap();
        assert_eq!(d["source"], "app");
        assert_eq!(d["op"], "opendir");
        assert_eq!(d["reason"], "timeout");
        assert_eq!(d["path"], "/Volumes/X");
        let e = to_error(Path::new("/p"), &Access::Denied("Operation not permitted".into())).unwrap();
        assert_eq!(e.detail.unwrap()["reason"], "permission_denied");
    }
}
