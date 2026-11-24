//! Font session MỒ CÔI: đăng ký session sống qua kill/crash (gắn với phiên đăng nhập, không với process), còn DB ghi là không bật.
//! `unregister_orphans` chỉ gỡ font mà DB này từng bật (dòng Inactive còn lại), không đụng font khác.
mod common;
use common::*;

use fontatlas_activate::{ActivationEngine, ActivationService, ConflictPolicy, DeactivatePolicy, FakeEngine, Scope};
use fontatlas_store::{ActivationRow, ActivationStatus, DesiredState, FaceKey};

const LIBRE: &str = "libre_baskerville.ttf";
const OPEN: &str = "open_sans.ttf";

fn on(svc: &ActivationService<FakeEngine>, env: &Env, k: FaceKey) {
    svc.activate_faces(&env.store, &[k], Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
}
fn off(svc: &ActivationService<FakeEngine>, env: &Env, k: FaceKey) {
    svc.deactivate_faces(&env.store, &[k], DeactivatePolicy::Ask, &mut no_progress()).unwrap();
}
fn row(env: &Env, k: &FaceKey) -> ActivationRow {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap()
}
fn path_of(env: &Env, k: &FaceKey) -> std::path::PathBuf {
    std::path::PathBuf::from(row(env, k).active_path.expect("active_path"))
}
fn regs(svc: &ActivationService<FakeEngine>) -> usize {
    svc.engine().registrations().len()
}

#[test]
fn a_session_registration_left_for_an_inactive_row_is_removed() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let p = path_of(&env, &k);
    off(&svc, &env, k);
    assert_eq!(regs(&svc), 0);
    // chết giữa lúc gỡ / DB ghi trước khi hệ điều hành xử lý: hệ điều hành còn đăng ký, DB nói Inactive
    svc.engine().register(&p, Scope::Session).unwrap();
    assert_eq!(regs(&svc), 1);
    let rep = svc.unregister_orphans(&env.store).unwrap();
    assert_eq!((rep.unregistered, rep.failed, rep.listing_failed), (1, 0, false), "{rep:?}");
    assert_eq!(regs(&svc), 0);
    let r = row(&env, &k);
    assert_eq!((r.state, r.status), (DesiredState::Inactive, ActivationStatus::Unregistered), "DB không đổi");
}

#[test]
fn a_font_the_db_wants_on_is_left_alone() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let rep = svc.unregister_orphans(&env.store).unwrap();
    assert_eq!(rep.unregistered, 0);
    assert_eq!(regs(&svc), 1, "font đang bật không bị gỡ");
}

#[test]
fn a_registration_this_db_never_enabled_is_not_touched() {
    let env = env_with(&[LIBRE, OPEN]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    off(&svc, &env, k);
    // OPEN chưa từng được bật trong DB này, nhưng app khác đăng ký session cùng file
    let other = canon(&env.fonts.join(OPEN));
    svc.engine().register(&other, Scope::Session).unwrap();
    let rep = svc.unregister_orphans(&env.store).unwrap();
    assert_eq!(rep.unregistered, 0, "{rep:?}");
    assert_eq!(regs(&svc), 1, "đăng ký của bên khác còn nguyên");
}

#[test]
fn a_persistent_registration_is_not_touched() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let p = path_of(&env, &k);
    off(&svc, &env, k);
    svc.engine().register(&p, Scope::Persistent).unwrap();
    let rep = svc.unregister_orphans(&env.store).unwrap();
    assert_eq!(rep.unregistered, 0);
    assert_eq!(svc.engine().registrations(), vec![(p, Scope::Persistent)]);
}

#[test]
fn a_path_another_row_wants_on_is_kept() {
    // hai dòng (hash) cùng đường file: một bật, một Inactive → đường file phải còn đăng ký
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let p = path_of(&env, &k);
    let mut ghost = row(&env, &k);
    ghost.hash = [9u8; 32];
    ghost.state = DesiredState::Inactive;
    ghost.status = ActivationStatus::Unregistered;
    env.store.lock().unwrap().put_activation(&ghost).unwrap();
    let rep = svc.unregister_orphans(&env.store).unwrap();
    assert_eq!(rep.unregistered, 0);
    assert_eq!(svc.engine().registrations(), vec![(p, Scope::Session)]);
}

#[test]
fn protected_font_dirs_are_never_touched() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    off(&svc, &env, k);
    let sys = std::path::PathBuf::from("/Library/Fonts/Orphan-Test.ttf");
    let mut r = row(&env, &k);
    r.hash = [7u8; 32];
    r.active_path = Some(sys.to_string_lossy().into_owned());
    env.store.lock().unwrap().put_activation(&r).unwrap();
    svc.engine().register(&sys, Scope::Session).unwrap();
    let rep = svc.unregister_orphans(&env.store).unwrap();
    assert_eq!(rep.unregistered, 0, "{rep:?}");
    assert_eq!(regs(&svc), 1);
}

#[test]
fn an_empty_or_unlistable_state_does_nothing_and_is_idempotent() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    assert_eq!(svc.unregister_orphans(&env.store).unwrap().unregistered, 0, "DB rỗng");
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let p = path_of(&env, &k);
    off(&svc, &env, k);
    svc.engine().register(&p, Scope::Session).unwrap();
    assert_eq!(svc.unregister_orphans(&env.store).unwrap().unregistered, 1);
    assert_eq!(svc.unregister_orphans(&env.store).unwrap().unregistered, 0, "lần hai không còn gì");
}
