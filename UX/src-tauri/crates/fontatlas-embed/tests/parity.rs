mod common;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use common::*;
use fontatlas_embed::{CandleEncoder, Encode, Gray224, ModelStore, Whitening, bundled_manifest, sha256_file, to_tensor};

#[test]
fn bundled_assets_valid() {
    let w = Whitening::bundled();
    assert_eq!((w.raw_dim, w.dim, w.version.as_str()), (768, 128, "white-v2"));
    let m = bundled_manifest();
    assert_eq!((m.sha256.len(), m.license.as_str()), (64, "Apache-2.0"));
    assert!(m.source.url.starts_with("https://huggingface.co/google/siglip2-base-patch16-224/resolve/75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2/"));
    assert_eq!(m.source.sha256, "612923381c76ec5a9bed335d1c48827e3f2e506ac31b044b63b2031fadee6a0b");
    assert_eq!(m.source.vision_offset + m.source.vision_len, 1_500_751_880 + 8 + m.source.header_len);
    assert!(m.size > 300_000_000 && m.size < m.source.size);
    for f in &m.notice_files {
        assert!(Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(f).exists(), "{f}");
    }
}

/// Server Range đọc thẳng từ đĩa (file 1,5 GB không nạp vào RAM).
fn serve_file_ranges(path: std::path::PathBuf) -> (std::sync::Arc<tiny_http::Server>, String) {
    use std::io::{Read, Seek, SeekFrom};
    let server = std::sync::Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let url = format!("http://{}/model.safetensors", server.server_addr().to_ip().unwrap());
    let s2 = server.clone();
    std::thread::spawn(move || {
        for rq in s2.incoming_requests() {
            let total = std::fs::metadata(&path).unwrap().len();
            let r = rq.headers().iter().find(|h| h.field.equiv("Range")).map(|h| h.value.as_str().to_string());
            let (a, b) = r
                .and_then(|r| {
                    let (a, b) = r.strip_prefix("bytes=")?.split_once('-')?;
                    Some((a.parse::<u64>().ok()?, b.parse::<u64>().ok()?))
                })
                .unwrap();
            let mut f = std::fs::File::open(&path).unwrap();
            f.seek(SeekFrom::Start(a)).unwrap();
            let cr = format!("bytes {a}-{b}/{total}");
            let h = tiny_http::Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()).unwrap();
            let _ = rq.respond(tiny_http::Response::new(206.into(), vec![h], f.take(b - a + 1), Some((b - a + 1) as usize), None));
        }
    });
    (server, url)
}

#[test]
#[ignore = "cần file HF thật: FONTATLAS_SIGLIP=<model.safetensors> (không cần mạng)"]
fn real_range_download_assembles_pinned_file_and_matches_torch() {
    let src = std::path::PathBuf::from(std::env::var("FONTATLAS_SIGLIP").unwrap());
    let (_srv, url) = serve_file_ranges(src);
    let mut m = bundled_manifest();
    m.source.url = url;
    let d = tempfile::tempdir().unwrap();
    let local = ModelStore::new(d.path().into(), m.clone()).ensure(&|_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(sha256_file(&local).unwrap(), m.sha256);
    check_parity(CandleEncoder::load_best(&local).unwrap());
}

fn check_parity(mut e: CandleEncoder) {
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
            println!("parity {} {f}: cos {c:.7}", e.device_id());
            assert!(c >= 0.9999, "{f}: cos {c}");
        }
    }
}

#[test]
#[ignore = "cần model thật: FONTATLAS_SIGLIP=<model.safetensors>"]
fn real_model_matches_pytorch_on_golden_pngs() {
    check_parity(CandleEncoder::load_best(Path::new(&std::env::var("FONTATLAS_SIGLIP").unwrap())).unwrap());
}
