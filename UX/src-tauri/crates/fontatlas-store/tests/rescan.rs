mod common;
use common::*;
use fontatlas_store::{FaceKey, FaceQuery, RescanStatus, SourceKind, Store, walk_fonts};


fn setup() -> (tempfile::TempDir, std::path::PathBuf, Store, i64) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(fonts.join("sub")).unwrap();
    for n in SAMPLES {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    std::fs::copy(core_fixture("open_sans.woff2"), fonts.join("sub/OPEN_SANS.WOFF2")).unwrap();
    std::fs::write(fonts.join("notes.txt"), "x").unwrap();
    std::fs::write(fonts.join(".hidden.ttf"), "x").unwrap();
    let db = d.path().join("db");
    let mut s = open_store(&db);
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    (d, fonts, s, src)
}

fn total(s: &Store) -> u64 {
    s.query_faces(&FaceQuery::default()).unwrap().total
}

#[test]
fn walk_filters_extensions_hidden_and_sorts() {
    let (_d, fonts, _s, _) = setup();
    let w = walk_fonts(&fonts);
    assert_eq!(w.len(), 6);
    assert!(w.iter().all(|f| f.size > 0 && f.mtime_ns > 0));
    assert!(w.windows(2).all(|p| p[0].path < p[1].path));
    assert!(w.iter().all(|f| !f.path.to_string_lossy().contains(".hidden")));
}

#[test]
fn first_scan_then_unchanged_scan() {
    let (_d, _f, mut s, src) = setup();
    let a = s.rescan_source(src).unwrap();
    assert_eq!((a.seen, a.analyzed, a.failed, a.unchanged), (6, 6, 0, 0));
    assert_eq!(total(&s), 6);
    let b = s.rescan_source(src).unwrap();
    assert_eq!((b.unchanged, b.analyzed, b.relinked), (6, 0, 0));
}

#[test]
fn rename_relinks_without_reanalysis_and_keeps_user_data() {
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    let row = s.query_faces(&FaceQuery { text: Some("roboto".into()), ..Default::default() }).unwrap().items.remove(0);
    let k = FaceKey::from_hex(&row.hash_hex, row.face_index).unwrap();
    s.set_favorite(&k, true).unwrap();
    let c = s.create_collection("C", None).unwrap();
    s.add_to_collection(c, &[k]).unwrap();
    let t = s.tag("tiêu đề").unwrap();
    s.tag_faces(t, &[k]).unwrap();
    std::fs::create_dir_all(fonts.join("moved")).unwrap();
    std::fs::rename(fonts.join("roboto.ttf"), fonts.join("moved/Roboto Renamed.ttf")).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!((st.relinked, st.analyzed, st.missing), (1, 0, 1));
    let old_missing: i64 = s.conn().query_row(
        "SELECT missing FROM file_locations WHERE path=?1", [fonts.join("roboto.ttf").to_string_lossy()], |r| r.get(0)).unwrap();
    assert_eq!(old_missing, 1);
    let fav = s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap().items;
    assert_eq!(fav.len(), 1);
    assert!(fav[0].path.as_deref().unwrap().ends_with("Roboto Renamed.ttf"));
    assert!(!fav[0].missing);
    assert_eq!(s.query_faces(&FaceQuery { collection_id: Some(c), ..Default::default() }).unwrap().total, 1);
    assert_eq!(s.query_faces(&FaceQuery { tag_ids: vec![t], ..Default::default() }).unwrap().total, 1);
}

#[test]
fn duplicate_in_two_sources_is_one_face() {
    let (d, _fonts, mut s, src) = setup();
    let other = d.path().join("Other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), other.join("roboto-copy.ttf")).unwrap();
    let src2 = s.add_source(SourceKind::Directory, "Other", Some(&other)).unwrap();
    s.rescan_source(src).unwrap();
    let st = s.rescan_source(src2).unwrap();
    assert_eq!((st.relinked, st.analyzed), (1, 0));
    assert_eq!(total(&s), 6, "cùng hash chỉ là một face");
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 7);
    s.remove_source(src).unwrap();
    let left = s.query_faces(&FaceQuery::default()).unwrap().items;
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].family, "Roboto");
    assert_eq!(count(&s, "SELECT count(*) FROM u.sources"), 1);
    fts_ok(&s);
    assert_eq!(fts_hits(&s, "open sans"), 0, "FTS phải dọn theo gc");
    assert_eq!(fts_hits(&s, "roboto"), 1);
}

