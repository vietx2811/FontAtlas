//! In báo cáo JSON: cargo run -p fontatlas-core --example dump -- <file…>
fn main() {
    let paths: Vec<std::path::PathBuf> = std::env::args().skip(1).map(Into::into).collect();
    for (p, r) in fontatlas_core::analyze_paths(&paths) {
        match r {
            Ok(rep) => println!("{}", serde_json::to_string_pretty(&rep).expect("json")),
            Err(e) => eprintln!("{}: {} ({})", p.display(), e, e.code()),
        }
    }
}
