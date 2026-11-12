//! Bộ kiểm mù cho chủ app (G4).
//!   pairing_blind --gf <google-fonts> --out <thư-mục-trang> [--seed N]        # dựng index.html + fonts/ + key.json
//!   pairing_blind --score <results.json> --key <key.json>                       # tính kết quả sau khi chủ app chấm
//! 20 gợi ý của mô hình (mức Good ở phong cách của nó) xen kẽ ngẫu nhiên với 10 cặp ngẫu nhiên làm đối chứng.
//! Trang không hiện phong cách, mức hay nguồn; key.json giữ đáp án. KHÔNG chỉnh ngưỡng theo kết quả bộ này.
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use fontatlas_core::classify::Class;
use fontatlas_core::pairing::{Fit, PairCtx, PairStyle, Role, Traits, family_root, pair_style, role_fit, traits_from_face};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

struct Cand {
    dir: String,
    file: PathBuf,
    name: String,
    t: Traits,
}

fn pick_file(dir: &Path) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.ends_with(".ttf") && !n.contains("Italic")
        })
        .collect();
    v.sort();
    v.iter().find(|p| p.to_string_lossy().contains("Regular")).cloned().or_else(|| v.into_iter().next())
}

fn load_all(gf: &Path) -> Vec<Cand> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(gf.join("ofl")).expect("ofl").filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    dirs.par_iter()
        .filter_map(|d| {
            let file = pick_file(d)?;
            // Trang gửi cho chủ app: bỏ font nặng (> 400 KB) để gói nhỏ.
            if std::fs::metadata(&file).ok()?.len() > 400_000 {
                return None;
            }
            let r = fontatlas_core::analyze_path(&file).ok()?;
            let face = r.faces.first()?;
            let name = face.metadata.typographic_family.clone().or_else(|| face.metadata.family.clone())?;
            Some(Cand { dir: d.file_name()?.to_string_lossy().into_owned(), file, name, t: traits_from_face(face)? })
        })
        .collect()
}

fn role_name(r: Role) -> &'static str {
    match r {
        Role::Body => "Body",
        Role::Caption => "Caption",
        Role::Heading => "Heading",
        Role::Display => "Display",
        Role::Quote => "Quote",
        Role::Label => "Label",
        Role::Numerals => "Numerals",
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct KeyItem {
    id: String,
    kind: String,
    style: String,
    roles: [String; 2],
    fonts: [String; 2],
    names: [String; 2],
    /// Mức mô hình cho từng phong cách (để phân tích sau, không hiện cho chủ app).
    model_fits: BTreeMap<String, String>,
}

fn fit_s(f: Fit) -> &'static str {
    match f {
        Fit::Good => "good",
        Fit::Ok => "ok",
        Fit::Weak => "weak",
        Fit::Unknown => "unknown",
    }
}

fn all_fits(a: &Traits, b: &Traits, ctx: &PairCtx) -> BTreeMap<String, String> {
    PairStyle::ALL.iter().map(|s| (s.id().to_string(), fit_s(pair_style(a, b, ctx, *s).fit).to_string())).collect()
}

