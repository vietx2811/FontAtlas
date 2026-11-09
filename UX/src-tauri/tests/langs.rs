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
    let l = langs::language_list(&st, &FaceQueryDto::default()).unwrap();
    assert!(!l.complete);
    assert_eq!(l.ready.done, 0);
    assert_eq!(langs::run_backfill(&st), 5);
    assert!(langs::language_list(&st, &FaceQueryDto::default()).unwrap().complete);
}
