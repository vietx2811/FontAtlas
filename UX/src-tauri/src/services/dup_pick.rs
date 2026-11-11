//! Chọn bản giữ cho một nhóm trùng lặp — hàm THUẦN (không đọc đĩa / DB), quy tắc cố định giải thích được bằng một câu:
//! 1 nguồn ưu tiên · 2 không mất chữ Việt · 3 version cao hơn · 4 định dạng (OTF > TTF > TTC > WOFF2 > WOFF) ·
//! 5 đầy đủ hơn (nhiều glyph / dung lượng lớn hơn) · 6 hoà → hỏi người dùng. Xem docs/team/research/2026-11-11-duplicates.md §4.1.
use std::collections::BTreeMap;

use crate::dto::analysis::DuplicateKindDto;

/// Một thành viên của nhóm, đã gom đủ dữ liệu để chấm.
#[derive(Debug, Clone, PartialEq)]
pub struct Cand {
    pub path: String,
    pub format: String,
    pub version: Option<String>,
    pub revision: Option<f32>,
    pub modified: Option<i64>,
    pub is_newest: bool,
    pub num_glyphs: Option<u32>,
    /// Số ký tự tiếng Việt còn thiếu (`vi_missing`); 0 = đủ.
    pub vi_missing: u32,
    pub is_variable: bool,
    pub file_size: u64,
    /// Thứ hạng nguồn: nhỏ hơn = ưu tiên giữ hơn (thứ tự nguồn trong Settings).
    pub source_rank: usize,
    pub license: String,
    /// Tệp có nhiều face (TTC/OTC): ẩn tệp sẽ ẩn cả các face khác.
    pub num_faces: u32,
}

/// Mã + tham số để UI dựng câu thường ("Bản mới hơn: 2.137 so với 2.001").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reason {
    pub code: &'static str,
    pub args: BTreeMap<String, String>,
}

