mod common;
use common::*;
use fontatlas_activate::{FakeEngine, Scope};
use fontatlas_broker::{Backend, EnsureError, EnsureRequest, FontReq, FontStatus, MAX_FONTS};
use fontatlas_store::{ActivationReason, ActivationScope, AutoApp, DesiredState};

fn ps(n: &str) -> FontReq {
    FontReq { ps: Some(n.into()), ..Default::default() }
}

fn fs(f: &str, s: &str) -> FontReq {
    FontReq { family: Some(f.into()), style: Some(s.into()), ..Default::default() }
}

fn req(app: &str, fonts: Vec<FontReq>) -> EnsureRequest {
    EnsureRequest { v: 1, app: app.into(), doc: Some("poster.psd".into()), fonts }
}

fn status(r: &fontatlas_broker::EnsureResponse, i: usize) -> FontStatus {
    r.results[i].status
}

#[test]
fn postscript_match_activates_session_with_auto_reason() {
    let (env, b) = backend(&[LIBRE, ROBOTO]);
    let r = b.ensure(&req("Adobe Photoshop 2026", vec![ps(LIBRE_PS)])).unwrap();
    assert!(r.ok);
    assert_eq!(status(&r, 0), FontStatus::Activated);
    assert_eq!(r.found, vec![LIBRE_PS.to_string()]);
    assert!(r.missing.is_empty());
    // chỉ font được hỏi mới bật, scope session
    assert_eq!(b.service.engine().registrations(), vec![(canon(&env.fonts.join(LIBRE)), Scope::Session)]);
    let s = b.store.lock().unwrap();
    let rows = s.activation_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reasons, vec![ActivationReason::Auto(AutoApp::Photoshop)]);
    assert_eq!(rows[0].scope, ActivationScope::Session);
    assert_eq!(rows[0].state, DesiredState::Active);
    let log = s.recent_auto_log(10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!((log[0].app.as_str(), log[0].outcome.as_str()), ("photoshop", "activated"));
    assert_eq!(log[0].doc.as_deref(), Some("poster.psd"));
}

#[test]
fn family_style_fallback_and_label() {
    let (_env, b) = backend(&[LIBRE]);
    let r = b.ensure(&req("illustrator", vec![fs("Libre Baskerville", "Regular"), fs("Libre Baskerville", "Bold")])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::Activated);
    assert_eq!(status(&r, 1), FontStatus::Missing, "style không có → thiếu, không đoán gần đúng");
    assert_eq!(r.found, vec!["Libre Baskerville / Regular".to_string()]);
    assert_eq!(r.missing, vec!["Libre Baskerville / Bold".to_string()]);
}

#[test]
fn ps_wins_then_falls_back_to_family_when_ps_unknown() {
    let (_env, b) = backend(&[LIBRE]);
    let both = FontReq { ps: Some("Wrong-Name".into()), family: Some("Libre Baskerville".into()), style: None };
    let r = b.ensure(&req("indesign", vec![both])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::Activated);
    assert_eq!(r.results[0].name, "Wrong-Name");
}

#[test]
fn unknown_fonts_are_missing_and_logged_once() {
    let (_env, b) = backend(&[LIBRE]);
    for _ in 0..3 {
        let r = b.ensure(&req("photoshop", vec![ps("Nope-Regular"), ps("")])).unwrap();
        assert_eq!(status(&r, 0), FontStatus::Missing);
        assert_eq!(status(&r, 1), FontStatus::Invalid);
        assert_eq!(r.missing.len(), 2);
    }
    assert!(b.service.engine().registrations().is_empty());
    let log = b.store.lock().unwrap().recent_auto_log(10).unwrap();
    assert_eq!(log.len(), 1, "hỏi lặp lại không làm đầy nhật ký");
    assert_eq!(log[0].outcome, "missing");
}

