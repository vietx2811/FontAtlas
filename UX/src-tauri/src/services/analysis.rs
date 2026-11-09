//! Chi tiết face, Similar và Pairing trên Index (không cần model: thiếu ảnh thì chạy theo trục).
use std::collections::HashMap;
use std::sync::Arc;

use fontatlas_index::{
    AxisId, Block, BodyWarning, ClashKind, FaceKey as IxKey, FaceProfile, Gate, GroupWeights,
    IndexError, Lang, PairAssessment, PairAxis, Q, Relation, Role, Side, SimilarHit,
    SimilarOptions, SimilarSource, explain_pair, fmt_num, summarize_face,
};
use fontatlas_store::{FaceKey, FaceQuery, SortKey};
use serde_json::Value;

use crate::dto::analysis::*;
use crate::dto::library::*;
use crate::error::CmdError;
use crate::index_bundle::{IndexBundle, ix_key, store_key};
use crate::state::AppState;

pub fn bundle(state: &AppState) -> Result<Arc<IndexBundle>, CmdError> {
    state
        .index
        .read()
        .ok()
        .and_then(|g| g.clone())
        .ok_or_else(|| CmdError::busy("Index chưa dựng xong"))
}

pub(crate) fn lang_of(l: LangDto) -> Lang {
    match l {
        LangDto::Vi => Lang::Vi,
        LangDto::En => Lang::En,
    }
}

pub(crate) fn dto_key(k: &IxKey) -> FaceKeyDto {
    FaceKeyDto {
        hash_hex: k.hash.clone(),
        face_index: k.index,
    }
}

pub(crate) fn ix_of(b: &IndexBundle, k: &FaceKeyDto) -> Result<IxKey, CmdError> {
    let sk = FaceKey::try_from(k)?;
    let key = ix_key(&sk);
    if b.meta.contains_key(&key) {
        Ok(key)
    } else {
        Err(CmdError::not_found("không có face này trong Index"))
    }
}

/// Hàng lưới của các face theo thứ tự đưa vào (một truy vấn, không N+1).
pub(crate) fn rows_of(
    state: &AppState,
    b: &IndexBundle,
    keys: &[IxKey],
) -> Result<HashMap<IxKey, FaceRowDto>, CmdError> {
    if keys.is_empty() {
        return Ok(HashMap::new());
    }
    let n = keys.len() as f32;
    let scope: Vec<(i64, f32)> = keys
        .iter()
        .enumerate()
        .filter_map(|(i, k)| Some((b.meta.get(k)?.face_id, n - i as f32)))
        .collect();
    let q = FaceQuery {
        include_missing: true,
        sort: SortKey::Match,
        limit: 1000,
        ..Default::default()
    };
    let page = state.with_store(|s| Ok(s.query_faces_scoped(&q, Some(&scope))?))?;
    Ok(page
        .items
        .into_iter()
        .filter_map(|r| Some((b.by_face_id.get(&r.face_id)?.clone(), FaceRowDto::from(r))))
        .collect())
}

fn axis_names(v: &[AxisId]) -> Vec<String> {
    v.iter().map(|a| a.key().to_string()).collect()
}

fn source_dto(s: SimilarSource) -> SimilarSourceDto {
    match s {
        SimilarSource::Axes => SimilarSourceDto::Axes,
        SimilarSource::AxesAndImage => SimilarSourceDto::AxesAndImage,
        SimilarSource::Image => SimilarSourceDto::Image,
    }
}

fn hits_dto(
    state: &AppState,
    b: &IndexBundle,
    hits: Vec<SimilarHit>,
) -> Result<Vec<SimilarHitDto>, CmdError> {
    let keys: Vec<IxKey> = hits.iter().map(|h| h.key.clone()).collect();
    let rows = rows_of(state, b, &keys)?;
    Ok(hits
        .into_iter()
        .filter_map(|h| {
            Some(SimilarHitDto {
                face: dto_key(&h.key),
                row: rows.get(&h.key)?.clone(),
                distance: h.distance,
                source: source_dto(h.source),
                same: axis_names(&h.same),
                differ: axis_names(&h.differ),
            })
        })
        .collect())
}

pub fn similar(
    state: &AppState,
    face: &FaceKeyDto,
    opts: &SimilarOptsDto,
) -> Result<Vec<SimilarHitDto>, CmdError> {
    let b = bundle(state)?;
    let q = ix_of(&b, face)?;
    let k = (opts.k as usize).clamp(1, 100);
    let source_ids = opts.source_ids.clone();
    let range = opts.weight_range;
    let vi_only = opts.vi_only;
    let filter = |p: &FaceProfile| {
        let Some(m) = b.meta.get(&p.key) else {
            return false;
        };
        !m.missing
            && (!vi_only || m.vi_supported)
            && (source_ids.is_empty() || m.source_ids.iter().any(|s| source_ids.contains(s)))
            && range.is_none_or(|[lo, hi]| p.weight_class.is_some_and(|w| w >= lo && w <= hi))
    };
    let w = GroupWeights {
        proportion: opts.weights.proportion,
        weight: opts.weights.weight,
        structure: opts.weights.structure,
        detail: opts.weights.detail,
    };
    let so = SimilarOptions {
        weights: w,
        embed_weight: if opts.use_image { None } else { Some(0.0) },
        k,
    };
    let hits = match b.index.similar_with(&q, &so, &filter) {
        Ok(h) => h,
        // chỉ chữ hoa / không phải Latin: so theo ảnh nếu có
        Err(IndexError::NotComparable(_)) if opts.use_image && b.index.has_image(&q) => {
            b.index.similar_image_only(&q, k, &filter)?
        }
        Err(e) => return Err(e.into()),
    };
    hits_dto(state, &b, hits)
}

