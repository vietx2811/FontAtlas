//! Gợi ý cho Pairing Desk (G4–G6). Mô hình (ngưỡng, mã lý do) nằm ở `fontatlas_core::pairing` (04); file này chỉ dựng
//! `Traits` từ hồ sơ Index, quét thư viện, chọn trang, và đọc/ghi lịch sử xem. Không điểm số, thứ tự tất định.
use std::collections::HashSet;

use fontatlas_core::classify::Class;
use fontatlas_core::pairing::{self, Fit, Reason, Role, Traits};
use fontatlas_index::{AxisId, FaceProfile, Q};

use crate::dto::library::FaceKeyDto;
use crate::dto::suggest::*;
use crate::error::CmdError;
use crate::index_bundle::IndexBundle;
use crate::services::analysis::{bundle, dto_key, ix_of, rows_of};
use crate::services::library::{EffMap, effective_classes};
use crate::state::AppState;

fn axis(p: &FaceProfile, a: AxisId) -> Option<f32> {
    let r = p.readings.get(a.index())?;
    (r.q != Q::Failed).then_some(r.value).flatten().filter(|v| v.is_finite())
}

fn reading(r: &fontatlas_index::Reading) -> Option<f32> {
    (r.q != Q::Failed).then_some(r.value).flatten().filter(|v| v.is_finite())
}

/// Số đo của một face cho mô hình G4. Số đo `Failed` → `None` (mô hình bỏ tiêu chí, không đoán).
/// Class = phân loại HIỆU LỰC (ghi đè của người dùng > tự động, `eff` từ `library::effective_classes`).
/// `has_tnum` chưa có trong Index nên để `None` (mô hình bỏ cổng tnum, không hạ Numerals).
pub(crate) fn traits_of(b: &IndexBundle, p: &FaceProfile, eff: &EffMap) -> Traits {
    let m = b.meta.get(&p.key);
    let e = m.and_then(|m| eff.get(&m.face_id));
    let class = e.and_then(|e| Class::parse(&e.class)).unwrap_or(Class::Uncertain);
    // Candidate chỉ có nghĩa khi lớp vẫn Uncertain (ghi đè của người dùng thì bỏ).
    let candidate = if class == Class::Uncertain { m.and_then(|m| m.auto_candidate.as_deref()).and_then(Class::parse) } else { None };
    Traits {
        class,
        candidate,
        subclass: e.and_then(|e| e.sub.clone()),
        x_height_em: axis(p, AxisId::XHeightEm),
        x_height_cap: axis(p, AxisId::XHeightCap),
        aperture: axis(p, AxisId::Aperture),
        aperture_e: axis(p, AxisId::ApertureE),
        contrast: axis(p, AxisId::Contrast),
        stem: axis(p, AxisId::StemNXh),
        spacing: axis(p, AxisId::Spacing),
        width: axis(p, AxisId::Width),
        stress_deg: axis(p, AxisId::StressDeg),
        a_storeys: reading(&p.a_storeys),
        g_storeys: reading(&p.g_storeys),
        vi_missing: Some(p.vi_missing as u32),
        has_tnum: None,
        ..Traits::default()
    }
}

pub(crate) fn fit_dto(f: Fit) -> FitDto {
    match f {
        Fit::Unknown => FitDto::Unknown,
        Fit::Weak => FitDto::Weak,
        Fit::Ok => FitDto::Ok,
        Fit::Good => FitDto::Good,
    }
}

pub(crate) fn reason_dto(r: &Reason) -> WhyDto {
    WhyDto {
        code: r.code.to_string(),
        params: r.params.iter().map(|(k, v)| WhyParamDto { key: (*k).to_string(), value: *v }).collect(),
    }
}

