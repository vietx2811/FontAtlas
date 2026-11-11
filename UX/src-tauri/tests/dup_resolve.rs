//! Giải quyết trùng lặp end-to-end: quét thật → xem trước → áp dụng (ẩn / Thùng rác giả) → hoàn tác.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::dup_resolve::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, dup_resolve, duplicates, library, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn patch_name(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let u16be = |s: &str| s.encode_utf16().flat_map(|c| c.to_be_bytes()).collect::<Vec<u8>>();
    let mut out = bytes.to_vec();
    for (a, b) in [(from.as_bytes().to_vec(), to.as_bytes().to_vec()), (u16be(from), u16be(to))] {
        let mut i = 0;
        while i + a.len() <= out.len() {
            if out[i..i + a.len()] == a[..] {
                out[i..i + a.len()].copy_from_slice(&b);
                i += a.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

fn patch_revision(bytes: &[u8], revision: f64) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let n = u16::from_be_bytes([out[4], out[5]]) as usize;
    for i in 0..n {
        let rec = 12 + i * 16;
        if &out[rec..rec + 4] == b"head" {
            let off = u32::from_be_bytes(out[rec + 8..rec + 12].try_into().unwrap()) as usize;
            out[off + 4..off + 8].copy_from_slice(&((revision * 65536.0).round() as u32).to_be_bytes());
            return out;
        }
    }
    panic!("không có bảng head");
}

fn state_with(root: &Path, files: &[(&str, Vec<u8>)]) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    for (rel, bytes) in files {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn all() -> FaceQueryDto {
    FaceQueryDto::default()
}

fn total(st: &AppState) -> u64 {
    library::query_faces(st, &FaceQueryDto { limit: 100, ..Default::default() }).unwrap().total
}

fn req(plan: &ResolvePlanDto, action: ResolveActionDto) -> ResolveRequestDto {
    ResolveRequestDto {
        items: plan
            .items
            .iter()
            .map(|i| ResolveItemDto {
                kind: i.kind,
                key: i.key.clone(),
                keeper_path: i.keeper.path.clone(),
                loser_paths: i.losers.iter().map(|l| l.path.clone()).collect(),
                member_paths: i.member_paths.clone(),
            })
            .collect(),
        action,
        explicit_trash_paths: vec![],
        mode: ResolveModeDto::Chosen,
    }
}

/// Thùng rác giả: chuyển vào thư mục riêng của test.
fn fake_trash(dir: &Path) -> impl Fn(&Path) -> Result<PathBuf, String> + '_ {
    move |p: &Path| {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let to = dir.join(format!("{}-{}", dir.read_dir().unwrap().count(), p.file_name().unwrap().to_string_lossy()));
        std::fs::rename(p, &to).map_err(|e| e.to_string())?;
        Ok(to)
    }
}

fn no_trash(_: &Path) -> Result<PathBuf, String> {
    Err("không được gọi".into())
}

#[test]
fn same_file_copies_hide_one_path_keep_the_other_and_never_touch_files() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let roboto = std::fs::read(fixture("roboto.ttf")).unwrap();
    let st = state_with(td.path(), &[("a/roboto.ttf", roboto.clone()), ("b/Roboto (1).ttf", roboto)]);

    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    assert_eq!(plan.items.len(), 1);
    let it = &plan.items[0];
    assert!(it.auto_safe);
    assert!(it.keeper.path.ends_with("a/roboto.ttf"), "giữ tên sạch: {}", it.keeper.path);
    assert_eq!(it.reason.code, "same_content_tidiest_path");
    assert_eq!((plan.keep_count, plan.hide_count), (1, 1));

    let r = dup_resolve::resolve_apply(&st, &req(&plan, ResolveActionDto::Hide), &no_trash, 1000).unwrap();
    assert!(r.failed.is_empty(), "{:?}", r.failed);
    assert_eq!(r.done.len(), 1);
    assert!(Path::new(&it.losers[0].path).is_file(), "ẩn không đụng tệp");
    assert_eq!(total(&st), 1, "font vẫn hiện một lần");
    assert!(duplicates::duplicate_groups(&st, &all()).unwrap().is_empty(), "nhóm biến mất");
    assert_eq!(duplicates::duplicate_summary(&st, &all()).unwrap().total, 0);

    let u = dup_resolve::resolve_undo(&st, &[r.done[0].id], 2000).unwrap();
    assert!(u.failed.is_empty() && !u.needs_rescan);
    assert_eq!(duplicates::duplicate_groups(&st, &all()).unwrap().len(), 1, "hoàn tác: nhóm trở lại");
}

#[test]
fn newer_version_is_kept_older_goes_to_trash_and_undo_puts_the_file_back() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let v_old = patch_revision(&base, 1.203);
    let v_new = patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.204);
    let st = state_with(td.path(), &[("old/playfair.ttf", v_old.clone()), ("new/playfair.ttf", v_new)]);

    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    let it = &plan.items[0];
    assert!(it.keeper.path.ends_with("new/playfair.ttf"));
    assert_eq!(it.reason.code, "newer_version");
    assert_eq!(it.reason.args["keep"], "Version 1.204");
    assert!(it.warnings.iter().any(|w| w.code == "may_reflow"));
    let loser = it.losers[0].path.clone();
    assert!(it.losers[0].trash.allowed);

    let trash_dir = td.path().join("trash");
    let r = dup_resolve::resolve_apply(&st, &req(&plan, ResolveActionDto::Trash), &fake_trash(&trash_dir), 1000).unwrap();
    assert!(r.failed.is_empty(), "{:?}", r.failed);
    assert!(!Path::new(&loser).exists(), "tệp cũ đã vào Thùng rác");
    let t = r.done[0].trash_path.clone().unwrap();
    assert!(Path::new(&t).exists());
    assert_eq!(r.done[0].action, ResolveActionDto::Trash);
    assert_eq!(total(&st), 1);

    let list = dup_resolve::resolve_list(&st, false).unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].can_undo);
    assert_eq!(list[0].family.as_deref(), Some("Playfair Display"));

    let u = dup_resolve::resolve_undo(&st, &[r.done[0].id], 2000).unwrap();
    assert!(u.failed.is_empty(), "{:?}", u.failed);
    assert!(u.needs_rescan);
    assert!(Path::new(&loser).is_file(), "tệp về đúng chỗ cũ");
    assert!(!Path::new(&t).exists());
    assert_eq!(std::fs::read(&loser).unwrap(), v_old, "nội dung nguyên vẹn");
    assert!(dup_resolve::resolve_list(&st, false).unwrap().is_empty());
}

