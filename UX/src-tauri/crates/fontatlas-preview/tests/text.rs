mod common;
use fontatlas_preview::*;

#[test]
fn normalize_text_nfc_trims_and_limits() {
    assert_eq!(normalize_text("  A\u{0306}\u{0301}n  "), "\u{1EAE}n"); // NFD → NFC "Ắn"
    assert_eq!(normalize_text("   "), DEFAULT_SAMPLE);
    assert_eq!(normalize_text(&"ơ".repeat(100)).chars().count(), MAX_TEXT_CHARS);
}

#[test]
fn canvas_size_and_buckets() {
    assert_eq!(canvas_size(48), (624, 112));
    assert_eq!(canvas_size(32), (416, 74));
    assert!(check_px(48).is_ok());
    assert_eq!(check_px(50).unwrap_err().code(), "bad_size");
    assert_eq!(check_px(50).unwrap_err().http_status(), 400);
}

#[test]
fn pick_text_keeps_requested_when_covered() {
    let (t, s) = pick_text("Ăn phở", |_| true, || unreachable!());
    assert_eq!((t.as_str(), s), ("Ăn phở", TextSource::Requested));
}

#[test]
fn pick_text_ignores_spaces_when_checking_coverage() {
    let (_, s) = pick_text("A B", |c| c != ' ', || unreachable!());
    assert_eq!(s, TextSource::Requested);
}

#[test]
fn pick_text_falls_back_to_latin() {
    let (t, s) = pick_text("Ăn phở", |c| c.is_ascii(), || unreachable!());
    assert_eq!((t.as_str(), s), (LATIN_FALLBACK, TextSource::LatinFallback));
}

#[test]
fn pick_text_uses_cmap_sample_for_non_latin_font() {
    let (t, s) = pick_text("Ăn phở", |c| !c.is_ascii(), || "αβγ".into());
    assert_eq!((t.as_str(), s), ("αβγ", TextSource::CmapSample));
    let (t, s) = pick_text("Ăn phở", |_| false, String::new);
    assert_eq!((t.as_str(), s), ("", TextSource::Nothing));
}

#[test]
fn fit_keeps_scale_when_ink_inside() {
    let (k, base) = render::fit(40.0, -10.0, 35.0, -8.0, 112, 4.0);
    assert_eq!((k, base), (1.0, 44.0));
}

#[test]
fn fit_shrinks_when_ink_exceeds_canvas() {
    // dấu chồng đưa mực lên 150 → co để (top - bottom)·k = 104
    let (k, base) = render::fit(40.0, -10.0, 150.0, -10.0, 112, 4.0);
    assert!((k - 104.0 / 160.0).abs() < 1e-5, "{k}");
    assert!((base - (4.0 + 150.0 * k)).abs() < 1e-4);
}

#[test]
fn truncate_at_never_keeps_partial_glyph() {
    // canvas 100, đệm 6 → mép phải tối đa 94
    assert_eq!(render::truncate_at(&[30.0, 60.0, 90.0, 120.0], 1.0, 100, 6.0), 2); // 6+90 = 96 > 94
    assert_eq!(render::truncate_at(&[30.0, 60.0], 0.5, 100, 6.0), 2);
    assert_eq!(render::truncate_at(&[200.0], 1.0, 100, 6.0), 0);
}

#[test]
fn nothing_fits_is_an_explicit_error() {
    let e = render::require_fit(0).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("nothing_fits", 422));
    assert_eq!(render::require_fit(3).unwrap(), 3);
}
