//! Cổng Plan 4b: ms/face cùng ảnh, cùng lô 4 cho candle-cpu | candle-metal | ort.
//! cargo run --release --example bench_backends -- <cpu|metal|ort> <model> [ort_threads]
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use fontatlas_core::{FontFile, standard_location};
use fontatlas_embed::{Backend, CandleEncoder, Encode, OrtEncoder, render_input, to_tensor};

const FIXTURES: [&str; 6] =
    ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "source_code_pro.ttf", "libre_baskerville.ttf", "public_sans_vf.ttf"];

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (which, model) = (a[1].as_str(), Path::new(&a[2]));
    let threads: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures");
    let imgs: Vec<Vec<f32>> = FIXTURES
        .iter()
        .map(|f| {
            let file = FontFile::open(&dir.join(f)).unwrap();
            let faces = file.faces().unwrap();
            let font = faces[0].font();
            let img = render_input(font, &standard_location(font)).unwrap();
            let mut t = vec![0.0; 3 * 224 * 224];
            to_tensor(&img, &mut t);
            t
        })
        .collect();
    let batch: Vec<Vec<f32>> = (0..4).map(|i| imgs[i % imgs.len()].clone()).collect();
    let t0 = Instant::now();
    let mut enc: Box<dyn Encode> = match which {
        "cpu" => Box::new(CandleEncoder::load(model, Backend::Cpu).unwrap()),
        "metal" => Box::new(CandleEncoder::load(model, Backend::Metal).unwrap()),
        "ort" => Box::new(OrtEncoder::load(model, threads).unwrap()),
        _ => panic!("cpu|metal|ort"),
    };
    let load_s = t0.elapsed().as_secs_f64();
    let c = AtomicBool::new(false);
    enc.encode(&batch, &c).unwrap(); // khởi động
    let n_batches = 16;
    let t1 = Instant::now();
    for i in 0..n_batches {
        let b: Vec<Vec<f32>> = (0..4).map(|j| imgs[(i * 4 + j) % imgs.len()].clone()).collect();
        enc.encode(&b, &c).unwrap();
    }
    let ms = t1.elapsed().as_secs_f64() * 1000.0 / (n_batches * 4) as f64;
    println!("{{\"backend\":\"{which}\",\"ms_per_face\":{ms:.2},\"load_s\":{load_s:.2},\"faces\":{}}}", n_batches * 4);
}
