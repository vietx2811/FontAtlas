//! "Tắt bản cũ và bật bản này": gỡ bản do bên khác đăng ký qua CoreText rồi bật bản của Font Atlas, có sổ để khôi phục.
//! Chỉ dùng FakeEngine + file tạm; không đụng font/thư mục font thật.
mod common;
use common::*;

use std::path::{Path, PathBuf};

use fontatlas_activate::{ActivationEngine, ConflictPolicy, EngineError, FakeEngine, FileOutcome, Owner, ReplaceTier, RestoreOutcome, Scope, replace_tier};
use fontatlas_store::FaceKey;

const PS: &str = "Dup-Regular";

fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap()
}

/// Thư viện có font `a` (PS = Dup-Regular) và một bản KHÁC nội dung do bên khác cung cấp ở `rel`.
fn setup(rel: &str, engine: FakeEngine) -> (Env, fontatlas_activate::ActivationService<FakeEngine>, FaceKey, PathBuf) {
    let env = env_with(&[]);
    let a = ingest_fake(&env, "a.ttf", "file-a", &[PS]);
    let foreign = env.dir.path().join(rel);
    std::fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    std::fs::write(&foreign, format!("noi dung khac cua {rel}")).unwrap();
    let foreign = canon(&foreign);
    let svc = service_with(&env, engine);
    svc.engine().set_names_for(&foreign, &[PS]);
    (env, svc, FaceKey { hash: a, face_index: 0 }, foreign)
}

fn run(svc: &fontatlas_activate::ActivationService<FakeEngine>, env: &Env, k: FaceKey, p: ConflictPolicy) -> fontatlas_activate::FileResult {
    svc.activate_faces(&env.store, &[k], None, p, &mut |_| {}).unwrap().results.remove(0)
}

const RIGHT: &str = "Thu vien.rightfontlibrary/fonts/dup.ttf";

#[test]
fn ask_still_asks_and_changes_nothing() {
    let (env, svc, k, foreign) = setup(RIGHT, FakeEngine::new());
    svc.engine().register(&foreign, Scope::Persistent).unwrap();
    let r = run(&svc, &env, k, ConflictPolicy::Ask);
    assert!(matches!(r.outcome, FileOutcome::NeedsConfirmation { .. }), "{:?}", r.outcome);
    assert_eq!(svc.engine().registrations(), vec![(foreign, Scope::Persistent)]);
    assert!(env.store.lock().unwrap().unrestored_replaced().unwrap().is_empty());
}

#[test]
fn replace_removes_the_foreign_persistent_registration_enables_ours_and_journals_it() {
    let (env, svc, k, foreign) = setup(RIGHT, FakeEngine::new());
    svc.engine().register(&foreign, Scope::Persistent).unwrap();
    let r = run(&svc, &env, k, ConflictPolicy::Replace);
    assert_eq!(r.outcome, FileOutcome::Activated);
    let ours = canon(&env.dir.path().join("extra/a.ttf"));
    assert_eq!(svc.engine().registrations(), vec![(ours, Scope::Session)], "bản cũ đã gỡ, chỉ còn bản của ta");
    assert_eq!(r.replaced.len(), 1);
    let rep = &r.replaced[0];
    assert_eq!((rep.path.as_str(), rep.owner, rep.scopes.clone()), (foreign.to_str().unwrap(), Owner::RightFont, vec![Scope::Persistent]));
    let j = env.store.lock().unwrap().unrestored_replaced().unwrap();
    assert_eq!(j.len(), 1);
    assert_eq!((j[0].id, j[0].old_path.as_str(), j[0].owner.as_str()), (rep.id, foreign.to_str().unwrap(), "right_font"));
    assert_eq!(j[0].hash, k.hash);
    assert_eq!(j[0].postscript_names, vec![PS.to_string()]);
}

#[test]
fn only_the_scope_that_really_held_it_is_recorded() {
    let (env, svc, k, foreign) = setup("Cho khac/dup.ttf", FakeEngine::new());
    svc.engine().register(&foreign, Scope::Session).unwrap();
    let r = run(&svc, &env, k, ConflictPolicy::Replace);
    assert_eq!(r.outcome, FileOutcome::Activated);
    assert_eq!(r.replaced[0].scopes, vec![Scope::Session], "gỡ persistent trả 201 là bình thường, không ghi vào sổ");
    assert_eq!(r.replaced[0].owner, Owner::Other);
}

