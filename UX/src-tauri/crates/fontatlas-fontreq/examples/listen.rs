//! Công cụ đo: mở nguồn font request, IN mọi request (pid, app, tên) và trả file fixture cho `LibreBaskerville-Regular`
//! (không đăng ký font nào vào hệ thống). Chạy: listen <giây> [fixture.ttf]
#[cfg(target_os = "macos")]
fn main() {
    use std::sync::Arc;
    use std::time::Duration;

    use fontatlas_broker::{EnsureRequest, EnsureResponse, FontResult, FontStatus};
    use fontatlas_fontreq::{Answer, FontRequestSource, Observer, PidAppNamer, Responder};

    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(30);
    let font = std::env::args().nth(2);
    let responder = Arc::new(Responder::new(
        move |q: &EnsureRequest| {
            let f = &q.fonts[0];
            let ps = f.ps.clone().unwrap_or_default();
            let hit = font.is_some() && (ps == "LibreBaskerville-Regular" || f.family.as_deref() == Some("Libre Baskerville"));
            Ok(EnsureResponse {
                ok: true,
                v: 1,
                app: q.app.clone(),
                results: vec![FontResult {
                    name: ps,
                    status: if hit { FontStatus::Activated } else { FontStatus::Missing },
                    path: if hit { font.clone() } else { None },
                }],
                found: vec![],
                missing: vec![],
            })
        },
        || true,
        PidAppNamer,
    ));
    let obs: Arc<Observer> = Arc::new(|pid, req, ans, d| {
        let app = fontatlas_fontreq::exe_path_of_pid(pid).unwrap_or_default();
        println!("REQUEST pid={pid} exe={app} ps={:?} family={:?} style={:?} -> {} ({d:?})", req.ps, req.family, req.style, if matches!(ans, Answer::Nothing) { "nothing" } else { "file" });
    });
    let src = FontRequestSource::start(responder, Some(obs)).expect("nguồn font request");
    println!("listening {secs}s");
    std::thread::sleep(Duration::from_secs(secs));
    drop(src);
}
#[cfg(not(target_os = "macos"))]
fn main() {}
