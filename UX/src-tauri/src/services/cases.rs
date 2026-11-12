//! Case và Combo: gom font, ghép vai, mở lại đúng chỗ đã dừng.
use std::collections::HashMap;

use fontatlas_store::{CaseFaceRow, CasePatch, CaseRow, ComboPatch, ComboRow, FaceKey, NewCombo};
use serde_json::Value;

use crate::dto::cases::*;
use crate::dto::library::{FaceKeyDto, FaceQueryDto, FaceRowDto};
use crate::error::CmdError;
use crate::state::AppState;

fn keys_of(faces: &[FaceKeyDto]) -> Result<Vec<FaceKey>, CmdError> {
    faces.iter().map(FaceKey::try_from).collect()
}

fn json_of<T: serde::Serialize>(v: &T) -> Result<String, CmdError> {
    serde_json::to_string(v).map_err(|e| CmdError::invalid_arg(e.to_string()))
}

fn settings_of(raw: &str) -> CaseSettingsDto {
    serde_json::from_str(raw).unwrap_or_default()
}

/// Hàng thư viện cho các khoá (mất file khỏi mọi nguồn → vắng mặt trong map).
fn rows_of(state: &AppState, keys: &[FaceKey]) -> Result<HashMap<FaceKey, FaceRowDto>, CmdError> {
    let mut out = HashMap::new();
    for chunk in keys.chunks(crate::services::library::MAX_FACE_KEYS) {
        let q = FaceQueryDto {
            face_keys: Some(chunk.iter().map(FaceKeyDto::from).collect()),
            include_missing: true,
            limit: 1000,
            ..Default::default()
        };
        let page = crate::services::library::query_faces(state, &q)?;
        for r in page.items {
            if let Ok(k) = FaceKey::from_hex(&r.hash_hex, r.face_index) {
                out.insert(k, r);
            }
        }
    }
    Ok(out)
}

fn combo_dto(c: ComboRow, rows: &HashMap<FaceKey, FaceRowDto>) -> ComboDto {
    ComboDto {
        id: c.id,
        case_id: c.case_id,
        idx: c.idx as u32,
        name: c.name,
        is_main: c.is_main,
        note: c.note,
        slots: c
            .slots
            .into_iter()
            .map(|s| ComboSlotDto {
                role: s.role,
                missing: rows.get(&s.key).is_none_or(|r| r.missing),
                row: rows.get(&s.key).cloned(),
                key: FaceKeyDto::from(&s.key),
                note: s.note,
                style: serde_json::from_str(&s.style).unwrap_or(Value::Null),
            })
            .collect(),
        roles: c.roles,
    }
}

fn origin_of(s: &str) -> CaseOriginDto {
    match s {
        "pinned" => CaseOriginDto::Pinned,
        "brief" => CaseOriginDto::Brief,
        "peek" => CaseOriginDto::Peek,
        _ => CaseOriginDto::Added,
    }
}

fn build(state: &AppState, c: CaseRow, faces: Vec<CaseFaceRow>, combos: Vec<ComboRow>) -> Result<CaseDto, CmdError> {
    let mut keys: Vec<FaceKey> = faces.iter().map(|f| f.key).collect();
    keys.extend(combos.iter().flat_map(|c| c.slots.iter().map(|s| s.key)));
    keys.sort_by_key(|k| (k.hash, k.face_index));
    keys.dedup();
    let rows = rows_of(state, &keys)?;
    Ok(CaseDto {
        id: c.id,
        name: c.name,
        brief: c.brief.as_deref().and_then(|b| serde_json::from_str(b).ok()),
        settings: settings_of(&c.settings),
        view_state: serde_json::from_str(&c.view_state).unwrap_or(Value::Null),
        created_at: c.created_at,
        updated_at: c.updated_at,
        opened_at: c.opened_at,
        deleted_at: c.deleted_at,
        faces: faces
            .into_iter()
            .map(|f| CaseFaceDto {
                missing: rows.get(&f.key).is_none_or(|r| r.missing),
                row: rows.get(&f.key).cloned(),
                key: FaceKeyDto::from(&f.key),
                origin: origin_of(&f.origin),
                note: f.note,
                added_at: f.added_at,
            })
            .collect(),
        combos: combos.into_iter().map(|c| combo_dto(c, &rows)).collect(),
    })
}

pub(crate) fn load(state: &AppState, id: i64) -> Result<CaseDto, CmdError> {
    let (c, faces, combos) = state.with_user(|u| Ok((u.case(id)?, u.case_faces(id)?, u.combos(id)?)))?;
    build(state, c, faces, combos)
}

