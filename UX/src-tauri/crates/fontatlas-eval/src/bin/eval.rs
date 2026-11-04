//! eval --fonts <DIR>... [--system-fonts] --similar <CSV> --pairing <CSV> [--reference <JSON> (mặc định: bản nhúng)]
//!      [--show-tune] [--no-expand]
//!      [--embed-model <vision.safetensors> [--whitening <BIN> (mặc định: bản nhúng)] [--embed-cache <JSON>]
//!       [--embed-weight <w> (bỏ qua sweep trên tune)]]
//! In bảng markdown cho split test và test-vi. Thoát 0 (chế độ báo cáo); CSV hỏng → 2.
//! Có --embed-model: nhúng ảnh mọi face, chọn EMBED_WEIGHT trên họ `tune` (family_split) bằng R@1+R@5 cùng họ,
//! rồi báo Similar test + cùng họ trên họ `test` ở ba chế độ: Số đo / Chỉ ảnh / Số đo + Hình ảnh.
use std::path::PathBuf;

use std::collections::HashMap;

use fontatlas_embed::{CandleEncoder, Whitening, family_split, sha256_file};
use fontatlas_eval::{
    Corpus, FamilyRecall, LabelSource, Outcome, SimilarReport, embed_corpus,
    image_only_same_family, load_pairs, load_similar, markdown, pairing_report, same_family,
    similar_report,
};
use fontatlas_index::{
    ExactEmbeddings, FaceKey, FaceProfile, Index, MAX_EMBED_WEIGHT, ReferenceTable, SCORER_VERSION,
    SuperfamilyTable,
};

struct MemExact(HashMap<FaceKey, Vec<f32>>);

impl ExactEmbeddings for MemExact {
    fn fetch(&self, k: &[FaceKey]) -> Vec<Option<Vec<f32>>> {
        k.iter().map(|x| self.0.get(x).cloned()).collect()
    }
}

fn key_str(k: &FaceKey) -> String {
    format!("{}:{}", k.hash, k.index)
}

/// Vector đã whiten của mọi face; đọc cache nếu khớp model + whitening, không thì tính và ghi cache.
fn embeddings(
    c: &Corpus,
    model: &std::path::Path,
    wh: &Whitening,
    cache: Option<&std::path::Path>,
) -> HashMap<FaceKey, Vec<f32>> {
    let model_sha = sha256_file(model).expect("đọc model");
    let tag = format!("{model_sha}|{}", wh.sha256);
    if let Some(p) = cache
        && let Ok(s) = std::fs::read_to_string(p)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&s)
        && v["tag"] == tag.as_str()
    {
        let by: HashMap<String, &FaceKey> = c
            .profiles
            .iter()
            .map(|p| (key_str(&p.key), &p.key))
            .collect();
        let m: HashMap<FaceKey, Vec<f32>> = v["vectors"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, x)| {
                Some((
                    (*by.get(k)?).clone(),
                    serde_json::from_value(x.clone()).ok()?,
                ))
            })
            .collect();
        eprintln!("embedding: {} vector từ cache {}", m.len(), p.display());
        return m;
    }
    let mut enc = CandleEncoder::load_best(model).expect("nạp model");
    let t = std::time::Instant::now();
    let (m, skipped) = embed_corpus(c, &mut enc, &|v| wh.apply(v)).expect("embed");
    eprintln!(
        "embedding: {} face ({} bỏ qua) trên {} trong {:.0} s",
        m.len(),
        skipped,
        enc.device_id(),
        t.elapsed().as_secs_f64()
    );
    if let Some(p) = cache {
        let vectors: serde_json::Map<String, serde_json::Value> = m
            .iter()
            .map(|(k, v)| (key_str(k), serde_json::json!(v)))
            .collect();
        let _ = std::fs::write(
            p,
            serde_json::json!({"tag": tag, "vectors": vectors}).to_string(),
        );
    }
    m
}

fn hits(r: &SimilarReport) -> (usize, usize, usize) {
    let n = |o: Outcome| r.rows.iter().filter(|x| x.1 == o).count();
    (
        n(Outcome::Hit),
        r.rows.len() - n(Outcome::NoData),
        n(Outcome::MustNotViolated),
    )
}

/// "đạt/có dữ liệu" không tính nhóm † (informed_by).
fn clean_hits(r: &SimilarReport) -> String {
    let rows: Vec<_> = r
        .rows
        .iter()
        .filter(|x| x.1 != Outcome::NoData && !r.informed.contains(&x.0))
        .collect();
    let h = rows.iter().filter(|x| x.1 == Outcome::Hit).count();
    format!("{h}/{}", rows.len())
}

fn pct(x: f32) -> String {
    format!("{:.1}%", x * 100.0)
}

