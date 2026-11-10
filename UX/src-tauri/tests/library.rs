use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::library::*;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, library, sources, system};
use app_lib::state::{AppDirs, AppState};

fn core_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn booted(dir: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&dir.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    st
}

/// Quét các fixture đã cho vào một nguồn; trả (state, thư mục).
fn scanned(root: &Path, names: &[&str]) -> (Arc<AppState>, PathBuf) {
    let st = booted(root);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in names {
        std::fs::copy(core_fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    (st, dir)
}

fn all(st: &AppState) -> FacePageDto {
    library::query_faces(st, &FaceQueryDto { limit: 100, ..Default::default() }).unwrap()
}

fn key_of(r: &FaceRowDto) -> FaceKeyDto {
    FaceKeyDto { hash_hex: r.hash_hex.clone(), face_index: r.face_index }
}

#[test]
fn face_row_json_matches_store_fixture() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = scanned(d.path(), &["roboto.ttf"]);
    let page = all(&st);
    let mut got = serde_json::to_value(&page.items[0]).unwrap();
    // `langBadge` do app thêm (Hyperglot); 20 khoá còn lại phải đúng bằng FaceRow của store
    assert!(got.as_object_mut().unwrap().remove("langBadge").is_some());
    let want: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-store/tests/fixtures/face_row.json")).unwrap(),
    )
    .unwrap();
    let keys = |v: &serde_json::Value| {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    assert_eq!(keys(&got), keys(&want));
    assert_eq!(keys(&got).len(), 20);
}

#[test]
fn favorites_collections_tags_roundtrip() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = scanned(d.path(), &["roboto.ttf", "open_sans.ttf", "playfair_display.ttf"]);
    let rows = all(&st).items;
    let (a, b) = (key_of(&rows[0]), key_of(&rows[1]));
    library::set_favorite(&st, &a, true).unwrap();
    let favs = library::query_faces(&st, &FaceQueryDto { favorites_only: true, ..Default::default() }).unwrap();
    assert_eq!(favs.total, 1);
    assert!(favs.items[0].is_favorite);
    library::set_favorite(&st, &a, false).unwrap();
    assert_eq!(library::query_faces(&st, &FaceQueryDto { favorites_only: true, ..Default::default() }).unwrap().total, 0);

    let c = library::create_collection(&st, " Menu ", Some("#f00")).unwrap();
    assert_eq!((c.name.as_str(), c.item_count), ("Menu", 0));
    library::add_to_collection(&st, c.id, &[a.clone(), b.clone()]).unwrap();
    assert_eq!(library::list_collections(&st).unwrap()[0].item_count, 2);
    let in_c = library::query_faces(&st, &FaceQueryDto { collection_id: Some(c.id), ..Default::default() }).unwrap();
    assert_eq!(in_c.total, 2);
    library::remove_from_collection(&st, c.id, std::slice::from_ref(&a)).unwrap();
    library::rename_collection(&st, c.id, "Poster").unwrap();
    assert_eq!(library::list_collections(&st).unwrap()[0].name, "Poster");
    assert_eq!(library::create_collection(&st, "  ", None).unwrap_err().code, "invalid_arg");

    let t = library::create_tag(&st, "serif").unwrap();
    library::tag_faces(&st, t.id, std::slice::from_ref(&a)).unwrap();
    assert_eq!(library::tags_of(&st, &a).unwrap()[0].name, "serif");
    assert_eq!(library::list_tags(&st).unwrap()[0].face_count, 1);
    library::untag_faces(&st, t.id, &[a]).unwrap();
    assert_eq!(library::list_tags(&st).unwrap()[0].face_count, 0);
    library::delete_collection(&st, c.id).unwrap();
    assert!(library::list_collections(&st).unwrap().is_empty());
}

#[test]
fn bad_hash_is_invalid_arg() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    let bad = FaceKeyDto { hash_hex: "xyz".into(), face_index: 0 };
    assert_eq!(library::set_favorite(&st, &bad, true).unwrap_err().code, "invalid_arg");
    assert_eq!(library::tags_of(&st, &bad).unwrap_err().code, "invalid_arg");
}

#[test]
fn retry_problems_requeues() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    std::fs::write(dir.join("bad.ttf"), b"khong phai font").unwrap();
    sources::rescan_sources(&st, None).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let p = library::list_problems(&st).unwrap();
    assert_eq!(p.len(), 1);
    assert!(p[0].path.as_deref().unwrap().ends_with("bad.ttf"));
    let log = Arc::new(std::sync::Mutex::new(0usize));
    let l2 = log.clone();
    st.hubs.scan.subscribe(Box::new(move |e| {
        if matches!(e, app_lib::dto::events::ScanEvent::Finished { .. }) {
            *l2.lock().unwrap() += 1;
        }
    }));
    assert_eq!(library::retry_problems(&st).unwrap(), 1);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(*log.lock().unwrap(), 1, "retry phải chạy một lượt quét");
    assert_eq!(library::retry_problems(&st).unwrap(), 1); // vẫn lỗi → vẫn còn một mục để thử lại
}

