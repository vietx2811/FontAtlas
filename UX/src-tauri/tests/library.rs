use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::analysis::ClassificationOverrideDto;
use app_lib::dto::library::*;
use app_lib::services::activation::NoActivation;
use app_lib::services::{analysis, boot, cases, cloud_download, library, sources, system};
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
    // `langBadge` do app thêm (Hyperglot); 24 khoá còn lại phải đúng bằng FaceRow của store
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
    assert_eq!(keys(&got).len(), 24);
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
    // tag hệ thống (style:/mood:/use:): chuẩn hoá, từ chối giá trị ngoài danh mục; namespace lạ là tag người dùng thường
    let sys = library::create_tag(&st, " Mood: CALM ").unwrap();
    assert_eq!(sys.name, "mood:calm");
    assert_eq!(library::create_tag(&st, "mood:calm").unwrap().id, sys.id, "idempotent");
    assert_eq!(library::create_tag(&st, "style:nope").unwrap_err().code, "invalid_arg");
    assert_eq!(library::create_tag(&st, "tet:2027").unwrap().name, "tet:2027");
    let cat = library::system_tag_catalog();
    assert_eq!(cat.iter().map(|g| g.namespace.as_str()).collect::<Vec<_>>(), ["style", "mood", "use"]);
    assert!(cat.iter().flat_map(|g| &g.tags).any(|t| t.name == "style:art_deco" && t.label == "Art Deco"));
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
fn ui_state_old_json_without_closed_sections_keeps_new_groups_open() {
    let d = tempfile::tempdir().unwrap();
    let st = booted(d.path());
    // Dữ liệu lưu trước khi có nhóm Case: không có `closedSections`.
    system::set_setting(&st, "ui.state", r#"{"sidebarCollapsed":true,"openSections":["col","tag"],"layout":"list","sampleSize":40,"infoExpanded":true}"#).unwrap();
    let ui = system::get_ui_state(&st).unwrap();
    assert!(ui.closed_sections.is_empty());
    assert_eq!(ui.open_sections, vec!["col", "tag"]);
    assert!(ui.sidebar_collapsed && ui.info_expanded && ui.layout == "list");
    let closed = UiStateDto { closed_sections: vec!["case".into()], ..Default::default() };
    system::set_ui_state(&st, &closed).unwrap();
    assert_eq!(system::get_ui_state(&st).unwrap(), closed);
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
fn reveal_source_opens_the_source_folder_from_the_store_and_refuses_unknown_or_unmounted() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    let id = sources::list_sources(&st).unwrap()[0].id;
    let seen = std::sync::Mutex::new(Vec::new());
    let open = |p: &Path| {
        seen.lock().unwrap().push(p.to_path_buf());
        Ok(())
    };
    system::reveal_source(&st, id, &open).unwrap();
    assert_eq!(*seen.lock().unwrap(), vec![dir.clone()], "mở đúng thư mục nguồn lưu trong Store");
    // id lạ → not_found, không mở gì
    assert_eq!(system::reveal_source(&st, id + 99, &open).unwrap_err().code, "not_found");
    // thư mục không còn (ổ cloud/ổ rời chưa mount) → mã riêng, không mở gì, kèm sourceId để UI nói rõ
    let gone = d.path().join("Fonts-gone");
    std::fs::rename(&dir, &gone).unwrap();
    let e = system::reveal_source(&st, id, &open).unwrap_err();
    assert_eq!(e.code, "source_unavailable");
    assert_eq!(e.detail.as_ref().unwrap()["sourceId"], id);
    assert_eq!(seen.lock().unwrap().len(), 1, "không mở Finder khi thư mục không còn");
    // lỗi của opener được gói thành `io`
    std::fs::rename(&gone, &dir).unwrap();
    assert_eq!(system::reveal_source(&st, id, &|_| Err("no finder".into())).unwrap_err().code, "io");
}

/// Giả lập "Tải về máy" không cần ổ cloud thật: stub iCloud kiểu cũ `.Tên.ttf.icloud` trong thư mục tạm được
/// `cloud_placeholders` coi là chưa tải; hàm tải giả chép fixture OFL vào tên thật rồi xoá stub (như hệ điều hành làm).
fn fake_os_download(p: &Path, _permit: app_lib::services::cloud::Permit, _cancel: &std::sync::atomic::AtomicBool) -> Result<(), fontatlas_activate::EnsureLocalError> {
    let name = p.file_name().unwrap().to_str().unwrap();
    std::fs::copy(core_fixture(name), p).map_err(|e| fontatlas_activate::EnsureLocalError::Io(e.to_string()))?;
    let _ = std::fs::remove_file(p.with_file_name(format!(".{name}.icloud")));
    Ok(())
}

#[test]
fn download_source_materializes_placeholders_then_rescans_so_the_fonts_appear() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    let id = sources::list_sources(&st).unwrap()[0].id;
    for n in ["open_sans.ttf", "libre_baskerville.ttf"] {
        std::fs::write(dir.join(format!(".{n}.icloud")), b"").unwrap();
    }
    let before = all(&st).total;
    let mut seen = Vec::new();
    let r = cloud_download::download_source_with(&st, id, &fake_os_download, &mut |p| seen.push(p)).unwrap();
    assert_eq!((r.total, r.downloaded, r.failed, r.cancelled, r.rescan_queued), (2, 2, 0, false, true));
    assert_eq!(seen.first().map(|p| (p.done, p.total)), Some((0, 2)), "báo tổng ngay từ đầu");
    assert_eq!(seen.last().map(|p| p.done), Some(2));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(all(&st).total > before, "quét lại xong thì font mới tải hiện ra");
    assert!(!st.cloud_jobs.is_running(id), "xong thì nhả đăng ký");
    // không còn gì để tải: báo 0, không quét lại vô ích
    let again = cloud_download::download_source_with(&st, id, &fake_os_download, &mut |_| {}).unwrap();
    assert_eq!((again.total, again.rescan_queued), (0, false));
}

