mod common;
use common::*;
use fontatlas_store::{FaceKey, FaceQuery, SortKey, Store, ViFilter};

/// 5 font mẫu + 2 face giả tên tiếng Việt.
fn seeded() -> (tempfile::TempDir, Store) {
    let d = tmp();
    let mut s = open_store(d.path());
    let mut items: Vec<_> = SAMPLES.iter().map(|n| item(1, &format!("/lib/{n}"), &report(n))).collect();
    let base = report("open_sans.ttf");
    items.push(item(1, "/lib/dep.ttf", &fake_report(&base, "dep", "Đẹp Sans")));
    items.push(item(1, "/lib/hn.ttf", &fake_report(&base, "hn", "Hà Nội Grotesk")));
    s.ingest(items, 10).unwrap();
    (d, s)
}

fn fams(s: &Store, q: &FaceQuery) -> Vec<String> {
    s.query_faces(q).unwrap().items.into_iter().map(|r| r.family).collect()
}
fn text(t: &str) -> FaceQuery {
    FaceQuery { text: Some(t.into()), ..Default::default() }
}

#[test]
fn search_folds_vietnamese_both_ways() {
    let (_d, s) = seeded();
    // "dep" là chuỗi con: cũng khớp "SourceCodePro" (co-DEP-ro) — đúng hành vi trigram.
    assert!(fams(&s, &text("dep")).contains(&"Đẹp Sans".to_string()));
    assert_eq!(fams(&s, &text("dep sans")), vec!["Đẹp Sans"]);
    assert_eq!(fams(&s, &text("ĐẸP SANS")), vec!["Đẹp Sans"]);
    assert_eq!(fams(&s, &text("Ha Noi")), vec!["Hà Nội Grotesk"]);
    assert_eq!(fams(&s, &text("oi gro")), vec!["Hà Nội Grotesk"]); // chuỗi con giữa từ
    assert_eq!(fams(&s, &text("No\u{0302}\u{0323}i")), vec!["Hà Nội Grotesk"]); // "Nội" dạng NFD
}

#[test]
fn short_and_hostile_queries_work() {
    let (_d, s) = seeded();
    assert!(fams(&s, &text("pl")).contains(&"Playfair Display".to_string()));
    assert!(fams(&s, &text("ro")).contains(&"Roboto".to_string()));
    for q in ["\"", "%", "_", "*", "a\"b", "NEAR(", "') OR 1=1 --"] {
        assert!(s.query_faces(&text(q)).is_ok(), "truy vấn {q:?} không được lỗi");
    }
    assert_eq!(s.query_faces(&text("%")).unwrap().total, 0);
}

#[test]
fn pagination_total_and_stable_order() {
    let (_d, s) = seeded();
    let all = s.query_faces(&FaceQuery::default()).unwrap();
    assert_eq!(all.total, 7);
    let p2 = s.query_faces(&FaceQuery { offset: 2, limit: 2, ..Default::default() }).unwrap();
    assert_eq!(p2.total, 7);
    assert_eq!(p2.items.len(), 2);
    assert_eq!(p2.items[0].face_id, all.items[2].face_id);
    let keys: Vec<String> = all.items.iter().map(|r| fontatlas_store::vi_fold(&r.family)).collect();
    assert!(!keys.is_empty());
    assert!(keys.windows(2).all(|p| p[0] <= p[1]), "{keys:?}");
    let big = s.query_faces(&FaceQuery { limit: 50_000, ..Default::default() }).unwrap();
    assert_eq!(big.items.len(), 7); // limit bị kẹp, không lỗi
}

#[test]
fn sort_by_weight_descending() {
    let (_d, s) = seeded();
    let rows = s.query_faces(&FaceQuery { sort: SortKey::Weight, descending: true, ..Default::default() }).unwrap().items;
    let w: Vec<u16> = rows.iter().filter_map(|r| r.weight_class).collect();
    assert!(w.len() >= 2, "{w:?}");
    assert!(w.windows(2).all(|p| p[0] >= p[1]), "{w:?}");
}

#[test]
fn filters_vi_license_favorites_collection_tags() {
    let (_d, mut s) = seeded();
    let all = s.query_faces(&FaceQuery::default()).unwrap().items;
    let roboto = all.iter().find(|r| r.family == "Roboto").unwrap();
    let k = FaceKey::from_hex(&roboto.hash_hex, roboto.face_index).unwrap();

    let vi = s.query_faces(&FaceQuery { vi: Some(ViFilter::Supported), ..Default::default() }).unwrap();
    assert!(!vi.items.is_empty());
    assert!(vi.items.iter().all(|r| r.supports_vi));
    let full = fams(&s, &FaceQuery { vi: Some(ViFilter::Full), ..Default::default() });
    assert!(!full.contains(&"Roboto".to_string()), "Roboto là precomposed_only (đã kiểm ở core)");
    assert!(full.contains(&"Open Sans".to_string()));

    let ofl = s.query_faces(&FaceQuery { license: vec!["ofl".into()], ..Default::default() }).unwrap();
    assert!(!ofl.items.is_empty());
    assert!(ofl.items.iter().all(|r| r.license_class == "ofl"));

    s.set_favorite(&k, true).unwrap();
    let fav = s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap();
    assert_eq!(fav.items.len(), 1);
    assert!(fav.items[0].is_favorite);

    let c = s.create_collection("C", None).unwrap();
    s.add_to_collection(c, &[k]).unwrap();
    assert_eq!(s.query_faces(&FaceQuery { collection_id: Some(c), ..Default::default() }).unwrap().total, 1);

    let t1 = s.tag("serif").unwrap();
    let t2 = s.tag("tiêu đề").unwrap();
    s.tag_faces(t1, &[k]).unwrap();
    assert_eq!(s.query_faces(&FaceQuery { tag_ids: vec![t1], ..Default::default() }).unwrap().total, 1);
    assert_eq!(s.query_faces(&FaceQuery { tag_ids: vec![t1, t2], ..Default::default() }).unwrap().total, 0, "nhiều tag = AND");
}