/// Stable family pages: rank once over all filtered faces, then slice the family list.
pub fn similar_families(
    state: &AppState,
    face: &FaceKeyDto,
    opts: &SimilarOptsDto,
    q: &FaceQueryDto,
) -> Result<SimilarFamilyPageDto, CmdError> {
    if opts.weight_range.is_some_and(|[lo, hi]| lo > hi) {
        return Err(CmdError::invalid_arg("weightRange đảo ngược"));
    }
    let b = bundle(state)?;
    let key = ix_of(&b, face)?;
    let allowed = crate::services::library::matching_keys(state, q)?;
    // A Latin source font should not be matched to CJK/Indic families by their Latin subset
    // (real system fonts: Avenir Next -> Hiragino, Sangam MN). Non-Latin sources keep all.
    let latin_source = !b.meta.get(&key).is_some_and(|m| m.non_latin_primary);
    let filter = |p: &FaceProfile| {
        allowed.contains(&p.key)
            && (!latin_source || !b.meta.get(&p.key).is_some_and(|m| m.non_latin_primary))
            && (!opts.vi_only || b.meta.get(&p.key).is_some_and(|m| m.vi_supported))
            && (opts.source_ids.is_empty()
                || b.meta
                    .get(&p.key)
                    .is_some_and(|m| m.source_ids.iter().any(|s| opts.source_ids.contains(s))))
            && opts
                .weight_range
                .is_none_or(|[lo, hi]| p.weight_class.is_some_and(|w| w >= lo && w <= hi))
    };
    let o = SimilarOptions {
        weights: GroupWeights {
            proportion: opts.weights.proportion,
            weight: opts.weights.weight,
            structure: opts.weights.structure,
            detail: opts.weights.detail,
        },
        embed_weight: if opts.use_image { None } else { Some(0.0) },
        k: 0,
    };
    let mut groups = b.index.similar_families(&key, &o, &filter, 0, b.len())?;
    // macOS hidden system families (".Aqua Kana", ".New York", ".Hiragino … Interface") are UI
    // fonts that design apps do not list; never suggest them (seen on real /System/Library/Fonts).
    groups.retain(|g| !is_hidden_family(&g.family));
    let total = groups.len() as u32;
    let limit = if q.limit == 0 { 12 } else { q.limit.min(100) } as usize;
    let selected: Vec<_> = groups
        .into_iter()
        .skip(q.offset as usize)
        .take(limit)
        .collect();
    let hits = hits_dto(
        state,
        &b,
        selected.iter().map(|g| g.representative.clone()).collect(),
    )?;
    let by_key: HashMap<_, _> = hits
        .into_iter()
        .map(|h| ((h.face.hash_hex.clone(), h.face.face_index), h))
        .collect();
    let items = selected
        .into_iter()
        .filter_map(|g| {
            Some(SimilarFamilyDto {
                family: g.family,
                representative: by_key
                    .get(&(
                        g.representative.key.hash.clone(),
                        g.representative.key.index,
                    ))?
                    .clone(),
                count: g.count as u32,
                faces: g.faces.iter().map(dto_key).collect(),
            })
        })
        .collect();
    Ok(SimilarFamilyPageDto {
        total,
        offset: q.offset,
        items,
    })
}

/// Family names starting with '.' are macOS-private system fonts.
pub fn is_hidden_family(family: &str) -> bool {
    family.starts_with('.')
}

#[cfg(test)]
mod hidden_family_tests {
    #[test]
    fn dot_prefixed_families_are_hidden() {
        assert!(super::is_hidden_family(".Aqua Kana"));
        assert!(super::is_hidden_family(".New York"));
        assert!(!super::is_hidden_family("New York"));
        assert!(!super::is_hidden_family("Avenir Next"));
    }
}

/// Raw units from Core (never reference percentile units).
pub fn axis_unit(axis: &str) -> Option<&'static str> {
    Some(match axis {
        "x_height_em" | "cap_height_em" | "ascender_em" | "descender_em" => "em",
        "x_height_cap" => "cap_height_ratio",
        "stem" | "aperture_e" | "width" | "mean_advance" | "spacing" => "x_height_ratio",
        "stress_deg" | "slant_deg" | "aperture" => "deg",
        "darkness" | "counter" => "area_ratio",
        "contrast" | "contrast_h" | "width_cv" | "serif_foot" | "terminal" => "ratio",
        "a_storeys" | "g_storeys" => "count",
        _ => return None,
    })
}