fn row(name: &str, r: &FamilyRecall) -> String {
    format!(
        "| {name} | {} | {} | {} |\n",
        pct(r.r1),
        pct(r.r5),
        r.queries
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut fonts: Vec<PathBuf> = Vec::new();
    let (mut similar, mut pairing, mut reference) = (None, None, None);
    let (mut show_tune, mut expand) = (false, true);
    let (mut embed_model, mut whitening, mut embed_cache, mut embed_weight) = (
        None::<PathBuf>,
        None::<PathBuf>,
        None::<PathBuf>,
        None::<f32>,
    );
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
            "--embed-model" => {
                i += 1;
                embed_model = args.get(i).map(PathBuf::from);
            }
            "--whitening" => {
                i += 1;
                whitening = args.get(i).map(PathBuf::from);
            }
            "--embed-cache" => {
                i += 1;
                embed_cache = args.get(i).map(PathBuf::from);
            }
            "--embed-weight" => {
                i += 1;
                embed_weight = args.get(i).and_then(|x| x.parse().ok());
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
    let axes_ix = Index::new(
        corpus.profiles.clone(),
        reference.clone(),
        SuperfamilyTable::bundled(),
    );
    let tune_sets: Vec<_> = sets.iter().filter(|s| s.split == "tune").cloned().collect();
    let test_sets: Vec<_> = sets.into_iter().filter(|s| s.split == "test").collect();
    let mut extra = String::new();
    let ix = match &embed_model {
        None => axes_ix,
        Some(model) => {
            let wh = match &whitening {
                Some(p) => Whitening::from_bytes(&std::fs::read(p).expect("đọc whitening"))
                    .expect("whitening hỏng"),
                None => Whitening::bundled(),
            };
            let embs = embeddings(&corpus, model, &wh, embed_cache.as_deref());
            let profiles: Vec<FaceProfile> = corpus
                .profiles
                .iter()
                .cloned()
                .map(|mut p| {
                    p.embedding = embs.get(&p.key).cloned();
                    p
                })
                .collect();
            let mut ix = Index::new(profiles, reference, SuperfamilyTable::bundled());
            ix.set_exact_source(Box::new(MemExact(embs.clone())));
            let split = |p: &FaceProfile| family_split(&p.family);
            let all = |_: &FaceProfile| true;
            // Chọn trọng số trên họ tune (cùng họ) — similar_sets tune chỉ in để tham khảo.
            let chosen = match embed_weight {
                Some(w) => w.clamp(0.0, MAX_EMBED_WEIGHT),
                None => {
                    extra.push_str("### Chọn EMBED_WEIGHT trên tune\n\n| w | R@1 cùng họ (tune) | R@5 cùng họ (tune) | similar_sets tune đạt | must_not |\n|---|---|---|---|---|\n");
                    let mut best = (f32::MIN, 0.0f32);
                    for step in 0..=((MAX_EMBED_WEIGHT * 20.0).round() as usize) {
                        let w = step as f32 * 0.05;
                        ix.set_embed_weight(w);
                        let r = same_family(&ix, &corpus, &|p| split(p) == "tune", &all);
                        let (h, n, mn) = hits(&similar_report(&ix, &corpus, &tune_sets));
                        extra.push_str(&format!(
                            "| {w:.2} | {} | {} | {h}/{n} | {mn} |\n",
                            pct(r.r1),
                            pct(r.r5)
                        ));
                        if r.r1 + r.r5 > best.0 + 1e-6 {
                            best = (r.r1 + r.r5, w);
                        }
                    }
                    best.1
                }
            };
            ix.set_embed_weight(chosen);
            extra.push_str(&format!(
                "\nEMBED_WEIGHT đã chọn: **{chosen:.2}** (tối đa {MAX_EMBED_WEIGHT}).\n\n"
            ));
            // S4 trên họ test: ba chế độ, cùng truy vấn, gallery = cả corpus.
            let q_test = |p: &FaceProfile| split(p) == "test";
            let comb = same_family(&ix, &corpus, &q_test, &all);
            let axes = same_family(&axes_ix, &corpus, &q_test, &all);
            let img = image_only_same_family(&corpus, &embs, &comb.keys, &all);
            // Cùng điều kiện: chỉ truy vấn test có ảnh (chế độ "Chỉ ảnh" không có phương án dự phòng).
            let q_img = |p: &FaceProfile| q_test(p) && embs.contains_key(&p.key);
            let comb_i = same_family(&ix, &corpus, &q_img, &all);
            let axes_i = same_family(&axes_ix, &corpus, &q_img, &all);
            let img_i = image_only_same_family(&corpus, &embs, &comb_i.keys, &all);
            let (sa, sc) = (
                similar_report(&axes_ix, &corpus, &test_sets),
                similar_report(&ix, &corpus, &test_sets),
            );
            let ((ha, na, ma), (hc, nc, mc)) = (hits(&sa), hits(&sc));
            let (ca, cc) = (clean_hits(&sa), clean_hits(&sc));
            extra.push_str("### S4 — cùng họ, truy vấn thuộc họ `test` (gallery: cả corpus)\n\n| chế độ | R@1 | R@5 | truy vấn |\n|---|---|---|---|\n");
            extra.push_str(&row("Số đo", &axes));
            extra.push_str(&row("Chỉ ảnh", &img));
            extra.push_str(&row("Số đo + Hình ảnh", &comb));
            extra.push_str(&format!(
                "\nChỉ truy vấn test CÓ ảnh ({} / {}; {} truy vấn không có ảnh):\n\n| chế độ | R@1 | R@5 | truy vấn |\n|---|---|---|---|\n",
                comb_i.queries,
                comb.queries,
                comb.queries - comb_i.queries
            ));
            extra.push_str(&row("Số đo", &axes_i));
            extra.push_str(&row("Chỉ ảnh", &img_i));
            extra.push_str(&row("Số đo + Hình ảnh", &comb_i));
            extra.push_str(&format!(
                "\n| similar_sets test | nhóm đạt | không tính † | vi phạm must_not |\n|---|---|---|---|\n| Số đo | {ha}/{na} | {ca} | {ma} |\n| Số đo + Hình ảnh | {hc}/{nc} | {cc} | {mc} |\n\n"
            ));
            ix
        }
    };
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
    print!("{extra}");
    println!("{}", markdown(&sim, &blocks));
}
