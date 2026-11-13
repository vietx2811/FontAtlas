//! pairing_eval --gf <google-fonts> --gold <pairing_gold.csv> [--out <md>]
//! Kiểm mô hình G4 (`fontatlas_core::pairing`) trên cặp/vai vàng. Mặt đại diện: file .ttf đầu tiên không Italic của `ofl/<gf_dir>`.
use std::path::{Path, PathBuf};

use fontatlas_core::classify::Class;
use fontatlas_core::pairing::{Fit, PairCtx, PairStyle, Role, Traits, family_root, pair_style, role_fit, traits_from_face};

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

fn pick_file(dir: &Path) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            (n.ends_with(".ttf") || n.ends_with(".otf")) && !n.contains("Italic")
        })
        .collect();
    v.sort();
    // Ưu tiên "Regular" rồi bản biến thiên.
    v.iter().find(|p| p.to_string_lossy().contains("Regular")).cloned().or_else(|| v.into_iter().next())
}

fn load(gf: &Path, dir: &str) -> Option<(Traits, String)> {
    let f = pick_file(&gf.join("ofl").join(dir))?;
    let r = fontatlas_core::analyze_path(&f).ok()?;
    let face = r.faces.first()?;
    if std::env::var_os("PAIRING_EVAL_DEBUG").is_some() {
        let a = &face.anatomy;
        eprintln!(
            "{dir}: x={:?} xcap={:?} contrast={:?} stem={:?} dark={:?} spacing={:?} width={:?} slant={:?} connect={:?} bounce={:?} ap={:?} ape={:?} vi_missing={}",
            a.x_height_em.value, a.x_height_cap.value, a.contrast.value, a.stem.value, a.darkness.value, a.spacing.value, a.width.value,
            a.slant_deg.value, a.connect.value, a.bounce.value, a.aperture.value, a.aperture_e.value, face.vietnamese.missing.len()
        );
    }
    let fam = face.metadata.typographic_family.clone().or_else(|| face.metadata.family.clone()).unwrap_or_else(|| dir.to_string());
    Some((traits_from_face(face)?, fam))
}

fn role(s: &str) -> Role {
    match s {
        "body" => Role::Body,
        "caption" => Role::Caption,
        "heading" => Role::Heading,
        "display" => Role::Display,
        "quote" => Role::Quote,
        "label" => Role::Label,
        "numerals" => Role::Numerals,
        o => panic!("vai lạ: {o}"),
    }
}

fn fit_name(f: Fit) -> &'static str {
    match f {
        Fit::Good => "good",
        Fit::Ok => "ok",
        Fit::Weak => "weak",
        Fit::Unknown => "unknown",
    }
}

fn verdict(expect: &str, f: Fit) -> bool {
    match expect {
        "ok" => f >= Fit::Ok,
        "weak" => f == Fit::Weak,
        o => panic!("expect lạ: {o}"),
    }
}

fn codes(r: &[fontatlas_core::pairing::Reason]) -> String {
    r.iter().map(|x| x.code).collect::<Vec<_>>().join(", ")
}

fn main() {
    let gf = PathBuf::from(arg("--gf").expect("--gf"));
    let gold = arg("--gold").expect("--gold");
    let mut rdr = csv::ReaderBuilder::new().comment(Some(b'#')).from_path(&gold).expect("đọc gold");
    let mut md = String::from("## Eval G4: cặp/vai vàng\n\n");
    let (mut pairs, mut roles) = (String::new(), String::new());
    let (mut pok, mut pn, mut rok, mut rn) = (0, 0, 0, 0);
    for rec in rdr.records() {
        let r = rec.expect("dòng csv");
        let g = |i: usize| r.get(i).unwrap_or("").to_string();
        let (kind, da, ra, db, rb, style, expect, source) = (g(0), g(1), g(2), g(3), g(4), g(5), g(6), g(7));
        let Some((ta, na)) = load(&gf, &da) else {
            md.push_str(&format!("- THIẾU font `{da}`\n"));
            continue;
        };
        if kind == "role" {
            let f = role_fit(&ta, role(&ra));
            let ok = verdict(&expect, f.fit);
            rn += 1;
            rok += ok as usize;
            roles.push_str(&format!("| {na} | {ra} | {} | {} | {} | {} |\n", expect, fit_name(f.fit), if ok { "✓" } else { "✗" }, codes(&f.reasons)));
            let _ = (rb, source);
        } else {
            let Some((tb, nb)) = load(&gf, &db) else {
                md.push_str(&format!("- THIẾU font `{db}`\n"));
                continue;
            };
            let ctx = PairCtx { same_family_root: family_root(&na) == family_root(&nb), ..PairCtx::default() };
            let st = PairStyle::from_id(&style).expect("style");
            let f = pair_style(&ta, &tb, &ctx, st);
            let ok = verdict(&expect, f.fit);
            pn += 1;
            pok += ok as usize;
            let cls = |t: &Traits| if t.class == Class::Uncertain { "?".to_string() } else { t.class.label().to_string() };
            pairs.push_str(&format!(
                "| {na} ({}) + {nb} ({}) | {style} | {expect} | {} | {} | {} | {} |\n",
                cls(&ta), cls(&tb), fit_name(f.fit), if ok { "✓" } else { "✗" }, codes(&f.reasons), codes(&f.warnings)
            ));
            let _ = (ra, rb, source);
        }
    }
    md.push_str(&format!("### Cặp: {pok}/{pn} đúng kỳ vọng\n\n| cặp | phong cách | kỳ vọng | mức | | lý do | cảnh báo |\n|---|---|---|---|---|---|---|\n{pairs}\n"));
    md.push_str(&format!("### Vai: {rok}/{rn} đúng kỳ vọng\n\n| họ | vai | kỳ vọng | mức | | lý do |\n|---|---|---|---|---|---|\n{roles}"));
    match arg("--out") {
        Some(p) => std::fs::write(p, &md).expect("ghi"),
        None => print!("{md}"),
    }
}
