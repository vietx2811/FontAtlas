//! Đường dẫn: canonical, trạng thái file (thiếu / iCloud chưa tải), đoán chủ sở hữu font trùng tên.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::types::Owner;

/// Trạng thái file trước khi đăng ký. Không bao giờ đọc nội dung (tránh kích hoạt tải iCloud ngầm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileProbe {
    Ready,
    Missing,
    NotDownloaded,
}

/// Chuỗi đưa cho hệ điều hành. Trên Windows có dạng `\\?\C:\…`; lưu nguyên văn (Plan 7b quyết).
pub fn canonical_path(p: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(p)
}

/// Cờ `SF_DATALESS` của APFS: file chỉ còn placeholder, nội dung nằm trên iCloud.
#[cfg(target_os = "macos")]
const SF_DATALESS: u32 = 0x4000_0000;

/// Hai file có cùng nội dung không? Chỉ đọc khi CẢ HAI `Ready` (file iCloud chưa tải mà bị mở sẽ bị tải về ngầm).
/// So kích thước trước (rẻ) từ handle đã mở (không TOCTOU, symlink được theo), chỉ khi bằng nhau mới so từng khối.
/// Lỗi đọc / không `Ready` → false: coi là khác, để người dùng quyết định.
pub fn same_content(a: &Path, b: &Path) -> bool {
    same_content_by(a, b, probe_file)
}

/// Như `same_content` với bộ thăm dò tuỳ ý (để test file "dataless" không cần iCloud thật).
pub(crate) fn same_content_by(a: &Path, b: &Path, probe: impl Fn(&Path) -> FileProbe) -> bool {
    use std::io::Read;
    if probe(a) != FileProbe::Ready || probe(b) != FileProbe::Ready {
        return false;
    }
    let (Ok(mut fa), Ok(mut fb)) = (std::fs::File::open(a), std::fs::File::open(b)) else { return false };
    let (Ok(ma), Ok(mb)) = (fa.metadata(), fb.metadata()) else { return false };
    if ma.len() != mb.len() {
        return false;
    }
    let (mut x, mut y) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    let mut remaining = ma.len();
    while remaining > 0 {
        let n = remaining.min(x.len() as u64) as usize;
        if fa.read_exact(&mut x[..n]).is_err() || fb.read_exact(&mut y[..n]).is_err() || x[..n] != y[..n] {
            return false;
        }
        remaining -= n as u64;
    }
    true
}

pub fn probe_file(p: &Path) -> FileProbe {
    match std::fs::metadata(p) {
        Ok(_m) => {
            #[cfg(target_os = "macos")]
            {
                use std::os::macos::fs::MetadataExt;
                if _m.st_flags() & SF_DATALESS != 0 {
                    return FileProbe::NotDownloaded;
                }
            }
            FileProbe::Ready
        }
        Err(_) if icloud_placeholder(p).is_some_and(|ph| ph.exists()) => FileProbe::NotDownloaded,
        Err(_) => FileProbe::Missing,
    }
}

/// Chờ tối đa bao lâu để tải một font từ ổ cloud trước khi bật.
pub const CLOUD_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// Vì sao không đưa được file cloud về máy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureLocalError {
    /// Không còn file (ổ rút, đã xoá).
    Missing,
    /// Quá thời gian chờ: mạng chậm hoặc ổ cloud không phản hồi.
    Timeout,
    /// Hệ điều hành báo lỗi khi đọc (offline, đăng xuất, hết dung lượng…).
    Io(String),
    /// Đọc xong nhưng file vẫn là placeholder tới hạn chót.
    StillCloudOnly,
    /// Chỉ có stub `.x.ttf.icloud` (kiểu iCloud cũ), không có file thật để đọc.
    PlaceholderStub,
    /// Người dùng huỷ giữa chừng (chỉ khi gọi bản có cờ huỷ): không phải lỗi của ổ cloud.
    Cancelled,
}

