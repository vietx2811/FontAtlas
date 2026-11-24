//! Phát hiện process Font Atlas KHÁC đang chạy (chỉ đọc danh sách process; không bao giờ kill/đụng tới chúng).
//! Dùng để bỏ qua dọn đăng ký session mồ côi khi có hai instance cùng chạy (đăng ký session là của cả phiên đăng nhập).
//! So khớp theo tên file thực thi của chính mình: tên chung chung (vd. "app" ở bản debug) chỉ làm ta bỏ qua dọn, an toàn hơn là gỡ nhầm.
use std::path::{Path, PathBuf};

/// Thuần: có process nào khác (pid khác, cùng tên file thực thi) trong danh sách không.
pub fn other_in(own_pid: u32, own_exe: &Path, procs: impl IntoIterator<Item = (u32, PathBuf)>) -> bool {
    let Some(name) = own_exe.file_name() else { return false };
    procs.into_iter().any(|(pid, p)| pid != own_pid && p.file_name() == Some(name))
}

#[cfg(target_os = "macos")]
fn processes() -> Vec<(u32, PathBuf)> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    // SAFETY: gọi libproc với bộ đệm do ta cấp đúng kích thước; kết quả chỉ đọc.
    unsafe {
        let n = libc::proc_listallpids(std::ptr::null_mut(), 0);
        if n <= 0 {
            return vec![];
        }
        let mut pids = vec![0 as libc::pid_t; n as usize + 64];
        let got = libc::proc_listallpids(pids.as_mut_ptr().cast(), (pids.len() * std::mem::size_of::<libc::pid_t>()) as libc::c_int);
        if got <= 0 {
            return vec![];
        }
        let mut out = vec![];
        for &pid in pids.iter().take(got as usize).filter(|p| **p > 0) {
            let mut buf = vec![0u8; 4096];
            let len = libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32);
            if len > 0 {
                out.push((pid as u32, PathBuf::from(OsStr::from_bytes(&buf[..len as usize]))));
            }
        }
        out
    }
}

#[cfg(not(target_os = "macos"))]
fn processes() -> Vec<(u32, PathBuf)> {
    vec![]
}

/// Có Font Atlas khác đang chạy không. Không đọc được (không lấy được đường file của mình) → coi là không.
pub fn another_running() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    other_in(std::process::id(), &exe, processes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn same_name_other_pid_is_another_instance() {
        let me = p("/Applications/Font Atlas.app/Contents/MacOS/Font Atlas");
        assert!(other_in(10, &me, [(10, me.clone()), (11, p("/Volumes/X/Font Atlas.app/Contents/MacOS/Font Atlas"))]));
    }

    #[test]
    fn only_myself_or_other_names_is_not() {
        let me = p("/a/Font Atlas");
        assert!(!other_in(10, &me, [(10, me.clone()), (12, p("/usr/bin/zsh")), (13, p("/Applications/Adobe Illustrator 2027/Adobe Illustrator"))]));
        assert!(!other_in(10, &me, []));
    }

    #[test]
    fn real_process_list_includes_me_but_no_other_unless_another_runs() {
        // không khẳng định "không có instance khác" (máy QA có thể chạy nhiều bản); chỉ kiểm hàm không panic và thấy chính mình
        if cfg!(target_os = "macos") {
            let me = std::process::id();
            assert!(processes().iter().any(|(pid, _)| *pid == me));
        }
        let _ = another_running();
    }
}