#[test]
fn undo_fails_cleanly_when_the_trash_was_emptied_or_the_old_place_is_taken() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let v_new = patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.204);
    let st = state_with(td.path(), &[("old/playfair.ttf", patch_revision(&base, 1.203)), ("new/playfair.ttf", v_new)]);
    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    let loser = plan.items[0].losers[0].path.clone();
    let trash_dir = td.path().join("trash");
    let r = dup_resolve::resolve_apply(&st, &req(&plan, ResolveActionDto::Trash), &fake_trash(&trash_dir), 1000).unwrap();
    let id = r.done[0].id;

    std::fs::write(&loser, b"tep khac").unwrap();
    let u = dup_resolve::resolve_undo(&st, &[id], 2000).unwrap();
    assert_eq!(u.failed[0].code, "path_occupied");
    assert_eq!(std::fs::read(&loser).unwrap(), b"tep khac", "không ghi đè tệp khác");
    std::fs::remove_file(&loser).unwrap();

    std::fs::remove_file(r.done[0].trash_path.as_ref().unwrap()).unwrap();
    let u = dup_resolve::resolve_undo(&st, &[id], 2000).unwrap();
    assert_eq!(u.failed[0].code, "trash_emptied");
    assert_eq!(dup_resolve::resolve_list(&st, false).unwrap().len(), 1, "quyết định vẫn còn, không nửa vời");
}