pub fn axis_stats(state: &AppState, q: &FaceQueryDto) -> Result<Vec<AxisStatsDto>, CmdError> {
    // Reject invalid ranges even though range filters are omitted from domain calculation.
    crate::services::library::validate_raw_ranges(&q.raw_axis_ranges)?;
    let b = bundle(state)?;
    let mut query = q.clone();
    query.axis_filters.clear();
    query.raw_axis_ranges.clear();
    query.hard.clear();
    let allowed = crate::services::library::matching_keys(state, &query)?;
    Ok(AxisId::ALL
        .iter()
        .copied()
        .map(|a| {
            let values = b
                .index
                .profiles_for_test()
                .iter()
                .filter(|p| allowed.contains(&p.key))
                .filter_map(|p| crate::services::library::raw_reading(p, a))
                .collect();
            stats_of(a, values)
        })
        .collect())
}

pub const HISTOGRAM_BINS_DEFAULT: u32 = 24;
const HISTOGRAM_BINS_MAX: u32 = 64;

/// Histogram cho 16 trục đo + `weight_class` + `width_class`. Mỗi trục đếm trên tập thoả mọi bộ lọc
/// khác, **trừ chính trục đó** (bộ lọc `rawAxisRanges`/`axisFilters` cùng trục và `weightClass`/`widthClass`).
/// Miền `lo..hi` lấy p1–p99 của tập chưa áp bộ lọc trục nào, nên không đổi khi người dùng kéo slider.
/// `bins` = 0 → mặc định 24; tối đa 64. Trục rời rạc luôn 9 bin.
pub fn axis_histograms(state: &AppState, q: &FaceQueryDto, bins: u32) -> Result<Vec<AxisHistogramDto>, CmdError> {
    use crate::services::library::{matching_keys, raw_matches, raw_reading, scope_of, to_store_query};
    crate::services::library::validate_raw_ranges(&q.raw_axis_ranges)?;
    crate::services::library::validate_class_ranges(q)?;
    let nb = if bins == 0 { HISTOGRAM_BINS_DEFAULT } else { bins.clamp(2, HISTOGRAM_BINS_MAX) } as usize;
    let b = bundle(state)?;
    // Tập nền: mọi bộ lọc không phải trục đo (text, class, ngôn ngữ, license, weight/width class…).
    let mut base = q.clone();
    base.axis_filters.clear();
    base.raw_axis_ranges.clear();
    base.hard.clear();
    let allowed = matching_keys(state, &base)?;
    let id_of = |k: &fontatlas_index::FaceKey| b.meta.get(k).map(|m| m.face_id);
    let hard = crate::services::describe::hard_of(&q.hard);
    let scope_ids = |filters: &[AxisFilterDto]| -> Option<std::collections::HashSet<i64>> {
        b.scope_for(filters, &hard).map(|v| v.into_iter().map(|x| x.0).collect())
    };
    let full_scope = scope_ids(&q.axis_filters);
    let mut out: Vec<AxisHistogramDto> = Vec::new();
    for a in AxisId::ALL.iter().copied() {
        let key = a.key();
        let others: Vec<RawAxisRangeDto> = q.raw_axis_ranges.iter().filter(|r| r.axis != key).cloned().collect();
        let other_filters: Vec<AxisFilterDto> = q.axis_filters.iter().filter(|f| f.axis != key).cloned().collect();
        let own_scope;
        let scope = if other_filters.len() == q.axis_filters.len() {
            &full_scope
        } else {
            own_scope = scope_ids(&other_filters);
            &own_scope
        };
        let mut domain: Vec<f32> = Vec::new();
        let mut counted: Vec<f32> = Vec::new();
        for p in b.index.profiles_for_test().iter().filter(|p| allowed.contains(&p.key)) {
            let Some(v) = raw_reading(p, a) else { continue };
            domain.push(v);
            let ok = raw_matches(p, &others) && scope.as_ref().is_none_or(|s| id_of(&p.key).is_some_and(|id| s.contains(&id)));
            if ok {
                counted.push(v);
            }
        }
        let st = stats_of(a, domain);
        let (lo, hi) = (st.p1.or(st.min).unwrap_or(0.0), st.p99.or(st.max).unwrap_or(0.0));
        let span = (hi - lo).max(f32::EPSILON);
        let mut hist = vec![0u32; nb];
        for v in &counted {
            let i = (((v - lo) / span) * nb as f32).floor().clamp(0.0, (nb - 1) as f32) as usize;
            hist[i] += 1;
        }
        out.push(AxisHistogramDto { axis: key.into(), unit: st.unit, count: counted.len() as u32, lo, hi, bins: hist });
    }
    // Trục rời rạc: đếm bằng SQL trên tập (trừ chính nó), áp luôn các bộ lọc trục đo (scope).
    let scope = scope_of(state, q)?;
    let sq = to_store_query(q);
    for (axis, which, lo, hi) in [
        ("weight_class", fontatlas_store::ClassAxis::Weight, 100.0, 900.0),
        ("width_class", fontatlas_store::ClassAxis::Width, 1.0, 9.0),
    ] {
        let bins = state.with_store(|s| Ok(s.class_histogram(&sq, scope.as_deref(), which)?))?;
        out.push(AxisHistogramDto {
            axis: axis.into(),
            unit: "class".into(),
            count: bins.iter().sum(),
            lo,
            hi,
            bins: bins.to_vec(),
        });
    }
    Ok(out)
}

