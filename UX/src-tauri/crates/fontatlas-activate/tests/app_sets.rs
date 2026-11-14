mod common;
use common::*;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use fontatlas_activate::{AppSetEventKind, AppSetTracker, FileOutcome};
use fontatlas_store::{ActivationReason, CaseApp, DesiredState};

const GRACE: Duration = Duration::from_secs(30);
const AI: &str = "com.adobe.Illustrator";
const ID: &str = "com.adobe.InDesign";

fn running(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}
fn app(b: &str) -> CaseApp {
    CaseApp { bundle_id: b.into(), app_name: b.into() }
}
fn reasons(env: &Env, k: &fontatlas_store::FaceKey) -> Vec<ActivationReason> {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().map(|r| r.reasons).unwrap_or_default()
}
fn state(env: &Env, k: &fontatlas_store::FaceKey) -> Option<DesiredState> {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().map(|r| r.state)
}

/// Env có một font trong một Case gắn với `apps`.
fn setup(apps: &[&str]) -> (Env, fontatlas_activate::ActivationService<fontatlas_activate::FakeEngine>, fontatlas_store::FaceKey, i64) {
    let env = env_with(&["libre_baskerville.ttf"]);
    let svc = service(&env);
    let k = key_of(&env, "libre_baskerville.ttf");
    let c = {
        let mut s = env.store.lock().unwrap();
        let c = s.create_collection("Brand Kit A", None).unwrap();
        s.add_to_collection(c, &[k]).unwrap();
        s.set_case_apps(c, &apps.iter().map(|b| app(b)).collect::<Vec<_>>()).unwrap();
        c
    };
    (env, svc, k, c)
}

#[test]
fn launch_turns_on_and_quit_turns_off_after_grace() {
    let (env, svc, k, c) = setup(&[AI]);
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();

    // app chưa chạy: không làm gì
    assert!(svc.sync_app_sets(&env.store, &running(&[]), t0, GRACE, &mut tr).unwrap().is_empty());
    assert_eq!(state(&env, &k), None);

    // app chạy → bật, lý do caseapp, scope session
    let ev = svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    assert_eq!(ev.len(), 1);
    assert_eq!((ev[0].collection_id, ev[0].kind, ev[0].changed), (c, AppSetEventKind::On, 1));
    assert_eq!(reasons(&env, &k), vec![ActivationReason::CaseApp(c)]);
    let row = env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap();
    assert_eq!(row.scope, fontatlas_store::ActivationScope::Session);
    assert_eq!(state(&env, &k), Some(DesiredState::Active));

    // gọi lại khi app còn chạy: không sự kiện nữa (idempotent)
    assert!(svc.sync_app_sets(&env.store, &running(&[AI]), t0 + Duration::from_secs(5), GRACE, &mut tr).unwrap().is_empty());

    // app thoát: trong ân hạn font còn
    let q = t0 + Duration::from_secs(10);
    assert!(svc.sync_app_sets(&env.store, &running(&[]), q, GRACE, &mut tr).unwrap().is_empty());
    assert!(svc.sync_app_sets(&env.store, &running(&[]), q + Duration::from_secs(29), GRACE, &mut tr).unwrap().is_empty());
    assert_eq!(state(&env, &k), Some(DesiredState::Active), "còn trong ân hạn");

    // hết ân hạn: gỡ
    let ev = svc.sync_app_sets(&env.store, &running(&[]), q + GRACE, GRACE, &mut tr).unwrap();
    assert_eq!(ev.len(), 1);
    assert_eq!((ev[0].kind, ev[0].changed), (AppSetEventKind::Off, 1));
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
    assert_eq!(tr.held(), 0);
}

#[test]
fn reopen_within_grace_keeps_fonts_and_cancels_the_release() {
    let (env, svc, k, _c) = setup(&[AI]);
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    svc.sync_app_sets(&env.store, &running(&[]), t0 + Duration::from_secs(1), GRACE, &mut tr).unwrap();
    let ev = svc.sync_app_sets(&env.store, &running(&[AI]), t0 + Duration::from_secs(10), GRACE, &mut tr).unwrap();
    assert!(ev.is_empty(), "mở lại trong ân hạn không bật lại, không báo");
    // quá hạn tính từ lúc thoát đầu tiên nhưng app đang chạy: không gỡ
    let ev = svc.sync_app_sets(&env.store, &running(&[AI]), t0 + Duration::from_secs(100), GRACE, &mut tr).unwrap();
    assert!(ev.is_empty());
    assert_eq!(state(&env, &k), Some(DesiredState::Active));
}

