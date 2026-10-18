//! Sinh dữ liệu cho harness S2.
//!   dump_previews --out <dir> <font dir>...        → <dir>/<id>.png + manifest.json
//!   dump_previews one <path> <face_index> <px> <text>  → PNG ra stdout
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use fontatlas_core::FontFile;
use fontatlas_preview::{DEFAULT_SAMPLE, render_face};
use rayon::prelude::*;

/// (path, faceIndex, family, hashHex, png)
type Row = (String, u32, String, String, Vec<u8>);

const EXT: [&str; 6] = ["ttf", "otf", "ttc", "otc", "woff", "woff2"];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| EXT.contains(&x.to_ascii_lowercase().as_str())) {
            out.push(p);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("one") {
        let [_, path, idx, px, text] = &args[..] else {
            eprintln!("dùng: dump_previews one <path> <face_index> <px> <text>");
            std::process::exit(2)
        };
        let file = FontFile::open(Path::new(path)).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
        let faces = file.faces().unwrap_or_default();
        let idx: u32 = idx.parse().unwrap_or(0);
        let Some(face) = faces.iter().find(|f| f.index() == idx) else { std::process::exit(1) };
        match render_face(face, text, px.parse().unwrap_or(48)) {
            Ok(r) => { let _ = std::io::stdout().write_all(&r.png); }
            Err(e) => { eprintln!("{}", e.code()); std::process::exit(1) }
        }
        return;
    }
    let Some(pos) = args.iter().position(|a| a == "--out") else {
        eprintln!("dùng: dump_previews --out <dir> <font dir>...");
        std::process::exit(2)
    };
    let out = PathBuf::from(&args[pos + 1]);
    let dirs: Vec<&String> = args.iter().enumerate().filter(|(i, _)| *i != pos && *i != pos + 1).map(|(_, a)| a).collect();
    let _ = std::fs::create_dir_all(&out);
    let mut files = Vec::new();
    for d in dirs { walk(Path::new(d), &mut files); }
    files.sort();

    let t = Instant::now();
    let errors = AtomicUsize::new(0);
    // theo thứ tự file → id ổn định
    let rendered: Vec<Vec<Row>> = files
        .par_iter()
        .map(|p| {
            let Ok(file) = FontFile::open(p) else { errors.fetch_add(1, Ordering::Relaxed); return vec![] };
            let Ok(faces) = file.faces() else { errors.fetch_add(1, Ordering::Relaxed); return vec![] };
            faces.iter().filter_map(|f| match render_face(f, DEFAULT_SAMPLE, 48) {
                Ok(r) => Some((p.display().to_string(), f.index(), f.metadata().family.unwrap_or_default(), file.hash_hex(), r.png)),
                Err(_) => { errors.fetch_add(1, Ordering::Relaxed); None }
            }).collect()
        })
        .collect();
    let mut manifest = Vec::new();
    for (id, (path, face_index, family, hash_hex, png)) in rendered.into_iter().flatten().enumerate() {
        let _ = std::fs::write(out.join(format!("{id}.png")), &png);
        manifest.push(serde_json::json!({ "id": id, "path": path, "faceIndex": face_index, "family": family, "hashHex": hash_hex }));
    }
    let _ = std::fs::write(out.join("manifest.json"), serde_json::to_string(&manifest).unwrap_or_default());
    let n = manifest.len().max(1);
    eprintln!("faces={} errors={} ms/face={:.3} (song song)", manifest.len(), errors.load(Ordering::Relaxed), t.elapsed().as_secs_f64() * 1e3 / n as f64);
}
