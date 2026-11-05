mod common;
use std::path::Path;

use common::*;
use fontatlas_store::{
    EmbedReader, FaceKey, JobResult, SETTING_EMBED_ENABLED, SETTING_LICENSE_NOTICE_ACK, SourceKind, Store,
};

const RAW: &str = "raw1";
const MV: &str = "raw1|white-a";

/// Store tạm có các fixture thật trong thư mục nguồn, đã rescan (face phân tích xong, có file_locations).
fn store_with(fixtures: &[&str]) -> (tempfile::TempDir, Store) {
    let d = tmp();
    let fonts = d.path().join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for f in fixtures {
        std::fs::copy(core_fixture(f), fonts.join(f)).unwrap();
    }
    let mut s = open_store(&d.path().join("data"));
    let id = s.add_source(SourceKind::Directory, "t", Some(&fonts)).unwrap();
    s.rescan_source(id).unwrap();
    (d, s)
}

fn reopen(d: &tempfile::TempDir, s: Store) -> Store {
    drop(s);
    open_store(&d.path().join("data"))
}

fn keys(s: &Store) -> Vec<FaceKey> {
    let mut st = s.conn().prepare("SELECT fi.hash, fa.face_index FROM faces fa JOIN files fi USING(file_id) ORDER BY fa.family_name").unwrap();
    st.query_map([], |r| {
        let h: Vec<u8> = r.get(0)?;
        Ok(FaceKey { hash: h.try_into().unwrap(), face_index: r.get::<_, i64>(1)? as u32 })
    })
    .unwrap()
    .map(|x| x.unwrap())
    .collect()
}

fn state_of(s: &Store, k: FaceKey, raw: &str) -> (String, Option<String>) {
    s.conn()
        .query_row(
            "SELECT status, error_code FROM pipeline_state WHERE hash=?1 AND face_index=?2 AND stage='embedding' AND version=?3",
            rusqlite::params![&k.hash[..], k.face_index, raw],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
}

fn done(n: usize) -> JobResult {
    JobResult::Done { raw: vec![0.5; n], white: Some(vec![0.1; 128]) }
}

fn finish_all(s: &mut Store, raw: &str, mv: &str) {
    s.enqueue_embeddings(raw).unwrap();
    let j = s.claim_embedding_batch(raw, 64).unwrap();
    let res: Vec<_> = j.iter().map(|x| (x.key, done(768))).collect();
    s.finish_embeddings(raw, mv, &res).unwrap();
}

#[test]
fn enqueues_all_fonts_regardless_of_license() {
    let (_d, mut s) = store_with(&["roboto.ttf", "open_sans.ttf"]);
    s.conn().execute("UPDATE faces SET license_class='other'", []).unwrap(); // font "thương mại" vẫn được embed
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 2);
}

#[test]
fn disabled_enqueues_nothing() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    assert!(s.embed_enabled().unwrap()); // mặc định bật
    s.set_setting(SETTING_EMBED_ENABLED, "0").unwrap();
    assert!(!s.embed_enabled().unwrap());
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 0);
    s.set_setting(SETTING_EMBED_ENABLED, "1").unwrap();
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 1);
}

#[test]
fn license_notice_flag_roundtrip() {
    let (_d, mut s) = store_with(&[]);
    assert_eq!(s.setting(SETTING_LICENSE_NOTICE_ACK).unwrap(), None);
    s.set_setting(SETTING_LICENSE_NOTICE_ACK, "1").unwrap();
    assert_eq!(s.setting(SETTING_LICENSE_NOTICE_ACK).unwrap().as_deref(), Some("1"));
}

#[test]
fn claim_returns_real_paths_and_marks_running() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 4).unwrap();
    assert_eq!(j.len(), 1);
    assert!(j[0].path.ends_with("roboto.ttf") && j[0].path.exists());
    assert_eq!(state_of(&s, j[0].key, RAW).0, "running");
    assert!(s.claim_embedding_batch(RAW, 4).unwrap().is_empty()); // không claim lại dòng running
}

#[test]
fn running_rows_reset_on_start() {
    let (d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    assert_eq!(s.claim_embedding_batch(RAW, 4).unwrap().len(), 1);
    let mut s2 = reopen(&d, s);
    assert_eq!(s2.reset_running_embeddings().unwrap(), 1);
    assert_eq!(s2.claim_embedding_batch(RAW, 4).unwrap().len(), 1);
}

#[test]
fn finish_ignores_non_running_or_lost_hash() {
    let (_d, mut s) = store_with(&["roboto.ttf", "playfair_display.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 4).unwrap();
    s.release_embeddings(RAW, &[j[0].key]).unwrap(); // không còn running
    s.conn().execute("PRAGMA foreign_keys=OFF", []).unwrap();
    s.conn().execute("DELETE FROM files WHERE hash = ?1", [&j[1].key.hash[..]]).unwrap(); // hash mất
    let n = s.finish_embeddings(RAW, MV, &[(j[0].key, done(768)), (j[1].key, done(768))]).unwrap();
    assert_eq!(n, 0);
    assert_eq!(s.count_embeddings(RAW).unwrap(), 0);
}

#[test]
fn finish_wrong_version_ignored() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 4).unwrap();
    assert_eq!(s.finish_embeddings("raw-khac", MV, &[(j[0].key, done(768))]).unwrap(), 0);
}

#[test]
fn finish_writes_raw_and_white_and_marks_done() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    finish_all(&mut s, RAW, MV);
    assert_eq!((s.count_embeddings(RAW).unwrap(), s.count_embeddings(MV).unwrap()), (1, 1));
    assert_eq!(state_of(&s, keys(&s)[0], RAW).0, "done");
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().already_done, 1);
}

