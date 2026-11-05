use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app_lib::dto::events::{EmbedEvent, ModelEvent};
use app_lib::dto::model::{ModelState, SettingsPatchDto};
use app_lib::services::activation::NoActivation;
use app_lib::services::lifecycle::on_exit;
use app_lib::services::{boot, embed, model, settings, sources};
use app_lib::state::{AppDirs, AppState, Deps};
use fontatlas_embed::{EmbedError, Encode, Manifest, extract_vision_file, make_manifest};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

// ---- model giả ----

fn fake_safetensors() -> Vec<u8> {
    let tensors: Vec<(String, Vec<usize>, Vec<f32>)> = vec![
        ("text_model.w".into(), vec![64], (0..64).map(|i| i as f32).collect()),
        ("vision_model.big".into(), vec![1 << 18], (0..(1 << 18)).map(|i| (i as f32 * 0.37).sin()).collect()),
    ];
    let mut hdr = serde_json::Map::new();
    hdr.insert("__metadata__".into(), serde_json::json!({"format": "pt"}));
    let mut data = Vec::new();
    for (k, shape, v) in &tensors {
        let a = data.len();
        data.extend(v.iter().flat_map(|x| x.to_le_bytes()));
        hdr.insert(k.clone(), serde_json::json!({"dtype": "F32", "shape": shape, "data_offsets": [a, data.len()]}));
    }
    let mut h = serde_json::to_vec(&serde_json::Value::Object(hdr)).unwrap();
    while !(8 + h.len()).is_multiple_of(8) {
        h.push(b' ');
    }
    let mut out = (h.len() as u64).to_le_bytes().to_vec();
    out.extend(h);
    out.extend(data);
    out
}

fn manifest_for(src: &[u8], url: &str) -> Manifest {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("src.safetensors");
    std::fs::write(&p, src).unwrap();
    make_manifest(&p, url, "vision.safetensors", "test", vec![]).unwrap()
}

/// Đặt file model ở trạng thái Ready (giả) trong thư mục models.
fn install_model(models: &Path, src: &[u8]) {
    std::fs::create_dir_all(models).unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("src.safetensors");
    std::fs::write(&p, src).unwrap();
    extract_vision_file(&p, &models.join("vision.safetensors")).unwrap();
}

struct FakeEnc {
    delay: Duration,
}

