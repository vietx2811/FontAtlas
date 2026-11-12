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
    let m = cases::set_combo_slot(&st, m.id, "heading".to_string(), Some(&playfair), Some("neo"), Some(&serde_json::json!({"weight": 700}))).unwrap();
    let m = cases::set_combo_slot(&st, m.id, "body".to_string(), Some(&sans), None, None).unwrap();
    assert_eq!(m.slots.len(), 2);
    assert_eq!(m.slots[0].role, "heading");
    assert_eq!(m.slots[0].style["weight"], 700);
    assert!(m.slots.iter().all(|s| s.row.is_some()));
    let c = cases::get_case(&st, c.id).unwrap();
    assert_eq!(c.faces.len(), 3, "Open Sans vào Case khi đặt vào vai");

    // create_combos batch + lỗi giữ nguyên trạng thái
    let made = cases::create_combos(
        &st,
        c.id,
        &[NewComboDto { name: Some("Lạnh".into()), slots: vec![NewSlotDto { role: "display".to_string(), face: roboto.clone() }] }],
    )
    .unwrap();
    assert_eq!(made.len(), 1);
    let bad_face = FaceKeyDto { hash_hex: "zz".into(), face_index: 0 };
    assert!(cases::create_combos(&st, c.id, &[NewComboDto { name: None, slots: vec![NewSlotDto { role: "body".to_string(), face: bad_face }] }]).is_err());
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
                    NewSlotDto { role: "body".to_string(), face: roboto.clone() },
                    NewSlotDto { role: "heading".to_string(), face: playfair.clone() },
                    NewSlotDto { role: "caption".to_string(), face: sans.clone() },
                ],
            },
            NewComboDto { name: Some("Một font".into()), slots: vec![NewSlotDto { role: "heading".to_string(), face: roboto.clone() }, NewSlotDto { role: "body".to_string(), face: roboto.clone() }] },
            NewComboDto { name: Some("Trống".into()), slots: vec![] },
        ],
    )
    .unwrap();
    let cmp = cases::case_compare(&st, c.id, &[], LangDto::Vi).unwrap();
    assert_eq!(cmp.columns.len(), 3);
    let a = &cmp.columns[0];
    assert_eq!(a.font_count, 3);
    assert_eq!(a.relations.len(), 2, "giọng ① (heading) so với body và caption");
    assert!(a.relations.iter().all(|r| r.from == "heading"));
    assert_eq!(a.relations.iter().map(|r| r.to.clone()).collect::<Vec<_>>(), vec!["body".to_string(), "caption".to_string()]);
    assert!(a.relations.iter().all(|r| r.reason.is_none() && r.assessment.as_ref().is_some_and(|x| !x.labels.is_empty() && !x.summary.is_empty())));
    assert_eq!(a.missing_count, 0);
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
        &PeekOpenDto { face: playfair.clone(), as_role: PeekRoleDto::Heading, candidate: Some(roboto.clone()), case_id: None, settings: settings.clone(), replace: false },
    )
    .unwrap();
    assert!(r.created);
    assert_eq!((r.anchor_role.as_str(), r.other_role.as_str()), ("display", "body"));
    assert_eq!(r.trial.as_ref(), Some(&roboto));
    assert_eq!(r.case.settings, settings, "mang theo mức ghép và lựa chọn tiếng Việt");
    let combo = r.case.combos.iter().find(|c| c.id == r.combo_id).unwrap();
    assert!(combo.is_main);
    assert_eq!(combo.slots.len(), 1, "ứng viên KHÔNG được đặt vào vai");
    assert_eq!((combo.slots[0].role.as_str(), &combo.slots[0].key), ("display", &playfair));
    let origins: Vec<_> = r.case.faces.iter().map(|f| (f.key.clone(), f.origin)).collect();
    assert!(origins.contains(&(playfair.clone(), CaseOriginDto::Peek)) && origins.contains(&(roboto.clone(), CaseOriginDto::Peek)));

    // lần hai: dùng Case đang mở, font là nội dung → vai body trong cùng Combo chính; không nhân đôi Case/Combo
    let r2 = cases::open_case_from_peek(
        &st,
        &PeekOpenDto { face: sans.clone(), as_role: PeekRoleDto::Body, candidate: None, case_id: None, settings: CaseSettingsDto::default(), replace: false },
    )
    .unwrap();
    assert!(!r2.created);
    assert_eq!(r2.case.id, r.case.id);
    assert_eq!(r2.case.combos.len(), 1);
    assert_eq!(r2.combo_id, r.combo_id);
    assert_eq!(r2.case.combos[0].slots.len(), 2);
    assert_eq!((r2.anchor_role.as_str(), r2.other_role.as_str(), r2.trial), ("body", "display", None));
    assert_eq!(r2.case.settings, settings, "Case có sẵn giữ cài đặt, peek không ghi đè");
    // case_id không tồn tại
    let bad = cases::open_case_from_peek(&st, &PeekOpenDto { face: sans, as_role: PeekRoleDto::Body, candidate: None, case_id: Some(9999), settings: CaseSettingsDto::default(), replace: false });
    assert_eq!(bad.unwrap_err().code, "not_found");
}

