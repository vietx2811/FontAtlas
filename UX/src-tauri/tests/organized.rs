//! B7 Organized Library: vị trí, chép an toàn (file gốc không đụng tới), xem trước, sắp xếp lại + hoàn tác, gói khôi phục.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::analysis::ClassificationOverrideDto;
use app_lib::dto::library::*;
use app_lib::dto::organized::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{analysis, boot, library, organized, sources};
use app_lib::state::{AppDirs, AppState};

fn core_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn booted(dir: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&dir.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    st
}

/// Nguồn "Fonts" có các fixture đã cho, đã quét xong và Index đã dựng.
fn scanned(root: &Path, names: &[&str]) -> (Arc<AppState>, PathBuf) {
    let st = booted(root);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in names {
        std::fs::copy(core_fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    (st, dir)
}

fn all(st: &AppState) -> FacePageDto {
    library::query_faces(st, &FaceQueryDto { limit: 100, ..Default::default() }).unwrap()
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p) } else { out.push(p) }
        }
    }
    out.sort();
    out
}

fn make_library(st: &Arc<AppState>, root: &Path) -> PathBuf {
    let lib_root = root.join("Organized");
    let s = organized::create(st, lib_root.to_str().unwrap()).unwrap();
    assert!(s.configured && s.source_id.is_some());
    lib_root
}

#[test]
fn location_check_reports_free_space_nesting_and_create_registers_the_library_as_a_source_once() {
    let d = tempfile::tempdir().unwrap();
    let (st, fonts) = scanned(d.path(), &["roboto.ttf"]);
    let ok = organized::check_location(&st, d.path().join("Organized").to_str().unwrap()).unwrap();
    assert!((ok.problem.clone(), ok.writable, ok.is_cloud, ok.nested_source) == (None, true, false, false) && ok.free_bytes > 0, "{ok:?}");
    let nested = organized::check_location(&st, fonts.join("inside").to_str().unwrap()).unwrap();
    assert_eq!(nested.problem.as_deref(), Some("nested_source"));
    assert_eq!(organized::create(&st, fonts.join("inside").to_str().unwrap()).unwrap_err().code, "nested_source");
    assert!(!organized::status(&st).unwrap().configured, "tạo thất bại không để lại gì");

    let root = make_library(&st, d.path());
    assert!(root.join("Library/.fontatlas-library").exists() && root.join("Snapshots").is_dir());
    assert_eq!(sources::list_sources(&st).unwrap().len(), 2, "Library/ là một nguồn thư mục");
    assert_eq!(organized::create(&st, root.to_str().unwrap()).unwrap_err().code, "already_configured");
}

#[test]
fn import_copies_without_touching_originals_keeps_names_verifies_and_snapshots() {
    let d = tempfile::tempdir().unwrap();
    let (st, fonts) = scanned(d.path(), &["roboto.ttf", "open_sans.ttf"]);
    let before: Vec<Vec<u8>> = ["roboto.ttf", "open_sans.ttf"].iter().map(|n| std::fs::read(fonts.join(n)).unwrap()).collect();
    let root = make_library(&st, d.path());
    let plan = organized::import_plan(&st, &OrganizeScopeDto::default()).unwrap();
    assert_eq!((plan.to_copy, plan.already_there, plan.cloud_not_downloaded, plan.enough_space), (2, 0, 0, true));
    assert!(plan.bytes > 0 && plan.sample.len() == 2);
    assert!(walk(&root.join("Library")).iter().all(|p| p.file_name().unwrap() == ".fontatlas-library"), "xem trước không ghi gì");

    let mut last = None;
    let r = organized::import(&st, &OrganizeScopeDto::default(), &mut |p| last = Some(p)).unwrap();
    assert_eq!((r.copied, r.issue_count, r.cancelled), (2, 0, false));
    assert_eq!(last.unwrap().done, 2);
    let files: Vec<String> = walk(&root.join("Library"))
        .iter()
        .filter(|p| p.file_name().unwrap() != ".fontatlas-library")
        .map(|p| p.strip_prefix(root.join("Library")).unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.ends_with("/roboto.ttf")) && files.iter().any(|f| f.ends_with("/open_sans.ttf")), "giữ tên file gốc: {files:?}");
    assert!(files.iter().all(|f| f.split('/').count() == 3), "Class/Họ/file: {files:?}");
    // file gốc nguyên vẹn
    for (i, n) in ["roboto.ttf", "open_sans.ttf"].iter().enumerate() {
        assert_eq!(std::fs::read(fonts.join(n)).unwrap(), before[i]);
    }
    assert!(!walk(&root.join("Library")).iter().any(|p| p.to_string_lossy().ends_with(".partial")));
    // gói khôi phục ngay sau lần chép
    let snap = r.snapshot.expect("snapshot sau mỗi lần chép");
    assert!(organized::pack_inspect(&snap).unwrap().files >= 2);
    assert_eq!(organized::status(&st).unwrap().snapshots, 1);

    // quét thư viện mới ⇒ cùng hash, không thêm font; chép lần hai không còn gì
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(all(&st).total, 2);
    let plan2 = organized::import_plan(&st, &OrganizeScopeDto::default()).unwrap();
    assert_eq!((plan2.to_copy, plan2.already_there), (0, 2));
    let s = organized::status(&st).unwrap();
    assert_eq!(s.files, 2);
}

