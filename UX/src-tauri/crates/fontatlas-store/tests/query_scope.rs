mod common;
use common::*;
use fontatlas_store::{FaceQuery, SortKey, Store, ViFilter};

fn seeded() -> (tempfile::TempDir, Store, Vec<(i64, String)>) {
    let d = tmp();
    let mut s = open_store(d.path());
    let base = report("open_sans.ttf");
    let items: Vec<_> = (0..12)
        .map(|i| item(1, &format!("/lib/{i}.ttf"), &fake_report(&base, &format!("s{i}"), &format!("Fam{i:02}"))))
        .collect();
    s.ingest(items, 10).unwrap();
    let rows = s.query_faces(&FaceQuery { limit: 100, ..Default::default() }).unwrap().items;
    let ids = rows.iter().map(|r| (r.face_id, r.family.clone())).collect();
    (d, s, ids)
}

#[test]
fn scope_limits_rows_and_total() {
    let (_d, s, ids) = seeded();
    let scope: Vec<(i64, f32)> = ids.iter().take(4).map(|(i, _)| (*i, 0.5)).collect();
    let p = s.query_faces_scoped(&FaceQuery::default(), Some(&scope)).unwrap();
    assert_eq!((p.total, p.items.len()), (4, 4));
    // gọi lại với scope khác: không dính dữ liệu cũ
    let p = s.query_faces_scoped(&FaceQuery::default(), Some(&scope[..1])).unwrap();
    assert_eq!(p.total, 1);
    assert_eq!(s.query_faces_scoped(&FaceQuery::default(), None).unwrap().total, 12);
}

#[test]
fn match_sort_orders_by_score_desc() {
    let (_d, s, ids) = seeded();
    let scope: Vec<(i64, f32)> = vec![(ids[3].0, 0.2), (ids[7].0, 0.9), (ids[5].0, 0.9), (ids[1].0, 0.5)];
    let q = FaceQuery { sort: SortKey::Match, ..Default::default() };
    let got: Vec<i64> = s.query_faces_scoped(&q, Some(&scope)).unwrap().items.iter().map(|r| r.face_id).collect();
    // điểm bằng nhau → theo family (Fam05 < Fam07)
    assert_eq!(got, vec![ids[5].0, ids[7].0, ids[1].0, ids[3].0]);
    // không có scope: Match = Family
    let a: Vec<i64> = s.query_faces(&q).unwrap().items.iter().map(|r| r.face_id).collect();
    let b: Vec<i64> = s.query_faces(&FaceQuery::default()).unwrap().items.iter().map(|r| r.face_id).collect();
    assert_eq!(a, b);
}

#[test]
fn scope_combines_with_text_and_vi_filters() {
    let (_d, s, ids) = seeded();
    let scope: Vec<(i64, f32)> = ids.iter().map(|(i, _)| (*i, 1.0)).collect();
    let q = FaceQuery { text: Some("Fam03".into()), ..Default::default() };
    assert_eq!(s.query_faces_scoped(&q, Some(&scope[..6])).unwrap().total, 1);
    assert_eq!(s.query_faces_scoped(&q, Some(&scope[6..])).unwrap().total, 0);
    let q = FaceQuery { vi: Some(ViFilter::Full), ..Default::default() };
    let all_full = s.query_faces(&q).unwrap().total;
    assert_eq!(s.query_faces_scoped(&q, Some(&scope)).unwrap().total, all_full);
}

#[test]
fn large_scope_50k_ids_ok() {
    let (_d, s, ids) = seeded();
    let mut scope: Vec<(i64, f32)> = (1_000_000..1_050_000).map(|i| (i, 0.1)).collect();
    scope.push((ids[2].0, 0.9));
    let p = s.query_faces_scoped(&FaceQuery { sort: SortKey::Match, ..Default::default() }, Some(&scope)).unwrap();
    assert_eq!((p.total, p.items.len()), (1, 1));
}
