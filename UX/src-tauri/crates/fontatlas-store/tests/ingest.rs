mod common;
use common::*;
use fontatlas_core::ANALYZER_VERSION;
use fontatlas_store::{Analysis, FaceKey, IngestItem, ScannedFile};

#[test]
fn report_writes_file_face_analysis_and_state() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("open_sans.ttf");
    let sum = s.ingest(vec![item(1, "/fonts/OpenSans.ttf", &r)], 1000).unwrap();
    assert_eq!(sum.faces_written, 1);
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE missing=0"), 1);
    let (ver, src, meta): (String, String, String) = s.conn().query_row(
        "SELECT analyzer_version, source, metadata_json FROM analyses", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(ver, ANALYZER_VERSION);
    assert_eq!(src, "local");
    assert!(meta.contains("Open Sans"));
    let (fam, key, vi, svi): (String, String, String, i64) = s.conn().query_row(
        "SELECT family_name, family_key, vi_status, supports_vi FROM faces", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
    assert_eq!(fam, "Open Sans");
    assert_eq!(key, "open sans");
    assert_eq!(vi, "full");
    assert_eq!(svi, 1);
    let (w, ng, full, ps): (Option<i64>, Option<i64>, Option<String>, Option<String>) = s.conn().query_row(
        "SELECT weight_class, num_glyphs, full_name, postscript_name FROM faces", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
    let m = &r.faces[0].metadata;
    assert_eq!(w, m.weight_class.map(i64::from));
    assert_eq!(ng, m.num_glyphs.map(i64::from));
    assert_eq!(full, m.full_name);
    assert_eq!(ps, m.postscript_name);
    let st: String = s.conn().query_row(
        "SELECT status FROM pipeline_state WHERE face_index=-1 AND stage='load'", [], |r| r.get(0)).unwrap();
    assert_eq!(st, "done");
    fts_ok(&s);
    assert_eq!(fts_hits(&s, "open sans"), 1);
}

#[test]
fn reingest_is_idempotent_and_face_id_stable() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 1).unwrap();
    let id1 = count(&s, "SELECT face_id FROM faces");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 2).unwrap();
    assert_eq!(count(&s, "SELECT face_id FROM faces"), id1);
    assert_eq!(count(&s, "SELECT count(*) FROM analyses"), 1);
    fts_ok(&s);
    assert_eq!(fts_hits(&s, "roboto"), 1);
}

#[test]
fn old_local_analysis_versions_are_replaced() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 1).unwrap();
    s.conn().execute("UPDATE analyses SET analyzer_version='core-0.0.0'", []).unwrap();
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 2).unwrap();
    let vers: Vec<String> = s.conn().prepare("SELECT analyzer_version FROM analyses").unwrap()
        .query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect();
    assert_eq!(vers, vec![ANALYZER_VERSION.to_string()]);
}

#[test]
fn collection_report_gives_one_row_per_face() {
    let d = tmp();
    let mut s = open_store(d.path());
    let mut r = fake_report(&report("roboto.ttf"), "ttc", "Duo");
    let mut second = r.faces[0].clone();
    second.index = 1;
    second.metadata.subfamily = Some("Bold".into());
    r.faces.push(second);
    r.container = fontatlas_core::Container::Collection;
    s.ingest(vec![item(1, "/a/Duo.ttc", &r)], 1).unwrap();
    let idx: Vec<i64> = s.conn().prepare("SELECT face_index FROM faces ORDER BY face_index").unwrap()
        .query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect();
    assert_eq!(idx, vec![0, 1]);
    assert_eq!(count(&s, "SELECT num_faces FROM files"), 2);
    assert_eq!(count(&s, "SELECT count(*) FROM files WHERE container='collection'"), 1);
}

#[test]
fn failure_is_recorded_with_code_path_and_attempts() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = *blake3::hash(b"rac").as_bytes();
    let fail = || IngestItem {
        source_id: 1,
        file: ScannedFile { path: "/a/broken.ttf".into(), size: 3, mtime_ns: 1 },
        hash: h,
        analysis: Analysis::Failed { code: "not_a_font".into(), detail: "không phải file font".into() },
    };
    assert_eq!(s.ingest(vec![fail()], 1).unwrap().failed, 1);
    s.ingest(vec![fail()], 2).unwrap();
    let p = s.problem_files().unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].code, "not_a_font");
    assert_eq!(p[0].stage, "load");
    assert_eq!(p[0].face_index, None);
    assert_eq!(p[0].attempts, 2);
    assert_eq!(p[0].path.as_deref(), Some("/a/broken.ttf"));
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 0);
    assert_eq!(s.retry_failed().unwrap(), 1);
    assert!(s.problem_files().unwrap().is_empty());
}

#[test]
fn known_relinks_and_unknown_known_is_error() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 1).unwrap();
    let known = IngestItem { analysis: Analysis::Known, ..item(1, "/b/Roboto copy.ttf", &r) };
    assert_eq!(s.ingest(vec![known], 2).unwrap().relinked, 1);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2);
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 1);
    let ghost = IngestItem { hash: [7; 32], analysis: Analysis::Known, ..item(1, "/c/x.ttf", &r) };
    assert_eq!(s.ingest(vec![ghost], 3).unwrap_err().code(), "not_found");
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2, "lô lỗi phải rollback");
}

#[test]
fn report_hash_must_match_item_hash() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    let bad = IngestItem { hash: [1; 32], ..item(1, "/a/R.ttf", &r) };
    assert_eq!(s.ingest(vec![bad], 1).unwrap_err().code(), "bad_hash");
}

#[test]
fn face_key_hex_roundtrip_and_validation() {
    let k = FaceKey::from_hex(&"ab".repeat(32), 3).unwrap();
    assert_eq!(k.hash_hex(), "ab".repeat(32));
    assert_eq!(FaceKey::from_hex("xyz", 0).unwrap_err().code(), "bad_hash");
}
