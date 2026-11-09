use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use app_lib::dto::langs::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, langs, library, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

const FILES: [&str; 5] = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf"];

fn state(root: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for f in FILES {
        std::fs::copy(fixture(f), dir.join(f)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

/// Chờ luồng nền tính xong độ phủ (hoặc tự tính đồng bộ nếu luồng chưa kịp mở).
fn settle(st: &Arc<AppState>) {
    langs::ensure_started(st);
    let t = Instant::now();
    loop {
        let l = langs::language_list(st, &FaceQueryDto::default()).unwrap();
        if l.complete && l.ready.total > 0 && !st.langs.is_running() {
            return;
        }
        assert!(t.elapsed() < Duration::from_secs(60), "backfill không xong: {:?}", l.ready);
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn q(codes: &[(&str, LangLevelDto)], mode: LangModeDto) -> FaceQueryDto {
    FaceQueryDto {
        langs: codes.iter().map(|(c, l)| LangFilterDto { code: (*c).into(), level: *l }).collect(),
        lang_mode: mode,
        ..Default::default()
    }
}

#[test]
fn backfill_filter_facet_badge_and_coverage() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle(&st);

    // trước khi tính xong `complete=false` đã được kiểm ở test riêng; giờ phải đủ
    let list = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert!(list.complete);
    assert_eq!(list.ready.done, list.ready.total);
    assert_eq!(list.ready.total, 5);
    assert!(list.db_version.starts_with("hg-12d0c093-"));
    let get = |code: &str| list.items.iter().find(|i| i.code == code).cloned();
    let eng = get("eng").expect("eng");
    assert_eq!(eng.near, 5, "cả 5 font Latin đạt ít nhất near với tiếng Anh");
    assert!(eng.full <= eng.near);
    let vie = get("vie");
    println!("vie = {vie:?}");
    assert!(list.items.iter().all(|i| i.near >= i.full && i.near <= 5 && i.near > 0), "chỉ ngôn ngữ có face");

    // lọc: AND / OR / mức
    let n = |qq: &FaceQueryDto| library::query_faces(&st, qq).unwrap().total;
    assert_eq!(n(&q(&[("eng", LangLevelDto::Near)], LangModeDto::All)), 5);
    assert_eq!(n(&q(&[("eng", LangLevelDto::Full)], LangModeDto::All)), eng.full as u64);
    assert_eq!(n(&q(&[("xxx", LangLevelDto::Near)], LangModeDto::All)), 0);
    assert_eq!(n(&q(&[("xxx", LangLevelDto::Near), ("eng", LangLevelDto::Near)], LangModeDto::Any)), 5, "OR");
    assert_eq!(n(&q(&[("xxx", LangLevelDto::Near), ("eng", LangLevelDto::Near)], LangModeDto::All)), 0, "AND");

    // facet: chọn ngôn ngữ không làm số đếm của chính nó tụt; filter khác thì có
    let narrowed = langs::language_list(&st, &q(&[("eng", LangLevelDto::Full)], LangModeDto::All)).unwrap();
    assert_eq!(narrowed.items.iter().find(|i| i.code == "eng").unwrap().near, eng.near);
    let by_text = langs::language_list(&st, &FaceQueryDto { text: Some("roboto".into()), ..Default::default() }).unwrap();
    assert!(by_text.items.iter().find(|i| i.code == "eng").unwrap().near <= 2);

    // Hyperglot `vie` (186) khớp tổng cũ: face không thiếu ký tự Việt nào ⇔ lọc vie full
    let rows = library::query_faces(&st, &FaceQueryDto::default()).unwrap().items;
    let no_missing = rows.iter().filter(|r| r.vi_missing == 0).count() as u64;
    assert_eq!(n(&q(&[("vie", LangLevelDto::Full)], LangModeDto::All)), no_missing);

    // badge: mặc định vie; đổi bằng badgeLang; ngôn ngữ lạ → null
    let page = library::query_faces(&st, &FaceQueryDto::default()).unwrap();
    for r in &page.items {
        let b = r.lang_badge.as_ref().expect("badge vie");
        assert_eq!((b.code.as_str(), b.total), ("vie", 186));
        if r.vi_missing == 0 {
            assert_eq!(b.state, LangBadgeStateDto::Full);
        } else {
            assert_eq!(b.have, 186 - r.vi_missing.min(186));
        }
    }
    let page = library::query_faces(&st, &FaceQueryDto { badge_lang: Some("eng".into()), ..Default::default() }).unwrap();
    assert!(page.items.iter().all(|r| matches!(r.lang_badge.as_ref().unwrap().state, LangBadgeStateDto::Full | LangBadgeStateDto::Near)));
    let page = library::query_faces(&st, &FaceQueryDto { badge_lang: Some("zzz".into()), ..Default::default() }).unwrap();
    assert!(page.items.iter().all(|r| r.lang_badge.is_none()));

    // lang_coverage: ký tự thiếu thật từ cmap. Roboto không có chữ Thái.
    let roboto = library::query_faces(&st, &FaceQueryDto { text: Some("roboto".into()), ..Default::default() }).unwrap().items.remove(0);
    let thai = langs::lang_coverage(&st, roboto.face_id, "tha").unwrap();
    assert_eq!(thai.level, LangCoverageLevelDto::Low);
    assert_eq!(thai.base_have, 0);
    assert_eq!(thai.missing.len() as u32, thai.base_total.min(200));
    assert_eq!(thai.truncated, thai.base_total > 200);
    let en = langs::lang_coverage(&st, roboto.face_id, "eng").unwrap();
    assert!(matches!(en.level, LangCoverageLevelDto::Full | LangCoverageLevelDto::Near));
    assert!(en.missing.len() as u32 == en.base_total - en.base_have);
    // khớp với giá trị đã lưu: face đạt filter ⇒ coverage cũng ≥ near
    assert_eq!(langs::lang_coverage(&st, -1, "eng").unwrap_err().code, "not_found");
    assert_eq!(langs::lang_coverage(&st, roboto.face_id, "nope").unwrap_err().code, "not_found");

    // chạy lại không tính thêm gì
    assert_eq!(langs::run_backfill(&st), 0);
}

#[test]
fn backfill_survives_unreadable_font_and_skips_missing_files() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle(&st);
    // xoá một file rồi quét lại: mục đó là missing, không làm backfill kẹt
    std::fs::remove_file(td.path().join("Fonts/roboto.ttf")).unwrap();
    let ids = st.with_store(|s| Ok(s.conn().query_row("SELECT count(*) FROM faces", [], |r| r.get::<_, i64>(0)).unwrap())).unwrap();
    assert_eq!(ids, 5);
    assert_eq!(langs::run_backfill(&st), 0);
    let l = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert!(l.complete);
}

#[test]
fn stale_db_version_is_recomputed() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle(&st);
    // giả lập dữ liệu ngôn ngữ cũ: đổi dấu về phiên bản khác
    st.with_store(|s| {
        s.conn().execute("UPDATE face_lang_state SET db_version = 'hg-old-0'", []).unwrap();
        Ok(())
    })
    .unwrap();
    st.langs.bump(); // sửa DB trực tiếp: báo cache danh sách ngôn ngữ
    let l = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert!(!l.complete);
    assert_eq!(l.ready.done, 0);
    assert_eq!(langs::run_backfill(&st), 5);
    assert!(langs::language_list(&st, &FaceQueryDto::default()).unwrap().complete);
}

fn make_stale(st: &Arc<AppState>) {
    st.with_store(|s| {
        s.conn().execute("UPDATE face_lang_state SET db_version = 'hg-old-0'", []).unwrap();
        Ok(())
    })
    .unwrap();
    st.langs.bump();
}

fn wait_idle(st: &Arc<AppState>) {
    let t = Instant::now();
    while st.langs.is_running() {
        assert!(t.elapsed() < Duration::from_secs(60), "luồng nền không nghỉ");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn skipped_faces_do_not_make_the_worker_spin_and_shutdown_joins() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle(&st);
    make_stale(&st);
    // một file mất (hoặc iCloud chưa tải): face đó phải ở lại pending, không đánh dấu
    std::fs::remove_file(td.path().join("Fonts/roboto.ttf")).unwrap();
    langs::ensure_started(&st);
    std::thread::sleep(Duration::from_millis(100));
    wait_idle(&st);
    let after_first = st.langs.passes();
    assert!(after_first >= 1);
    let l = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert_eq!((l.ready.done, l.ready.total), (4, 5), "roboto vẫn pending");
    // không quay vòng: không có lượt mới sau nửa giây (lần thử lại chỉ sau 60 s)
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(st.langs.passes(), after_first, "luồng không được chạy lại liên tục");
    assert!(!st.langs.is_running());
    // gọi lại ensure_started nhiều lần không sinh thêm luồng, chỉ thêm một lượt mỗi lần đánh thức
    for _ in 0..3 {
        langs::ensure_started(&st);
    }
    std::thread::sleep(Duration::from_millis(300));
    wait_idle(&st);
    assert!(st.langs.passes() <= after_first + 3);
    // shutdown join được nhanh dù đang chờ thử lại
    let t = Instant::now();
    st.langs.shutdown();
    assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
    langs::ensure_started(&st); // sau shutdown không mở lại
    assert!(!st.langs.is_running());
}

#[test]
fn write_failure_mid_pass_does_not_hang_or_mark_the_face() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle(&st);
    make_stale(&st);
    // ghi dấu "đã tính" cho đúng một font bị từ chối (như BUSY khi quét đang giữ khoá ghi)
    st.with_store(|s| {
        s.conn()
            .execute_batch(
                "CREATE TRIGGER deny_one BEFORE INSERT ON face_lang_state
                 WHEN NEW.hash = (SELECT hash FROM files WHERE file_id = (SELECT min(file_id) FROM files))
                 BEGIN SELECT RAISE(ABORT, 'busy'); END;",
            )
            .unwrap();
        Ok(())
    })
    .unwrap();
    // `put_face_langs` dùng UPSERT trên face_lang_state có sẵn hàng → trigger INSERT vẫn kích hoạt với ON CONFLICT? xoá hàng cũ để chắc
    st.with_store(|s| {
        s.conn().execute("DELETE FROM face_lang_state", []).unwrap();
        Ok(())
    })
    .unwrap();
    langs::ensure_started(&st);
    std::thread::sleep(Duration::from_millis(100));
    wait_idle(&st);
    let l = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert_eq!((l.ready.done, l.ready.total), (4, 5));
    let n = st.langs.passes();
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(st.langs.passes(), n, "không quay vòng khi ghi lỗi");
    // hết lỗi → lần đánh thức sau tính nốt
    st.with_store(|s| {
        s.conn().execute_batch("DROP TRIGGER deny_one").unwrap();
        Ok(())
    })
    .unwrap();
    langs::ensure_started(&st);
    std::thread::sleep(Duration::from_millis(100));
    wait_idle(&st);
    let l = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert!(l.complete, "{:?}", l.ready);
}

#[test]
fn badge_separates_unknown_from_below_near_and_prefers_stored_level() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    // chưa tính (chờ luồng nền xong rồi xoá dấu)
    settle(&st);
    // (xoá dấu): ngôn ngữ khác = unknown, vie vẫn có số từ vi_missing
    st.with_store(|s| {
        s.conn().execute_batch("DELETE FROM face_lang_state; DELETE FROM face_langs;").unwrap();
        Ok(())
    })
    .unwrap();
    let thai = FaceQueryDto { badge_lang: Some("tha".into()), text: Some("roboto".into()), ..Default::default() };
    let b = library::query_faces(&st, &thai).unwrap().items.remove(0).lang_badge.unwrap();
    assert_eq!(b.state, LangBadgeStateDto::Unknown);
    let vie = FaceQueryDto { text: Some("roboto".into()), ..Default::default() };
    let b = library::query_faces(&st, &vie).unwrap().items.remove(0).lang_badge.unwrap();
    assert_ne!(b.state, LangBadgeStateDto::Unknown, "vie dùng vi_missing khi chưa tính");
    // đã tính: Thái dưới near ⇒ Partial (không còn Unknown)
    assert!(langs::run_backfill(&st) >= 5);
    let b = library::query_faces(&st, &thai).unwrap().items.remove(0).lang_badge.unwrap();
    assert_eq!(b.state, LangBadgeStateDto::Partial);
    // vie: mức đã lưu thắng
    let roboto = library::query_faces(&st, &vie).unwrap().items.remove(0);
    let stored = langs::lang_coverage(&st, roboto.face_id, "vie").unwrap();
    let b = roboto.lang_badge.unwrap();
    match stored.level {
        LangCoverageLevelDto::Full => assert_eq!(b.state, LangBadgeStateDto::Full),
        LangCoverageLevelDto::Near => assert_eq!(b.state, LangBadgeStateDto::Near),
        _ => {}
    }
}

#[test]
fn language_list_is_cached_until_bumped() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle(&st);
    let a = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    st.with_store(|s| {
        s.conn().execute_batch("DELETE FROM face_langs WHERE lang = 'eng'").unwrap();
        Ok(())
    })
    .unwrap();
    assert_eq!(langs::language_list(&st, &FaceQueryDto::default()).unwrap(), a, "trúng cache");
    st.langs.bump();
    let b = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert!(b.items.iter().all(|i| i.code != "eng"), "bump làm mất hiệu lực cache");
}
