//! Lệnh thư viện: truy vấn lưới, yêu thích, collection, tag, lỗi nạp, kiểu trong họ.
use std::sync::Arc;

use fontatlas_store::{FaceQuery, Store, StoreError};
use serde_json::Value;

use crate::dto::library::*;
use crate::error::CmdError;
use crate::state::AppState;

pub const MAX_FACE_KEYS: usize = 2000;

/// Phạm vi theo bộ lọc trục (từ `describe`): cần Index; rỗng thì không lọc.
fn axis_scope(
    state: &AppState,
    filters: &[AxisFilterDto],
    hard: &[String],
) -> Result<Option<Vec<(i64, f32)>>, CmdError> {
    if filters.is_empty() && hard.is_empty() {
        return Ok(None);
    }
    Ok(crate::services::analysis::bundle(state)?
        .scope_for(filters, &crate::services::describe::hard_of(hard)))
}

pub(crate) fn to_store_query(q: &FaceQueryDto) -> FaceQuery {
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
        langs: crate::services::langs::lang_filters(q).0,
        lang_any: crate::services::langs::lang_filters(q).1,
        weight_class: q.weight_class,
        width_class: q.width_class,
        var_weight: q.var_axis_match.wght,
        var_width: q.var_axis_match.wdth,
        badge_lang: Some(q.badge_lang.clone().unwrap_or_else(|| crate::services::langs::DEFAULT_BADGE_LANG.to_string())),
    }
}

/// Phạm vi face (điểm) từ các bộ lọc không nằm trong SQL: trục, class, trait, raw range, faceKeys.
pub(crate) fn scope_of(state: &AppState, q: &FaceQueryDto) -> Result<Option<Vec<(i64, f32)>>, CmdError> {
    let mut scope = axis_scope(state, &q.axis_filters, &q.hard)?;
    validate_raw_ranges(&q.raw_axis_ranges)?;
    validate_class_ranges(q)?;
    if !q.classes.is_empty() || !q.subclasses.is_empty() {
        let ids = class_face_ids(state, &q.classes, &q.subclasses)?;
        match &mut scope {
            Some(prev) => prev.retain(|x| ids.contains(&x.0)),
            None => scope = Some(ids.iter().map(|id| (*id, 1.0)).collect()),
        }
    }
    if !q.traits.is_empty() {
        let ids = trait_face_ids(state, &q.traits)?;
        match &mut scope {
            Some(prev) => prev.retain(|x| ids.contains(&x.0)),
            None => scope = Some(ids.iter().map(|id| (*id, 1.0)).collect()),
        }
    }
    if !q.raw_axis_ranges.is_empty() {
        let b = crate::services::analysis::bundle(state)?;
        let raw: Vec<(i64, f32)> = b
            .index
            .profiles()
            .iter()
            .filter(|p| raw_matches(p, &q.raw_axis_ranges))
            .filter_map(|p| Some((b.meta.get(&p.key)?.face_id, 1.0)))
            .collect();
        if let Some(prev) = &mut scope {
            let ids: std::collections::HashSet<_> = raw.iter().map(|x| x.0).collect();
            prev.retain(|x| ids.contains(&x.0));
        } else {
            scope = Some(raw);
        }
    }
    if let Some(fk) = &q.face_keys {
        if fk.len() > MAX_FACE_KEYS {
            return Err(CmdError::invalid_arg(format!(
                "faceKeys tối đa {MAX_FACE_KEYS}"
            )));
        }
        let keys = keys_of(fk)?;
        let ids = state.with_store(|s| Ok(s.face_ids_of(&keys)?))?;
        let n = ids.len() as f32;
        // điểm giảm dần theo thứ tự đưa vào → sort "match" giữ nguyên thứ tự đó
        let mut by_keys: Vec<(i64, f32)> = ids
            .iter()
            .enumerate()
            .map(|(i, (_, id))| (*id, n - i as f32))
            .collect();
        if let Some(prev) = &scope {
            let allowed: std::collections::HashSet<i64> = prev.iter().map(|x| x.0).collect();
            by_keys.retain(|x| allowed.contains(&x.0));
        }
        scope = Some(by_keys);
    }
    Ok(scope)
}

