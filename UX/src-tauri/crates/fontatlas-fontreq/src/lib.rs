//! fontatlas-fontreq: trả lời "font request" của hệ thống (CoreText) bằng thư viện của Font Atlas.
//!
//! Photoshop/After Effects (và vài app khác) hỏi font còn thiếu qua `CTFontManagerCreateFontRequestRunLoopSource`:
//! nguồn nào đăng ký sẽ được gọi với thuộc tính font + pid của app hỏi, và trả mảng `CFURL` của file font.
//! Crate tách làm hai:
//! - [`Responder`] (mọi nền tảng, test được): lọc tên nội bộ, ánh xạ pid → app, hỏi broker (`ensure`), nhớ "không có" ít giây.
//! - `mac` (chỉ macOS): đăng ký nguồn runloop trên một luồng riêng và đổi [`Answer`] thành `CFArray<CFURL>`.
mod responder;
pub use responder::{Answer, AppNamer, Request, Responder, app_label_from_exe_path, is_internal_name, ANSWER_DEADLINE, NEGATIVE_TTL};

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use mac::{FontRequestSource, Observer, PidAppNamer, StartError, exe_path_of_pid};
