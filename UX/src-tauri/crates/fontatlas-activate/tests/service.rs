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

/// Font đã cài ở nơi khác (vd FontBase) có nội dung GIỐNG HỆT file trong thư viện → `same_file`; khác một byte → không.
#[test]
fn conflict_same_file_compares_bytes_not_paths() {
    let env = env_with(&[LIBRE]);
    let installed = env.dir.path().join("FontBase");
    std::fs::create_dir_all(&installed).unwrap();
    let identical = installed.join("copy.ttf");
    std::fs::copy(env.fonts.join(LIBRE), &identical).unwrap();
    let k = key_of(&env, LIBRE);

    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, &identical));
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    match &rep.results[0].outcome {
        FileOutcome::AlreadyAvailable { path, .. } => assert_eq!(path, identical.to_str().unwrap(), "bản sao nguyên văn → đã có sẵn"),
        o => panic!("{o:?}"),
    }
    assert_eq!((rep.changed, rep.needs_confirmation), (0, 0));
    assert!(svc.engine().registrations().is_empty(), "không đăng ký engine");
    assert!(row(&env, &k.hash).is_none(), "không ghi dòng kích hoạt");

    // Khác một byte cuối (cùng kích thước): không còn là cùng file.
    let mut bytes = std::fs::read(&identical).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    let changed = installed.join("changed.ttf");
    std::fs::write(&changed, &bytes).unwrap();
    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, &changed));
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    match &rep.results[0].outcome {
        FileOutcome::NeedsConfirmation { conflicts } => assert!(!conflicts[0].same_file, "khác nội dung phải hỏi"),
        o => panic!("{o:?}"),
    }

    // Khác kích thước (thêm đuôi): khác. File không đọc được (đường dẫn giả): khác.
    bytes.push(0);
    std::fs::write(&changed, &bytes).unwrap();
    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, &changed));
    match &activate(&svc, &env, &[k], ConflictPolicy::Ask).results[0].outcome {
        FileOutcome::NeedsConfirmation { conflicts } => assert!(!conflicts[0].same_file),
        o => panic!("{o:?}"),
    }
    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, std::path::Path::new(RIGHTFONT_PATH)));
    match &activate(&svc, &env, &[k], ConflictPolicy::Ask).results[0].outcome {
        FileOutcome::NeedsConfirmation { conflicts } => assert!(!conflicts[0].same_file),
        o => panic!("{o:?}"),
    }
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

// ---------- Task 6: deactivate (đếm lý do) + set_scope ----------
use fontatlas_activate::DeactivatePolicy;

fn deactivate(
    svc: &ActivationService<FakeEngine>,
    env: &Env,
    keys: &[fontatlas_store::FaceKey],
    policy: DeactivatePolicy,
) -> fontatlas_activate::ActivationReport {
    svc.deactivate_faces(&env.store, keys, policy, &mut no_progress()).unwrap()
}

fn collection(env: &Env, name: &str, keys: &[fontatlas_store::FaceKey]) -> i64 {
    let mut s = env.store.lock().unwrap();
    let c = s.create_collection(name, None).unwrap();
    s.add_to_collection(c, keys).unwrap();
    c
}

fn activate_coll(svc: &ActivationService<FakeEngine>, env: &Env, c: i64) {
    svc.activate_collection(&env.store, c, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
}

fn deactivate_coll(svc: &ActivationService<FakeEngine>, env: &Env, c: i64) -> fontatlas_activate::ActivationReport {
    svc.deactivate_collection(&env.store, c, &mut no_progress()).unwrap()
}

fn outcome_of(rep: &fontatlas_activate::ActivationReport, k: &fontatlas_store::FaceKey) -> FileOutcome {
    let hex: String = k.hash.iter().map(|b| format!("{b:02x}")).collect();
    rep.results.iter().find(|r| r.hash_hex == hex).unwrap().outcome.clone()
}

#[test]
fn deactivate_uses_stored_path_even_if_file_deleted() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    std::fs::remove_file(env.fonts.join(LIBRE)).unwrap();
    let rep = deactivate(&svc, &env, &[k], DeactivatePolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
    assert_eq!(rep.changed, 1);
    assert!(svc.engine().registrations().is_empty());
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.state, DesiredState::Inactive);
    assert_eq!(r.status, ActivationStatus::Unregistered);
    assert!(r.active_path.is_some(), "giữ active_path");
}

