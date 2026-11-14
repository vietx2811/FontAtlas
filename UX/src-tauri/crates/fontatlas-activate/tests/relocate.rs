mod common;
use common::*;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use fontatlas_activate::{ActivationEngine, ActivationService, ConflictPolicy, EngineError, FakeEngine, FileOutcome, Scope, RELOCATE_PENDING_KEY};
use fontatlas_store::{ActivationScope, DesiredState, SourceKind};

fn no_progress() -> impl FnMut(fontatlas_activate::Progress) {
    |_| {}
}

/// Một font đang bật từ thư mục nguồn; thêm nguồn thứ hai `org/` chứa bản chép cùng nội dung.
fn setup(scope: Scope) -> (Env, ActivationService<FakeEngine>, fontatlas_store::FaceKey, PathBuf, PathBuf) {
    let env = env_with(&["libre_baskerville.ttf"]);
    let svc = service(&env);
    let k = key_of(&env, "libre_baskerville.ttf");
    svc.activate_faces(&env.store, &[k], Some(scope), ConflictPolicy::Ask, &mut no_progress()).unwrap();
    let old = canon(&env.fonts.join("libre_baskerville.ttf"));
    let org = env.dir.path().join("org");
    std::fs::create_dir_all(&org).unwrap();
    std::fs::copy(&old, org.join("libre_baskerville.ttf")).unwrap();
    {
        let mut s = env.store.lock().unwrap();
        let id = s.add_source(SourceKind::Directory, "org", Some(&org)).unwrap();
        s.rescan_source(id).unwrap();
    }
    let new = canon(&org.join("libre_baskerville.ttf"));
    (env, svc, k, old, new)
}

fn regs(svc: &ActivationService<FakeEngine>) -> Vec<(PathBuf, Scope)> {
    svc.engine().registrations()
}

#[test]
fn moves_the_registration_to_the_organized_copy_registering_new_first() {
    let (env, svc, k, old, new) = setup(Scope::Session);
    let calls: Arc<Mutex<Vec<(String, PathBuf)>>> = Arc::default();
    {
        let c = calls.clone();
        svc.engine().on_call(move |op, p| c.lock().unwrap().push((op.to_string(), p.to_path_buf())));
    }
    let rep = svc.relocate(&env.store, &[k.hash], Some(&env.dir.path().join("org")), &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated);
    assert_eq!(regs(&svc), vec![(new.clone(), Scope::Session)], "chỉ còn đường mới");
    let row = env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap();
    assert_eq!(row.active_path.as_deref(), Some(new.to_str().unwrap()));
    assert_eq!(row.state, DesiredState::Active);
    let c = calls.lock().unwrap();
    let reg_new = c.iter().position(|(o, p)| o == "register" && *p == new).expect("register new");
    let unreg_old = c.iter().position(|(o, p)| o == "unregister" && *p == old).expect("unregister old");
    assert!(reg_new < unreg_old, "đăng ký đường mới TRƯỚC khi gỡ đường cũ: {c:?}");
    assert!(env.store.lock().unwrap().setting(RELOCATE_PENDING_KEY).unwrap().unwrap_or_default().is_empty(), "sổ chờ gỡ đã sạch");
}

#[test]
fn persistent_scope_is_preserved() {
    let (env, svc, k, _old, new) = setup(Scope::Persistent);
    svc.relocate(&env.store, &[k.hash], Some(&env.dir.path().join("org")), &mut no_progress()).unwrap();
    assert_eq!(regs(&svc), vec![(new, Scope::Persistent)]);
    assert_eq!(env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap().scope, ActivationScope::Persistent);
}

#[test]
fn second_call_is_a_no_op() {
    let (env, svc, k, _old, _new) = setup(Scope::Session);
    let org = env.dir.path().join("org");
    svc.relocate(&env.store, &[k.hash], Some(&org), &mut no_progress()).unwrap();
    let rep = svc.relocate(&env.store, &[k.hash], Some(&org), &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::AlreadyActive);
    assert_eq!(regs(&svc).len(), 1);
}

#[test]
fn no_copy_under_the_preferred_root_changes_nothing() {
    let (env, svc, k, old, _new) = setup(Scope::Session);
    let elsewhere = env.dir.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let rep = svc.relocate(&env.store, &[k.hash], Some(&elsewhere), &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::MissingFile);
    assert_eq!(regs(&svc), vec![(old, Scope::Session)], "đường cũ vẫn đăng ký");
}

#[test]
fn a_failed_new_registration_keeps_the_old_path_untouched() {
    let (env, svc, k, old, new) = setup(Scope::Session);
    svc.engine().fail_register_of(&new);
    let rep = svc.relocate(&env.store, &[k.hash], Some(&env.dir.path().join("org")), &mut no_progress()).unwrap();
    assert!(matches!(rep.results[0].outcome, FileOutcome::Failed { .. }), "{:?}", rep.results[0].outcome);
    assert_eq!(regs(&svc), vec![(old.clone(), Scope::Session)]);
    let row = env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap();
    assert_eq!(row.active_path.as_deref(), Some(old.to_str().unwrap()));
    assert_eq!(row.state, DesiredState::Active);
}

