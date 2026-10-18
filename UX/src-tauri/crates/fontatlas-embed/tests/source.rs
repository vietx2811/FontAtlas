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

/// Thêm nguồn thứ hai (thư mục riêng chứa một fixture), quét, trả source_id.
fn add_second_source(d: &tempfile::TempDir, fixture: &str) -> i64 {
    let dir = d.path().join("fonts2");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(fixture), fixture_bytes(fixture)).unwrap();
    let mut s = open_data(d);
    let id = s.add_source(fontatlas_store::SourceKind::Directory, "t2", Some(&dir)).unwrap();
    s.rescan_source(id).unwrap();
    id
}

#[test]
fn removed_font_embeddings_gone_from_source_after_gc() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    let src2 = add_second_source(&d, "playfair_display.ttf");
    let mv = run(&d);
    let raw = "raw-test|render-v2";
    let data = d.path().join("data");
    assert_eq!(StoreEmbeddingSource::open(&data, &mv).unwrap().load_all().unwrap().len(), 2);
    let gone = key_of(&open_data(&d), "Playfair Display");
    open_data(&d).remove_source(src2).unwrap(); // gọi gc()
    for v in [mv.as_str(), raw] {
        let all = StoreEmbeddingSource::open(&data, v).unwrap().load_all().unwrap();
        assert_eq!(all.len(), 1, "{v}");
        assert!(all.iter().all(|(k, _)| *k != gone), "{v}: vector của font đã xoá vẫn còn");
    }
    assert_eq!(StoreEmbeddingSource::open(&data, &mv).unwrap().fetch(&[gone]).unwrap(), vec![None]);
}

#[test]
fn finish_after_font_removed_writes_nothing() {
    let d = store_dir_with(&[("roboto.ttf", 0)]);
    let src2 = add_second_source(&d, "playfair_display.ttf");
    let raw = "raw-test|render-v2";
    let mut s = open_data(&d);
    s.enqueue_embeddings(raw).unwrap();
    let jobs = s.claim_embedding_batch(raw, 4).unwrap();
    assert_eq!(jobs.len(), 2);
    s.remove_source(src2).unwrap(); // font bị xoá khi lô đang chạy
    let results: Vec<_> = jobs
        .iter()
        .map(|j| (j.key, fontatlas_store::JobResult::Done { raw: vec![0.5; 768], white: Some(vec![0.5; 128]) }))
        .collect();
    assert_eq!(s.finish_embeddings(raw, "mv", &results).unwrap(), 1, "chỉ ghi face còn trong thư viện");
    s.gc().unwrap();
    let data = d.path().join("data");
    assert_eq!(StoreEmbeddingSource::open(&data, raw).unwrap().load_all().unwrap().len(), 1);
    assert_eq!(StoreEmbeddingSource::open(&data, "mv").unwrap().load_all().unwrap().len(), 1);
}