#[test]
fn links_follow_the_keeper_when_an_older_version_is_hidden() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let v_new = patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.204);
    let st = state_with(td.path(), &[("old/playfair.ttf", patch_revision(&base, 1.203)), ("new/playfair.ttf", v_new)]);
    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    let (keep, lose) = (plan.items[0].keeper.face.clone(), plan.items[0].losers[0].face.clone());
    library::set_favorite(&st, &lose, true).unwrap();

    let r = dup_resolve::resolve_apply(&st, &req(&plan, ResolveActionDto::Hide), &no_trash, 1000).unwrap();
    assert_eq!(r.done[0].links_moved, 1);
    let fav = library::query_faces(&st, &FaceQueryDto { favorites_only: true, limit: 10, ..Default::default() }).unwrap();
    assert_eq!(fav.items.len(), 1, "yêu thích không biến mất khỏi thư viện");
    assert_eq!(fav.items[0].hash_hex, keep.hash_hex);
    dup_resolve::resolve_undo(&st, &[r.done[0].id], 2000).unwrap();
    let fav = library::query_faces(&st, &FaceQueryDto { favorites_only: true, limit: 10, ..Default::default() }).unwrap();
    assert_eq!(fav.items.len(), 1);
    assert_eq!(fav.items[0].hash_hex, lose.hash_hex, "hoàn tác trả yêu thích về bản cũ");
}

#[test]
fn apply_revalidates_and_refuses_unsafe_requests() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let v_new = patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.204);
    let st = state_with(td.path(), &[("old/playfair.ttf", patch_revision(&base, 1.203)), ("new/playfair.ttf", v_new)]);
    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    let it = &plan.items[0];
    let mk = |keeper: &str, losers: Vec<String>| ResolveRequestDto {
        items: vec![ResolveItemDto { kind: it.kind, key: it.key.clone(), keeper_path: keeper.into(), loser_paths: losers, member_paths: it.member_paths.clone() }],
        action: ResolveActionDto::Hide,
        explicit_trash_paths: vec![],
        mode: ResolveModeDto::Chosen,
    };
    // bản giữ không thuộc nhóm / bản thua trùng bản giữ / bản thua lạ
    let r = dup_resolve::resolve_apply(&st, &mk("/khong/co.ttf", vec![it.losers[0].path.clone()]), &no_trash, 1).unwrap();
    assert_eq!(r.failed[0].code, "keeper_not_in_group");
    let r = dup_resolve::resolve_apply(&st, &mk(&it.keeper.path, vec![it.keeper.path.clone()]), &no_trash, 1).unwrap();
    assert_eq!(r.failed[0].code, "not_allowed");
    let r = dup_resolve::resolve_apply(&st, &mk(&it.keeper.path, vec!["/la.ttf".into()]), &no_trash, 1).unwrap();
    assert_eq!(r.failed[0].code, "loser_not_in_group");
    // bản giữ mất khỏi đĩa → không ẩn gì
    std::fs::remove_file(&it.keeper.path).unwrap();
    let r = dup_resolve::resolve_apply(&st, &mk(&it.keeper.path, vec![it.losers[0].path.clone()]), &no_trash, 1).unwrap();
    assert_eq!(r.failed[0].code, "keeper_missing");
    assert!(r.done.is_empty());
    assert!(dup_resolve::resolve_list(&st, true).unwrap().is_empty());
}

#[test]
fn trash_is_refused_for_paths_outside_user_folders_but_hide_is_allowed() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let v_new = patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.204);
    let st = state_with(td.path(), &[("old/playfair.ttf", patch_revision(&base, 1.203)), ("new/playfair.ttf", v_new)]);
    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    // đổi nguồn thành "system": chỉ ẩn được, không Thùng rác
    st.with_user(|s| {
        s.conn().execute("UPDATE u.sources SET kind='system'", []).unwrap();
        Ok(())
    })
    .unwrap();
    let r = dup_resolve::resolve_apply(&st, &req(&plan, ResolveActionDto::Trash), &no_trash, 1).unwrap();
    assert_eq!(r.failed[0].code, "not_allowed");
    assert!(Path::new(&plan.items[0].losers[0].path).is_file());
    let r = dup_resolve::resolve_apply(&st, &req(&plan, ResolveActionDto::Hide), &no_trash, 2).unwrap();
    assert!(r.failed.is_empty() && r.done.len() == 1);
}

