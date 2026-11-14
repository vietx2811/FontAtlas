//! G7: nhận diện font Variable trùng tên PostScript với bản static (chỉ đọc library).
mod common;
use common::*;
use fontatlas_store::Store;

/// Thêm một face static giả (Roboto) với tên PostScript `ps`; trả về face_id.
fn add_static(s: &mut Store, seed: &str, ps: &str) -> i64 {
    let base = report("roboto.ttf");
    let mut r = fake_report(&base, seed, "Oswald");
    for f in &mut r.faces {
        f.metadata.postscript_name = Some(ps.to_string());
    }
    let path = format!("/lib/{seed}.ttf");
    s.ingest(vec![item(1, &path, &r)], 10).unwrap();
    s.conn().query_row("SELECT f.face_id FROM faces f JOIN file_locations l ON l.file_id = f.file_id WHERE l.path = ?1", [&path], |r| r.get(0)).unwrap()
}

fn add_vf(s: &mut Store) -> i64 {
    let r = report("oswald_vf.ttf");
    s.ingest(vec![item(1, "/lib/oswald_vf.ttf", &r)], 10).unwrap();
    s.conn().query_row("SELECT face_id FROM faces WHERE is_variable = 1", [], |r| r.get(0)).unwrap()
}

#[test]
fn default_instance_clash_is_exact_and_names_the_static() {
    let d = tmp();
    let mut s = open_store(d.path());
    let vf = add_vf(&mut s);
    let ps: String = s.conn().query_row("SELECT postscript_name FROM faces WHERE face_id = ?1", [vf], |r| r.get(0)).unwrap();
    let st = add_static(&mut s, "static-regular", &ps);
    let c = s.name_clashes().unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].vf_face_id, vf);
    assert_eq!(c[0].static_face_ids, vec![st]);
    assert_eq!(c[0].ps_names, vec![ps]);
    assert!(c[0].exact);
}

#[test]
fn only_instance_derived_name_is_not_exact() {
    let d = tmp();
    let mut s = open_store(d.path());
    let vf = add_vf(&mut s);
    let st = add_static(&mut s, "static-light", "Oswald-Regular_Light");
    let c = s.name_clashes().unwrap();
    assert_eq!(c.len(), 1, "{c:?}");
    assert_eq!((c[0].vf_face_id, c[0].static_face_ids.clone(), c[0].exact), (vf, vec![st], false));
    assert_eq!(c[0].ps_names, vec!["Oswald-Regular_Light".to_string()]);
}

#[test]
fn different_postscript_names_are_not_a_clash() {
    let d = tmp();
    let mut s = open_store(d.path());
    add_vf(&mut s);
    add_static(&mut s, "other", "OswaldStatic-Heavy");
    assert!(s.name_clashes().unwrap().is_empty());
}

#[test]
fn missing_static_no_longer_clashes() {
    let d = tmp();
    let mut s = open_store(d.path());
    let vf = add_vf(&mut s);
    let ps: String = s.conn().query_row("SELECT postscript_name FROM faces WHERE face_id = ?1", [vf], |r| r.get(0)).unwrap();
    add_static(&mut s, "gone", &ps);
    assert_eq!(s.name_clashes().unwrap().len(), 1);
    s.conn().execute("UPDATE file_locations SET missing = 1 WHERE path = '/lib/gone.ttf'", []).unwrap();
    assert!(s.name_clashes().unwrap().is_empty());
}

#[test]
fn two_variable_fonts_never_clash_with_each_other() {
    let d = tmp();
    let mut s = open_store(d.path());
    add_vf(&mut s);
    let r = fake_report(&report("oswald_vf.ttf"), "vf-twin", "Oswald");
    s.ingest(vec![item(1, "/lib/vf-twin.ttf", &r)], 10).unwrap();
    assert!(s.name_clashes().unwrap().is_empty());
}

/// Cặp fixture thật dùng cho native QA «Giữ cả hai»: Roboto VF (OFL) có PostScript mặc định trùng `roboto.ttf` static.
#[test]
fn real_roboto_vf_clashes_with_the_roboto_static_fixture() {
    let d = tmp();
    let mut s = open_store(d.path());
    let st = report("roboto.ttf");
    let vf = report("roboto_vf.ttf");
    s.ingest(vec![item(1, "/lib/roboto.ttf", &st), item(1, "/lib/roboto_vf.ttf", &vf)], 10).unwrap();
    let c = s.name_clashes().unwrap();
    assert_eq!(c.len(), 1, "{c:?}");
    assert!(c[0].exact, "PostScript mặc định phải trùng đúng: {c:?}");
    assert_eq!(c[0].ps_names, vec!["Roboto-Regular".to_string()]);
    assert_eq!(c[0].static_face_ids.len(), 1);
}