pub(crate) fn role_id(r: Role) -> String {
    serde_json::to_value(r).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

fn role_of_id(s: &str) -> Option<Role> {
    serde_json::from_value(serde_json::Value::String(s.trim().to_ascii_lowercase())).ok()
}

/// Tên vai → hồ sơ: tên (đồng nghĩa) → `profile` → `size`. `None` = không nhận ra.
pub(crate) fn resolve_role(r: &RoleRefDto) -> Option<Role> {
    pairing::role_from_name(&r.name)
        .or_else(|| r.profile.as_deref().and_then(role_of_id))
        .or(r.size.map(|s| match s {
            RoleSizeDto::Text => Role::Body,
            RoleSizeDto::Heading => Role::Heading,
            RoleSizeDto::Display => Role::Display,
        }))
}

/// Chưa đọc được `tnum` từ Index: nói rõ cổng này chưa kiểm (không hạ mức, không loại font).
pub(crate) fn with_tnum_note(r: Role, mut why: Vec<WhyDto>) -> Vec<WhyDto> {
    if r == Role::Numerals && why.len() < 3 && !why.iter().any(|w| w.code == "tnum_missing") {
        why.push(WhyDto { code: "tnum_unknown".into(), params: vec![] });
    }
    why
}

fn unknown_role() -> RoleFitDto {
    RoleFitDto {
        fit: FitDto::Unknown,
        profile: None,
        reasons: vec![WhyDto { code: "role_unknown".into(), params: vec![] }],
    }
}

pub fn role_fit(state: &AppState, face: &FaceKeyDto, role: &RoleRefDto) -> Result<RoleFitDto, CmdError> {
    let b = bundle(state)?;
    let key = ix_of(&b, face)?;
    let Some(r) = resolve_role(role) else {
        return Ok(unknown_role());
    };
    let p = b.index.profile(&key).ok_or_else(|| CmdError::not_found("face"))?;
    let eff = effective_classes(state)?;
    let x = pairing::role_fit(&traits_of(&b, p, &eff), r);
    Ok(RoleFitDto { fit: fit_dto(x.fit), profile: Some(role_id(r)), reasons: with_tnum_note(r, x.reasons.iter().map(reason_dto).collect()) })
}

/// Font hợp một vai trong cả thư viện (nguồn "Hợp vai" của cột 2). Chỉ good/ok; good trước, rồi tên họ, rồi key (tất định).
pub fn role_fit_faces(state: &AppState, role: &RoleRefDto, opts: &RoleFitListOptsDto) -> Result<RoleFitPageDto, CmdError> {
    role_fit_faces_excluding(state, role, opts, None)
}

/// Như `role_fit_faces`, loại `exclude` (font neo) TRƯỚC khi đếm `total` và cắt trang.
pub(crate) fn role_fit_faces_excluding(
    state: &AppState,
    role: &RoleRefDto,
    opts: &RoleFitListOptsDto,
    exclude: Option<&fontatlas_index::FaceKey>,
) -> Result<RoleFitPageDto, CmdError> {
    let Some(r) = resolve_role(role) else {
        return Ok(RoleFitPageDto { profile: None, items: vec![], total: 0 });
    };
    let b = bundle(state)?;
    let eff = effective_classes(state)?;
    let allowed: Option<HashSet<fontatlas_index::FaceKey>> = opts
        .filter
        .as_ref()
        .map(|q| crate::services::library::matching_keys(state, q))
        .transpose()?;
    // Face không Latin (CJK, Indic…) không gợi ý cho chữ Latin; face missing bỏ.
    let mut hits: Vec<(Fit, &FaceProfile, Vec<Reason>)> = Vec::new();
    for p in b.index.profiles() {
        let Some(m) = b.meta.get(&p.key) else { continue };
        if m.missing || m.non_latin_primary || (opts.project_vi && p.vi_missing > 0) || exclude.is_some_and(|e| *e == p.key) {
            continue;
        }
        if allowed.as_ref().is_some_and(|s| !s.contains(&p.key)) {
            continue;
        }
        let x = pairing::role_fit(&traits_of(&b, p, &eff), r);
        if matches!(x.fit, Fit::Good | Fit::Ok) {
            hits.push((x.fit, p, x.reasons));
        }
    }
    hits.sort_by(|a, c| {
        c.0.cmp(&a.0).then_with(|| a.1.family.cmp(&c.1.family)).then_with(|| a.1.style.cmp(&c.1.style)).then_with(|| a.1.key.cmp(&c.1.key))
    });
    let total = hits.len() as u64;
    let limit = opts.limit.clamp(1, 100) as usize;
    let page: Vec<_> = hits.into_iter().skip(opts.offset as usize).take(limit).collect();
    let keys: Vec<_> = page.iter().map(|h| h.1.key.clone()).collect();
    let rows = rows_of(state, &b, &keys)?;
    let items = page
        .into_iter()
        .filter_map(|(fit, p, reasons)| {
            Some(RoleFitHitDto { face: dto_key(&p.key), row: rows.get(&p.key)?.clone(), fit: fit_dto(fit), reasons: with_tnum_note(r, reasons.iter().map(reason_dto).collect()) })
        })
        .collect();
    Ok(RoleFitPageDto { profile: Some(role_id(r)), items, total })
}

// ---- font vừa xem (nguồn "Recent") ----

pub fn record_view(state: &AppState, face: &FaceKeyDto) -> Result<(), CmdError> {
    let k = fontatlas_store::FaceKey::try_from(face)?;
    state.with_store(|s| Ok(s.record_view(&k)?))
}

pub fn recent_views(state: &AppState, limit: u32) -> Result<Vec<RecentViewDto>, CmdError> {
    let views = state.with_store(|s| Ok(s.recent_views(limit as usize)?))?;
    let keys: Vec<_> = views.iter().map(|(k, _)| *k).collect();
    let ids = state.with_store(|s| Ok(s.face_ids_of(&keys)?))?;
    let by_key: std::collections::HashMap<_, _> = ids.into_iter().map(|(k, id)| ((k.hash_hex(), k.face_index), id)).collect();
    let id_list: Vec<i64> = by_key.values().copied().collect();
    let rows = crate::services::duplicates::rows_by_id(state, &id_list)?;
    // Mới nhất trước; face đã bị xoá khỏi thư viện thì bỏ (không treo "ma").
    Ok(views
        .into_iter()
        .filter_map(|(k, at)| {
            let id = by_key.get(&(k.hash_hex(), k.face_index))?;
            let row = rows.get(id)?.clone();
            Some(RecentViewDto { face: FaceKeyDto { hash_hex: k.hash_hex(), face_index: k.face_index }, row: Some(row), viewed_at: at })
        })
        .collect())
}

pub fn clear_recent_views(state: &AppState) -> Result<u32, CmdError> {
    state.with_store(|s| Ok(s.clear_recent_views()? as u32))
}
