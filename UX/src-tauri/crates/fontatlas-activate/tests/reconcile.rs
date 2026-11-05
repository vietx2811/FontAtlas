mod common;
use common::*;

use fontatlas_activate::{ActivationEngine, ActivationService, ConflictPolicy, FakeEngine, Scope};
use fontatlas_store::{ActivationReason, ActivationRow, ActivationStatus, DesiredState, FaceKey};

const LIBRE: &str = "libre_baskerville.ttf";
const ROBOTO: &str = "roboto.ttf";

fn activate(svc: &ActivationService<FakeEngine>, env: &Env, k: FaceKey, scope: Scope) {
    svc.activate_faces(&env.store, &[k], Some(scope), ConflictPolicy::Ask, &mut no_progress()).unwrap();
}

fn reconcile(svc: &ActivationService<FakeEngine>, env: &Env) -> fontatlas_activate::ReconcileReport {
    svc.reconcile(&env.store, &mut no_progress()).unwrap()
}

fn row(env: &Env, k: &FaceKey) -> ActivationRow {
    env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap()
}

#[test]
fn reconcile_reregisters_session_after_logout() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Session);
    svc.engine().simulate_logout();
    assert!(svc.engine().registrations().is_empty());
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.reregistered, 1);
    assert_eq!(svc.engine().registrations(), vec![(canon(&env.fonts.join(LIBRE)), Scope::Session)]);
    assert!(row(&env, &k).verified_at.is_some());
}

#[test]
fn reconcile_treats_already_registered_as_ok() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Session);
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.failed, 0);
    assert_eq!(rep.reregistered, 0);
    assert_eq!(row(&env, &k).status, ActivationStatus::Registered);
}

#[test]
fn reconcile_missing_then_back() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Persistent);
    let f = env.fonts.join(LIBRE);
    let keep = env.dir.path().join("keep.ttf");
    std::fs::rename(&f, &keep).unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.missing, 1);
    assert!(svc.engine().registrations().is_empty(), "không để font ma");
    let r = row(&env, &k);
    assert_eq!(r.state, DesiredState::Active);
    assert_eq!(r.status, ActivationStatus::MissingFile);
    std::fs::rename(&keep, &f).unwrap();
    reconcile(&svc, &env);
    assert_eq!(row(&env, &k).status, ActivationStatus::Registered);
    assert_eq!(svc.engine().registrations(), vec![(canon(&f), Scope::Persistent)]);
}

#[test]
fn reconcile_moved_file() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Session);
    std::fs::rename(env.fonts.join(LIBRE), env.fonts.join("moi.ttf")).unwrap();
    rescan(&env);
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.moved, 1);
    let new = canon(&env.fonts.join("moi.ttf"));
    assert_eq!(svc.engine().registrations(), vec![(new.clone(), Scope::Session)]);
    assert_eq!(row(&env, &k).active_path.as_deref(), Some(new.to_str().unwrap()));
}

#[test]
fn reconcile_not_downloaded_unregisters_and_keeps_desire() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Persistent);
    std::fs::remove_file(env.fonts.join(LIBRE)).unwrap();
    std::fs::write(env.fonts.join(format!(".{LIBRE}.icloud")), b"").unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.not_downloaded, 1);
    assert!(svc.engine().registrations().is_empty());
    let r = row(&env, &k);
    assert_eq!((r.state, r.status), (DesiredState::Active, ActivationStatus::NotDownloaded));
    assert_eq!(r.reasons, vec![ActivationReason::Manual]);
}

#[test]
fn reconcile_cleans_inactive_leftover() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let p = canon(&env.fonts.join(LIBRE));
    env.store
        .lock()
        .unwrap()
        .put_activation(&ActivationRow {
            reasons: vec![],
            hash: k.hash,
            state: DesiredState::Inactive,
            scope: Scope::Session,
            method: "fake".into(),
            active_path: Some(p.to_string_lossy().into_owned()),
            status: ActivationStatus::Registered,
            conflict_ps: None,
            last_error: None,
            activated_at: None,
            verified_at: None,
            updated_at: 0,
            engine_version: 1,
        })
        .unwrap();
    svc.engine().register(&p, Scope::Session).unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.cleaned, 1);
    assert!(svc.engine().registrations().is_empty());
    assert_eq!(row(&env, &k).status, ActivationStatus::Unregistered);
}

