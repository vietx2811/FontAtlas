mod common;
use common::*;
use fontatlas_store::{FaceKey, SourceKind, Store, legacy_preview};

fn scanned() -> (tempfile::TempDir, std::path::PathBuf, Store, i64) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    for n in SAMPLES {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    s.rescan_source(src).unwrap();
    (d, fonts, s, src)
}

#[test]
fn reads_latest_analyses_with_face_id_and_sources() {
    let (_d, _f, s, src) = scanned();
    let mut rows = Vec::new();
    let n = s.for_each_latest_analysis(&mut |a| rows.push(a)).unwrap();
    assert_eq!((n, rows.len()), (5, 5));
    for a in &rows {
        assert_eq!(a.source_ids, vec![src]);
        assert_eq!(a.container.as_deref(), Some("sfnt"));
        assert!(a.face_id > 0 && a.file_size > 0 && !a.missing);
        let m: serde_json::Value = serde_json::from_str(&a.metadata_json).unwrap();
        assert!(m["family"].is_string());
    }
    let one = s.analysis_of(&rows[0].key).unwrap().unwrap();
    assert_eq!(one.face_id, rows[0].face_id);
    assert!(s.analysis_of(&FaceKey { hash: [9; 32], face_index: 0 }).unwrap().is_none());
}

#[test]
fn face_path_skips_missing_locations() {
    let (_d, fonts, s, src) = scanned();
    let mut rows = Vec::new();
    s.for_each_latest_analysis(&mut |a| rows.push(a)).unwrap();
    let k = rows[0].key.clone();
    assert!(s.face_path(&k).unwrap().unwrap().starts_with(&fonts));
    // thêm một vị trí thứ hai (id lớn hơn), rồi đánh dấu vị trí đầu missing
    s.conn()
        .execute(
            "INSERT INTO file_locations(file_id,source_id,path,size,mtime_ns,last_seen_at,missing)
             SELECT file_id,source_id,'/other/copy.ttf',size,mtime_ns,last_seen_at,0 FROM file_locations
             WHERE file_id=(SELECT file_id FROM files WHERE hash=?1) LIMIT 1",
            [&k.hash[..]],
        )
        .unwrap();
    s.conn()
        .execute("UPDATE file_locations SET missing=1 WHERE path<>'/other/copy.ttf' AND file_id=(SELECT file_id FROM files WHERE hash=?1)", [&k.hash[..]])
        .unwrap();
    assert_eq!(s.face_path(&k).unwrap().unwrap().to_string_lossy(), "/other/copy.ttf");
    let _ = src;
    s.conn().execute("UPDATE file_locations SET missing=1", []).unwrap();
    assert!(s.face_path(&k).unwrap().is_none());
}

#[test]
fn latest_analysis_used_until_reanalyzed() {
    let (_d, _f, s, _) = scanned();
    s.conn().execute("UPDATE analyses SET analyzer_version='core-0.0.1'", []).unwrap();
    let mut rows = Vec::new();
    assert_eq!(s.for_each_latest_analysis(&mut |a| rows.push(a)).unwrap(), 5);
    assert!(rows.iter().all(|a| a.analyzer_version == "core-0.0.1"));
    assert_eq!(s.stale_analysis_count().unwrap(), 5);
    // thêm dòng mới cho một face → chỉ dòng mới được trả, mỗi face một dòng
    s.conn()
        .execute(
            "INSERT INTO analyses(hash,face_index,analyzer_version,source,metadata_json,vietnamese_json,anatomy_json,analyzed_at)
             SELECT hash,face_index,?1,'local',metadata_json,vietnamese_json,anatomy_json,analyzed_at+1 FROM analyses
             WHERE rowid=(SELECT min(rowid) FROM analyses)",
            [fontatlas_core::ANALYZER_VERSION],
        )
        .unwrap();
    let mut rows = Vec::new();
    assert_eq!(s.for_each_latest_analysis(&mut |a| rows.push(a)).unwrap(), 5);
    assert_eq!(rows.iter().filter(|a| a.analyzer_version == fontatlas_core::ANALYZER_VERSION).count(), 1);
    assert_eq!(s.stale_analysis_count().unwrap(), 4);
}

#[test]
fn face_ids_of_skips_unknown_keys() {
    let (_d, _f, s, _) = scanned();
    let mut rows = Vec::new();
    s.for_each_latest_analysis(&mut |a| rows.push(a)).unwrap();
    let keys = vec![rows[2].key.clone(), FaceKey { hash: [7; 32], face_index: 0 }, rows[0].key.clone()];
    let got = s.face_ids_of(&keys).unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0], (rows[2].key.clone(), rows[2].face_id));
}

#[test]
fn legacy_preview_counts_without_writing() {
    let d = tmp();
    let fonts = d.path().join("F");
    std::fs::create_dir_all(&fonts).unwrap();
    let old = d.path().join("fontatlas.db");
    let p = |n: &str| fonts.join(n).to_string_lossy().to_string();
    let (a, b, c) = (p("a.ttf"), p("b.ttf"), p("c.ttf"));
    make_legacy_db(&old, true, &fonts, &[
        LegacyFont { id: "A", path: &a, family: "A", subfamily: "Regular", tags: r#"["x","y"]"#, collection: Some("col-1"), favorite: true, source: Some("src-1") },
        LegacyFont { id: "B", path: &b, family: "B", subfamily: "Regular", tags: r#"["x"]"#, collection: None, favorite: true, source: Some("src-1") },
        LegacyFont { id: "C", path: &c, family: "C", subfamily: "Regular", tags: "[]", collection: None, favorite: false, source: None },
    ]);
    let before = fingerprint(&old);
    let pv = legacy_preview(&old).unwrap();
    assert_eq!((pv.fonts, pv.favorites, pv.tags), (3, 2, 2));
    assert!(pv.sources >= 1 && pv.collections >= 1, "{pv:?}");
    assert_eq!(fingerprint(&old), before);
    let junk = d.path().join("junk.db");
    std::fs::write(&junk, vec![7u8; 4096]).unwrap();
    assert_eq!(legacy_preview(&junk).unwrap_err().code(), "legacy");
}

#[test]
fn backup_user_db_creates_readable_copy() {
    let d = tmp();
    let mut s = open_store(d.path());
    s.create_collection("A", None).unwrap();
    s.create_collection("B", None).unwrap();
    let p = s.backup_user_db().unwrap();
    assert!(p.starts_with(d.path().join("backups")));
    let c = rusqlite::Connection::open(&p).unwrap();
    let n: i64 = c.query_row("SELECT count(*) FROM collections", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 2);
}
