mod common;
use common::*;
use fontatlas_core::AxisInfo;
use fontatlas_store::{ClassAxis, FaceQuery, Store, create_library_v2_for_test, width_pct_span};
use rusqlite::Connection;

fn ax(tag: &str, min: f32, def: f32, max: f32) -> AxisInfo {
    AxisInfo { tag: tag.into(), min, default: def, max }
}

/// Mỗi mẫu: (family, usWeightClass, usWidthClass, trục fvar).
type Spec = (&'static str, Option<u16>, Option<u16>, Vec<AxisInfo>);

fn seeded(specs: Vec<Spec>) -> (tempfile::TempDir, Store) {
    let d = tmp();
    let mut s = open_store(d.path());
    let base = report("roboto.ttf");
    let items: Vec<_> = specs
        .into_iter()
        .map(|(fam, w, wd, axes)| {
            let mut r = fake_report(&base, fam, fam);
            for f in &mut r.faces {
                f.metadata.weight_class = w;
                f.metadata.width_class = wd;
                f.metadata.axes = axes.clone();
            }
            item(1, &format!("/lib/{fam}.ttf"), &r)
        })
        .collect();
    s.ingest(items, 10).unwrap();
    (d, s)
}

fn names(s: &Store, q: &FaceQuery) -> Vec<String> {
    let mut v: Vec<String> = s.query_faces(q).unwrap().items.into_iter().map(|r| r.family).collect();
    v.sort();
    v
}

fn library() -> Vec<Spec> {
    vec![
        ("Thin", Some(100), Some(5), vec![]),
        ("Regular", Some(400), Some(5), vec![]),
        ("Bold", Some(700), Some(5), vec![]),
        ("Old", Some(4), Some(5), vec![]), // font cũ ghi 4 = 400
        ("Odd", Some(350), Some(3), vec![]),
        ("NoOs2", None, None, vec![]),
        ("Inter", Some(400), Some(5), vec![ax("wght", 100.0, 400.0, 900.0)]),
        ("Cond", Some(400), Some(3), vec![ax("wght", 400.0, 400.0, 400.0), ax("wdth", 75.0, 75.0, 100.0)]),
    ]
}

#[test]
fn weight_class_filter_with_and_without_axis_match() {
    let (_d, s) = seeded(library());
    let mut q = FaceQuery { weight_class: Some([600, 900]), ..Default::default() };
    assert_eq!(names(&s, &q), vec!["Bold"], "tắt wght: chỉ usWeightClass");
    q.var_weight = true;
    assert_eq!(names(&s, &q), vec!["Bold", "Inter"], "Inter 100–900 giao 600–900");
    q.weight_class = Some([400, 500]);
    assert_eq!(names(&s, &q), vec!["Cond", "Inter", "Old", "Regular"], "400 cũ ×100; Cond wght 400–400 giao");
    q.weight_class = Some([300, 300]);
    q.var_weight = false;
    assert!(names(&s, &q).is_empty(), "350 không rơi vào 300–300");
    q.weight_class = Some([350, 350]);
    assert_eq!(names(&s, &q), vec!["Odd"], "giá trị lẻ lọc theo số");
}

#[test]
fn width_class_filter_maps_wdth_percent() {
    let (_d, s) = seeded(library());
    let q = FaceQuery { width_class: Some([3, 3]), ..Default::default() };
    assert_eq!(names(&s, &q), vec!["Cond", "Odd"]);
    let q = FaceQuery { width_class: Some([5, 5]), var_width: true, ..Default::default() };
    // Cond wdth 75–100 giao class 5 (100); font không có OS/2 width bị loại
    assert_eq!(names(&s, &q), vec!["Bold", "Cond", "Inter", "Old", "Regular", "Thin"]);
    let q = FaceQuery { width_class: Some([7, 9]), var_width: true, ..Default::default() };
    assert!(names(&s, &q).is_empty());
    let (lo, hi) = width_pct_span(1, 9);
    assert_eq!((lo, hi), (50.0, 200.0));
    assert_eq!(width_pct_span(5, 5), (93.75, 106.25));
}

#[test]
fn histograms_exclude_their_own_axis() {
    let (_d, s) = seeded(library());
    let q = FaceQuery { weight_class: Some([700, 900]), width_class: Some([5, 5]), ..Default::default() };
    let w = s.class_histogram(&q, None, ClassAxis::Weight).unwrap();
    // bỏ weight_class, vẫn áp width 5: Thin, Regular, Bold, Old, Inter(400)
    assert_eq!(w, [1, 0, 0, 3, 0, 0, 1, 0, 0], "{w:?}");
    let wd = s.class_histogram(&q, None, ClassAxis::Width).unwrap();
    // bỏ width, vẫn áp weight 700–900: chỉ Bold (width 5)
    assert_eq!(wd[4], 1);
    assert_eq!(wd.iter().sum::<u32>(), 1);
}

#[test]
fn histogram_respects_scope() {
    let (_d, s) = seeded(library());
    let all = s.query_faces(&FaceQuery::default()).unwrap();
    let thin = all.items.iter().find(|r| r.family == "Thin").unwrap().face_id;
    let h = s.class_histogram(&FaceQuery::default(), Some(&[(thin, 1.0)]), ClassAxis::Weight).unwrap();
    assert_eq!(h, [1, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn migration_v2_to_v3_backfills_axis_ranges_with_backup() {
    let d = tmp();
    {
        let c = Connection::open(d.path().join("library.db")).unwrap();
        create_library_v2_for_test(&c);
        c.execute("INSERT INTO files (file_id, hash, size, container, num_faces, first_seen_at) VALUES (1, zeroblob(32), 7, 'sfnt', 1, 1)", []).unwrap();
        c.execute(
            "INSERT INTO faces (face_id, file_id, face_index, family_name, outline_format, license_class, vi_status, family_key, search_text, is_variable, weight_class)
             VALUES (1, 1, 0, 'Inter', 'glyf', 'ofl', 'full', 'inter', 'inter', 1, 400)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO analyses (hash, face_index, analyzer_version, source, metadata_json, vietnamese_json, anatomy_json, analyzed_at)
             VALUES (zeroblob(32), 0, 'a', 'local', ?1, '{}', '{}', 1)",
            [r#"{"axes":[{"tag":"opsz","min":14,"default":14,"max":32},{"tag":"wght","min":100,"default":400,"max":900}]}"#],
        )
        .unwrap();
        // bản phân tích hỏng JSON của face khác không được làm hỏng migration
        c.execute("INSERT INTO files (file_id, hash, size, container, num_faces, first_seen_at) VALUES (2, x'0101010101010101010101010101010101010101010101010101010101010101', 1, 'sfnt', 1, 1)", []).unwrap();
        c.execute(
            "INSERT INTO faces (face_id, file_id, face_index, family_name, outline_format, license_class, vi_status, family_key, search_text, is_variable)
             VALUES (2, 2, 0, 'Broken', 'glyf', 'ofl', 'full', 'broken', 'broken', 1)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO analyses (hash, face_index, analyzer_version, source, metadata_json, vietnamese_json, anatomy_json, analyzed_at)
             VALUES (x'0101010101010101010101010101010101010101010101010101010101010101', 0, 'a', 'local', 'not json', '{}', '{}', 1)",
            [],
        )
        .unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(!rep.library_recreated);
    let v: i64 = s.conn().query_row("PRAGMA main.user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, 3);
    let (a, b, c, e): (Option<f64>, Option<f64>, Option<f64>, Option<f64>) = s
        .conn()
        .query_row("SELECT wght_min, wght_max, wdth_min, wdth_max FROM faces WHERE face_id = 1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap();
    assert_eq!((a, b, c, e), (Some(100.0), Some(900.0), None, None));
    let broken: Option<f64> = s.conn().query_row("SELECT wght_min FROM faces WHERE face_id = 2", [], |r| r.get(0)).unwrap();
    assert_eq!(broken, None);
    let backups = std::fs::read_dir(d.path().join("backups"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("library.v2."))
        .count();
    assert_eq!(backups, 1);
}