/// Đếm face theo class/subclass trong tập thoả mọi bộ lọc *khác* (bỏ `classes`/`subclasses`
/// để facet không tự triệt tiêu chính nó).
pub fn class_facets(state: &AppState, q: &FaceQueryDto) -> Result<Vec<ClassFacetDto>, CmdError> {
    let mut query = q.clone();
    query.classes.clear();
    query.subclasses.clear();
    let allowed = crate::services::library::matching_keys(state, &query)?;
    let b = bundle(state)?;
    let eff = crate::services::library::effective_classes(state)?;
    let mut classified = 0usize;
    let mut by: std::collections::BTreeMap<String, (u32, std::collections::BTreeMap<String, u32>)> = Default::default();
    for (k, m) in &b.meta {
        if !allowed.contains(k) {
            continue;
        }
        let Some((c, sub)) = eff.get(&m.face_id) else { continue };
        classified += 1;
        let e = by.entry(c.clone()).or_default();
        e.0 += 1;
        if let Some(s) = sub {
            *e.1.entry(s.clone()).or_default() += 1;
        }
    }
    // Face thoả bộ lọc nhưng không có hồ sơ trong Index (chưa phân tích / JSON hỏng): tách thành bucket riêng
    // để tổng facet = tổng thư viện.
    let unclassified = allowed.len().saturating_sub(classified) as u32;
    let mut out: Vec<ClassFacetDto> = by
        .into_iter()
        .map(|(class, (count, subs))| ClassFacetDto {
            class,
            count,
            subclasses: subs.into_iter().map(|(subclass, count)| SubclassFacetDto { subclass, count }).collect(),
        })
        .collect();
    if unclassified > 0 {
        out.push(ClassFacetDto { class: "unclassified".into(), count: unclassified, subclasses: Vec::new() });
    }
    Ok(out)
}

fn stats_of(axis: AxisId, mut values: Vec<f32>) -> AxisStatsDto {
    values.retain(|v| v.is_finite());
    values.sort_by(f32::total_cmp);
    let percentile = |p: f64| -> Option<f32> {
        if values.is_empty() {
            return None;
        }
        let x = (values.len() - 1) as f64 * p;
        let lo = x.floor() as usize;
        let hi = x.ceil() as usize;
        Some((values[lo] as f64 * (1.0 - x.fract()) + values[hi] as f64 * x.fract()) as f32)
    };
    AxisStatsDto {
        axis: axis.key().into(),
        unit: axis_unit(axis.key()).unwrap_or("ratio").into(),
        count: values.len() as u32,
        min: values.first().copied(),
        max: values.last().copied(),
        p1: percentile(0.01),
        p99: percentile(0.99),
    }
}

fn gate_str(g: Gate) -> &'static str {
    match g {
        Gate::Safe => "safe",
        Gate::Controlled => "controlled",
        Gate::Experimental => "experimental",
        Gate::Blocked => "blocked",
    }
}

fn pair_axis_str(a: PairAxis) -> &'static str {
    match a {
        PairAxis::XHeight => "x_height",
        PairAxis::Weight => "weight",
        PairAxis::Contrast => "contrast",
        PairAxis::Serif => "serif",
        PairAxis::Width => "width",
        PairAxis::Aperture => "aperture",
        PairAxis::Stress => "stress",
        PairAxis::Slant => "slant",
    }
}

fn relation(r: Relation) -> (&'static str, &'static str) {
    match r {
        Relation::Harmony => ("harmony", "●"),
        Relation::Contrast => ("contrast", "◐"),
        Relation::ContrastWithSerif => ("contrast_with_serif", "◐"),
        Relation::Clash => ("clash", "◆"),
        Relation::Unsure => ("unsure", "○"),
        Relation::WeightHint => ("weight_hint", "◑"),
    }
}

