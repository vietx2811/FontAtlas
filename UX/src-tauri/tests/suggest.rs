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

fn anchor_of(st: &AppState, fam: &str) -> FaceKeyDto {
    key(&find(st, fam))
}

#[test]
fn partners_are_ranked_deterministically_and_explain_themselves() {
    let (_d, st) = setup();
    let a = anchor_of(&st, "Playfair Display");
    let req = SuggestPartnersReqDto { limit: 100, ..Default::default() };
    let x = app_lib::services::suggest_pair::suggest_partners(&st, &a, &req).unwrap();
    let y = app_lib::services::suggest_pair::suggest_partners(&st, &a, &req).unwrap();
    assert_eq!(x, y);
    assert_eq!(x.items.len() as u64, x.total);
    for it in &x.items {
        assert_ne!(it.gate, "blocked");
        assert!(!it.styles.is_empty());
        assert!(matches!(it.role_fit, FitDto::Good | FitDto::Ok));
        assert!(it.row.family != "Playfair Display" || it.face != a);
        assert!(it.reasons.len() <= 4 && it.reasons.iter().all(|r| !r.code.contains(' ')));
    }
    let v = serde_json::to_value(&x).unwrap();
    assert!(!v.to_string().contains("\"score\""));
    // một phong cách cụ thể: mọi kết quả đều khớp nó
    let one = SuggestPartnersReqDto { styles: vec!["contrast".into()], limit: 100, ..Default::default() };
    let c = app_lib::services::suggest_pair::suggest_partners(&st, &a, &one).unwrap();
    assert!(c.items.iter().all(|i| i.styles.contains(&"contrast".to_string())));
    // phong cách lạ → lỗi rõ ràng
    let bad = SuggestPartnersReqDto { styles: vec!["nope".into()], ..Default::default() };
    assert!(app_lib::services::suggest_pair::suggest_partners(&st, &a, &bad).is_err());
}

#[test]
fn desk_sources_fits_role_similar_pinned() {
    let (_d, st) = setup();
    let a = anchor_of(&st, "Roboto");
    let fits = app_lib::services::suggest_pair::desk_suggest(&st, &DeskSuggestReqDto { anchor: Some(a.clone()), ..Default::default() }).unwrap();
    assert!(fits.items.iter().all(|i| i.face != a), "không gợi ý lại chính font neo");
    assert!(fits.items.iter().all(|i| matches!(i.fit, Some(FitDto::Good | FitDto::Ok))));
    let sim = app_lib::services::suggest_pair::desk_suggest(&st, &DeskSuggestReqDto { source: "similar".into(), anchor: Some(a.clone()), ..Default::default() }).unwrap();
    assert!(!sim.items.is_empty() && sim.items.iter().all(|i| i.face != a));
    assert!(sim.items.iter().all(|i| i.reasons.iter().all(|r| r.code.starts_with("similar_") || r.code == "image_close")));
    // similar thiếu anchor / nguồn lạ / pinned thiếu case: lỗi, không panic
    for bad in [DeskSuggestReqDto { source: "similar".into(), ..Default::default() }, DeskSuggestReqDto { source: "zzz".into(), ..Default::default() }, DeskSuggestReqDto { source: "pinned".into(), ..Default::default() }] {
        assert!(app_lib::services::suggest_pair::desk_suggest(&st, &bad).is_err());
    }
}

#[test]
fn combos_stable_by_seed_one_per_title_family_and_respect_locks() {
    let (_d, st) = setup();
    let req = SuggestCombosReqDto { k: 6, seed: 7, ..Default::default() };
    let f = app_lib::services::suggest_pair::suggest_combos;
    let a = f(&st, &req).unwrap();
    assert_eq!(a, f(&st, &req).unwrap(), "cùng seed cùng kết quả");
    assert!(a.len() <= 6);
    let mut fams = std::collections::HashSet::new();
    for c in &a {
        assert!(fams.insert(c.slots[0].row.family.clone()), "tối đa 1 gợi ý mỗi family tiêu đề");
        assert_eq!(c.slots.len(), 2);
        assert_ne!(c.slots[0].face, c.slots[1].face);
        assert!(c.reasons.len() <= 4);
    }
    // khoá font tiêu đề: mọi gợi ý giữ nguyên font đó
    let lock = LockedSlotDto { role: req.roles[0].clone(), face: anchor_of(&st, "Playfair Display") };
    let l = f(&st, &SuggestCombosReqDto { locked: vec![lock.clone()], ..req.clone() }).unwrap();
    assert!(l.iter().all(|c| c.slots[0].face == lock.face && c.slots[0].locked));
    // lỗi đầu vào
    assert!(f(&st, &SuggestCombosReqDto { roles: vec![], ..req.clone() }).is_err());
    assert!(f(&st, &SuggestCombosReqDto { scope: "case".into(), ..req.clone() }).is_err());
    assert!(f(&st, &SuggestCombosReqDto { roles: vec![RoleRefDto { name: "Zzz lạ".into(), profile: None, size: None }], ..req }).is_err());
}

#[test]
fn six_fixture_library_gives_real_partners_and_combos() {
    let (_d, st) = setup();
    let a = anchor_of(&st, "Playfair Display");
    let p = app_lib::services::suggest_pair::suggest_partners(&st, &a, &SuggestPartnersReqDto { limit: 100, ..Default::default() }).unwrap();
    assert!(p.total >= 1, "Playfair Display (serif tương phản) phải có đối tác body trong 6 font mẫu");
    let c = app_lib::services::suggest_pair::suggest_combos(&st, &SuggestCombosReqDto { k: 6, seed: 7, ..Default::default() }).unwrap();
    assert!(!c.is_empty());
}
