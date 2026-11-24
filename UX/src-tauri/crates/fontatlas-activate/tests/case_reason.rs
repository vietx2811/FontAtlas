mod common;
use common::*;

use fontatlas_activate::{ConflictPolicy, DeactivatePolicy, FileOutcome};
use fontatlas_store::{ActivationReason, DesiredState};

fn reasons(env: &Env, k: &fontatlas_store::FaceKey) -> Vec<ActivationReason> {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().map(|r| r.reasons).unwrap_or_default()
}
fn on(env: &Env, k: &fontatlas_store::FaceKey) -> bool {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().is_some_and(|r| r.state == DesiredState::Active)
}

fn setup() -> (Env, fontatlas_activate::ActivationService<fontatlas_activate::FakeEngine>, fontatlas_store::FaceKey, i64) {
    let env = env_with(&["libre_baskerville.ttf"]);
    let svc = service(&env);
    let k = key_of(&env, "libre_baskerville.ttf");
    let c = {
        let mut s = env.store.lock().unwrap();
        let c = s.create_case("Brand Kit A", None, None).unwrap();
        s.add_to_case(c, &[k], "added").unwrap();
        c
    };
    (env, svc, k, c)
}

#[test]
fn activating_a_case_records_the_case_reason_not_manual() {
    let (env, svc, k, c) = setup();
    svc.activate_case(&env.store, c, &[k], None, ConflictPolicy::Ask, &mut |_| {}).unwrap();
    assert!(on(&env, &k));
    assert_eq!(reasons(&env, &k), vec![ActivationReason::Case(c)]);
}

#[test]
fn deactivate_case_lifts_only_its_own_reason_and_keeps_fonts_held_otherwise() {
    let (env, svc, k, c) = setup();
    // một Case khác cũng giữ font, và font còn lý do bật tay
    svc.activate_case(&env.store, c, &[k], None, ConflictPolicy::Ask, &mut |_| {}).unwrap();
    let c2 = env.store.lock().unwrap().create_case("Brand Kit B", None, None).unwrap();
    env.store.lock().unwrap().add_to_case(c2, &[k], "added").unwrap();
    assert_ne!(c, c2);
    svc.activate_case(&env.store, c2, &[k], None, ConflictPolicy::Ask, &mut |_| {}).unwrap();
    assert_eq!(reasons(&env, &k), vec![ActivationReason::Case(c), ActivationReason::Case(c2)]);
    let r = svc.deactivate_case(&env.store, c, &mut |_| {}).unwrap();
    assert!(matches!(r.results[0].outcome, FileOutcome::KeptActive { .. }), "{:?}", r.results[0].outcome);
    assert_eq!(reasons(&env, &k), vec![ActivationReason::Case(c2)]);
    assert!(on(&env, &k));
    // Case cuối cùng gỡ → font tắt
    let r = svc.deactivate_case(&env.store, c2, &mut |_| {}).unwrap();
    assert_eq!(r.results[0].outcome, FileOutcome::Deactivated);
    assert!(!on(&env, &k));
    assert!(reasons(&env, &k).is_empty());
}

#[test]
fn turning_a_font_off_by_hand_wins_over_the_case_reason() {
    let (env, svc, k, c) = setup();
    svc.activate_case(&env.store, c, &[k], None, ConflictPolicy::Ask, &mut |_| {}).unwrap();
    let r = svc.deactivate_faces(&env.store, &[k], DeactivatePolicy::Ask, &mut |_| {}).unwrap();
    assert_eq!(r.results[0].outcome, FileOutcome::Deactivated);
    assert!(!on(&env, &k));
    assert!(reasons(&env, &k).is_empty());
}
