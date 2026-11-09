use fontatlas_describe::*;
use std::collections::HashMap;

fn ids(p: &Parsed) -> Vec<&str> {
    p.matched.iter().map(|m| m.id.as_str()).collect()
}

fn axes(p: &Parsed) -> Vec<&str> {
    let mut v: Vec<&str> = p.filters.iter().map(|f| f.axis.as_str()).collect();
    v.sort();
    v
}

#[test]
fn bundled_dictionary_valid() {
    let d = Dictionary::bundled();
    assert_eq!(d.len(), 58);
    assert_eq!(d.version(), "0.1.0-draft");
}

#[test]
fn warm_maps_to_aperture_terminal_contrast_stress() {
    let p = Dictionary::bundled().parse("ấm");
    assert_eq!(ids(&p), ["warm"]);
    assert_eq!(axes(&p), ["aperture", "contrast", "stress_deg", "terminal"]);
    assert!(p.filters.iter().all(|f| f.strength == Strength::Preferred));
}

#[test]
fn diacritic_free_input_matches() {
    let p = Dictionary::bundled().parse("am ap");
    assert_eq!(ids(&p), ["warm"]);
    assert_eq!(p.matched[0].text, "am ap");
    assert!(Dictionary::bundled().parse("WARM").matched.iter().any(|m| m.id == "warm"));
}

#[test]
fn longest_phrase_wins() {
    let p = Dictionary::bundled().parse("tối giản Hàn");
    assert_eq!(ids(&p), ["korean_minimal"]);
    assert!(p.unknown_words.is_empty());
    let q = Dictionary::bundled().parse("xyz tối giản");
    assert_eq!(ids(&q), ["minimal"]);
    assert_eq!(q.unknown_words, ["xyz"]);
}

#[test]
fn luxury_vs_casual_conflict_reported() {
    let p = Dictionary::bundled().parse("sang bình dân");
    assert_eq!(ids(&p), ["luxury", "casual"]);
    let mut cx: Vec<&str> = p.conflicts.iter().map(|c| c.axis.as_str()).collect();
    cx.sort();
    assert_eq!(cx, ["contrast", "x_height_em"]);
    assert!(!axes(&p).contains(&"x_height_em") && !axes(&p).contains(&"contrast"));
    // stem giao được → còn một bộ lọc đã gộp
    let stem = p.filters.iter().find(|f| f.axis == "stem").expect("stem gộp");
    let (lo, hi) = stem.range.unwrap();
    assert!((lo - 0.45).abs() < 1e-6 && (hi - 0.5).abs() < 1e-6, "{lo} {hi}");
}

#[test]
fn unmappable_reason_returned() {
    let p = Dictionary::bundled().parse("đương đại");
    assert_eq!(ids(&p), ["contemporary"]);
    assert_eq!(p.unmappable.len(), 1);
    assert!(p.unmappable[0].1.contains("metadata"), "{:?}", p.unmappable);
    assert!(p.filters.is_empty());
}

struct Face {
    pct: HashMap<&'static str, f32>,
    cls: HashMap<&'static str, &'static str>,
    missing: usize,
    supports: bool,
}

impl FaceView for Face {
    fn percentile(&self, axis: &str) -> Option<f32> {
        self.pct.get(axis).copied()
    }
    fn class(&self, axis: &str) -> Option<String> {
        self.cls.get(axis).map(|s| s.to_string())
    }
    fn vi_missing(&self) -> usize {
        self.missing
    }
    fn supports_vi(&self) -> bool {
        self.supports
    }
}