#[test]
fn a_corrupted_copy_is_refused_by_hash() {
    let (env, svc, k, old, _new) = setup(Scope::Session);
    let org = env.dir.path().join("org");
    let mut b = std::fs::read(org.join("libre_baskerville.ttf")).unwrap();
    b.push(0);
    std::fs::write(org.join("libre_baskerville.ttf"), b).unwrap();
    let rep = svc.relocate(&env.store, &[k.hash], Some(&org), &mut no_progress()).unwrap();
    assert!(matches!(&rep.results[0].outcome, FileOutcome::Failed { code, .. } if code == "content_changed"));
    assert_eq!(regs(&svc), vec![(old, Scope::Session)]);
}

#[test]
fn a_failed_unregister_of_the_old_path_is_warned_journaled_and_finished_by_reconcile() {
    let (env, svc, k, old, new) = setup(Scope::Persistent);
    svc.engine().fail_next_unregister(EngineError::Rejected { code: 1, message: "boom".into() });
    let rep = svc.relocate(&env.store, &[k.hash], Some(&env.dir.path().join("org")), &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::Activated);
    assert!(rep.results[0].warning.is_some());
    assert_eq!(regs(&svc).len(), 2, "cả hai còn đăng ký tạm thời (cùng nội dung)");
    assert!(env.store.lock().unwrap().setting(RELOCATE_PENDING_KEY).unwrap().unwrap().contains(old.to_str().unwrap()));
    // lần mở sau
    svc.reconcile(&env.store, &mut |_| {}).unwrap();
    assert_eq!(regs(&svc), vec![(new, Scope::Persistent)], "reconcile gỡ nốt đường cũ");
    assert!(env.store.lock().unwrap().setting(RELOCATE_PENDING_KEY).unwrap().unwrap_or_default().is_empty());
}

#[test]
fn a_crash_between_the_two_steps_leaves_no_orphan_after_reconcile() {
    let (env, svc, k, old, new) = setup(Scope::Persistent);
    // mô phỏng: đường mới đã đăng ký, DB đã trỏ đường mới, sổ ghi đường cũ, nhưng chưa kịp gỡ đường cũ
    svc.engine().register(&new, Scope::Persistent).unwrap();
    {
        let mut s = env.store.lock().unwrap();
        let mut row = s.activation_row(&k.hash).unwrap().unwrap();
        row.active_path = Some(new.to_str().unwrap().to_string());
        s.put_activation(&row).unwrap();
        s.set_setting(RELOCATE_PENDING_KEY, &format!("[{{\"path\":{:?},\"persistent\":true}}]", old.to_str().unwrap())).unwrap();
    }
    assert_eq!(regs(&svc).len(), 2);
    svc.reconcile(&env.store, &mut |_| {}).unwrap();
    assert_eq!(regs(&svc), vec![(new, Scope::Persistent)]);
}

#[test]
fn pending_entry_that_became_the_active_path_again_is_not_unregistered() {
    let (env, svc, k, old, _new) = setup(Scope::Session);
    env.store.lock().unwrap().set_setting(RELOCATE_PENDING_KEY, &format!("[{{\"path\":{:?},\"persistent\":false}}]", old.to_str().unwrap())).unwrap();
    svc.reconcile(&env.store, &mut |_| {}).unwrap();
    assert_eq!(regs(&svc), vec![(old, Scope::Session)], "đường đang dùng không bị gỡ nhầm");
    let _ = k;
}

#[test]
fn inactive_files_are_skipped() {
    let (env, svc, k, old, _new) = setup(Scope::Session);
    svc.deactivate_faces(&env.store, &[k], fontatlas_activate::DeactivatePolicy::Ask, &mut no_progress()).unwrap();
    let rep = svc.relocate(&env.store, &[k.hash], Some(&env.dir.path().join("org")), &mut no_progress()).unwrap();
    assert_eq!(rep.results[0].outcome, FileOutcome::AlreadyInactive);
    assert!(regs(&svc).is_empty());
    let _ = (old, Path::new(""));
}

#[test]
fn reconcile_keeps_a_still_valid_registered_path_instead_of_hopping_to_a_new_copy() {
    let (env, svc, k, old, _new) = setup(Scope::Persistent);
    let rep = svc.reconcile(&env.store, &mut |_| {}).unwrap();
    assert_eq!(rep.moved, 0, "thêm bản chép không được âm thầm đổi đường của font đang bật");
    assert_eq!(regs(&svc), vec![(old.clone(), Scope::Persistent)]);
    assert_eq!(env.store.lock().unwrap().activation_row(&k.hash).unwrap().unwrap().active_path.as_deref(), Some(old.to_str().unwrap()));
}
