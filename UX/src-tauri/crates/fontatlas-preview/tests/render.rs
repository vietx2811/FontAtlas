mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_preview::*;

/// Giải mã PNG GrayscaleAlpha → (w, h, kênh alpha).
fn alpha(r: &Rendered) -> (u32, u32, Vec<u8>) {
    let dec = png::Decoder::new(std::io::Cursor::new(&r.png));
    let mut rd = dec.read_info().unwrap();
    let mut buf = vec![0; rd.output_buffer_size().unwrap()];
    let info = rd.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::GrayscaleAlpha);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    (info.width, info.height, buf[..info.buffer_size()].chunks(2).map(|p| p[1]).collect())
}
fn render(path: &std::path::Path, idx: u32, text: &str, px: u32) -> Rendered {
    let f = file(path);
    let faces = f.faces().unwrap();
    render_face(&faces[idx as usize], text, px).unwrap()
}

#[test]
fn png_has_bucket_size_and_ink() {
    for px in PX_BUCKETS {
        let r = render(&core_fixture("open_sans.ttf"), 0, DEFAULT_SAMPLE, px);
        let (w, h, a) = alpha(&r);
        assert_eq!((w, h), canvas_size(px));
        assert_eq!((r.width, r.height), (w, h));
        assert!(a.iter().any(|&v| v > 128), "phải có mực ở px {px}");
        assert_eq!(r.source, TextSource::Requested);
    }
}

#[test]
fn stacked_marks_are_not_clipped() {
    let r = render(&core_fixture("open_sans.ttf"), 0, "ỖẪẬ Ấp", 48);
    let (w, h, a) = alpha(&r);
    let row = |y: u32| &a[(y * w) as usize..((y + 1) * w) as usize];
    for y in [0, 1, h - 2, h - 1] {
        assert!(row(y).iter().all(|&v| v == 0), "hàng {y} phải trống (không cắt mép)");
    }
    assert!(a.iter().any(|&v| v > 128));
}

#[test]
fn ttc_face_one_matches_standalone() {
    let ttc = render(&fixture("pair.ttc"), 1, DEFAULT_SAMPLE, 48);
    let solo = render(&core_fixture("playfair_display.ttf"), 0, DEFAULT_SAMPLE, 48);
    let face0 = render(&fixture("pair.ttc"), 0, DEFAULT_SAMPLE, 48);
    assert_eq!(alpha(&ttc).2, alpha(&solo).2, "face 1 trong TTC phải vẽ đúng Playfair");
    assert_ne!(alpha(&ttc).2, alpha(&face0).2, "face 0 và face 1 phải khác nhau");
}

#[test]
fn missing_glyphs_fall_back_not_blank() {
    let r = render(&core_fixture("open_sans.ttf"), 0, "漢字かな", 48);
    assert_eq!((r.source, r.text.as_str()), (TextSource::LatinFallback, LATIN_FALLBACK));
    assert!(alpha(&r).2.iter().any(|&v| v > 128));
}

#[test]
fn cmap_sample_skips_controls_spaces_and_pua() {
    let f = file(&core_fixture("roboto.ttf"));
    let faces = f.faces().unwrap();
    let s = cmap_sample(faces[0].font());
    assert_eq!(s.chars().count(), CMAP_SAMPLE_CHARS);
    assert!(s.chars().all(|c| !c.is_control() && !c.is_whitespace() && !('\u{E000}'..='\u{F8FF}').contains(&c)), "{s:?}");
}

#[test]
fn long_text_is_truncated_without_partial_glyph() {
    let r = render(&core_fixture("roboto.ttf"), 0, &"W".repeat(60), 48);
    assert!(r.truncated);
    let (w, _, a) = alpha(&r);
    assert!(a.chunks(w as usize).all(|row| row[w as usize - 1] == 0), "vùng đệm phải không có mực");
}

#[test]
fn no_outline_font_is_no_outlines() {
    let raw = std::fs::read(core_fixture("roboto.ttf")).unwrap();
    let raw = rename_table(rename_table(raw, b"glyf", b"glyX"), b"loca", b"locX");
    let f = FontFile::from_bytes(raw).unwrap();
    let faces = f.faces().unwrap();
    let e = render_face(&faces[0], DEFAULT_SAMPLE, 48).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("no_outlines", 422));
}

#[test]
fn bad_size_rejected_before_render() {
    let f = file(&core_fixture("roboto.ttf"));
    let faces = f.faces().unwrap();
    assert_eq!(render_face(&faces[0], "A", 50).unwrap_err().code(), "bad_size");
}

#[test]
#[ignore = "hiệu năng: cargo test -p fontatlas-preview --release --test render -- --ignored --nocapture"]
fn render_under_1ms_per_face() {
    let files: Vec<FontFile> = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf"]
        .iter().map(|n| file(&core_fixture(n))).collect();
    let faces: Vec<_> = files.iter().map(|f| f.faces().unwrap().remove(0)).collect();
    let n = 200;
    let t = std::time::Instant::now();
    for _ in 0..n {
        for f in &faces { render_face(f, DEFAULT_SAMPLE, 48).unwrap(); }
    }
    let per = t.elapsed().as_secs_f64() * 1e3 / (n * faces.len()) as f64;
    eprintln!("render_face: {per:.3} ms/face (đơn luồng)");
    assert!(per <= 1.0, "spec 7.1: ~0,67 ms, ngưỡng 1 ms");
}