#[test]
fn two_apps_keep_the_case_on_until_the_last_one_quits() {
    let (env, svc, k, _c) = setup(&[AI, ID]);
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI, ID]), t0, GRACE, &mut tr).unwrap();
    // một app thoát, app kia còn: không bao giờ gỡ
    svc.sync_app_sets(&env.store, &running(&[ID]), t0 + Duration::from_secs(1), GRACE, &mut tr).unwrap();
    let ev = svc.sync_app_sets(&env.store, &running(&[ID]), t0 + Duration::from_secs(500), GRACE, &mut tr).unwrap();
    assert!(ev.is_empty());
    assert_eq!(state(&env, &k), Some(DesiredState::Active));
    // app cuối thoát
    svc.sync_app_sets(&env.store, &running(&[]), t0 + Duration::from_secs(600), GRACE, &mut tr).unwrap();
    let ev = svc.sync_app_sets(&env.store, &running(&[]), t0 + Duration::from_secs(631), GRACE, &mut tr).unwrap();
    assert_eq!(ev[0].kind, AppSetEventKind::Off);
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
}

#[test]
fn releasing_never_turns_off_a_font_that_has_another_reason() {
    let (env, svc, k, c) = setup(&[AI]);
    // người dùng đã bật tay (persistent) trước khi app chạy
    svc.activate_faces(&env.store, &[k], Some(fontatlas_activate::Scope::Persistent), fontatlas_activate::ConflictPolicy::Ask, &mut |_| {}).unwrap();
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    assert_eq!(reasons(&env, &k), vec![ActivationReason::Manual, ActivationReason::CaseApp(c)]);
    let row = env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap();
    assert_eq!(row.scope, fontatlas_store::ActivationScope::Persistent, "không hạ persistent xuống session");
    svc.sync_app_sets(&env.store, &running(&[]), t0, GRACE, &mut tr).unwrap();
    svc.sync_app_sets(&env.store, &running(&[]), t0 + GRACE, GRACE, &mut tr).unwrap();
    assert_eq!(reasons(&env, &k), vec![ActivationReason::Manual]);
    assert_eq!(state(&env, &k), Some(DesiredState::Active), "bật tay vẫn giữ");
}

#[test]
fn a_manual_deactivate_wins_over_the_app_reason() {
    let (env, svc, k, _c) = setup(&[AI]);
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    svc.deactivate_faces(&env.store, &[k], fontatlas_activate::DeactivatePolicy::Ask, &mut |_| {}).unwrap();
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
    assert!(reasons(&env, &k).is_empty());
    // app còn chạy: không bật lại giữa chừng (chỉ bật khi app chạy lại)
    svc.sync_app_sets(&env.store, &running(&[AI]), t0 + Duration::from_secs(5), GRACE, &mut tr).unwrap();
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
}

#[test]
fn leftovers_from_a_crash_are_released_on_the_first_sync() {
    let (env, svc, k, c) = setup(&[AI]);
    // lần chạy trước: bật xong rồi Font Atlas chết (tracker mới, app không còn chạy)
    let mut old = AppSetTracker::new();
    svc.sync_app_sets(&env.store, &running(&[AI]), Instant::now(), GRACE, &mut old).unwrap();
    assert_eq!(reasons(&env, &k), vec![ActivationReason::CaseApp(c)]);
    let mut fresh = AppSetTracker::new();
    let ev = svc.sync_app_sets(&env.store, &running(&[]), Instant::now(), GRACE, &mut fresh).unwrap();
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].kind, AppSetEventKind::Off);
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
}

#[test]
fn leftovers_with_the_app_still_running_are_kept() {
    let (env, svc, k, _c) = setup(&[AI]);
    let mut old = AppSetTracker::new();
    svc.sync_app_sets(&env.store, &running(&[AI]), Instant::now(), GRACE, &mut old).unwrap();
    let mut fresh = AppSetTracker::new();
    svc.sync_app_sets(&env.store, &running(&[AI]), Instant::now(), GRACE, &mut fresh).unwrap();
    assert_eq!(state(&env, &k), Some(DesiredState::Active));
    assert_eq!(fresh.held(), 1);
}

#[test]
fn unbinding_or_deleting_the_case_releases_immediately() {
    let (env, svc, k, c) = setup(&[AI]);
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    env.store.lock().unwrap().set_case_apps(c, &[]).unwrap();
    let ev = svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    assert_eq!(ev[0].kind, AppSetEventKind::Off, "bỏ gắn app thì gỡ ngay, không ân hạn");
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
    assert_eq!(tr.held(), 0);
}

#[test]
fn deleting_a_case_via_the_service_releases_its_app_reason() {
    let (env, svc, k, c) = setup(&[AI]);
    let mut tr = AppSetTracker::new();
    svc.sync_app_sets(&env.store, &running(&[AI]), Instant::now(), GRACE, &mut tr).unwrap();
    svc.on_collection_deleted(&env.store, c, &mut |_| {}).unwrap();
    env.store.lock().unwrap().delete_collection(c).unwrap();
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
}

#[test]
fn release_all_on_exit_clears_every_case_app_reason() {
    let (env, svc, k, _c) = setup(&[AI]);
    let mut tr = AppSetTracker::new();
    svc.sync_app_sets(&env.store, &running(&[AI]), Instant::now(), GRACE, &mut tr).unwrap();
    assert_eq!(svc.release_all_case_apps(&env.store).unwrap(), 1);
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
    assert!(env.store.lock().unwrap().case_app_reason_collections().unwrap().is_empty());
}