fn settle_langs(st: &Arc<AppState>) {
    app_lib::services::langs::ensure_started(st);
    let t = std::time::Instant::now();
    while !app_lib::services::langs::coverage_complete(st) || st.langs.is_running() {
        assert!(t.elapsed() < Duration::from_secs(60), "backfill không xong");
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn brief(uses: Vec<BriefUseDto>, tones: Vec<ToneDto>) -> BriefDto {
    BriefDto { uses, tones, ..Default::default() }
}

#[test]
fn brief_starter_maps_chips_to_filters_anchors_and_combos() {
    use app_lib::services::brief;
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle_langs(&st);
    let b = brief(vec![BriefUseDto::Editorial], vec![ToneDto { pair: TonePairDto::ClassicModern, value: -0.9 }]);
    let r = brief::brief_starter(&st, &b, None).unwrap();
    let ids: Vec<&str> = r.describe.matched.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, vec!["classic", "editorial"], "tone trước, rồi mục đích dùng");
    assert!(!r.describe.filters.is_empty());
    assert!(r.query.axis_filters.iter().any(|f| f.axis == "serif_foot"));
    // vi mặc định bật: ràng buộc cứng + ngôn ngữ Việt + bộ lọc hỗ trợ
    assert!(r.query.hard.contains(&"vi_supported".to_string()));
    assert_eq!(r.query.langs.len(), 1);
    assert_eq!(r.query.langs[0].code, "vie");
    assert_eq!(r.query.vi, Some(ViFilterDto::Supported));
    // truy vấn theo vai: trọng lượng và khớp trục Variable
    assert_eq!(r.display_query.weight_class, Some([600, 900]));
    assert_eq!(r.body_query.weight_class, Some([400, 500]));
    assert!(r.display_query.var_axis_match.wght && r.query.weight_class.is_none());
    // neo và Combo
    assert!(r.anchors.len() <= 3);
    let fams: std::collections::HashSet<_> = r.anchors.iter().map(|a| a.family.to_lowercase()).collect();
    assert_eq!(fams.len(), r.anchors.len(), "mỗi neo một family");
    assert!(r.combos.len() <= r.anchors.len());
    for (i, c) in r.combos.iter().enumerate() {
        assert_eq!(c.rank as usize, i + 1);
        assert!(r.anchors.iter().any(|a| a.hash_hex == c.heading.hash_hex));
        assert!(!c.assessment.summary.is_empty());
    }
    let bodies: std::collections::HashSet<_> = r.combos.iter().map(|c| c.body.family.clone()).collect();
    assert_eq!(bodies.len(), r.combos.len(), "không lặp font body giữa các Combo");
    assert!(r.ai.is_none());
}

#[test]
fn brief_starter_edge_cases_are_explicit_not_guessed() {
    use app_lib::services::brief;
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle_langs(&st);
    let note_codes = |r: &BriefStarterDto| r.notes.iter().map(|n| n.code.clone()).collect::<Vec<_>>();

    // packaging: suy ra theo vai từ term có sẵn, và nói rõ đó là suy ra
    let r = brief::brief_starter(&st, &brief(vec![BriefUseDto::Packaging], vec![]), None).unwrap();
    assert!(r.describe.unmappable.is_empty(), "không còn trả mục packaging không ánh xạ được");
    let inferred = r.notes.iter().find(|n| n.code == "packaging_inferred").expect("ghi chú suy ra");
    assert_eq!(inferred.detail.as_deref(), Some("display=display+heavy; body=readable_small+clean"));
    // điều kiện chung không có term; từng vai có bộ lọc trục của term suy ra
    assert!(r.query.axis_filters.is_empty());
    let axes = |q: &FaceQueryDto| q.axis_filters.iter().map(|f| f.axis.clone()).collect::<std::collections::BTreeSet<_>>();
    assert!(axes(&r.display_query).contains("darkness"), "heavy → darkness");
    assert!(axes(&r.body_query).contains("x_height_em"), "readable_small → x_height_em");
    assert!(!axes(&r.body_query).contains("darkness"));
    assert!(!note_codes(&r).contains(&"no_terms".to_string()));

    // branding một mình: chỉ class, không term
    let r = brief::brief_starter(&st, &brief(vec![BriefUseDto::Branding], vec![]), None).unwrap();
    assert!(note_codes(&r).contains(&"branding_class_only".to_string()));
    assert!(note_codes(&r).contains(&"no_terms".to_string()));

    // brief trống: vẫn trả neo từ cả thư viện + ghi chú
    let r = brief::brief_starter(&st, &BriefDto::default(), None).unwrap();
    assert!(note_codes(&r).contains(&"no_terms".to_string()));
    assert!(!r.anchors.is_empty(), "thư viện 3 font: có neo");

    // thanh trượt trong vùng chết: bỏ qua
    let r = brief::brief_starter(&st, &brief(vec![], vec![ToneDto { pair: TonePairDto::WarmCool, value: 0.1 }]), None).unwrap();
    assert!(r.describe.matched.is_empty());

    // hai đầu đối nghịch (một từ thanh trượt, một từ khoá tự do): xung đột, không tự chọn
    let b = BriefDto { tones: vec![ToneDto { pair: TonePairDto::WarmCool, value: -1.0 }], text: Some("cool".into()), ..Default::default() };
    let r = brief::brief_starter(&st, &b, None).unwrap();
    assert!(!r.describe.conflicts.is_empty());
    let conflicted: Vec<&str> = r.describe.conflicts.iter().map(|c| c.axis.as_str()).collect();
    assert!(r.query.axis_filters.iter().all(|f| !conflicted.contains(&f.axis.as_str())));

    // chỉ font đang bật: chưa bật font nào → ghi chú, không áp điều kiện
    let b = BriefDto { needs: BriefNeedsDto { installed_only: true, ..Default::default() }, ..Default::default() };
    let r = brief::brief_starter(&st, &b, None).unwrap();
    assert!(note_codes(&r).contains(&"installed_none_active".to_string()));
    assert!(r.query.face_keys.is_none());

    // ràng buộc cờ: variable/italic/license mở
    let b = BriefDto { needs: BriefNeedsDto { vi: false, variable: true, italic: true, open_license: true, ..Default::default() }, ..Default::default() };
    let r = brief::brief_starter(&st, &b, None).unwrap();
    assert_eq!((r.query.variable, r.query.italic), (Some(true), Some(true)));
    assert_eq!(r.query.license, vec!["ofl".to_string(), "apache".to_string()]);
    assert!(r.query.hard.is_empty() && r.query.langs.is_empty() && r.query.vi.is_none());
}

#[test]
fn brief_starter_on_empty_library_returns_notes_not_errors() {
    use app_lib::services::brief;
    let td = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(&td.path().join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let b = brief(vec![BriefUseDto::Poster], vec![ToneDto { pair: TonePairDto::QuietBold, value: 0.8 }]);
    let r = brief::brief_starter(&st, &b, None).unwrap();
    assert!(r.anchors.is_empty() && r.combos.is_empty());
    assert!(r.notes.iter().any(|n| n.code == "few_anchors" || n.code == "index_unavailable"));
}

#[test]
fn brief_starter_warns_when_language_coverage_is_still_computing() {
    use app_lib::services::brief;
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    st.langs.shutdown(); // dừng luồng nền, rồi đánh dấu mọi face là chưa tính
    st.with_store(|s| {
        s.conn().execute_batch("DELETE FROM face_lang_state; DELETE FROM face_langs;").unwrap();
        Ok(())
    })
    .unwrap();
    let r = brief::brief_starter(&st, &BriefDto::default(), None).unwrap();
    assert!(r.notes.iter().any(|n| n.code == "langs_incomplete"));
    let no_vi = BriefDto { needs: BriefNeedsDto { vi: false, ..Default::default() }, ..Default::default() };
    assert!(!brief::brief_starter(&st, &no_vi, None).unwrap().notes.iter().any(|n| n.code == "langs_incomplete"), "không cần tiếng Việt thì không cảnh báo");
}

#[test]
fn open_case_from_peek_is_idempotent() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let (roboto, playfair) = (key_of(&st, "roboto"), key_of(&st, "playfair"));
    let req = PeekOpenDto { face: playfair.clone(), as_role: PeekRoleDto::Heading, candidate: Some(roboto.clone()), case_id: None, settings: CaseSettingsDto::default(), replace: false };
    let first = cases::open_case_from_peek(&st, &req).unwrap();
    let second = cases::open_case_from_peek(&st, &req).unwrap();
    let third = cases::open_case_from_peek(&st, &PeekOpenDto { case_id: Some(first.case.id), ..req.clone() }).unwrap();
    assert!(first.created && !second.created && !third.created);
    for r in [&second, &third] {
        assert_eq!(r.case.id, first.case.id);
        assert_eq!(r.combo_id, first.combo_id);
        assert_eq!(r.case.combos.len(), 1, "bấm nhiều lần không tạo Combo trùng");
        assert_eq!(r.case.combos[0].slots.len(), 1);
        assert_eq!(r.case.faces.len(), 2);
        assert_eq!(r.trial, first.trial);
    }
    assert_eq!(cases::list_cases(&st, false).unwrap().len(), 1, "cũng không tạo Case trùng");
}

#[test]
fn brief_starter_is_deterministic() {
    use app_lib::services::brief;
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    settle_langs(&st);
    let briefs = [
        BriefDto::default(),
        brief(vec![BriefUseDto::Editorial], vec![ToneDto { pair: TonePairDto::ClassicModern, value: -0.9 }]),
        brief(vec![BriefUseDto::Packaging, BriefUseDto::Poster], vec![ToneDto { pair: TonePairDto::QuietBold, value: 0.5 }]),
        BriefDto { text: Some("warm friendly".into()), needs: BriefNeedsDto { vi: false, ..Default::default() }, ..Default::default() },
    ];
    for b in &briefs {
        let a = serde_json::to_value(brief::brief_starter(&st, b, None).unwrap()).unwrap();
        for _ in 0..3 {
            assert_eq!(serde_json::to_value(brief::brief_starter(&st, b, None).unwrap()).unwrap(), a, "cùng brief + cùng thư viện ⇒ cùng kết quả");
        }
    }
}

#[test]
fn trash_is_purged_at_boot_not_by_list_cases() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let old = cases::create_case(&st, Some("Cũ"), None, None).unwrap();
    let fresh = cases::create_case(&st, Some("Mới xoá"), None, None).unwrap();
    cases::delete_case(&st, old.id).unwrap();
    cases::delete_case(&st, fresh.id).unwrap();
    // đẩy mốc xoá của Case "Cũ" về 31 ngày trước
    st.with_user(|u| {
        u.conn().execute("UPDATE u.cases SET deleted_at = deleted_at - ?1 WHERE id = ?2", rusqlite::params![31_i64 * 24 * 3600 * 1000, old.id]).unwrap();
        Ok(())
    })
    .unwrap();
    // lệnh đọc không có tác dụng phụ
    assert_eq!(cases::list_cases(&st, true).unwrap().len(), 2);
    assert_eq!(cases::list_cases(&st, true).unwrap().len(), 2);
    // purge chạy ở khởi động
    assert_eq!(cases::purge_trash(&st), 1);
    let left = cases::list_cases(&st, true).unwrap();
    assert_eq!(left.iter().map(|c| c.id).collect::<Vec<_>>(), vec![fresh.id]);
    assert!(cases::restore_case(&st, fresh.id).is_ok(), "Case xoá gần đây vẫn khôi phục được");
}