fn build(gf: &Path, out: &Path, seed: u64) {
    let cands: Vec<Cand> = load_all(gf);
    // Chữ mẫu có đủ dấu chồng: chỉ dùng font đủ tiếng Việt, kể cả cho Display và đối chứng.
    let ok: Vec<usize> = (0..cands.len()).filter(|&i| cands[i].t.vi_missing == Some(0)).collect();
    eprintln!("{} họ phân tích được, {} đủ tiếng Việt", cands.len(), ok.len());
    let mut rng = Rng(seed | 1);
    let mut used: HashSet<usize> = HashSet::new();
    let mut items: Vec<(KeyItem, usize, usize, Role, Role)> = vec![];

    // (phong cách, vai a, vai b, lớp a, lớp b, số lượng)
    type Plan = (PairStyle, Role, Role, Option<Class>, Option<Class>, usize);
    let plan: Vec<Plan> = vec![
        (PairStyle::Contrast, Role::Heading, Role::Body, Some(Class::Serif), Some(Class::Sans), 5),
        (PairStyle::Contrast, Role::Heading, Role::Body, Some(Class::Sans), Some(Class::Serif), 2),
        (PairStyle::Superfamily, Role::Heading, Role::Body, None, None, 2),
        (PairStyle::SameStructure, Role::Heading, Role::Body, None, None, 3),
        (PairStyle::SameEra, Role::Heading, Role::Body, None, None, 3),
        (PairStyle::DisplayVsNeutral, Role::Display, Role::Body, None, None, 5),
    ];
    let mut n = 0;
    for (style, ra, rb, ca, cb, count) in plan {
        let mut made = 0;
        let mut tries = 0;
        while made < count && tries < 200_000 {
            tries += 1;
            let (i, j) = (ok[rng.below(ok.len())], ok[rng.below(ok.len())]);
            if i == j || used.contains(&i) || used.contains(&j) {
                continue;
            }
            let (a, b) = (&cands[i], &cands[j]);
            if ca.is_some_and(|c| a.t.class != c) || cb.is_some_and(|c| b.t.class != c) {
                continue;
            }
            let same = family_root(&a.name) == family_root(&b.name);
            if (style == PairStyle::Superfamily) != same {
                continue;
            }
            let ctx = PairCtx { same_family_root: same, ..PairCtx::default() };
            if role_fit(&a.t, ra).fit < Fit::Ok || role_fit(&b.t, rb).fit < Fit::Ok || pair_style(&a.t, &b.t, &ctx, style).fit != Fit::Good {
                continue;
            }
            used.insert(i);
            used.insert(j);
            n += 1;
            made += 1;
            items.push((
                KeyItem {
                    id: String::new(),
                    kind: "model".into(),
                    style: style.id().into(),
                    roles: [role_name(ra).into(), role_name(rb).into()],
                    fonts: [a.dir.clone(), b.dir.clone()],
                    names: [a.name.clone(), b.name.clone()],
                    model_fits: all_fits(&a.t, &b.t, &ctx),
                },
                i,
                j,
                ra,
                rb,
            ));
        }
        eprintln!("{} {}: {made}/{count}", style.id(), n);
    }
    // 10 cặp ngẫu nhiên làm đối chứng.
    let mut c = 0;
    while c < 10 {
        let (i, j) = (ok[rng.below(ok.len())], ok[rng.below(ok.len())]);
        if i == j || used.contains(&i) || used.contains(&j) {
            continue;
        }
        used.insert(i);
        used.insert(j);
        let (ra, rb) = if c % 2 == 0 { (Role::Heading, Role::Body) } else { (Role::Display, Role::Body) };
        let (a, b) = (&cands[i], &cands[j]);
        let ctx = PairCtx { same_family_root: family_root(&a.name) == family_root(&b.name), ..PairCtx::default() };
        items.push((
            KeyItem {
                id: String::new(),
                kind: "control".into(),
                style: "random".into(),
                roles: [role_name(ra).into(), role_name(rb).into()],
                fonts: [a.dir.clone(), b.dir.clone()],
                names: [a.name.clone(), b.name.clone()],
                model_fits: all_fits(&a.t, &b.t, &ctx),
            },
            i,
            j,
            ra,
            rb,
        ));
        c += 1;
    }
    // Xáo trộn rồi đặt id theo thứ tự hiện.
    for k in (1..items.len()).rev() {
        items.swap(k, rng.below(k + 1));
    }
    for (k, it) in items.iter_mut().enumerate() {
        it.0.id = format!("c{:02}", k + 1);
    }

    let fonts_dir = out.join("fonts");
    std::fs::create_dir_all(&fonts_dir).expect("tạo fonts/");
    let mut css = String::new();
    let mut cards = String::new();
    for (key, i, j, ra, _rb) in &items {
        for (slot, idx) in [("a", *i), ("b", *j)] {
            let c = &cands[idx];
            let fname = c.file.file_name().unwrap().to_string_lossy().into_owned();
            let rel = format!("fonts/{}/{}", c.dir, fname);
            std::fs::create_dir_all(fonts_dir.join(&c.dir)).expect("mkdir");
            std::fs::copy(&c.file, out.join(&rel)).expect("copy font");
            css.push_str(&format!(
                "@font-face{{font-family:\"{}_{slot}\";src:url(\"{}\");font-weight:100 900;}}\n",
                key.id,
                rel.replace(' ', "%20").replace('[', "%5B").replace(']', "%5D").replace(',', "%2C")
            ));
        }
        let top = match ra {
            Role::Display => ("Hoa sen nở rộ", "display"),
            _ => ("Mùa nước nổi ở miền Tây", "heading"),
        };
        cards.push_str(&format!(
            r#"<article class="card" data-id="{id}"><div class="lab">{ra_n}</div><div class="s-{kind}" style="font-family:'{id}_a'">{top}</div><div class="lab">{rb_n}</div><p class="body" style="font-family:'{id}_b'">Ở nơi dòng sông Cửu Long chảy ra biển, những chiếc ghe nhỏ lặng lẽ qua chợ nổi lúc bình minh. Người bán hàng rao: “Dừa tươi, thơm ngọt, ba nghìn một trái!” Quả ổi, củ khoai, chén chè đậu xanh, tất cả đều ướt sương.</p><div class="fonts">{na} + {nb}</div><div class="rate" role="radiogroup" aria-label="Chấm {id}"><label><input type="radio" name="{id}" value="hop"><span>Hợp</span></label><label><input type="radio" name="{id}" value="tam"><span>Tạm</span></label><label><input type="radio" name="{id}" value="khong"><span>Không</span></label></div></article>
"#,
            id = key.id,
            ra_n = key.roles[0],
            rb_n = key.roles[1],
            kind = top.1,
            top = top.0,
            na = esc(&key.names[0]),
            nb = esc(&key.names[1]),
        ));
    }
    let html = page(&css, &cards, items.len());
    std::fs::write(out.join("index.html"), html).expect("ghi html");
    let key: Vec<&KeyItem> = items.iter().map(|x| &x.0).collect();
    std::fs::write(out.join("key.json"), serde_json::to_string_pretty(&key).unwrap()).expect("ghi key");
    eprintln!("Xong: {} thẻ ({} gợi ý + {} đối chứng) → {}", items.len(), items.len() - 10, 10, out.display());
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;")
}