pub fn query_faces(state: &AppState, q: &FaceQueryDto) -> Result<FacePageDto, CmdError> {
    let scope = scope_of(state, q)?;
    let sq = to_store_query(q);
    let badge = sq.badge_lang.clone().unwrap_or_default();
    let page = state.with_store(|s| Ok(s.query_faces_scoped(&sq, scope.as_deref())?))?;
    Ok(FacePageDto {
        total: page.total,
        offset: page.offset,
        items: page
            .items
            .into_iter()
            .map(|r| {
                let b = crate::services::langs::badge_of(&badge, r.lang_hit.as_ref(), r.lang_computed, r.vi_missing);
                let mut d = FaceRowDto::from(r);
                d.lang_badge = b;
                d
            })
            .collect(),
    })
}

type EffMap = std::sync::Arc<std::collections::HashMap<i64, (String, Option<String>)>>;
static CLASS_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
type EffCache = Option<(std::sync::Weak<crate::index_bundle::IndexBundle>, u64, EffMap)>;
static EFF_CACHE: std::sync::Mutex<EffCache> = std::sync::Mutex::new(None);

/// Gọi sau mỗi lần đổi/xoá ghi đè phân loại để cache phân loại hiệu lực dựng lại.
pub(crate) fn bump_class_generation() {
    CLASS_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

/// Phân loại hiệu lực (ghi đè > tự động) của từng face trong Index: `face_id → (class, subclass)`.
/// Cache theo (bundle, thế hệ ghi đè) nên không đọc lại user.db cho mỗi truy vấn/facet.
pub(crate) fn effective_classes(state: &AppState) -> Result<EffMap, CmdError> {
    let b = crate::services::analysis::bundle(state)?;
    let generation = CLASS_GEN.load(std::sync::atomic::Ordering::SeqCst);
    if let Ok(g) = EFF_CACHE.lock()
        && let Some((w, gen_, m)) = g.as_ref()
        && *gen_ == generation
        && w.upgrade().is_some_and(|a| std::sync::Arc::ptr_eq(&a, &b))
    {
        return Ok(m.clone());
    }
    let ov: std::collections::HashMap<(String, u32), _> = state
        .with_store(|s| Ok(s.classification_overrides()?))?
        .into_iter()
        .map(|o| ((o.key.hash_hex(), o.key.face_index), o))
        .collect();
    let map: EffMap = std::sync::Arc::new(
        b.meta
            .iter()
            .map(|(k, m)| {
                let eff = match ov.get(&(k.hash.clone(), k.index)) {
                    Some(o) => (o.class.clone(), o.subclass.clone()),
                    None => (m.auto_class.clone(), m.auto_subclass.clone()),
                };
                (m.face_id, eff)
            })
            .collect(),
    );
    if let Ok(mut g) = EFF_CACHE.lock() {
        *g = Some((std::sync::Arc::downgrade(&b), generation, map.clone()));
    }
    Ok(map)
}

pub const TRAITS: [&str; 6] = ["stencil", "inline_outline", "rounded", "fat_face", "pixel", "decorative"];

/// Face có ít nhất một trait trong `traits`. Traits lấy từ phân loại TỰ ĐỘNG (tên/Panose/sFamilyClass); ghi đè của
/// người dùng chỉ đổi class/subclass, không đổi traits.
fn trait_face_ids(state: &AppState, traits: &[String]) -> Result<std::collections::HashSet<i64>, CmdError> {
    if let Some(bad) = traits.iter().find(|t| !TRAITS.contains(&t.as_str())) {
        return Err(CmdError::invalid_arg(format!("traits: tag không hợp lệ: {bad}")));
    }
    let b = crate::services::analysis::bundle(state)?;
    Ok(b.meta.values().filter(|m| m.auto_traits.iter().any(|t| traits.contains(t))).map(|m| m.face_id).collect())
}

fn class_face_ids(state: &AppState, classes: &[String], subclasses: &[String]) -> Result<std::collections::HashSet<i64>, CmdError> {
    use fontatlas_core::classify::{Class, subclasses_of};
    let classes: Vec<String> = classes.iter().map(|c| c.to_ascii_lowercase()).collect();
    for c in &classes {
        if c != "unclassified" && Class::parse(c).is_none() {
            return Err(CmdError::invalid_arg(format!("classes: class không hợp lệ: {c}")));
        }
    }
    let known: Vec<&str> = [Class::Serif, Class::Sans, Class::Mono, Class::Script]
        .into_iter()
        .flat_map(subclasses_of)
        .copied()
        .collect();
    for s in subclasses {
        if !known.contains(&s.as_str()) {
            return Err(CmdError::invalid_arg(format!("subclasses: subclass không hợp lệ: {s}")));
        }
    }
    let mut out: std::collections::HashSet<i64> = effective_classes(state)?
        .iter()
        .filter(|(_, (c, sub))| {
            (classes.is_empty() || classes.contains(c))
                && (subclasses.is_empty() || sub.as_ref().is_some_and(|s| subclasses.contains(s)))
        })
        .map(|(id, _)| *id)
        .collect();
    // "unclassified": face chưa có hồ sơ trong Index (chưa phân tích / JSON hỏng) — luôn tách riêng, không lẫn vào "uncertain".
    if classes.iter().any(|c| c == "unclassified") && subclasses.is_empty() {
        let b = crate::services::analysis::bundle(state)?;
        let known_ids: std::collections::HashSet<i64> = b.meta.values().map(|m| m.face_id).collect();
        out.extend(state.with_store(|s| Ok(s.all_face_ids()?))?.into_iter().filter(|id| !known_ids.contains(id)));
    }
    Ok(out)
}

pub(crate) fn validate_class_ranges(q: &FaceQueryDto) -> Result<(), CmdError> {
    if q.weight_class.is_some_and(|[a, b]| a > b || !(100..=900).contains(&a) || !(100..=900).contains(&b)) {
        return Err(CmdError::invalid_arg("weightClass: khoảng không hợp lệ (100–900, min ≤ max)"));
    }
    if q.width_class.is_some_and(|[a, b]| a == 0 || a > b || b > 9) {
        return Err(CmdError::invalid_arg("widthClass: khoảng không hợp lệ (1–9, min ≤ max)"));
    }
    Ok(())
}

pub(crate) fn validate_raw_ranges(ranges: &[RawAxisRangeDto]) -> Result<(), CmdError> {
    if ranges.len() > 16 {
        return Err(CmdError::invalid_arg("rawAxisRanges tối đa 16"));
    }
    for r in ranges {
        if !fontatlas_index::AxisId::ALL
            .iter()
            .any(|a| a.key() == r.axis)
            || !r.range.iter().all(|x| x.is_finite())
            || r.range[0] > r.range[1]
        {
            return Err(CmdError::invalid_arg(
                "rawAxisRanges: trục hoặc khoảng không hợp lệ",
            ));
        }
    }
    Ok(())
}

pub(crate) fn raw_reading(
    p: &fontatlas_index::FaceProfile,
    axis: fontatlas_index::AxisId,
) -> Option<f32> {
    let r = p.get(axis);
    // Uppercase stem divides by cap-height: never mix it into x-height domains/filters.
    if r.q == fontatlas_index::Q::Failed
        || (axis == fontatlas_index::AxisId::StemNXh && r.method != axis.default_method())
    {
        return None;
    }
    r.value.filter(|v| v.is_finite())
}

pub(crate) fn raw_matches(p: &fontatlas_index::FaceProfile, ranges: &[RawAxisRangeDto]) -> bool {
    ranges.iter().all(|r| {
        fontatlas_index::AxisId::ALL
            .iter()
            .copied()
            .find(|a| a.key() == r.axis)
            .and_then(|a| raw_reading(p, a))
            .is_some_and(|v| v >= r.range[0] && v <= r.range[1])
    })
}

/// Every matching face, using existing Store predicates and paging beyond Store's 1000-row cap.
/// Used for family ranking/stats; the caller owns family paging, not SQL face paging.
pub(crate) fn matching_keys(
    state: &AppState,
    q: &FaceQueryDto,
) -> Result<std::collections::HashSet<fontatlas_index::FaceKey>, CmdError> {
    let mut query = q.clone();
    query.offset = 0;
    query.limit = 1000;
    let mut out = std::collections::HashSet::new();
    loop {
        let page = query_faces(state, &query)?;
        let n = page.items.len();
        for r in page.items {
            out.insert(fontatlas_index::FaceKey {
                hash: r.hash_hex,
                index: r.face_index,
            });
        }
        if n == 0 || u64::from(query.offset) + n as u64 >= page.total {
            break;
        }
        query.offset = query.offset.saturating_add(n as u32);
    }
    Ok(out)
}

pub fn set_favorite(state: &AppState, face: &FaceKeyDto, on: bool) -> Result<(), CmdError> {
    let k = FaceKey::try_from(face)?;
    state.with_user(|u| Ok(u.set_favorite(&k, on)?))
}

use fontatlas_store::FaceKey;

pub fn list_collections(state: &AppState) -> Result<Vec<CollectionDto>, CmdError> {
    state.with_user(|u| Ok(u.collections()?.into_iter().map(Into::into).collect()))
}

pub fn create_collection(
    state: &AppState,
    name: &str,
    color: Option<&str>,
) -> Result<CollectionDto, CmdError> {
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

/// Gỡ kích hoạt font của collection trước khi xoá (service Plan 7).
pub fn before_delete_collection(state: &AppState, id: i64) -> Result<(), CmdError> {
    crate::services::activation::before_delete_collection(state, id)
}

pub fn delete_collection(state: &AppState, id: i64) -> Result<(), CmdError> {
    before_delete_collection(state, id)?;
    state.with_user(|u| Ok(u.delete_collection(id)?))
}

pub fn add_to_collection(state: &AppState, id: i64, faces: &[FaceKeyDto]) -> Result<(), CmdError> {
    let keys = keys_of(faces)?;
    state.with_user(|u| Ok(u.add_to_collection(id, &keys).map(|_| ())?))
}

pub fn remove_from_collection(
    state: &AppState,
    id: i64,
    faces: &[FaceKeyDto],
) -> Result<(), CmdError> {
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

/// Danh mục tag hệ thống (style/mood/use). Chỉ đọc.
pub fn system_tag_catalog() -> Vec<SystemTagGroupDto> {
    use fontatlas_core::system_tags::{SYSTEM_TAG_GROUPS, tag_name};
    SYSTEM_TAG_GROUPS
        .iter()
        .map(|g| SystemTagGroupDto {
            namespace: g.namespace.into(),
            label: g.label.into(),
            tags: g.tags.iter().map(|t| SystemTagDto { key: t.key.into(), label: t.label.into(), name: tag_name(g.namespace, t.key) }).collect(),
        })
        .collect()
}

pub fn create_tag(state: &AppState, name: &str) -> Result<TagDto, CmdError> {
    use fontatlas_core::system_tags::{canonical_name, check_name};
    let name = name.trim();
    if name.is_empty() {
        return Err(CmdError::invalid_arg("tên tag trống"));
    }
    // tiền tố dành riêng (style:/mood:/use:) chỉ nhận giá trị trong danh mục, lưu dạng chuẩn hoá
    let canon;
    let name = match check_name(name) {
        Some(true) => {
            canon = canonical_name(name).unwrap_or_else(|| name.to_string());
            canon.as_str()
        }
        Some(false) => return Err(CmdError::invalid_arg(format!("tag hệ thống không có trong danh mục: {name}"))),
        None => name,
    };
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
        let rows = st.query_map([family], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (h, i) = row?;
            let hash: [u8; 32] = h
                .as_slice()
                .try_into()
                .map_err(|_| StoreError::BadHash("blob".into()))?;
            out.push(FaceKey {
                hash,
                face_index: i as u32,
            });
        }
        out
    };
    let mut out = Vec::new();
    for key in keys {
        let Some(a) = s.analysis_of(&key)? else {
            continue;
        };
        let m: Value = serde_json::from_str(&a.metadata_json).unwrap_or(Value::Null);
        let instances = m
            .get("named_instance_coords")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let is_vf = m
            .get("axes")
            .and_then(Value::as_array)
            .is_some_and(|x| !x.is_empty());
        let face = FaceKeyDto::from(&key);
        if is_vf && !instances.is_empty() {
            for inst in instances {
                let name = str_of(&inst, "name").unwrap_or_default();
                let coords: Vec<AxisCoordDto> = inst
                    .get("coords")
                    .and_then(Value::as_array)
                    .map(|c| {
                        c.iter()
                            .filter_map(|p| {
                                Some(AxisCoordDto {
                                    tag: p.get(0)?.as_str()?.to_string(),
                                    value: p.get(1)?.as_f64()? as f32,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let wght = coords
                    .iter()
                    .find(|c| c.tag == "wght")
                    .map(|c| c.value.round() as u16);
                let ital = coords.iter().any(|c| c.tag == "ital" && c.value == 1.0)
                    || name.contains("Italic");
                out.push(FamilyStyleDto {
                    face: face.clone(),
                    subfamily: Some(name.clone()),
                    weight: wght,
                    italic: ital,
                    instance: Some(InstanceDto { name, coords }),
                });
            }
        } else {
            let weight = m
                .get("weight_class")
                .and_then(Value::as_u64)
                .and_then(|w| u16::try_from(w).ok());
            let sub = str_of(&m, "typographic_subfamily").or_else(|| str_of(&m, "subfamily"));
            let italic = m
                .get("italic_angle")
                .and_then(Value::as_f64)
                .is_some_and(|x| x != 0.0)
                || sub.as_deref().is_some_and(|x| x.contains("Italic"));
            out.push(FamilyStyleDto {
                face,
                subfamily: sub,
                weight,
                italic,
                instance: None,
            });
        }
    }
    Ok(out)
}
