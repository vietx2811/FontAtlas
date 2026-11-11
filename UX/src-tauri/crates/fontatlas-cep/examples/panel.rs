//! Cài/gỡ/xem trạng thái panel ở thư mục extensions thật của Adobe (dùng để thử tay và đo).
//! Dùng: panel <install|remove|status> <state_dir> <broker.json> [extensions_dir]
use fontatlas_cep::{PanelInstaller, default_extensions_dir};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!("panel <install|remove|status> <state_dir> <broker.json> [extensions_dir]");
        std::process::exit(2);
    }
    let ext = a.get(4).map(Into::into).or_else(default_extensions_dir).expect("HOME");
    let i = PanelInstaller::new(ext, &a[2], &a[3], env!("CARGO_PKG_VERSION"));
    match a[1].as_str() {
        "install" => println!("{:?}", i.install()),
        "remove" => println!("{:?}", i.remove()),
        _ => println!("{:?}", i.status()),
    }
}
