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

/// File whitening "đơn vị": μ = 0, W = [I_dim; 0] (raw_dim × dim) — dùng cho test không cần fit.
pub fn identity_whitening_bytes(raw_dim: usize, dim: usize) -> Vec<u8> {
    let header = serde_json::json!({"version":"white-identity","raw_dim":raw_dim,"dim":dim,"shrink":0.0,"source":"test","split":"tune","n":0}).to_string();
    let mut b = b"FAWHITEN".to_vec();
    b.extend_from_slice(&(header.len() as u32).to_le_bytes());
    b.extend_from_slice(header.as_bytes());
    for _ in 0..raw_dim {
        b.extend_from_slice(&0f32.to_le_bytes());
    }
    for r in 0..raw_dim {
        for c in 0..dim {
            b.extend_from_slice(&(if r == c { 1f32 } else { 0f32 }).to_le_bytes());
        }
    }
    b
}

// ---- helper cho worker/source: store tạm có font thật đã rescan ----
use fontatlas_store::{FaceKey, SourceKind, Store};

/// Thư mục tạm: `fonts/` chứa bản sao fixture, `data/` là store đã rescan.
pub fn store_dir_with(fixtures: &[(&str, usize)]) -> tempfile::TempDir {
    store_dir_from(
        fixtures
            .iter()
            .map(|(f, pad)| {
                let mut b = fixture_bytes(f);
                b.extend(std::iter::repeat_n(0u8, *pad)); // thêm byte 0 cuối file → hash khác, font vẫn đọc được
                (format!("{pad}_{f}"), b)
            })
            .collect(),
    )
}

pub fn store_dir_from(files: Vec<(String, Vec<u8>)>) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let fonts = d.path().join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for (name, b) in files {
        std::fs::write(fonts.join(name), b).unwrap();
    }
    let mut s = Store::open(&d.path().join("data")).unwrap().0;
    let id = s.add_source(SourceKind::Directory, "t", Some(&fonts)).unwrap();
    s.rescan_source(id).unwrap();
    d
}

pub fn open_data(d: &tempfile::TempDir) -> Store {
    Store::open(&d.path().join("data")).unwrap().0
}

pub fn add_font_and_rescan(dir: &std::path::Path, fixture: &str) {
    std::fs::copy(self::fixture(fixture), dir.join("fonts").join(fixture)).unwrap();
    let mut s = Store::open(&dir.join("data")).unwrap().0;
    let id = s.sources().unwrap()[0].id;
    s.rescan_source(id).unwrap();
}

pub fn key_of(s: &Store, family: &str) -> FaceKey {
    s.conn()
        .query_row(
            "SELECT fi.hash, fa.face_index FROM faces fa JOIN files fi USING(file_id) WHERE fa.family_name=?1",
            [family],
            |r| {
                let h: Vec<u8> = r.get(0)?;
                Ok(FaceKey { hash: h.try_into().unwrap(), face_index: r.get::<_, i64>(1)? as u32 })
            },
        )
        .unwrap()
}

pub fn state_of(s: &Store, k: FaceKey, raw: &str) -> (String, Option<String>) {
    s.conn()
        .query_row(
            "SELECT status, error_code FROM pipeline_state WHERE hash=?1 AND face_index=?2 AND stage='embedding' AND version=?3",
            (&k.hash[..], k.face_index, raw),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
}

pub fn count_status(s: &Store, status: &str) -> i64 {
    s.conn()
        .query_row("SELECT count(*) FROM pipeline_state WHERE stage='embedding' AND status=?1", [status], |r| r.get(0))
        .unwrap()
}

// ---- encoder thật tí hon (candle) cho test ----
use candle_transformers::models::siglip::VisionConfig;
use fontatlas_embed::{Backend, CandleEncoder, Encode};

/// Phải khớp config trong tools/embed/make_tiny_siglip.py.
pub fn tiny_vision_config() -> VisionConfig {
    VisionConfig {
        hidden_size: 32,
        intermediate_size: 64,
        num_hidden_layers: 2,
        num_attention_heads: 4,
        num_channels: 3,
        image_size: 224,
        patch_size: 16,
        hidden_act: candle_nn::Activation::GeluPytorchTanh,
        layer_norm_eps: 1e-6,
    }
}

/// SigLIP tí hon (32 chiều) lặp thành 768 chiều để khớp whitening 768→128 của worker.
pub struct Tiny768(pub CandleEncoder);

impl Encode for Tiny768 {
    fn raw_dim(&self) -> usize {
        768
    }
    fn encode(&mut self, t: &[Vec<f32>], c: &std::sync::atomic::AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(self.0.encode(t, c)?.into_iter().map(|v| v.repeat(768 / v.len())).collect())
    }
}

pub fn tiny_768() -> Box<dyn Encode> {
    Box::new(Tiny768(CandleEncoder::load_with(&data("tiny_siglip.safetensors"), Backend::Cpu, &tiny_vision_config()).unwrap()))
}