#[test]
fn download_source_failures_are_reported_by_code_and_do_not_hide_the_other_files() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    let id = sources::list_sources(&st).unwrap()[0].id;
    std::fs::write(dir.join(".open_sans.ttf.icloud"), b"").unwrap();
    std::fs::write(dir.join(".libre_baskerville.ttf.icloud"), b"").unwrap();
    let fetch = |p: &Path, permit: app_lib::services::cloud::Permit, c: &std::sync::atomic::AtomicBool| {
        if p.ends_with("open_sans.ttf") {
            return Err(fontatlas_activate::EnsureLocalError::Timeout);
        }
        fake_os_download(p, permit, c)
    };
    let r = cloud_download::download_source_with(&st, id, &fetch, &mut |_| {}).unwrap();
    assert_eq!((r.downloaded, r.failed), (1, 1));
    assert_eq!(r.first_failure_code.as_deref(), Some("cloud_timeout"));
    assert!(r.rescan_queued, "có font về máy là quét lại");
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
}

#[test]
fn download_source_refuses_unknown_unmounted_and_already_running_sources() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    let id = sources::list_sources(&st).unwrap()[0].id;
    let never = |_: &Path, _p: app_lib::services::cloud::Permit, _c: &std::sync::atomic::AtomicBool| -> Result<(), fontatlas_activate::EnsureLocalError> { panic!("không được tải") };
    assert_eq!(cloud_download::download_source_with(&st, id + 99, &never, &mut |_| {}).unwrap_err().code, "not_found");
    assert!(!st.cloud_jobs.is_running(id + 99), "lỗi sớm vẫn nhả đăng ký");
    // đang chạy → busy; huỷ được lần đang chạy
    std::fs::write(dir.join(".open_sans.ttf.icloud"), b"").unwrap();
    let st2 = Arc::clone(&st);
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let go_rx = std::sync::Mutex::new(go_rx);
    let h = std::thread::spawn(move || {
        let fetch = |p: &Path, permit: app_lib::services::cloud::Permit, c: &std::sync::atomic::AtomicBool| {
            started_tx.send(()).unwrap();
            go_rx.lock().unwrap().recv().unwrap();
            fake_os_download(p, permit, c)
        };
        cloud_download::download_source_with(&st2, id, &fetch, &mut |_| {})
    });
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(cloud_download::download_source_with(&st, id, &never, &mut |_| {}).unwrap_err().code, "busy");
    assert!(cloud_download::cancel_download(&st, id));
    go_tx.send(()).unwrap();
    let r = h.join().unwrap().unwrap();
    assert_eq!(r.downloaded, 1, "file đang tải chạy nốt");
    assert!(!cloud_download::cancel_download(&st, id), "hết chạy thì không còn gì để huỷ");
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    // thư mục không còn → source_unavailable
    std::fs::rename(&dir, d.path().join("Fonts-gone")).unwrap();
    let e = cloud_download::download_source_with(&st, id, &never, &mut |_| {}).unwrap_err();
    assert_eq!(e.code, "source_unavailable");
}