#[test]
fn same_name_different_content_gets_a_hash_suffix_instead_of_overwriting() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let (a, b) = (d.path().join("A"), d.path().join("B"));
    for p in [&a, &b] {
        std::fs::create_dir_all(p).unwrap();
        std::fs::copy(core_fixture("roboto.ttf"), p.join("Roboto.ttf")).unwrap();
    }
    // bản B khác nội dung (thêm byte cuối) nhưng cùng họ và cùng tên file
    let mut bytes = std::fs::read(b.join("Roboto.ttf")).unwrap();
    bytes.push(0);
    std::fs::write(b.join("Roboto.ttf"), bytes).unwrap();
    for p in [&a, &b] {
        sources::add_source(&st, p.to_str().unwrap()).unwrap();
    }
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    let root = make_library(&st, d.path());
    let r = organized::import(&st, &OrganizeScopeDto::default(), &mut |_| {}).unwrap();
    assert_eq!(r.copied, 2);
    let names: Vec<String> = walk(&root.join("Library"))
        .iter()
        .filter(|p| p.file_name().unwrap() != ".fontatlas-library")
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Roboto.ttf".to_string()) && names.iter().any(|n| n.starts_with("Roboto-") && n.ends_with(".ttf")), "{names:?}");
}

#[test]
fn resort_only_proposes_never_auto_moves_and_undo_restores_everything() {
    let d = tempfile::tempdir().unwrap();
    let (st, _fonts) = scanned(d.path(), &["roboto.ttf", "open_sans.ttf"]);
    let root = make_library(&st, d.path());
    organized::import(&st, &OrganizeScopeDto::default(), &mut |_| {}).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(organized::resort_preview(&st).unwrap().total, 0, "mới xếp xong: không có gì để sắp xếp lại");

    // người dùng đổi class của một font ⇒ class đổi sau khi đã xếp: chỉ ĐỀ XUẤT, thư viện chưa đổi
    let rows = all(&st).items;
    let key = FaceKeyDto { hash_hex: rows[0].hash_hex.clone(), face_index: rows[0].face_index };
    let target_class = if rows[0].family.contains("Roboto") { "mono" } else { "script" };
    analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key.clone(), class: target_class.into(), subclass: None }).unwrap();
    let before = walk(&root.join("Library"));
    let plan = organized::resort_preview(&st).unwrap();
    assert_eq!(plan.total, 1);
    assert_eq!(plan.moves[0].face, key);
    assert_eq!(walk(&root.join("Library")), before, "xem trước không di chuyển gì");

    let done = organized::resort_apply(&st).unwrap();
    assert_eq!((done.moved, done.failed), (1, 0));
    let after = walk(&root.join("Library"));
    assert_ne!(after, before);
    assert!(after.iter().any(|p| p.to_string_lossy().contains(&format!("/{}/", if target_class == "mono" { "Mono" } else { "Script" }))));
    assert_eq!(organized::resort_preview(&st).unwrap().total, 0);

    let undone = organized::resort_undo(&st, done.batch_id).unwrap();
    assert_eq!((undone.moved, undone.failed), (1, 0));
    assert_eq!(walk(&root.join("Library")), before, "hoàn tác trả mọi file về chỗ cũ");
}

