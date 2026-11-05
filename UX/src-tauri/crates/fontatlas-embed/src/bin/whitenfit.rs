//! Dựng file whitening từ kho font mở (Google Fonts OFL/Apache), chỉ nửa họ "tune".
//! Tuỳ chọn --report-test: đo thêm R@1/R@5 cùng họ (chỉ ảnh) trên nửa "test", thô và đã whiten.
//!
//! whitenfit --fonts <DIR> --model <model.safetensors> --out <BIN> --source "<mô tả nguồn>" [--report-test]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use fontatlas_core::{FontFile, LicenseClass, standard_location};
use fontatlas_embed::{BATCH, DIM, EmbedError, CandleEncoder, Encode, RAW_DIM, Whitening, family_split, fit, render_input, to_tensor};
use rayon::prelude::*;

struct Face {
    family: String,
    split: &'static str,
    img: fontatlas_embed::Gray224,
}

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1).cloned())
}

fn peak_rss_mb() -> f64 {
    let mut u: libc_rusage = unsafe { std::mem::zeroed() };
    unsafe { getrusage(0, &mut u) };
    u.ru_maxrss as f64 / 1e6 // macOS: byte
}

#[repr(C)]
struct libc_rusage {
    ru_utime: [i64; 2],
    ru_stime: [i64; 2],
    ru_maxrss: i64,
    _rest: [i64; 13],
}
unsafe extern "C" {
    fn getrusage(who: i32, usage: *mut libc_rusage) -> i32;
}

/// R@1/R@5/R@10 cùng họ: với mỗi face thuộc họ có ≥ 2 face, hạng của face cùng họ đầu tiên.
fn recall(vecs: &[Vec<f32>], fam: &[String]) -> (f64, f64, f64, usize) {
    let mut cnt: HashMap<&str, usize> = HashMap::new();
    fam.iter().for_each(|f| *cnt.entry(f).or_default() += 1);
    let n = vecs.len();
    let (mut r1, mut r5, mut r10, mut q) = (0, 0, 0, 0);
    for i in 0..n {
        if cnt[fam[i].as_str()] < 2 {
            continue;
        }
        q += 1;
        let mut s: Vec<(f32, usize)> = (0..n).filter(|j| *j != i).map(|j| (vecs[i].iter().zip(&vecs[j]).map(|(a, b)| a * b).sum(), j)).collect();
        s.sort_by(|a, b| b.0.total_cmp(&a.0));
        if let Some(rank) = s.iter().position(|(_, j)| fam[*j] == fam[i]) {
            r1 += (rank < 1) as usize;
            r5 += (rank < 5) as usize;
            r10 += (rank < 10) as usize;
        }
    }
    let d = q.max(1) as f64;
    (r1 as f64 / d, r5 as f64 / d, r10 as f64 / d, q)
}

fn l2(v: &[f32]) -> Option<Vec<f32>> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    (n > 0.0 && n.is_finite()).then(|| v.iter().map(|x| x / n).collect())
}

