#![allow(dead_code)]
//! Thư viện tạm có font OFL thật + FakeEngine đã "biết" PS name của từng file.
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fontatlas_activate::{ActivationService, FakeEngine};
use fontatlas_broker::OwnedBackend;
use fontatlas_store::{SourceKind, Store};

pub struct Env {
    pub dir: tempfile::TempDir,
    pub fonts: PathBuf,
}

pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures").join(name)
}

pub const LIBRE: &str = "libre_baskerville.ttf";
pub const ROBOTO: &str = "roboto.ttf";
pub const LIBRE_PS: &str = "LibreBaskerville-Regular";
pub const ROBOTO_PS: &str = "Roboto-Regular";

/// Backend với các fixture đã quét; `engine` tuỳ biến trước khi dạy tên.
pub fn backend_with(names: &[&str], engine: FakeEngine) -> (Env, OwnedBackend<FakeEngine>) {
    let dir = tempfile::tempdir().unwrap();
    let fonts = dir.path().join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in names {
        std::fs::copy(fixture(n), fonts.join(n)).unwrap();
    }
    let (mut store, _) = Store::open(&dir.path().join("db")).unwrap();
    let source = store.add_source(SourceKind::Directory, "t", Some(&fonts)).unwrap();
    store.rescan_source(source).unwrap();
    teach(&engine, &store);
    (Env { dir, fonts }, OwnedBackend { service: ActivationService::new(engine), store: Mutex::new(store) })
}

pub fn backend(names: &[&str]) -> (Env, OwnedBackend<FakeEngine>) {
    backend_with(names, FakeEngine::new())
}

pub fn teach(engine: &FakeEngine, s: &Store) {
    let hashes: Vec<Vec<u8>> = {
        let mut st = s.conn().prepare("SELECT hash FROM main.files").unwrap();
        st.query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    for h in hashes {
        let h: [u8; 32] = h.try_into().unwrap();
        let t = s.file_target(&h).unwrap().unwrap();
        let names: Vec<String> = t.faces.iter().filter_map(|f| f.postscript_name.clone()).collect();
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        for loc in &t.locations {
            if let Ok(c) = std::fs::canonicalize(&loc.path) {
                engine.set_names_for(&c, &refs);
            }
            engine.set_names_for(&loc.path, &refs);
        }
    }
}

pub fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap()
}