#[test]
fn deactivate_not_registered_is_ok() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let p = canon(&env.fonts.join(LIBRE));
    use fontatlas_activate::ActivationEngine;
    svc.engine().unregister(&p, Scope::Session).unwrap(); // bị gỡ ngoài app
    let rep = deactivate(&svc, &env, &[k], DeactivatePolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
}

#[test]
fn deactivate_inactive_is_already_inactive() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    assert_eq!(deactivate(&svc, &env, &[k], DeactivatePolicy::Ask).results[0].outcome, FileOutcome::AlreadyInactive);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    deactivate(&svc, &env, &[k], DeactivatePolicy::Ask);
    assert_eq!(deactivate(&svc, &env, &[k], DeactivatePolicy::Ask).results[0].outcome, FileOutcome::AlreadyInactive);
}

#[test]
fn two_overlapping_collections_keep_shared_file() {
    let env = env_with(&[LIBRE, ROBOTO, "open_sans.ttf"]);
    let svc = service(&env);
    let (f1, f2, f3) = (key_of(&env, LIBRE), key_of(&env, ROBOTO), key_of(&env, "open_sans.ttf"));
    let a = collection(&env, "A", &[f1, f2]);
    let b = collection(&env, "B", &[f2, f3]);
    activate_coll(&svc, &env, a);
    activate_coll(&svc, &env, b);
    assert_eq!(svc.engine().registrations().len(), 3);
    let rep = deactivate_coll(&svc, &env, a);
    assert_eq!(outcome_of(&rep, &f1), FileOutcome::Deactivated);
    assert_eq!(outcome_of(&rep, &f2), FileOutcome::KeptActive { manual: false, collection_ids: vec![b] });
    let mut left: Vec<_> = svc.engine().registrations().into_iter().map(|(p, _)| p).collect();
    left.sort();
    let mut want = vec![canon(&env.fonts.join(ROBOTO)), canon(&env.fonts.join("open_sans.ttf"))];
    want.sort();
    assert_eq!(left, want);
    deactivate_coll(&svc, &env, b);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn manual_plus_collection() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let f1 = key_of(&env, LIBRE);
    activate(&svc, &env, &[f1], ConflictPolicy::Ask);
    let a = collection(&env, "A", &[f1]);
    activate_coll(&svc, &env, a);
    let rep = deactivate_coll(&svc, &env, a);
    assert_eq!(rep.results[0].outcome, FileOutcome::KeptActive { manual: true, collection_ids: vec![] });
    assert_eq!(svc.engine().registrations().len(), 1);
    assert_eq!(deactivate(&svc, &env, &[f1], DeactivatePolicy::Ask).results[0].outcome, FileOutcome::Deactivated);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn manual_deactivate_held_by_collection_asks() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let f1 = key_of(&env, LIBRE);
    let a = collection(&env, "A", &[f1]);
    activate_coll(&svc, &env, a);
    let rep = deactivate(&svc, &env, &[f1], DeactivatePolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::HeldByCollections { collection_ids: vec![a] });
    assert_eq!(svc.engine().registrations().len(), 1);
    assert_eq!(row(&env, &f1.hash).unwrap().reasons, vec![ActivationReason::Collection(a)]);
    let rep = deactivate(&svc, &env, &[f1], DeactivatePolicy::Force);
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
    assert!(row(&env, &f1.hash).unwrap().reasons.is_empty());
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn deactivate_collection_after_item_removed() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let f1 = key_of(&env, LIBRE);
    let a = collection(&env, "A", &[f1]);
    activate_coll(&svc, &env, a);
    env.store.lock().unwrap().remove_from_collection(a, &[f1]).unwrap();
    let rep = deactivate_coll(&svc, &env, a);
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn deactivate_all_clears_every_registration_both_scopes() {
    let env = env_with(&[LIBRE, ROBOTO]);
    let svc = service(&env);
    let (f1, f2) = (key_of(&env, LIBRE), key_of(&env, ROBOTO));
    svc.activate_faces(&env.store, &[f1], Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    let a = collection(&env, "A", &[f2]);
    svc.activate_collection(&env.store, a, Some(Scope::Persistent), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    let rep = svc.deactivate_all(&env.store, &mut no_progress()).unwrap();
    assert_eq!(rep.changed, 2);
    assert!(svc.engine().registrations().is_empty());
    for k in [f1, f2] {
        let r = row(&env, &k.hash).unwrap();
        assert!(r.reasons.is_empty());
        assert_eq!(r.state, DesiredState::Inactive);
    }
}

#[test]
fn set_scope_session_to_persistent_keeps_font_visible() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let rep = svc.set_scope(&env.store, None, Scope::Persistent).unwrap();
    assert_eq!(rep.changed, 1);
    let p = canon(&env.fonts.join(LIBRE));
    assert_eq!(svc.engine().registrations(), vec![(p, Scope::Persistent)]);
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.scope, Scope::Persistent);
    assert_eq!(r.status, ActivationStatus::Registered);
    // gọi lại: không còn gì để đổi
    assert_eq!(svc.set_scope(&env.store, None, Scope::Persistent).unwrap().results.len(), 0);
}

#[test]
fn set_scope_register_failure_keeps_old_scope() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    svc.engine().fail_next_register(EngineError::Rejected { code: 7, message: "x".into() });
    let rep = svc.set_scope(&env.store, Some(&[k.hash]), Scope::Persistent).unwrap();
    assert_eq!(rep.failed, 1);
    let p = canon(&env.fonts.join(LIBRE));
    assert_eq!(svc.engine().registrations(), vec![(p, Scope::Session)]);
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.scope, Scope::Session);
    assert_eq!(r.status, ActivationStatus::Registered);
}