pub(crate) fn candidate_dto(
    x: &PairAssessment,
    pa: &FaceProfile,
    pb: &FaceProfile,
    row: FaceRowDto,
    lang: Lang,
) -> PairCandidateDto {
    PairCandidateDto {
        face: dto_key(&pb.key),
        row,
        gate: gate_str(x.gate).into(),
        labels: x
            .labels
            .iter()
            .map(|l| {
                let (rel, sym) = relation(l.relation);
                PairLabelDto {
                    axis: pair_axis_str(l.axis).into(),
                    relation: rel.into(),
                    symbol: sym.into(),
                    delta: l.delta,
                    a: l.a,
                    b: l.b,
                    role_reversed: l.role_reversed,
                }
            })
            .collect(),
        clashes: x
            .clashes
            .iter()
            .map(|c| match c {
                ClashKind::Axis(a) => format!("axis:{}", pair_axis_str(*a)),
                ClashKind::Global => "global".into(),
                ClashKind::Skeleton => "skeleton".into(),
            })
            .collect(),
        body_warnings: x
            .body_warnings
            .iter()
            .map(|w| {
                match w {
                    BodyWarning::HighContrast => "high_contrast",
                    BodyWarning::WeightOutOfRange => "weight_out_of_range",
                    BodyWarning::MissingRegularOrBold => "missing_regular_or_bold",
                    BodyWarning::LowXHeight => "low_x_height",
                }
                .into()
            })
            .collect(),
        heading_overflow: x.heading_overflow,
        insufficient_data: x.insufficient_data,
        weight_hint: x.weight_hint.map(|v| fmt_num(v, lang)),
        block: x.block.map(|Block::MissingVietnamese { side, missing }| {
            format!(
                "missing_vietnamese:{}:{missing}",
                if side == Side::A { "a" } else { "b" }
            )
        }),
        sentences: explain_pair(x, pa, pb, lang),
        summary: String::new(),
    }
}

pub(crate) fn roles(o: &PairOptsDto) -> (Role, Role) {
    match o.role {
        PairRoleDto::Heading => (Role::Heading, Role::Body),
        PairRoleDto::Body => (Role::Body, Role::Heading),
        PairRoleDto::Code => (Role::Code, Role::Body),
    }
}

fn collection_faces(state: &AppState, id: i64) -> Result<std::collections::HashSet<i64>, CmdError> {
    state.with_store(|s| Ok(s.collection_face_ids(id)?.into_iter().collect()))
}

pub fn pair_candidates(
    state: &AppState,
    face: &FaceKeyDto,
    opts: &PairOptsDto,
) -> Result<Vec<PairCandidateDto>, CmdError> {
    let b = bundle(state)?;
    let q = ix_of(&b, face)?;
    let (qr, cr) = roles(opts);
    let k = (opts.k as usize).clamp(1, 100);
    let in_collection = opts
        .collection_id
        .map(|c| collection_faces(state, c))
        .transpose()?;
    let filter = |p: &FaceProfile| {
        let Some(m) = b.meta.get(&p.key) else {
            return false;
        };
        !m.missing
            && in_collection
                .as_ref()
                .is_none_or(|s| s.contains(&m.face_id))
    };
    // cổng áp TRƯỚC khi cắt top-k (lọc sau sẽ rỗng khi top-k toàn cổng khác)
    let want = opts.mode.map(|m| match m {
        PairModeDto::Safe => Gate::Safe,
        PairModeDto::Controlled => Gate::Controlled,
        PairModeDto::Experimental => Gate::Experimental,
    });
    let got = b
        .index
        .pair_candidates_gated(&q, qr, cr, opts.project_vi, k, &filter, want)?;
    let keys: Vec<IxKey> = got.iter().map(|(k, _)| k.clone()).collect();
    let rows = rows_of(state, &b, &keys)?;
    let lang = lang_of(opts.lang);
    let pa = b
        .index
        .profile(&q)
        .ok_or_else(|| CmdError::not_found("face"))?;
    let reference = b.index.reference();
    Ok(got
        .into_iter()
        .filter_map(|(key, x)| {
            let pb = b.index.profile(&key)?;
            let mut d = candidate_dto(&x, pa, pb, rows.get(&key)?.clone(), lang);
            d.summary = summarize_face(pb, reference, lang);
            Some(d)
        })
        .collect())
}

pub fn assess_pair(
    state: &AppState,
    a: &FaceKeyDto,
    b_key: &FaceKeyDto,
    opts: &PairOptsDto,
) -> Result<PairCandidateDto, CmdError> {
    let b = bundle(state)?;
    let (ka, kb) = (ix_of(&b, a)?, ix_of(&b, b_key)?);
    let (ra, rb) = roles(opts);
    let x = b.index.assess_pair(&ka, ra, &kb, rb, opts.project_vi)?;
    let rows = rows_of(state, &b, std::slice::from_ref(&kb))?;
    let (pa, pb) = (b.index.profile(&ka), b.index.profile(&kb));
    let (Some(pa), Some(pb), Some(row)) = (pa, pb, rows.get(&kb)) else {
        return Err(CmdError::not_found("face"));
    };
    let lang = lang_of(opts.lang);
    let mut d = candidate_dto(&x, pa, pb, row.clone(), lang);
    d.summary = summarize_face(pb, b.index.reference(), lang);
    Ok(d)
}

// ---- chi tiết face ----

