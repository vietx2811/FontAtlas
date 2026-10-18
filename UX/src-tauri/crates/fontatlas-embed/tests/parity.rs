mod common;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use common::*;
use fontatlas_embed::{CandleEncoder, Encode, Gray224, Whitening, bundled_manifest, to_tensor};

#[test]
fn bundled_assets_valid() {
    let w = Whitening::bundled();
    assert_eq!((w.raw_dim, w.dim), (768, 128));
    assert!(w.version.starts_with("white-v1"));
    let m = bundled_manifest();
    assert_eq!((m.sha256.len(), m.license.as_str()), (64, "Apache-2.0"));
    assert!(m.size > 300_000_000);
    for f in &m.notice_files {
        assert!(Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(f).exists(), "{f}");
    }
}

#[test]
#[ignore = "cần model thật: FONTATLAS_SIGLIP=<model.safetensors>"]
fn real_model_matches_pytorch_on_golden_pngs() {
    let mut e = CandleEncoder::load_best(Path::new(&std::env::var("FONTATLAS_SIGLIP").unwrap())).unwrap();
    let refs: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("data/torch_ref.json")).unwrap();
    for chunk in SAMPLES_LATIN.chunks(4) {
        let ts: Vec<Vec<f32>> = chunk
            .iter()
            .map(|f| {
                let mut t = vec![0.0; 3 * 224 * 224];
                to_tensor(&Gray224(load_golden_png(f)), &mut t);
                t
            })
            .collect();
        for (f, o) in chunk.iter().zip(e.encode(&ts, &AtomicBool::new(false)).unwrap()) {
            let r: Vec<f32> = serde_json::from_value(refs[*f].clone()).unwrap();
            let c = cos(&o, &r);
            println!("parity {f}: cos {c:.6}");
            assert!(c >= 0.9999, "{f}: cos {c}");
        }
    }
}