impl Reason {
    pub fn new(code: &'static str) -> Self {
        Reason { code, args: BTreeMap::new() }
    }
    pub fn with(mut self, k: &str, v: impl ToString) -> Self {
        self.args.insert(k.into(), v.to_string());
        self
    }
    /// JSON `{code, args}` để lưu vào sổ.
    pub fn json(&self) -> String {
        serde_json::json!({ "code": self.code, "args": self.args }).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// Chỉ số của bản giữ trong `cands`.
    pub keeper: usize,
    pub losers: Vec<usize>,
    /// Vì sao giữ bản này.
    pub reason: Reason,
    /// Tự động an toàn (không cần người dùng xem).
    pub auto_safe: bool,
    /// Vì sao KHÔNG tự quyết (khi `auto_safe` = false).
    pub review: Option<Reason>,
    /// Lưu ý không chặn (vd văn bản cũ có thể xuống dòng khác).
    pub warnings: Vec<Reason>,
}

/// Điểm định dạng: lớn hơn = nên giữ hơn. OTF (CFF) > TTF > TTC/OTC > WOFF2 > WOFF.
pub fn format_rank(fmt: &str) -> u8 {
    match fmt {
        "otf" => 5,
        "ttf" | "sfnt" => 4,
        "otc" | "ttc" => 3,
        "woff2" => 2,
        "woff" => 1,
        _ => 0,
    }
}

fn depth(p: &str) -> usize {
    p.matches('/').count()
}

/// Tên kiểu "Font (1).ttf", "Font copy.ttf" là bản tải lại / sao chép.
fn looks_like_copy(p: &str) -> bool {
    let name = p.rsplit('/').next().unwrap_or(p).to_lowercase();
    name.contains(" copy") || name.contains(" (") && name.contains(')') || name.contains("-copy") || name.contains("_copy")
}

fn only<T: PartialEq>(best: &[usize], f: impl Fn(usize) -> T, pick: impl Fn(&T, &T) -> bool) -> Option<usize> {
    // Trả chỉ số duy nhất đứng đầu theo `pick(a, b)` = "a tốt hơn b"; None nếu có từ hai bản ngang nhau ở đỉnh.
    let top = best.iter().copied().fold(None::<usize>, |acc, i| match acc {
        None => Some(i),
        Some(j) => Some(if pick(&f(i), &f(j)) { i } else { j }),
    })?;
    let ties = best.iter().filter(|&&i| i != top && !pick(&f(top), &f(i))).count();
    (ties == 0).then_some(top)
}

pub fn pick_keeper(kind: DuplicateKindDto, newest_known: bool, cands: &[Cand]) -> Option<Decision> {
    if cands.len() < 2 {
        return None;
    }
    let all: Vec<usize> = (0..cands.len()).collect();
    let mut warnings = Vec::new();
    let mut review: Option<Reason> = None;
    let finish = |keeper: usize, reason: Reason, review: Option<Reason>, mut warnings: Vec<Reason>| {
        let losers: Vec<usize> = all.iter().copied().filter(|&i| i != keeper).collect();
        let mut review = review;
        // an toàn hơn: tệp nhiều face không bao giờ tự động
        if review.is_none() && losers.iter().any(|&i| cands[i].num_faces > 1) {
            review = Some(Reason::new("collection_file"));
        }
        if kind != DuplicateKindDto::SameFile {
            let lic: std::collections::BTreeSet<&str> = cands.iter().map(|c| c.license.as_str()).collect();
            if review.is_none() && lic.len() > 1 {
                review = Some(Reason::new("different_license"));
            }
            if review.is_none() && cands.iter().any(|c| c.is_variable) && cands.iter().any(|c| !c.is_variable) {
                review = Some(Reason::new("variable_vs_static"));
            }
            let versions: std::collections::BTreeSet<String> = cands.iter().filter_map(|c| c.version.clone()).collect();
            if versions.len() > 1 {
                warnings.push(Reason::new("may_reflow"));
            }
        }
        Some(Decision { keeper, losers, reason, auto_safe: review.is_none(), review, warnings })
    };

    // 1. nguồn ưu tiên (duy nhất một bản ở hạng tốt nhất)
    let best_rank = cands.iter().map(|c| c.source_rank).min().unwrap_or(0);
    let at_best: Vec<usize> = all.iter().copied().filter(|&i| cands[i].source_rank == best_rank).collect();
    if at_best.len() == 1 && cands.iter().any(|c| c.source_rank != best_rank) {
        let k = at_best[0];
        // nguồn ưu tiên mà kém hơn rõ rệt về nội dung → không tự động
        for (i, c) in cands.iter().enumerate() {
            if i == k {
                continue;
            }
            let k_c = &cands[k];
            let less_vi = c.vi_missing < k_c.vi_missing;
            let older = matches!((c.revision, k_c.revision), (Some(a), Some(b)) if a > b);
            let fewer = matches!((k_c.num_glyphs, c.num_glyphs), (Some(a), Some(b)) if a < b);
            if kind != DuplicateKindDto::SameFile && (less_vi || older || fewer) {
                review = Some(Reason::new("preferred_source_is_worse"));
            }
        }
        return finish(k, Reason::new("preferred_source"), review, warnings);
    }
    let pool = at_best.clone();

    // 2. không mất chữ Việt
    if kind != DuplicateKindDto::SameFile {
        let min_missing = pool.iter().map(|&i| cands[i].vi_missing).min().unwrap_or(0);
        let max_missing = pool.iter().map(|&i| cands[i].vi_missing).max().unwrap_or(0);
        if min_missing != max_missing {
            let top: Vec<usize> = pool.iter().copied().filter(|&i| cands[i].vi_missing == min_missing).collect();
            if top.len() == 1 {
                let r = Reason::new("more_complete_vi").with("missing", max_missing - min_missing);
                return finish(top[0], r, None, warnings);
            }
        }
    }

    // 3. version cao hơn (chỉ khi phân biệt được rõ)
    if kind == DuplicateKindDto::Versions && newest_known {
        if let Some(k) = pool.iter().copied().find(|&i| cands[i].is_newest) {
            let fewer = pool.iter().any(|&i| i != k && matches!((cands[k].num_glyphs, cands[i].num_glyphs), (Some(a), Some(b)) if (a as f64) < (b as f64) * 0.98));
            let r = Reason::new("newer_version")
                .with("keep", cands[k].version.clone().unwrap_or_default())
                .with("other", pool.iter().filter(|&&i| i != k).filter_map(|&i| cands[i].version.clone()).next().unwrap_or_default());
            let rv = fewer.then(|| Reason::new("newer_has_fewer_glyphs"));
            return finish(k, r, rv, warnings);
        }
    }
    if kind == DuplicateKindDto::Versions && !newest_known {
        review = Some(Reason::new("newest_unknown"));
    }

    // 4. định dạng
    if kind != DuplicateKindDto::SameFile {
        if let Some(k) = only(&pool, |i| format_rank(&cands[i].format), |a, b| a > b) {
            // bản được chọn phải thực sự hơn (không phải tất cả cùng hạng)
            if pool.iter().any(|&i| i != k && format_rank(&cands[i].format) < format_rank(&cands[k].format)) {
                let r = Reason::new("preferred_format").with("format", cands[k].format.to_uppercase());
                return finish(k, r, review, warnings);
            }
        }
    }

    // 5. đầy đủ hơn
    if kind != DuplicateKindDto::SameFile {
        if let Some(k) = only(&pool, |i| cands[i].num_glyphs.unwrap_or(0), |a, b| a > b)
            && pool.iter().any(|&i| i != k && cands[i].num_glyphs.unwrap_or(0) < cands[k].num_glyphs.unwrap_or(0))
        {
            let r = Reason::new("more_glyphs").with("glyphs", cands[k].num_glyphs.unwrap_or(0));
            return finish(k, r, review, warnings);
        }
        if let Some(k) = only(&pool, |i| cands[i].file_size, |a, b| a > b)
            && pool.iter().any(|&i| i != k && cands[i].file_size < cands[k].file_size)
        {
            return finish(k, Reason::new("larger_file"), review, warnings);
        }
    } else {
        // cùng nội dung: giữ bản không giống bản sao, nông hơn, đường dẫn ngắn hơn — an toàn vì nội dung giống hệt
        let score = |i: usize| (looks_like_copy(&cands[i].path), depth(&cands[i].path), cands[i].path.len(), cands[i].path.clone());
        let k = pool.iter().copied().min_by_key(|&i| score(i)).unwrap_or(0);
        return finish(k, Reason::new("same_content_tidiest_path"), None, warnings);
    }

    // 6. hoà
    let k = pool.first().copied().unwrap_or(0);
    warnings.clear();
    finish(k, Reason::new("tie"), Some(Reason::new("tie")), vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(path: &str, format: &str) -> Cand {
        Cand {
            path: path.into(),
            format: format.into(),
            version: None,
            revision: None,
            modified: None,
            is_newest: false,
            num_glyphs: Some(500),
            vi_missing: 0,
            is_variable: false,
            file_size: 100,
            source_rank: 0,
            license: "ofl".into(),
            num_faces: 1,
        }
    }

    use DuplicateKindDto::*;

    #[test]
    fn preferred_source_wins_for_identical_copies() {
        let mut a = c("/lib/A.ttf", "ttf");
        let mut b = c("/dl/A.ttf", "ttf");
        a.source_rank = 0;
        b.source_rank = 1;
        let d = pick_keeper(SameFile, false, &[b, a]).unwrap();
        assert_eq!(d.keeper, 1);
        assert_eq!(d.reason.code, "preferred_source");
        assert!(d.auto_safe);
    }

    #[test]
    fn identical_copies_in_one_source_keep_the_tidiest_path() {
        let d = pick_keeper(SameFile, false, &[c("/f/Roboto (1).ttf", "ttf"), c("/f/sub/Roboto.ttf", "ttf"), c("/f/Roboto.ttf", "ttf")]).unwrap();
        assert_eq!(d.keeper, 2, "tên sạch + nông hơn");
        assert_eq!(d.losers, vec![0, 1]);
        assert_eq!(d.reason.code, "same_content_tidiest_path");
        assert!(d.auto_safe);
    }

    #[test]
    fn a_source_that_is_worse_on_content_needs_review_not_automatic() {
        let mut a = c("/lib/A.ttf", "ttf");
        a.revision = Some(1.0);
        let mut b = c("/dl/A.ttf", "ttf");
        b.revision = Some(2.0);
        b.source_rank = 1;
        let d = pick_keeper(Versions, true, &[a, b]).unwrap();
        assert_eq!(d.keeper, 0);
        assert!(!d.auto_safe);
        assert_eq!(d.review.unwrap().code, "preferred_source_is_worse");
    }

    #[test]
    fn never_prefers_the_font_that_loses_vietnamese_coverage() {
        let mut a = c("/f/A.otf", "otf");
        a.vi_missing = 12;
        let b = c("/f/B.ttf", "ttf");
        let d = pick_keeper(Formats, false, &[a, b]).unwrap();
        assert_eq!(d.keeper, 1, "TTF đủ chữ Việt thắng OTF thiếu 12 ký tự");
        assert_eq!(d.reason.code, "more_complete_vi");
        assert_eq!(d.reason.args["missing"], "12");
    }

    #[test]
    fn newest_version_wins_when_known() {
        let mut a = c("/f/A1.ttf", "ttf");
        a.version = Some("Version 2.001".into());
        let mut b = c("/f/A2.ttf", "ttf");
        b.version = Some("Version 2.137".into());
        b.is_newest = true;
        let d = pick_keeper(Versions, true, &[a, b]).unwrap();
        assert_eq!(d.keeper, 1);
        assert_eq!(d.reason.code, "newer_version");
        assert_eq!(d.reason.args["keep"], "Version 2.137");
        assert!(d.auto_safe);
        assert!(d.warnings.iter().any(|w| w.code == "may_reflow"), "khác version → cảnh báo xuống dòng");
    }

    #[test]
    fn newer_but_with_fewer_glyphs_needs_review() {
        let mut a = c("/f/A1.ttf", "ttf");
        a.num_glyphs = Some(1000);
        let mut b = c("/f/A2.ttf", "ttf");
        b.num_glyphs = Some(600);
        b.is_newest = true;
        let d = pick_keeper(Versions, true, &[a, b]).unwrap();
        assert_eq!(d.keeper, 1);
        assert!(!d.auto_safe);
        assert_eq!(d.review.unwrap().code, "newer_has_fewer_glyphs");
    }

    #[test]
    fn unknown_newest_with_no_other_difference_is_a_tie_for_review() {
        let d = pick_keeper(Versions, false, &[c("/f/A1.ttf", "ttf"), c("/f/A2.ttf", "ttf")]).unwrap();
        assert!(!d.auto_safe);
        assert_eq!(d.review.unwrap().code, "tie");
    }

    #[test]
    fn otf_beats_ttf_and_web_formats_rank_last() {
        let d = pick_keeper(Formats, false, &[c("/f/A.ttf", "ttf"), c("/f/A.otf", "otf")]).unwrap();
        assert_eq!(d.keeper, 1);
        assert_eq!(d.reason.code, "preferred_format");
        assert_eq!(d.reason.args["format"], "OTF");
        let d = pick_keeper(Formats, false, &[c("/f/A.woff2", "woff2"), c("/f/A.ttf", "ttf")]).unwrap();
        assert_eq!(d.keeper, 1);
    }

    #[test]
    fn variable_versus_static_needs_review() {
        let mut a = c("/f/A.ttf", "ttf");
        a.is_variable = true;
        let d = pick_keeper(Formats, false, &[a, c("/f/B.otf", "otf")]).unwrap();
        assert!(!d.auto_safe);
        assert_eq!(d.review.unwrap().code, "variable_vs_static");
    }

    #[test]
    fn different_licenses_need_review() {
        let mut a = c("/f/A.ttf", "ttf");
        a.license = "other".into();
        let d = pick_keeper(Formats, false, &[a, c("/f/B.otf", "otf")]).unwrap();
        assert!(!d.auto_safe);
        assert_eq!(d.review.unwrap().code, "different_license");
    }

    #[test]
    fn collection_files_are_never_automatic() {
        let mut a = c("/f/A.ttc", "ttc");
        a.num_faces = 4;
        let d = pick_keeper(Formats, false, &[a, c("/f/B.otf", "otf")]).unwrap();
        assert_eq!(d.keeper, 1);
        assert!(!d.auto_safe);
        assert_eq!(d.review.unwrap().code, "collection_file");
    }

    #[test]
    fn bigger_file_breaks_a_format_tie_and_full_tie_asks_the_user() {
        let mut a = c("/f/A.ttf", "ttf");
        a.num_glyphs = Some(500);
        a.file_size = 90;
        let b = c("/f/B.ttf", "ttf");
        let d = pick_keeper(Formats, false, &[a.clone(), b.clone()]).unwrap();
        assert_eq!(d.keeper, 1);
        assert_eq!(d.reason.code, "larger_file");
        let mut e = b.clone();
        e.path = "/f/C.ttf".into();
        let d = pick_keeper(Formats, false, &[b, e]).unwrap();
        assert!(!d.auto_safe);
        assert_eq!(d.reason.code, "tie");
    }

    #[test]
    fn keeps_at_least_one_and_losers_exclude_the_keeper() {
        let d = pick_keeper(Formats, false, &[c("/f/A.ttf", "ttf"), c("/f/A.otf", "otf"), c("/f/A.woff", "woff")]).unwrap();
        assert_eq!(d.losers.len(), 2);
        assert!(!d.losers.contains(&d.keeper));
        assert!(pick_keeper(Formats, false, &[c("/f/A.ttf", "ttf")]).is_none());
    }
}
