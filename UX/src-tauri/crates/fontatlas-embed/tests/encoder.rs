mod common;
use std::sync::atomic::AtomicBool;

use common::*;
use fontatlas_embed::{Backend, CandleEncoder, EmbedError, Encode, select_backend};

const PIXELS: usize = 3 * 224 * 224;

fn tiny(b: Backend) -> CandleEncoder {
    CandleEncoder::load_with(&data("tiny_siglip.safetensors"), b, &tiny_vision_config()).unwrap()
}

fn inputs() -> Vec<Vec<f32>> {
    let x0 = (0..PIXELS).map(|i| (i % 251) as f32 / 125.0 - 1.0).collect();
    let x1 = (0..PIXELS).map(|i| ((31 * i) % 241) as f32 / 120.0 - 1.0).collect();
    vec![x0, x1]
}

fn refs() -> Vec<Vec<f32>> {
    let r: serde_json::Value = serde_json::from_str(include_str!("data/tiny_siglip_ref.json")).unwrap();
    ["x0", "x1"].iter().map(|k| serde_json::from_value(r[*k].clone()).unwrap()).collect()
}

fn assert_close(out: &[Vec<f32>], want: &[Vec<f32>]) {
    for (o, r) in out.iter().zip(want) {
        assert_eq!(o.len(), 32);
        let c = cos(o, r);
        let max_abs = o.iter().zip(r).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
        assert!(c >= 0.9999 && max_abs < 1e-3, "cos {c} max_abs {max_abs}");
    }
}

#[test]
fn tiny_cpu_matches_torch() {
    let mut e = tiny(Backend::Cpu);
    assert_eq!((e.raw_dim(), e.device_id()), (32, "cpu"));
    assert_close(&e.encode(&inputs(), &AtomicBool::new(false)).unwrap(), &refs());
}

#[test]
fn tiny_selected_backend_matches_torch() {
    let b = select_backend();
    if cfg!(target_os = "macos") {
        assert_eq!(b, Backend::Metal, "máy dev macOS có GPU");
    }
    let mut e = tiny(b);
    assert_close(&e.encode(&inputs(), &AtomicBool::new(false)).unwrap(), &refs());
}

#[test]
fn partial_batch_same_as_single() {
    let mut e = tiny(Backend::Cpu);
    let xs = inputs();
    let both = e.encode(&xs, &AtomicBool::new(false)).unwrap();
    let one = e.encode(&xs[1..], &AtomicBool::new(false)).unwrap();
    assert!(cos(&both[1], &one[0]) > 0.99999);
}

#[test]
fn batch_limits() {
    let mut e = tiny(Backend::Cpu);
    let c = AtomicBool::new(false);
    assert!(matches!(e.encode(&[], &c), Err(EmbedError::Model(_))));
    assert!(matches!(e.encode(&vec![vec![0.0; PIXELS]; 5], &c), Err(EmbedError::Model(_))));
}

#[test]
fn cancelled_before_run() {
    let mut e = tiny(Backend::Cpu);
    assert!(matches!(e.encode(&inputs(), &AtomicBool::new(true)), Err(EmbedError::Cancelled)));
}

#[test]
fn wrong_tensor_length_is_error() {
    let mut e = tiny(Backend::Cpu);
    assert!(matches!(e.encode(&[vec![0.0; 10]], &AtomicBool::new(false)), Err(EmbedError::Model(_))));
}

#[test]
fn garbage_file_rejected() {
    let p = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(p.path(), b"khong phai safetensors").unwrap();
    assert!(matches!(CandleEncoder::load(p.path(), Backend::Cpu), Err(EmbedError::Model(_))));
}

#[test]
fn wrong_shapes_for_config_rejected() {
    // file tí hon (hidden 32) nạp bằng config base (hidden 768) → lệch shape
    assert!(matches!(CandleEncoder::load(&data("tiny_siglip.safetensors"), Backend::Cpu), Err(EmbedError::Model(_))));
}

/// Bản sao model tí hon với một trọng số NaN (giả Metal/GPU trả kết quả hỏng).
fn nan_model(dir: &std::path::Path) -> std::path::PathBuf {
    let mut ts = candle_core::safetensors::load(data("tiny_siglip.safetensors"), &candle_core::Device::Cpu).unwrap();
    let k = "vision_model.post_layernorm.weight";
    let n = ts[k].dims1().unwrap();
    ts.insert(k.into(), candle_core::Tensor::from_vec(vec![f32::NAN; n], n, &candle_core::Device::Cpu).unwrap());
    let p = dir.join("nan.safetensors");
    candle_core::safetensors::save(&ts, &p).unwrap();
    p
}

#[test]
fn verified_rejects_non_finite_output() {
    let d = tempfile::tempdir().unwrap();
    let e = CandleEncoder::load_with(&nan_model(d.path()), Backend::Cpu, &tiny_vision_config()).unwrap();
    assert!(matches!(e.verified(), Err(EmbedError::Model(_))));
}

#[test]
fn verified_accepts_good_model() {
    assert!(tiny(Backend::Cpu).verified().is_ok());
}
