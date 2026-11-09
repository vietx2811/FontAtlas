mod common;
use common::*;
use fontatlas_store::{FaceKey, FaceQuery, LangFilter, LangRow, Store, create_library_v1_for_test};
use rusqlite::Connection;

const V: &str = "hg-test-1";

fn row(lang: &str, have: u32, level: &str) -> LangRow {
    LangRow { lang: lang.into(), have, total: 60, level: level.into() }
}

fn seeded() -> (tempfile::TempDir, Store, Vec<FaceKey>) {
    let d = tmp();
    let mut s = open_store(d.path());
    let items: Vec<_> = SAMPLES.iter().map(|n| item(1, &format!("/lib/{n}"), &report(n))).collect();
    s.ingest(items, 10).unwrap();
    let keys = SAMPLES
        .iter()
        .map(|n| FaceKey { hash: hash_of(&report(n)), face_index: 0 })
        .collect();
    (d, s, keys)
}

fn langs(codes: &[(&str, bool)], any: bool) -> FaceQuery {
    FaceQuery {
        langs: codes.iter().map(|(c, n)| LangFilter { code: (*c).into(), near_ok: *n }).collect(),
        lang_any: any,
        ..Default::default()
    }
}

#[test]
fn migration_v1_to_v2_backs_up_and_keeps_data() {
    let d = tmp();
    {
        let c = Connection::open(d.path().join("library.db")).unwrap();
        create_library_v1_for_test(&c);
        c.execute("INSERT INTO files (hash, size, container, num_faces, first_seen_at) VALUES (zeroblob(32), 7, 'sfnt', 1, 1)", [])
            .unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(!rep.library_recreated, "migration thành công, không dựng lại");
    let v: i64 = s.conn().query_row("PRAGMA main.user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, i64::from(fontatlas_store::LIBRARY_VERSION));
    let size: i64 = s.conn().query_row("SELECT size FROM files", [], |r| r.get(0)).unwrap();
    assert_eq!(size, 7, "dữ liệu cũ nguyên vẹn");
    for t in ["face_langs", "face_lang_state"] {
        let n: i64 = s.conn().query_row("SELECT count(*) FROM main.sqlite_schema WHERE name=?1", [t], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "{t}");
        let rows: i64 = s.conn().query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get(0)).unwrap();
        assert_eq!(rows, 0);
    }
    // bản sao lưu library.v1 mở được và còn dữ liệu cũ
    let backups: Vec<_> = std::fs::read_dir(d.path().join("backups"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("library.v1."))
        .collect();
    assert_eq!(backups.len(), 1);
    let bc = Connection::open(backups[0].path()).unwrap();
    let bsize: i64 = bc.query_row("SELECT size FROM files", [], |r| r.get(0)).unwrap();
    assert_eq!(bsize, 7);
    let bv: i64 = bc.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(bv, 1);
}

#[test]
fn pending_progress_and_put_roundtrip() {
    let (_d, s, keys) = seeded();
    let p = s.lang_progress(V).unwrap();
    assert_eq!((p.done, p.total), (0, 5));
    let pending = s.lang_pending(V, 100).unwrap();
    assert_eq!(pending.len(), 5);
    let fams: Vec<i64> = pending.iter().map(|x| x.face_id).collect();
    assert_eq!(fams.len(), 5);
    assert!(pending[0].path.to_string_lossy().starts_with("/lib/"));
    assert_eq!(s.lang_pending(V, 2).unwrap().len(), 2, "limit");

    s.put_face_langs(&keys[0], V, &[row("vie", 60, "full"), row("eng", 55, "near")]).unwrap();
    s.put_face_langs(&keys[1], V, &[]).unwrap(); // đã tính, không đạt ngôn ngữ nào
    let p = s.lang_progress(V).unwrap();
    assert_eq!((p.done, p.total), (2, 5));
    assert_eq!(s.lang_pending(V, 100).unwrap().len(), 3);
    assert!(s.lang_state_is(&keys[1], V).unwrap());
    // đổi phiên bản dữ liệu ⇒ coi như chưa tính
    assert_eq!(s.lang_pending("hg-test-2", 100).unwrap().len(), 5);
    assert_eq!(s.lang_progress("hg-test-2").unwrap().done, 0);

    // ghi lại thay hàng cũ, không nhân đôi
    s.put_face_langs(&keys[0], V, &[row("vie", 60, "full")]).unwrap();
    let n: i64 = s.conn().query_row("SELECT count(*) FROM face_langs", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn filter_modes_levels_and_badge() {
    let (_d, s, keys) = seeded();
    s.put_face_langs(&keys[0], V, &[row("vie", 60, "full"), row("fra", 56, "near")]).unwrap();
    s.put_face_langs(&keys[1], V, &[row("vie", 56, "near"), row("fra", 60, "full")]).unwrap();
    s.put_face_langs(&keys[2], V, &[row("fra", 60, "full")]).unwrap();
    let total = |q: &FaceQuery| s.query_faces(q).unwrap().total;

    assert_eq!(total(&langs(&[("vie", false)], false)), 1, "chỉ full");
    assert_eq!(total(&langs(&[("vie", true)], false)), 2, "≥ near");
    assert_eq!(total(&langs(&[("vie", true), ("fra", false)], false)), 1, "AND: vie≥near và fra full");
    assert_eq!(total(&langs(&[("vie", true), ("fra", true)], false)), 2, "AND");
    assert_eq!(total(&langs(&[("vie", false), ("fra", false)], true)), 3, "OR: face0 (vie full) + face1 + face2 (fra full)");
    assert_eq!(total(&langs(&[("xxx", true)], false)), 0, "ngôn ngữ lạ");
    assert_eq!(total(&langs(&[], false)), 5, "không lọc");

    // badge: điền theo ngôn ngữ yêu cầu, không có hàng thì None
    let q = FaceQuery { badge_lang: Some("vie".into()), ..Default::default() };
    let page = s.query_faces(&q).unwrap();
    let hit = |i: usize| page.items.iter().find(|r| r.hash_hex == hex(&keys[i])).unwrap().lang_hit.clone();
    assert_eq!(hit(0).map(|h| (h.level, h.have, h.total)), Some(("full".into(), 60, 60)));
    assert_eq!(hit(1).map(|h| h.level), Some("near".into()));
    assert!(hit(2).is_none());
    let none = s.query_faces(&FaceQuery::default()).unwrap();
    assert!(none.items.iter().all(|r| r.lang_hit.is_none()));
}

fn hex(k: &FaceKey) -> String {
    k.hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn counts_follow_the_current_filter_but_ignore_the_lang_filter_itself() {
    let (_d, s, keys) = seeded();
    s.put_face_langs(&keys[0], V, &[row("vie", 60, "full"), row("fra", 56, "near")]).unwrap();
    s.put_face_langs(&keys[1], V, &[row("vie", 56, "near")]).unwrap();
    let get = |q: &FaceQuery, l: &str| s.lang_counts(q, None).unwrap().into_iter().find(|c| c.lang == l).map(|c| (c.full, c.near));
    assert_eq!(get(&FaceQuery::default(), "vie"), Some((1, 2)), "near gồm cả full");
    assert_eq!(get(&FaceQuery::default(), "fra"), Some((0, 1)));
    // filter langs không làm đếm tụt (facet): chọn fra vẫn thấy vie=2
    assert_eq!(get(&langs(&[("fra", true)], false), "vie"), Some((1, 2)));
    // các filter khác có ảnh hưởng: text
    let roboto = FaceQuery { text: Some("roboto".into()), ..Default::default() };
    let c = get(&roboto, "vie");
    assert!(c.is_none() || c.unwrap().1 <= 1);
}

#[test]
fn prune_removes_rows_of_vanished_faces() {
    let (_d, s, keys) = seeded();
    s.put_face_langs(&keys[0], V, &[row("vie", 60, "full")]).unwrap();
    let ghost = FaceKey { hash: [9; 32], face_index: 0 };
    s.put_face_langs(&ghost, V, &[row("vie", 60, "full")]).unwrap();
    assert_eq!(s.prune_langs().unwrap(), 2, "1 hàng face_langs + 1 dấu state của face ma");
    let n: i64 = s.conn().query_row("SELECT count(*) FROM face_langs", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn counts_respect_scope() {
    let (_d, s, keys) = seeded();
    s.put_face_langs(&keys[0], V, &[row("vie", 60, "full")]).unwrap();
    s.put_face_langs(&keys[1], V, &[row("vie", 56, "near")]).unwrap();
    let ids = s.face_ids_of(&keys[..1]).unwrap();
    let scope: Vec<(i64, f32)> = ids.iter().map(|(_, id)| (*id, 1.0)).collect();
    let c = s.lang_counts(&FaceQuery::default(), Some(&scope)).unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].full, c[0].near), (1, 1));
    // scope không rò sang truy vấn sau
    assert_eq!(s.query_faces(&FaceQuery::default()).unwrap().total, 5);
}

/// 20k face tổng hợp (mỗi face ~15 ngôn ngữ ≥ near): đo các truy vấn nền của ô chọn ngôn ngữ.
#[test]
fn lang_queries_are_fast_on_20k_faces() {
    let d = tmp();
    let s = open_store(d.path());
    let c = s.conn();
    c.execute_batch(
        "BEGIN;
         WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i < 20000)
           INSERT INTO files(file_id, hash, size, container, num_faces, first_seen_at)
           SELECT i, randomblob(32), 1, 'sfnt', 1, 1 FROM n;
         INSERT INTO file_locations(file_id, source_id, path, size, mtime_ns, last_seen_at, missing, scan_gen)
           SELECT file_id, 1, '/x/' || file_id, 1, 1, 1, 0, 1 FROM files;
         INSERT INTO faces(file_id, face_index, family_name, outline_format, license_class, vi_status, family_key, search_text,
                           weight_class, is_variable)
           SELECT file_id, 0, 'F' || file_id, 'glyf', CASE file_id % 3 WHEN 0 THEN 'ofl' ELSE 'other' END, 'full',
                  'f' || (file_id % 5000), 'f' || file_id, 400, 0 FROM files;
         WITH RECURSIVE k(j) AS (SELECT 1 UNION ALL SELECT j+1 FROM k WHERE j < 15)
           INSERT INTO face_langs(hash, face_index, lang, have, total, level)
           SELECT f.hash, 0, 'l' || k.j, 60, 60, CASE (f.file_id + k.j) % 2 WHEN 0 THEN 'full' ELSE 'near' END
           FROM files f, k;
         INSERT INTO face_lang_state(hash, face_index, db_version) SELECT hash, 0, 'v' FROM files;
         COMMIT;",
    )
    .unwrap();
    let t = std::time::Instant::now();
    let all = s.lang_counts_unfiltered().unwrap();
    let t_unf = t.elapsed();
    assert_eq!(all.len(), 15);
    assert_eq!(all[0].near, 20000);
    let q = FaceQuery { license: vec!["ofl".into()], ..Default::default() };
    let t = std::time::Instant::now();
    let f = s.lang_counts(&q, None).unwrap();
    let t_f = t.elapsed();
    assert_eq!(f[0].near, 6666);
    let t = std::time::Instant::now();
    let p = s.lang_progress("v").unwrap();
    let t_p = t.elapsed();
    assert_eq!((p.done, p.total), (20000, 20000));
    let t = std::time::Instant::now();
    let pend = s.lang_pending("other", usize::MAX >> 1).unwrap();
    let t_pend = t.elapsed();
    assert_eq!(pend.len(), 20000);
    eprintln!("20k faces: unfiltered {t_unf:?}, filtered {t_f:?}, progress {t_p:?}, pending(all) {t_pend:?}");
    assert!(t_unf.as_millis() < 500 && t_p.as_millis() < 500 && t_pend.as_millis() < 3000);
}