/// Tải xong khi nguồn ĐANG được quét: lượt quét hiện tại có thể đã đi qua chỗ vừa đổi, nên phải xếp thêm một lượt SAU nó,
/// và báo cáo chỉ nói `rescanQueued` khi thật sự đã xếp.
#[test]
fn download_during_a_scan_queues_a_follow_up_scan_and_reports_it_truthfully() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    let id = sources::list_sources(&st).unwrap()[0].id;
    std::fs::write(dir.join(".open_sans.ttf.icloud"), b"").unwrap();
    // giữ lượt quét hiện tại lại (tạm dừng) để chắc chắn nguồn đang là `current` lúc tải xong
    st.scans.pause(&st);
    st.scans.enqueue(&st, &[id]);
    let t = std::time::Instant::now();
    while !st.scans.is_current(id) && t.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(st.scans.is_current(id), "nguồn đang được quét");
    assert!(!st.scans.is_queued(id));
    let r = cloud_download::download_source_with(&st, id, &fake_os_download, &mut |_| {}).unwrap();
    assert_eq!((r.downloaded, r.rescan_queued), (1, true));
    assert!(st.scans.is_queued(id), "có lượt quét thứ hai chờ sau lượt hiện tại");
    // gọi thêm lần nữa không xếp trùng
    assert!(st.scans.enqueue_follow_up(&st, &[id]));
    st.scans.resume(&st);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(!st.scans.is_queued(id));
    let names: Vec<_> = all(&st).items.iter().map(|r| r.family.clone()).collect();
    assert!(names.iter().any(|f| f.contains("Open Sans")), "font vừa tải hiện ra sau lượt quét thứ hai: {names:?}");
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

#[test]
fn family_groups_follows_query_faces_order_and_filters() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = scanned(d.path(), &["roboto.ttf", "open_sans.ttf", "playfair_display.ttf"]);
    let q = FaceQueryDto { group_by_family: true, limit: 100, ..Default::default() };
    let page = library::query_faces(&st, &q).unwrap();
    let groups = library::family_groups(&st, &q).unwrap();
    assert_eq!(groups.iter().map(|g| g.count as u64).sum::<u64>(), page.total);
    let mut next = 0;
    for g in &groups {
        assert_eq!(g.start, next);
        assert!(page.items[g.start as usize..(g.start + g.count) as usize].iter().all(|r| r.family == g.family));
        assert_eq!(g.match_count, g.count);
        assert!(g.style_total >= g.match_count);
        next += g.count;
    }
    // bộ lọc dùng chung với query_faces: tìm "roboto" chỉ còn nhóm Roboto, styleTotal không bị lọc
    let q = FaceQueryDto { text: Some("roboto".into()), ..q };
    let g = library::family_groups(&st, &q).unwrap();
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].family, "Roboto");
}

#[test]
fn rename_and_delete_tag_follow_system_tag_rules() {
    let d = tempfile::tempdir().unwrap();
    let (st, _) = scanned(d.path(), &["roboto.ttf"]);
    let a = key_of(&all(&st).items[0]);
    let t = library::create_tag(&st, "retro").unwrap();
    let other = library::create_tag(&st, "vintage").unwrap();
    library::tag_faces(&st, t.id, std::slice::from_ref(&a)).unwrap();
    assert_eq!(library::rename_tag(&st, t.id, "  Nostalgia ").unwrap().name, "Nostalgia");
    assert_eq!(library::tags_of(&st, &a).unwrap()[0].name, "Nostalgia", "gán thẻ còn nguyên");
    assert_eq!(library::rename_tag(&st, t.id, "VINTAGE").unwrap_err().code, "conflict");
    assert_eq!(library::rename_tag(&st, t.id, " ").unwrap_err().code, "invalid_arg");
    assert_eq!(library::rename_tag(&st, 9999, "x").unwrap_err().code, "not_found");
    // thường → tiền tố dành riêng: chỉ nhận tên trong danh mục, lưu dạng chuẩn
    assert_eq!(library::rename_tag(&st, t.id, "style:nope").unwrap_err().code, "invalid_arg");
    assert_eq!(library::rename_tag(&st, t.id, "Mood: CALM").unwrap().name, "mood:calm");
    // Tag hệ thống không đổi thành tên thường
    assert_eq!(library::rename_tag(&st, t.id, "plain").unwrap_err().code, "invalid_arg");
    assert_eq!(library::delete_tag(&st, t.id).unwrap(), 1, "số face đã bỏ thẻ");
    assert_eq!(library::list_tags(&st).unwrap().iter().map(|x| x.id).collect::<Vec<_>>(), [other.id]);
    assert_eq!(library::delete_tag(&st, t.id).unwrap_err().code, "not_found");
}

