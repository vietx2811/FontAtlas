//! Đo (spike): một request tạo ngay trong CÙNG tiến trình có tới nguồn font request của chính tiến trình đó không?
//! Chạy: selfprobe <số lần> — in số request nhận được cho tên probe.
#[cfg(target_os = "macos")]
fn main() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use fontatlas_broker::{EnsureRequest, EnsureResponse, FontResult, FontStatus};
    use fontatlas_fontreq::{FontRequestSource, Observer, PidAppNamer, Responder};
    use objc2_core_foundation::CFString;
    use objc2_core_text::CTFont;

    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let seen = Arc::new(AtomicUsize::new(0));
    let s2 = seen.clone();
    let responder = Arc::new(Responder::new(
        |q: &EnsureRequest| {
            Ok(EnsureResponse { ok: true, v: 1, app: q.app.clone(), results: vec![FontResult { name: q.fonts[0].ps.clone().unwrap_or_default(), status: FontStatus::Missing, path: None }], found: vec![], missing: vec![] })
        },
        || true,
        PidAppNamer,
    ));
    let obs: Arc<Observer> = Arc::new(move |pid, req, _a, d| {
        s2.fetch_add(1, Ordering::SeqCst);
        println!("REQUEST pid={pid} ps={:?} family={:?} ({d:?})", req.ps, req.family);
    });
    let _src = FontRequestSource::start(responder, Some(obs)).expect("nguồn");
    for i in 0..n {
        let name = format!("FontAtlasProbe-{:08x}{i}", std::process::id() as u64 * 7919 + i as u64);
        let t = Instant::now();
        // chạy trên luồng riêng: nguồn nằm ở luồng khác nên không tự khoá
        let h = std::thread::spawn(move || {
            let n = CFString::from_str(&name);
            let f = unsafe { CTFont::with_name(&n, 12.0, std::ptr::null()) };
            unsafe { f.post_script_name() }.to_string()
        });
        let got = h.join().unwrap();
        println!("probe {i}: got={got} took={:?} seen_total={}", t.elapsed(), seen.load(Ordering::SeqCst));
        std::thread::sleep(Duration::from_millis(200));
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {}