#[test]
fn ignoring_a_group_hides_it_everywhere_and_unignoring_brings_it_back() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let v_new = patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.204);
    let st = state_with(td.path(), &[("old/playfair.ttf", patch_revision(&base, 1.203)), ("new/playfair.ttf", v_new)]);
    let g = app_lib::services::duplicates::duplicate_groups(&st, &all()).unwrap();
    assert_eq!(g.len(), 1);
    let (kind, key) = (g[0].kind, g[0].key.clone());

    app_lib::services::duplicates::duplicate_ignore(&st, kind, &key, 5).unwrap();
    assert!(app_lib::services::duplicates::duplicate_groups(&st, &all()).unwrap().is_empty());
    assert_eq!(app_lib::services::duplicates::duplicate_summary(&st, &all()).unwrap().total, 0);
    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::All, &all()).unwrap();
    assert!(plan.items.is_empty() && plan.needs_review == 0, "nhóm đã bỏ qua không vào 'giải quyết tất cả'");
    let ig = app_lib::services::duplicates::duplicate_ignored(&st).unwrap();
    assert_eq!(ig.len(), 1);
    assert_eq!((ig[0].group.key.as_str(), ig[0].ignored_at), (key.as_str(), 5));
    assert_eq!(ig[0].group.faces.len(), 2);

    app_lib::services::duplicates::duplicate_unignore(&st, kind, &key).unwrap();
    assert_eq!(app_lib::services::duplicates::duplicate_groups(&st, &all()).unwrap().len(), 1);
    assert!(app_lib::services::duplicates::duplicate_ignored(&st).unwrap().is_empty());
}

#[test]
fn source_priority_decides_which_copy_is_kept() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let roboto = std::fs::read(fixture("roboto.ttf")).unwrap();
    let st = state_with(td.path(), &[("a/roboto.ttf", roboto.clone())]);
    // nguồn thứ hai có bản sao cùng nội dung
    let dir2 = td.path().join("Second");
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir2.join("roboto.ttf"), roboto).unwrap();
    sources::add_source(&st, dir2.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));

    let order = dup_resolve::source_priority(&st).unwrap();
    assert_eq!(order.len(), 2);
    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    assert_eq!(plan.items[0].reason.code, "preferred_source");
    assert_eq!(plan.items[0].keeper.source_id, order[0], "mặc định: nguồn thêm trước");

    let flipped = dup_resolve::set_source_priority(&st, &[order[1]]).unwrap();
    assert_eq!(flipped, vec![order[1], order[0]]);
    let plan = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    assert_eq!(plan.items[0].keeper.source_id, order[1], "đổi thứ tự → đổi bản giữ");
    assert!(dup_resolve::set_source_priority(&st, &[9999]).is_err());
}

#[test]
fn groups_that_need_a_human_are_not_in_all_safe_but_are_counted() {
    let _g = serial();
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    // cùng revision và cùng ngày head, chỉ khác chuỗi version → không phân biệt được bản mới hơn → cần người xem
    let st = state_with(td.path(), &[("a/p.ttf", patch_revision(&base, 1.2)), ("b/p.ttf", patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.2))]);
    let safe = dup_resolve::resolve_preview(&st, &ResolveScopeDto::AllSafe, &all()).unwrap();
    let every = dup_resolve::resolve_preview(&st, &ResolveScopeDto::All, &all()).unwrap();
    assert_eq!(safe.needs_review, 1);
    assert!(safe.items.is_empty(), "không tự quyết khi không biết bản nào mới hơn");
    assert_eq!(every.items.len(), 1);
    let item = &every.items[0];
    assert!(!item.auto_safe && item.review.is_some());
    let one = dup_resolve::resolve_preview(&st, &ResolveScopeDto::Group { kind: item.kind, key: item.key.clone() }, &all()).unwrap();
    assert_eq!(one.items.len(), 1, "xem trước từng nhóm vẫn có quyết định mặc định để người dùng đổi");
}