impl EnsureLocalError {
    /// Mã ổn định cho `FileOutcome::Failed.code` (giao diện dịch theo mã).
    pub fn code(&self) -> &'static str {
        match self {
            EnsureLocalError::Missing => "missing_file",
            EnsureLocalError::Timeout => "cloud_timeout",
            EnsureLocalError::Io(_) | EnsureLocalError::StillCloudOnly => "cloud_download_failed",
            EnsureLocalError::PlaceholderStub => "cloud_stub",
            EnsureLocalError::Cancelled => "cancelled",
        }
    }

    pub fn message(&self) -> String {
        match self {
            EnsureLocalError::Missing => "không tìm thấy file".into(),
            EnsureLocalError::Timeout => format!("hết {} giây chờ tải font từ ổ cloud", CLOUD_DOWNLOAD_TIMEOUT.as_secs()),
            EnsureLocalError::Io(m) => format!("không tải được font từ ổ cloud: {m}"),
            EnsureLocalError::StillCloudOnly => "font vẫn chưa về máy sau khi tải".into(),
            EnsureLocalError::PlaceholderStub => "font mới có bản giữ chỗ iCloud, chưa có file để tải".into(),
            EnsureLocalError::Cancelled => "đã huỷ".into(),
        }
    }
}

/// Bảo đảm file đã nằm trên máy: nếu là placeholder (dataless) thì đọc 1 byte trên luồng riêng để hệ điều hành
/// tải về, chờ tối đa `timeout`, rồi thăm dò lại tới khi cờ dataless gỡ. Không bao giờ treo quá `timeout`
/// (luồng đọc bị kẹt được bỏ lại). File đã ở máy: trả ngay, không đọc gì.
pub fn ensure_local(p: &Path, timeout: Duration) -> Result<(), EnsureLocalError> {
    ensure_local_by(p, timeout, probe_file, read_first_byte)
}

/// Như `ensure_local`, nhưng `hold` (vd giấy phép của cổng tải) được chuyển vào luồng đọc và chỉ nhả khi lần đọc thật sự
/// kết thúc — kể cả khi người gọi đã hết hạn và bỏ đi — để số lần tải thật không vượt giới hạn của cổng.
pub fn ensure_local_holding<H: Send + 'static>(p: &Path, timeout: Duration, hold: H) -> Result<(), EnsureLocalError> {
    ensure_local_hold_by(p, timeout, probe_file, read_first_byte, hold, None)
}

/// Như `ensure_local_holding` nhưng dừng trong khoảng 100 ms sau khi `cancel` bật (trả `Cancelled`) thay vì chờ hết `timeout`.
/// Lần đọc đang chạy trên luồng riêng vẫn chạy nốt và giữ `hold` tới khi xong.
pub fn ensure_local_holding_cancellable<H: Send + 'static>(p: &Path, timeout: Duration, hold: H, cancel: &AtomicBool) -> Result<(), EnsureLocalError> {
    ensure_local_hold_by(p, timeout, probe_file, read_first_byte, hold, Some(cancel))
}

fn read_first_byte(p: &Path) -> std::io::Result<()> {
    use std::io::Read;
    let mut b = [0u8; 1];
    // chỉ cần hệ điều hành bắt đầu tải: số byte đọc được (0 với file rỗng) không quan trọng
    let _read = std::fs::File::open(p)?.read(&mut b)?;
    Ok(())
}

/// Như `ensure_local` với bộ thăm dò và hàm tải thay được (test dataless không cần ổ cloud thật).
pub(crate) fn ensure_local_by(
    p: &Path,
    timeout: Duration,
    probe: impl Fn(&Path) -> FileProbe,
    fetch: impl FnOnce(&Path) -> std::io::Result<()> + Send + 'static,
) -> Result<(), EnsureLocalError> {
    ensure_local_hold_by(p, timeout, probe, fetch, (), None)
}