#[test]
fn new_model_version_requeues_all() {
    let (_d, mut s) = store_with(&["roboto.ttf", "playfair_display.ttf"]);
    finish_all(&mut s, RAW, MV);
    assert_eq!(s.enqueue_embeddings("raw2").unwrap().pending, 2);
}

#[test]
fn rewhiten_from_raw_without_model() {
    let (_d, mut s) = store_with(&["roboto.ttf", "playfair_display.ttf"]);
    finish_all(&mut s, RAW, MV);
    assert_eq!(s.rewhiten(RAW, "raw1|white-b", &|r| Some(r[..128].to_vec())).unwrap(), 2);
    assert_eq!(s.count_embeddings("raw1|white-b").unwrap(), 2);
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 0); // không phải chạy lại model
    assert_eq!(s.gc_embeddings(&[RAW, "raw1|white-b"]).unwrap(), 2); // xoá white-a
}

#[test]
fn no_glyphs_is_skipped_and_not_requeued() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 4).unwrap();
    s.finish_embeddings(RAW, MV, &[(j[0].key, JobResult::Skipped { code: "no_glyphs" })]).unwrap();
    assert_eq!(state_of(&s, j[0].key, RAW), ("skipped".into(), Some("no_glyphs".into())));
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 0);
}

#[test]
fn missing_location_fails_with_missing_file() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    s.conn().execute("UPDATE file_locations SET missing=1", []).unwrap();
    assert!(s.claim_embedding_batch(RAW, 4).unwrap().is_empty());
    assert_eq!(state_of(&s, keys(&s)[0], RAW), ("failed".into(), Some("missing_file".into())));
}

#[test]
fn f32_roundtrip_stream_and_read_only_reader() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 1).unwrap();
    let raw: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
    s.finish_embeddings(RAW, MV, &[(j[0].key, JobResult::Done { raw: raw.clone(), white: Some(raw[..128].to_vec()) })]).unwrap();
    let ro = EmbedReader::open(s.dir()).unwrap(); // writer vẫn mở
    assert_eq!(ro.embeddings_f32(RAW, &[j[0].key]).unwrap()[0].as_deref(), Some(&raw[..]));
    let mut n = 0;
    ro.for_each_embedding(MV, &mut |_, v| {
        assert_eq!(v.len(), 128);
        n += 1
    })
    .unwrap();
    assert_eq!(n, 1);
    assert!(ro.conn().execute("DELETE FROM embeddings", []).is_err()); // chỉ đọc
}

#[test]
fn retry_failed_requeues_embedding_failures() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 1).unwrap();
    s.finish_embeddings(RAW, MV, &[(j[0].key, JobResult::Failed { code: "inference", detail: "x".into() })]).unwrap();
    s.retry_failed().unwrap();
    assert_eq!(s.claim_embedding_batch(RAW, 1).unwrap().len(), 1);
}

#[test]
fn settings_go_to_user_db_only() {
    let (d, mut s) = store_with(&[]);
    s.set_setting(SETTING_LICENSE_NOTICE_ACK, "1").unwrap();
    let v: String = rusqlite::Connection::open(d.path().join("data").join(fontatlas_store::USER_FILE))
        .unwrap()
        .query_row("SELECT value FROM settings WHERE key=?1", [SETTING_LICENSE_NOTICE_ACK], |r| r.get(0))
        .unwrap();
    assert_eq!(v, "1");
    let _ = Path::new(""); // giữ import
}

#[test]
fn failed_is_not_requeued_by_enqueue() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 1).unwrap();
    s.finish_embeddings(RAW, MV, &[(j[0].key, JobResult::Failed { code: "stale_file", detail: "x".into() })]).unwrap();
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 0); // thử lại chỉ qua retry_failed
    assert_eq!(state_of(&s, j[0].key, RAW).0, "failed");
}

#[test]
fn requeue_missing_only_when_location_is_back() {
    let (_d, mut s) = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    s.conn().execute("UPDATE file_locations SET missing=1", []).unwrap();
    s.claim_embedding_batch(RAW, 4).unwrap(); // → failed/missing_file
    assert_eq!(s.requeue_missing_embeddings(RAW).unwrap(), 0); // ổ vẫn chưa cắm
    s.conn().execute("UPDATE file_locations SET missing=0", []).unwrap();
    assert_eq!(s.requeue_missing_embeddings(RAW).unwrap(), 1);
    assert_eq!(state_of(&s, keys(&s)[0], RAW).0, "pending");
}