#[test]
fn restore_gives_the_old_font_back_with_the_same_url_and_scope_and_turns_ours_off() {
    let (env, svc, k, foreign) = setup(RIGHT, FakeEngine::new());
    svc.engine().register(&foreign, Scope::Persistent).unwrap();
    let id = run(&svc, &env, k, ConflictPolicy::Replace).replaced[0].id;
    assert_eq!(svc.restore_replaced(&env.store, id).unwrap(), RestoreOutcome::Restored);
    assert_eq!(svc.engine().registrations(), vec![(foreign.clone(), Scope::Persistent)], "bản cũ về đúng URL + scope; bản của ta tắt");
    assert!(env.store.lock().unwrap().unrestored_replaced().unwrap().is_empty());
    assert_eq!(svc.restore_replaced(&env.store, id).unwrap(), RestoreOutcome::AlreadyRestored);
    assert_eq!(svc.restore_replaced(&env.store, 9999).unwrap_err().code(), "not_found");
}

#[test]
fn restore_when_the_old_file_is_gone_reports_it_and_keeps_the_record() {
    let (env, svc, k, foreign) = setup(RIGHT, FakeEngine::new());
    svc.engine().register(&foreign, Scope::Persistent).unwrap();
    let id = run(&svc, &env, k, ConflictPolicy::Replace).replaced[0].id;
    std::fs::remove_file(&foreign).unwrap();
    assert_eq!(svc.restore_replaced(&env.store, id).unwrap(), RestoreOutcome::MissingFile);
    assert_eq!(env.store.lock().unwrap().unrestored_replaced().unwrap().len(), 1, "chưa khôi phục được thì giữ sổ");
}

#[test]
fn system_font_is_never_touched() {
    let engine = FakeEngine::new().with_system_font(PS, Path::new("/System/Library/Fonts/Supplemental/Dup.ttf"));
    let (env, svc, k, _foreign) = setup(RIGHT, engine);
    let r = run(&svc, &env, k, ConflictPolicy::Replace);
    assert!(matches!(&r.outcome, FileOutcome::Failed { code, .. } if code == "replace_protected"), "{:?}", r.outcome);
    assert!(svc.engine().registrations().is_empty());
    assert!(env.store.lock().unwrap().unrestored_replaced().unwrap().is_empty());
}

#[test]
fn adobe_fonts_only_get_guidance() {
    let (env, svc, k, adobe) = setup("Adobe/CoreSync/plugins/livetype/.r/.1.otf", FakeEngine::new());
    svc.engine().register(&adobe, Scope::Session).unwrap();
    let r = run(&svc, &env, k, ConflictPolicy::Replace);
    assert!(matches!(&r.outcome, FileOutcome::Failed { code, .. } if code == "replace_guide_only"), "{:?}", r.outcome);
    assert_eq!(svc.engine().registrations(), vec![(adobe, Scope::Session)], "không đụng");
}

#[test]
fn a_provider_that_is_not_an_api_registration_cannot_be_removed() {
    // có trong providers (như font thư mục) nhưng gỡ cả hai scope đều 201
    let engine = FakeEngine::new();
    let (env, svc, k, foreign) = setup("Cho khac/dup.ttf", engine);
    let sys = FakeEngine::new().with_system_font(PS, &foreign);
    let svc2 = service_with(&env, sys);
    svc2.engine().set_names_for(&foreign, &[PS]);
    let r = run(&svc2, &env, k, ConflictPolicy::Replace);
    assert!(matches!(&r.outcome, FileOutcome::Failed { code, .. } if code == "replace_not_registered"), "{:?}", r.outcome);
    assert!(svc2.engine().registrations().is_empty());
    drop(svc);
}

#[test]
fn if_the_second_foreign_copy_cannot_be_removed_the_first_is_given_back() {
    let (env, svc, k, first) = setup("Cho khac/dup1.ttf", FakeEngine::new());
    let second = canon(&{
        let p = env.dir.path().join("Cho khac/dup2.ttf");
        std::fs::write(&p, b"khac nua").unwrap();
        p
    });
    svc.engine().set_names_for(&second, &[PS]);
    svc.engine().register(&first, Scope::Session).unwrap();
    // bản thứ hai chỉ "có sẵn" (không gỡ được): dựng engine mới cho cả hai
    let engine = FakeEngine::new().with_system_font(PS, &second);
    let svc = service_with(&env, engine);
    svc.engine().set_names_for(&first, &[PS]);
    svc.engine().register(&first, Scope::Session).unwrap();
    let r = run(&svc, &env, k, ConflictPolicy::Replace);
    assert!(matches!(&r.outcome, FileOutcome::Failed { code, .. } if code == "replace_not_registered"), "{:?}", r.outcome);
    assert_eq!(svc.engine().registrations(), vec![(first, Scope::Session)], "đã gỡ bản đầu thì phải trả lại");
    assert!(env.store.lock().unwrap().unrestored_replaced().unwrap().is_empty());
}