pub(crate) fn ensure_local_hold_by<H: Send + 'static>(
    p: &Path,
    timeout: Duration,
    probe: impl Fn(&Path) -> FileProbe,
    fetch: impl FnOnce(&Path) -> std::io::Result<()> + Send + 'static,
    hold: H,
    cancel: Option<&AtomicBool>,
) -> Result<(), EnsureLocalError> {
    let cancelled = || cancel.is_some_and(|c| c.load(Ordering::SeqCst));
    match probe(p) {
        FileProbe::Ready => return Ok(()),
        FileProbe::Missing => return Err(EnsureLocalError::Missing),
        FileProbe::NotDownloaded => {}
    }
    let deadline = Instant::now() + timeout;
    let (tx, rx) = mpsc::channel();
    let path = p.to_path_buf();
    std::thread::Builder::new()
        .name("cloud-fetch".into())
        .spawn(move || {
            let _hold = hold; // nhả sau khi đọc xong, không phải khi người gọi bỏ cuộc
            let _ = tx.send(fetch(&path));
        })
        .map_err(|e| EnsureLocalError::Io(e.to_string()))?;
    // chờ theo lát 100 ms để bắt cờ huỷ; hết `deadline` vẫn là Timeout như trước
    let received = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left.min(Duration::from_millis(100))) {
            Err(mpsc::RecvTimeoutError::Timeout) if cancelled() => return Err(EnsureLocalError::Cancelled),
            Err(mpsc::RecvTimeoutError::Timeout) if left > Duration::from_millis(100) => continue,
            other => break other,
        }
    };
    match received {
        Ok(Ok(())) => {}
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound && !p.exists() => return Err(EnsureLocalError::PlaceholderStub),
        Ok(Err(e)) => return Err(EnsureLocalError::Io(e.to_string())),
        Err(mpsc::RecvTimeoutError::Timeout) => return Err(EnsureLocalError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => return Err(EnsureLocalError::Io("luồng tải bị dừng bất ngờ".into())),
    }
    // đọc xong có thể trả về trước khi hệ điều hành gỡ cờ dataless: thăm dò lại tới hạn chót
    loop {
        match probe(p) {
            FileProbe::Ready => return Ok(()),
            FileProbe::Missing => return Err(EnsureLocalError::Missing),
            FileProbe::NotDownloaded => {}
        }
        if cancelled() {
            return Err(EnsureLocalError::Cancelled);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(EnsureLocalError::StillCloudOnly);
        }
        std::thread::sleep(left.min(Duration::from_millis(100)));
    }
}

/// `dir/x.ttf` → `dir/.x.ttf.icloud` (kiểu placeholder cũ của iCloud Drive).
fn icloud_placeholder(p: &Path) -> Option<PathBuf> {
    let name = p.file_name()?.to_str()?;
    Some(p.with_file_name(format!(".{name}.icloud")))
}

/// Canonical của `p`; nếu `p` chưa tồn tại thì canonical phần tổ tiên còn tồn tại rồi nối phần còn lại.
fn canonical_lossy(p: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(p) {
        return c;
    }
    let mut rest: Vec<std::ffi::OsString> = vec![];
    let mut cur = p.to_path_buf();
    while let Some(name) = cur.file_name().map(|n| n.to_os_string()) {
        rest.push(name);
        cur.pop();
        if let Ok(mut c) = std::fs::canonicalize(&cur) {
            c.extend(rest.iter().rev());
            return c;
        }
    }
    p.to_path_buf()
}

/// Các thành phần đường dẫn đã canonical hoá, viết thường (APFS không phân biệt hoa/thường), bỏ gốc `/`,
/// bỏ tiền tố firmlink `/System/Volumes/Data` và `/Volumes/<tên ổ>`.
fn norm_components(path: &Path) -> Vec<String> {
    let mut c: Vec<String> = canonical_lossy(path)
        .components()
        .filter(|c| !matches!(c, std::path::Component::RootDir))
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    if c.starts_with(&["system".into(), "volumes".into(), "data".into()]) {
        c.drain(..3);
    }
    if c.len() > 2 && c[0] == "volumes" {
        c.drain(..2);
    }
    c
}