#[test]
fn state_lists_rows_with_reasons() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let st = svc.state(&env.store).unwrap();
    assert_eq!(st.len(), 1);
    let hex: String = k.hash.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(st[0].hash_hex, hex);
    assert_eq!(st[0].state, DesiredState::Active);
    assert_eq!(st[0].status, ActivationStatus::Registered);
    assert_eq!(st[0].reasons, vec![ActivationReason::Manual]);
    assert_eq!(st[0].active_path.as_deref(), Some(canon(&env.fonts.join(LIBRE)).to_str().unwrap()));
}

// ---------- review fix: đăng ký mồ côi + dòng Error ----------
fn force_status(env: &Env, k: &fontatlas_store::FaceKey, status: ActivationStatus) {
    let mut s = env.store.lock().unwrap();
    let mut r = s.activation_row(&k.hash).unwrap().unwrap();
    r.status = status;
    s.put_activation(&r).unwrap();
}

#[test]
fn activate_unregisters_orphan_left_by_unknown_or_error_row() {
    for orphan_status in [ActivationStatus::Unknown, ActivationStatus::Error] {
        let env = env_with(&[LIBRE]);
        let svc = service(&env);
        let k = key_of(&env, LIBRE);
        svc.activate_faces(&env.store, &[k], Some(Scope::Persistent), ConflictPolicy::Ask, &mut no_progress()).unwrap();
        force_status(&env, &k, orphan_status); // đăng ký (P, Persistent) vẫn còn trong hệ thống
        std::fs::rename(env.fonts.join(LIBRE), env.fonts.join("moi.ttf")).unwrap();
        rescan(&env);
        teach(svc.engine(), &env);
        let rep = svc.activate_faces(&env.store, &[k], Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
        assert_eq!(rep.results[0].outcome, FileOutcome::Activated, "{orphan_status:?}");
        let new = canon(&env.fonts.join("moi.ttf"));
        assert_eq!(svc.engine().registrations(), vec![(new, Scope::Session)], "{orphan_status:?}: không để lại đăng ký cũ");
    }
}

#[test]
fn set_scope_old_scope_unregister_failure_keeps_old_record_and_reconcile_cleans() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    let p = canon(&env.fonts.join(LIBRE));
    svc.engine().fail_next_unregister(EngineError::Rejected { code: 5, message: "x".into() });
    let rep = svc.set_scope(&env.store, None, Scope::Persistent).unwrap();
    assert_eq!(rep.failed, 1);
    let r = row(&env, &k.hash).unwrap();
    assert_eq!((r.scope, r.status, r.state), (Scope::Session, ActivationStatus::Error, DesiredState::Active));
    assert_eq!(r.active_path.as_deref(), Some(p.to_str().unwrap()));
    assert!(r.last_error.is_some());
    assert_eq!(svc.engine().registrations().len(), 2, "cả hai scope còn đăng ký");
    svc.reconcile(&env.store, &mut no_progress()).unwrap();
    assert_eq!(svc.engine().registrations(), vec![(p, Scope::Session)]);
    assert_eq!(row(&env, &k.hash).unwrap().status, ActivationStatus::Registered);
}