#[test]
fn restore_is_staged_then_applied_before_the_database_opens_and_the_old_one_is_kept() {
    let d = tempfile::tempdir().unwrap();
    let (st, fonts) = scanned(d.path(), &["roboto.ttf"]);
    let root = make_library(&st, d.path());
    organized::import(&st, &OrganizeScopeDto::default(), &mut |_| {}).unwrap();
    let pack = organized::snapshot_now(&st).unwrap();
    drop(st);

    // "máy mới": app mở lần đầu tạo DB rỗng, người dùng chọn gói, app xếp khôi phục
    let d2 = tempfile::tempdir().unwrap();
    let st2 = booted(d2.path());
    assert!(sources::list_sources(&st2).unwrap().is_empty());
    let info = organized::pack_restore_stage(&st2, &pack, &[]).unwrap();
    assert_eq!(info.sources, 2);
    assert!(organized::pack_restore_pending(&st2).is_some());
    drop(st2);

    let st3 = booted(d2.path());
    let list = sources::list_sources(&st3).unwrap();
    assert_eq!(list.len(), 2, "nguồn Fonts + Organized Library đã trở lại");
    assert!(list.iter().any(|s| s.path.as_deref() == Some(&*fonts.to_string_lossy())));
    assert!(organized::pack_restore_pending(&st3).is_none());
    assert!(std::fs::read_dir(d2.path().join("data")).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("pre-restore-")), "DB cũ được dời, không xoá");
    let _ = root;

    // gói hỏng bị từ chối ngay lúc xếp, không để lại lệnh chờ
    let bad = d.path().join("bad");
    std::fs::create_dir_all(&bad).unwrap();
    assert_eq!(organized::pack_restore_stage(&st3, bad.to_str().unwrap(), &[]).unwrap_err().code, "invalid_pack");
    assert!(organized::pack_restore_pending(&st3).is_none());
}

#[test]
fn safe_component_strips_path_hazards_and_class_folders_are_stable() {
    assert_eq!(organized::safe_component("AC/DC: Bold?", "x"), "AC_DC_ Bold_");
    assert_eq!(organized::safe_component("  ..hidden.. ", "x"), "hidden");
    assert_eq!(organized::safe_component("...", "Unnamed"), "Unnamed");
    assert_eq!(organized::safe_component(&"a".repeat(200), "x").chars().count(), 80);
    assert_eq!(organized::class_folder("sans"), "Sans");
    assert_eq!(organized::class_folder("uncertain"), "Unclassified");
    assert_eq!(organized::class_folder("whatever"), "Unclassified");
}

#[test]
fn organized_copies_do_not_show_up_as_duplicates_of_their_originals() {
    use app_lib::services::duplicates;
    let d = tempfile::tempdir().unwrap();
    let (st, _fonts) = scanned(d.path(), &["roboto.ttf", "open_sans.ttf"]);
    let q = FaceQueryDto { limit: 100, ..Default::default() };
    assert!(duplicates::duplicate_groups(&st, &q).unwrap().is_empty());
    make_library(&st, d.path());
    organized::import(&st, &OrganizeScopeDto::default(), &mut |_| {}).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(app_lib::index_bundle::wait_rebuilt(&st, Duration::from_secs(60)));
    let groups = duplicates::duplicate_groups(&st, &q).unwrap();
    assert!(groups.is_empty(), "bản chép trong Organized Library không phải 'trùng lặp': {groups:?}");
}

#[test]
fn a_cloud_location_is_warned_about_but_not_refused() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let p = d.path().join("Library/CloudStorage/Dropbox/Fonts-Organized");
    let chk = organized::check_location(&st, p.to_str().unwrap()).unwrap();
    assert_eq!((chk.is_cloud, chk.cloud_provider.as_deref(), chk.problem.clone()), (true, Some("dropbox"), None));
    assert!(!p.exists(), "kiểm tra không để lại thư mục");
    let s = organized::create(&st, p.to_str().unwrap()).unwrap();
    assert!(s.configured && s.is_cloud, "người dùng vẫn chọn được; UI chịu trách nhiệm cảnh báo");
}
