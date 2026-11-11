mod common;
use common::*;
use fontatlas_store::{FaceKey, FaceQuery, NewResolution, NewPair, SourceKind, Store};

struct W {
    _d: tempfile::TempDir,
    s: Store,
    a: FaceKey,
    b: FaceKey,
}

/// A có hai đường dẫn (cùng nội dung), B là bản khác nội dung cùng family.
fn setup() -> W {
    let d = tmp();
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(d.path())).unwrap();
    let base = report("roboto.ttf");
    let ra = fake_report(&base, "a", "Dup");
    let rb = fake_report(&base, "b", "Dup");
    s.ingest(vec![item(src, "/f/A.ttf", &ra), item(src, "/g/A.ttf", &ra), item(src, "/f/B.ttf", &rb)], 1).unwrap();
    let a = FaceKey { hash: hash_of(&ra), face_index: 0 };
    let b = FaceKey { hash: hash_of(&rb), face_index: 0 };
    W { _d: d, s, a, b }
}

fn total(s: &Store) -> u64 {
    s.query_faces(&FaceQuery::default()).unwrap().total
}

fn res(w: &W, loser: &FaceKey, loser_path: &str, action: &str) -> NewResolution {
    NewResolution {
        kind: "versions".into(),
        group_key: "dup".into(),
        keeper: w.a,
        keeper_path: "/f/A.ttf".into(),
        loser: *loser,
        loser_path: loser_path.into(),
        action: action.into(),
        trash_path: None,
        reason: r#"{"code":"newer"}"#.into(),
        was_active: false,
        loser_scope: None,
    }
}

#[test]
fn hiding_a_copy_keeps_the_face_visible_through_its_other_path() {
    let mut w = setup();
    assert_eq!(total(&w.s), 2);
    let mut r = res(&w, &w.a.clone(), "/g/A.ttf", "hide");
    r.kind = "same_file".into();
    w.s.apply_resolutions("b1", &[r], 10).unwrap();
    assert_eq!(total(&w.s), 2, "A vẫn hiện nhờ /f/A.ttf");
    let rows = w.s.duplicate_rows().unwrap();
    assert!(rows.iter().all(|r| r.path != "/g/A.ttf"), "đường dẫn đã ẩn không còn trong nhóm trùng");
    // activation không thấy đường dẫn đã ẩn (không chỉ xếp sau): `locate` duyệt MỌI vị trí nên phải vắng hẳn
    let t = w.s.file_target(&w.a.hash).unwrap().unwrap();
    let paths: Vec<String> = t.locations.iter().map(|l| l.path.to_string_lossy().into_owned()).collect();
    assert_eq!(paths, vec!["/f/A.ttf".to_string()]);
}

/// Review #2: mọi nơi đọc "đường dẫn của face" đều bỏ vị trí đã ẩn — dù nó có location_id nhỏ nhất.
#[test]
fn every_path_reader_skips_hidden_locations() {
    let mut w = setup();
    // thứ tự chèn: /f/A.ttf trước, /g/A.ttf sau → ẩn /f/A.ttf (vị trí "đầu tiên") để bản còn lại phải là /g/A.ttf
    let mut r = res(&w, &w.a.clone(), "/f/A.ttf", "hide");
    r.kind = "same_file".into();
    r.keeper_path = "/g/A.ttf".into();
    w.s.apply_resolutions("b1", &[r], 10).unwrap();
    assert_eq!(w.s.face_path(&w.a).unwrap().unwrap().to_string_lossy(), "/g/A.ttf");
    assert_eq!(w.s.face_read_path(&w.a).unwrap().unwrap().to_string_lossy(), "/g/A.ttf");
    let row = w.s.query_faces(&FaceQuery::default()).unwrap().items.into_iter().find(|r| r.hash_hex == w.a.hash_hex()).unwrap();
    assert_eq!(row.path.as_deref(), Some("/g/A.ttf"), "FaceRow.path không lấy vị trí đã ẩn");
    assert!(w.s.has_location("/g/A.ttf").unwrap() && !w.s.has_location("/f/A.ttf").unwrap());

    // ẩn nốt bản B (khác hash) → không còn đường dẫn nào cho B ở mọi nơi
    let mut hide_b = res(&w, &w.b.clone(), "/f/B.ttf", "hide");
    hide_b.keeper_path = "/g/A.ttf".into(); // /f/A.ttf đã bị ẩn ở trên nên không thể là bản giữ
    w.s.apply_resolutions("b2", &[hide_b], 11).unwrap();
    assert!(w.s.face_path(&w.b).unwrap().is_none());
    assert!(w.s.face_read_path(&w.b).unwrap().is_none());
    assert!(w.s.face_local_path(&w.b).unwrap().is_none());
    assert!(w.s.file_target(&w.b.hash).unwrap().unwrap().locations.is_empty(), "activation không có chỗ nào để bật B");
    // Worker nhúng / ngôn ngữ chạy trên kết nối CHỈ-library (không thấy sổ u.*): với tệp đã vào Thùng rác, cờ missing là
    // thứ chúng thấy. Sau khi đánh dấu, job nhúng của A phải dùng vị trí còn lại.
    assert!(w.s.set_location_missing("/f/A.ttf", true).unwrap());
    w.s.enqueue_embeddings("v1").unwrap();
    let jobs = w.s.claim_embedding_batch("v1", 10).unwrap();
    let a_job = jobs.iter().find(|j| j.key == w.a).expect("A vẫn được nhúng");
    assert_eq!(a_job.path.to_string_lossy(), "/g/A.ttf", "nhúng từ vị trí chưa missing");
    assert!(!w.s.set_location_missing("/f/A.ttf", true).unwrap(), "đã missing rồi: không đổi");
    assert!(w.s.set_location_missing("/f/A.ttf", false).unwrap(), "hoàn tác đưa về có mặt");
}