#[test]
fn repeat_request_is_already_active_and_not_logged_again() {
    let (_env, b) = backend(&[LIBRE]);
    b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::AlreadyActive);
    assert_eq!(r.found.len(), 1);
    assert_eq!(b.service.engine().registrations().len(), 1);
    assert_eq!(b.store.lock().unwrap().recent_auto_log(10).unwrap().len(), 1);
}

#[test]
fn second_app_adds_its_own_reason() {
    let (_env, b) = backend(&[LIBRE]);
    b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    let r = b.ensure(&req("Adobe Illustrator", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::AlreadyActive);
    let rows = b.store.lock().unwrap().activation_rows().unwrap();
    assert_eq!(rows[0].reasons, vec![ActivationReason::Auto(AutoApp::Photoshop), ActivationReason::Auto(AutoApp::Illustrator)]);
}

#[test]
fn duplicate_names_in_one_request_register_once() {
    let (_env, b) = backend(&[LIBRE]);
    let r = b.ensure(&req("sketch", vec![ps(LIBRE_PS), ps(LIBRE_PS), fs("Libre Baskerville", "Regular")])).unwrap();
    assert_eq!(r.results.len(), 3);
    assert!(r.results.iter().all(|x| x.status.usable()));
    assert_eq!(b.service.engine().registrations().len(), 1);
}

#[test]
fn different_system_font_is_conflict_not_overwritten() {
    let other = tempfile::tempdir().unwrap();
    let theirs = other.path().join("Other.ttf");
    std::fs::write(&theirs, b"khac noi dung").unwrap();
    let (_env, b) = backend_with(&[LIBRE], FakeEngine::new().with_system_font(LIBRE_PS, &theirs));
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::Conflict);
    assert_eq!(r.missing, vec![LIBRE_PS.to_string()]);
    assert!(b.service.engine().registrations().is_empty(), "không tự ghi đè font của bên khác");
}

#[test]
fn identical_system_font_is_available_without_registering() {
    let other = tempfile::tempdir().unwrap();
    let theirs = other.path().join("Same.ttf");
    std::fs::copy(fixture(LIBRE), &theirs).unwrap();
    let (_env, b) = backend_with(&[LIBRE], FakeEngine::new().with_system_font(LIBRE_PS, &theirs));
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::Available);
    assert_eq!(r.found.len(), 1);
    assert!(b.service.engine().registrations().is_empty());
    assert!(b.store.lock().unwrap().recent_auto_log(10).unwrap().is_empty(), "đã có sẵn: không vào nhật ký");
}

#[test]
fn file_deleted_from_disk_is_unavailable() {
    let (env, b) = backend(&[LIBRE]);
    std::fs::remove_file(env.fonts.join(LIBRE)).unwrap();
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::Unavailable);
    assert!(b.service.engine().registrations().is_empty());
}

#[test]
fn persistent_manual_activation_keeps_scope_and_gains_reason() {
    let (env, b) = backend(&[LIBRE]);
    let k = {
        let s = b.store.lock().unwrap();
        s.faces_by_postscript(LIBRE_PS).unwrap().remove(0)
    };
    b.service
        .activate_faces(&b.store, &[k], Some(Scope::Persistent), Default::default(), &mut |_| {})
        .unwrap();
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::AlreadyActive);
    assert_eq!(b.service.engine().registrations(), vec![(canon(&env.fonts.join(LIBRE)), Scope::Persistent)]);
    let rows = b.store.lock().unwrap().activation_rows().unwrap();
    assert_eq!(rows[0].scope, ActivationScope::Persistent);
    assert_eq!(rows[0].reasons, vec![ActivationReason::Manual, ActivationReason::Auto(AutoApp::Photoshop)]);
}

