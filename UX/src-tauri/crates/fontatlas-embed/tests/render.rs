mod common;
use common::*;
use fontatlas_embed::{EmbedError, Gray224, to_tensor};

#[test]
fn renders_with_ink_inside_margins() {
    for f in SAMPLES_LATIN {
        let img = render_fixture(f).unwrap();
        assert_eq!(img.0.len(), 224 * 224);
        let ink: Vec<usize> = (0..224 * 224).filter(|i| img.0[*i] < 128).collect();
        assert!(ink.len() > 2000, "{f}: {}", ink.len());
        let xs = ink.iter().map(|i| i % 224);
        assert!(xs.clone().min().unwrap() >= 10 && xs.max().unwrap() <= 213, "{f}");
    }
}

#[test]
fn tensor_normalisation_is_siglip() {
    let mut img = Gray224(vec![255; 224 * 224]);
    img.0[0] = 0;
    let mut t = vec![0f32; 3 * 224 * 224];
    to_tensor(&img, &mut t);
    assert_eq!(t[0], -1.0);
    assert_eq!(t[1], 1.0);
    assert_eq!(t[224 * 224], -1.0); // kênh G cùng giá trị
}

#[test]
fn golden_png_matches() {
    for f in SAMPLES_LATIN {
        let img = render_fixture(f).unwrap();
        let gold = load_golden_png(f);
        assert_eq!(gold.len(), 224 * 224, "{f}");
        let bad = img.0.iter().zip(&gold).filter(|(a, b)| (**a as i16 - **b as i16).abs() > 2).count();
        assert!(bad * 200 <= 224 * 224, "{f}: {bad} pixel lệch > 2");
    }
}

#[test]
fn missing_vietnamese_still_renders() {
    assert!(render_fixture("libre_baskerville.ttf").is_ok());
}

#[test]
fn no_cmap_is_no_glyphs() {
    let bytes = rename_table(fixture_bytes("roboto.ttf"), b"cmap", b"cmaq");
    assert!(matches!(render_bytes(bytes, 0), Err(EmbedError::NoGlyphs)));
}