#[test]
fn deactivate_all_cleans_error_rows() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    force_status(&env, &k, ActivationStatus::Error);
    let rep = svc.deactivate_all(&env.store, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn deactivate_faces_retries_inactive_error_row() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    // gỡ lần đầu thất bại → Inactive + Error, font vẫn đăng ký
    svc.engine().fail_next_unregister(EngineError::Rejected { code: 5, message: "x".into() });
    let first = deactivate(&svc, &env, &[k], DeactivatePolicy::Ask);
    assert!(matches!(first.results[0].outcome, FileOutcome::Failed { .. }));
    let r = row(&env, &k.hash).unwrap();
    assert_eq!((r.state, r.status), (DesiredState::Inactive, ActivationStatus::Error));
    assert_eq!(svc.engine().registrations().len(), 1);
    let again = deactivate(&svc, &env, &[k], DeactivatePolicy::Ask);
    assert_eq!(again.results[0].outcome, FileOutcome::Deactivated);
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn deactivate_after_interrupted_set_scope_removes_both_scopes() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask); // (P, Session)
    svc.engine().panic_on_register(&canon(&env.fonts.join(LIBRE)));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| svc.set_scope(&env.store, None, Scope::Persistent)));
    assert_eq!(row(&env, &k.hash).unwrap().status, ActivationStatus::Unknown);
    let rep = deactivate(&svc, &env, &[k], DeactivatePolicy::Ask);
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
    assert!(svc.engine().registrations().is_empty(), "scope cũ còn lại là đăng ký mồ côi");
}

#[test]
fn deactivate_all_cleans_inactive_error_rows() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    svc.engine().fail_next_unregister(EngineError::Rejected { code: 5, message: "x".into() });
    deactivate(&svc, &env, &[k], DeactivatePolicy::Ask); // Inactive + Error, vẫn đăng ký
    assert_eq!(svc.engine().registrations().len(), 1);
    let rep = svc.deactivate_all(&env.store, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
    assert!(svc.engine().registrations().is_empty());
}

// ---------- review fix: collection bị xoá ----------
#[test]
fn on_collection_deleted_unregisters_orphans_before_store_delete() {
    let env = env_with(&[LIBRE, ROBOTO]);
    let svc = service(&env);
    let (f1, f2) = (key_of(&env, LIBRE), key_of(&env, ROBOTO));
    let a = collection(&env, "A", &[f1, f2]);
    activate_coll(&svc, &env, a);
    activate(&svc, &env, &[f2], ConflictPolicy::Ask); // f2: thủ công + collection
    let rep = svc.on_collection_deleted(&env.store, a, &mut no_progress()).unwrap();
    assert_eq!(outcome_of(&rep, &f1), FileOutcome::Deactivated);
    assert_eq!(outcome_of(&rep, &f2), FileOutcome::KeptActive { manual: true, collection_ids: vec![] });
    env.store.lock().unwrap().delete_collection(a).unwrap();
    assert_eq!(svc.engine().registrations().len(), 1);
}

#[test]
fn on_collection_deleted_after_store_delete_sweeps_orphans() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let f1 = key_of(&env, LIBRE);
    let a = collection(&env, "A", &[f1]);
    activate_coll(&svc, &env, a);
    env.store.lock().unwrap().delete_collection(a).unwrap(); // store đã gỡ lý do trong cùng transaction
    assert!(row(&env, &f1.hash).unwrap().reasons.is_empty());
    let rep = svc.on_collection_deleted(&env.store, a, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::Deactivated);
    assert!(svc.engine().registrations().is_empty());
    assert_eq!(row(&env, &f1.hash).unwrap().state, DesiredState::Inactive);
}

#[test]
fn new_collection_does_not_inherit_deleted_collection_reasons() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let f1 = key_of(&env, LIBRE);
    let a = collection(&env, "A", &[f1]);
    activate_coll(&svc, &env, a);
    env.store.lock().unwrap().delete_collection(a).unwrap();
    let b = env.store.lock().unwrap().create_collection("B", None).unwrap();
    assert_ne!(a, b);
    assert!(!row(&env, &f1.hash).unwrap().reasons.contains(&ActivationReason::Collection(b)));
}

