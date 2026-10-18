//! eval --fonts <DIR>... [--system-fonts] --similar <CSV> --pairing <CSV> [--reference <JSON> (mặc định: bản nhúng)]
//!      [--show-tune] [--no-expand]
//! In bảng markdown cho split test và test-vi. Thoát 0 (chế độ báo cáo); CSV hỏng → 2.
use std::path::PathBuf;

use fontatlas_eval::{
    Corpus, LabelSource, load_pairs, load_similar, markdown, pairing_report, similar_report,
};
use fontatlas_index::{Index, ReferenceTable, SCORER_VERSION, SuperfamilyTable};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut fonts: Vec<PathBuf> = Vec::new();
    let (mut similar, mut pairing, mut reference) = (None, None, None);
    let (mut show_tune, mut expand) = (false, true);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fonts" => {
                while i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    i += 1;
                    fonts.push(PathBuf::from(&args[i]));
                }
            }
            "--system-fonts" => {
                for d in [
                    "/System/Library/Fonts",
                    "/System/Library/Fonts/Supplemental",
                    "/Library/Fonts",
                ] {
                    if std::path::Path::new(d).exists() {
                        fonts.push(d.into());
                    }
                }
            }
            "--similar" => {
                i += 1;
                similar = args.get(i).map(PathBuf::from);
            }
            "--pairing" => {
                i += 1;
                pairing = args.get(i).map(PathBuf::from);
            }
            "--reference" => {
                i += 1;
                reference = args.get(i).map(PathBuf::from);
            }
            "--show-tune" => show_tune = true,
            "--no-expand" => expand = false,
            other => {
                eprintln!("tham số lạ: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    let (Some(similar), Some(pairing)) = (similar, pairing) else {
        eprintln!("cần --similar <CSV> và --pairing <CSV>");
        std::process::exit(2);
    };
    let sets = load_similar(&similar).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let rows = load_pairs(&pairing).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let reference = match reference {
        Some(p) => std::fs::read_to_string(&p)
            .map_err(|e| e.to_string())
            .and_then(|s| ReferenceTable::from_json(&s).map_err(|e| e.to_string()))
            .unwrap_or_else(|e| {
                eprintln!("reference {}: {e}", p.display());
                std::process::exit(2)
            }),
        // Mặc định: bảng nhúng reference-v1.
        None => ReferenceTable::bundled(),
    };
    let corpus = Corpus::load(&fonts, expand);
    eprintln!(
        "corpus: {} face, {} file lỗi",
        corpus.profiles.len(),
        corpus.failed_files
    );
    println!(
        "scorer {SCORER_VERSION} · reference {} · analyzer {}",
        reference.version,
        fontatlas_core::ANALYZER_VERSION
    );
    let ix = Index::new(
        corpus.profiles.clone(),
        reference,
        SuperfamilyTable::bundled(),
    );
    let test_sets: Vec<_> = sets.into_iter().filter(|s| s.split == "test").collect();
    let sim = similar_report(&ix, &corpus, &test_sets);
    let mut splits = vec!["test", "test-vi"];
    if show_tune {
        splits.push("tune");
    }
    let mut blocks = Vec::new();
    for sp in splits {
        for src in [LabelSource::Expected, LabelSource::Owner] {
            blocks.push((
                sp.to_string(),
                src,
                pairing_report(&ix, &corpus, &rows, sp, src),
            ));
        }
    }
    println!("{}", markdown(&sim, &blocks));
}
