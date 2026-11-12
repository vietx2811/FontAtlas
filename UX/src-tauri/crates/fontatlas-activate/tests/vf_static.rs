//! G7: font variable (VF) và static trùng tên PostScript. `Conflict` nói rõ bên nào là VF để giao diện hỏi
//! "dùng VF / dùng static / giữ cả hai"; các lựa chọn dùng lại Ask / Replace / Proceed có sẵn (không tự thay).
mod common;
use common::*;

use fontatlas_activate::{ActivationEngine, ConflictPolicy, FakeEngine, FileOutcome, Owner, Scope, is_variable_font};
use fontatlas_store::FaceKey;

const PS: &str = "Oswald-Regular";

#[test]
fn is_variable_font_reads_the_fvar_table_only_from_real_fonts() {
    assert_eq!(is_variable_font(&fixture("oswald_vf.ttf")), Some(true));
    assert_eq!(is_variable_font(&fixture("libre_baskerville.ttf")), Some(false));
    assert_eq!(is_variable_font(&fixture("khong-co.ttf")), None, "thiếu file: không biết");
    let d = tempfile::tempdir().unwrap();
    let junk = d.path().join("x.ttf");
    std::fs::write(&junk, b"this is not a font at all").unwrap();
    assert_eq!(is_variable_font(&junk), None, "không phải sfnt: không biết, không phải 'static'");
    std::fs::write(&junk, b"abc").unwrap();
    assert_eq!(is_variable_font(&junk), None, "file quá ngắn");
}

/// Hai file trong thư viện cùng PostScript: `vf.ttf` (variable thật) và `st.ttf` (static).
fn pair() -> (Env, fontatlas_activate::ActivationService<FakeEngine>, FaceKey, FaceKey) {
    let env = env_with(&[]);
    let vf = ingest_fake_from(&env, "vf.ttf", "vf", &[PS], "oswald_vf.ttf");
    let st = ingest_fake(&env, "st.ttf", "st", &[PS]);
    let svc = service_with(&env, FakeEngine::new());
    (env, svc, FaceKey { hash: vf, face_index: 0 }, FaceKey { hash: st, face_index: 0 })
}

fn go(svc: &fontatlas_activate::ActivationService<FakeEngine>, env: &Env, k: FaceKey, p: ConflictPolicy) -> fontatlas_activate::FileResult {
    svc.activate_faces(&env.store, &[k], None, p, &mut |_| {}).unwrap().results.remove(0)
}

fn conflicts(r: &fontatlas_activate::FileResult) -> Vec<fontatlas_activate::Conflict> {
    match &r.outcome {
        FileOutcome::NeedsConfirmation { conflicts } => conflicts.clone(),
        o => panic!("cần NeedsConfirmation, được {o:?}"),
    }
}

#[test]
fn enabling_the_static_while_the_vf_is_on_reports_which_side_is_variable() {
    let (env, svc, vf, st) = pair();
    assert_eq!(go(&svc, &env, vf, ConflictPolicy::Ask).outcome, FileOutcome::Activated);
    let c = conflicts(&go(&svc, &env, st, ConflictPolicy::Ask));
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].owner, c[0].variable, c[0].existing_variable), (Owner::FontAtlas, false, Some(true)));
    assert_eq!(c[0].postscript_name, PS);
}

#[test]
fn enabling_the_vf_while_the_static_is_on_is_the_mirror_case() {
    let (env, svc, vf, st) = pair();
    assert_eq!(go(&svc, &env, st, ConflictPolicy::Ask).outcome, FileOutcome::Activated);
    let c = conflicts(&go(&svc, &env, vf, ConflictPolicy::Ask));
    assert_eq!((c[0].variable, c[0].existing_variable), (true, Some(false)));
}

#[test]
fn a_foreign_vf_is_reported_too() {
    let env = env_with(&[]);
    let st = ingest_fake(&env, "st.ttf", "st", &[PS]);
    let foreign = env.dir.path().join("Lib.rightfontlibrary/fonts/Oswald[wght].ttf");
    std::fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    std::fs::copy(fixture("oswald_vf.ttf"), &foreign).unwrap();
    let foreign = canon(&foreign);
    let svc = service_with(&env, FakeEngine::new());
    svc.engine().set_names_for(&foreign, &[PS]);
    svc.engine().register(&foreign, Scope::Persistent).unwrap();
    let c = conflicts(&go(&svc, &env, FaceKey { hash: st, face_index: 0 }, ConflictPolicy::Ask));
    assert_eq!((c[0].owner, c[0].variable, c[0].existing_variable), (Owner::RightFont, false, Some(true)));
}

#[test]
fn two_statics_are_a_plain_conflict_not_a_pair() {
    let env = env_with(&[]);
    let a = ingest_fake(&env, "a.ttf", "a", &[PS]);
    let b = ingest_fake(&env, "b.ttf", "b", &[PS]);
    let svc = service_with(&env, FakeEngine::new());
    assert_eq!(go(&svc, &env, FaceKey { hash: a, face_index: 0 }, ConflictPolicy::Ask).outcome, FileOutcome::Activated);
    let c = conflicts(&go(&svc, &env, FaceKey { hash: b, face_index: 0 }, ConflictPolicy::Ask));
    assert_eq!((c[0].variable, c[0].existing_variable), (false, Some(false)), "cùng loại: không phải cặp VF/static");
}

#[test]
fn use_new_one_replaces_ours_and_keep_both_registers_both_and_ask_never_changes_anything() {
    // "dùng bản mới" = Replace: tắt bản của ta kia rồi bật bản này
    let (env, svc, vf, st) = pair();
    go(&svc, &env, vf, ConflictPolicy::Ask);
    let before = svc.engine().registrations();
    assert!(matches!(go(&svc, &env, st, ConflictPolicy::Ask).outcome, FileOutcome::NeedsConfirmation { .. }));
    assert_eq!(svc.engine().registrations(), before, "Ask không đổi gì");
    let confirmed: Vec<_> = vec![fontatlas_activate::ConfirmedConflict { hash: st.hash, existing_path: conflicts(&go(&svc, &env, st, ConflictPolicy::Ask))[0].existing_path.clone() }];
    let r = svc.replace_faces(&env.store, &[st], None, &confirmed, &mut |_| {}).unwrap().results.remove(0);
    assert_eq!(r.outcome, FileOutcome::Activated, "{:?}", r.outcome);
    let regs = svc.engine().registrations();
    assert_eq!(regs.len(), 1, "chỉ còn bản static");
    assert_eq!(regs[0].0, canon(&env.dir.path().join("extra/st.ttf")));

    // "giữ cả hai" = Proceed
    let (env, svc, vf, st) = pair();
    go(&svc, &env, vf, ConflictPolicy::Ask);
    assert_eq!(go(&svc, &env, st, ConflictPolicy::Proceed).outcome, FileOutcome::Activated);
    assert_eq!(svc.engine().registrations().len(), 2);
}