#[test]
fn removed_file_marked_missing_and_comes_back() {
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    let bytes = std::fs::read(fonts.join("roboto.ttf")).unwrap();
    std::fs::remove_file(fonts.join("roboto.ttf")).unwrap();
    assert_eq!(s.rescan_source(src).unwrap().missing, 1);
    assert_eq!(total(&s), 5);
    std::fs::write(fonts.join("roboto.ttf"), bytes).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.analyzed, 0, "nội dung cũ đã có kết quả");
    assert_eq!(total(&s), 6);
}

#[test]
fn changed_content_same_path_follows_new_hash() {
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    std::fs::copy(core_fixture("playfair_display.ttf"), fonts.join("roboto.ttf")).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.unchanged, 5);
    assert_eq!(st.relinked, 1, "nội dung mới trùng hash Playfair đã có");
    assert_eq!(total(&s), 5, "Roboto không còn vị trí nào, Playfair ở 2 vị trí");
}

#[test]
fn broken_file_is_reported_once_and_retry_reanalyzes() {
    let (_d, fonts, mut s, src) = setup();
    std::fs::write(fonts.join("broken.ttf"), b"not a font at all").unwrap();
    let a = s.rescan_source(src).unwrap();
    assert_eq!(a.failed, 1);
    let p = s.problem_files().unwrap();
    assert_eq!(p[0].code, "not_a_font");
    assert!(p[0].path.as_deref().unwrap().ends_with("broken.ttf"));
    assert_eq!(s.rescan_source(src).unwrap().failed, 0, "lỗi cùng phiên bản không chạy lại");
    s.retry_failed().unwrap();
    assert_eq!(s.rescan_source(src).unwrap().failed, 1);
}

#[test]
fn analyzer_version_change_reanalyzes_only_stale() {
    let (_d, _f, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    s.conn().execute(
        "UPDATE pipeline_state SET version='core-0.0.0' WHERE face_index=-1 AND hash=(SELECT hash FROM files LIMIT 1)", []).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!((st.analyzed, st.unchanged), (1, 5));
}

#[test]
fn user_db_bytes_unchanged_by_rescan_and_library_rebuild() {
    let (_d, fonts, mut s, src) = setup();
    s.set_favorite(&FaceKey { hash: [3; 32], face_index: 0 }, true).unwrap();
    s.conn().execute_batch("PRAGMA u.wal_checkpoint(TRUNCATE);").unwrap();
    let dir = s.dir().to_path_buf();
    let (up, wal) = (dir.join("user.db"), dir.join("user.db-wal"));
    let before = (fingerprint(&up), fingerprint(&wal));
    assert!(before.0.is_some());
    s.rescan_source(src).unwrap();
    std::fs::remove_file(fonts.join("roboto.ttf")).unwrap();
    s.rescan_source(src).unwrap();
    s.gc().unwrap();
    assert_eq!((fingerprint(&up), fingerprint(&wal)), before, "rescan/gc không được ghi user.db");
    drop(s);
    let after_close = (fingerprint(&up), fingerprint(&wal));
    for f in ["library.db-wal", "library.db-shm"] {
        let _ = std::fs::remove_file(dir.join(f));
    }
    std::fs::write(dir.join("library.db"), b"rac").unwrap();
    let (s2, rep) = Store::open(&dir).unwrap();
    assert!(rep.library_recreated);
    assert_eq!(fingerprint(&up), after_close.0, "dựng lại library không được ghi user.db");
    // Mở lại DB ở WAL sẽ tạo user.db-wal rỗng (0 byte) — không có khung dữ liệu nào được ghi.
    let wal_len = fingerprint(&wal).map(|f| f.0).unwrap_or(0);
    assert_eq!(wal_len, after_close.1.map(|f| f.0).unwrap_or(0));
    assert_eq!(s2.favorites().unwrap().len(), 1);
}

#[test]
fn unknown_or_pathless_source_is_not_found() {
    let (_d, _f, mut s, _src) = setup();
    assert_eq!(s.rescan_source(4242).unwrap_err().code(), "not_found");
    let g = s.add_source(SourceKind::GoogleFonts, "Google", None).unwrap();
    assert_eq!(s.rescan_source(g).unwrap_err().code(), "not_found");
}

#[test]
fn unplugged_root_marks_nothing_missing() {
    let (d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    let away = d.path().join("Fonts-rut-o");
    std::fs::rename(&fonts, &away).unwrap(); // mô phỏng rút ổ ngoài
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.status, RescanStatus::SourceUnavailable);
    assert_eq!(st.missing, 0);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE missing=1"), 0);
    assert_eq!(total(&s), 6);
    std::fs::rename(&away, &fonts).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!((st.status, st.unchanged), (RescanStatus::Ok, 6));
}

#[test]
fn missing_does_not_depend_on_clock() {
    // Hai lần quét trong cùng mili-giây vẫn đánh missing đúng (thế hệ quét, không phải thời gian).
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    s.conn().execute("UPDATE file_locations SET last_seen_at = 9999999999999", []).unwrap(); // đồng hồ "tương lai"
    std::fs::remove_file(fonts.join("roboto.ttf")).unwrap();
    assert_eq!(s.rescan_source(src).unwrap().missing, 1);
}

#[cfg(unix)]
#[test]
fn unreadable_subfolder_does_not_mark_its_fonts_missing() {
    use std::os::unix::fs::PermissionsExt;
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    let sub = fonts.join("sub");
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o000)).unwrap();
    let st = s.rescan_source(src);
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let st = st.unwrap();
    assert_eq!(st.missing, 0, "thư mục con không đọc được không có nghĩa là font bị xoá");
    assert_eq!(total(&s), 6);
}

