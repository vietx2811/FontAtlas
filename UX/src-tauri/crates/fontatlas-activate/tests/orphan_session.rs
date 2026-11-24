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

#[test]
fn listing_failure_is_reported_and_nothing_is_removed() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let p = path_of(&env, &k);
    off(&svc, &env, k);
    svc.engine().register(&p, Scope::Session).unwrap();
    svc.engine().fail_available_font_paths();
    let rep = svc.unregister_orphans(&env.store).unwrap();
    assert!(rep.listing_failed, "{rep:?}");
    assert_eq!(rep.unregistered, 0);
    assert_eq!(regs(&svc), 1);
}

#[test]
fn a_busy_hash_is_skipped_and_picked_up_next_time() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let p = path_of(&env, &k);
    off(&svc, &env, k);
    svc.engine().register(&p, Scope::Session).unwrap();
    {
        let _busy = svc.hold_for_test(k.hash).expect("giữ guard");
        let rep = svc.unregister_orphans(&env.store).unwrap();
        assert_eq!(rep.unregistered, 0, "thao tác khác đang giữ hash: bỏ qua");
        assert_eq!(regs(&svc), 1);
    }
    assert_eq!(svc.unregister_orphans(&env.store).unwrap().unregistered, 1, "hết bận: dọn được");
}

#[test]
fn a_hash_turned_on_for_the_same_path_while_sweeping_keeps_the_registration() {
    // Hai dòng Inactive cùng đường file P (file thay tại chỗ → hash khác). Khi quét dòng thứ nhất (đang giữ guard của nó), một thao tác khác
    // bật dòng thứ hai trên P: P KHÔNG được gỡ.
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    on(&svc, &env, k);
    let p = path_of(&env, &k);
    off(&svc, &env, k);
    svc.engine().register(&p, Scope::Session).unwrap();
    let base = row(&env, &k);
    let (mut a, mut b) = (base.clone(), base.clone());
    a.hash = [1u8; 32];
    b.hash = [2u8; 32];
    for r in [&a, &b] {
        env.store.lock().unwrap().put_activation(r).unwrap();
    }
    // bỏ dòng gốc khỏi tập ứng viên: chỉ còn a, b
    env.store.lock().unwrap().delete_activation(&k.hash).ok();
    let store_addr = &env.store as *const std::sync::Mutex<fontatlas_store::Store> as usize;
    let mut on_b = b.clone();
    on_b.state = DesiredState::Active;
    on_b.status = ActivationStatus::Registered;
    let fired = std::sync::atomic::AtomicBool::new(false);
    let fired = std::sync::Arc::new(fired);
    let f2 = fired.clone();
    svc.engine().on_call(move |op, _| {
        if op == "available" && !f2.swap(true, std::sync::atomic::Ordering::SeqCst) {
            // SAFETY: `env` sống hết test; khoá Mutex trong hook không giữ qua lời gọi nào khác.
            let st = unsafe { &*(store_addr as *const std::sync::Mutex<fontatlas_store::Store>) };
            st.lock().unwrap().put_activation(&on_b).unwrap();
        }
    });
    let _ = svc.unregister_orphans(&env.store).unwrap();
    assert_eq!(regs(&svc), 1, "P còn đăng ký vì một hash khác vừa được bật trên cùng đường file");
}