fn face(pct: &[(&'static str, f32)]) -> Face {
    Face { pct: pct.iter().copied().collect(), cls: HashMap::new(), missing: 0, supports: true }
}

const TINY: &str = r#"{
 "schema_version":"1.0.0","dictionary_version":"t","axes":{"contrast":{},"stem":{},"serif_foot":{}},
 "entries":[
  {"id":"a","terms":{"vi":["aaa"],"en":[]},"group":"tone","filters":[{"axis":"contrast","range":[0.5,1.0],"strength":"required","weight":1.0}],"needs_metadata":[],"rationale":{"vi":"","en":""},"sources":[],"subjectivity":"low","culture":"intl"},
  {"id":"b","terms":{"vi":["bbb"],"en":[]},"group":"tone","filters":[{"axis":"contrast","range":[0.2,0.8],"strength":"preferred","weight":0.4},{"axis":"stem","range":[0.0,0.3],"strength":"preferred","weight":0.5}],"needs_metadata":[],"rationale":{"vi":"","en":""},"sources":[],"subjectivity":"low","culture":"intl"}
 ]}"#;

#[test]
fn required_beats_preferred_and_null_axis_skipped() {
    let d = Dictionary::from_json(TINY).unwrap();
    let p = d.parse("aaa bbb");
    let c = p.filters.iter().find(|f| f.axis == "contrast").unwrap();
    assert_eq!(c.strength, Strength::Required);
    assert_eq!(c.range, Some((0.5, 0.8)));
    assert!((c.weight - 1.0).abs() < 1e-6);
    // contrast trượt → loại
    assert!(evaluate(&p.filters, &p.hard, &face(&[("contrast", 0.1)])).is_none());
    // contrast thoả, stem thiếu giá trị → bỏ qua (không trượt)
    let s = evaluate(&p.filters, &p.hard, &face(&[("contrast", 0.6)])).unwrap();
    assert!((s - 1.0).abs() < 1e-6, "{s}");
    // stem ngoài khoảng → trừ điểm theo khoảng cách
    let s2 = evaluate(&p.filters, &p.hard, &face(&[("contrast", 0.6), ("stem", 0.5)])).unwrap();
    assert!((s2 - (1.0 - 0.5 * 0.2)).abs() < 1e-5, "{s2}");
}

#[test]
fn vietnamese_ready_hard_constraint() {
    let d = Dictionary::bundled();
    let p = d.parse("đủ tiếng Việt");
    assert_eq!(ids(&p), ["vietnamese_ready"]);
    assert_eq!(p.hard, [HardConstraint::ViSupported, HardConstraint::ViComplete]);
    let mut f = face(&[]);
    assert!(evaluate(&p.filters, &p.hard, &f).is_some());
    f.missing = 3;
    assert!(evaluate(&p.filters, &p.hard, &f).is_none());
    f.missing = 0;
    f.supports = false;
    assert!(evaluate(&p.filters, &p.hard, &f).is_none());
}

#[test]
fn invalid_dictionary_rejected() {
    let bad_axis = TINY.replace("\"axis\":\"stem\"", "\"axis\":\"khong_co\"");
    assert!(Dictionary::from_json(&bad_axis).is_err());
    let bad_range = TINY.replace("[0.2,0.8]", "[0.8,0.2]");
    assert!(Dictionary::from_json(&bad_range).is_err());
    let no_weight = TINY.replace(",\"weight\":0.4", "");
    assert!(Dictionary::from_json(&no_weight).is_err());
}

#[test]
fn class_filter_equals_roundtrip() {
    let d = Dictionary::bundled();
    let p = d.parse("tối giản Hàn");
    let sf = p.filters.iter().find(|f| f.axis == "serif_foot").unwrap();
    assert_eq!(sf.equals.as_deref(), Some("sans"));
    let mut f = face(&[]);
    f.cls.insert("serif_foot", "serif");
    // preferred lệch lớp → trừ điểm, không loại
    assert!(evaluate(&p.filters, &p.hard, &f).unwrap() < 0.0);
}

#[test]
fn parse_ids_selects_terms_without_text_and_overrides_strength() {
    use fontatlas_describe::{Dictionary, Strength};
    let d = Dictionary::bundled();
    let p = d.parse_ids(&[("warm", Some(Strength::Required)), ("editorial", None), ("warm", None), ("khong_co", None)]);
    assert_eq!(p.matched.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["warm", "editorial"], "id trùng chỉ một lần, theo thứ tự");
    assert_eq!(p.unknown_words, vec!["khong_co"]);
    assert!(p.filters.iter().filter(|f| f.term_id == "warm").all(|f| f.strength == Strength::Required));
    assert!(!p.filters.is_empty());
    // khớp với parse theo văn bản khi không ghi đè độ chặt
    let by_text = d.parse("warm");
    let by_id = d.parse_ids(&[("warm", None)]);
    assert_eq!(by_text.filters, by_id.filters);
    assert!(d.has_id("warm") && !d.has_id("khong_co"));
    // mục không ánh xạ được trả lý do của từ điển
    let pk = d.parse_ids(&[("packaging", None)]);
    assert_eq!(pk.unmappable.len(), 1);
    assert!(pk.filters.is_empty());
    // hai term đối nghịch cùng trục → mâu thuẫn, không tự chọn
    let c = d.parse_ids(&[("warm", None), ("cool", None)]);
    assert!(!c.conflicts.is_empty());
}