#[test]
fn manual_deactivate_wins_over_auto_reason() {
    let (_env, b) = backend(&[LIBRE]);
    b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    let k = b.store.lock().unwrap().faces_by_postscript(LIBRE_PS).unwrap().remove(0);
    let rep = b
        .service
        .deactivate_faces(&b.store, &[k], Default::default(), &mut |_| {})
        .unwrap();
    assert!(matches!(rep.results[0].outcome, fontatlas_activate::FileOutcome::Deactivated), "{:?}", rep.results[0].outcome);
    assert!(b.service.engine().registrations().is_empty());
    let rows = b.store.lock().unwrap().activation_rows().unwrap();
    assert!(rows[0].reasons.is_empty());
}

#[test]
fn rejects_bad_requests() {
    let (_env, b) = backend(&[LIBRE]);
    let bad_v = EnsureRequest { v: 2, ..req("photoshop", vec![]) };
    assert!(matches!(b.ensure(&bad_v), Err(EnsureError::BadRequest(_))));
    let many = req("photoshop", (0..=MAX_FONTS).map(|i| ps(&format!("F{i}"))).collect());
    assert!(matches!(b.ensure(&many), Err(EnsureError::BadRequest(_))));
    let ctl = req("photoshop", vec![ps("Evil\nName")]);
    assert!(matches!(b.ensure(&ctl), Err(EnsureError::BadRequest(_))));
    let long = req("photoshop", vec![ps(&"x".repeat(600))]);
    assert!(matches!(b.ensure(&long), Err(EnsureError::BadRequest(_))));
    let ok_max = req("photoshop", (0..MAX_FONTS).map(|i| ps(&format!("F{i}"))).collect());
    assert_eq!(b.ensure(&ok_max).unwrap().missing.len(), MAX_FONTS);
}

#[test]
fn unknown_app_is_other_and_empty_list_is_ok() {
    let (_env, b) = backend(&[LIBRE]);
    let r = b.ensure(&req("'; DROP TABLE x;--", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::Activated);
    let rows = b.store.lock().unwrap().activation_rows().unwrap();
    assert_eq!(rows[0].reasons, vec![ActivationReason::Auto(AutoApp::Other)]);
    let e = b.ensure(&req("photoshop", vec![])).unwrap();
    assert!(e.found.is_empty() && e.missing.is_empty());
}

#[test]
fn setting_zero_disables() {
    let (_env, b) = backend(&[LIBRE]);
    assert!(b.enabled());
    b.store.lock().unwrap().set_setting(fontatlas_broker::SETTING_AUTO_ACTIVATION, "0").unwrap();
    assert!(!b.enabled());
    b.store.lock().unwrap().set_setting(fontatlas_broker::SETTING_AUTO_ACTIVATION, "1").unwrap();
    assert!(b.enabled());
}

#[test]
fn usable_results_carry_the_file_path_and_others_do_not() {
    let (env, b) = backend(&[LIBRE]);
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS), ps("Nope-Regular")])).unwrap();
    let want = env.fonts.join(LIBRE).to_string_lossy().into_owned();
    assert_eq!(status(&r, 0), FontStatus::Activated);
    assert!(r.results[0].path.as_deref().is_some_and(|p| p.ends_with(&want) || canon(&env.fonts.join(LIBRE)).to_string_lossy() == p), "{:?}", r.results[0].path);
    assert_eq!(r.results[1].path, None);
    // lần hỏi lại (already_active) vẫn có đường dẫn: helper cần nó mỗi lần Photoshop hỏi
    let again = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&again, 0), FontStatus::AlreadyActive);
    assert!(again.results[0].path.is_some());
    // đường dẫn trên đĩa KHÔNG ra HTTP
    let json = serde_json::to_value(&r).unwrap();
    assert!(json["results"][0].get("path").is_none(), "{json}");
}