#[test]
fn a_conflicting_font_is_not_turned_on_and_is_counted() {
    let env = env_with(&["libre_baskerville.ttf"]);
    // hai file khác nội dung cùng PS name: một file bật trước (Manual), file trong Case xung đột
    let other = ingest_fake(&env, "twin.ttf", "x", &["LibreBaskerville-Regular"]);
    let engine = fontatlas_activate::FakeEngine::new();
    teach(&engine, &env);
    let svc = fontatlas_activate::ActivationService::new(engine);
    let first = key_of(&env, "libre_baskerville.ttf");
    svc.activate_faces(&env.store, &[first], None, fontatlas_activate::ConflictPolicy::Ask, &mut |_| {}).unwrap();
    let c = {
        let mut s = env.store.lock().unwrap();
        let c = s.create_collection("Brand Kit A", None).unwrap();
        s.add_to_collection(c, &[fontatlas_store::FaceKey { hash: other, face_index: 0 }]).unwrap();
        s.set_case_apps(c, &[app(AI)]).unwrap();
        c
    };
    let mut tr = AppSetTracker::new();
    let ev = svc.sync_app_sets(&env.store, &running(&[AI]), Instant::now(), GRACE, &mut tr).unwrap();
    assert_eq!((ev[0].collection_id, ev[0].changed, ev[0].conflicts), (c, 0, 1));
    assert_eq!(state(&env, &fontatlas_store::FaceKey { hash: other, face_index: 0 }), None, "không bật font xung đột");
    let _ = FileOutcome::Activated;
}

#[test]
fn suppress_removes_only_the_app_reason_and_keeps_other_reasons() {
    let (env, svc, k, c) = setup(&[AI]);
    // bật tay (persistent) trước, app chạy sau → hai lý do
    svc.activate_faces(&env.store, &[k], Some(fontatlas_activate::Scope::Persistent), fontatlas_activate::ConflictPolicy::Ask, &mut |_| {}).unwrap();
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    assert_eq!(reasons(&env, &k), vec![ActivationReason::Manual, ActivationReason::CaseApp(c)]);
    svc.suppress_case_app(&env.store, c, &mut tr).unwrap();
    assert_eq!(reasons(&env, &k), vec![ActivationReason::Manual], "lý do bật tay còn nguyên");
    assert_eq!(state(&env, &k), Some(DesiredState::Active));
    assert!(tr.is_suppressed(c));
}

#[test]
fn suppress_turns_the_fonts_off_and_the_observer_does_not_turn_them_back_on_until_relaunch() {
    let (env, svc, k, c) = setup(&[AI]);
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI]), t0, GRACE, &mut tr).unwrap();
    assert_eq!(state(&env, &k), Some(DesiredState::Active));
    svc.suppress_case_app(&env.store, c, &mut tr).unwrap();
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive), "chỉ có lý do app → tắt");

    // app vẫn chạy qua nhiều nhịp: không bật lại, không sự kiện
    for i in 1..=5 {
        let ev = svc.sync_app_sets(&env.store, &running(&[AI]), t0 + Duration::from_secs(i), GRACE, &mut tr).unwrap();
        assert!(ev.is_empty(), "nhịp {i}");
    }
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));

    // app thoát hết → hết hiệu lực
    svc.sync_app_sets(&env.store, &running(&[]), t0 + Duration::from_secs(10), GRACE, &mut tr).unwrap();
    assert!(!tr.is_suppressed(c));
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));

    // chạy lại → bật như thường
    let ev = svc.sync_app_sets(&env.store, &running(&[AI]), t0 + Duration::from_secs(20), GRACE, &mut tr).unwrap();
    assert_eq!(ev.len(), 1);
    assert_eq!((ev[0].kind, ev[0].changed), (AppSetEventKind::On, 1));
    assert_eq!(state(&env, &k), Some(DesiredState::Active));
    assert_eq!(reasons(&env, &k), vec![ActivationReason::CaseApp(c)]);
}

#[test]
fn suppress_with_two_apps_lasts_until_both_have_quit() {
    let (env, svc, k, c) = setup(&[AI, ID]);
    let mut tr = AppSetTracker::new();
    let t0 = Instant::now();
    svc.sync_app_sets(&env.store, &running(&[AI, ID]), t0, GRACE, &mut tr).unwrap();
    svc.suppress_case_app(&env.store, c, &mut tr).unwrap();
    // một app thoát, app kia còn chạy: vẫn bị chặn
    svc.sync_app_sets(&env.store, &running(&[ID]), t0 + Duration::from_secs(2), GRACE, &mut tr).unwrap();
    svc.sync_app_sets(&env.store, &running(&[ID, AI]), t0 + Duration::from_secs(4), GRACE, &mut tr).unwrap();
    assert_eq!(state(&env, &k), Some(DesiredState::Inactive));
    assert!(tr.is_suppressed(c));
}
