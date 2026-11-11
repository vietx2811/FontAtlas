//! Tiến trình giả làm "app thiết kế" cho test: hỏi một font theo tên PostScript và in ra font nhận được.
//! Chạy: requester <PostScript name>
#[cfg(target_os = "macos")]
fn main() {
    use objc2_core_foundation::CFString;
    use objc2_core_text::{CTFont, kCTFontURLAttribute};
    let name = std::env::args().nth(1).expect("tên font");
    let n = CFString::from_str(&name);
    let f = unsafe { CTFont::with_name(&n, 12.0, std::ptr::null()) };
    let got = unsafe { f.post_script_name() }.to_string();
    let url = unsafe { f.attribute(kCTFontURLAttribute) };
    println!("requested={name}\ngot={got}\nurl={}", if url.is_some() { "yes" } else { "no" });
}
#[cfg(not(target_os = "macos"))]
fn main() {}
