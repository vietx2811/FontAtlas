use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::dto::suggest::*;
use app_lib::services::{boot, library, sources, suggest};
use app_lib::state::{AppDirs, AppState};

const SIX: [&str; 6] = [
    "roboto.ttf",
    "open_sans.ttf",
    "playfair_display.ttf",
    "libre_baskerville.ttf",
    "source_code_pro.ttf",
    "public_sans_vf.ttf",
];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/fontatlas-core/tests/fixtures")
        .join(name)
}

fn lib6(root: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in SIX {
        std::fs::copy(fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

fn find(st: &AppState, family: &str) -> FaceRowDto {
    let q = FaceQueryDto {
        text: Some(family.into()),
        limit: 50,
        ..Default::default()
    };
    library::query_faces(st, &q)
        .unwrap()
        .items
        .into_iter()
        .find(|r| r.family == family)
        .unwrap_or_else(|| panic!("{family}"))
}

fn key(r: &FaceRowDto) -> FaceKeyDto {
    FaceKeyDto {
        hash_hex: r.hash_hex.clone(),
        face_index: r.face_index,
    }
}


fn role(name: &str) -> RoleRefDto {
    RoleRefDto { name: name.into(), profile: None, size: None }
}

fn setup() -> (tempfile::TempDir, Arc<AppState>) {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    (d, st)
}

#[test]
fn role_fit_one_face_gives_level_and_coded_reasons_never_a_score() {
    let (_d, st) = setup();
    let k = key(&find(&st, "Roboto"));
    let x = suggest::role_fit(&st, &k, &role("Body")).unwrap();
    assert_eq!(x.profile.as_deref(), Some("body"));
    assert_ne!(x.fit, FitDto::Unknown, "Roboto đo đủ: phải có kết luận cho Body");
    assert!(x.reasons.len() <= 3);
    assert!(x.reasons.iter().all(|r| !r.code.is_empty() && !r.code.contains(' ')), "lý do là mã, không phải câu");
    let v = serde_json::to_value(&x).unwrap();
    assert!(v.get("score").is_none());
    // đồng nghĩa tiếng Việt cùng hồ sơ
    assert_eq!(suggest::role_fit(&st, &k, &role("Đoạn văn")).unwrap().profile.as_deref(), Some("body"));
}

#[test]
fn unknown_role_is_unknown_unless_profile_or_size_given() {
    let (_d, st) = setup();
    let k = key(&find(&st, "Roboto"));
    let u = suggest::role_fit(&st, &k, &role("Zzz lạ")).unwrap();
    assert_eq!((u.fit, u.profile.clone()), (FitDto::Unknown, None));
    assert_eq!(u.reasons[0].code, "role_unknown");
    let by_profile = RoleRefDto { name: "Zzz lạ".into(), profile: Some("caption".into()), size: None };
    assert_eq!(suggest::role_fit(&st, &k, &by_profile).unwrap().profile.as_deref(), Some("caption"));
    let by_size = RoleRefDto { name: "Zzz lạ".into(), profile: None, size: Some(RoleSizeDto::Display) };
    assert_eq!(suggest::role_fit(&st, &k, &by_size).unwrap().profile.as_deref(), Some("display"));
    // vai lạ không có gợi ý nào: danh sách rỗng, không lỗi
    let page = suggest::role_fit_faces(&st, &role("Zzz lạ"), &RoleFitListOptsDto::default()).unwrap();
    assert_eq!((page.items.len(), page.total, page.profile), (0, 0, None));
}

#[test]
fn role_fit_list_is_good_or_ok_only_deterministic_and_paged() {
    let (_d, st) = setup();
    let o = RoleFitListOptsDto { limit: 100, ..Default::default() };
    let a = suggest::role_fit_faces(&st, &role("Body"), &o).unwrap();
    let b = suggest::role_fit_faces(&st, &role("Body"), &o).unwrap();
    assert_eq!(a, b, "cùng input, cùng thứ tự");
    assert!(a.total >= 1 && a.items.len() as u64 == a.total);
    assert!(a.items.iter().all(|h| matches!(h.fit, FitDto::Good | FitDto::Ok)));
    // good đứng trước ok
    let first_ok = a.items.iter().position(|h| h.fit == FitDto::Ok).unwrap_or(a.items.len());
    assert!(a.items[first_ok..].iter().all(|h| h.fit == FitDto::Ok));
    // phân trang
    let p = suggest::role_fit_faces(&st, &role("Body"), &RoleFitListOptsDto { limit: 1, offset: 0, ..Default::default() }).unwrap();
    assert_eq!((p.items.len(), p.total), (1, a.total));
    assert_eq!(p.items[0], a.items[0]);
}

#[test]
fn recent_views_record_order_dedupe_and_clear() {
    let (_d, st) = setup();
    let (r, o, p) = (key(&find(&st, "Roboto")), key(&find(&st, "Open Sans")), key(&find(&st, "Playfair Display")));
    for k in [&r, &o, &p, &r] {
        suggest::record_view(&st, k).unwrap();
    }
    let got = suggest::recent_views(&st, 10).unwrap();
    let fams: Vec<&str> = got.iter().map(|v| v.row.as_ref().unwrap().family.as_str()).collect();
    assert_eq!(fams, ["Roboto", "Playfair Display", "Open Sans"]);
    assert_eq!(suggest::clear_recent_views(&st).unwrap(), 3);
    assert!(suggest::recent_views(&st, 10).unwrap().is_empty());
}

#[test]
fn user_class_override_changes_the_fit() {
    use app_lib::services::analysis;
    let (_d, st) = setup();
    let k = key(&find(&st, "Roboto"));
    let before = suggest::role_fit(&st, &k, &role("Body")).unwrap();
    assert_ne!(before.fit, FitDto::Weak);
    analysis::set_classification_override(&st, &app_lib::dto::analysis::ClassificationOverrideDto { face: k.clone(), class: "blackletter".into(), subclass: None }).unwrap();
    let after = suggest::role_fit(&st, &k, &role("Body")).unwrap();
    assert_eq!(after.fit, FitDto::Weak, "người dùng nói blackletter: không hợp Body");
    assert!(after.reasons.iter().any(|r| r.code == "class_unfit"));
    // danh sách theo vai cũng theo class đã sửa
    let page = suggest::role_fit_faces(&st, &role("Body"), &RoleFitListOptsDto { limit: 100, ..Default::default() }).unwrap();
    assert!(page.items.iter().all(|h| h.row.family != "Roboto"));
}

#[test]
fn numerals_without_tnum_data_is_never_weak_for_that_reason() {
    let (_d, st) = setup();
    let k = key(&find(&st, "Roboto"));
    let x = suggest::role_fit(&st, &k, &role("Numerals")).unwrap();
    assert!(x.reasons.iter().all(|r| r.code != "tnum_missing"));
    assert!(x.reasons.iter().any(|r| r.code == "tnum_unknown") || x.reasons.len() == 3);
}
