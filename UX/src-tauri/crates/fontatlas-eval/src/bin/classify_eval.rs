//! classify_eval --gf <google-fonts> --tags <families.csv> [--split tune|test|all] [--out <md>]
//! Phân loại mặt Regular của từng họ GF bằng `fontatlas_core::classify`, so với ground truth (tag ≥ 50 + category).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fontatlas_core::classify::{Class, ClassifyInput, classify};
use fontatlas_eval::classify_gt::{Truth, load_tags, truth};
use rayon::prelude::*;

struct Fam {
    name: String,
    category: String,
    file: PathBuf,
}

fn field(block: &str, key: &str) -> Option<String> {
    block.lines().find_map(|l| l.trim().strip_prefix(&format!("{key}:")).map(|v| v.trim().trim_matches('"').to_string()))
}

/// Chọn file của mặt normal weight 400 (hoặc normal đầu tiên, hoặc đầu tiên).
fn read_family(dir: &Path) -> Option<Fam> {
    let pb = std::fs::read_to_string(dir.join("METADATA.pb")).ok()?;
    let name = field(&pb, "name")?;
    let category = pb.lines().find_map(|l| l.strip_prefix("category:").map(|v| v.trim().trim_matches('"').to_string()))?;
    let mut faces: Vec<(String, u32, String)> = Vec::new();
    for b in pb.split("fonts {").skip(1) {
        let b = b.split('}').next().unwrap_or("");
        let (Some(style), Some(w), Some(f)) = (field(b, "style"), field(b, "weight"), field(b, "filename")) else { continue };
        faces.push((style, w.parse().unwrap_or(400), f));
    }
    let pick = faces.iter().find(|f| f.0 == "normal" && f.1 == 400).or_else(|| faces.iter().find(|f| f.0 == "normal")).or(faces.first())?;
    let file = dir.join(&pick.2);
    file.exists().then_some(Fam { name, category, file })
}

