//! Nguồn font request trên macOS. Chạy trên MỘT luồng riêng có runloop riêng (callback có thể chờ tải file placeholder
//! tới `AUTO_DOWNLOAD_TIMEOUT`, không được chặn luồng giao diện).
#![allow(deprecated)]
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::Arc;
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
}

impl FontRequestSource {
    /// Đăng ký nguồn. Trả `Err` nếu hệ thống từ chối; khi đó app vẫn chạy, chỉ mất tự bật font cho Photoshop/AE.
    pub fn start(responder: Arc<Responder>, observer: Option<Arc<Observer>>) -> Result<Self, StartError> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let (tx, rx) = mpsc::channel::<bool>();
        let thread = std::thread::Builder::new()
            .name("fontatlas-fontreq".into())
            .spawn(move || run(responder, observer, stop2, tx))
            .map_err(StartError::Thread)?;
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(true) => Ok(Self { stop, thread: Some(thread) }),
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
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
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
    while !stop.load(Ordering::SeqCst) {
        CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, 0.25, false);
    }
    rl.remove_source(Some(&src), unsafe { kCFRunLoopDefaultMode });
    drop(block);
}

fn read_request(d: &CFDictionary) -> Request {
    let d = unsafe { d.cast_unchecked::<CFString, CFString>() };
    let get = |k: &CFString| d.get(k).map(|v| v.to_string()).filter(|s| !s.is_empty());
    unsafe { Request { ps: get(kCTFontNameAttribute), family: get(kCTFontFamilyNameAttribute), style: get(kCTFontStyleNameAttribute) } }
}

/// Hợp đồng của hệ thống (libFontRegistry): mảng `CFURL` của file font; hệ thống cấp sandbox extension cho app hỏi.
/// Mảng trả về được "cho đi" (+1) như spike 06-1; callback không có chỗ nào để giải phóng sau.
fn to_cf_array(a: &Answer) -> NonNull<CFArray> {
    let arr: CFRetained<CFArray> = match a {
        Answer::Files(paths) => {
            let urls: Vec<CFRetained<CFURL>> = paths.iter().filter_map(CFURL::from_file_path).collect();
            unsafe { CFRetained::cast_unchecked(CFArray::<CFURL>::from_retained_objects(&urls)) }
        }
        Answer::Nothing => unsafe { CFRetained::cast_unchecked(CFArray::<CFString>::from_retained_objects(&[])) },
    };
    let p = NonNull::from(&*arr);
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