#[test]
fn if_ours_cannot_be_registered_the_old_font_is_given_back_and_nothing_is_journalled() {
    let (env, svc, k, foreign) = setup(RIGHT, FakeEngine::new());
    svc.engine().register(&foreign, Scope::Persistent).unwrap();
    svc.engine().fail_next_register(EngineError::Rejected { code: 7, message: "từ chối".into() });
    let r = run(&svc, &env, k, ConflictPolicy::Replace);
    assert!(matches!(&r.outcome, FileOutcome::Failed { code, .. } if code.starts_with("engine.")), "{:?}", r.outcome);
    assert_eq!(svc.engine().registrations(), vec![(foreign, Scope::Persistent)], "bản cũ được trả lại đúng scope");
    assert!(env.store.lock().unwrap().unrestored_replaced().unwrap().is_empty());
    assert!(r.replaced.is_empty());
}

#[test]
fn nothing_to_replace_activates_normally() {
    let env = env_with(&[]);
    let a = ingest_fake(&env, "a.ttf", "file-a", &[PS]);
    let svc = service(&env);
    let r = run(&svc, &env, FaceKey { hash: a, face_index: 0 }, ConflictPolicy::Replace);
    assert_eq!(r.outcome, FileOutcome::Activated);
    assert!(r.replaced.is_empty());
}

#[test]
fn replace_tiers_follow_the_matrix() {
    let home = Path::new("/Users/u");
    let p = |s: &str| Path::new(s).to_path_buf();
    let t = |o: Owner, s: &str| replace_tier(o, &p(s), home);
    assert_eq!(t(Owner::System, "/System/Library/Fonts/A.ttf"), ReplaceTier::Protected);
    assert_eq!(t(Owner::System, "/Library/Fonts/A.ttf"), ReplaceTier::Protected);
    assert_eq!(t(Owner::UserInstalled, "/Users/u/Library/Fonts/A.ttf"), ReplaceTier::Protected);
    // đường dẫn trong thư mục font luôn bị bảo vệ, bất kể phân loại
    assert_eq!(t(Owner::Other, "/Users/u/Library/Fonts/A.ttf"), ReplaceTier::Protected);
    assert_eq!(t(Owner::RightFont, "/Library/Fonts/A.ttf"), ReplaceTier::Protected);
    assert_eq!(t(Owner::AdobeFonts, "/x/Adobe/CoreSync/plugins/livetype/.r/.1.otf"), ReplaceTier::Guide);
    assert_eq!(t(Owner::Suitcase, "/x/Suitcase Fusion/a.ttf"), ReplaceTier::Guide);
    for o in [Owner::FontAtlas, Owner::RightFont, Owner::FontBase, Owner::Typeface, Owner::Other] {
        assert_eq!(t(o, "/Volumes/X/lib/A.ttf"), ReplaceTier::Replace, "{o:?}");
    }
}

#[test]
fn owners_of_typeface_and_suitcase_are_guessed_from_the_path() {
    use fontatlas_activate::classify_owner;
    use std::collections::HashSet;
    let home = Path::new("/Users/u");
    let none = HashSet::new();
    assert_eq!(classify_owner(Path::new("/Users/u/Typeface/Library/a.ttf"), home, &none), Owner::Typeface);
    assert_eq!(classify_owner(Path::new("/Users/u/Library/Application Support/Typeface/fonts/a.ttf"), home, &none), Owner::Typeface);
    assert_eq!(classify_owner(Path::new("/Users/u/Library/Application Support/Extensis/Suitcase Fusion/a.ttf"), home, &none), Owner::Suitcase);
    assert_eq!(classify_owner(Path::new("/Volumes/Fonts/Suitcase Fusion 2024/a.ttf"), home, &none), Owner::Suitcase);
    assert_eq!(classify_owner(Path::new("/Users/u/Dropbox/a.ttf"), home, &none), Owner::Other);
}
