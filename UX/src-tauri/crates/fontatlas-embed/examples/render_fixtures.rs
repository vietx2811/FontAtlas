//! Ghi ảnh vàng tests/data/render/<fixture>.png (PNG xám 8-bit) cho các fixture Latin của core.
use std::path::PathBuf;

const FIXTURES: [&str; 6] = [
    "roboto.ttf",
    "open_sans.ttf",
    "playfair_display.ttf",
    "source_code_pro.ttf",
    "public_sans_vf.ttf",
    "libre_baskerville.ttf",
];

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("tests/data/render");
    std::fs::create_dir_all(&out_dir).expect("mkdir");
    for name in FIXTURES {
        let bytes = std::fs::read(root.join("../fontatlas-core/tests/fixtures").join(name)).expect("fixture");
        let f = fontatlas_core::FontFile::from_bytes(bytes).expect("font");
        let faces = f.faces().expect("faces");
        let font = faces[0].font();
        let img = fontatlas_embed::render_input(font, &fontatlas_core::standard_location(font)).expect("render");
        let file = std::fs::File::create(out_dir.join(format!("{name}.png"))).expect("create");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 224, 224);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().expect("header").write_image_data(&img.0).expect("data");
        println!("{name}");
    }
}