fn main() {
    let mut a = std::env::args().skip(1);
    let (mut gf, mut tags, mut split, mut out, mut dump) = (None, None, "all".to_string(), None, None);
    while let Some(k) = a.next() {
        match k.as_str() {
            "--gf" => gf = a.next(),
            "--tags" => tags = a.next(),
            "--split" => split = a.next().unwrap_or_default(),
            "--out" => out = a.next(),
            "--dump" => dump = a.next(),
            _ => {}
        }
    }
    let (Some(gf), Some(tags)) = (gf, tags) else {
        eprintln!("dùng: classify_eval --gf <google-fonts> --tags <families.csv> [--split tune|test|all] [--out md]");
        std::process::exit(2);
    };
    let tagmap = load_tags(&std::fs::read_to_string(&tags).expect("đọc families.csv"));
    let mut fams = Vec::new();
    for lic in ["ofl", "apache", "ufl"] {
        if let Ok(rd) = std::fs::read_dir(Path::new(&gf).join(lic)) {
            fams.extend(rd.flatten().filter_map(|e| read_family(&e.path())));
        }
    }
    fams.retain(|f| split == "all" || fontatlas_embed::family_split(&f.name) == split);
    let results: Vec<(String, Truth, fontatlas_core::classify::Classification, ClassifyInput)> = fams
        .par_iter()
        .filter_map(|f| {
            let gt = truth(&f.category, tagmap.get(&f.name).map(Vec::as_slice).unwrap_or(&[]))?;
            let r = fontatlas_core::analyze_path(&f.file).ok()?;
            let face = r.faces.first()?;
            let inp = ClassifyInput::from_json(&serde_json::to_value(&face.metadata).ok()?, &serde_json::to_value(&face.anatomy).ok()?);
            let mut row = format!("{},{},{}", f.name.replace(',', " "), gt.primary.label(), gt.sub.iter().next().copied().unwrap_or(""));
            let m = &inp.m;
            for x in [m.serif_foot, m.contrast, m.stress_abs, m.slant_abs, m.aperture, m.width, m.width_cv, m.spacing, m.stem, m.x_height_em, m.a_storeys, m.g_storeys, m.connect, m.bounce, m.terminal, m.aperture_e, m.counter] {
                row.push_str(&format!(",{}", x.map_or(String::new(), |v| format!("{v:.4}"))));
            }
            let md = serde_json::to_value(&face.metadata).ok()?;
            row.push_str(&format!(",{},{},{},{}", md["panose"][0], md["family_class"][0], md["is_fixed_pitch"], gt.accept.iter().map(|c| c.label()).collect::<Vec<_>>().join("|")));
            let c = classify(&inp);
            let sc = |k: Class| c.signals.iter().filter(|s| s.vote == k).map(|s| s.weight).sum::<f32>();
            row.push_str(&format!(",{},{},{:.3},{:.2},{:.2},{:.2},{:.2},{:.2},{},{}", c.class.label(), c.subclass.clone().unwrap_or_default(), c.confidence, sc(Class::Serif), sc(Class::Sans), sc(Class::Script), sc(Class::Display), sc(Class::Mono), fontatlas_embed::family_split(&f.name), gt.sub.iter().next().copied().unwrap_or("")));
            Some((row, gt, c, inp))
        })
        .collect();
    if let Some(d) = &dump {
        let mut rows: Vec<&str> = results.iter().map(|r| r.0.as_str()).collect();
        rows.sort();
        std::fs::write(d, format!("name,gt,gtsub,serif_foot,contrast,stress,slant,aperture,width,width_cv,spacing,stem,xh,a_st,g_st,connect,bounce,terminal,aperture_e,counter,panose0,sfc,fixed,accept,pred,predsub,conf,s_serif,s_sans,s_script,s_display,s_mono,split,gtsub2\n{}\n", rows.join("\n"))).expect("ghi dump");
    }
    let mut md = String::new();
    let n = results.len();
    let (mut ok, mut unc, mut wrong) = (0, 0, 0);
    let mut cm: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut wrong_list = Vec::new();
    for (name, gt, c, _) in &results {
        let pred = c.class;
        *cm.entry((gt.primary.label(), pred.label())).or_default() += 1;
        if pred == Class::Uncertain {
            unc += 1;
        } else if gt.accept.contains(&pred) {
            ok += 1;
        } else {
            wrong += 1;
            wrong_list.push(format!("{name}: GT {:?} → {}", gt.primary.label(), pred.label()));
        }
    }
    let pct = |x: usize, d: usize| if d == 0 { 0.0 } else { 100.0 * x as f64 / d as f64 };
    md.push_str(&format!(
        "## Class — tập `{split}` ({n} họ có đáp án)\n\n| đúng | Uncertain | sai | đúng/tổng | đúng/đã gán |\n|---|---|---|---|---|\n| {ok} | {unc} | {wrong} | {:.1}% | {:.1}% |\n\n",
        pct(ok, n),
        pct(ok, ok + wrong)
    ));
    let preds = ["Serif", "Sans", "Mono", "Script", "Display", "Blackletter", "Symbol", "Uncertain"];
    md.push_str("Hàng = đáp án (primary), cột = dự đoán.\n\n| GT \\ dự đoán |");
    for p in preds {
        md.push_str(&format!(" {p} |"));
    }
    md.push_str("\n|---|");
    md.push_str(&"---|".repeat(preds.len()));
    md.push('\n');
    for g in ["Serif", "Sans", "Mono", "Script", "Display", "Blackletter"] {
        md.push_str(&format!("| {g} |"));
        for p in preds {
            md.push_str(&format!(" {} |", cm.get(&(g, p)).copied().unwrap_or(0)));
        }
        md.push('\n');
    }
    // precision / coverage theo ngưỡng confidence (đoán thô = class hoặc candidate)
    let raw = |c: &fontatlas_core::classify::Classification| c.candidate.unwrap_or(c.class);
    let curve = |title: &str, pick: &dyn Fn(&Truth) -> bool| -> String {
        let rows: Vec<_> = results.iter().filter(|r| pick(&r.1)).collect();
        let total = rows.len();
        let mut o = format!("\n## {title} ({total} họ)\n\n| ngưỡng confidence | đã gán | độ phủ | đúng | precision (đúng/đã gán) |\n|---|---|---|---|---|\n");
        for th in [0.0f32, 0.5, 0.6, 0.7, 0.75, 0.8, 0.85, 0.88, 0.9, 0.93, 0.95] {
            let sel: Vec<_> = rows.iter().filter(|(_, _, c, _)| raw(c) != Class::Uncertain && c.confidence >= th).collect();
            let good = sel.iter().filter(|(_, gt, c, _)| gt.accept.contains(&raw(c))).count();
            o.push_str(&format!("| {th:.2} | {} | {:.1}% | {} | {:.1}% |\n", sel.len(), pct(sel.len(), total), good, pct(good, sel.len())));
        }
        o
    };
    use Class::{Mono, Sans, Serif};
    md.push_str(&curve("Precision/coverage — toàn bộ", &|_| true));
    md.push_str(&curve("Precision/coverage — chỉ nhóm văn bản (đáp án Serif/Sans/Mono)", &|g| matches!(g.primary, Serif | Sans | Mono)));
    md.push_str(&curve("Precision/coverage — theo hình dạng (loại họ GF chỉ có nhãn Display theo mục đích, không tag hình dạng)", &|g| !(g.primary == Class::Display && g.accept.len() == 1)));
    // subclass
    let (mut sn, mut sok, mut sunc, mut swrong) = (0, 0, 0, 0);
    let mut scm: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (_, gt, c, inp) in &results {
        // subclass đo riêng khỏi ngưỡng class: dùng class đoán thô (candidate) nếu đã bị hạ về Uncertain
        let rc = c.candidate.unwrap_or(c.class);
        if gt.sub.is_empty() || !gt.accept.contains(&rc) {
            continue;
        }
        sn += 1;
        let sub = c.subclass.clone().or_else(|| fontatlas_core::classify::subclass_for(inp, rc));
        let pred = sub.clone().unwrap_or_else(|| "uncertain".into());
        let truth = gt.sub.iter().next().copied().unwrap_or("").to_string();
        *scm.entry((truth.clone(), pred.clone())).or_default() += 1;
        if sub.is_none() {
            sunc += 1;
        } else if gt.sub.contains(pred.as_str()) {
            sok += 1;
        } else {
            swrong += 1;
        }
    }
    md.push_str(&format!(
        "\n## Subclass — họ đã đúng class và có nhãn con duy nhất ({sn})\n\n| đúng | uncertain | sai | đúng/tổng | đúng/đã gán |\n|---|---|---|---|---|\n| {sok} | {sunc} | {swrong} | {:.1}% | {:.1}% |\n\n| GT subclass | dự đoán | số họ |\n|---|---|---|\n",
        pct(sok, sn),
        pct(sok, sok + swrong)
    ));
    for ((g, p), c) in &scm {
        md.push_str(&format!("| {g} | {p} | {c} |\n"));
    }
    md.push_str("\n### Sai class (tối đa 40)\n\n");
    wrong_list.sort();
    for l in wrong_list.iter().take(40) {
        md.push_str(&format!("- {l}\n"));
    }
    print!("{md}");
    if let Some(o) = out {
        std::fs::write(o, md).expect("ghi báo cáo");
    }
}