#[test]
fn hiding_a_different_content_version_removes_it_from_queries_and_undo_restores_it() {
    let mut w = setup();
    let ids = w.s.apply_resolutions("b1", &[res(&w, &w.b.clone(), "/f/B.ttf", "hide")], 10).unwrap();
    assert_eq!(total(&w.s), 1);
    let q = FaceQuery { include_hidden: true, ..Default::default() };
    assert_eq!(w.s.query_faces(&q).unwrap().total, 2, "include_hidden thấy lại bản đã ẩn");
    assert!(w.s.face_path(&w.b).unwrap().is_none(), "không mở bản đã ẩn");
    let t = w.s.file_target(&w.b.hash).unwrap().unwrap();
    assert!(t.locations.iter().all(|l| l.missing), "bản đã ẩn không thể bật");

    let row = w.s.undo_resolution(ids[0], 20).unwrap();
    assert_eq!(row.undone_at, Some(20));
    assert_eq!(total(&w.s), 2);
    assert!(w.s.face_path(&w.b).unwrap().is_some());
}

#[test]
fn resolving_twice_for_the_same_path_is_rejected() {
    let mut w = setup();
    w.s.apply_resolutions("b1", &[res(&w, &w.b.clone(), "/f/B.ttf", "hide")], 10).unwrap();
    assert!(w.s.apply_resolutions("b2", &[res(&w, &w.b.clone(), "/f/B.ttf", "hide")], 11).is_err());
    assert_eq!(w.s.resolutions(true).unwrap().len(), 1, "lần lỗi không để lại gì");
}

#[test]
fn a_failed_batch_leaves_nothing_behind() {
    let mut w = setup();
    let good = res(&w, &w.b.clone(), "/f/B.ttf", "hide");
    let mut bad = res(&w, &w.b.clone(), "/g/A.ttf", "hide");
    bad.action = "delete".into(); // không hợp lệ
    assert!(w.s.apply_resolutions("b1", &[good, bad], 10).is_err());
    assert_eq!(w.s.resolutions(false).unwrap().len(), 0);
    assert_eq!(total(&w.s), 2);
}

#[test]
fn links_are_copied_to_the_keeper_and_undo_removes_only_what_was_copied() {
    let mut w = setup();
    let (a, b) = (w.a, w.b);
    // người dùng đã yêu thích B, cho B vào collection + tag + Case; A đã sẵn trong collection
    w.s.set_favorite(&b, true).unwrap();
    let col = w.s.create_collection("Bộ", None).unwrap();
    w.s.add_to_collection(col, &[a, b]).unwrap();
    let tag = w.s.tag("logo").unwrap();
    w.s.tag_faces(tag, &[b]).unwrap();
    let case = w.s.create_case("Dự án", None, None).unwrap();
    w.s.add_to_case(case, &[b], "added").unwrap();

    let ids = w.s.apply_resolutions("b1", &[res(&w, &b, "/f/B.ttf", "hide")], 10).unwrap();
    assert!(w.s.favorites().unwrap().contains(&a), "yêu thích chuyển sang bản giữ");
    assert_eq!(w.s.tags_of(&a).unwrap().len(), 1);
    assert!(w.s.case_faces(case).unwrap().iter().any(|f| f.key == a));
    assert!(w.s.favorites().unwrap().contains(&b), "không xoá dữ liệu của bản bị ẩn");

    w.s.undo_resolution(ids[0], 20).unwrap();
    assert!(!w.s.favorites().unwrap().contains(&a), "hoàn tác gỡ bản sao đã chuyển");
    assert!(w.s.favorites().unwrap().contains(&b));
    assert!(w.s.tags_of(&a).unwrap().is_empty());
    assert!(!w.s.case_faces(case).unwrap().iter().any(|f| f.key == a));
    // A vốn đã trong collection trước khi giải quyết: hoàn tác KHÔNG được gỡ nó
    assert!(w.s.collection_faces(col).unwrap().len() >= 2);
}

