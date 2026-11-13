#![cfg(target_os = "macos")]
//! Thật sự đi qua CoreText: nguồn font request chạy trong tiến trình test, một tiến trình khác hỏi font.
//! Chỉ TRẢ URL file fixture (không đăng ký font nào vào hệ thống), nên không để lại gì sau test.
//!
//! MỘT test duy nhất, MỘT nguồn: nguồn font request là tài nguyên của cả phiên đăng nhập, và đã thấy dịch vụ font của hệ
//! thống làm kẹt tiến trình hỏi hàng chục giây khi trong cùng tiến trình có nhiều nguồn được mở/đóng liên tiếp
//! (app thật chỉ mở một nguồn suốt đời). Nên các kịch bản chạy tuần tự trên cùng một nguồn, kịch bản "đóng giữa chừng" ở cuối.
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fontatlas_broker::{EnsureError, EnsureRequest, EnsureResponse, FontResult, FontStatus};
use fontatlas_fontreq::{Answer, FontRequestSource, Observer, PidAppNamer, Request, Responder, last_returned_retain_count};

type Seen = Vec<(i32, Request, Answer, Duration)>;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures/libre_baskerville.ttf").canonicalize().unwrap()
}

fn requester() -> PathBuf {
    let exe = std::env::current_exe().unwrap(); // target/<profile>/deps/coretext_e2e-xxxx
    exe.parent().unwrap().parent().unwrap().join("examples").join("requester")
}

fn spawn(name: &str) -> Child {
    Command::new(requester()).arg(name).stdout(Stdio::piped()).spawn().expect("chạy requester (cargo build --examples)")
}

/// Chờ tiến trình hỏi có hạn: nếu hệ thống giữ yêu cầu quá lâu thì test báo lỗi rõ thay vì treo.
fn finish(mut child: Child, name: &str, limit: Duration) -> String {
    use std::io::Read;
    let t0 = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if t0.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            panic!("requester kẹt >{limit:?} khi hỏi {name} (hệ thống không trả lời?)");
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    out
}

fn ask(name: &str) -> String {
    finish(spawn(name), name, Duration::from_secs(30))
}

#[test]
fn coretext_end_to_end_on_one_source() {
    // Phụ thuộc trạng thái của cả phiên đăng nhập (dịch vụ font, tiến trình khác giữ nguồn, máy bận): không chặn CI chung.
    // Chạy tay trước khi đổi gì ở lớp này: FONTATLAS_E2E_CORETEXT=1 cargo test -p fontatlas-fontreq --test coretext_e2e
    if std::env::var_os("FONTATLAS_E2E_CORETEXT").is_none() {
        eprintln!("bỏ qua: đặt FONTATLAS_E2E_CORETEXT=1 để chạy test đi qua CoreText thật (xem docs L2)");
        return;
    }
    let path = fixture();
    let want = path.to_string_lossy().into_owned();
    let asked = Arc::new(AtomicU64::new(0));
    // hành vi của "thư viện" giả: 0 = bình thường, 1 = chậm 6 s (để thử đóng giữa chừng)
    let mode = Arc::new(AtomicU64::new(0));
    let (a2, m2, w2) = (asked.clone(), mode.clone(), want.clone());
    let responder = Arc::new(Responder::new(
        move |q: &EnsureRequest| -> Result<EnsureResponse, EnsureError> {
            a2.fetch_add(1, Ordering::SeqCst);
            if m2.load(Ordering::SeqCst) == 1 {
                std::thread::sleep(Duration::from_secs(6));
                return Err(EnsureError::Internal("chậm".into()));
            }
            let ps = q.fonts[0].ps.clone().unwrap_or_default();
            let hit = ps == "LibreBaskerville-Regular";
            Ok(EnsureResponse {
                ok: true,
                v: 1,
                app: q.app.clone(),
                results: vec![FontResult {
                    name: ps,
                    status: if hit { FontStatus::Activated } else { FontStatus::Missing },
                    path: hit.then(|| w2.clone()),
                }],
                found: vec![],
                missing: vec![],
            })
        },
        || true,
        PidAppNamer,
    ));
    let log: Arc<Mutex<Seen>> = Arc::default();
    let l2 = log.clone();
    let obs: Arc<Observer> = Arc::new(move |pid, req, ans, d| l2.lock().unwrap().push((pid, req.clone(), ans.clone(), d)));
    let src = FontRequestSource::start(responder, Some(obs)).expect("nguồn font request");

    // 1. app khác hỏi font ta có → nhận đúng font; hỏi tên lạ → không nhận
    let served = ask("LibreBaskerville-Regular");
    assert!(served.contains("got=LibreBaskerville-Regular"), "app hỏi phải nhận đúng font ta trả: {served}");
    assert!(served.contains("url=yes"), "{served}");
    std::thread::sleep(Duration::from_millis(300));
    // 2. không rò: hệ thống đã release phần mảng ta "cho đi" (còn đúng 1 = ref của probe) → `mem::forget` đúng hợp đồng (+1 trao đi)
    assert_eq!(last_returned_retain_count(), Some(1), "mảng trả đi bị rò (hệ thống không release)");
    let none = ask("FontAtlasNoSuchFont-Regular");
    assert!(!none.contains("got=FontAtlasNoSuchFont-Regular"), "{none}");
    std::thread::sleep(Duration::from_millis(300));
    // mảng rỗng: CoreFoundation dùng chung một mảng rỗng bất tử (retain count -1) nên không thể rò
    assert_eq!(last_returned_retain_count(), Some(-1));
    {
        let log = log.lock().unwrap();
        let hit = log.iter().find(|(_, r, _, _)| r.ps.as_deref() == Some("LibreBaskerville-Regular")).expect("nguồn nhận request");
        assert_eq!(hit.2, Answer::Files(vec![path]));
        assert!(hit.3 < Duration::from_secs(20), "trả lời chậm: {:?}", hit.3);
        assert!(log.iter().any(|(_, r, a, _)| r.ps.as_deref() == Some("FontAtlasNoSuchFont-Regular") && *a == Answer::Nothing));
    }
    assert!(asked.load(Ordering::SeqCst) >= 2);

    // 3. đóng nguồn khi một callback đang chờ việc nền chậm: không giữ app thoát, và tiến trình hỏi không bị kẹt
    mode.store(1, Ordering::SeqCst);
    let child = spawn("SlowThing-Regular");
    std::thread::sleep(Duration::from_millis(800)); // callback đã vào, đang chờ việc nền
    let t = Instant::now();
    drop(src);
    assert!(t.elapsed() < Duration::from_secs(20), "đóng nguồn mất {:?}", t.elapsed());
    let out = finish(child, "SlowThing-Regular", Duration::from_secs(40));
    assert!(!out.contains("got=SlowThing-Regular"), "{out}");
}
