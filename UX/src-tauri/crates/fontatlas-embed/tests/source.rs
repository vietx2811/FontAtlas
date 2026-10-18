mod common;
use common::*;
use fontatlas_embed::{EmbedWorker, StoreEmbeddingSource, Whitening, WorkerConfig};

fn run(d: &tempfile::TempDir) -> String {
    let cfg = WorkerConfig {
        store_dir: d.path().join("data"),
        raw_version: "raw-test|render-v2".into(),
        whitening: Some(Whitening::from_bytes(&identity_whitening_bytes(768, 128)).unwrap()),
        render_threads: 2,
    };
    let enc = tiny_768();
    let w = EmbedWorker::start(cfg, enc, Box::new(|_| {})).unwrap();
    let mv = w.model_version().unwrap().to_string();
    w.join().unwrap();
    mv
}

#[test]
fn reads_with_separate_connection_while_writer_open() {
    let d = store_dir_with(&[("roboto.ttf", 0), ("playfair_display.ttf", 0)]);
    let mv = run(&d);
    let _writer = open_data(&d); // writer vẫn mở (WAL)
    let src = StoreEmbeddingSource::open(&d.path().join("data"), &mv).unwrap();
    let all = src.load_all().unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|(_, v)| v.len() == 128));
    let keys: Vec<_> = all.iter().map(|x| x.0).collect();
    assert_eq!(src.fetch(&keys).unwrap().iter().filter(|v| v.is_some()).count(), 2);
}

#[test]
fn other_versions_invisible() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    run(&d);
    let src = StoreEmbeddingSource::open(&d.path().join("data"), "khong-co").unwrap();
    assert!(src.load_all().unwrap().is_empty());
}