#[test]
fn pairs_and_combo_slots_follow_the_keeper_and_undo_puts_them_back() {
    let mut w = setup();
    let (a, b) = (w.a, w.b);
    let pid = w
        .s
        .save_pair(&NewPair { a: b, a_role: "heading".into(), b, b_role: "body".into(), mode: None, preset: None, sample_text: None, collection_id: None, note: None })
        .unwrap();
    let case = w.s.create_case("C", None, None).unwrap();
    let combo = w.s.create_combo(case, None, None).unwrap();
    w.s.set_combo_slot(combo, "heading", Some(&b), None, None).unwrap();

    let ids = w.s.apply_resolutions("b1", &[res(&w, &b, "/f/B.ttf", "hide")], 10).unwrap();
    let p = w.s.pairs(None).unwrap().into_iter().find(|p| p.id == pid).unwrap();
    assert_eq!((p.a, p.b), (a, a), "cả hai phía của cặp trỏ sang bản giữ");
    let slot = &w.s.combo(combo).unwrap().slots;
    assert!(slot.iter().any(|s| s.role == "heading" && s.key == a));

    w.s.undo_resolution(ids[0], 20).unwrap();
    let p = w.s.pairs(None).unwrap().into_iter().find(|p| p.id == pid).unwrap();
    assert_eq!((p.a, p.b), (b, b));
    assert!(w.s.combo(combo).unwrap().slots.iter().any(|s| s.role == "heading" && s.key == b));
}

#[test]
fn undo_does_not_overwrite_a_pair_the_user_changed_meanwhile() {
    let mut w = setup();
    let (a, b) = (w.a, w.b);
    let pid = w
        .s
        .save_pair(&NewPair { a: b, a_role: "heading".into(), b: a, b_role: "body".into(), mode: None, preset: None, sample_text: None, collection_id: None, note: None })
        .unwrap();
    let ids = w.s.apply_resolutions("b1", &[res(&w, &b, "/f/B.ttf", "hide")], 10).unwrap();
    w.s.delete_pair(pid).unwrap();
    w.s.undo_resolution(ids[0], 20).unwrap(); // cặp đã bị xoá: hoàn tác vẫn thành công
    assert_eq!(total(&w.s), 2);
}

#[test]
fn ignored_groups_are_remembered_and_can_be_restored() {
    let mut w = setup();
    assert!(w.s.ignored_groups().unwrap().is_empty());
    w.s.ignore_group("versions", "dup-ps", 10).unwrap();
    w.s.ignore_group("versions", "dup-ps", 11).unwrap(); // lặp lại: không lỗi, không nhân đôi
    w.s.ignore_group("same_file", "abcd", 12).unwrap();
    let l = w.s.ignored_groups().unwrap();
    assert_eq!(l.len(), 2);
    assert_eq!(l[0], ("same_file".to_string(), "abcd".to_string(), 12), "mới nhất trước");
    assert!(w.s.unignore_group("versions", "dup-ps").unwrap());
    assert!(!w.s.unignore_group("versions", "dup-ps").unwrap(), "đã bỏ rồi");
    assert_eq!(w.s.ignored_groups().unwrap().len(), 1);
    assert!(w.s.ignore_group("bogus", "x", 1).is_err(), "loại nhóm không hợp lệ");
}

