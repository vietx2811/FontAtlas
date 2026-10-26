//! Kho font để đánh giá: phân tích thư mục, mở rộng variable font ra các độ đậm có tên, tra theo tên.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fontatlas_core::{FaceReport, FontFile, MeasureAt, analyze_bytes};
use fontatlas_index::{AxisId, FaceKey, FaceProfile};
use rayon::prelude::*;

const EXTS: [&str; 6] = ["ttf", "otf", "ttc", "otc", "woff", "woff2"];
/// Độ đậm mở rộng cho variable font.
const VF_WEIGHTS: [(f32, &str); 6] = [
    (300.0, "Light"),
    (400.0, "Regular"),
    (500.0, "Medium"),
    (600.0, "SemiBold"),
    (700.0, "Bold"),
    (900.0, "Black"),
];

/// Nguồn của một profile: file, face trong file, vị trí trục (VF mở rộng) — để render đúng instance.
#[derive(Debug, Clone, PartialEq)]
pub struct Origin {
    pub path: PathBuf,
    pub face_index: u32,
    /// `None` = vị trí chuẩn của font (như app).
    pub loc: Option<Vec<(String, f32)>>,
}

#[derive(Debug, Clone, Default)]
pub struct Corpus {
    pub profiles: Vec<FaceProfile>,
    /// Số file không phân tích được.
    pub failed_files: usize,
    pub origins: HashMap<FaceKey, Origin>,
}

fn walk(p: &Path, out: &mut Vec<PathBuf>) {
    if p.is_file() {
        out.push(p.to_path_buf());
        return;
    }
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

/// Tên độ đậm → weight (bảng resolve).
fn style_weight(style: &str) -> Option<f32> {
    match style.to_ascii_lowercase().replace([' ', '-'], "").as_str() {
        "regular" | "roman" | "book" => Some(400.0),
        "medium" => Some(500.0),
        "semibold" | "demibold" => Some(600.0),
        "bold" => Some(700.0),
        "light" => Some(300.0),
        "black" => Some(900.0),
        _ => None,
    }
}

fn upright(p: &FaceProfile) -> bool {
    p.get(AxisId::SlantDeg).value.is_none_or(|s| s.abs() < 1.0)
}

/// Phân tích một file; VF (khi `expand`) được thay bằng các profile ở từng độ đậm.
fn load_file(path: &Path, expand: bool) -> Option<Vec<(FaceProfile, Origin)>> {
    let raw = std::fs::read(path).ok()?;
    let report = analyze_bytes(raw.clone()).ok()?;
    let file = FontFile::from_bytes(raw).ok()?;
    let faces = file.faces().ok()?;
    let mut out = Vec::new();
    for fr in &report.faces {
        let wght = fr.metadata.axes.iter().find(|a| a.tag == "wght");
        let face = faces.iter().find(|f| f.index() == fr.index);
        match (expand, wght, face) {
            (true, Some(w), Some(face)) => {
                for (target, name) in VF_WEIGHTS {
                    if target < w.min || target > w.max {
                        continue;
                    }
                    // Core không kẹp MeasureAt::Location (M8) → eval tự kẹp và bỏ trục font không có.
                    let mut loc = vec![("wght".to_string(), target)];
                    for (tag, v) in [("wdth", 100.0f32), ("opsz", 14.0)] {
                        if let Some(a) = fr.metadata.axes.iter().find(|a| a.tag == tag) {
                            loc.push((tag.to_string(), v.clamp(a.min, a.max)));
                        }
                    }
                    let r_loc = loc.clone();
                    let anatomy = face.measure(MeasureAt::Location(loc));
                    let r = FaceReport {
                        index: fr.index,
                        metadata: fr.metadata.clone(),
                        vietnamese: fr.vietnamese.clone(),
                        anatomy,
                    };
                    let mut p = FaceProfile::from_report(&report.hash_hex, &r);
                    p.style = name.to_string();
                    p.weight_class = Some(target as u16);
                    p.key.index = fr.index * 1000 + target as u32;
                    let o = Origin {
                        path: path.to_path_buf(),
                        face_index: fr.index,
                        loc: Some(r_loc),
                    };
                    out.push((p, o));
                }
            }
            _ => out.push((
                FaceProfile::from_report(&report.hash_hex, fr),
                Origin {
                    path: path.to_path_buf(),
                    face_index: fr.index,
                    loc: None,
                },
            )),
        }
    }
    Some(out)
}

impl Corpus {
    pub fn load(dirs: &[PathBuf], expand_variable: bool) -> Corpus {
        let mut files = Vec::new();
        for d in dirs {
            walk(d, &mut files);
        }
        files.sort();
        let per: Vec<Option<Vec<(FaceProfile, Origin)>>> = files
            .par_iter()
            .map(|p| load_file(p, expand_variable))
            .collect();
        let failed_files = per.iter().filter(|x| x.is_none()).count();
        let mut pairs: Vec<(FaceProfile, Origin)> = per.into_iter().flatten().flatten().collect();
        // Trùng (cùng hash, index) khi một file xuất hiện ở nhiều thư mục.
        pairs.sort_by(|a, b| a.0.key.cmp(&b.0.key));
        pairs.dedup_by(|a, b| a.0.key == b.0.key);
        let origins = pairs
            .iter()
            .map(|(p, o)| (p.key.clone(), o.clone()))
            .collect();
        Corpus {
            profiles: pairs.into_iter().map(|x| x.0).collect(),
            failed_files,
            origins,
        }
    }

    /// Family không phân biệt hoa thường; style khớp tên, rồi theo weight gần nhất; ưu tiên chữ đứng.
    pub fn resolve(&self, family: &str, style: &str) -> Option<&FaceProfile> {
        let cands: Vec<&FaceProfile> = self
            .profiles
            .iter()
            .filter(|p| p.family.eq_ignore_ascii_case(family))
            .collect();
        if cands.is_empty() {
            return None;
        }
        let exact: Vec<&FaceProfile> = cands
            .iter()
            .copied()
            .filter(|p| p.style.eq_ignore_ascii_case(style))
            .collect();
        if let Some(p) = exact.iter().find(|p| upright(p)).or(exact.first()) {
            return Some(p);
        }
        let target = style_weight(style)?;
        let dist = |p: &FaceProfile| {
            p.weight_class
                .map(f32::from)
                .or_else(|| style_weight(&p.style))
                .map(|w| (w - target).abs())
        };
        let mut scored: Vec<(f32, bool, &FaceProfile)> = cands
            .iter()
            .filter_map(|p| Some((dist(p)?, !upright(p), *p)))
            .collect();
        scored.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then(a.0.total_cmp(&b.0))
                .then(a.2.key.cmp(&b.2.key))
        });
        scored.first().map(|x| x.2)
    }

    pub fn origin(&self, k: &FaceKey) -> Option<&Origin> {
        self.origins.get(k)
    }

    /// Mọi face của family (cho "Family|*").
    pub fn family(&self, family: &str) -> Vec<&FaceProfile> {
        self.profiles
            .iter()
            .filter(|p| p.family.eq_ignore_ascii_case(family))
            .collect()
    }
}
