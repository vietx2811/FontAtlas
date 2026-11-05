//! Lệnh thư viện: truy vấn lưới, yêu thích, collection, tag, lỗi nạp, kiểu trong họ.
use std::sync::Arc;

use fontatlas_store::{FaceQuery, Store, StoreError};
use serde_json::Value;

use crate::dto::library::*;
use crate::error::CmdError;
use crate::state::AppState;

pub const MAX_FACE_KEYS: usize = 2000;

/// Phạm vi theo trục mô tả: Task 15 nối vào Index. Trước đó chỉ chấp nhận danh sách rỗng.
fn axis_scope(_state: &AppState, filters: &[AxisFilterDto]) -> Result<Option<Vec<(i64, f32)>>, CmdError> {
    if filters.is_empty() {
        return Ok(None);
    }
    Err(CmdError::busy("Index chưa dựng xong"))
}

fn to_store_query(q: &FaceQueryDto) -> FaceQuery {
    FaceQuery {
        text: q.text.clone(),
        vi: q.vi.map(|v| match v {
            ViFilterDto::Full => fontatlas_store::ViFilter::Full,
            ViFilterDto::Supported => fontatlas_store::ViFilter::Supported,
        }),
        license: q.license.clone(),
        variable: q.variable,
        italic: q.italic,
        source_ids: q.source_ids.clone(),
        favorites_only: q.favorites_only,
        collection_id: q.collection_id,
        tag_ids: q.tag_ids.clone(),
        include_missing: q.include_missing,
        sort: q.sort.into(),
        descending: q.descending,
        offset: q.offset,
        limit: q.limit,
    }
}

pub fn query_faces(state: &AppState, q: &FaceQueryDto) -> Result<FacePageDto, CmdError> {
    let mut scope = axis_scope(state, &q.axis_filters)?;
    if let Some(fk) = &q.face_keys {
        if fk.len() > MAX_FACE_KEYS {
            return Err(CmdError::invalid_arg(format!("faceKeys tối đa {MAX_FACE_KEYS}")));
        }
        let keys = keys_of(fk)?;
        let ids = state.with_store(|s| Ok(s.face_ids_of(&keys)?))?;
        let n = ids.len() as f32;
        // điểm giảm dần theo thứ tự đưa vào → sort "match" giữ nguyên thứ tự đó
        let mut by_keys: Vec<(i64, f32)> = ids.iter().enumerate().map(|(i, (_, id))| (*id, n - i as f32)).collect();
        if let Some(prev) = &scope {
            let allowed: std::collections::HashSet<i64> = prev.iter().map(|x| x.0).collect();
            by_keys.retain(|x| allowed.contains(&x.0));
        }
        scope = Some(by_keys);
    }
    let sq = to_store_query(q);
    state.with_store(|s| Ok(s.query_faces_scoped(&sq, scope.as_deref())?.into()))
}

pub fn set_favorite(state: &AppState, face: &FaceKeyDto, on: bool) -> Result<(), CmdError> {
    let k = FaceKey::try_from(face)?;
    state.with_user(|u| Ok(u.set_favorite(&k, on)?))
}

use fontatlas_store::FaceKey;

pub fn list_collections(state: &AppState) -> Result<Vec<CollectionDto>, CmdError> {
    state.with_user(|u| Ok(u.collections()?.into_iter().map(Into::into).collect()))
}

pub fn create_collection(state: &AppState, name: &str, color: Option<&str>) -> Result<CollectionDto, CmdError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CmdError::invalid_arg("tên collection trống"));
    }
    state.with_user(|u| {
        let id = u.create_collection(name, color)?;
        u.collections()?
            .into_iter()
            .find(|c| c.id == id)
            .map(Into::into)
            .ok_or_else(|| CmdError::internal("collection vừa tạo không thấy"))
    })
}

pub fn rename_collection(state: &AppState, id: i64, name: &str) -> Result<(), CmdError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CmdError::invalid_arg("tên collection trống"));
    }
    state.with_user(|u| Ok(u.rename_collection(id, name)?))
}

/// Gỡ kích hoạt font của collection trước khi xoá (Task 17 nối vào service Plan 7).
pub fn before_delete_collection(_state: &AppState, _id: i64) -> Result<(), CmdError> {
    Ok(())
}

pub fn delete_collection(state: &AppState, id: i64) -> Result<(), CmdError> {
    before_delete_collection(state, id)?;
    state.with_user(|u| Ok(u.delete_collection(id)?))
}

pub fn add_to_collection(state: &AppState, id: i64, faces: &[FaceKeyDto]) -> Result<(), CmdError> {
    let keys = keys_of(faces)?;
    state.with_user(|u| Ok(u.add_to_collection(id, &keys).map(|_| ())?))
}

pub fn remove_from_collection(state: &AppState, id: i64, faces: &[FaceKeyDto]) -> Result<(), CmdError> {
    let keys = keys_of(faces)?;
    state.with_user(|u| Ok(u.remove_from_collection(id, &keys).map(|_| ())?))
}

pub fn list_tags(state: &AppState) -> Result<Vec<TagDto>, CmdError> {
    state.with_user(|u| Ok(u.tags()?.into_iter().map(Into::into).collect()))
}