#[test]
fn reconcile_resolves_unknown_rows() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    svc.engine().panic_on_register(&canon(&env.fonts.join(LIBRE)));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        svc.activate_faces(&env.store, &[k], None, ConflictPolicy::Ask, &mut no_progress())
    }));
    assert_eq!(row(&env, &k).status, ActivationStatus::Unknown);
    reconcile(&svc, &env);
    assert_eq!(row(&env, &k).status, ActivationStatus::Registered);
    assert_eq!(svc.engine().registrations(), vec![(canon(&env.fonts.join(LIBRE)), Scope::Session)]);
}

#[test]
fn set_scope_crash_midway_recovers() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Session);
    let p = canon(&env.fonts.join(LIBRE));
    svc.engine().panic_on_register(&p);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        svc.set_scope(&env.store, None, Scope::Persistent)
    }));
    assert_eq!(row(&env, &k).status, ActivationStatus::Unknown);
    reconcile(&svc, &env);
    assert_eq!(svc.engine().registrations(), vec![(p, Scope::Persistent)]);
    let r = row(&env, &k);
    assert_eq!((r.scope, r.status), (Scope::Persistent, ActivationStatus::Registered));
}

#[test]
fn reconcile_finishes_interrupted_deactivate() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Session);
    // Mô phỏng crash sau transaction ý định của deactivate: Inactive + Unknown, vẫn đăng ký.
    {
        let mut s = env.store.lock().unwrap();
        let mut r = s.activation_row(&k.hash).unwrap().unwrap();
        s.clear_activation_reasons(&k.hash).unwrap();
        r.state = DesiredState::Inactive;
        r.status = ActivationStatus::Unknown;
        s.put_activation(&r).unwrap();
    }
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.cleaned, 1);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn reconcile_drops_deleted_collection_reason() {
    let env = env_with(&[LIBRE, ROBOTO]);
    let svc = service(&env);
    let (f1, f2) = (key_of(&env, LIBRE), key_of(&env, ROBOTO));
    let a = {
        let mut s = env.store.lock().unwrap();
        let a = s.create_collection("A", None).unwrap();
        s.add_to_collection(a, &[f1, f2]).unwrap();
        a
    };
    svc.activate_collection(&env.store, a, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    activate(&svc, &env, f2, Scope::Session);
    env.store.lock().unwrap().delete_collection(a).unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(svc.engine().registrations(), vec![(canon(&env.fonts.join(ROBOTO)), Scope::Session)]);
    assert_eq!(row(&env, &f1).state, DesiredState::Inactive);
    assert_eq!(row(&env, &f2).reasons, vec![ActivationReason::Manual]);
    assert_eq!(rep.auto_deactivated.len(), 1);
    let ad = &rep.auto_deactivated[0];
    let hex: String = f1.hash.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(ad.hash_hex, hex);
    assert_eq!(ad.cause, "collection_deleted");
    assert_eq!(ad.removed_reasons, vec![ActivationReason::Collection(a)]);
    assert_eq!(ad.faces, vec![0]);
}

#[test]
fn reconcile_reports_left_collection() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let f1 = key_of(&env, LIBRE);
    let a = {
        let mut s = env.store.lock().unwrap();
        let a = s.create_collection("A", None).unwrap();
        s.add_to_collection(a, &[f1]).unwrap();
        a
    };
    svc.activate_collection(&env.store, a, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    env.store.lock().unwrap().remove_from_collection(a, &[f1]).unwrap();
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.auto_deactivated.len(), 1);
    assert_eq!(rep.auto_deactivated[0].cause, "left_collection");
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn shutdown_unregisters_session_only() {
    let env = env_with(&[LIBRE, ROBOTO]);
    let svc = service(&env);
    let (s1, p1) = (key_of(&env, LIBRE), key_of(&env, ROBOTO));
    activate(&svc, &env, s1, Scope::Session);
    activate(&svc, &env, p1, Scope::Persistent);
    assert_eq!(svc.shutdown(&env.store).unwrap(), 1);
    assert_eq!(svc.engine().registrations(), vec![(canon(&env.fonts.join(ROBOTO)), Scope::Persistent)]);
    let r = row(&env, &s1);
    assert_eq!((r.state, r.status), (DesiredState::Active, ActivationStatus::Unregistered));
    assert_eq!(r.reasons, vec![ActivationReason::Manual]);
}

#[test]
fn shutdown_then_reconcile_restores_session() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, k, Scope::Session);
    svc.shutdown(&env.store).unwrap();
    assert!(svc.engine().registrations().is_empty());
    // app mở lại (cùng phiên đăng nhập)
    let rep = reconcile(&svc, &env);
    assert_eq!(rep.reregistered, 1);
    assert_eq!(svc.engine().registrations(), vec![(canon(&env.fonts.join(LIBRE)), Scope::Session)]);
}

