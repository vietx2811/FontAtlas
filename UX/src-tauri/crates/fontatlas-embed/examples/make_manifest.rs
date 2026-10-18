//! Tính data/model-manifest.json từ model.safetensors chính thức đã có trong máy (không cần mạng).
//! cargo run --release --example make_manifest -- <model.safetensors> [--extract <vision.safetensors>] > data/model-manifest.json
use std::path::Path;

const URL: &str =
    "https://huggingface.co/google/siglip2-base-patch16-224/resolve/75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2/model.safetensors";

fn main() {
    let src = std::env::args().nth(1).expect("đường dẫn model.safetensors");
    let m = fontatlas_embed::make_manifest(
        Path::new(&src),
        URL,
        "siglip2_base_p16_224_75de2d5_vision.safetensors",
        "Apache-2.0",
        vec!["model-NOTICE.txt".into(), "model-LICENSE-Apache-2.0.txt".into()],
    )
    .unwrap();
    let a: Vec<String> = std::env::args().collect();
    if let Some(i) = a.iter().position(|x| x == "--extract") {
        let dst = Path::new(&a[i + 1]);
        fontatlas_embed::extract_vision_file(Path::new(&src), dst).unwrap();
        assert_eq!(fontatlas_embed::sha256_file(dst).unwrap(), m.sha256, "file ghép phải khớp manifest");
        eprintln!("đã ghép {}", dst.display());
    }
    println!("{}", serde_json::to_string_pretty(&m).unwrap());
}