#[test]
fn source_priority_defaults_to_creation_order_and_can_be_reordered() {
    let mut w = setup();
    let first = w.s.sources().unwrap()[0].id;
    let second = w.s.add_source(SourceKind::Directory, "B", Some(std::path::Path::new("/tmp/zzz-b"))).unwrap();
    let third = w.s.add_source(SourceKind::Directory, "C", Some(std::path::Path::new("/tmp/zzz-c"))).unwrap();
    assert_eq!(w.s.source_priority().unwrap(), vec![first, second, third]);
    w.s.set_source_priority(&[third, first]).unwrap(); // thiếu `second`: nguồn không nêu xếp sau, theo thứ tự tạo
    assert_eq!(w.s.source_priority().unwrap(), vec![third, first, second]);
    let fourth = w.s.add_source(SourceKind::Directory, "D", Some(std::path::Path::new("/tmp/zzz-d"))).unwrap();
    assert_eq!(w.s.source_priority().unwrap(), vec![third, first, second, fourth], "nguồn mới xếp cuối");
    assert!(w.s.set_source_priority(&[third, 99_999]).is_err(), "nguồn không tồn tại");
    assert_eq!(w.s.source_priority().unwrap(), vec![third, first, second, fourth], "lỗi thì không đổi gì");
}

fn three() -> (W, FaceKey) {
    // thêm bản C (cùng family, khác nội dung) để dựng chuỗi A→B→C
    let mut w = setup();
    let src = w.s.sources().unwrap()[0].id;
    let rc = fake_report(&report("roboto.ttf"), "c", "Dup");
    w.s.ingest(vec![item(src, "/f/C.ttf", &rc)], 2).unwrap();
    let c = FaceKey { hash: hash_of(&rc), face_index: 0 };
    (w, c)
}

fn res_between(keeper: &FaceKey, kp: &str, loser: &FaceKey, lp: &str) -> NewResolution {
    NewResolution {
        kind: "versions".into(),
        group_key: "dup".into(),
        keeper: *keeper,
        keeper_path: kp.into(),
        loser: *loser,
        loser_path: lp.into(),
        action: "hide".into(),
        trash_path: None,
        reason: "{}".into(),
        was_active: false,
        loser_scope: None,
    }
}

/// Review #4: bản giữ đang bị ẩn bởi quyết định sống không thể làm bản giữ của quyết định khác.
#[test]
fn a_hidden_face_cannot_become_a_keeper() {
    let (mut w, c) = three();
    w.s.apply_resolutions("b1", &[res_between(&w.a, "/f/A.ttf", &w.b, "/f/B.ttf")], 10).unwrap();
    // C giữ, nhưng dùng B (đã ẩn) làm bản giữ → từ chối
    assert!(w.s.apply_resolutions("b2", &[res_between(&w.b, "/f/B.ttf", &c, "/f/C.ttf")], 11).is_err());
}

/// Review #8: chuỗi A→B→C — không hoàn tác quyết định cũ trước quyết định mới phụ thuộc vào nó.
#[test]
fn undo_is_blocked_while_a_newer_resolution_depends_on_the_keeper() {
    let (mut w, c) = three();
    // r1: B bị ẩn, giữ A.  r2: A bị ẩn, giữ C  (A, bản giữ của r1, thành bản thua của r2)
    let r1 = w.s.apply_resolutions("b1", &[res_between(&w.a, "/f/A.ttf", &w.b, "/f/B.ttf")], 10).unwrap()[0];
    let r2 = w.s.apply_resolutions("b2", &[res_between(&c, "/f/C.ttf", &w.a, "/f/A.ttf")], 11).unwrap()[0];
    assert_eq!(w.s.newer_dependents(r1).unwrap(), vec![r2]);
    assert!(w.s.undo_resolution(r1, 20).is_err(), "r2 phụ thuộc bản giữ của r1");
    assert_eq!(w.s.resolutions(true).unwrap().len(), 2, "lỗi thì không đổi gì");
    // hoàn tác theo thứ tự ngược thì được
    w.s.undo_resolution(r2, 21).unwrap();
    w.s.undo_resolution(r1, 22).unwrap();
    assert_eq!(total(&w.s), 3);
}

/// Minor: hai bản thua cùng mẻ cùng nằm trong một collection → liên kết trên bản giữ là CHUNG; hoàn tác một quyết định
/// không được xoá nó khi quyết định kia còn sống.
#[test]
fn a_link_shared_by_two_resolutions_of_one_batch_survives_until_both_are_undone() {
    let (mut w, c) = three();
    let (a, b) = (w.a, w.b);
    let col = w.s.create_collection("Chung", None).unwrap();
    w.s.add_to_collection(col, &[b, c]).unwrap();
    let ids = w
        .s
        .apply_resolutions("b1", &[res_between(&a, "/f/A.ttf", &b, "/f/B.ttf"), res_between(&a, "/f/A.ttf", &c, "/f/C.ttf")], 500)
        .unwrap();
    assert!(w.s.collection_faces(col).unwrap().contains(&a));
    w.s.undo_resolution(ids[0], 600).unwrap();
    assert!(w.s.collection_faces(col).unwrap().contains(&a), "quyết định thứ hai còn sống vẫn cần liên kết này");
    w.s.undo_resolution(ids[1], 601).unwrap();
    assert!(!w.s.collection_faces(col).unwrap().contains(&a), "cả hai đã hoàn tác → gỡ");
}