fn page(css: &str, cards: &str, total: usize) -> String {
    format!(
        r#"<!doctype html>
<html lang="vi"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Kiểm mù cặp chữ</title>
<style>
:root{{color-scheme:light dark;--bg:#faf8f4;--fg:#1d1b18;--mut:#6b655d;--line:#d9d3c8;--card:#fff;--acc:#b3261e}}
@media (prefers-color-scheme:dark){{:root{{--bg:#171512;--fg:#efeae2;--mut:#a39c91;--line:#3a352e;--card:#1f1c18;--acc:#ff8a80}}}}
body{{margin:0;background:var(--bg);color:var(--fg);font:16px/1.5 -apple-system,system-ui,sans-serif}}
main{{max-width:760px;margin:0 auto;padding:16px}}
h1{{font-size:20px;margin:8px 0}} .intro{{color:var(--mut)}}
.bar{{position:sticky;top:0;background:var(--bg);padding:8px 0;border-bottom:1px solid var(--line);display:flex;gap:12px;align-items:center;flex-wrap:wrap;z-index:2}}
.card{{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:16px;margin:16px 0}}
.lab{{font:12px/1 system-ui;color:var(--mut);text-transform:uppercase;letter-spacing:.06em;margin:6px 0}}
.s-heading{{font-size:30px;line-height:1.2}} .s-display{{font-size:52px;line-height:1.1}}
.body{{font-size:17px;line-height:1.6;margin:4px 0 8px}}
.fonts{{font:12px system-ui;color:var(--mut);margin-bottom:10px}}
.rate{{display:flex;gap:8px}} .rate label{{flex:1}} .rate input{{position:absolute;opacity:0}}
.rate span{{display:block;text-align:center;padding:10px;border:1px solid var(--line);border-radius:8px;cursor:pointer;font:15px system-ui}}
.rate input:checked+span{{background:var(--fg);color:var(--bg);border-color:var(--fg)}}
.rate input:focus-visible+span{{outline:2px solid var(--acc);outline-offset:2px}}
button{{font:15px system-ui;padding:8px 14px;border-radius:8px;border:1px solid var(--fg);background:var(--fg);color:var(--bg);cursor:pointer}}
{css}</style></head><body><main>
<h1>Kiểm mù cặp chữ</h1>
<p class="intro">{total} cặp. Mỗi thẻ là một tiêu đề và một đoạn văn. Chấm theo cảm nhận của bạn: <b>Hợp</b> (dùng được ngay), <b>Tạm</b> (chấp nhận được), <b>Không</b> (không hợp). Không có đáp án đúng; thứ tự đã xáo trộn. Bạn có thể dừng và quay lại: lựa chọn được giữ trong trình duyệt này.</p>
<div class="bar"><span id="prog">0/{total}</span><button id="exp" disabled>Xuất kết quả (JSON)</button><span id="msg" role="status"></span></div>
{cards}
</main>
<script>
const KEY="pairing-blind-v1";
const radios=[...document.querySelectorAll('input[type=radio]')];
let saved={{}};try{{saved=JSON.parse(localStorage.getItem(KEY)||"{{}}")}}catch(e){{}}
radios.forEach(r=>{{if(saved[r.name]===r.value)r.checked=true;r.addEventListener('change',()=>{{saved[r.name]=r.value;try{{localStorage.setItem(KEY,JSON.stringify(saved))}}catch(e){{}};upd()}})}});
function upd(){{const n=Object.keys(saved).length;document.getElementById('prog').textContent=n+"/{total}";document.getElementById('exp').disabled=n<{total}}}
document.getElementById('exp').addEventListener('click',()=>{{
 const out={{created:new Date().toISOString(),ratings:Object.entries(saved).sort().map(([id,rating])=>({{id,rating}}))}};
 const blob=new Blob([JSON.stringify(out,null,2)],{{type:'application/json'}});
 const a=document.createElement('a');a.href=URL.createObjectURL(blob);a.download='pairing-blind-results.json';a.click();
 document.getElementById('msg').textContent='Đã tải pairing-blind-results.json. Gửi tệp này lại.';
}});
upd();
</script></body></html>
"#
    )
}

#[derive(Deserialize)]
struct Results {
    ratings: Vec<Rating>,
}
#[derive(Deserialize)]
struct Rating {
    id: String,
    rating: String,
}

fn score(results: &Path, key: &Path) {
    let r: Results = serde_json::from_slice(&std::fs::read(results).expect("đọc kết quả")).expect("json kết quả");
    let k: Vec<KeyItem> = serde_json::from_slice(&std::fs::read(key).expect("đọc key")).expect("json key");
    let rate: BTreeMap<&str, &str> = r.ratings.iter().map(|x| (x.id.as_str(), x.rating.as_str())).collect();
    let tally = |f: &dyn Fn(&KeyItem) -> bool| -> (usize, usize, usize, usize) {
        let (mut h, mut t, mut n, mut all) = (0, 0, 0, 0);
        for it in k.iter().filter(|i| f(i)) {
            match rate.get(it.id.as_str()) {
                Some(&"hop") => h += 1,
                Some(&"tam") => t += 1,
                Some(&"khong") => n += 1,
                _ => continue,
            }
            all += 1;
        }
        (h, t, n, all)
    };
    let row = |label: &str, c: (usize, usize, usize, usize)| {
        let p = |x: usize| if c.3 == 0 { "-".into() } else { format!("{:.0}%", 100.0 * x as f32 / c.3 as f32) };
        println!("| {label} | {} | {} | {} | {} | {} | {} |", c.3, c.0, c.1, c.2, p(c.0), p(c.0 + c.1));
    };
    println!("| nhóm | số thẻ | Hợp | Tạm | Không | Hợp % | Hợp+Tạm % |\n|---|---|---|---|---|---|---|");
    row("mô hình (20)", tally(&|i| i.kind == "model"));
    row("đối chứng ngẫu nhiên (10)", tally(&|i| i.kind == "control"));
    let mut styles: Vec<&str> = k.iter().filter(|i| i.kind == "model").map(|i| i.style.as_str()).collect();
    styles.sort();
    styles.dedup();
    for s in styles {
        row(&format!("  {s}"), tally(&|i| i.kind == "model" && i.style == s));
    }
}

fn main() {
    if let (Some(res), Some(key)) = (arg("--score"), arg("--key")) {
        score(Path::new(&res), Path::new(&key));
        return;
    }
    let gf = PathBuf::from(arg("--gf").expect("--gf"));
    let out = PathBuf::from(arg("--out").expect("--out"));
    let seed = arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(20261112);
    build(&gf, &out, seed);
}