#[test]
fn missing_files_hidden_unless_requested() {
    let (_d, s) = seeded();
    s.conn().execute("UPDATE file_locations SET missing=1 WHERE path='/lib/roboto.ttf'", []).unwrap();
    assert_eq!(s.query_faces(&FaceQuery::default()).unwrap().total, 6);
    let with = s.query_faces(&FaceQuery { include_missing: true, ..Default::default() }).unwrap();
    assert_eq!(with.total, 7);
    assert!(with.items.iter().any(|r| r.family == "Roboto" && r.missing));
}

#[test]
fn query_deserializes_from_camel_case_json() {
    let q: FaceQuery = serde_json::from_str(r#"{"text":"dep","favoritesOnly":true,"sort":"weight","limit":20}"#).unwrap();
    assert!(q.favorites_only && q.sort == SortKey::Weight && q.limit == 20);
}

#[test]
#[ignore = "hiệu năng: cargo test -p fontatlas-store --release --test query -- --ignored --nocapture"]
fn perf_50k_faces_under_50ms() {
    let d = tmp();
    let mut s = open_store(d.path());
    let base = report("open_sans.ttf");
    for chunk in 0..50 {
        let items = (0..1000).map(|i| {
            let n = chunk * 1000 + i;
            item(1, &format!("/big/{n}.ttf"), &fake_report(&base, &format!("f{n}"), &format!("Họ Phông {n}")))
        }).collect();
        s.ingest(items, 1).unwrap();
    }
    let qs = [
        FaceQuery::default(),
        FaceQuery { offset: 40_000, ..Default::default() },
        text("phong 4999"),
        FaceQuery { vi: Some(ViFilter::Full), sort: SortKey::Weight, ..Default::default() },
    ];
    for q in &qs {
        let t = std::time::Instant::now();
        let p = s.query_faces(q).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        eprintln!("{q:?} -> total {} in {ms:.1} ms", p.total);
        assert!(ms < 50.0, "S6: < 50 ms trên 50.000 font");
    }
}

#[test]
fn row_carries_file_size_in_bytes() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("open_sans.ttf");
    let mut it = item(1, "/fonts/OpenSans.ttf", &r);
    it.file.size = 12345;
    s.ingest(vec![it], 1000).unwrap();
    let row = s.query_faces(&FaceQuery::default()).unwrap().items.remove(0);
    assert_eq!(row.file_size, 12345);
}

/// Hợp đồng JSON TS↔Rust cho Plan 5. Sinh lại: UPDATE_FIXTURES=1 cargo test -p fontatlas-store --test query face_row_json
#[test]
fn face_row_json_matches_contract_fixture() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("open_sans.ttf");
    let mut it = item(1, "/fonts/OpenSans.ttf", &r);
    it.file.size = 12345;
    s.ingest(vec![it], 1_700_000_000_000).unwrap();
    let row = s.query_faces(&FaceQuery::default()).unwrap().items.remove(0);
    let got = serde_json::to_value(&row).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/face_row.json");
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&got).unwrap() + "\n").unwrap();
    }
    let want: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture face_row.json")).unwrap();
    assert_eq!(got, want);
    for k in ["faceId", "hashHex", "faceIndex", "family", "fileSize", "viStatus", "isFavorite", "addedAt"] {
        assert!(got.get(k).is_some(), "thiếu khoá camelCase {k}");
    }
}

#[test]
fn word_prefix_matches_rank_before_substring_matches() {
    let (_d, mut s) = seeded();
    let base = report("open_sans.ttf");
    // "adept" chứa "dep" giữa từ và đứng trước "dep sans" theo bảng chữ cái
    s.ingest(vec![item(1, "/lib/adept.ttf", &fake_report(&base, "adept", "Adept Grotesk"))], 11).unwrap();
    for descending in [false, true] {
        let got = fams(&s, &FaceQuery { text: Some("dep".into()), descending, ..Default::default() });
        assert_eq!(got.first().map(String::as_str), Some("Đẹp Sans"), "desc={descending}: {got:?}");
        let pos = |f: &str| got.iter().position(|x| x == f).unwrap();
        assert!(pos("Đẹp Sans") < pos("Source Code Pro"));
        assert!(pos("Đẹp Sans") < pos("Adept Grotesk"));
    }
    // trong nhóm chuỗi con vẫn theo sort người dùng chọn
    let asc = fams(&s, &text("dep"));
    assert!(asc.iter().position(|x| x == "Adept Grotesk") < asc.iter().position(|x| x == "Source Code Pro"));
}