/// Review #8: undo chỉ xoá liên kết do CHÍNH lần giải quyết tạo; người dùng xoá rồi tạo lại sau đó thì giữ nguyên.
#[test]
fn undo_keeps_a_link_the_user_recreated_after_resolving() {
    let mut w = setup();
    let (a, b) = (w.a, w.b);
    w.s.set_favorite(&b, true).unwrap();
    let id = w.s.apply_resolutions("b1", &[res(&w, &b, "/f/B.ttf", "hide")], 1000).unwrap()[0];
    assert!(w.s.favorites().unwrap().contains(&a));
    // người dùng bỏ yêu thích A rồi tự bấm lại (dấu thời gian khác)
    w.s.set_favorite(&a, false).unwrap();
    w.s.set_favorite(&a, true).unwrap();
    w.s.undo_resolution(id, 2000).unwrap();
    assert!(w.s.favorites().unwrap().contains(&a), "yêu thích do người dùng tạo lại không bị xoá nhầm");
}

/// Review #8: không tạo cặp tự trỏ (a == b) khi cả hai phía là bản thua và bản giữ.
#[test]
fn a_pair_of_the_loser_and_the_keeper_is_not_turned_into_a_self_pair() {
    let mut w = setup();
    let (a, b) = (w.a, w.b);
    let pid = w
        .s
        .save_pair(&NewPair { a, a_role: "heading".into(), b, b_role: "body".into(), mode: None, preset: None, sample_text: None, collection_id: None, note: None })
        .unwrap();
    w.s.apply_resolutions("b1", &[res(&w, &b, "/f/B.ttf", "hide")], 10).unwrap();
    let p = w.s.pairs(None).unwrap().into_iter().find(|p| p.id == pid).unwrap();
    assert_eq!((p.a, p.b), (a, b), "cặp (A,B) giữ nguyên, không thành (A,A)");
}

/// Review #9: pha 1 ghi sổ pending (đã ẩn) trước khi đụng tệp; complete mới đánh dấu xong; abort = undo.
#[test]
fn a_pending_resolution_already_hides_and_can_be_completed_or_aborted() {
    let mut w = setup();
    let id = w.s.begin_resolution("b1", &res(&w, &w.b.clone(), "/f/B.ttf", "hide"), 10).unwrap();
    assert_eq!(total(&w.s), 1, "pending đã ẩn");
    let p = w.s.pending_resolutions().unwrap();
    assert_eq!((p.len(), p[0].id, p[0].status.as_str()), (1, id, "pending"));
    w.s.complete_resolution(id, Some("/trash/B.ttf")).unwrap();
    assert!(w.s.pending_resolutions().unwrap().is_empty());
    assert_eq!(w.s.resolution(id).unwrap().trash_path.as_deref(), Some("/trash/B.ttf"));
    // một pending khác bị huỷ → trở lại như chưa làm gì
    let id2 = w.s.begin_resolution("b2", &res(&w, &w.a.clone(), "/g/A.ttf", "hide"), 11).unwrap();
    w.s.undo_resolution(id2, 12).unwrap();
    assert!(w.s.pending_resolutions().unwrap().is_empty());
    assert!(w.s.is_hidden_path("/f/B.ttf").unwrap() && !w.s.is_hidden_path("/g/A.ttf").unwrap());
}

#[test]
fn resolutions_survive_a_rescan_of_the_index() {
    let mut w = setup();
    w.s.apply_resolutions("b1", &[res(&w, &w.b.clone(), "/f/B.ttf", "hide")], 10).unwrap();
    // ingest lại cùng vị trí (như rescan): vẫn ẩn
    let base = report("roboto.ttf");
    let rb = fake_report(&base, "b", "Dup");
    let src = w.s.sources().unwrap()[0].id;
    w.s.ingest(vec![item(src, "/f/B.ttf", &rb)], 30).unwrap();
    assert_eq!(total(&w.s), 1);
}
