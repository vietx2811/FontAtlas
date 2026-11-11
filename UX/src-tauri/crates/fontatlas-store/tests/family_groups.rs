//! R3: `family_groups` + `group_by_family` — mỗi family liền nhau, `start` khớp thứ tự `query_faces`.
mod common;
use common::*;
use fontatlas_store::{FaceQuery, SortKey, Store};

/// (family, weight, italic, release_year, first_seen)
type Spec = (&'static str, u16, bool, Option<u16>, i64);

fn seeded(specs: &[Spec]) -> (tempfile::TempDir, Store) {
    let d = tmp();
    let mut s = open_store(d.path());
    let base = report("roboto.ttf");
    let items: Vec<_> = specs
        .iter()
        .enumerate()
        .map(|(i, (fam, w, it, _, _))| {
            let mut r = fake_report(&base, &format!("{fam}-{w}-{it}-{i}"), fam);
            for f in &mut r.faces {
                f.metadata.weight_class = Some(*w);
                f.metadata.italic_angle = Some(if *it { -12.0 } else { 0.0 });
            }
            item(1, &format!("/lib/{i}-{fam}.ttf"), &r)
        })
        .collect();
    s.ingest(items, 10).unwrap();
    for (i, (_, _, _, ry, seen)) in specs.iter().enumerate() {
        s.conn()
            .execute(
                "UPDATE faces SET release_year = ?1, release_year_src = ?2 WHERE file_id = (SELECT file_id FROM file_locations WHERE path = ?3)",
                rusqlite::params![ry, ry.map(|_| "copyright"), format!("/lib/{i}-{}.ttf", specs[i].0)],
            )
            .unwrap();
        s.conn()
            .execute(
                "UPDATE files SET first_seen_at = ?1 WHERE file_id = (SELECT file_id FROM file_locations WHERE path = ?2)",
                rusqlite::params![seen, format!("/lib/{i}-{}.ttf", specs[i].0)],
            )
            .unwrap();
    }
    (d, s)
}

fn lib() -> Vec<Spec> {
    vec![
        ("Beta", 700, false, Some(2020), 30),
        ("Alpha", 400, false, Some(2005), 10),
        ("Beta", 400, true, Some(2019), 31),
        ("Gamma", 400, false, None, 50),
        ("Alpha", 700, false, Some(2010), 11),
        ("Beta", 400, false, Some(2018), 29),
        ("Alpha", 400, true, Some(2006), 12),
        ("Delta", 300, false, Some(1999), 5),
    ]
}

const SORTS: [SortKey; 6] = [SortKey::Family, SortKey::Weight, SortKey::Added, SortKey::Released, SortKey::FileCreated, SortKey::Auto];

fn grouped(sort: SortKey, descending: bool) -> FaceQuery {
    FaceQuery { sort, descending, group_by_family: true, limit: 100, ..Default::default() }
}

#[test]
fn groups_are_contiguous_and_start_matches_query_faces_for_every_sort_and_direction() {
    let (_d, s) = seeded(&lib());
    for sort in SORTS {
        for desc in [false, true] {
            let q = grouped(sort, desc);
            let page = s.query_faces(&q).unwrap();
            let groups = s.family_groups(&q, None).unwrap();
            assert_eq!(groups.iter().map(|g| g.count as u64).sum::<u64>(), page.total, "{sort:?} desc={desc}: tổng count = total");
            let mut next = 0u32;
            let mut seen = std::collections::HashSet::new();
            for g in &groups {
                assert_eq!(g.start, next, "{sort:?} desc={desc}: nhóm bắt đầu đúng nơi nhóm trước kết thúc");
                assert!(seen.insert(g.family.clone()), "{sort:?} desc={desc}: family {} bị xé", g.family);
                for r in &page.items[g.start as usize..(g.start + g.count) as usize] {
                    assert_eq!(r.family, g.family, "{sort:?} desc={desc}: face trong khoảng nhóm thuộc đúng family");
                }
                assert_eq!(g.match_count, g.count);
                next += g.count;
            }
            assert_eq!(next as u64, page.total);
        }
    }
}

#[test]
fn group_order_follows_the_sort_key_and_direction() {
    let (_d, s) = seeded(&lib());
    let fam = |q: &FaceQuery| s.family_groups(q, None).unwrap().into_iter().map(|g| g.family).collect::<Vec<_>>();
    assert_eq!(fam(&grouped(SortKey::Family, false)), ["Alpha", "Beta", "Delta", "Gamma"]);
    assert_eq!(fam(&grouped(SortKey::Family, true)), ["Gamma", "Delta", "Beta", "Alpha"]);
    // Released: nhóm theo năm cực trị; tăng dần = năm sớm nhất của family; Gamma (không rõ) luôn cuối
    assert_eq!(fam(&grouped(SortKey::Released, false)), ["Delta", "Alpha", "Beta", "Gamma"]);
    // giảm dần = năm muộn nhất: Beta 2020, Alpha 2010, Delta 1999
    assert_eq!(fam(&grouped(SortKey::Released, true)), ["Beta", "Alpha", "Delta", "Gamma"]);
    // Added: tăng = sớm nhất (Delta 5, Alpha 10, Beta 29, Gamma 50); giảm = muộn nhất (Gamma 50, Beta 31, Alpha 12, Delta 5)
    assert_eq!(fam(&grouped(SortKey::Added, false)), ["Delta", "Alpha", "Beta", "Gamma"]);
    assert_eq!(fam(&grouped(SortKey::Added, true)), ["Gamma", "Beta", "Alpha", "Delta"]);
    // Weight: tăng = kiểu nhẹ nhất (Delta 300, rồi Alpha/Beta/Gamma 400 theo tên); giảm = nặng nhất (700: Alpha, Beta; 400 Gamma; 300 Delta)
    assert_eq!(fam(&grouped(SortKey::Weight, false)), ["Delta", "Alpha", "Beta", "Gamma"]);
    assert_eq!(fam(&grouped(SortKey::Weight, true)), ["Alpha", "Beta", "Gamma", "Delta"]);
}

#[test]
fn faces_inside_a_group_go_weight_then_upright_before_italic() {
    let (_d, s) = seeded(&lib());
    let page = s.query_faces(&grouped(SortKey::Family, false)).unwrap();
    let alpha: Vec<(u16, bool)> = page.items.iter().filter(|r| r.family == "Alpha").map(|r| (r.weight_class.unwrap(), r.is_italic)).collect();
    assert_eq!(alpha, [(400, false), (400, true), (700, false)]);
}

#[test]
fn filters_shrink_match_count_but_style_total_stays_unfiltered() {
    let (_d, s) = seeded(&lib());
    let q = FaceQuery { italic: Some(false), ..grouped(SortKey::Family, false) };
    let groups = s.family_groups(&q, None).unwrap();
    let alpha = groups.iter().find(|g| g.family == "Alpha").unwrap();
    assert_eq!((alpha.match_count, alpha.style_total), (2, 3), "Alpha: 2 thẳng / 3 kiểu");
    let beta = groups.iter().find(|g| g.family == "Beta").unwrap();
    assert_eq!((beta.match_count, beta.style_total), (2, 3));
    assert!(groups.iter().all(|g| g.match_count <= g.style_total));
    // lọc cho family nào hết mặt thì nhóm biến mất
    let q = FaceQuery { text: Some("Gamma".into()), ..grouped(SortKey::Family, false) };
    let only = s.family_groups(&q, None).unwrap();
    assert_eq!(only.len(), 1);
    assert_eq!((only[0].start, only[0].count, only[0].style_total), (0, 1, 1));
}

#[test]
fn accent_variants_of_a_family_name_stay_one_group() {
    let (_d, s) = seeded(&[("Café", 400, false, None, 1), ("Cafe", 700, false, None, 2), ("Zed", 400, false, None, 3)]);
    let groups = s.family_groups(&grouped(SortKey::Family, false), None).unwrap();
    assert_eq!(groups.len(), 2, "Café/Cafe cùng family_key");
    assert_eq!(groups[0].count, 2);
}

#[test]
fn group_by_family_with_scope_orders_groups_by_best_score() {
    let (_d, s) = seeded(&lib());
    let ids: Vec<(String, i64)> = s
        .query_faces(&FaceQuery { limit: 100, ..Default::default() })
        .unwrap()
        .items
        .into_iter()
        .map(|r| (r.family, r.face_id))
        .collect();
    let id = |fam: &str, nth: usize| ids.iter().filter(|(f, _)| f == fam).nth(nth).unwrap().1;
    // Delta điểm thấp; Gamma cao nhất; Beta có một kiểu điểm rất cao
    let scope = vec![(id("Delta", 0), 1.0), (id("Gamma", 0), 9.0), (id("Alpha", 0), 2.0), (id("Beta", 0), 3.0), (id("Beta", 1), 8.0)];
    let q = grouped(SortKey::Match, false);
    let groups = s.family_groups(&q, Some(&scope)).unwrap();
    assert_eq!(groups.iter().map(|g| g.family.as_str()).collect::<Vec<_>>(), ["Gamma", "Beta", "Alpha", "Delta"]);
    let page = s.query_faces_scoped(&q, Some(&scope)).unwrap();
    for g in &groups {
        assert!(page.items[g.start as usize..(g.start + g.count) as usize].iter().all(|r| r.family == g.family));
    }
}

#[test]
fn family_groups_over_2300_faces_stays_fast() {
    let d = tmp();
    let mut s = open_store(d.path());
    let base = report("roboto.ttf");
    let items: Vec<_> = (0..2300)
        .map(|i| {
            let fam = format!("Fam{:03}", i / 5);
            let mut r = fake_report(&base, &format!("s{i}"), &fam);
            for f in &mut r.faces {
                f.metadata.weight_class = Some(100 + (i % 9) as u16 * 100);
            }
            item(1, &format!("/lib/{i}.ttf"), &r)
        })
        .collect();
    s.ingest(items, 10).unwrap();
    let q = grouped(SortKey::Released, true);
    let t = std::time::Instant::now();
    let groups = s.family_groups(&q, None).unwrap();
    let ms = t.elapsed().as_millis();
    assert_eq!(groups.len(), 460);
    assert!(ms < 50, "family_groups mất {ms} ms (mục tiêu < 50 ms)");
}