#[test]
fn removing_a_source_keeps_all_user_data_and_reports_real_numbers() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf", "open_sans.ttf"]);
    let rows = all(&st).items;
    let (a, b) = (key_of(&rows[0]), key_of(&rows[1]));

    // dữ liệu người dùng quanh font `a`
    library::set_favorite(&st, &a, true).unwrap();
    let col = library::create_collection(&st, "Menu", None).unwrap();
    library::add_to_collection(&st, col.id, std::slice::from_ref(&a)).unwrap();
    let tag = library::create_tag(&st, "retro").unwrap();
    library::tag_faces(&st, tag.id, &[a.clone(), b.clone()]).unwrap();
    let case = cases::create_case(&st, Some("Dự án"), None, None).unwrap();
    cases::add_to_case(&st, Some(case.id), std::slice::from_ref(&a), None).unwrap();
    analysis::set_classification_override(&st, &ClassificationOverrideDto { face: a.clone(), class: "serif".into(), subclass: None }).unwrap();

    let src = sources::list_sources(&st).unwrap()[0].id;
    let preview = sources::remove_source_preview(&st, src).unwrap();
    assert_eq!(
        (preview.removed_locations, preview.faces_left_without_location, preview.cases_affected, preview.collections_affected, preview.tags_affected),
        (2, 2, 1, 1, 1),
        "xem trước = số thật, không đổi gì"
    );
    assert_eq!(all(&st).total, 2, "xem trước không gỡ");

    let done = sources::remove_source(&st, src).unwrap();
    assert_eq!(done, preview, "báo cáo của lệnh gỡ khớp bản xem trước");
    assert_eq!(all(&st).total, 0, "font rời lưới");

    // dữ liệu người dùng GIỮ NGUYÊN
    assert_eq!(library::list_tags(&st).unwrap()[0].face_count, 2, "Tag vẫn gán cho 2 face");
    assert_eq!(library::tags_of(&st, &a).unwrap()[0].name, "retro");
    assert_eq!(library::list_collections(&st).unwrap()[0].item_count, 1);
    let c = cases::get_case(&st, case.id).unwrap();
    assert_eq!((c.faces.len(), c.faces[0].missing), (1, true), "Dự án giữ font, đánh dấu missing");
    let overrides = st.with_store(|s| Ok(s.classification_overrides().unwrap())).unwrap();
    assert_eq!(overrides.len(), 1, "ghi đè phân loại còn");

    // thêm lại nguồn: khớp lại theo hash
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let back = library::query_faces(&st, &FaceQueryDto { favorites_only: true, ..Default::default() }).unwrap();
    assert_eq!((back.total, back.items[0].hash_hex.clone()), (1, a.hash_hex.clone()), "yêu thích khớp lại");
    assert_eq!(library::query_faces(&st, &FaceQueryDto { tag_ids: vec![tag.id], ..Default::default() }).unwrap().total, 2);
    let c = cases::get_case(&st, case.id).unwrap();
    assert!(!c.faces[0].missing, "Dự án hết missing khi font trở lại");
}