fn reading_json(key: &str, m: &Value, percentile: Option<f32>) -> AxisReadingDto {
    let q = m.get("quality");
    AxisReadingDto {
        key: key.into(),
        value: m.get("value").filter(|v| !v.is_null()).cloned(),
        raw_value: m
            .get("value")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite()),
        unit: if key == "stem" && m.get("method").and_then(Value::as_str) == Some("H_stem_over_cap")
        {
            Some("cap_height_ratio".into())
        } else {
            axis_unit(key).map(String::from)
        },
        quality: q
            .and_then(|q| q.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("failed")
            .into(),
        reason: q
            .and_then(|q| q.get("reason"))
            .and_then(Value::as_str)
            .map(String::from),
        method: m.get("method").and_then(Value::as_str).map(String::from),
        percentile,
    }
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn opt_str(m: &Value, k: &str) -> Option<String> {
    m.get(k).and_then(Value::as_str).map(String::from)
}

fn file_format(path: Option<&str>, container: Option<&str>, outline: &str) -> String {
    if let Some(ext) = path
        .and_then(|p| std::path::Path::new(p).extension())
        .and_then(|e| e.to_str())
    {
        let e = ext.to_ascii_lowercase();
        if ["ttf", "otf", "ttc", "otc", "woff", "woff2"].contains(&e.as_str()) {
            return e;
        }
    }
    match (container, outline) {
        (Some("woff1"), _) => "woff",
        (Some("woff2"), _) => "woff2",
        (Some("collection"), "cff" | "cff2") => "otc",
        (Some("collection"), _) => "ttc",
        (_, "cff" | "cff2") => "otf",
        _ => "ttf",
    }
    .into()
}

pub fn get_face(
    state: &AppState,
    face: &FaceKeyDto,
    lang: LangDto,
) -> Result<FaceDetailDto, CmdError> {
    let b = bundle(state)?;
    let key = ix_of(&b, face)?;
    let sk = store_key(&key).ok_or_else(|| CmdError::invalid_arg("khoá face"))?;
    let a = state
        .with_store(|s| Ok(s.analysis_of(&sk)?))?
        .ok_or_else(|| CmdError::not_found("chưa có bản phân tích của face này"))?;
    let path = state
        .with_store(|s| Ok(s.face_path(&sk)?))
        .ok()
        .flatten()
        .map(|p| p.to_string_lossy().into_owned());
    let parse = |s: &str| {
        serde_json::from_str::<Value>(s)
            .map_err(|e| CmdError::internal(format!("JSON phân tích hỏng: {e}")))
    };
    let (m, v, an) = (
        parse(&a.metadata_json)?,
        parse(&a.vietnamese_json)?,
        parse(&a.anatomy_json)?,
    );
    let profile = b
        .index
        .profile(&key)
        .ok_or_else(|| CmdError::not_found("face"))?;
    let reference = b.index.reference();
    let l = lang_of(lang);

    let mut axes = Vec::new();
    for ax in AxisId::ALL {
        let raw = an.get(ax.key()).cloned().unwrap_or(Value::Null);
        let rd = profile.get(ax);
        let pct = rd
            .value
            .filter(|_| rd.q != Q::Failed)
            .and_then(|val| match ax {
                AxisId::StressDeg | AxisId::SlantDeg => {
                    reference.magnitude_percentile(ax, &rd.method, val)
                }
                AxisId::SerifFoot => None,
                _ => reference.percentile(ax, &rd.method, val),
            });
        let mut d = reading_json(ax.key(), &raw, pct);
        if ax == AxisId::SerifFoot
            && let Some(val) = rd.value.filter(|_| rd.q != Q::Failed)
        {
            d.value = Some(Value::String(
                if val < 1.2 {
                    "sans"
                } else if val > 1.6 {
                    "serif"
                } else {
                    "uncertain"
                }
                .into(),
            ));
        }
        axes.push(d);
    }
    for extra in [
        // Stem/bar of H: stays stable on condensed faces where o-contrast inflates (eval 2026-11-07).
        "contrast_h",
        "cap_height_em",
        "descender_em",
        "terminal",
        "a_storeys",
        "g_storeys",
    ] {
        axes.push(reading_json(
            extra,
            an.get(extra).unwrap_or(&Value::Null),
            None,
        ));
    }

    let outline = opt_str(&m, "outline_format").unwrap_or_else(|| "none".into());
    let marks = v.get("marks").cloned().unwrap_or(Value::Null);
    let classification_info = classification_of(state, &sk, &m, &an)?;
    let classification = match classification_info.class.as_str() {
        "uncertain" => None,
        _ => Some(classification_info.label.clone()),
    };
    let rows = rows_of(state, &b, std::slice::from_ref(&key))?;
    let row = rows
        .get(&key)
        .cloned()
        .ok_or_else(|| CmdError::not_found("hàng của face"))?;
    let has_image = b.index.has_image(&key);
    let similar_top = similar(
        state,
        face,
        &SimilarOptsDto {
            k: 3,
            ..Default::default()
        },
    )
    .unwrap_or_default();

    let named_instances = m
        .get("named_instance_coords")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|i| InstanceDto {
                    name: opt_str(i, "name").unwrap_or_default(),
                    coords: i
                        .get("coords")
                        .and_then(Value::as_array)
                        .map(|c| {
                            c.iter()
                                .filter_map(|p| {
                                    Some(AxisCoordDto {
                                        tag: p.get(0)?.as_str()?.into(),
                                        value: p.get(1)?.as_f64()? as f32,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    let variable_axes = m
        .get("axes")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|x| {
                    Some(VariableAxisDto {
                        tag: x.get("tag")?.as_str()?.into(),
                        min: x.get("min")?.as_f64()? as f32,
                        default: x.get("default")?.as_f64()? as f32,
                        max: x.get("max")?.as_f64()? as f32,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let opsz = m
        .get("os2_optical_size_pt")
        .and_then(Value::as_array)
        .and_then(|x| Some([x.first()?.as_f64()? as f32, x.get(1)?.as_f64()? as f32]));
    let flag = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);

    Ok(FaceDetailDto {
        row,
        summary: summarize_face(profile, reference, l),
        classification,
        classification_info,
        source: if has_image { "axes_and_image" } else { "axes" }.into(),
        has_image,
        file: FileInfoDto {
            format: file_format(path.as_deref(), a.container.as_deref(), &outline),
            container: a.container.clone(),
            outline,
            num_glyphs: m.get("num_glyphs").and_then(Value::as_u64).unwrap_or(0) as u32,
            size_bytes: a.file_size,
            path,
        },
        metadata: MetadataDto {
            license: opt_str(&m, "license_description"),
            license_url: opt_str(&m, "license_url"),
            designer: opt_str(&m, "designer"),
            manufacturer: opt_str(&m, "manufacturer"),
            version: opt_str(&m, "version"),
            copyright: opt_str(&m, "copyright"),
            vendor_url: opt_str(&m, "vendor_url"),
            variable_axes,
            named_instances,
            scripts: strs(m.get("scripts")),
            opsz_range: opsz,
            features: FeaturesDto {
                gsub: strs(m.get("gsub_features")),
                gpos: strs(m.get("gpos_features")),
            },
        },
        vietnamese: VietnameseDto {
            status: opt_str(&v, "status").unwrap_or_default(),
            supports: flag("supports_vietnamese"),
            missing: strs(v.get("missing")),
            // required_total gồm 52 chữ ASCII a–z/A–Z; "134" là số chữ tiếng Việt (có dấu hoặc đ)
            total: v
                .get("required_total")
                .and_then(Value::as_u64)
                .map_or(134, |t| t.saturating_sub(52).max(1)) as u32,
            precomposed_composite: v
                .get("precomposed_composite")
                .and_then(Value::as_u64)
                .is_some_and(|n| n > 0),
            gpos_mark: flag("gpos_mark"),
            gpos_mkmk: flag("gpos_mkmk"),
            gsub_ccmp: flag("gsub_ccmp"),
            dong_sign: flag("dong_sign"),
            stacked_marks_top_em: v
                .get("stacked_marks_top_em")
                .and_then(Value::as_f64)
                .map(|x| x as f32),
            stacked_marks_overflow: flag("stacked_marks_overflow"),
            stack_style: marks
                .get("stack_style")
                .and_then(Value::as_str)
                .map(String::from),
            mark_weight_ratio: reading_json(
                "mark_weight_ratio",
                marks.get("mark_weight_ratio").unwrap_or(&Value::Null),
                None,
            ),
        },
        axes,
        similar_top,
        extras: extras_of(&m),
    })
}

#[cfg(test)]
mod raw_stats_tests {
    use super::*;
    #[test]
    fn empty_single_constant_nan_and_outlier() {
        let a = AxisId::Width;
        let s = stats_of(a, vec![f32::NAN, f32::INFINITY]);
        assert_eq!(
            (s.count, s.min, s.max, s.p1, s.p99),
            (0, None, None, None, None)
        );
        for v in [vec![2.0], vec![2.0; 10]] {
            let s = stats_of(a, v);
            assert_eq!(
                (s.min, s.max, s.p1, s.p99),
                (Some(2.0), Some(2.0), Some(2.0), Some(2.0))
            );
        }
        let mut v = vec![1.0; 100];
        v.push(1000.0);
        let s = stats_of(a, v);
        assert_eq!(
            (s.count, s.min, s.max, s.p1, s.p99),
            (101, Some(1.0), Some(1000.0), Some(1.0), Some(1.0))
        );
        let s = stats_of(a, vec![0.0, 100.0]);
        assert_eq!((s.p1, s.p99), (Some(1.0), Some(99.0)));
    }
}

/// Phân loại tự động từ JSON đã lưu, rồi áp ghi đè của người dùng (ưu tiên hơn).
pub fn classification_of(state: &AppState, key: &FaceKey, meta: &Value, anatomy: &Value) -> Result<ClassificationDto, CmdError> {
    use fontatlas_core::classify::{Class, ClassifyInput, Usage, classify, usage_for};
    let input = ClassifyInput::from_json(meta, anatomy);
    let auto = classify(&input);
    let ov = state.with_user(|u| Ok(u.classification_override(key)?))?;
    let usage_str = |u: &[Usage]| -> Vec<String> {
        u.iter()
            .map(|x| match x {
                Usage::Text => "text",
                Usage::Display => "display",
                Usage::Ui => "ui",
            })
            .map(String::from)
            .collect()
    };
    let signals: Vec<ClassSignalDto> = auto
        .signals
        .iter()
        .map(|s| ClassSignalDto { source: s.source.clone(), detail: s.detail.clone(), vote: s.vote.map_or("usage", |c| c.snake()).into(), weight: s.weight })
        .collect();
    let snake = |c: Class| c.snake().to_string();
    if let Some(o) = ov
        && let Some(c) = Class::parse(&o.class)
    {
        return Ok(ClassificationDto {
            class: snake(c),
            label: c.label().into(),
            subclass: o.subclass,
            confidence: 1.0,
            candidate: None,
            traits: auto.traits.clone(),
            signals,
            usage: usage_str(&usage_for(&input, c)),
            source: "override".into(),
            auto_class: Some(snake(auto.class)),
        });
    }
    Ok(ClassificationDto {
        class: snake(auto.class),
        label: auto.class.label().into(),
        subclass: auto.subclass,
        confidence: auto.confidence,
        candidate: auto.candidate.map(snake),
        traits: auto.traits,
        signals,
        usage: usage_str(&auto.usage),
        source: "auto".into(),
        auto_class: None,
    })
}

/// Ghi đè phân loại của người dùng (lưu ở user.db). `class` và `subclass` phải thuộc taxonomy.
pub fn set_classification_override(state: &AppState, o: &ClassificationOverrideDto) -> Result<(), CmdError> {
    use fontatlas_core::classify::{Class, subclasses_of};
    let c = Class::parse(&o.class).filter(|c| *c != Class::Uncertain).ok_or_else(|| CmdError::invalid_arg(format!("class không hợp lệ: {}", o.class)))?;
    if let Some(sub) = &o.subclass
        && !subclasses_of(c).contains(&sub.as_str())
    {
        return Err(CmdError::invalid_arg(format!("subclass {sub} không thuộc {}", c.label())));
    }
    let k = FaceKey::try_from(&o.face)?;
    if state.with_store(|s| Ok(s.face_ids_of(&[k])?))?.is_empty() {
        return Err(CmdError::not_found("không có face này trong thư viện"));
    }
    state.with_user(|u| Ok(u.set_classification_override(&k, &o.class.to_ascii_lowercase(), o.subclass.as_deref())?))?;
    crate::services::library::bump_class_generation();
    Ok(())
}

pub fn clear_classification_override(state: &AppState, face: &FaceKeyDto) -> Result<(), CmdError> {
    let k = FaceKey::try_from(face)?;
    state.with_user(|u| Ok(u.clear_classification_override(&k)?))?;
    crate::services::library::bump_class_generation();
    Ok(())
}

/// Dữ liệu D2 từ `metadata_json` (rỗng với phân tích cũ chưa chạy lại).
pub(crate) fn extras_of(m: &Value) -> FaceExtrasDto {
    let x: fontatlas_core::extras::FontExtras = m.get("extras").and_then(|e| serde_json::from_value(e.clone()).ok()).unwrap_or_default();
    FaceExtrasDto {
        extras_available: m.get("extras").is_some_and(|e| e.is_object()),
        vendor_id: opt_str(m, "vendor_id"),
        years: YearsDto { created: x.years.created, modified: x.years.modified, copyright: x.years.copyright },
        ot_groups: x
            .ot_groups
            .into_iter()
            .map(|g| OtGroupDto {
                group: g.group,
                tags: g.tags,
                sets: g.sets.into_iter().map(|s| StylisticSetDto { tag: s.tag, name: s.name }).collect(),
            })
            .collect(),
        kerning: KerningDto { source: if x.kerning.source.is_empty() { "none".into() } else { x.kerning.source }, pairs: x.kerning.pairs, class_pairs: x.kerning.class_pairs },
        style: StyleStructureDto {
            italic_kind: if x.style.italic_kind.is_empty() { "upright".into() } else { x.style.italic_kind },
            ital_axis: x.style.ital_axis,
            slnt_axis: x.style.slnt_axis,
            wght_range: x.style.wght_range,
            wdth_range: x.style.wdth_range,
            opsz_range: x.style.opsz_range,
        },
        embedding: EmbeddingDto {
            fs_type: x.embedding.fs_type,
            level: if x.embedding.level.is_empty() { "unknown".into() } else { x.embedding.level },
            flags: x.embedding.flags,
        },
        license_kind: if x.license_kind.is_empty() { "unknown".into() } else { x.license_kind },
    }
}

#[cfg(test)]
mod extras_tests {
    use super::extras_of;
    use serde_json::json;

    #[test]
    fn old_analysis_without_extras_is_flagged_unavailable() {
        let old = extras_of(&json!({ "vendor_id": "GOOG", "family": "X" }));
        assert!(!old.extras_available, "phân tích cũ phải báo extrasAvailable=false");
        // mặc định không được hiểu là "font không có kerning": cờ mới là cách phân biệt
        assert_eq!(old.kerning.source, "none");
        assert_eq!(old.vendor_id.as_deref(), Some("GOOG"));
        let new = extras_of(&json!({ "extras": { "kerning": { "source": "gpos", "pairs": 3, "class_pairs": 1 } } }));
        assert!(new.extras_available);
        assert_eq!(new.kerning.source, "gpos");
        assert!(!extras_of(&json!({ "extras": null })).extras_available);
    }
}
