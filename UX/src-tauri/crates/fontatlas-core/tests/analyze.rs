mod common;
use common::*;
use fontatlas_core::{ANALYZER_VERSION, analyze_path, analyze_paths};

#[test]
fn report_has_version_hash_and_faces() {
    let r = analyze_path(&fixture("open_sans.ttf")).unwrap();
    assert_eq!(r.analyzer_version, ANALYZER_VERSION);
    assert_eq!(r.hash_hex.len(), 64);
    assert_eq!(r.faces.len(), 1);
    assert!(r.faces[0].vietnamese.supports_vietnamese);
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"quality\""));
}

#[test]
fn batch_preserves_order_and_isolates_errors() {
    let mut paths: Vec<_> = SAMPLES.iter().map(|s| fixture(s)).collect();
    paths.insert(2, std::path::PathBuf::from("/nonexistent/broken.ttf"));
    let out = analyze_paths(&paths);
    assert_eq!(out.len(), paths.len());
    for (i, (p, r)) in out.iter().enumerate() {
        assert_eq!(p, &paths[i]);
        assert_eq!(r.is_err(), i == 2);
    }
    assert_eq!(out[2].1.as_ref().unwrap_err().code(), "io");
}

#[test]
fn woff2_and_ttf_give_same_analysis_different_hash() {
    let a = analyze_path(&fixture("open_sans.ttf")).unwrap();
    let b = analyze_path(&fixture("open_sans.woff2")).unwrap();
    assert_ne!(a.hash_hex, b.hash_hex);
    assert_eq!(a.faces[0].anatomy.contrast.value, b.faces[0].anatomy.contrast.value);
    assert_eq!(a.faces[0].vietnamese.missing, b.faces[0].vietnamese.missing);
}

#[test]
#[ignore = "đo hiệu năng trên font hệ thống: cargo test -p fontatlas-core --release -- --ignored --nocapture"]
fn perf_system_fonts() {
    let dir = std::path::Path::new("/System/Library/Fonts");
    let paths: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok().map(|e| e.path())).collect();
    let t = std::time::Instant::now();
    let out = analyze_paths(&paths);
    let faces: usize = out.iter().filter_map(|(_, r)| r.as_ref().ok()).map(|r| r.faces.len()).sum();
    eprintln!("{} file, {faces} face, {:?}", paths.len(), t.elapsed());
}

#[test]
fn apple_bitmap_font_with_bhed_is_accepted() {
    // Font bitmap Apple dùng 'bhed' thay 'head'; là file hợp lệ, chỉ không có outline.
    let Some(p) = system_font(&["/System/Library/Fonts/Supplemental/NISC18030.ttf"]) else { return };
    let r = analyze_path(&p).unwrap_or_else(|e| panic!("{e} ({})", e.code()));
    let f = &r.faces[0];
    assert!(f.metadata.family.is_some());
    assert_eq!(
        f.anatomy.x_height_em.quality,
        fontatlas_core::Quality::Failed(fontatlas_core::FailReason::NoOutlines)
    );
}
