mod common;
use std::sync::atomic::AtomicBool;

use common::*;
use fontatlas_embed::{EmbedError, Encode, OrtEncoder};

fn expected(t: &[f32]) -> Vec<f32> {
    let hw = 224 * 224;
    // cộng bằng f64: cộng dồn 50.176 số f32 giống nhau lệch tới ~5e-4
    let m: Vec<f64> = (0..3).map(|c| t[c * hw..(c + 1) * hw].iter().map(|v| *v as f64).sum::<f64>() / hw as f64).collect();
    (0..768).map(|j| (m[j % 3] * (j + 1) as f64 / 768.0) as f32).collect()
}

#[test]
fn tiny_model_matches_formula_and_pads() {
    let mut e = OrtEncoder::load(&data("tiny_encoder.onnx"), 2).unwrap();
    assert_eq!(e.raw_dim(), 768);
    let ts: Vec<Vec<f32>> = (0..3).map(|i| tensor_of_gray((i * 80) as u8)).collect(); // 3 < 4 → pad
    let out = e.encode(&ts, &AtomicBool::new(false)).unwrap();
    assert_eq!(out.len(), 3);
    for (t, o) in ts.iter().zip(&out) {
        let ex = expected(t);
        // Bản ORT tĩnh của pyke (macOS arm64) tính MatMul ở độ chính xác ~bf16 (lệch tương đối tới ~4e-3),
        // Python ORT thì khớp tuyệt đối → dung sai tương đối 5e-3. Parity model thật kiểm ở tests/parity.rs.
        for (j, (a, b)) in o.iter().zip(&ex).enumerate() {
            assert!((a - b).abs() <= 5e-3 * b.abs() + 1e-5, "t0={} j={j} got {a} want {b}", t[0]);
        }
    }
}

#[test]
fn batch_limits() {
    let mut e = OrtEncoder::load(&data("tiny_encoder.onnx"), 1).unwrap();
    assert!(e.encode(&vec![tensor_of_gray(255); 5], &AtomicBool::new(false)).is_err());
    assert!(e.encode(&[], &AtomicBool::new(false)).unwrap().is_empty());
}

#[test]
fn cancelled_before_run() {
    let mut e = OrtEncoder::load(&data("tiny_encoder.onnx"), 1).unwrap();
    assert!(matches!(e.encode(&[tensor_of_gray(255)], &AtomicBool::new(true)), Err(EmbedError::Cancelled)));
}

#[test]
fn wrong_output_shape_rejected() {
    assert!(matches!(OrtEncoder::load(&data("tiny_bad_shape.onnx"), 1), Err(EmbedError::Model(_))));
}

#[test]
fn garbage_file_rejected() {
    let p = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(p.path(), b"not onnx").unwrap();
    assert!(matches!(OrtEncoder::load(p.path(), 1), Err(EmbedError::Model(_))));
}

#[test]
fn wrong_tensor_length_is_error() {
    let mut e = OrtEncoder::load(&data("tiny_encoder.onnx"), 1).unwrap();
    assert!(e.encode(&[vec![0.0; 10]], &AtomicBool::new(false)).is_err());
}
