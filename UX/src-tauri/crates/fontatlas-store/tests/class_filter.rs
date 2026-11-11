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
fn histograms_exclude_their_own_axis_and_have_two_series() {
    let (_d, s) = seeded(library());
    let q = FaceQuery { weight_class: Some([700, 900]), width_class: Some([5, 5]), ..Default::default() };
    let w = s.class_histogram(&q, None, ClassAxis::Weight).unwrap();
    // bỏ weight_class, vẫn áp width 5: Thin, Regular, Bold, Old, Inter(400)
    assert_eq!(w.default_bins, [1, 0, 0, 3, 0, 0, 1, 0, 0], "{w:?}");
    // Inter wght 100–900 phủ mọi nấc: chuỗi span cộng thêm 1 vào các nấc 1,2,3,5,6,8,9 (nấc 4 đã có mặc định, 7 cũng)
    assert_eq!(w.span_bins, [2, 1, 1, 3, 1, 1, 2, 1, 1], "{w:?}");
    let wd = s.class_histogram(&q, None, ClassAxis::Width).unwrap();
    // bỏ width, vẫn áp weight 700–900 (có wght): Bold (5) và Inter (span 100–900, width 5)
    assert_eq!(wd.default_bins[4], 1, "var_weight tắt: Inter không khớp weight 700–900");
    assert_eq!(wd.default_bins.iter().sum::<u32>(), 1);
    let q2 = FaceQuery { var_weight: true, ..q.clone() };
    assert_eq!(s.class_histogram(&q2, None, ClassAxis::Width).unwrap().default_bins[4], 2, "bật wght: thêm Inter");
}

#[test]
fn weight_and_width_outliers_are_clamped_the_same_in_filter_and_histogram() {
    let (_d, s) = seeded(vec![
        ("Zero", Some(0), Some(0), vec![]),
        ("Fifty", Some(50), Some(10), vec![]),
        ("Over", Some(1000), Some(5), vec![]),
        ("Over2", Some(950), Some(5), vec![]),
        ("Mid", Some(400), Some(5), vec![]),
        ("Nine", Some(900), Some(9), vec![]),
    ]);
    let full = FaceQuery { weight_class: Some([100, 900]), ..Default::default() };
    // kéo hết 100–900 vẫn giữ 50→100, 950/1000→900; chỉ 0 (không có giá trị) bị loại
    assert_eq!(names(&s, &full), vec!["Fifty", "Mid", "Nine", "Over", "Over2"]);
    let h = s.class_histogram(&FaceQuery::default(), None, ClassAxis::Weight).unwrap();
    assert_eq!(h.default_bins, [1, 0, 0, 1, 0, 0, 0, 0, 3], "{h:?}");
    assert_eq!(h.default_bins.iter().sum::<u32>() as usize, names(&s, &full).len(), "histogram và filter cùng tập");
    // width: 0 và 10 không hợp lệ → NULL, cả khi lọc 1–9
    let wfull = FaceQuery { width_class: Some([1, 9]), ..Default::default() };
    assert_eq!(names(&s, &wfull), vec!["Mid", "Nine", "Over", "Over2"]);
    let wh = s.class_histogram(&FaceQuery::default(), None, ClassAxis::Width).unwrap();
    assert_eq!(wh.default_bins, [0, 0, 0, 0, 3, 0, 0, 0, 1], "{wh:?}");
}

#[test]
fn width_boundaries_belong_to_one_class_only() {
    // wdth 87.5 = đúng class 4; 81.25 là trung điểm giữa class 3 (75) và 4 (87.5) → thuộc class 4 (nửa mở)
    let (_d, s) = seeded(vec![
        ("OnBoundary", Some(400), Some(5), vec![ax("wdth", 81.25, 87.5, 81.25)]),
        ("Wide", Some(400), Some(5), vec![ax("wdth", 100.0, 100.0, 200.0)]),
        ("Max", Some(400), Some(5), vec![ax("wdth", 200.0, 200.0, 200.0)]),
    ]);
    let three = FaceQuery { width_class: Some([3, 3]), var_width: true, ..Default::default() };
    assert!(names(&s, &three).is_empty(), "81.25 không còn khớp class 3");
    let four = FaceQuery { width_class: Some([4, 4]), var_width: true, ..Default::default() };
    assert_eq!(names(&s, &four), vec!["OnBoundary"]);
    let nine = FaceQuery { width_class: Some([9, 9]), var_width: true, ..Default::default() };
    assert_eq!(names(&s, &nine), vec!["Max", "Wide"], "class 9 đóng ở 200 (font chỉ có đúng wdth 200 vẫn khớp)");
    let h = s.class_histogram(&FaceQuery::default(), None, ClassAxis::Width).unwrap();
    assert_eq!(h.span_bins[2], 0);
    assert_eq!(h.span_bins[3], 1);
    assert_eq!(h.span_bins[8], 2, "histogram và filter cùng tập ở biên 200");
    assert_eq!(h.span_bins[4], 3, "mặc định class 5 cho cả ba");
}

