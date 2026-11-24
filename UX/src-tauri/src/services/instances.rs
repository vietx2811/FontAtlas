//! Phát hiện process Font Atlas KHÁC đang chạy (chỉ đọc danh sách process; không bao giờ kill/đụng tới chúng).
//! Dùng để bỏ qua dọn đăng ký session mồ côi khi có hai instance cùng chạy (đăng ký session là của cả phiên đăng nhập).
//! So khớp theo tên file thực thi của chính mình: tên chung chung (vd. "app" ở bản debug) chỉ làm ta bỏ qua dọn, an toàn hơn là gỡ nhầm.
use std::path::{Path, PathBuf};

/// Thuần: có process nào khác (pid khác, cùng tên file thực thi) trong danh sách không. Process CON của chính mình (ppid == pid của ta, vd. probe
/// font-request chạy lại `current_exe`) không tính là instance khác.
pub fn other_in(own_pid: u32, own_exe: &Path, procs: impl IntoIterator<Item = (u32, u32, PathBuf)>) -> bool {
    let Some(name) = own_exe.file_name() else { return false };
    procs.into_iter().any(|(pid, ppid, p)| pid != own_pid && ppid != own_pid && p.file_name() == Some(name))
}

#[cfg(target_os = "macos")]
fn processes() -> Vec<(u32, u32, PathBuf)> {
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
                let mut info: libc::proc_bsdinfo = std::mem::zeroed();
                let sz = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
                let ppid = if libc::proc_pidinfo(pid, libc::PROC_PIDTBSDINFO, 0, (&raw mut info).cast(), sz) == sz { info.pbi_ppid } else { 0 };
                out.push((pid as u32, ppid, PathBuf::from(OsStr::from_bytes(&buf[..len as usize]))));
            }
        }
        out
    }
}

#[cfg(not(target_os = "macos"))]
fn processes() -> Vec<(u32, u32, PathBuf)> {
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
        assert!(other_in(10, &me, [(10, 1, me.clone()), (11, 1, p("/Volumes/X/Font Atlas.app/Contents/MacOS/Font Atlas"))]));
    }

    #[test]
    fn my_own_child_process_is_not_another_instance() {
        // probe font-request: current_exe chạy lại với PROBE_FLAG, ppid = pid của ta
        let me = p("/a/Font Atlas");
        assert!(!other_in(10, &me, [(10, 1, me.clone()), (50, 10, me.clone())]));
        assert!(other_in(10, &me, [(10, 1, me.clone()), (50, 10, me.clone()), (60, 1, me.clone())]));
    }

    #[test]
    fn only_myself_or_other_names_is_not() {
        let me = p("/a/Font Atlas");
        assert!(!other_in(10, &me, [(10, 1, me.clone()), (12, 1, p("/usr/bin/zsh")), (13, 1, p("/Applications/Adobe Illustrator 2027/Adobe Illustrator"))]));
        assert!(!other_in(10, &me, []));
    }

    #[test]
    fn real_process_list_includes_me_but_no_other_unless_another_runs() {
        // không khẳng định "không có instance khác" (máy QA có thể chạy nhiều bản); chỉ kiểm hàm không panic và thấy chính mình
        if cfg!(target_os = "macos") {
            let me = std::process::id();
            assert!(processes().iter().any(|(pid, _, _)| *pid == me));
        }
        let _ = another_running();
    }
}