fn main() -> Result<(), EmbedError> {
    let fonts = PathBuf::from(arg("--fonts").expect("--fonts"));
    let model = PathBuf::from(arg("--model").expect("--model"));
    let out = PathBuf::from(arg("--out").expect("--out"));
    let source = arg("--source").expect("--source");
    let report = std::env::args().any(|a| a == "--report-test");
    let wversion = arg("--version").unwrap_or_else(|| "white-v2".into());
    let compare_cpu: usize = arg("--compare-cpu").and_then(|n| n.parse().ok()).unwrap_or(0);

    let t0 = Instant::now();
    let files = fontatlas_store::walk_fonts(&fonts);
    // Render song song; chỉ font OFL/Apache; nửa test chỉ khi --report-test.
    let rendered: Vec<Result<Face, ()>> = files
        .par_iter()
        .flat_map_iter(|f| {
            let mut v = Vec::new();
            let Ok(bytes) = std::fs::read(&f.path) else { return v };
            let Ok(ff) = FontFile::from_bytes(bytes) else { return v };
            let Ok(faces) = ff.faces() else { return v };
            for face in faces {
                let md = face.metadata();
                if !matches!(md.license_class, LicenseClass::Ofl | LicenseClass::Apache) {
                    continue;
                }
                let family = md.typographic_family.or(md.family).unwrap_or_default();
                let split = family_split(&family);
                if split == "test" && !report {
                    continue;
                }
                let font = face.font();
                v.push(match render_input(font, &standard_location(font)) {
                    Ok(img) => Ok(Face { family, split, img }),
                    Err(_) => Err(()),
                });
            }
            v
        })
        .collect();
    let no_glyphs = rendered.iter().filter(|r| r.is_err()).count();
    let faces: Vec<Face> = rendered.into_iter().flatten().collect();
    let t_render = t0.elapsed().as_secs_f64();
    eprintln!("files {} faces {} (render lỗi/no_glyphs {no_glyphs}) render {t_render:.1}s", files.len(), faces.len());

    let mut enc = CandleEncoder::load_best(&model)?;
    let t1 = Instant::now();
    let mut raw: Vec<Vec<f32>> = Vec::with_capacity(faces.len());
    let stop = AtomicBool::new(false);
    for chunk in faces.chunks(BATCH) {
        let ts: Vec<Vec<f32>> = chunk
            .iter()
            .map(|f| {
                let mut t = vec![0.0; 3 * 224 * 224];
                to_tensor(&f.img, &mut t);
                t
            })
            .collect();
        raw.extend(enc.encode(&ts, &stop)?);
    }
    let ms_per = t1.elapsed().as_secs_f64() * 1e3 / faces.len().max(1) as f64;
    // Cùng ảnh trên CPU: kiểm vector CPU ↔ thiết bị chính có dùng chung version được không.
    let mut cpu_cos = serde_json::Value::Null;
    if compare_cpu > 0 && enc.device_id() != "cpu" {
        let mut cpu = fontatlas_embed::CandleEncoder::load(&model, fontatlas_embed::Backend::Cpu)?;
        let n = compare_cpu.min(faces.len());
        let mut cs = Vec::with_capacity(n);
        for (ci, chunk) in faces[..n].chunks(BATCH).enumerate() {
            let ts: Vec<Vec<f32>> = chunk
                .iter()
                .map(|f| {
                    let mut t = vec![0.0; 3 * 224 * 224];
                    to_tensor(&f.img, &mut t);
                    t
                })
                .collect();
            for (j, v) in cpu.encode(&ts, &stop)?.iter().enumerate() {
                let r = &raw[ci * BATCH + j];
                let dot: f32 = v.iter().zip(r).map(|(a, b)| a * b).sum();
                let nn = v.iter().map(|a| a * a).sum::<f32>().sqrt() * r.iter().map(|a| a * a).sum::<f32>().sqrt();
                cs.push(dot / nn);
            }
        }
        let min = cs.iter().cloned().fold(f32::INFINITY, f32::min);
        cpu_cos = serde_json::json!({"n": n, "min": min, "mean": cs.iter().sum::<f32>() / cs.len().max(1) as f32});
    }
    let tune: Vec<Vec<f32>> = faces.iter().zip(&raw).filter(|(f, _)| f.split == "tune").map(|(_, r)| r.clone()).collect();
    let mut w = fit(&tune, DIM, 0.1)?;
    let header = serde_json::json!({"version":wversion,"raw_version":fontatlas_embed::raw_version(&fontatlas_embed::sha256_file(&model)?),"raw_dim":RAW_DIM,"dim":DIM,"shrink":0.1,"source":source,"split":"tune","n":tune.len()});
    let bytes = w.to_bytes(&header);
    std::fs::write(&out, &bytes)?;
    w = Whitening::from_bytes(&bytes)?;
    let mut summary = serde_json::json!({
        "files": files.len(), "faces": faces.len(), "tune_faces": tune.len(), "no_glyphs": no_glyphs,
        "render_s": t_render, "encode_ms_per_face": ms_per, "device": enc.device_id(), "cpu_vs_device_cos": cpu_cos,
        "whitening": {"version": w.version, "sha256": w.sha256, "bytes": bytes.len()},
    });
    if report {
        let (mut fam, mut rv, mut wv) = (Vec::new(), Vec::new(), Vec::new());
        for (f, r) in faces.iter().zip(&raw) {
            if f.split != "test" {
                continue;
            }
            if let (Some(a), Some(b)) = (l2(r), w.apply(r)) {
                fam.push(f.family.clone());
                rv.push(a);
                wv.push(b);
            }
        }
        let (r1, r5, r10, q) = recall(&rv, &fam);
        let (w1, w5, w10, _) = recall(&wv, &fam);
        summary["test"] = serde_json::json!({"faces": fam.len(), "queries": q,
            "raw": {"r1": r1, "r5": r5, "r10": r10}, "whiten": {"r1": w1, "r5": w5, "r10": w10}});
    }
    summary["peak_rss_mb"] = serde_json::json!(peak_rss_mb());
    summary["total_s"] = serde_json::json!(t0.elapsed().as_secs_f64());
    println!("{summary}");
    Ok(())
}