#[test]
fn removing_a_source_keeps_faces_that_live_in_another_source() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["roboto.ttf"]);
    // nguồn thứ hai có cùng file (khác thư mục, cùng hash)
    let other = d.path().join("Other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(dir.join("roboto.ttf"), other.join("roboto.ttf")).unwrap();
    sources::add_source(&st, other.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let first = sources::list_sources(&st).unwrap().into_iter().find(|s| s.path.as_deref() == Some(dir.to_str().unwrap())).unwrap();
    let r = sources::remove_source(&st, first.id).unwrap();
    assert_eq!((r.removed_locations, r.faces_left_without_location, r.active_fonts), (1, 0, 0), "face còn ở nguồn kia");
    assert_eq!(all(&st).total, 1);
}

// ---- A2: theo dõi thư mục cloud (FSEvents) — chỉ thư mục tạm, `is_cloud` giả ----

fn wait_until(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let t0 = std::time::Instant::now();
    while t0.elapsed() < Duration::from_secs(secs) {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("hết {secs}s chờ: {what}");
}

#[test]
fn cloud_watch_rescans_the_source_after_a_font_appears_and_disappears() {
    use app_lib::services::cloud_watch::CloudWatch;
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["open_sans.ttf"]);
    assert_eq!(all(&st).total, 1);
    // test dùng thời gian rút ngắn; trong app mặc định là QUIET 2 s / MAX 30 s
    let w = CloudWatch::with_timing(Duration::from_millis(300), Duration::from_secs(3));
    w.sync_with(&st, &|_| true);
    assert_eq!(w.watching().len(), 1);
    std::thread::sleep(Duration::from_millis(600)); // cho luồng FSEvents sẵn sàng

    std::fs::copy(core_fixture("roboto.ttf"), dir.join("roboto.ttf")).unwrap();
    wait_until("font mới tự hiện", 30, || all(&st).total == 2);

    std::fs::remove_file(dir.join("roboto.ttf")).unwrap();
    wait_until("font xoá tự biến mất", 30, || all(&st).total == 1);
    w.shutdown();
}

#[test]
fn cloud_watch_ignores_non_font_noise_and_stops_for_removed_or_non_cloud_sources() {
    use app_lib::services::cloud_watch::CloudWatch;
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["open_sans.ttf"]);
    let w = CloudWatch::with_timing(Duration::from_millis(200), Duration::from_secs(2));
    w.sync_with(&st, &|_| false);
    assert!(w.watching().is_empty(), "không phải ổ cloud thì không theo dõi");
    w.sync_with(&st, &|_| true);
    assert_eq!(w.watching().len(), 1);
    std::thread::sleep(Duration::from_millis(600));
    let queued = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let q2 = queued.clone();
    st.hubs.scan.subscribe(Box::new(move |e| {
        if matches!(e, app_lib::dto::events::ScanEvent::Queued { .. }) {
            q2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }));
    std::fs::write(dir.join("notes.txt"), b"x").unwrap();
    std::fs::write(dir.join(".DS_Store"), b"x").unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(queued.load(std::sync::atomic::Ordering::SeqCst), 0, "file không phải font không gây quét");
    // một cụm 30 thay đổi font chỉ ra đúng MỘT lượt quét
    for i in 0..30 {
        std::fs::copy(core_fixture("roboto.ttf"), dir.join(format!("r{i}.ttf"))).unwrap();
    }
    wait_until("có lượt quét", 30, || queued.load(std::sync::atomic::Ordering::SeqCst) >= 1);
    std::thread::sleep(Duration::from_millis(1200));
    assert_eq!(queued.load(std::sync::atomic::Ordering::SeqCst), 1, "cụm sự kiện được gom thành một lượt");
    sources::remove_source(&st, 1).unwrap();
    w.sync_with(&st, &|_| true);
    assert!(w.watching().is_empty(), "nguồn đã xoá thì dừng theo dõi");
    w.shutdown();
}

#[test]
fn cloud_watch_setting_defaults_on_and_off_stops_every_watcher_at_once() {
    use app_lib::dto::model::SettingsPatchDto;
    use app_lib::services::settings;
    let d = tempfile::tempdir().unwrap();
    let (st, _dir) = scanned(d.path(), &["open_sans.ttf"]);
    assert!(settings::get_settings(&st).unwrap().cloud_watch, "mặc định BẬT");
    st.cloud_watch.treat_all_as_cloud_for_test(true);
    st.cloud_watch.sync(&st);
    assert_eq!(st.cloud_watch.watching().len(), 1);

    let off = settings::set_settings(&st, SettingsPatchDto { cloud_watch: Some(false), ..Default::default() }).unwrap();
    assert!(!off.cloud_watch);
    assert!(st.cloud_watch.watching().is_empty(), "tắt = dừng mọi watcher ngay, không chờ khởi động lại");
    st.cloud_watch.sync(&st); // các lần đồng bộ sau (thêm nguồn, mở app) cũng không bật lại khi đang tắt
    assert!(st.cloud_watch.watching().is_empty());

    let on = settings::set_settings(&st, SettingsPatchDto { cloud_watch: Some(true), ..Default::default() }).unwrap();
    assert!(on.cloud_watch);
    assert_eq!(st.cloud_watch.watching().len(), 1, "bật lại = đồng bộ lại ngay");
    // đổi công tắc khác không đụng tới watcher
    settings::set_settings(&st, SettingsPatchDto { rescan_on_launch: Some(false), ..Default::default() }).unwrap();
    assert_eq!(st.cloud_watch.watching().len(), 1);
    st.cloud_watch.shutdown();
}

#[test]
fn name_clashes_command_is_empty_when_names_differ_and_reads_only() {
    let d = tempfile::tempdir().unwrap();
    let (st, dir) = scanned(d.path(), &["oswald_vf.ttf", "roboto.ttf", "open_sans.ttf"]);
    assert!(library::name_clashes(&st).unwrap().is_empty());
    // chỉ đọc: file trên đĩa không đổi
    assert_eq!(std::fs::read(dir.join("oswald_vf.ttf")).unwrap(), std::fs::read(core_fixture("oswald_vf.ttf")).unwrap());
}