// ---------- review fix: nội dung file đổi / đường dẫn không UTF-8 ----------
#[test]
fn activate_rejects_file_whose_content_changed() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    std::fs::copy(fixture(ROBOTO), env.fonts.join(LIBRE)).unwrap(); // cùng tên, nội dung khác, chưa rescan
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert!(matches!(&rep.results[0].outcome, FileOutcome::Failed { code, .. } if code == "file_changed"), "{:?}", rep.results[0].outcome);
    assert!(svc.engine().registrations().is_empty());
    assert!(row(&env, &k.hash).is_none(), "không đăng ký dưới hash cũ");
}

// APFS (macOS) từ chối tên file không phải UTF-8 (EILSEQ), nên tình huống chỉ tạo được trên Linux;
// nhánh `BadPath` còn lại là phòng thủ cho Linux / đường dẫn Windows có surrogate lẻ.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn activate_rejects_non_utf8_canonical_path() {
    use std::os::unix::ffi::OsStrExt;
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    // Vị trí trong DB là symlink ASCII; đích thật có tên không phải UTF-8.
    let target = env.fonts.join(std::ffi::OsStr::from_bytes(b"f\xff\xfe.ttf"));
    std::fs::rename(env.fonts.join(LIBRE), &target).unwrap();
    std::os::unix::fs::symlink(&target, env.fonts.join(LIBRE)).unwrap();
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert!(matches!(&rep.results[0].outcome, FileOutcome::Failed { code, .. } if code == "path_not_utf8"), "{:?}", rep.results[0].outcome);
    assert!(svc.engine().registrations().is_empty());
}

// ---------- "Có sẵn trên máy", scope hiệu lực, set_scope báo lỗi ----------

fn installed_copy(env: &Env, name: &str) -> std::path::PathBuf {
    let d = env.dir.path().join("FontBase");
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join("copy.ttf");
    std::fs::copy(env.fonts.join(name), &p).unwrap();
    p
}

#[test]
fn already_available_is_recomputed_and_forgotten_when_the_other_side_removes_it() {
    let env = env_with(&[LIBRE]);
    let k = key_of(&env, LIBRE);
    let copy = installed_copy(&env, LIBRE);
    let with = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, &copy));
    // Kể cả preflight (dry run) cũng báo có sẵn; chưa ghi nhớ gì.
    assert!(matches!(with.preflight(&env.store, &[k]).unwrap().results[0].outcome, FileOutcome::AlreadyAvailable { .. }));
    activate(&with, &env, &[k], ConflictPolicy::Ask);
    // Mở app lại (service mới, cùng DB) mà không truyền hash: vẫn tính ra nhờ tập đã nhớ.
    let av = with.availability(&env.store, &[]).unwrap();
    assert_eq!(av.len(), 1);
    assert_eq!(av[0].path, copy.to_str().unwrap());
    // Bên kia gỡ font → quay về "chưa bật" và quên luôn.
    let without = service(&env);
    assert!(without.availability(&env.store, &[k.hash]).unwrap().is_empty());
    assert!(without.availability(&env.store, &[]).unwrap().is_empty());
}

