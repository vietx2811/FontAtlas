#![allow(dead_code)]
use std::path::PathBuf;

use fontatlas_embed::{EmbedError, Gray224, render_input};

pub const SAMPLES_LATIN: [&str; 6] = [
    "roboto.ttf",
    "open_sans.ttf",
    "playfair_display.ttf",
    "source_code_pro.ttf",
    "public_sans_vf.ttf",
    "libre_baskerville.ttf",
];

pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures").join(name)
}

pub fn fixture_bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture(name)).expect("fixture")
}

pub fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

/// Render face `index` của một file font tại vị trí đo chuẩn.
pub fn render_bytes(bytes: Vec<u8>, index: usize) -> Result<Gray224, EmbedError> {
    let f = fontatlas_core::FontFile::from_bytes(bytes).map_err(|e| EmbedError::Font(e.to_string()))?;
    let faces = f.faces().map_err(|e| EmbedError::Font(e.to_string()))?;
    let face = faces.get(index).ok_or_else(|| EmbedError::Font("index".into()))?;
    render_input(face.font(), &fontatlas_core::standard_location(face.font()))
}

pub fn render_fixture(name: &str) -> Result<Gray224, EmbedError> {
    render_bytes(fixture_bytes(name), 0)
}

/// Đổi tag một bảng trong table directory (chép từ test của core).
pub fn rename_table(mut d: Vec<u8>, from: &[u8; 4], to: &[u8; 4]) -> Vec<u8> {
    let n = u16::from_be_bytes([d[4], d[5]]) as usize;
    for i in 0..n {
        let p = 12 + 16 * i;
        if &d[p..p + 4] == from {
            d[p..p + 4].copy_from_slice(to);
        }
    }
    d
}

/// Đọc ảnh vàng (PNG xám hoặc RGB/RGBA), trả kênh R.
pub fn load_golden_png(name: &str) -> Vec<u8> {
    let file = std::fs::File::open(data(&format!("render/{name}.png"))).expect("golden png");
    let dec = png::Decoder::new(std::io::BufReader::new(file));
    let mut r = dec.read_info().expect("png info");
    let mut buf = vec![0; r.output_buffer_size().expect("size")];
    let info = r.next_frame(&mut buf).expect("png frame");
    let ch = info.color_type.samples();
    buf[..info.buffer_size()].chunks(ch).map(|p| p[0]).collect()
}

pub fn tensor_of_gray(v: u8) -> Vec<f32> {
    let mut t = vec![0.0; 3 * 224 * 224];
    fontatlas_embed::to_tensor(&Gray224(vec![v; 224 * 224]), &mut t);
    t
}

pub fn norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

pub fn cos(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>() / (norm(a) * norm(b))
}

pub fn pearson(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    sab / (saa.sqrt() * sbb.sqrt())
}

pub fn col(v: &[Vec<f32>], i: usize) -> Vec<f32> {
    v.iter().map(|x| x[i]).collect()
}

/// Dữ liệu tổng hợp: `centers` tâm ngẫu nhiên (LCG), mỗi mẫu = tâm + nhiễu, biên độ chiều j = 1/(1+j/32).
pub fn lcg_clusters(n: usize, dim: usize, centers: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut s = seed;
    let mut rnd = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((s >> 33) as f32) / (1u64 << 31) as f32 - 0.5
    };
    let cs: Vec<Vec<f32>> = (0..centers.max(1)).map(|_| (0..dim).map(|_| rnd()).collect()).collect();
    (0..n)
        .map(|i| {
            let c = &cs[i % cs.len()];
            (0..dim).map(|j| c[j] + rnd() / (1.0 + j as f32 / 32.0)).collect()
        })
        .collect()
}