// ---------- review fix: snapshot cũ không được ghi đè ----------
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use fontatlas_activate::DeactivatePolicy;

/// Invariant: `state=Active` ⇔ có ít nhất một lý do.
fn assert_invariant(env: &Env, k: &FaceKey) {
    let r = row(env, k);
    assert_eq!(r.state == DesiredState::Active, !r.reasons.is_empty(), "{r:?}");
}

/// Khi file đầu tiên của vòng lặp gọi `op`, tắt file còn lại bằng `deactivate_faces`.
fn deactivate_other_once(
    svc: &Arc<ActivationService<FakeEngine>>,
    env: &Arc<Env>,
    op: &'static str,
    a: FaceKey,
    b: FaceKey,
) {
    let (pa, fired) = (canon(&env.fonts.join(LIBRE)), Arc::new(AtomicBool::new(false)));
    let (s2, e2) = (svc.clone(), env.clone());
    svc.engine().on_call(move |o, path| {
        if o == op && !fired.swap(true, Ordering::SeqCst) {
            let other = if path == pa { b } else { a };
            s2.deactivate_faces(&e2.store, &[other], DeactivatePolicy::Ask, &mut |_| {}).unwrap();
        }
    });
}

fn two_active() -> (Arc<Env>, Arc<ActivationService<FakeEngine>>, FaceKey, FaceKey) {
    let env = Arc::new(env_with(&[LIBRE, ROBOTO]));
    let svc = Arc::new(service(&env));
    let (a, b) = (key_of(&env, LIBRE), key_of(&env, ROBOTO));
    activate(&svc, &env, a, Scope::Session);
    activate(&svc, &env, b, Scope::Session);
    (env, svc, a, b)
}

fn one_inactive_and_unregistered(env: &Env, svc: &ActivationService<FakeEngine>, a: &FaceKey, b: &FaceKey) {
    let inactive: Vec<_> = [a, b].into_iter().filter(|k| row(env, k).state == DesiredState::Inactive).collect();
    assert_eq!(inactive.len(), 1, "đúng một file bị tắt giữa chừng");
    assert!(row(env, inactive[0]).reasons.is_empty());
    assert_eq!(svc.engine().registrations().len(), 1, "file bị tắt không được hồi sinh");
    assert_invariant(env, a);
    assert_invariant(env, b);
}

#[test]
fn set_scope_does_not_resurrect_row_deactivated_mid_run() {
    let (env, svc, a, b) = two_active();
    deactivate_other_once(&svc, &env, "register", a, b);
    svc.set_scope(&env.store, None, Scope::Persistent).unwrap();
    one_inactive_and_unregistered(&env, &svc, &a, &b);
}

#[test]
fn reconcile_does_not_resurrect_row_deactivated_mid_run() {
    let (env, svc, a, b) = two_active();
    svc.engine().simulate_logout();
    deactivate_other_once(&svc, &env, "register", a, b);
    reconcile(&svc, &env);
    one_inactive_and_unregistered(&env, &svc, &a, &b);
}

#[test]
fn shutdown_does_not_resurrect_row_deactivated_mid_run() {
    let (env, svc, a, b) = two_active();
    deactivate_other_once(&svc, &env, "unregister", a, b);
    svc.shutdown(&env.store).unwrap();
    let inactive: Vec<_> = [a, b].into_iter().filter(|k| row(&env, k).state == DesiredState::Inactive).collect();
    assert_eq!(inactive.len(), 1);
    assert_invariant(&env, &a);
    assert_invariant(&env, &b);
}

// ---------- review fix: shutdown gỡ cả dòng Unknown / Error ----------
fn set_status(env: &Env, k: &FaceKey, status: ActivationStatus) {
    let mut s = env.store.lock().unwrap();
    let mut r = s.activation_row(&k.hash).unwrap().unwrap();
    r.status = status;
    s.put_activation(&r).unwrap();
}

#[test]
fn shutdown_unregisters_unknown_and_error_session_rows() {
    let (env, svc, a, b) = two_active();
    set_status(&env, &a, ActivationStatus::Unknown);
    set_status(&env, &b, ActivationStatus::Error);
    assert_eq!(svc.shutdown(&env.store).unwrap(), 2);
    assert!(svc.engine().registrations().is_empty());
    for k in [a, b] {
        assert_eq!(row(&env, &k).status, ActivationStatus::Unregistered);
        assert_eq!(row(&env, &k).state, DesiredState::Active, "giữ mong muốn để mở lại thì bật lại");
    }
}