fn default_name(state: &AppState) -> Result<String, CmdError> {
    let n = state.with_user(|u| Ok(u.cases(true)?.len()))?;
    Ok(format!("Case {}", n + 1))
}

pub fn create_case(state: &AppState, name: Option<&str>, brief: Option<&BriefDto>, settings: Option<&CaseSettingsDto>) -> Result<CaseDto, CmdError> {
    let name = match name {
        Some(n) => n.to_string(),
        None => default_name(state)?,
    };
    let b = brief.map(json_of).transpose()?;
    let s = settings.map(json_of).transpose()?;
    let id = state.with_user(|u| Ok(u.create_case(&name, b.as_deref(), s.as_deref())?))?;
    load(state, id)
}

pub fn list_cases(state: &AppState, include_deleted: bool) -> Result<Vec<CaseSummaryDto>, CmdError> {
    state.with_user(|u| {
        Ok(u.cases(include_deleted)?
            .into_iter()
            .map(|c| CaseSummaryDto {
                id: c.id,
                name: c.name,
                face_count: c.face_count,
                combo_count: c.combo_count,
                updated_at: c.updated_at,
                opened_at: c.opened_at,
                deleted_at: c.deleted_at,
            })
            .collect())
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Dọn hẳn Case đã nằm trong thùng rác quá 30 ngày. Gọi lúc khởi động (không để lệnh đọc có tác dụng phụ).
pub fn purge_trash(state: &AppState) -> usize {
    state.with_user(|u| Ok(u.purge_cases(now_ms())?)).unwrap_or(0)
}

/// Mở Case: đánh dấu vừa mở rồi trả đủ nội dung.
pub fn open_case(state: &AppState, id: i64) -> Result<CaseDto, CmdError> {
    state.with_user(|u| Ok(u.touch_case(id)?))?;
    load(state, id)
}

pub fn get_case(state: &AppState, id: i64) -> Result<CaseDto, CmdError> {
    load(state, id)
}

pub fn update_case(state: &AppState, id: i64, p: &CasePatchDto) -> Result<CaseDto, CmdError> {
    let patch = CasePatch {
        name: p.name.clone(),
        brief: if p.clear_brief { Some(None) } else { p.brief.as_ref().map(|b| json_of(b).map(Some)).transpose()? },
        settings: p.settings.as_ref().map(json_of).transpose()?,
        view_state: p.view_state.as_ref().map(json_of).transpose()?,
    };
    state.with_user(|u| Ok(u.update_case(id, &patch).map(|_| ())?))?;
    load(state, id)
}

pub fn delete_case(state: &AppState, id: i64) -> Result<(), CmdError> {
    state.with_user(|u| Ok(u.delete_case(id)?))
}

pub fn restore_case(state: &AppState, id: i64) -> Result<CaseDto, CmdError> {
    state.with_user(|u| Ok(u.restore_case(id)?))?;
    load(state, id)
}

/// `case_id = None`: Case đang mở gần nhất, chưa có thì tạo — lấy, tạo, thêm, đánh dấu mở trong MỘT khối nguyên tử
/// (hai lệnh song song không tạo hai Case).
pub fn add_to_case(state: &AppState, case_id: Option<i64>, faces: &[FaceKeyDto], origin: Option<CaseOriginDto>) -> Result<AddToCaseDto, CmdError> {
    let keys = keys_of(faces)?;
    let origin = origin.unwrap_or(CaseOriginDto::Added);
    let (id, added, already, created) = state.with_user(|u| match case_id {
        Some(id) => {
            let (a, al) = u.add_to_case(id, &keys, origin.as_str())?;
            u.touch_case(id)?;
            Ok((id, a, al, false))
        }
        None => Ok(u.add_to_latest_case(&keys, origin.as_str())?),
    })?;
    Ok(AddToCaseDto { case_id: id, added: added as u32, already_in: already as u32, created })
}

pub fn remove_from_case(state: &AppState, case_id: i64, faces: &[FaceKeyDto]) -> Result<u32, CmdError> {
    let keys = keys_of(faces)?;
    state.with_user(|u| Ok(u.remove_from_case(case_id, &keys)? as u32))
}

fn combo_by_id(state: &AppState, id: i64) -> Result<ComboDto, CmdError> {
    let c = state.with_user(|u| Ok(u.combo(id)?))?;
    let keys: Vec<FaceKey> = c.slots.iter().map(|s| s.key).collect();
    let rows = rows_of(state, &keys)?;
    Ok(combo_dto(c, &rows))
}

pub fn create_combo(state: &AppState, case_id: i64, name: Option<&str>, from_combo: Option<i64>) -> Result<ComboDto, CmdError> {
    let id = state.with_user(|u| Ok(u.create_combo(case_id, name, from_combo)?))?;
    combo_by_id(state, id)
}

pub fn update_combo(state: &AppState, id: i64, p: &ComboPatchDto) -> Result<ComboDto, CmdError> {
    let patch = ComboPatch {
        name: p.name.clone(),
        is_main: p.is_main,
        note: if p.clear_note { Some(None) } else { p.note.clone().map(Some) },
    };
    state.with_user(|u| Ok(u.update_combo(id, &patch).map(|_| ())?))?;
    combo_by_id(state, id)
}

pub fn delete_combo(state: &AppState, id: i64) -> Result<(), CmdError> {
    state.with_user(|u| Ok(u.delete_combo(id)?))
}

pub fn reorder_combos(state: &AppState, case_id: i64, ids: &[i64]) -> Result<(), CmdError> {
    state.with_user(|u| Ok(u.reorder_combos(case_id, ids)?))
}

pub fn set_combo_slot(
    state: &AppState,
    combo_id: i64,
    role: ComboRoleDto,
    face: Option<&FaceKeyDto>,
    note: Option<&str>,
    style: Option<&Value>,
) -> Result<ComboDto, CmdError> {
    let key = face.map(FaceKey::try_from).transpose()?;
    let style = style.map(json_of).transpose()?;
    state.with_user(|u| Ok(u.set_combo_slot(combo_id, &role, key.as_ref(), note, style.as_deref()).map(|_| ())?))?;
    combo_by_id(state, combo_id)
}

pub fn add_combo_role(state: &AppState, combo_id: i64, name: &str) -> Result<ComboDto, CmdError> {
    state.with_user(|u| Ok(u.add_combo_role(combo_id, name).map(|_| ())?))?;
    combo_by_id(state, combo_id)
}

pub fn remove_combo_role(state: &AppState, combo_id: i64, role: &str) -> Result<ComboDto, CmdError> {
    state.with_user(|u| Ok(u.remove_combo_role(combo_id, role).map(|_| ())?))?;
    combo_by_id(state, combo_id)
}

pub fn rename_combo_role(state: &AppState, combo_id: i64, role: &str, new_name: &str) -> Result<ComboDto, CmdError> {
    state.with_user(|u| Ok(u.rename_combo_role(combo_id, role, new_name).map(|_| ())?))?;
    combo_by_id(state, combo_id)
}

pub fn reorder_combo_roles(state: &AppState, combo_id: i64, roles: &[String]) -> Result<ComboDto, CmdError> {
    state.with_user(|u| Ok(u.reorder_combo_roles(combo_id, roles).map(|_| ())?))?;
    combo_by_id(state, combo_id)
}

pub fn create_combos(state: &AppState, case_id: i64, combos: &[NewComboDto]) -> Result<Vec<ComboDto>, CmdError> {
    let mut new = Vec::with_capacity(combos.len());
    for c in combos {
        let slots = c
            .slots
            .iter()
            .map(|s| Ok((s.role.clone(), FaceKey::try_from(&s.face)?)))
            .collect::<Result<Vec<_>, CmdError>>()?;
        new.push(NewCombo { name: c.name.clone(), slots });
    }
    let ids = state.with_user(|u| Ok(u.create_combos(case_id, &new)?))?;
    ids.into_iter().map(|id| combo_by_id(state, id)).collect()
}

const MAX_COMPARE_COMBOS: usize = 8;

fn pair_mode(m: Option<PairingLevelDto>) -> Option<crate::dto::analysis::PairModeDto> {
    use crate::dto::analysis::PairModeDto as P;
    m.map(|m| match m {
        PairingLevelDto::Safe => P::Safe,
        PairingLevelDto::Controlled => P::Controlled,
        PairingLevelDto::Experimental => P::Experimental,
    })
}

/// Compare Combo: mỗi cột là một Combo; giọng ① (vai đầu theo thứ tự display → quote) so với từng vai
/// dùng font khác. `combo_ids` rỗng = mọi Combo của Case (tối đa 8). Quan hệ không đánh giá được trả
/// `assessment: None` kèm `reason` (không nuốt lỗi). Neo thiếu font thì lấy vai kế tiếp còn font làm neo,
/// vai thiếu font vẫn có một dòng quan hệ `missing_font`.
pub fn case_compare(state: &AppState, case_id: i64, combo_ids: &[i64], lang: crate::dto::analysis::LangDto) -> Result<CaseCompareDto, CmdError> {
    use crate::dto::analysis::{PairOptsDto, PairRoleDto};
    let case = load(state, case_id)?;
    let picked: Vec<&ComboDto> = if combo_ids.is_empty() {
        case.combos.iter().collect()
    } else {
        combo_ids
            .iter()
            .map(|id| case.combos.iter().find(|c| c.id == *id).ok_or_else(|| CmdError::not_found(format!("Combo #{id} không thuộc Case"))))
            .collect::<Result<_, _>>()?
    };
    if picked.len() > MAX_COMPARE_COMBOS {
        return Err(CmdError::invalid_arg(format!("so sánh tối đa {MAX_COMPARE_COMBOS} Combo")));
    }
    let mut columns = Vec::with_capacity(picked.len());
    for combo in picked {
        let mut relations = Vec::new();
        let none = |from, to, reason: &str, detail: Option<String>| ComboRelationDto { from, to, assessment: None, reason: Some(reason.into()), reason_detail: detail };
        // neo = vai đầu còn font; các vai khác dùng font khác neo được so với neo
        let anchor = combo.slots.iter().find(|s| !s.missing);
        if let Some(first) = anchor {
            for other in combo.slots.iter().filter(|s| s.key != first.key && s.role != first.role) {
                if other.missing {
                    relations.push(none(first.role.clone(), other.role.clone(), "missing_font", None));
                    continue;
                }
                let opts = PairOptsDto {
                    role: PairRoleDto::Body,
                    mode: pair_mode(case.settings.mode),
                    project_vi: case.settings.vi,
                    k: 1,
                    collection_id: None,
                    lang,
                };
                relations.push(match crate::services::analysis::assess_pair(state, &first.key, &other.key, &opts) {
                    Ok(a) => ComboRelationDto { from: first.role.clone(), to: other.role.clone(), assessment: Some(a), reason: None, reason_detail: None },
                    Err(e) if e.code == "not_found" => none(first.role.clone(), other.role.clone(), "not_analyzed", None),
                    Err(e) => none(first.role.clone(), other.role.clone(), "unavailable", Some(e.message)),
                });
            }
        }
        let mut seen: Vec<&FaceKeyDto> = Vec::new();
        for s in &combo.slots {
            if !seen.contains(&&s.key) {
                seen.push(&s.key);
            }
        }
        let present: Vec<&FaceRowDto> = combo.slots.iter().filter(|s| !s.missing).filter_map(|s| s.row.as_ref()).collect();
        let mut licenses: Vec<String> = present.iter().map(|r| r.license_class.clone()).collect();
        licenses.sort();
        licenses.dedup();
        columns.push(ComboColumnDto {
            combo: combo.clone(),
            relations,
            font_count: seen.len() as u32,
            missing_count: combo.slots.iter().filter(|s| s.missing).count() as u32,
            worst_vi_missing: present.iter().map(|r| r.vi_missing).max(),
            licenses,
        });
    }
    Ok(CaseCompareDto { case_id, columns })
}

/// "Pair from peek" (2d): mở Case với font hiện tại làm neo ở vai tương ứng, mang theo mức ghép/tiếng Việt.
/// Ứng viên đang xem chỉ được thêm vào Case (origin `peek`) và trả lại làm `trial`, KHÔNG ghi vào vai.
pub fn open_case_from_peek(state: &AppState, p: &PeekOpenDto) -> Result<PeekOpenResultDto, CmdError> {
    let (anchor_role, other_role) = match p.as_role {
        PeekRoleDto::Heading => ("display".to_string(), "body".to_string()),
        PeekRoleDto::Body => ("body".to_string(), "display".to_string()),
    };
    let anchor = FaceKey::try_from(&p.face)?;
    let cand = p.candidate.as_ref().map(FaceKey::try_from).transpose()?;
    let settings = json_of(&p.settings)?;
    // một khối nguyên tử trong store: lỗi giữa chừng thì không để lại Case/Combo/font nào
    let out = state.with_user(|u| Ok(u.peek_open(p.case_id, &settings, &anchor, cand.as_ref(), anchor_role.as_str(), p.replace)?))?;
    let occupied = match out.occupied {
        Some(k) => Some(PeekOccupiedDto { role: anchor_role.clone(), row: rows_of(state, &[k])?.remove(&k), key: FaceKeyDto::from(&k) }),
        None => None,
    };
    Ok(PeekOpenResultDto { case: load(state, out.case_id)?, combo_id: out.combo_id, anchor_role, other_role, trial: p.candidate.clone(), created: out.created, occupied })
}
