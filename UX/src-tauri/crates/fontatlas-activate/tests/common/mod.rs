#![allow(dead_code)]
//! Store tạm có font thật (fixture OFL copy vào thư mục tạm) + FakeEngine đã "biết" PS name của từng file.
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fontatlas_activate::{ActivationService, FakeEngine};
use fontatlas_store::{Analysis, FaceKey, IngestItem, ScannedFile, SourceKind, Store};

pub struct Env {
    pub dir: tempfile::TempDir,
    pub store: Mutex<Store>,
    pub fonts: PathBuf,
    pub source: i64,
}

pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures").join(name)
}

/// Copy fixture vào `<tmp>/fonts`, thêm nguồn thư mục rồi rescan.
pub fn env_with(names: &[&str]) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let fonts = dir.path().join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in names {
        std::fs::copy(fixture(n), fonts.join(n)).unwrap();
    }
    let (mut store, _) = Store::open(&dir.path().join("db")).unwrap();
    let source = store.add_source(SourceKind::Directory, "t", Some(&fonts)).unwrap();
    store.rescan_source(source).unwrap();
    Env { dir, store: Mutex::new(store), fonts, source }
}

pub fn rescan(env: &Env) {
    env.store.lock().unwrap().rescan_source(env.source).unwrap();
}

/// Ingest một file ngoài thư mục nguồn: nội dung = fixture + `seed` (nên mỗi `seed` có hash thật khác nhau,
/// khớp với kiểm tra hash của service) và các face có PS name cho trước — để có hai file khác hash mà cùng
/// PS name, hoặc một "TTC" nhiều face.
pub fn ingest_fake(env: &Env, file_name: &str, seed: &str, ps_names: &[&str]) -> [u8; 32] {
    ingest_fake_from(env, file_name, seed, ps_names, "libre_baskerville.ttf")
}

/// Như `ingest_fake` nhưng nền là fixture `base` (vd `oswald_vf.ttf`: font variable thật).
pub fn ingest_fake_from(env: &Env, file_name: &str, seed: &str, ps_names: &[&str], base: &str) -> [u8; 32] {
    let extra = env.dir.path().join("extra");
    std::fs::create_dir_all(&extra).unwrap();
    let path = extra.join(file_name);
    let mut bytes = std::fs::read(fixture(base)).unwrap();
    bytes.extend_from_slice(seed.as_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let hash = *blake3::hash(&bytes).as_bytes();
    let mut r = fontatlas_core::analyze_path(&fixture(base)).unwrap();
    r.hash_hex = blake3::hash(&bytes).to_hex().to_string();
    let base = r.faces[0].clone();
    r.faces = ps_names
        .iter()
        .enumerate()
        .map(|(i, ps)| {
            let mut f = base.clone();
            f.index = i as u32;
            f.metadata.postscript_name = Some(ps.to_string());
            f
        })
        .collect();
    let item = IngestItem {
        source_id: 999,
        file: ScannedFile { path: path.clone(), size: bytes.len() as u64, mtime_ns: 1, created_ns: None },
        hash,
        analysis: Analysis::Report(r),
    };
    env.store.lock().unwrap().ingest(vec![item], 1).unwrap();
    hash
}

/// Dạy FakeEngine PS name của mọi vị trí hiện có trong store (gọi lại sau khi đổi tên/rescan).
pub fn teach(engine: &FakeEngine, env: &Env) {
    let s = env.store.lock().unwrap();
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

pub fn service(env: &Env) -> ActivationService<FakeEngine> {
    service_with(env, FakeEngine::new())
}

pub fn service_with(env: &Env, engine: FakeEngine) -> ActivationService<FakeEngine> {
    teach(&engine, env);
    ActivationService::new(engine)
}

/// Face 0 của file có tên `file_name` trong thư mục nguồn.
pub fn key_of(env: &Env, file_name: &str) -> FaceKey {
    let s = env.store.lock().unwrap();
    let h: Vec<u8> = s
        .conn()
        .query_row(
            "SELECT f.hash FROM main.files f JOIN main.file_locations l USING(file_id) WHERE l.path LIKE ?1 OR l.path LIKE ?2",
            // Dấu phân cách đường dẫn là `/` (Unix) hoặc `\\` (Windows).
            [format!("%/{file_name}"), format!("%\\{file_name}")],
            |r| r.get(0),
        )
        .unwrap();
    FaceKey { hash: h.try_into().unwrap(), face_index: 0 }
}

pub fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap()
}

pub fn no_progress() -> impl FnMut(fontatlas_activate::Progress) {
    |_| {}
}
