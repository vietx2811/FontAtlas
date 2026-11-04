mod common;
use common::*;

use std::sync::{Arc, Barrier};

use fontatlas_activate::{
    ActivationService, ConflictPolicy, EngineError, FakeEngine, FileOutcome, Owner, Progress,
    SETTING_ACTIVATION_KEEP, Scope,
};
use fontatlas_store::{ActivationReason, ActivationStatus, DesiredState};

const LIBRE: &str = "libre_baskerville.ttf";
const ROBOTO: &str = "roboto.ttf";
const LIBRE_PS: &str = "LibreBaskerville-Regular";
const RIGHTFONT_PATH: &str = "/Users/u/Library/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/L/x.ttf";

fn activate(
    svc: &ActivationService<FakeEngine>,
    env: &Env,
    keys: &[fontatlas_store::FaceKey],
    policy: ConflictPolicy,
) -> fontatlas_activate::ActivationReport {
    svc.activate_faces(&env.store, keys, None, policy, &mut no_progress()).unwrap()
}

fn row(env: &Env, hash: &[u8; 32]) -> Option<fontatlas_store::ActivationRow> {
    env.store.lock().unwrap().activation_row(hash).unwrap()
}

#[test]
fn activate_face_registers_canonical_path_session_by_default() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated);
    assert_eq!(rep.changed, 1);
    let c = canon(&env.fonts.join(LIBRE));
    assert_eq!(svc.engine().registrations(), vec![(c.clone(), Scope::Session)]);
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.status, ActivationStatus::Registered);
    assert_eq!(r.state, DesiredState::Active);
    assert_eq!(r.active_path.as_deref(), Some(c.to_str().unwrap()));
    assert_eq!(r.method, "fake");
    assert_eq!(rep.results[0].path.as_deref(), Some(c.to_str().unwrap()));
}

#[test]
fn activate_releases_store_lock_between_files() {
    let env = env_with(&[LIBRE, ROBOTO]);
    let svc = service(&env);
    let keys = [key_of(&env, LIBRE), key_of(&env, ROBOTO)];
    let mut free = vec![];
    svc.activate_faces(&env.store, &keys, None, ConflictPolicy::Ask, &mut |_p: Progress| {
        free.push(env.store.try_lock().is_ok());
    })
    .unwrap();
    assert_eq!(free, vec![true, true]);
}

#[test]
fn activate_uses_persistent_when_keep_setting_on() {
    let env = env_with(&[LIBRE]);
    env.store.lock().unwrap().set_setting(SETTING_ACTIVATION_KEEP, "1").unwrap();
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_eq!(svc.engine().registrations()[0].1, Scope::Persistent);
    assert_eq!(row(&env, &k.hash).unwrap().scope, Scope::Persistent);
}

#[test]
fn activate_twice_is_already_active() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::AlreadyActive);
    assert_eq!(rep.changed, 0);
    assert_eq!(svc.engine().registrations().len(), 1);
}

#[test]
fn activate_adds_manual_reason() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_eq!(row(&env, &k.hash).unwrap().reasons, vec![ActivationReason::Manual]);
}

#[test]
fn activate_collection_adds_collection_reason() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let c = {
        let mut s = env.store.lock().unwrap();
        let c = s.create_collection("Dự án A", None).unwrap();
        s.add_to_collection(c, &[k]).unwrap();
        c
    };
    let rep = svc.activate_collection(&env.store, c, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated);
    assert_eq!(row(&env, &k.hash).unwrap().reasons, vec![ActivationReason::Collection(c)]);
}

#[test]
fn already_active_file_gains_new_reason() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let c = {
        let mut s = env.store.lock().unwrap();
        let c = s.create_collection("A", None).unwrap();
        s.add_to_collection(c, &[k]).unwrap();
        c
    };
    let rep = svc.activate_collection(&env.store, c, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::AlreadyActive);
    assert_eq!(row(&env, &k.hash).unwrap().reasons, vec![ActivationReason::Manual, ActivationReason::Collection(c)]);
    assert_eq!(svc.engine().registrations().len(), 1);
}

#[test]
fn needs_confirmation_adds_no_reason() {
    let env = env_with(&[LIBRE]);
    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, std::path::Path::new(RIGHTFONT_PATH)));
    let k = key_of(&env, LIBRE);
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert!(matches!(rep.results[0].outcome, FileOutcome::NeedsConfirmation { .. }));
    assert!(row(&env, &k.hash).is_none());
}