#[test]
fn internal_names_are_ignored_for_ps_and_family_and_never_logged() {
    let (_env, b) = backend(&[LIBRE]);
    let fam = FontReq { family: Some("AdobeSansMM".into()), style: Some("Regular".into()), ..Default::default() };
    let sys = FontReq { ps: Some(".SFNS-Regular".into()), ..Default::default() };
    let r = b.ensure(&req("photoshop", vec![ps("AdobeSansMM"), fam, sys])).unwrap();
    assert!(r.results.iter().all(|x| x.status == FontStatus::Missing));
    assert!(b.store.lock().unwrap().recent_auto_log(10).unwrap().is_empty(), "tên nội bộ không làm đầy nhật ký");
    assert!(b.service.engine().registrations().is_empty());
    // ps nội bộ + family thật → vẫn tìm theo family
    let mixed = FontReq { ps: Some("AdobeSansMM".into()), family: Some("Libre Baskerville".into()), style: Some("Regular".into()) };
    assert_eq!(status(&b.ensure(&req("photoshop", vec![mixed])).unwrap(), 0), FontStatus::Activated);
}

fn add_variant(env: &Env, b: &fontatlas_broker::OwnedBackend<fontatlas_activate::FakeEngine>, name: &str, pad: usize) {
    let mut bytes = std::fs::read(fixture(LIBRE)).unwrap();
    bytes.extend(std::iter::repeat_n(0u8, pad)); // cùng tên font, khác hash
    std::fs::write(env.fonts.join(name), bytes).unwrap();
    let mut s = b.store.lock().unwrap();
    let src = s.sources().unwrap()[0].id;
    s.rescan_source(src).unwrap();
    teach(b.service.engine(), &s);
}

fn newest_of(b: &fontatlas_broker::OwnedBackend<fontatlas_activate::FakeEngine>) -> fontatlas_store::FaceKey {
    let s = b.store.lock().unwrap();
    let mut v = s.faces_by_postscript(LIBRE_PS).unwrap();
    v.sort_by_key(|k| (std::cmp::Reverse(s.file_first_seen(&k.hash).unwrap()), k.hash));
    v[0]
}

#[test]
fn several_versions_with_one_name_pick_the_newest_and_log_ambiguous_with_the_count() {
    let (env, b) = backend(&[LIBRE]);
    add_variant(&env, &b, "libre_v2.ttf", 4);
    add_variant(&env, &b, "libre_v3.ttf", 8);
    assert_eq!(b.store.lock().unwrap().faces_by_postscript(LIBRE_PS).unwrap().len(), 3);
    let newest = newest_of(&b);
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::Activated);
    let rows = b.store.lock().unwrap().activation_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].hash, newest.hash);
    // hỏi lại → vẫn đúng bản đó (ổn định), nhật ký ghi `ambiguous ×3` đúng một lần (dedupe)
    b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    let log = b.store.lock().unwrap().recent_auto_log(20).unwrap();
    let amb: Vec<_> = log.iter().filter(|l| l.outcome == "ambiguous").collect();
    assert_eq!(amb.len(), 1, "{log:?}");
    assert_eq!(amb[0].requested, format!("{LIBRE_PS} ×3"));
    assert_eq!(b.store.lock().unwrap().activation_rows().unwrap()[0].hash, newest.hash);
}

#[test]
fn the_version_the_user_already_activated_wins_over_a_newer_one() {
    let (env, b) = backend(&[LIBRE]);
    let first = b.store.lock().unwrap().faces_by_postscript(LIBRE_PS).unwrap()[0];
    add_variant(&env, &b, "libre_v2.ttf", 4);
    add_variant(&env, &b, "libre_v3.ttf", 8);
    assert_ne!(newest_of(&b).hash, first.hash, "bản đầu không phải bản mới nhất");
    b.service.activate_faces(&b.store, &[first], Some(Scope::Persistent), Default::default(), &mut |_| {}).unwrap();
    let r = b.ensure(&req("photoshop", vec![ps(LIBRE_PS)])).unwrap();
    assert_eq!(status(&r, 0), FontStatus::AlreadyActive);
    let rows = b.store.lock().unwrap().activation_rows().unwrap();
    assert_eq!(rows.len(), 1, "không bật thêm bản thứ hai");
    assert_eq!(rows[0].hash, first.hash);
}