impl Encode for FakeEnc {
    fn raw_dim(&self) -> usize {
        768
    }
    fn encode(&mut self, t: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        let start = Instant::now();
        while start.elapsed() < self.delay {
            if cancel.load(Ordering::SeqCst) {
                return Err(EmbedError::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(t.iter().enumerate().map(|(k, x)| (0..768).map(|i| ((x[i % x.len()] * 3.0 + i as f32 + k as f32) % 7.0) + 1.0).collect()).collect())
    }
}

fn deps(manifest: Manifest, delay_ms: u64, free: u64) -> Deps {
    Deps {
        manifest,
        encoder: Box::new(move |_| Ok((Box::new(FakeEnc { delay: Duration::from_millis(delay_ms) }) as Box<dyn Encode>, "cpu"))),
        disk_free: Box::new(move |_| Ok(free)),
    }
}

fn state_with(root: &Path, d: Deps) -> Arc<AppState> {
    let st = AppState::with_deps(AppDirs::under(&root.join("data")), Box::new(NoActivation), d);
    boot::run_boot(&st);
    st
}

fn scan3(st: &Arc<AppState>, root: &Path) {
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf"] {
        std::fs::copy(fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
}

type Log = Arc<Mutex<Vec<EmbedEvent>>>;

fn record(st: &AppState) -> Log {
    let log: Log = Arc::default();
    let l2 = log.clone();
    st.hubs.embed.subscribe(Box::new(move |e| l2.lock().unwrap().push(e)));
    log
}

fn wait_finished(log: &Log, n: usize) {
    let t = Instant::now();
    while log.lock().unwrap().iter().filter(|e| matches!(e, EmbedEvent::Finished { .. })).count() < n {
        assert!(t.elapsed() < Duration::from_secs(60), "embed không xong");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn worker_not_started_without_model() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let st = state_with(d.path(), deps(manifest_for(&src, "http://127.0.0.1:1/x"), 0, u64::MAX));
    scan3(&st, d.path());
    assert!(!embed::ensure_started(&st).unwrap());
    assert!(!st.embed.is_running());
    let s = embed::embed_status(&st).unwrap();
    assert!(!s.running && s.device.is_none());
    assert_eq!(model::model_status(&st).state, ModelState::Missing);
}

#[test]
fn embed_status_eta_and_device() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let st = state_with(d.path(), deps(manifest_for(&src, "http://127.0.0.1:1/x"), 150, u64::MAX));
    scan3(&st, d.path());
    install_model(&st.dirs.models, &src);
    let log = record(&st);
    assert!(embed::ensure_started(&st).unwrap());
    wait_finished(&log, 1);
    let ev = log.lock().unwrap();
    let statuses: Vec<_> = ev.iter().filter_map(|e| if let EmbedEvent::Status { status } = e { Some(status) } else { None }).collect();
    assert!(statuses.iter().all(|s| s.device.as_deref() == Some("cpu")));
    assert!(statuses.iter().any(|s| s.ms_per_face.is_some() && s.eta_ms.is_some() && s.pending > 0), "{statuses:?}");
    let last = embed::embed_status(&st).unwrap();
    assert_eq!((last.pending, last.done), (0, 3));
    assert!(last.ms_per_face.is_some() && last.model_version.is_some());
}

#[test]
fn toggle_off_cancels_worker() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let st = state_with(d.path(), deps(manifest_for(&src, "http://127.0.0.1:1/x"), 2000, u64::MAX));
    scan3(&st, d.path());
    install_model(&st.dirs.models, &src);
    assert!(embed::ensure_started(&st).unwrap());
    assert!(st.embed.is_running());
    let t = Instant::now();
    let s = settings::set_settings(&st, SettingsPatchDto { embed_enabled: Some(false), ..Default::default() }).unwrap();
    assert!(!s.embed_enabled && !st.embed.is_running());
    assert!(t.elapsed() < Duration::from_secs(5), "huỷ phải kịp: {:?}", t.elapsed());
    assert!(!embed::ensure_started(&st).unwrap(), "tắt thì không mở lại");
}

#[test]
fn restart_joins_previous_worker_and_resumes_running_jobs() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let st = state_with(d.path(), deps(manifest_for(&src, "http://127.0.0.1:1/x"), 0, u64::MAX));
    scan3(&st, d.path());
    install_model(&st.dirs.models, &src);
    let log = record(&st);
    assert!(embed::ensure_started(&st).unwrap());
    wait_finished(&log, 1);
    // app chết giữa lô: các job đang 'running'
    st.with_store(|s| {
        s.conn().execute("UPDATE pipeline_state SET status='running' WHERE stage='embedding'", []).unwrap();
        Ok(())
    })
    .unwrap();
    assert_eq!(embed::embed_status(&st).unwrap().pending, 3);
    assert!(embed::ensure_started(&st).unwrap()); // worker cũ đã xong → join rồi mở mới
    wait_finished(&log, 2);
    let s = embed::embed_status(&st).unwrap();
    assert_eq!((s.pending, s.done, s.failed), (0, 3, 0));
}

#[test]
fn shutdown_cancels_and_joins_workers() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let st = state_with(d.path(), deps(manifest_for(&src, "http://127.0.0.1:1/x"), 3000, u64::MAX));
    scan3(&st, d.path());
    install_model(&st.dirs.models, &src);
    assert!(embed::ensure_started(&st).unwrap());
    std::thread::sleep(Duration::from_millis(100));
    let t = Instant::now();
    on_exit(&st);
    assert!(t.elapsed() < Duration::from_secs(5), "{:?}", t.elapsed());
    assert!(!st.embed.is_running() && !st.scans.status().running);
    // job bị huỷ giữa lô trả về pending, không có hàng failed
    assert_eq!(embed::embed_status(&st).unwrap().failed, 0);
}

#[test]
fn queue_pause_all_pauses_scan_and_embed() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let st = state_with(d.path(), deps(manifest_for(&src, "http://127.0.0.1:1/x"), 0, u64::MAX));
    embed::pause_all(&st);
    assert!(st.scans.is_paused() && st.embed.is_paused());
    embed::resume_all(&st);
    assert!(!st.scans.is_paused() && !st.embed.is_paused());
}

#[test]
fn model_download_insufficient_disk() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let m = manifest_for(&src, "http://127.0.0.1:1/x");
    let st = state_with(d.path(), deps(m.clone(), 0, 1000));
    let e = model::start_download(&st).unwrap_err();
    assert_eq!(e.code, "insufficient_disk");
    let det = e.detail.unwrap();
    assert_eq!(det["free"], 1000);
    assert_eq!(det["required"].as_u64().unwrap(), (m.size as f64 * 1.1) as u64);
    assert!(!st.model.is_downloading());
}

