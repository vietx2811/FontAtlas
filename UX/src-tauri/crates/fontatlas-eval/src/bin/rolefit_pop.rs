//! Phân bố mức hợp vai (good/ok/weak/unknown) trên tập eval 1722+ họ OFL, từ số đo đã lưu trong CSV (không cần font).
//!   rolefit_pop --csv docs/team/eval/classify-small-serif/after_core-0.9.0_regular.csv [--dump out.tsv]
//! Dùng cho bảng "trước/sau" khi đổi mô hình: chạy ở commit trước và commit sau rồi so. `vi_missing` = None (CSV không có):
//! chỉ đo ảnh hưởng của HÌNH DẠNG, không lẫn cổng tiếng Việt. Class = cột `pred` (phân loại tự động), không có `candidate`.
use std::collections::BTreeMap;

use fontatlas_core::classify::Class;
use fontatlas_core::pairing::{Fit, Role, Traits, role_fit};

const ROLES: [(Role, &str); 7] = [
    (Role::Body, "body"),
    (Role::Caption, "caption"),
    (Role::Heading, "heading"),
    (Role::Display, "display"),
    (Role::Quote, "quote"),
    (Role::Label, "label"),
    (Role::Numerals, "numerals"),
];

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

fn num(r: &csv::StringRecord, h: &BTreeMap<String, usize>, k: &str) -> Option<f32> {
    r.get(*h.get(k)?)?.trim().parse::<f32>().ok().filter(|v| v.is_finite())
}

fn name_of(f: Fit) -> &'static str {
    match f {
        Fit::Good => "good",
        Fit::Ok => "ok",
        Fit::Weak => "weak",
        Fit::Unknown => "unknown",
    }
}

#[derive(Default, Clone, Copy)]
struct C([usize; 4]);

impl C {
    fn add(&mut self, f: Fit) {
        self.0[match f {
            Fit::Good => 0,
            Fit::Ok => 1,
            Fit::Weak => 2,
            Fit::Unknown => 3,
        }] += 1;
    }
    fn line(&self) -> String {
        let n = self.0.iter().sum::<usize>().max(1) as f64;
        let p = |i: usize| 100.0 * self.0[i] as f64 / n;
        format!("good {:>5.1}%  ok {:>5.1}%  weak {:>5.1}%  unknown {:>5.1}%  (n={})", p(0), p(1), p(2), p(3), n as usize)
    }
}

fn main() {
    let Some(path) = arg("--csv") else {
        eprintln!("cách dùng: rolefit_pop --csv <file.csv> [--dump <out.tsv>]");
        std::process::exit(2);
    };
    let mut rdr = csv::Reader::from_path(&path).expect("đọc csv");
    let h: BTreeMap<String, usize> = rdr.headers().expect("header").iter().enumerate().map(|(i, s)| (s.to_string(), i)).collect();
    let mut dump = String::new();
    // nhóm → vai → đếm
    let mut groups: BTreeMap<&str, BTreeMap<&str, C>> = BTreeMap::new();
    let mut n_all = 0usize;
    for rec in rdr.records() {
        let r = rec.expect("dòng");
        let name = r.get(h["name"]).unwrap_or("").to_string();
        let gt = r.get(h["gt"]).unwrap_or("").to_string();
        let class = Class::parse(&r.get(h["pred"]).unwrap_or("").to_lowercase()).unwrap_or(Class::Uncertain);
        let t = Traits {
            class,
            subclass: r.get(h["predsub"]).filter(|s| !s.is_empty()).map(String::from),
            x_height_em: num(&r, &h, "xh"),
            aperture: num(&r, &h, "aperture"),
            aperture_e: num(&r, &h, "aperture_e"),
            contrast: num(&r, &h, "contrast"),
            stem: num(&r, &h, "stem"),
            spacing: num(&r, &h, "spacing"),
            width: num(&r, &h, "width"),
            stress_deg: num(&r, &h, "stress"),
            a_storeys: num(&r, &h, "a_st"),
            g_storeys: num(&r, &h, "g_st"),
            ..Traits::default()
        };
        n_all += 1;
        let text = gt == "Sans" || gt == "Serif";
        for (role, rn) in ROLES {
            let x = role_fit(&t, role);
            groups.entry("tất cả").or_default().entry(rn).or_default().add(x.fit);
            groups.entry(if text { "chữ đọc (gt Sans+Serif)" } else { "khác (Script, Mono, Blackletter, Uncertain…)" }).or_default().entry(rn).or_default().add(x.fit);
            let codes: Vec<&str> = x.reasons.iter().map(|r| r.code).collect();
            dump.push_str(&format!("{name}\t{gt}\t{rn}\t{}\t{}\n", name_of(x.fit), codes.join(",")));
        }
    }
    println!("họ: {n_all}");
    for (g, by_role) in &groups {
        println!("\n## {g}");
        for (_, rn) in ROLES {
            println!("  {rn:<9} {}", by_role[rn].line());
        }
    }
    if let Some(p) = arg("--dump") {
        std::fs::write(p, dump).expect("ghi dump");
    }
}