#[test]
fn case_compare_reports_missing_fonts_instead_of_swallowing_them() {
    use app_lib::dto::analysis::LangDto;
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let (roboto, sans, playfair) = (key_of(&st, "roboto"), key_of(&st, "open sans"), key_of(&st, "playfair"));
    let ghost = FaceKeyDto { hash_hex: "cd".repeat(32), face_index: 0 };
    let c = cases::create_case(&st, Some("Thiếu"), None, None).unwrap();
    cases::create_combos(
        &st,
        c.id,
        &[
            NewComboDto { name: Some("Giữa thiếu".into()), slots: vec![
                NewSlotDto { role: "heading".to_string(), face: playfair.clone() },
                NewSlotDto { role: "body".to_string(), face: ghost.clone() },
                NewSlotDto { role: "caption".to_string(), face: sans.clone() },
            ] },
            NewComboDto { name: Some("Neo thiếu".into()), slots: vec![
                NewSlotDto { role: "display".to_string(), face: ghost.clone() },
                NewSlotDto { role: "body".to_string(), face: roboto.clone() },
                NewSlotDto { role: "caption".to_string(), face: sans.clone() },
            ] },
        ],
    )
    .unwrap();
    let cmp = cases::case_compare(&st, c.id, &[], LangDto::Vi).unwrap();
    let a = &cmp.columns[0];
    assert_eq!(a.missing_count, 1, "một vai thiếu font — không được báo sạch");
    assert_eq!(a.relations.len(), 2);
    let body = a.relations.iter().find(|r| r.to == "body").unwrap();
    assert!(body.assessment.is_none() && body.reason.as_deref() == Some("missing_font"));
    assert!(a.relations.iter().find(|r| r.to == "caption").unwrap().assessment.is_some());
    assert!(a.combo.slots.iter().any(|s| s.missing) && a.combo.slots.iter().filter(|s| s.missing).count() == 1);
    // neo thiếu: lấy vai kế tiếp còn font làm neo, vẫn trả quan hệ còn lại
    let b = &cmp.columns[1];
    assert_eq!(b.missing_count, 1);
    let anchor_role = b.relations.iter().map(|r| r.from.clone()).next().unwrap();
    assert_eq!(anchor_role, "body", "neo = vai đầu còn font");
    assert!(b.relations.iter().any(|r| r.to == "caption" && r.assessment.is_some()));
    assert!(b.relations.iter().any(|r| r.to == "display" && r.reason.as_deref() == Some("missing_font")));
    // CaseFaceDto cũng báo missing
    let full = cases::get_case(&st, c.id).unwrap();
    assert!(full.faces.iter().find(|f| f.key == ghost).unwrap().missing);
    assert!(!full.faces.iter().find(|f| f.key == sans).unwrap().missing);
}