#[test]
fn crash_after_intent_leaves_unknown_row() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    svc.engine().panic_on_register(&canon(&env.fonts.join(LIBRE)));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        svc.activate_faces(&env.store, &[k], None, ConflictPolicy::Ask, &mut no_progress())
    }));
    assert!(r.is_err());
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.status, ActivationStatus::Unknown);
    assert_eq!(r.state, DesiredState::Active);
    assert_eq!(r.reasons, vec![ActivationReason::Manual]);
    assert!(svc.engine().registrations().is_empty());
    // khoá in_flight được nhả dù panic
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_ne!(rep.results[0].outcome, FileOutcome::Busy);
}

#[test]
fn register_error_rolls_back_new_reason() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    svc.engine().fail_next_register(EngineError::Rejected { code: 999, message: "x".into() });
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let r = row(&env, &k.hash).unwrap();
    assert!(r.reasons.is_empty());
    assert_eq!(r.state, DesiredState::Inactive);
    assert_eq!(r.status, ActivationStatus::Error);
}

#[test]
fn engine_failure_records_error() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    svc.engine().fail_next_register(EngineError::Rejected { code: 999, message: "x".into() });
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert!(matches!(&rep.results[0].outcome, FileOutcome::Failed { code, .. } if code == "engine.rejected"));
    assert_eq!(rep.failed, 1);
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.status, ActivationStatus::Error);
    assert_eq!(r.last_error.as_deref(), Some("engine.rejected"));
}

#[test]
fn preflight_ignores_own_old_path() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let old = canon(&env.fonts.join(LIBRE));
    std::fs::rename(env.fonts.join(LIBRE), env.fonts.join("renamed.ttf")).unwrap();
    rescan(&env);
    teach(svc.engine(), &env);
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated, "{:?}", rep.results[0].outcome);
    let new = canon(&env.fonts.join("renamed.ttf"));
    assert_eq!(svc.engine().registrations(), vec![(new, Scope::Session)]);
    assert!(!svc.engine().registrations().iter().any(|(p, _)| *p == old));
}

#[test]
fn moved_file_reregisters_new_path() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    std::fs::create_dir_all(env.fonts.join("sub")).unwrap();
    std::fs::rename(env.fonts.join(LIBRE), env.fonts.join("sub").join(LIBRE)).unwrap();
    rescan(&env);
    teach(svc.engine(), &env);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let new = canon(&env.fonts.join("sub").join(LIBRE));
    assert_eq!(svc.engine().registrations(), vec![(new.clone(), Scope::Session)]);
    assert_eq!(row(&env, &k.hash).unwrap().active_path.as_deref(), Some(new.to_str().unwrap()));
}

#[test]
fn same_batch_duplicate_ps_name() {
    let env = env_with(&[]);
    let a = ingest_fake(&env, "a.ttf", "file-a", &["Dup-Regular"]);
    let b = ingest_fake(&env, "b.ttf", "file-b", &["Dup-Regular"]);
    let svc = service(&env);
    let keys = [fontatlas_store::FaceKey { hash: a, face_index: 0 }, fontatlas_store::FaceKey { hash: b, face_index: 0 }];
    let rep = activate(&svc, &env, &keys, ConflictPolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated);
    match &rep.results[1].outcome {
        FileOutcome::NeedsConfirmation { conflicts } => {
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].owner, Owner::FontAtlas);
            assert_eq!(conflicts[0].postscript_name, "Dup-Regular");
            assert!(conflicts[0].existing_path.ends_with("a.ttf"));
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn concurrent_activation_same_hash_is_busy() {
    let env = Arc::new(env_with(&[LIBRE]));
    let svc = Arc::new(service(&env));
    let k = key_of(&env, LIBRE);
    let barrier = Arc::new(Barrier::new(2));
    svc.engine().block_register_until(barrier.clone());
    let (e2, s2) = (env.clone(), svc.clone());
    let t = std::thread::spawn(move || {
        s2.activate_faces(&e2.store, &[k], None, ConflictPolicy::Ask, &mut no_progress()).unwrap()
    });
    while !svc.engine().is_blocked() {
        std::thread::yield_now();
    }
    let second = svc.activate_faces(&env.store, &[k], None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert_eq!(second.results[0].outcome, FileOutcome::Busy);
    barrier.wait();
    let first = t.join().unwrap();
    assert_eq!(first.results[0].outcome, FileOutcome::Activated);
    assert_eq!(svc.engine().registrations().len(), 1);
}

#[test]
fn activate_reports_progress_per_file() {
    let env = env_with(&[LIBRE, ROBOTO]);
    let svc = service(&env);
    let keys = [key_of(&env, LIBRE), key_of(&env, ROBOTO)];
    let mut seen = vec![];
    svc.activate_faces(&env.store, &keys, None, ConflictPolicy::Ask, &mut |p: Progress| seen.push((p.done, p.total)))
        .unwrap();
    assert_eq!(seen, vec![(1, 2), (2, 2)]);
}

#[test]
fn preflight_flags_rightfont_duplicate() {
    let env = env_with(&[LIBRE]);
    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, std::path::Path::new(RIGHTFONT_PATH)));
    let k = key_of(&env, LIBRE);
    let pre = svc.preflight(&env.store, &[k]).unwrap();
    assert!(matches!(pre.results[0].outcome, FileOutcome::NeedsConfirmation { .. }));
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    match &rep.results[0].outcome {
        FileOutcome::NeedsConfirmation { conflicts } => {
            assert_eq!(conflicts[0].owner, Owner::RightFont);
            assert_eq!(conflicts[0].existing_path, RIGHTFONT_PATH);
        }
        o => panic!("{o:?}"),
    }
    assert_eq!(rep.needs_confirmation, 1);
    assert!(svc.engine().registrations().is_empty());
    assert!(row(&env, &k.hash).is_none());
}

