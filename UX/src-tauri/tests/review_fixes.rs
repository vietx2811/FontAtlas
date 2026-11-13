//! Hồi quy cho các phát hiện của reviewer độc lập (race, tắt app, xoá nguồn khi đang quét, ràng buộc cứng).
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app_lib::dto::library::*;
use app_lib::services::activation::NoActivation;
use app_lib::services::lifecycle::on_exit;
use app_lib::services::{boot, describe, embed, library, sources};
use app_lib::state::{AppDirs, AppState, Deps};
use fontatlas_embed::{EmbedError, Encode, Manifest, extract_vision_file, make_manifest};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn fake_model() -> Vec<u8> {
    let v: Vec<f32> = (0..(1 << 16)).map(|i| (i as f32 * 0.37).sin()).collect();
    let mut hdr = serde_json::Map::new();
    let data: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    hdr.insert("vision_model.big".into(), serde_json::json!({"dtype": "F32", "shape": [v.len()], "data_offsets": [0, data.len()]}));
    let mut h = serde_json::to_vec(&serde_json::Value::Object(hdr)).unwrap();
    while !(8 + h.len()).is_multiple_of(8) {
        h.push(b' ');
    }
    let mut out = (h.len() as u64).to_le_bytes().to_vec();
    out.extend(h);
    out.extend(data);
    out
}

struct SlowEnc(Duration);

impl Encode for SlowEnc {
    fn raw_dim(&self) -> usize {
        768
    }
    fn encode(&mut self, t: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        let s = Instant::now();
        while s.elapsed() < self.0 {
            if cancel.load(Ordering::SeqCst) {
                return Err(EmbedError::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(t.iter().map(|_| (0..768).map(|i| (i % 7) as f32 + 1.0).collect()).collect())
    }
}

fn state_with(root: &Path, loads: Arc<AtomicUsize>, delay: Duration) -> (Arc<AppState>, Vec<u8>) {
    let src = fake_model();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("s");
    std::fs::write(&p, &src).unwrap();
    let m: Manifest = make_manifest(&p, "http://127.0.0.1:1/x", "vision.safetensors", "t", vec![]).unwrap();
    let deps = Deps {
        manifest: m,
        encoder: Box::new(move |_| {
            loads.fetch_add(1, Ordering::SeqCst);
            Ok((Box::new(SlowEnc(delay)) as Box<dyn Encode>, "cpu"))
        }),
        disk_free: Box::new(|_| Ok(u64::MAX)),
    };
    let st = AppState::with_deps(AppDirs::under(&root.join("data")), Box::new(NoActivation), deps);
    boot::run_boot(&st);
    std::fs::create_dir_all(&st.dirs.models).unwrap();
    extract_vision_file(&p, &st.dirs.models.join("vision.safetensors")).unwrap();
    (st, src)
}

fn fonts(root: &Path, n: usize) -> PathBuf {
    let d = root.join("Fonts");
    std::fs::create_dir_all(&d).unwrap();
    let raw = std::fs::read(fixture("roboto.ttf")).unwrap();
    for i in 0..n {
        let mut b = raw.clone();
        b.extend_from_slice(&(i as u32).to_le_bytes());
        std::fs::write(d.join(format!("r{i:04}.ttf")), b).unwrap();
    }
    d
}

#[test]
fn concurrent_ensure_started_opens_exactly_one_worker() {
    let d = tempfile::tempdir().unwrap();
    let loads = Arc::new(AtomicUsize::new(0));
    let (st, _) = state_with(d.path(), loads.clone(), Duration::from_millis(400));
    let dir = fonts(d.path(), 6);
    // thêm font nhưng KHÔNG để quét tự mở worker: model đã Ready nên after_idle sẽ mở → đợi nó rồi dừng
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    st.embed.stop();
    loads.store(0, Ordering::SeqCst);
    let hs: Vec<_> = (0..6)
        .map(|_| {
            let s = st.clone();
            std::thread::spawn(move || embed::ensure_started(&s).unwrap())
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    assert_eq!(loads.load(Ordering::SeqCst), 1, "mỗi lần chỉ một worker (một lần nạp encoder)");
    st.embed.stop();
}

#[test]
fn remove_source_while_scanning_leaves_no_orphans() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = state_with(d.path(), Arc::default(), Duration::ZERO);
    let dir = fonts(d.path(), 20);
    st.scans.pause(&st);
    let s = sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    std::thread::sleep(Duration::from_millis(200)); // quét đang kẹt ở callback đầu
    sources::remove_source(&st, s.id).unwrap();
    st.scans.resume(&st);
    assert!(st.scans.wait_idle(Duration::from_secs(30)));
    let total = library::query_faces(&st, &FaceQueryDto { include_missing: true, ..Default::default() }).unwrap().total;
    assert_eq!(total, 0, "font của nguồn đã xoá không được hiện lại");
    assert!(sources::list_sources(&st).unwrap().is_empty());
}

#[test]
fn exit_during_scan_is_prompt_and_starts_nothing_after() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = state_with(d.path(), Arc::default(), Duration::from_millis(50));
    let dir = fonts(d.path(), 800);
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let t = Instant::now();
    on_exit(&st);
    assert!(t.elapsed() < Duration::from_secs(25), "{:?}", t.elapsed()); // quét đầy đủ cần ~40 s (800 font × 50 ms): xong sớm hơn nghĩa là đã huỷ
    assert!(!st.scans.status().running);
    assert!(!st.embed.is_running(), "không mở worker embed sau khi tắt");
}

#[test]
fn describe_hard_constraint_filters_incomplete_vietnamese() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = state_with(d.path(), Arc::default(), Duration::ZERO);
    let dir = d.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["roboto.ttf", "libre_baskerville.ttf"] {
        std::fs::copy(fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    app_lib::index_bundle::wait_rebuilt(&st, Duration::from_secs(30));
    let desc = describe::describe(&st, "đủ tiếng Việt");
    assert!(desc.hard.contains(&"vi_complete".to_string()));
    let q = FaceQueryDto { hard: desc.hard, limit: 10, ..Default::default() };
    let fams: Vec<String> = library::query_faces(&st, &q).unwrap().items.into_iter().map(|r| r.family).collect();
    assert_eq!(fams, ["Roboto"], "Libre Baskerville thiếu 68 ký tự phải bị loại");
}

#[test]
fn redownload_while_download_stuck_reports_busy() {
    // tải không dừng kịp (server treo) → redownload báo busy, không xoá file
    let d = tempfile::tempdir().unwrap();
    let (st, _) = state_with(d.path(), Arc::default(), Duration::ZERO);
    let model_file = st.dirs.models.join("vision.safetensors");
    assert!(model_file.exists());
    let hold = Arc::new(Mutex::new(()));
    let _ = hold;
    // mô phỏng "đang tải": cờ downloading bật mà không có luồng
    app_lib::services::model::force_downloading_for_test(&st, true);
    let r = app_lib::services::model::redownload_within(&st, Duration::from_millis(100));
    assert_eq!(r.unwrap_err().code, "busy");
    assert!(model_file.exists(), "không được xoá file khi không dừng được lần tải");
    app_lib::services::model::force_downloading_for_test(&st, false);
}