#[test]
fn peek_does_not_overwrite_an_occupied_role_without_replace() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let (roboto, playfair) = (key_of(&st, "roboto"), key_of(&st, "playfair"));
    let req = |face: &FaceKeyDto, replace: bool| PeekOpenDto { face: face.clone(), as_role: PeekRoleDto::Heading, candidate: None, case_id: None, settings: CaseSettingsDto::default(), replace };
    let r1 = cases::open_case_from_peek(&st, &req(&playfair, false)).unwrap();
    assert!(r1.occupied.is_none());
    let r2 = cases::open_case_from_peek(&st, &req(&roboto, false)).unwrap();
    let occ = r2.occupied.clone().expect("vai display đã có Playfair");
    assert_eq!((occ.role.as_str(), &occ.key), ("display", &playfair));
    assert!(occ.row.is_some());
    let slot = |r: &PeekOpenResultDto| r.case.combos[0].slots.iter().find(|s| s.role == "display").unwrap().key.clone();
    assert_eq!(slot(&r2), playfair, "chưa đồng ý ⇒ không đổi");
    let r3 = cases::open_case_from_peek(&st, &req(&roboto, true)).unwrap();
    assert!(r3.occupied.is_none());
    assert_eq!(slot(&r3), roboto);
}

#[test]
fn concurrent_add_to_case_without_id_creates_one_case() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let keys: Vec<FaceKeyDto> = vec![key_of(&st, "roboto"), key_of(&st, "playfair"), key_of(&st, "open sans")];
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let (st, k) = (Arc::clone(&st), keys[i % 3].clone());
            std::thread::spawn(move || cases::add_to_case(&st, None, &[k], None).unwrap())
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(cases::list_cases(&st, true).unwrap().len(), 1, "hai lệnh song song không tạo hai Case");
    assert_eq!(results.iter().filter(|r| r.created).count(), 1);
    assert!(results.iter().all(|r| r.case_id == results[0].case_id));
    assert_eq!(cases::get_case(&st, results[0].case_id).unwrap().faces.len(), 3);
}