/// Đường dẫn nằm trong các thư mục font mà hệ điều hành bật theo thư mục: /System/Library, /Library/Fonts,
/// /Network/Library/Fonts và ~/Library/Fonts (của mọi người dùng, kể cả khi đi qua symlink, firmlink hay khác hoa/thường).
pub fn in_protected_font_dir(path: &Path, home: &Path) -> bool {
    let c = norm_components(path);
    let s = |v: &[&str]| c.len() >= v.len() && c.iter().zip(v).all(|(a, b)| a == b);
    if s(&["system", "library"]) || s(&["library", "fonts"]) || s(&["network", "library", "fonts"]) {
        return true;
    }
    if c.len() >= 4 && c[0] == "users" && c[2] == "library" && c[3] == "fonts" {
        return true;
    }
    let mut h = norm_components(home);
    if h.is_empty() {
        return false;
    }
    h.extend(["library".to_string(), "fonts".to_string()]);
    c.starts_with(&h)
}

/// Ma trận "Thay thế" theo chủ sở hữu + đường dẫn (xem docs/team/research/2026-11-11-replace-activation.md §3).
/// Đường dẫn trong 3 thư mục font LUÔN bị bảo vệ, bất kể phân loại: hệ điều hành bật chúng theo thư mục (không gỡ
/// được qua API) và cách duy nhất là xoá/di chuyển file — Font Atlas không bao giờ làm. So sánh trên đường dẫn đã
/// canonical hoá (symlink, firmlink, hoa/thường) VÀ trên đường dẫn thô; cái nào chặt hơn thắng.
pub fn replace_tier(owner: Owner, path: &Path, home: &Path) -> crate::types::ReplaceTier {
    use crate::types::ReplaceTier::*;
    if in_protected_font_dir(path, home) || matches!(owner, Owner::System | Owner::UserInstalled) {
        return Protected;
    }
    match owner {
        Owner::AdobeFonts | Owner::Suitcase => Guide,
        _ => Replace,
    }
}

