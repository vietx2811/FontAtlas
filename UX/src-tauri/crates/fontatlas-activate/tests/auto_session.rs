//! Auto-on (`auto:<app>`) chỉ sống trong MỘT phiên của app: lúc mở lại (`reconcile`) lý do auto của dòng scope session bị xoá,
//! dòng nào hết lý do thì bị tắt. Đúng cả khi app bị kill (không chạy `shutdown`). "Giữ bật" (scope persistent) giữ nguyên.
mod common;
use common::*;

use fontatlas_activate::{ActivationService, ConflictPolicy, FakeEngine, Scope};
use fontatlas_store::{ActivationReason, AutoApp, DesiredState, FaceKey};

const LIBRE: &str = "libre_baskerville.ttf";

/// Lượt mở app (xoá auto-on của phiên trước).
fn reconcile(svc: &ActivationService<FakeEngine>, env: &Env) -> fontatlas_activate::ReconcileReport {
    svc.reconcile_startup(&env.store, &mut no_progress()).unwrap()
}
/// Lượt thường giữa phiên (sau quét, remove/rebuild nguồn, lượt bù): KHÔNG được đụng auto-on.
fn routine(svc: &ActivationService<FakeEngine>, env: &Env) -> fontatlas_activate::ReconcileReport {
    svc.reconcile(&env.store, &mut no_progress()).unwrap()
}
fn row(env: &Env, k: &FaceKey) -> fontatlas_store::ActivationRow {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap()
}
fn auto(svc: &ActivationService<FakeEngine>, env: &Env, k: FaceKey) {
    svc.activate_auto(&env.store, &[k], AutoApp::Illustrator).unwrap();
}
fn collection_with(env: &Env, k: FaceKey) -> i64 {
    let mut s = env.store.lock().unwrap();
    let c = s.create_collection("C", None).unwrap();
    s.add_to_collection(c, &[k]).unwrap();
    c
}
fn registered(svc: &ActivationService<FakeEngine>) -> usize {
    svc.engine().registrations().len()
}

#[test]
fn a_font_held_only_by_auto_is_turned_off_after_the_app_is_killed_and_reopened() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    auto(&svc, &env, k);
    assert_eq!(row(&env, &k).reasons, vec![ActivationReason::Auto(AutoApp::Illustrator)]);
    assert_eq!(registered(&svc), 1);
    // app bị kill: KHÔNG chạy shutdown; đăng ký session còn nguyên trong hệ điều hành cho tới lúc reconcile lần sau
    let rep = reconcile(&svc, &env);
    let r = row(&env, &k);
    assert_eq!((r.state, r.reasons.clone()), (DesiredState::Inactive, vec![]), "{r:?}");
    assert_eq!(registered(&svc), 0, "font không được còn đăng ký sau khi mở lại app");
    assert_eq!(rep.session_auto_cleared, 1);
    assert!(rep.auto_deactivated.is_empty(), "tắt có chủ ý theo thiết kế, không báo như sự cố: {rep:?}");
    assert_eq!(rep.failed, 0);
}

#[test]
fn clean_shutdown_then_reopen_also_leaves_an_auto_only_font_off() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    auto(&svc, &env, k);
    svc.shutdown(&env.store).unwrap();
    assert_eq!(registered(&svc), 0);
    let rep = reconcile(&svc, &env);
    assert_eq!(row(&env, &k).state, DesiredState::Inactive);
    assert_eq!(registered(&svc), 0, "không bật lại");
    assert_eq!(rep.reregistered, 0);
    assert_eq!(rep.session_auto_cleared, 1);
}

#[test]
fn auto_plus_collection_keeps_the_collection_and_the_font_stays_on() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let c = collection_with(&env, k);
    svc.activate_collection(&env.store, c, Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    auto(&svc, &env, k);
    assert_eq!(row(&env, &k).reasons.len(), 2);
    let rep = reconcile(&svc, &env);
    let r = row(&env, &k);
    assert_eq!(r.state, DesiredState::Active);
    assert_eq!(r.reasons, vec![ActivationReason::Collection(c)]);
    assert_eq!(registered(&svc), 1);
    assert_eq!(rep.session_auto_cleared, 0);
}