#[test]
fn brief_starter_with_unreadable_case_is_an_error_not_a_silent_fallback() {
    use app_lib::services::brief;
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    assert_eq!(brief::brief_starter(&st, &BriefDto::default(), Some(9999)).unwrap_err().code, "not_found");
    let c = cases::create_case(&st, None, None, None).unwrap();
    cases::delete_case(&st, c.id).unwrap();
    assert_eq!(brief::brief_starter(&st, &BriefDto::default(), Some(c.id)).unwrap_err().code, "not_found");
}

#[test]
fn custom_combo_roles_through_services_carry_codes_and_order() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let roboto = key_of(&st, "roboto");
    let playfair = key_of(&st, "playfair");
    let c = cases::create_case(&st, Some("G3"), None, None).unwrap();
    let m = cases::create_combo(&st, c.id, None, None).unwrap();
    assert!(m.roles.is_empty());
    let m = cases::set_combo_slot(&st, m.id, "Label".to_string(), Some(&roboto), None, None).unwrap();
    assert_eq!((m.roles.clone(), m.slots.len()), (vec!["Label".to_string()], 1));
    let m = cases::add_combo_role(&st, m.id, "Numerals").unwrap();
    assert_eq!(m.roles, vec!["Label", "Numerals"]);
    assert_eq!(cases::add_combo_role(&st, m.id, "label").unwrap_err().code, "role_duplicate");
    assert_eq!(cases::add_combo_role(&st, m.id, " ").unwrap_err().code, "role_name_invalid");
    assert_eq!(cases::remove_combo_role(&st, m.id, "zzz").unwrap_err().code, "role_not_found");
    let m = cases::set_combo_slot(&st, m.id, "numerals".to_string(), Some(&playfair), None, None).unwrap();
    assert_eq!(m.slots.iter().map(|s| (s.role.as_str(), s.row.is_some())).collect::<Vec<_>>(), vec![("Label", true), ("Numerals", true)]);
    let m = cases::reorder_combo_roles(&st, m.id, &["Numerals".to_string(), "Label".to_string()]).unwrap();
    assert_eq!(m.slots[0].role, "Numerals");
    assert_eq!(cases::reorder_combo_roles(&st, m.id, &["Label".to_string()]).unwrap_err().code, "role_set_mismatch");
    let m = cases::rename_combo_role(&st, m.id, "Label", "Nhãn").unwrap();
    assert_eq!(m.roles, vec!["Numerals", "Nhãn"]);
    let m = cases::remove_combo_role(&st, m.id, "Numerals").unwrap();
    assert_eq!((m.roles, m.slots.len()), (vec!["Nhãn".to_string()], 1));
}