/// Đoán ai đang cung cấp font ở `path`. Khớp đầu tiên thắng; UI ghi "có thể là …".
pub fn classify_owner(path: &Path, home: &Path, ours: &HashSet<PathBuf>) -> Owner {
    if ours.contains(path) || ours.contains(&canonical_lossy(path)) {
        return Owner::FontAtlas;
    }
    if in_protected_font_dir(path, home) {
        let c = norm_components(path);
        let is_user = (c.len() >= 4 && c[0] == "users" && c[2] == "library" && c[3] == "fonts")
            || { let mut h = norm_components(home); h.extend(["library".to_string(), "fonts".to_string()]); c.starts_with(&h) };
        return if is_user { Owner::UserInstalled } else { Owner::System };
    }
    let comps: Vec<String> = path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    if comps.windows(4).any(|w| w[0] == "Adobe" && w[1] == "CoreSync" && w[2] == "plugins" && w[3] == "livetype") {
        return Owner::AdobeFonts;
    }
    if comps.iter().any(|c| c.ends_with(".rightfontlibrary")) {
        return Owner::RightFont;
    }
    if comps.iter().any(|c| c.eq_ignore_ascii_case("FontBase")) {
        return Owner::FontBase;
    }
    if comps.iter().any(|c| c.eq_ignore_ascii_case("Typeface")) {
        return Owner::Typeface;
    }
    if comps.iter().any(|c| c.eq_ignore_ascii_case("Extensis") || c.to_lowercase().contains("suitcase")) {
        return Owner::Suitcase;
    }
    Owner::Other
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    /// Bộ thăm dò giả: trả `NotDownloaded` cho tới khi cờ `local` bật.
    fn probe_flag(local: &Arc<AtomicBool>) -> impl Fn(&Path) -> FileProbe + use<> {
        let local = Arc::clone(local);
        move |_| if local.load(Ordering::SeqCst) { FileProbe::Ready } else { FileProbe::NotDownloaded }
    }

    #[test]
    fn ensure_local_ready_file_never_spawns_a_fetch() {
        let fetched = Arc::new(AtomicUsize::new(0));
        let f = Arc::clone(&fetched);
        let r = ensure_local_by(Path::new("/x.ttf"), Duration::from_secs(1), |_| FileProbe::Ready, move |_| {
            f.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert_eq!(r, Ok(()));
        assert_eq!(fetched.load(Ordering::SeqCst), 0, "file đã ở máy: không đọc gì thêm");
    }

    #[test]
    fn ensure_local_missing_is_reported() {
        let r = ensure_local_by(Path::new("/x.ttf"), Duration::from_secs(1), |_| FileProbe::Missing, |_| Ok(()));
        assert_eq!(r, Err(EnsureLocalError::Missing));
    }

    #[test]
    fn ensure_local_downloads_then_succeeds() {
        let local = Arc::new(AtomicBool::new(false));
        let l = Arc::clone(&local);
        let r = ensure_local_by(Path::new("/x.ttf"), Duration::from_secs(5), probe_flag(&local), move |_| {
            std::thread::sleep(Duration::from_millis(50)); // "đang tải"
            l.store(true, Ordering::SeqCst);
            Ok(())
        });
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn ensure_local_times_out_instead_of_hanging() {
        let local = Arc::new(AtomicBool::new(false));
        let t = Instant::now();
        let r = ensure_local_by(Path::new("/x.ttf"), Duration::from_millis(150), probe_flag(&local), |_| {
            std::thread::sleep(Duration::from_secs(5)); // ổ cloud không phản hồi
            Ok(())
        });
        assert_eq!(r, Err(EnsureLocalError::Timeout));
        assert!(t.elapsed() < Duration::from_secs(2), "phải trả về đúng hạn, không đợi fetch: {:?}", t.elapsed());
    }

    #[test]
    fn ensure_local_fetch_error_is_download_failed() {
        let local = Arc::new(AtomicBool::new(false));
        let r = ensure_local_by(Path::new("/x.ttf"), Duration::from_secs(1), probe_flag(&local), |_| {
            Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "mạng đứt"))
        });
        match r {
            Err(EnsureLocalError::Io(m)) => assert!(m.contains("mạng đứt"), "{m}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ensure_local_read_ok_but_still_placeholder_gives_up_at_deadline() {
        let local = Arc::new(AtomicBool::new(false));
        let t = Instant::now();
        let r = ensure_local_by(Path::new("/x.ttf"), Duration::from_millis(400), probe_flag(&local), |_| Ok(()));
        assert_eq!(r, Err(EnsureLocalError::StillCloudOnly));
        assert!(t.elapsed() >= Duration::from_millis(300) && t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
    }

    #[test]
    fn ensure_local_waits_for_late_materialisation_after_read() {
        // read trả về trước khi cờ dataless được gỡ: phải thăm dò lại tới hạn chót
        let local = Arc::new(AtomicBool::new(false));
        let l = Arc::clone(&local);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            l.store(true, Ordering::SeqCst);
        });
        let r = ensure_local_by(Path::new("/x.ttf"), Duration::from_secs(5), probe_flag(&local), |_| Ok(()));
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn ensure_local_icloud_stub_without_real_file_is_stub_error() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".a.ttf.icloud"), b"").unwrap();
        let r = ensure_local(&d.path().join("a.ttf"), Duration::from_secs(1));
        assert_eq!(r, Err(EnsureLocalError::PlaceholderStub));
    }

    #[test]
    fn ensure_local_real_ready_file_is_ok() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("a.ttf");
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(ensure_local(&f, Duration::from_secs(1)), Ok(()));
    }

    /// Giữ chỗ (permit) trong luồng tải: chỉ nhả khi lần đọc thật sự kết thúc, kể cả khi người gọi đã hết hạn.
    struct DropFlag(Arc<AtomicBool>);
    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn hold_is_kept_until_the_stuck_read_really_finishes_even_after_a_timeout() {
        let dropped = Arc::new(AtomicBool::new(false));
        let local = Arc::new(AtomicBool::new(false));
        let r = ensure_local_hold_by(
            Path::new("/x.ttf"),
            Duration::from_millis(60),
            probe_flag(&local),
            |_| {
                std::thread::sleep(Duration::from_millis(400)); // đọc kẹt
                Ok(())
            },
            DropFlag(Arc::clone(&dropped)),
            None,
        );
        assert_eq!(r, Err(EnsureLocalError::Timeout));
        assert!(!dropped.load(Ordering::SeqCst), "người gọi đã bỏ cuộc nhưng lần tải thật vẫn chạy → chưa được nhả chỗ");
        std::thread::sleep(Duration::from_millis(700));
        assert!(dropped.load(Ordering::SeqCst), "đọc xong thì nhả");
    }

    #[test]
    fn hold_is_released_when_the_file_is_already_local_and_when_it_is_missing() {
        for probe in [FileProbe::Ready, FileProbe::Missing] {
            let dropped = Arc::new(AtomicBool::new(false));
            let _ = ensure_local_hold_by(Path::new("/x.ttf"), Duration::from_secs(1), move |_| probe, |_| Ok(()), DropFlag(Arc::clone(&dropped)), None);
            assert!(dropped.load(Ordering::SeqCst), "{probe:?}: không có lần tải nào → nhả ngay");
        }
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(EnsureLocalError::Timeout.code(), "cloud_timeout");
        assert_eq!(EnsureLocalError::Io("x".into()).code(), "cloud_download_failed");
        assert_eq!(EnsureLocalError::StillCloudOnly.code(), "cloud_download_failed");
        assert_eq!(EnsureLocalError::Missing.code(), "missing_file");
        assert_eq!(EnsureLocalError::PlaceholderStub.code(), "cloud_stub");
        assert_eq!(EnsureLocalError::Cancelled.code(), "cancelled");
    }

    #[test]
    fn cancel_stops_a_long_wait_within_a_fraction_of_a_second_and_the_read_keeps_its_hold() {
        let dropped = Arc::new(AtomicBool::new(false));
        let local = Arc::new(AtomicBool::new(false));
        let cancel = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&cancel);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            c.store(true, Ordering::SeqCst);
        });
        let t = Instant::now();
        let r = ensure_local_hold_by(
            Path::new("/x.ttf"),
            Duration::from_secs(120), // hạn chót rất dài: huỷ phải cắt ngang
            probe_flag(&local),
            |_| {
                std::thread::sleep(Duration::from_millis(600)); // đọc kẹt
                Ok(())
            },
            DropFlag(Arc::clone(&dropped)),
            Some(&cancel),
        );
        assert_eq!(r, Err(EnsureLocalError::Cancelled));
        assert!(t.elapsed() < Duration::from_secs(2), "huỷ dừng nhanh: {:?}", t.elapsed());
        assert!(!dropped.load(Ordering::SeqCst), "lần đọc thật còn chạy → còn giữ chỗ");
        std::thread::sleep(Duration::from_millis(900));
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn cancel_also_cuts_the_wait_for_the_dataless_flag_to_clear() {
        let local = Arc::new(AtomicBool::new(false)); // không bao giờ về máy
        let cancel = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&cancel);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            c.store(true, Ordering::SeqCst);
        });
        let t = Instant::now();
        let r = ensure_local_hold_by(Path::new("/x.ttf"), Duration::from_secs(60), probe_flag(&local), |_| Ok(()), (), Some(&cancel));
        assert_eq!(r, Err(EnsureLocalError::Cancelled));
        assert!(t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
    }

    #[test]
    fn classify_owner_table() {
        let home = Path::new("/Users/u");
        let ours: HashSet<PathBuf> = [PathBuf::from("/lib/mine.ttf")].into();
        let cases = [
            ("/lib/mine.ttf", Owner::FontAtlas),
            ("/System/Library/Fonts/Helvetica.ttc", Owner::System),
            ("/Library/Fonts/Arial.ttf", Owner::System),
            ("/Network/Library/Fonts/Shared.otf", Owner::System),
            ("/Users/u/Library/Fonts/Inter.ttf", Owner::UserInstalled),
            ("/Users/u/Library/Application Support/Adobe/CoreSync/plugins/livetype/.r/.123.otf", Owner::AdobeFonts),
            (
                "/Users/u/Library/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/A/x.ttf",
                Owner::RightFont,
            ),
            ("/Users/u/fontbase/Sans/x.ttf", Owner::FontBase),
            ("/Volumes/Ext/fonts/x.ttf", Owner::Other),
            ("/Users/u/Library/FontsOld/x.ttf", Owner::Other),
        ];
        for (p, want) in cases {
            assert_eq!(classify_owner(Path::new(p), home, &ours), want, "{p}");
        }
    }

    #[test]
    fn probe_missing_and_ready() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("a.ttf");
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(probe_file(&f), FileProbe::Ready);
        assert_eq!(probe_file(&d.path().join("khong-co.ttf")), FileProbe::Missing);
    }

    #[test]
    fn probe_icloud_placeholder() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".x.ttf.icloud"), b"").unwrap();
        assert_eq!(probe_file(&d.path().join("x.ttf")), FileProbe::NotDownloaded);
    }

    #[cfg(unix)]
    #[test]
    fn canonical_resolves_symlink() {
        let d = tempfile::tempdir().unwrap();
        let target = d.path().join("real.ttf");
        std::fs::write(&target, b"x").unwrap();
        let link = d.path().join("link.ttf");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(canonical_path(&link).unwrap(), std::fs::canonicalize(&target).unwrap());
    }

    fn two(bytes: &[u8]) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("a.ttf"), d.path().join("b.ttf"));
        std::fs::write(&a, bytes).unwrap();
        std::fs::write(&b, bytes).unwrap();
        (d, a, b)
    }

    #[test]
    fn same_content_dataless_provider_is_never_opened() {
        // Cùng nội dung nhưng bộ thăm dò báo "chưa tải" → false, và không mở file (không kích hoạt tải iCloud).
        let (_d, a, b) = two(b"font");
        assert!(same_content(&a, &b));
        let bp = b.clone();
        assert!(!same_content_by(&a, &b, |p| if p == bp { FileProbe::NotDownloaded } else { FileProbe::Ready }));
        let ap = a.clone();
        assert!(!same_content_by(&a, &b, |p| if p == ap { FileProbe::NotDownloaded } else { FileProbe::Ready }));
    }

    #[test]
    fn same_content_icloud_placeholder_is_false() {
        let (d, a, _b) = two(b"font");
        let missing = d.path().join("c.ttf");
        std::fs::write(d.path().join(".c.ttf.icloud"), b"").unwrap();
        assert!(!same_content(&a, &missing));
        assert!(!same_content(&a, &d.path().join("khong-co.ttf")));
    }

    #[test]
    fn same_content_large_file_compares_every_block() {
        let mut big = vec![7u8; 3 * 1024 * 1024 + 5];
        let (_d, a, b) = two(&big);
        assert!(same_content(&a, &b));
        let last = big.len() - 1;
        big[last] = 8; // khác đúng byte cuối, nằm ở khối cuối (không chia hết 64 KiB)
        std::fs::write(&b, &big).unwrap();
        assert!(!same_content(&a, &b));
        big[last] = 7;
        big[100_000] = 9; // khác ở giữa
        std::fs::write(&b, &big).unwrap();
        assert!(!same_content(&a, &b));
        std::fs::write(&b, &big[..big.len() - 1]).unwrap(); // khác kích thước
        assert!(!same_content(&a, &b));
    }

    #[cfg(unix)]
    #[test]
    fn same_content_follows_symlinks_and_dangling_is_false() {
        let (d, a, b) = two(b"font");
        let link = d.path().join("link.ttf");
        std::os::unix::fs::symlink(&b, &link).unwrap();
        assert!(same_content(&a, &link));
        std::fs::remove_file(&b).unwrap();
        assert!(!same_content(&a, &link), "symlink treo");
    }
}
