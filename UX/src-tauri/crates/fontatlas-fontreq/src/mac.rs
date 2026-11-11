//! Nguồn font request trên macOS. Chạy trên MỘT luồng riêng có runloop riêng (callback có thể chờ tải file placeholder
//! tới `AUTO_DOWNLOAD_TIMEOUT`, không được chặn luồng giao diện).
#![allow(deprecated)]
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2_core_foundation::{CFArray, CFDictionary, CFRetained, CFRunLoop, CFString, CFURL, kCFRunLoopDefaultMode};
use objc2_core_text::{
    CTFontManagerCreateFontRequestRunLoopSource, kCTFontFamilyNameAttribute, kCTFontNameAttribute, kCTFontStyleNameAttribute,
};

use crate::{Answer, AppNamer, Request, Responder};

#[derive(Debug)]
pub enum StartError {
    /// Hệ thống không tạo được nguồn (macOS quá cũ hoặc bị từ chối).
    NoSource,
    Thread(std::io::Error),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::NoSource => write!(f, "hệ thống không tạo được nguồn font request"),
            StartError::Thread(e) => write!(f, "không mở được luồng font request: {e}"),
        }
    }
}
impl std::error::Error for StartError {}

/// Gọi sau MỖI request (để ghi log, đo thời gian chờ của app hỏi). Chạy trên luồng của nguồn: phải nhanh.
pub type Observer = dyn Fn(i32, &Request, &Answer, Duration) + Send + Sync;

pub struct FontRequestSource {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    done: mpsc::Receiver<()>,
    responder: Arc<Responder>,
}

/// Chờ luồng nguồn dừng tối đa ngần này khi đóng; quá hạn thì bỏ rơi (detach). Một callback đang chạy không bao giờ giữ app thoát.
const CLOSE_WAIT: Duration = Duration::from_secs(1);

impl FontRequestSource {
    /// Đăng ký nguồn. Trả `Err` nếu hệ thống từ chối; khi đó app vẫn chạy, chỉ mất tự bật font cho Photoshop/AE.
    pub fn start(responder: Arc<Responder>, observer: Option<Arc<Observer>>) -> Result<Self, StartError> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let (tx, rx) = mpsc::channel::<bool>();
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let r2 = responder.clone();
        let thread = std::thread::Builder::new()
            .name("fontatlas-fontreq".into())
            .spawn(move || {
                run(r2, observer, stop2, tx);
                let _ = done_tx.send(());
            })
            .map_err(StartError::Thread)?;
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(true) => Ok(Self { stop, thread: Some(thread), done: done_rx, responder }),
            _ => {
                stop.store(true, Ordering::SeqCst);
                let _ = thread.join();
                Err(StartError::NoSource)
            }
        }
    }
}

impl Drop for FontRequestSource {
    fn drop(&mut self) {
        // đóng responder trước: callback đang chờ việc nền sẽ trả rỗng trong ≤ 50 ms thay vì chờ hết hạn
        self.responder.close();
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            match self.done.recv_timeout(CLOSE_WAIT) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = t.join();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => drop(t), // detach: luồng tự thoát khi callback xong
            }
        }
    }
}

fn run(responder: Arc<Responder>, observer: Option<Arc<Observer>>, stop: Arc<AtomicBool>, ready: mpsc::Sender<bool>) {
    let block = RcBlock::new(move |attrs: NonNull<CFDictionary>, pid: libc::pid_t| -> NonNull<CFArray> {
        let t0 = Instant::now();
        // Mọi panic đều thành "không có font": không để lỗi của ta làm sập app đang chạy.
        let (req, answer) = catch_unwind(AssertUnwindSafe(|| {
            let req = read_request(unsafe { attrs.as_ref() });
            let answer = responder.answer(pid, &req);
            (req, answer)
        }))
        .unwrap_or_else(|_| (Request::default(), Answer::Nothing));
        if let Some(o) = &observer {
            let _ = catch_unwind(AssertUnwindSafe(|| o(pid, &req, &answer, t0.elapsed())));
        }
        to_cf_array(&answer)
    });
    let Some(src) = (unsafe { CTFontManagerCreateFontRequestRunLoopSource(0, &block) }) else {
        let _ = ready.send(false);
        return;
    };
    let Some(rl) = CFRunLoop::current() else {
        let _ = ready.send(false);
        return;
    };
    rl.add_source(Some(&src), unsafe { kCFRunLoopDefaultMode });
    let _ = ready.send(true);
    // Mỗi vòng chạy ≤ 0,25 s khi rỗi (kiểm cờ dừng); một callback đang chạy giữ vòng đó tới khi xong (≤ ANSWER_DEADLINE).
    while !stop.load(Ordering::SeqCst) {
        CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, 0.25, false);
    }
    // Ân hạn trước khi gỡ: đóng nguồn khi hệ thống còn một request đang chờ trả lời thì tiến trình hỏi font bị kẹt hàng chục giây
    // (đã đo: >40 s). `Drop` đã `close()` responder nên callback đang chạy/đến muộn trả rỗng ngay; ta cứ phục vụ thêm một nhịp
    // để các câu trả lời đó kịp về, rồi mới gỡ.
    let grace = std::time::Instant::now() + Duration::from_millis(400);
    while std::time::Instant::now() < grace {
        CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, 0.1, false);
    }
    rl.remove_source(Some(&src), unsafe { kCFRunLoopDefaultMode });
    // Huỷ hẳn nguồn (đóng cổng mach) để dịch vụ font của hệ thống biết nguồn đã chết, thay vì còn gửi request vào chỗ không
    // ai trả lời (đã thấy: tiến trình hỏi font đứng hàng chục giây sau khi một nguồn trong cùng tiến trình bị đóng).
    src.invalidate();
    drop(block);
}

