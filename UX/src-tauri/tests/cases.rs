use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::cases::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, cases, library, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

const FILES: [&str; 3] = ["roboto.ttf", "playfair_display.ttf", "open_sans.ttf"];

fn state(root: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for f in FILES {
        std::fs::copy(fixture(f), dir.join(f)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

fn key_of(st: &Arc<AppState>, text: &str) -> FaceKeyDto {
    let r = library::query_faces(st, &FaceQueryDto { text: Some(text.into()), ..Default::default() }).unwrap().items.remove(0);
    FaceKeyDto { hash_hex: r.hash_hex, face_index: r.face_index }
}

#[test]
fn case_lifecycle_through_services() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let roboto = key_of(&st, "roboto");
    let playfair = key_of(&st, "playfair");
    let sans = key_of(&st, "open sans");

    // Add to Case khi chưa có Case nào → tạo mới; lần sau dùng lại Case đang mở
    let a = cases::add_to_case(&st, None, std::slice::from_ref(&roboto), None).unwrap();
    assert!(a.created && a.added == 1);
    let b = cases::add_to_case(&st, None, &[roboto.clone(), playfair.clone()], Some(CaseOriginDto::Pinned)).unwrap();
    assert!(!b.created);
    assert_eq!((b.case_id, b.added, b.already_in), (a.case_id, 1, 1));

    let c = cases::get_case(&st, a.case_id).unwrap();
    assert_eq!(c.name, "Case 1");
    assert_eq!(c.faces.len(), 2);
    assert!(c.faces.iter().all(|f| f.row.is_some()), "có hàng thư viện cho từng font");
    assert_eq!(c.faces.iter().find(|f| f.key == playfair).unwrap().origin, CaseOriginDto::Pinned);
    assert!(c.settings.vi, "mặc định tính tiếng Việt");

    // Combo + vai
    let m = cases::create_combo(&st, c.id, None, None).unwrap();
    assert!(m.is_main && m.name == "Combo 1");
    let m = cases::set_combo_slot(&st, m.id, ComboRoleDto::Heading, Some(&playfair), Some("neo"), Some(&serde_json::json!({"weight": 700}))).unwrap();
    let m = cases::set_combo_slot(&st, m.id, ComboRoleDto::Body, Some(&sans), None, None).unwrap();
    assert_eq!(m.slots.len(), 2);
    assert_eq!(m.slots[0].role, ComboRoleDto::Heading);
    assert_eq!(m.slots[0].style["weight"], 700);
    assert!(m.slots.iter().all(|s| s.row.is_some()));
    let c = cases::get_case(&st, c.id).unwrap();
    assert_eq!(c.faces.len(), 3, "Open Sans vào Case khi đặt vào vai");

    // create_combos batch + lỗi giữ nguyên trạng thái
    let made = cases::create_combos(
        &st,
        c.id,
        &[NewComboDto { name: Some("Lạnh".into()), slots: vec![NewSlotDto { role: ComboRoleDto::Display, face: roboto.clone() }] }],
    )
    .unwrap();
    assert_eq!(made.len(), 1);
    let bad_face = FaceKeyDto { hash_hex: "zz".into(), face_index: 0 };
    assert!(cases::create_combos(&st, c.id, &[NewComboDto { name: None, slots: vec![NewSlotDto { role: ComboRoleDto::Body, face: bad_face }] }]).is_err());
    assert_eq!(cases::get_case(&st, c.id).unwrap().combos.len(), 2);

    // cập nhật Case
    let upd = cases::update_case(
        &st,
        c.id,
        &CasePatchDto {
            name: Some("Poster bia".into()),
            brief: Some(BriefDto { uses: vec![BriefUseDto::Poster], ..Default::default() }),
            view_state: Some(serde_json::json!({"mode": "compare", "pinned": [1]})),
            settings: Some(CaseSettingsDto { vi: false, mode: Some(PairingLevelDto::Controlled) }),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(upd.name, "Poster bia");
    assert_eq!(upd.brief.unwrap().uses, vec![BriefUseDto::Poster]);
    assert_eq!(upd.view_state["pinned"][0], 1);
    assert_eq!(upd.settings, CaseSettingsDto { vi: false, mode: Some(PairingLevelDto::Controlled) });
    let cleared = cases::update_case(&st, c.id, &CasePatchDto { clear_brief: true, ..Default::default() }).unwrap();
    assert!(cleared.brief.is_none() && cleared.name == "Poster bia");

    // list + xoá mềm + khôi phục
    let other = cases::create_case(&st, Some("Menu"), None, None).unwrap();
    assert_eq!(cases::list_cases(&st, false).unwrap().len(), 2);
    cases::delete_case(&st, other.id).unwrap();
    assert_eq!(cases::list_cases(&st, false).unwrap().len(), 1);
    assert_eq!(cases::list_cases(&st, true).unwrap().len(), 2);
    assert!(cases::get_case(&st, other.id).is_err());
    assert_eq!(cases::restore_case(&st, other.id).unwrap().id, other.id);
    assert_eq!(cases::open_case(&st, c.id).unwrap().id, c.id);
    assert_eq!(cases::add_to_case(&st, None, std::slice::from_ref(&sans), None).unwrap().case_id, c.id, "Case vừa mở là Case mặc định");

    // bỏ font khỏi Case gỡ luôn vai
    assert_eq!(cases::remove_from_case(&st, c.id, std::slice::from_ref(&playfair)).unwrap(), 1);
    let after = cases::get_case(&st, c.id).unwrap();
    assert!(after.combos[0].slots.iter().all(|s| s.key != playfair));

    // lỗi
    assert_eq!(cases::get_case(&st, 9999).unwrap_err().code, "not_found");
    assert_eq!(cases::create_case(&st, Some(" "), None, None).unwrap_err().code, "invalid_arg");
}

#[test]
fn missing_font_stays_in_case_without_row() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let ghost = FaceKeyDto { hash_hex: "ab".repeat(32), face_index: 0 };
    let r = cases::add_to_case(&st, None, std::slice::from_ref(&ghost), None).unwrap();
    let c = cases::get_case(&st, r.case_id).unwrap();
    assert_eq!(c.faces.len(), 1);
    assert!(c.faces[0].row.is_none(), "font không có trong thư viện: giữ khoá, không có hàng");
}

#[test]
fn case_compare_columns_relations_and_limits() {
    use app_lib::dto::analysis::LangDto;
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let (roboto, playfair, sans) = (key_of(&st, "roboto"), key_of(&st, "playfair"), key_of(&st, "open sans"));
    let c = cases::create_case(&st, Some("Cmp"), None, None).unwrap();
    let made = cases::create_combos(
        &st,
        c.id,
        &[
            NewComboDto {
                name: Some("Ba giọng".into()),
                slots: vec![
                    NewSlotDto { role: ComboRoleDto::Body, face: roboto.clone() },
                    NewSlotDto { role: ComboRoleDto::Heading, face: playfair.clone() },
                    NewSlotDto { role: ComboRoleDto::Caption, face: sans.clone() },
                ],
            },
            NewComboDto { name: Some("Một font".into()), slots: vec![NewSlotDto { role: ComboRoleDto::Heading, face: roboto.clone() }, NewSlotDto { role: ComboRoleDto::Body, face: roboto.clone() }] },
            NewComboDto { name: Some("Trống".into()), slots: vec![] },
        ],
    )
    .unwrap();
    let cmp = cases::case_compare(&st, c.id, &[], LangDto::Vi).unwrap();
    assert_eq!(cmp.columns.len(), 3);
    let a = &cmp.columns[0];
    assert_eq!(a.font_count, 3);
    assert_eq!(a.relations.len(), 2, "giọng ① (heading) so với body và caption");
    assert!(a.relations.iter().all(|r| r.from == ComboRoleDto::Heading));
    assert_eq!(a.relations.iter().map(|r| r.to).collect::<Vec<_>>(), vec![ComboRoleDto::Body, ComboRoleDto::Caption]);
    assert!(a.relations.iter().all(|r| !r.assessment.labels.is_empty() && !r.assessment.summary.is_empty()));
    assert!(a.worst_vi_missing.is_some());
    assert!(!a.licenses.is_empty());
    assert_eq!(cmp.columns[1].font_count, 1);
    assert!(cmp.columns[1].relations.is_empty(), "cùng một font: không có quan hệ giữa các giọng");
    assert_eq!((cmp.columns[2].font_count, cmp.columns[2].worst_vi_missing), (0, None));

    // chọn và thứ tự theo combo_ids
    let two = cases::case_compare(&st, c.id, &[made[1].id, made[0].id], LangDto::En).unwrap();
    assert_eq!(two.columns.iter().map(|x| x.combo.id).collect::<Vec<_>>(), vec![made[1].id, made[0].id]);
    let other = cases::create_case(&st, Some("Khác"), None, None).unwrap();
    let foreign = cases::create_combo(&st, other.id, None, None).unwrap();
    assert_eq!(cases::case_compare(&st, c.id, &[foreign.id], LangDto::Vi).unwrap_err().code, "not_found");
    for _ in 0..8 {
        cases::create_combo(&st, c.id, None, None).unwrap();
    }
    assert_eq!(cases::case_compare(&st, c.id, &[], LangDto::Vi).unwrap_err().code, "invalid_arg", ">8 combo");
}

#[test]
fn pair_from_peek_anchors_font_and_carries_settings_and_trial() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let (roboto, playfair, sans) = (key_of(&st, "roboto"), key_of(&st, "playfair"), key_of(&st, "open sans"));
    let settings = CaseSettingsDto { vi: false, mode: Some(PairingLevelDto::Experimental) };
    let r = cases::open_case_from_peek(
        &st,
        &PeekOpenDto { face: playfair.clone(), as_role: PeekRoleDto::Heading, candidate: Some(roboto.clone()), case_id: None, settings: settings.clone() },
    )
    .unwrap();
    assert!(r.created);
    assert_eq!((r.anchor_role, r.other_role), (ComboRoleDto::Display, ComboRoleDto::Body));
    assert_eq!(r.trial.as_ref(), Some(&roboto));
    assert_eq!(r.case.settings, settings, "mang theo mức ghép và lựa chọn tiếng Việt");
    let combo = r.case.combos.iter().find(|c| c.id == r.combo_id).unwrap();
    assert!(combo.is_main);
    assert_eq!(combo.slots.len(), 1, "ứng viên KHÔNG được đặt vào vai");
    assert_eq!((combo.slots[0].role, &combo.slots[0].key), (ComboRoleDto::Display, &playfair));
    let origins: Vec<_> = r.case.faces.iter().map(|f| (f.key.clone(), f.origin)).collect();
    assert!(origins.contains(&(playfair.clone(), CaseOriginDto::Peek)) && origins.contains(&(roboto.clone(), CaseOriginDto::Peek)));

    // lần hai: dùng Case đang mở, font là nội dung → vai body trong cùng Combo chính; không nhân đôi Case/Combo
    let r2 = cases::open_case_from_peek(
        &st,
        &PeekOpenDto { face: sans.clone(), as_role: PeekRoleDto::Body, candidate: None, case_id: None, settings: CaseSettingsDto::default() },
    )
    .unwrap();
    assert!(!r2.created);
    assert_eq!(r2.case.id, r.case.id);
    assert_eq!(r2.case.combos.len(), 1);
    assert_eq!(r2.combo_id, r.combo_id);
    assert_eq!(r2.case.combos[0].slots.len(), 2);
    assert_eq!((r2.anchor_role, r2.other_role, r2.trial), (ComboRoleDto::Body, ComboRoleDto::Display, None));
    assert!(r2.case.settings.vi);
    // case_id không tồn tại
    let bad = cases::open_case_from_peek(&st, &PeekOpenDto { face: sans, as_role: PeekRoleDto::Body, candidate: None, case_id: Some(9999), settings: CaseSettingsDto::default() });
    assert_eq!(bad.unwrap_err().code, "not_found");
}