#[test]
fn family_styles_static_and_variable() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = scanned(d.path(), &["roboto.ttf", "public_sans_vf.ttf"]);
    let roboto = library::get_family_styles(&st, "Roboto").unwrap();
    assert_eq!(roboto.len(), 1);
    assert!(roboto[0].instance.is_none());
    let ps = library::get_family_styles(&st, "Public Sans").unwrap();
    assert!(ps.len() >= 9, "{}", ps.len());
    assert!(ps.iter().any(|s| s.weight == Some(700)));
    assert!(ps.iter().all(|s| s.instance.is_some()));
    assert!(library::get_family_styles(&st, "Khong Co").unwrap().is_empty());
}

#[test]
fn face_keys_filter_keeps_given_order() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = scanned(d.path(), &["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf"]);
    let mut keys: Vec<FaceKeyDto> = all(&st).items.iter().map(key_of).collect();
    keys.reverse();
    keys.push(FaceKeyDto { hash_hex: "ab".repeat(32), face_index: 0 }); // không tồn tại → bị bỏ
    let q = FaceQueryDto { sort: SortKeyDto::Match, face_keys: Some(keys.clone()), ..Default::default() };
    let got: Vec<FaceKeyDto> = library::query_faces(&st, &q).unwrap().items.iter().map(key_of).collect();
    assert_eq!(got, keys[..5]);
    let empty = FaceQueryDto { face_keys: Some(vec![]), ..Default::default() };
    assert_eq!(library::query_faces(&st, &empty).unwrap().total, 0);
    let too_many = FaceQueryDto { face_keys: Some(vec![keys[0].clone(); library::MAX_FACE_KEYS + 1]), ..Default::default() };
    assert_eq!(library::query_faces(&st, &too_many).unwrap_err().code, "invalid_arg");
}

#[test]
fn ui_state_roundtrip_and_size_cap() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    assert_eq!(system::get_ui_state(&st).unwrap(), UiStateDto::default());
    let ui = UiStateDto { sidebar_collapsed: true, sample_size: 48, ..Default::default() };
    system::set_ui_state(&st, &ui).unwrap();
    assert_eq!(system::get_ui_state(&st).unwrap(), ui);
    let big = UiStateDto { layout: "x".repeat(system::MAX_SETTING_BYTES), ..Default::default() };
    assert_eq!(system::set_ui_state(&st, &big).unwrap_err().code, "invalid_arg");
    assert_eq!(system::get_ui_state(&st).unwrap(), ui);
}

#[test]
fn get_set_setting_only_ui_keys_and_64k_cap() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    system::set_setting(&st, "ui.recent_views", "[1,2]").unwrap();
    assert_eq!(system::get_setting(&st, "ui.recent_views").unwrap().as_deref(), Some("[1,2]"));
    assert_eq!(system::get_setting(&st, "ui.chua_co").unwrap(), None);
    for bad in ["embed_enabled", "ui.", "ui.HOA", "ui.a b", &format!("ui.{}", "a".repeat(61))] {
        assert_eq!(system::get_setting(&st, bad).unwrap_err().code, "invalid_arg", "{bad}");
        assert_eq!(system::set_setting(&st, bad, "x").unwrap_err().code, "invalid_arg", "{bad}");
    }
    assert_eq!(system::set_setting(&st, "ui.big", &"x".repeat(system::MAX_SETTING_BYTES + 1)).unwrap_err().code, "invalid_arg");
    system::set_setting(&st, "ui.big", &"x".repeat(system::MAX_SETTING_BYTES)).unwrap();
}

#[test]
fn detect_font_managers_lists_known_names() {
    let m = system::detect_font_managers();
    let names: Vec<&str> = m.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["RightFont", "FontBase", "Typeface", "Suitcase Fusion", "FontExplorer X"]);
    assert!(m.iter().all(|x| x.active_count.is_none()));
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("FontBase.app")).unwrap();
    let m = system::detect_font_managers_in(&[d.path().to_path_buf()]);
    assert_eq!(m.iter().filter(|x| x.installed).map(|x| x.name.as_str()).collect::<Vec<_>>(), ["FontBase"]);
}

#[test]
fn reveal_missing_path_is_not_found() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    let face = key_of(&all(&st).items[0]);
    let seen = std::sync::Mutex::new(None);
    system::reveal_in_finder(&st, &face, &|p| {
        *seen.lock().unwrap() = Some(p.to_path_buf());
        Ok(())
    })
    .unwrap();
    assert!(seen.lock().unwrap().as_ref().unwrap().starts_with(&dir));
    let unknown = FaceKeyDto { hash_hex: "cd".repeat(32), face_index: 0 };
    assert_eq!(system::reveal_in_finder(&st, &unknown, &|_| Ok(())).unwrap_err().code, "not_found");
}

#[test]
fn backup_and_disk_free() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    library::create_collection(&st, "A", None).unwrap();
    let p = system::backup_user_db(&st).unwrap();
    assert!(Path::new(&p).exists());
    assert!(system::disk_free(&st, "models").unwrap() > 0 && system::disk_free(&st, "data").unwrap() > 0);
    assert_eq!(system::disk_free(&st, "../etc").unwrap_err().code, "invalid_arg");
}
