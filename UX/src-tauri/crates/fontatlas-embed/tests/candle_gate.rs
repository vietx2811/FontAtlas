//! Cổng Plan 4b: candle (SigLIP vision có head) khớp torch trên ảnh vàng; CPU ↔ Metal.
mod common;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use common::*;
use fontatlas_embed::{Backend, CandleEncoder, Encode, Gray224, to_tensor};

fn model() -> PathBuf {
    PathBuf::from(std::env::var("FONTATLAS_SIGLIP").expect("FONTATLAS_SIGLIP=<model.safetensors>"))
}

fn golden_tensors() -> Vec<Vec<f32>> {
    SAMPLES_LATIN
        .iter()
        .map(|f| {
            let mut t = vec![0.0; 3 * 224 * 224];
            to_tensor(&Gray224(load_golden_png(f)), &mut t);
            t
        })
        .collect()
}

fn encode_all(e: &mut CandleEncoder) -> Vec<Vec<f32>> {
    golden_tensors().chunks(4).flat_map(|c| e.encode(c, &AtomicBool::new(false)).unwrap()).collect()
}

fn check_backend(b: Backend) {
    let mut e = CandleEncoder::load(&model(), b).unwrap();
    let refs: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("data/torch_ref.json")).unwrap();
    let out = encode_all(&mut e);
    let cs: Vec<f32> = SAMPLES_LATIN
        .iter()
        .zip(&out)
        .map(|(f, o)| cos(o, &serde_json::from_value::<Vec<f32>>(refs[*f].clone()).unwrap()))
        .collect();
    let min = cs.iter().cloned().fold(f32::INFINITY, f32::min);
    println!("gate {}: cos min {min:.6} mean {:.6}", e.device_id(), cs.iter().sum::<f32>() / cs.len() as f32);
    assert!(min >= 0.999, "{}: cos min {min}", e.device_id());
}

#[test]
#[ignore = "cần model thật: FONTATLAS_SIGLIP"]
fn candle_cpu_matches_torch_on_golden_pngs() {
    check_backend(Backend::Cpu);
}

#[test]
#[ignore = "cần model thật + Metal: FONTATLAS_SIGLIP"]
fn candle_metal_matches_torch_on_golden_pngs() {
    check_backend(Backend::Metal);
}

#[test]
#[ignore = "cần model thật + Metal: FONTATLAS_SIGLIP"]
fn cpu_and_metal_agree() {
    let a = encode_all(&mut CandleEncoder::load(&model(), Backend::Cpu).unwrap());
    let b = encode_all(&mut CandleEncoder::load(&model(), Backend::Metal).unwrap());
    let min = a.iter().zip(&b).map(|(x, y)| cos(x, y)).fold(f32::INFINITY, f32::min);
    println!("cpu↔metal cos min {min:.7}");
    assert!(min >= 0.999);
}