pub fn tags_of(state: &AppState, face: &FaceKeyDto) -> Result<Vec<TagDto>, CmdError> {
    let k = FaceKey::try_from(face)?;
    state.with_user(|u| Ok(u.tags_of(&k)?.into_iter().map(Into::into).collect()))
}

pub fn create_tag(state: &AppState, name: &str) -> Result<TagDto, CmdError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CmdError::invalid_arg("tên tag trống"));
    }
    state.with_user(|u| {
        let id = u.tag(name)?;
        u.tags()?
            .into_iter()
            .find(|t| t.id == id)
            .map(Into::into)
            .ok_or_else(|| CmdError::internal("tag vừa tạo không thấy"))
    })
}

pub fn tag_faces(state: &AppState, tag_id: i64, faces: &[FaceKeyDto]) -> Result<(), CmdError> {
    let keys = keys_of(faces)?;
    state.with_user(|u| Ok(u.tag_faces(tag_id, &keys).map(|_| ())?))
}

pub fn untag_faces(state: &AppState, tag_id: i64, faces: &[FaceKeyDto]) -> Result<(), CmdError> {
    let keys = keys_of(faces)?;
    state.with_user(|u| Ok(u.untag_faces(tag_id, &keys).map(|_| ())?))
}

pub fn list_problems(state: &AppState) -> Result<Vec<ProblemDto>, CmdError> {
    state.with_store(|s| Ok(s.problem_files()?.into_iter().map(Into::into).collect()))
}

/// Đưa mục lỗi về `pending` rồi quét lại mọi nguồn thư mục; trả số mục đã đưa lại hàng đợi.
pub fn retry_problems(state: &Arc<AppState>) -> Result<u32, CmdError> {
    let n = state.with_store(|s| Ok(s.retry_failed()?))?;
    if n > 0 {
        crate::services::sources::rescan_sources(state, None)?;
    }
    Ok(n as u32)
}

fn str_of(m: &Value, k: &str) -> Option<String> {
    m.get(k).and_then(Value::as_str).map(String::from)
}

/// Mọi kiểu của một họ: face tĩnh một dòng; font biến thiên một dòng cho mỗi named instance.
pub fn get_family_styles(state: &AppState, family: &str) -> Result<Vec<FamilyStyleDto>, CmdError> {
    state.with_store(|s| family_styles(s, family).map_err(CmdError::from))
}

fn family_styles(s: &Store, family: &str) -> Result<Vec<FamilyStyleDto>, StoreError> {
    let keys: Vec<FaceKey> = {
        let mut st = s.conn().prepare(
            "SELECT fi.hash, f.face_index FROM faces f JOIN files fi ON fi.file_id = f.file_id
             WHERE f.family_name = ?1
               AND EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0)
             ORDER BY f.weight_class, f.is_italic, f.face_id",
        )?;
        let rows = st.query_map([family], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (h, i) = row?;
            let hash: [u8; 32] = h.as_slice().try_into().map_err(|_| StoreError::BadHash("blob".into()))?;
            out.push(FaceKey { hash, face_index: i as u32 });
        }
        out
    };
    let mut out = Vec::new();
    for key in keys {
        let Some(a) = s.analysis_of(&key)? else { continue };
        let m: Value = serde_json::from_str(&a.metadata_json).unwrap_or(Value::Null);
        let instances = m.get("named_instance_coords").and_then(Value::as_array).cloned().unwrap_or_default();
        let is_vf = m.get("axes").and_then(Value::as_array).is_some_and(|x| !x.is_empty());
        let face = FaceKeyDto::from(&key);
        if is_vf && !instances.is_empty() {
            for inst in instances {
                let name = str_of(&inst, "name").unwrap_or_default();
                let coords: Vec<AxisCoordDto> = inst
                    .get("coords")
                    .and_then(Value::as_array)
                    .map(|c| {
                        c.iter()
                            .filter_map(|p| Some(AxisCoordDto { tag: p.get(0)?.as_str()?.to_string(), value: p.get(1)?.as_f64()? as f32 }))
                            .collect()
                    })
                    .unwrap_or_default();
                let wght = coords.iter().find(|c| c.tag == "wght").map(|c| c.value.round() as u16);
                let ital = coords.iter().any(|c| c.tag == "ital" && c.value == 1.0) || name.contains("Italic");
                out.push(FamilyStyleDto {
                    face: face.clone(),
                    subfamily: Some(name.clone()),
                    weight: wght,
                    italic: ital,
                    instance: Some(InstanceDto { name, coords }),
                });
            }
        } else {
            let weight = m.get("weight_class").and_then(Value::as_u64).and_then(|w| u16::try_from(w).ok());
            let sub = str_of(&m, "typographic_subfamily").or_else(|| str_of(&m, "subfamily"));
            let italic = m.get("italic_angle").and_then(Value::as_f64).is_some_and(|x| x != 0.0)
                || sub.as_deref().is_some_and(|x| x.contains("Italic"));
            out.push(FamilyStyleDto { face, subfamily: sub, weight, italic, instance: None });
        }
    }
    Ok(out)
}