fn read_request(d: &CFDictionary) -> Request {
    let d = unsafe { d.cast_unchecked::<CFString, CFString>() };
    let get = |k: &CFString| d.get(k).map(|v| v.to_string()).filter(|s| !s.is_empty());
    unsafe { Request { ps: get(kCTFontNameAttribute), family: get(kCTFontFamilyNameAttribute), style: get(kCTFontStyleNameAttribute) } }
}

/// Mảng cuối cùng ta trả đi, giữ thêm MỘT tham chiếu để đo xem hệ thống có giải phóng phần "cho đi" không
/// (xem [`last_returned_retain_count`]). Mỗi lần thay thì tham chiếu của mảng trước được nhả.
static LAST_RETURNED: Mutex<Option<Probe>> = Mutex::new(None);

/// CFArray bất biến (không có ai ghi) nên chia sẻ giữa luồng được; chỉ dùng để đếm retain.
struct Probe(CFRetained<CFArray>);
// SAFETY: CFArray bất biến, CFRetain/CFRelease an toàn đa luồng.
unsafe impl Send for Probe {}

/// Chỉ để đo/kiểm: retain count của mảng cuối cùng ta trả cho hệ thống. Ta tạo mảng (+1), giữ thêm một ref cho probe (+1),
/// "cho đi" ref đầu (mem::forget). Nếu hệ thống TIÊU THỤ phần cho đi (release sau khi dùng) thì còn đúng 1 (ref của probe):
/// không rò. Còn 2 nghĩa là hệ thống không release → mỗi request rò một mảng nhỏ (cần đổi sang trả mảng autorelease).
pub fn last_returned_retain_count() -> Option<isize> {
    LAST_RETURNED.lock().ok()?.as_ref().map(|a| a.0.retain_count() as isize)
}

/// Hợp đồng của hệ thống (libFontRegistry): mảng `CFURL` của file font; hệ thống cấp sandbox extension cho app hỏi.
/// Mảng trả về được "cho đi" (+1) như spike 06-1; callback không có chỗ nào để giải phóng sau.
/// Có rò hay không: đo bằng `last_returned_retain_count` trong tests/coretext_e2e.rs.
fn to_cf_array(a: &Answer) -> NonNull<CFArray> {
    let arr: CFRetained<CFArray> = match a {
        Answer::Files(paths) => {
            let urls: Vec<CFRetained<CFURL>> = paths.iter().filter_map(CFURL::from_file_path).collect();
            unsafe { CFRetained::cast_unchecked(CFArray::<CFURL>::from_retained_objects(&urls)) }
        }
        Answer::Nothing => unsafe { CFRetained::cast_unchecked(CFArray::<CFString>::from_retained_objects(&[])) },
    };
    let p = NonNull::from(&*arr);
    if let Ok(mut g) = LAST_RETURNED.lock() {
        *g = Some(Probe(arr.clone()));
    }
    std::mem::forget(arr);
    p
}

/// Đường dẫn file chạy của một tiến trình (`proc_pidpath`).
pub fn exe_path_of_pid(pid: i32) -> Option<String> {
    let mut buf = vec![0u8; 4096];
    let n = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    if n <= 0 {
        return None;
    }
    buf.truncate(n as usize);
    String::from_utf8(buf).ok()
}

/// `AppNamer` thật: pid → "Adobe Photoshop 2026" (tên gói `.app`).
pub struct PidAppNamer;
impl AppNamer for PidAppNamer {
    fn app_name(&self, pid: i32) -> Option<String> {
        crate::app_label_from_exe_path(&exe_path_of_pid(pid)?)
    }
}