#[cfg(unix)]
#[test]
fn unreadable_known_file_is_not_marked_missing() {
    use std::os::unix::fs::PermissionsExt;
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    let f = fonts.join("roboto.ttf");
    let mut bytes = std::fs::read(&f).unwrap();
    bytes.push(0); // đổi size/mtime để buộc băm lại
    std::fs::write(&f, &bytes).unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o000)).unwrap();
    let st = s.rescan_source(src);
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
    let st = st.unwrap();
    assert_eq!(st.unreadable.len(), 1);
    assert_eq!(st.missing, 0);
    assert_eq!(total(&s), 6);
}

#[test]
fn transient_io_failure_is_retried_on_next_scan() {
    use fontatlas_store::{Analysis, IngestItem};
    let (_d, fonts, mut s, src) = setup();
    let f = walk_fonts(&fonts).into_iter().find(|x| x.path.ends_with("roboto.ttf")).unwrap();
    let h = *blake3::hash(&std::fs::read(&f.path).unwrap()).as_bytes();
    let io = IngestItem { source_id: src, file: f, hash: h, analysis: Analysis::Failed { code: "io".into(), detail: "EIO".into() } };
    s.ingest(vec![io], 1).unwrap();
    assert!(s.problem_files().unwrap().is_empty(), "lỗi io tạm thời không phải lỗi font");
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.analyzed, 6, "roboto phải được phân tích lại tự động");
}

#[test]
fn gc_removes_orphan_embeddings_analyses_and_state_but_keeps_community() {
    let (_d, _fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    s.conn().execute_batch(
        "INSERT INTO community_snapshots(snapshot_id,name,version,format_version,sha256,imported_at)
           VALUES (1,'gf','1',1,zeroblob(32),0);
         INSERT INTO embeddings(hash,face_index,model_version,source,snapshot_id,dim,dtype,vec,created_at)
           SELECT hash, 0, 'm@1', 'local', NULL, 4, 'f32', zeroblob(16), 0 FROM files;
         INSERT INTO embeddings(hash,face_index,model_version,source,snapshot_id,dim,dtype,vec,created_at)
           SELECT hash, 0, 'm@1', 'community', 1, 4, 'f32', zeroblob(16), 0 FROM files;
         PRAGMA u.wal_checkpoint(TRUNCATE);",
    ).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM embeddings WHERE source='local'"), 6);
    let dir = s.dir().to_path_buf();
    let before = (fingerprint(&dir.join("user.db")), fingerprint(&dir.join("user.db-wal")));
    // mô phỏng xoá nguồn ở phía library (remove_source cũng gọi gc)
    s.conn().execute("DELETE FROM file_locations WHERE source_id=?1", [src]).unwrap();
    s.gc().unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM embeddings WHERE source='local'"), 0, "embedding local mồ côi phải bị xoá");
    assert_eq!(count(&s, "SELECT count(*) FROM analyses WHERE source='local'"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM pipeline_state"), 0);
    assert_eq!(count(&s, "SELECT count(*) FROM embeddings WHERE source='community'"), 6, "dữ liệu cộng đồng khoá theo hash, không phải rác");
    assert_eq!((fingerprint(&dir.join("user.db")), fingerprint(&dir.join("user.db-wal"))), before, "gc không được ghi user.db");
}