#[test]
fn available_collection_member_counts_as_satisfied_without_a_row() {
    let env = env_with(&[LIBRE]);
    let k = key_of(&env, LIBRE);
    let copy = installed_copy(&env, LIBRE);
    let svc = service_with(&env, FakeEngine::new().with_system_font(LIBRE_PS, &copy));
    let c = collection(&env, "A", &[k]);
    let rep = svc.activate_collection(&env.store, c, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert!(matches!(rep.results[0].outcome, FileOutcome::AlreadyAvailable { .. }));
    assert_eq!((rep.changed, rep.needs_confirmation, rep.failed), (0, 0, 0));
    assert!(row(&env, &k.hash).is_none());
    // Cả khi người dùng chọn "vẫn bật" cho cả lô, file cùng nội dung vẫn không bị đăng ký đôi.
    let rep = activate(&svc, &env, &[k], ConflictPolicy::Proceed);
    assert!(matches!(rep.results[0].outcome, FileOutcome::AlreadyAvailable { .. }));
    assert!(svc.engine().registrations().is_empty());
}

#[test]
fn our_own_active_face_is_not_reported_as_available() {
    let env = env_with(&[LIBRE]);
    let k = key_of(&env, LIBRE);
    let svc = service(&env);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    assert!(svc.availability(&env.store, &[k.hash]).unwrap().is_empty());
}

#[test]
fn collection_activation_does_not_downgrade_a_persistent_face() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    svc.activate_faces(&env.store, &[k], Some(Scope::Persistent), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    let c = collection(&env, "A", &[k]);
    // Bật collection (scope mặc định = session, KHÔNG tường minh) → vẫn "Giữ bật".
    let rep = svc.activate_collection(&env.store, c, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::AlreadyActive);
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.scope, Scope::Persistent);
    assert!(r.reasons.contains(&ActivationReason::Collection(c)));
    assert_eq!(svc.engine().registrations().len(), 1);
    assert_eq!(svc.engine().registrations()[0].1, Scope::Persistent);
    // Người dùng chọn tường minh "chỉ phiên này" khi face đang được giữ persistent: scope tối đa thắng.
    let rep = svc.activate_faces(&env.store, &[k], Some(Scope::Session), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::AlreadyActive);
    assert_eq!(row(&env, &k.hash).unwrap().scope, Scope::Persistent);
}

#[test]
fn explicit_persistent_upgrades_a_session_face_but_implicit_default_does_not_touch_it() {
    let env = env_with(&[LIBRE]);
    env.store.lock().unwrap().set_setting(SETTING_ACTIVATION_KEEP, "0").unwrap();
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    env.store.lock().unwrap().set_setting(SETTING_ACTIVATION_KEEP, "1").unwrap();
    // Mặc định đổi sang persistent nhưng lần bật này không tường minh: face giữ nguyên scope session.
    assert_eq!(activate(&svc, &env, &[k], ConflictPolicy::Ask).results[0].outcome, FileOutcome::AlreadyActive);
    assert_eq!(row(&env, &k.hash).unwrap().scope, Scope::Session);
    // Tường minh "Giữ bật" thì nâng lên.
    let rep = svc.activate_faces(&env.store, &[k], Some(Scope::Persistent), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated);
    assert_eq!(row(&env, &k.hash).unwrap().scope, Scope::Persistent);
    assert_eq!(svc.engine().registrations(), vec![(canon(&env.fonts.join(LIBRE)), Scope::Persistent)]);
}

#[test]
fn set_scope_reports_rows_it_cannot_change_instead_of_skipping_silently() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    // Dòng đang ở trạng thái Error (vd lần gỡ trước thất bại).
    let mut r = row(&env, &k.hash).unwrap();
    r.status = ActivationStatus::Error;
    env.store.lock().unwrap().put_activation(&r).unwrap();
    let rep = svc.set_scope(&env.store, Some(&[k.hash]), Scope::Persistent).unwrap();
    assert_eq!(rep.failed, 1);
    match &rep.results[0].outcome {
        FileOutcome::Failed { code, .. } => assert_eq!(code, "scope_unsettled"),
        o => panic!("{o:?}"),
    }
    assert_eq!(row(&env, &k.hash).unwrap().scope, Scope::Session, "không đổi gì");
}

#[test]
fn set_scope_engine_failure_on_unregister_is_reported() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    activate(&svc, &env, &[k], ConflictPolicy::Ask);
    svc.engine().fail_next_unregister(EngineError::Rejected { code: 9, message: "x".into() });
    let rep = svc.set_scope(&env.store, Some(&[k.hash]), Scope::Persistent).unwrap();
    assert_eq!(rep.failed, 1);
    assert_eq!(row(&env, &k.hash).unwrap().status, ActivationStatus::Error);
}

#[test]
fn set_scope_keeps_collection_reasons() {
    let env = env_with(&[LIBRE]);
    let svc = service(&env);
    let k = key_of(&env, LIBRE);
    let c = collection(&env, "A", &[k]);
    svc.activate_collection(&env.store, c, None, ConflictPolicy::Ask, &mut no_progress()).unwrap();
    svc.set_scope(&env.store, Some(&[k.hash]), Scope::Persistent).unwrap();
    let r = row(&env, &k.hash).unwrap();
    assert_eq!(r.scope, Scope::Persistent);
    assert_eq!(r.reasons, vec![ActivationReason::Collection(c)]);
}