#[test]
fn manual_case_and_caseapp_reasons_are_untouched() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let case = {
        let mut s = env.store.lock().unwrap();
        let c = s.create_case("K", None, None).unwrap();
        s.add_to_case(c, &[k], "added").unwrap();
        c
    };
    svc.activate_faces(&env.store, &[k], Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    svc.activate_case(&env.store, case, &[k], Some(Scope::Session), ConflictPolicy::Ask, &mut |_| {}).unwrap();
    svc.activate_case_app(&env.store, case).unwrap();
    auto(&svc, &env, k);
    let before: Vec<_> = row(&env, &k).reasons.into_iter().filter(|r| !matches!(r, ActivationReason::Auto(_))).collect();
    assert_eq!(before.len(), 3, "manual + case + caseapp: {before:?}");
    reconcile(&svc, &env);
    let r = row(&env, &k);
    assert_eq!(r.reasons, before, "chỉ lý do auto bị xoá");
    assert_eq!(r.state, DesiredState::Active);
    assert_eq!(registered(&svc), 1);
}

#[test]
fn keep_on_survives_reopen_because_it_is_persistent() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    auto(&svc, &env, k);
    svc.set_scope(&env.store, Some(&[k.hash]), Scope::Persistent).unwrap(); // nút "Giữ bật"
    let rep = reconcile(&svc, &env);
    let r = row(&env, &k);
    assert_eq!(r.state, DesiredState::Active, "{r:?}");
    assert_eq!(r.reasons, vec![ActivationReason::Auto(AutoApp::Illustrator)], "dòng persistent giữ nguyên, kể cả lý do auto");
    assert_eq!(svc.engine().registrations().iter().filter(|(_, s)| *s == Scope::Persistent).count(), 1);
    assert_eq!(rep.session_auto_cleared, 0);
}

#[test]
fn a_font_that_left_its_collection_is_still_reported_even_when_auto_was_also_present() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let c = collection_with(&env, k);
    svc.activate_collection(&env.store, c, Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    auto(&svc, &env, k);
    env.store.lock().unwrap().remove_from_collection(c, &[k]).unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(row(&env, &k).state, DesiredState::Inactive);
    assert_eq!(rep.auto_deactivated.len(), 1, "rời collection vẫn được báo: {rep:?}");
    assert_eq!(rep.auto_deactivated[0].cause, "left_collection");
    assert_eq!(rep.session_auto_cleared, 0, "đã báo như sự cố thật, không tính là auto tạm thời");
}

#[test]
fn deleting_the_collection_while_auto_was_on_turns_the_font_off_quietly_at_reopen() {
    // `delete_collection` gỡ lý do collection trong cùng giao dịch nên không còn biết vì sao; chỉ còn auto (tạm thời) → tắt, không báo.
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let c = collection_with(&env, k);
    svc.activate_collection(&env.store, c, Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    auto(&svc, &env, k);
    env.store.lock().unwrap().delete_collection(c).unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(row(&env, &k).state, DesiredState::Inactive);
    assert_eq!(registered(&svc), 0);
    assert_eq!(rep.session_auto_cleared, 1);
}

#[test]
fn reopening_twice_is_idempotent() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    auto(&svc, &env, k);
    reconcile(&svc, &env);
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.session_auto_cleared, 0);
    assert_eq!(row(&env, &k).state, DesiredState::Inactive);
    assert_eq!(registered(&svc), 0);
}

#[test]
fn routine_reconcile_passes_in_mid_session_keep_the_auto_font_on() {
    // Hồi quy (review Lead): reconcile_now còn chạy sau mỗi lượt quét/remove_source/rebuild/lượt bù. Xoá auto ở đó sẽ gỡ font vừa bật
    // cho app thiết kế giữa phiên, và panel không gửi lại vì đã đánh dấu tài liệu "đã xử lý".
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    auto(&svc, &env, k);
    for _ in 0..3 {
        let rep = routine(&svc, &env);
        assert_eq!(rep.session_auto_cleared, 0);
        assert!(rep.auto_deactivated.is_empty());
    }
    let r = row(&env, &k);
    assert_eq!(r.state, DesiredState::Active, "{r:?}");
    assert_eq!(r.reasons, vec![ActivationReason::Auto(AutoApp::Illustrator)], "lý do auto còn nguyên giữa phiên");
    assert_eq!(registered(&svc), 1);
    // …và đến lần mở app KẾ TIẾP mới bị xoá
    reconcile(&svc, &env);
    assert_eq!(row(&env, &k).state, DesiredState::Inactive);
}

#[test]
fn startup_treats_an_orphan_active_session_row_without_reasons_as_cleared_not_as_a_silent_failure() {
    // App chết giữa lúc xoá lý do auto và lúc tắt dòng: lần mở sau thấy "Active, không lý do, scope session".
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    auto(&svc, &env, k);
    env.store.lock().unwrap().remove_activation_reason(&k.hash, ActivationReason::Auto(AutoApp::Illustrator)).unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(row(&env, &k).state, DesiredState::Inactive);
    assert_eq!(registered(&svc), 0);
    assert!(rep.auto_deactivated.is_empty(), "{rep:?}");
    assert_eq!(rep.session_auto_cleared, 1);
}