#[test]
fn histogram_respects_scope() {
    let (_d, s) = seeded(library());
    let all = s.query_faces(&FaceQuery::default()).unwrap();
    let thin = all.items.iter().find(|r| r.family == "Thin").unwrap().face_id;
    let h = s.class_histogram(&FaceQuery::default(), Some(&[(thin, 1.0)]), ClassAxis::Weight).unwrap();
    assert_eq!(h.default_bins, [1, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(h.span_bins, h.default_bins);
    assert_eq!(s.face_ids_matching(&FaceQuery::default(), Some(&[(thin, 1.0)])).unwrap(), vec![thin]);
    assert_eq!(s.face_ids_matching(&FaceQuery::default(), None).unwrap().len(), 8);
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
        // bản cũ hơn của face 1 có dải khác: min và max phải cùng lấy từ bản mới nhất, không trộn
        c.execute(
            "INSERT INTO analyses (hash, face_index, analyzer_version, source, metadata_json, vietnamese_json, anatomy_json, analyzed_at)
             VALUES (zeroblob(32), 0, 'old', 'local', ?1, '{}', '{}', 0)",
            [r#"{"axes":[{"tag":"wght","min":200,"default":300,"max":300}]}"#],
        )
        .unwrap();
        // face 3: Variable, JSON hợp lệ nhưng không có `axes`; face 4: không phải Variable (không được đánh dấu)
        for (id, var, mj) in [(3, 1, r#"{"family":"x"}"#), (4, 0, r#"{"family":"y"}"#)] {
            let h = format!("x'{}'", format!("{:02x}", id).repeat(32));
            c.execute(&format!("INSERT INTO files (file_id, hash, size, container, num_faces, first_seen_at) VALUES ({id}, {h}, 1, 'sfnt', 1, 1)"), []).unwrap();
            c.execute(
                &format!("INSERT INTO faces (face_id, file_id, face_index, family_name, outline_format, license_class, vi_status, family_key, search_text, is_variable)
                          VALUES ({id}, {id}, 0, 'F{id}', 'glyf', 'ofl', 'full', 'f{id}', 'f{id}', {var})"),
                [],
            )
            .unwrap();
            c.execute(
                &format!("INSERT INTO analyses (hash, face_index, analyzer_version, source, metadata_json, vietnamese_json, anatomy_json, analyzed_at) VALUES ({h}, 0, 'a', 'local', ?1, '{{}}', '{{}}', 1)"),
                [mj],
            )
            .unwrap();
        }
        for hash in ["zeroblob(32)", "x'0101010101010101010101010101010101010101010101010101010101010101'", "x'0303030303030303030303030303030303030303030303030303030303030303'", "x'0404040404040404040404040404040404040404040404040404040404040404'"] {
            c.execute(&format!("INSERT INTO pipeline_state(hash,face_index,stage,status,version,attempts,updated_at) VALUES ({hash},-1,'load','done','v-now',0,1)"), []).unwrap();
        }
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(!rep.library_recreated);
    assert_eq!(rep.axes_reanalyze_files, 2, "Broken (JSON hỏng) và F3 (không có axes)");
    let marked: Vec<i64> = {
        let mut st = s.conn().prepare("SELECT f.face_id FROM pipeline_state p JOIN files fi ON fi.hash = p.hash JOIN faces f ON f.file_id = fi.file_id WHERE p.version = 'axes-backfill' ORDER BY 1").unwrap();
        st.query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(marked, vec![2, 3], "face 1 (đủ axes) và face 4 (không Variable) không bị đánh dấu");
    let v: i64 = s.conn().query_row("PRAGMA main.user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, i64::from(fontatlas_store::LIBRARY_VERSION), "migrate lên bản mới nhất");
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

#[test]
fn reingest_keeps_axis_range_columns_in_sync() {
    let d = tmp();
    let mut s = open_store(d.path());
    let base = report("roboto.ttf");
    let make = |axes: Vec<AxisInfo>| {
        let mut r = fake_report(&base, "same", "Same");
        for f in &mut r.faces {
            f.metadata.axes = axes.clone();
            f.metadata.weight_class = Some(400);
        }
        item(1, "/lib/same.ttf", &r)
    };
    let cols = |s: &Store| -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        s.conn().query_row("SELECT wght_min, wght_max, wdth_min, wdth_max FROM faces", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap()
    };
    s.ingest(vec![make(vec![ax("wght", 100.0, 400.0, 900.0)])], 10).unwrap();
    assert_eq!(cols(&s), (Some(100.0), Some(900.0), None, None));
    // phân tích lại: thêm wdth và đổi dải wght
    s.ingest(vec![make(vec![ax("wght", 300.0, 400.0, 700.0), ax("wdth", 75.0, 100.0, 125.0)])], 11).unwrap();
    assert_eq!(cols(&s), (Some(300.0), Some(700.0), Some(75.0), Some(125.0)));
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 1, "cập nhật tại chỗ, không nhân bản");
    // font không còn là Variable: cột về NULL (không giữ dải cũ)
    s.ingest(vec![make(vec![])], 12).unwrap();
    assert_eq!(cols(&s), (None, None, None, None));
    let q = FaceQuery { weight_class: Some([900, 900]), var_weight: true, ..Default::default() };
    assert!(s.query_faces(&q).unwrap().items.is_empty());
}
