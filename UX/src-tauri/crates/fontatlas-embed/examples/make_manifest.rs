//! Tính data/model-manifest.json từ model.safetensors chính thức đã có trong máy (không cần mạng).
//! cargo run --release --example make_manifest -- <model.safetensors> > data/model-manifest.json
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
    println!("{}", serde_json::to_string_pretty(&m).unwrap());
}
