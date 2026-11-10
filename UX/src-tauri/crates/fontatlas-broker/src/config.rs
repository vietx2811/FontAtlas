//! `broker.json`: nơi client (helper, panel, script) đọc cổng + token. Quyền 0600, ghi nguyên tử.
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const BROKER_FILE: &str = "broker.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerFile {
    pub v: u32,
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

/// 32 byte ngẫu nhiên của hệ điều hành, dạng hex (64 ký tự).
pub fn new_token() -> std::io::Result<String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| std::io::Error::other(format!("không lấy được số ngẫu nhiên: {e}")))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

impl BrokerFile {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join(BROKER_FILE)
    }

    /// Ghi vào file tạm cùng thư mục (0600 ngay lúc tạo) rồi đổi tên: không ai đọc được bản dở dang.
    pub fn write(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let tmp = dir.join(format!(".{BROKER_FILE}.{}.tmp", self.pid));
        let json = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        let mut f = open_private(&tmp)?;
        let res = f.write_all(&json).and_then(|_| f.sync_all());
        drop(f);
        match res.and_then(|_| std::fs::rename(&tmp, Self::path(dir))) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                Err(e)
            }
        }
    }

    pub fn read(dir: &Path) -> std::io::Result<BrokerFile> {
        let bytes = std::fs::read(Self::path(dir))?;
        serde_json::from_slice(&bytes).map_err(std::io::Error::other)
    }

    /// Xoá file nếu còn là của tiến trình `pid` (không xoá file của bản app khác vừa khởi động).
    pub fn remove_if_mine(dir: &Path, pid: u32) {
        if Self::read(dir).is_ok_and(|b| b.pid == pid) {
            let _ = std::fs::remove_file(Self::path(dir));
        }
    }
}

#[cfg(unix)]
fn open_private(p: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let _ = std::fs::remove_file(p);
    std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(p)
}

#[cfg(not(unix))]
fn open_private(p: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(p)
}