#[test]
fn proceed_registers_despite_conflict() {
    let env = env_with(&[LIBRE]);
    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, std::path::Path::new(RIGHTFONT_PATH)));
    let k = key_of(&env, LIBRE);
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Proceed);
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated);
    assert_eq!(row(&env, &k.hash).unwrap().conflict_ps.as_deref(), Some(LIBRE_PS));
}

#[test]
fn own_registration_is_not_a_conflict() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let pre = svc.preflight(&env.store, &[k]).unwrap();
    assert_eq!(pre.results[0].outcome, FileOutcome::AlreadyActive);
}

#[test]
fn preflight_without_conflict_is_ready_and_writes_nothing() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let pre = svc.preflight(&env.store, &[k]).unwrap();
    assert_eq!(pre.results[0].outcome, FileOutcome::Ready);
    assert!(svc.engine().registrations().is_empty());
    assert!(row(&env, &k.hash).is_none());
}

#[test]
fn missing_file_outcome() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    std::fs::remove_file(env.fonts.join(LIBRE)).unwrap();
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::MissingFile);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn not_downloaded_outcome() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    std::fs::remove_file(env.fonts.join(LIBRE)).unwrap();
    std::fs::write(env.fonts.join(format!(".{LIBRE}.icloud")), b"").unwrap();
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::NotDownloaded);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn unknown_hash_is_not_found() {
    let env = env_with(&[]);
    let svc = service(&env);
    let rep = activate(&svc, &env, &[fontatlas_store::FaceKey { hash: [9; 32], face_index: 0 }], ConflictPolicy::Ask);
    assert!(matches!(&rep.results[0].outcome, FileOutcome::Failed { code, .. } if code == "not_found"));
}

#[test]
fn ttc_face_reports_extra_faces() {
    let env = env_with(&[]);
    let h = ingest_fake(&env, "family.ttc", "ttc", &["A-0", "A-1"]);
    let svc = service(&env);
    let rep = activate(&svc, &env, &[fontatlas_store::FaceKey { hash: h, face_index: 1 }], ConflictPolicy::Ask);
    assert_eq!(rep.results[0].requested_faces, vec![1]);
    assert_eq!(rep.results[0].extra_faces, vec![0]);
    assert_eq!(svc.engine().registrations().len(), 1);
}

#[test]
fn activate_collection_activates_each_file_once() {
    let env = env_with(&[]);
    let h = ingest_fake(&env, "family.ttc", "ttc2", &["B-0", "B-1"]);
    let svc = service(&env);
    let c = {
        let mut s = env.store.lock().unwrap();
        let c = s.create_collection("A", None).unwrap();
        s.add_to_collection(
            c,
            &[fontatlas_store::FaceKey { hash: h, face_index: 0 }, fontatlas_store::FaceKey { hash: h, face_index: 1 }],
        )
        .unwrap();
        c
    };
    let rep = svc.activate_collection(&env.store, c, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert_eq!(rep.results.len(), 1);
    assert_eq!(rep.results[0].requested_faces, vec![0, 1]);
    assert_eq!(svc.engine().registrations().len(), 1);
}
