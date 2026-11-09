mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_preview::missing_graphemes;

fn missing(font: &str, text: &[&str]) -> Vec<String> {
    let f: FontFile = file(&core_fixture(font));
    let faces = f.faces().unwrap();
    missing_graphemes(faces[0].font(), &text.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn latin_is_never_reported_missing_for_any_fixture_font() {
    // Chống báo nhầm: phép dò cũ coi mọi ký tự của font trùng font hệ thống là thiếu.
    for font in ["roboto.ttf", "open_sans.ttf", "bebas_neue.ttf", "fira_sans_regular.ttf", "playfair_display.ttf", "source_code_pro.ttf", "libre_baskerville.ttf", "dancing_script_vf.ttf", "oswald_vf.ttf", "public_sans_vf.ttf"] {
        let m = missing(font, &["H", "a", "x", "g", "Q", "1", "-"]);
        assert!(m.is_empty(), "{font}: {m:?}");
    }
}

#[test]
fn cjk_is_missing_from_latin_fonts() {
    assert_eq!(missing("roboto.ttf", &["a", "龍", "龍"]), vec!["龍", "龍"]);
    assert_eq!(missing("bebas_neue.ttf", &["龍"]), vec!["龍"]);
}

#[test]
fn whitespace_and_controls_are_skipped() {
    assert!(missing("bebas_neue.ttf", &[" ", "\n", "\t"]).is_empty());
    assert!(missing("roboto.ttf", &[" ", "\n", "\t", "\r"]).is_empty());
}

#[test]
fn vietnamese_precomposed_and_decomposed_agree_on_a_full_font() {
    let nfc = "ấ"; let nfd: String = "a\u{0302}\u{0301}".into();
    assert!(missing("roboto.ttf", &[nfc, &nfd, "ở", "ặ", "Ữ"]).is_empty());
}

#[test]
fn a_mark_the_font_lacks_makes_the_grapheme_missing_but_the_base_alone_is_fine() {
    // U+20DD (khung tròn bao quanh) hầu như không có trong font Latin; chữ nền "a" thì có.
    let m = missing("roboto.ttf", &["a", "a\u{20DD}"]);
    assert_eq!(m, vec!["a\u{20DD}"]);
}

#[test]
fn joiners_and_variation_selectors_do_not_need_glyphs() {
    assert!(missing("roboto.ttf", &["a\u{FE0F}", "a\u{200D}"]).is_empty());
}