#[test]
fn settings_roundtrip_and_validation() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let st = state_with(d.path(), deps(manifest_for(&src, "http://127.0.0.1:1/x"), 0, u64::MAX));
    let s = settings::get_settings(&st).unwrap();
    assert_eq!(
        (s.embed_enabled, s.license_notice_ack, s.ui_lang.as_str(), s.sample_text.as_str(), s.project_vi_default, s.keep_active_on_exit, s.rescan_on_launch),
        (true, false, "vi", settings::DEFAULT_SAMPLE, true, false, true)
    );
    let long = format!("  {}  ", "Đ".repeat(100));
    let nfd = "Ho\u{0302}\u{0300}";
    let p = SettingsPatchDto {
        license_notice_ack: Some(true),
        ui_lang: Some("en".into()),
        sample_text: Some(long),
        project_vi_default: Some(false),
        rescan_on_launch: Some(false),
        keep_active_on_exit: Some(true),
        ..Default::default()
    };
    let s = settings::set_settings(&st, p).unwrap();
    assert!(s.license_notice_ack && s.keep_active_on_exit && !s.project_vi_default && !s.rescan_on_launch);
    assert_eq!((s.ui_lang.as_str(), s.sample_text.chars().count()), ("en", 64));
    let s = settings::set_settings(&st, SettingsPatchDto { sample_text: Some(nfd.into()), ..Default::default() }).unwrap();
    assert_eq!(s.sample_text, "Hồ", "chuẩn hoá NFC");
    let bad = settings::set_settings(&st, SettingsPatchDto { ui_lang: Some("fr".into()), ..Default::default() }).unwrap_err();
    assert_eq!(bad.code, "invalid_arg");
    assert_eq!(settings::get_settings(&st).unwrap().ui_lang, "en");
}

// ---- tải model qua HTTP giả ----

struct Srv {
    server: Arc<tiny_http::Server>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Srv {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Server hỗ trợ Range; `flip` đảo byte cuối mỗi đoạn (giả lỗi checksum).
fn serve(file: Vec<u8>, flip: bool) -> (Srv, String) {
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let url = format!("http://{}/model.safetensors", server.server_addr().to_ip().unwrap());
    let s2 = server.clone();
    let handle = std::thread::spawn(move || {
        for rq in s2.incoming_requests() {
            let range = rq.headers().iter().find(|h| h.field.equiv("Range")).and_then(|h| {
                let (a, b) = h.value.as_str().strip_prefix("bytes=")?.split_once('-')?;
                Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))
            });
            match range {
                Some((a, b)) => {
                    let mut part = file[a..=b].to_vec();
                    if flip && let Some(l) = part.last_mut() {
                        *l ^= 0xff;
                    }
                    let cr = format!("bytes {a}-{b}/{}", file.len());
                    let h = tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
                    let _ = rq.respond(tiny_http::Response::from_data(part).with_status_code(206).with_header(h));
                }
                None => {
                    let _ = rq.respond(tiny_http::Response::from_data(file.clone()));
                }
            }
        }
    });
    (Srv { server, handle: Some(handle) }, url)
}

fn model_log(st: &AppState) -> Arc<Mutex<Vec<ModelEvent>>> {
    let log: Arc<Mutex<Vec<ModelEvent>>> = Arc::default();
    let l2 = log.clone();
    st.hubs.model.subscribe(Box::new(move |e| l2.lock().unwrap().push(e)));
    log
}

#[test]
fn embed_kicked_when_model_becomes_ready_and_after_scan_idle() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let (_srv, url) = serve(src.clone(), false);
    let st = state_with(d.path(), deps(manifest_for(&src, &url), 0, u64::MAX));
    scan3(&st, d.path()); // model chưa có → chưa mở worker
    assert!(!st.embed.is_running());
    let log = record(&st);
    let ml = model_log(&st);
    model::start_download(&st).unwrap();
    assert!(model::wait_download(&st, Duration::from_secs(60)));
    assert!(matches!(ml.lock().unwrap().last(), Some(ModelEvent::Ready)));
    assert!(ml.lock().unwrap().iter().any(|e| matches!(e, ModelEvent::Progress { .. })));
    assert_eq!(model::model_status(&st).state, ModelState::Ready);
    wait_finished(&log, 1); // tải xong → ensure_started → embed 3 face
    assert_eq!(embed::embed_status(&st).unwrap().done, 3);
    // thêm font mới → quét xong (Idle) → worker mở lại tự động
    std::fs::copy(fixture("source_code_pro.ttf"), d.path().join("Fonts/source_code_pro.ttf")).unwrap();
    sources::rescan_sources(&st, None).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    wait_finished(&log, 2);
    assert_eq!(embed::embed_status(&st).unwrap().done, 4);
}

#[test]
fn model_last_error_survives_restart() {
    let d = tempfile::tempdir().unwrap();
    let src = fake_safetensors();
    let (_srv, url) = serve(src.clone(), true);
    let m = manifest_for(&src, &url);
    {
        let st = state_with(d.path(), deps(m.clone(), 0, u64::MAX));
        let ml = model_log(&st);
        model::start_download(&st).unwrap();
        assert!(model::wait_download(&st, Duration::from_secs(60)));
        assert!(matches!(ml.lock().unwrap().last(), Some(ModelEvent::Failed { error }) if error.code == "checksum"));
        assert_eq!(model::model_status(&st).state, ModelState::ChecksumMismatch);
    }
    // mở lại app: trạng thái suy ra được từ settings
    let st = state_with(d.path(), deps(m, 0, u64::MAX));
    let s = model::model_status(&st);
    assert_eq!(s.state, ModelState::ChecksumMismatch);
    assert_eq!(s.last_error.map(|e| e.code), Some("checksum".into()));
}
