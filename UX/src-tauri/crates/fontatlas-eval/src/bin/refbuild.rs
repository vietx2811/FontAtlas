//! refbuild --fonts <DIR>... --out <JSON> --version ref-v1 --source "<mô tả>" [--terminal-check]
//! Dựng thư viện tham chiếu từ kho OFL/Apache: phân vị ở vị trí chuẩn (không mở rộng VF),
//! D_fam trên kho đã mở rộng VF. --terminal-check: Spearman giữa terminal_c_normal và contrast.
use std::path::{Path, PathBuf};

use fontatlas_core::{
    FontFile, LicenseClass, MeasureAt, axes::detail::terminal_c_normal, geom::GlyphCtx,
    standard_location,
};
use fontatlas_eval::{Corpus, build_reference, d_fam, spearman};
use fontatlas_index::{FaceProfile, Thresholds};
use rayon::prelude::*;

const EXTS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];

fn walk(p: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(p) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTS.contains(&e.to_ascii_lowercase().as_str()))
        {
            out.push(p);
        }
    }
}

/// (terminal, contrast) của mọi face mã nguồn mở có chữ thường, đo ở vị trí chuẩn.
fn terminal_pairs(dirs: &[PathBuf]) -> Vec<(f32, f32)> {
    let mut files = Vec::new();
    for d in dirs {
        walk(d, &mut files);
    }
    files
        .par_iter()
        .flat_map_iter(|p| {
            let mut out = Vec::new();
            let Ok(f) = FontFile::open(p) else { return out };
            if f.validate().is_err() {
                return out;
            }
            let Ok(faces) = f.faces() else { return out };
            for face in faces {
                let md = face.metadata();
                if !matches!(md.license_class, LicenseClass::Ofl | LicenseClass::Apache) {
                    continue;
                }
                let a = face.measure(MeasureAt::Standard);
                if a.contrast.method != "o_stroke_width_max_min" {
                    continue;
                }
                let ctx = GlyphCtx::new(face.font().clone(), standard_location(face.font()));
                if let (Some(t), Some(c)) = (terminal_c_normal(&ctx).value, a.contrast.value) {
                    out.push((t, c));
                }
            }
            out
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut fonts: Vec<PathBuf> = Vec::new();
    let (mut out, mut version, mut source) = (None, "ref-v1".to_string(), String::new());
    let mut terminal_check = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fonts" => {
                while i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    i += 1;
                    fonts.push(PathBuf::from(&args[i]));
                }
            }
            "--out" => {
                i += 1;
                out = args.get(i).map(PathBuf::from);
            }
            "--version" => {
                i += 1;
                version = args.get(i).cloned().unwrap_or(version);
            }
            "--source" => {
                i += 1;
                source = args.get(i).cloned().unwrap_or_default();
            }
            "--terminal-check" => terminal_check = true,
            other => {
                eprintln!("tham số lạ: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    let Some(out) = out else {
        eprintln!("cần --out <JSON>");
        std::process::exit(2);
    };
    let open = |c: Corpus| -> Vec<FaceProfile> {
        c.profiles.into_iter().filter(|p| p.open_license).collect()
    };
    let std_corpus = Corpus::load(&fonts, false);
    let failed = std_corpus.failed_files;
    let std_profiles = open(std_corpus);
    let mut table = build_reference(&std_profiles, &version, &source);
    let expanded = open(Corpus::load(&fonts, true));
    if let Some((d, n)) = d_fam(&expanded, &Thresholds::v1(), &table) {
        table.d_fam = d;
        table.d_fam_pairs = n;
    }
    println!(
        "faces={} (file lỗi {failed}) · bảng (trục, method)={} · d_fam={:.3} ({} cặp, kho mở rộng VF {} profile)",
        table.faces,
        table.quantiles.len(),
        table.d_fam,
        table.d_fam_pairs,
        expanded.len()
    );
    for q in &table.quantiles {
        println!("  {} / {} n={}", q.axis, q.method, q.n);
    }
    if terminal_check {
        let pairs = terminal_pairs(&fonts);
        let (t, c): (Vec<f32>, Vec<f32>) = pairs.iter().copied().unzip();
        match spearman(&t, &c) {
            Some(r) => println!("terminal vs contrast: n={}, spearman r={r:.3}", pairs.len()),
            None => println!("terminal vs contrast: n={}, spearman r=None", pairs.len()),
        }
    }
    let json = serde_json::to_string(&table).unwrap_or_else(|e| {
        eprintln!("json: {e}");
        std::process::exit(2)
    });
    if let Err(e) = std::fs::write(&out, json) {
        eprintln!("ghi {}: {e}", out.display());
        std::process::exit(2);
    }
}
