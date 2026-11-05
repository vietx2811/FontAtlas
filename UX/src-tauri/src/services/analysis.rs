//! Chi tiết face, Similar và Pairing trên Index (không cần model: thiếu ảnh thì chạy theo trục).
use std::collections::HashMap;
use std::sync::Arc;

use fontatlas_index::{
    AxisId, Block, BodyWarning, ClashKind, FaceKey as IxKey, FaceProfile, Gate, GroupWeights, IndexError, Lang, PairAssessment,
    PairAxis, Q, Relation, Role, SimilarHit, SimilarOptions, SimilarSource, Side, explain_pair, fmt_num, summarize_face,
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

fn lang_of(l: LangDto) -> Lang {
    match l {
        LangDto::Vi => Lang::Vi,
        LangDto::En => Lang::En,
    }
}

fn dto_key(k: &IxKey) -> FaceKeyDto {
    FaceKeyDto { hash_hex: k.hash.clone(), face_index: k.index }
}

fn ix_of(b: &IndexBundle, k: &FaceKeyDto) -> Result<IxKey, CmdError> {
    let sk = FaceKey::try_from(k)?;
    let key = ix_key(&sk);
    if b.meta.contains_key(&key) { Ok(key) } else { Err(CmdError::not_found("không có face này trong Index")) }
}

/// Hàng lưới của các face theo thứ tự đưa vào (một truy vấn, không N+1).
fn rows_of(state: &AppState, b: &IndexBundle, keys: &[IxKey]) -> Result<HashMap<IxKey, FaceRowDto>, CmdError> {
    if keys.is_empty() {
        return Ok(HashMap::new());
    }
    let n = keys.len() as f32;
    let scope: Vec<(i64, f32)> =
        keys.iter().enumerate().filter_map(|(i, k)| Some((b.meta.get(k)?.face_id, n - i as f32))).collect();
    let q = FaceQuery { include_missing: true, sort: SortKey::Match, limit: 1000, ..Default::default() };
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

fn hits_dto(state: &AppState, b: &IndexBundle, hits: Vec<SimilarHit>) -> Result<Vec<SimilarHitDto>, CmdError> {
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

pub fn similar(state: &AppState, face: &FaceKeyDto, opts: &SimilarOptsDto) -> Result<Vec<SimilarHitDto>, CmdError> {
    let b = bundle(state)?;
    let q = ix_of(&b, face)?;
    let k = (opts.k as usize).clamp(1, 100);
    let source_ids = opts.source_ids.clone();
    let range = opts.weight_range;
    let vi_only = opts.vi_only;
    let filter = |p: &FaceProfile| {
        let Some(m) = b.meta.get(&p.key) else { return false };
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
    let so = SimilarOptions { weights: w, embed_weight: if opts.use_image { None } else { Some(0.0) }, k };
    let hits = match b.index.similar_with(&q, &so, &filter) {
        Ok(h) => h,
        // chỉ chữ hoa / không phải Latin: so theo ảnh nếu có
        Err(IndexError::NotComparable(_)) if opts.use_image && b.index.has_image(&q) => b.index.similar_image_only(&q, k, &filter)?,
        Err(e) => return Err(e.into()),
    };
    hits_dto(state, &b, hits)
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

fn candidate_dto(
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
            format!("missing_vietnamese:{}:{missing}", if side == Side::A { "a" } else { "b" })
        }),
        sentences: explain_pair(x, pa, pb, lang),
        summary: String::new(),
    }
}

fn roles(o: &PairOptsDto) -> (Role, Role) {
    match o.role {
        PairRoleDto::Heading => (Role::Heading, Role::Body),
        PairRoleDto::Body => (Role::Body, Role::Heading),
        PairRoleDto::Code => (Role::Code, Role::Body),
    }
}

fn collection_faces(state: &AppState, id: i64) -> Result<std::collections::HashSet<i64>, CmdError> {
    state.with_store(|s| Ok(s.collection_face_ids(id)?.into_iter().collect()))
}

pub fn pair_candidates(state: &AppState, face: &FaceKeyDto, opts: &PairOptsDto) -> Result<Vec<PairCandidateDto>, CmdError> {
    let b = bundle(state)?;
    let q = ix_of(&b, face)?;
    let (qr, cr) = roles(opts);
    let k = (opts.k as usize).clamp(1, 100);
    let in_collection = opts.collection_id.map(|c| collection_faces(state, c)).transpose()?;
    let filter = |p: &FaceProfile| {
        let Some(m) = b.meta.get(&p.key) else { return false };
        !m.missing && in_collection.as_ref().is_none_or(|s| s.contains(&m.face_id))
    };
    let mut got = b.index.pair_candidates_filtered(&q, qr, cr, opts.project_vi, k, &filter)?;
    if let Some(mode) = opts.mode {
        let want = match mode {
            PairModeDto::Safe => Gate::Safe,
            PairModeDto::Controlled => Gate::Controlled,
            PairModeDto::Experimental => Gate::Experimental,
        };
        got.retain(|(_, x)| x.gate == want);
    }
    let keys: Vec<IxKey> = got.iter().map(|(k, _)| k.clone()).collect();
    let rows = rows_of(state, &b, &keys)?;
    let lang = lang_of(opts.lang);
    let pa = b.index.profile(&q).ok_or_else(|| CmdError::not_found("face"))?;
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

pub fn assess_pair(state: &AppState, a: &FaceKeyDto, b_key: &FaceKeyDto, opts: &PairOptsDto) -> Result<PairCandidateDto, CmdError> {
    let b = bundle(state)?;
    let (ka, kb) = (ix_of(&b, a)?, ix_of(&b, b_key)?);
    let (ra, rb) = roles(opts);
    let x = b.index.assess_pair(&ka, ra, &kb, rb, opts.project_vi)?;
    let rows = rows_of(state, &b, std::slice::from_ref(&kb))?;
    let (pa, pb) = (b.index.profile(&ka), b.index.profile(&kb));
    let (Some(pa), Some(pb), Some(row)) = (pa, pb, rows.get(&kb)) else { return Err(CmdError::not_found("face")) };
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
        quality: q.and_then(|q| q.get("kind")).and_then(Value::as_str).unwrap_or("failed").into(),
        reason: q.and_then(|q| q.get("reason")).and_then(Value::as_str).map(String::from),
        method: m.get("method").and_then(Value::as_str).map(String::from),
        percentile,
    }
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn opt_str(m: &Value, k: &str) -> Option<String> {
    m.get(k).and_then(Value::as_str).map(String::from)
}

fn file_format(path: Option<&str>, container: Option<&str>, outline: &str) -> String {
    if let Some(ext) = path.and_then(|p| std::path::Path::new(p).extension()).and_then(|e| e.to_str()) {
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

pub fn get_face(state: &AppState, face: &FaceKeyDto, lang: LangDto) -> Result<FaceDetailDto, CmdError> {
    let b = bundle(state)?;
    let key = ix_of(&b, face)?;
    let sk = store_key(&key).ok_or_else(|| CmdError::invalid_arg("khoá face"))?;
    let a = state
        .with_store(|s| Ok(s.analysis_of(&sk)?))?
        .ok_or_else(|| CmdError::not_found("chưa có bản phân tích của face này"))?;
    let path = state.with_store(|s| Ok(s.face_path(&sk)?)).ok().flatten().map(|p| p.to_string_lossy().into_owned());
    let parse = |s: &str| serde_json::from_str::<Value>(s).map_err(|e| CmdError::internal(format!("JSON phân tích hỏng: {e}")));
    let (m, v, an) = (parse(&a.metadata_json)?, parse(&a.vietnamese_json)?, parse(&a.anatomy_json)?);
    let profile = b.index.profile(&key).ok_or_else(|| CmdError::not_found("face"))?;
    let reference = b.index.reference();
    let l = lang_of(lang);

    let mut axes = Vec::new();
    for ax in AxisId::ALL {
        let raw = an.get(ax.key()).cloned().unwrap_or(Value::Null);
        let rd = profile.get(ax);
        let pct = rd.value.filter(|_| rd.q != Q::Failed).and_then(|val| match ax {
            AxisId::StressDeg | AxisId::SlantDeg => reference.magnitude_percentile(ax, &rd.method, val),
            AxisId::SerifFoot => None,
            _ => reference.percentile(ax, &rd.method, val),
        });
        let mut d = reading_json(ax.key(), &raw, pct);
        if ax == AxisId::SerifFoot
            && let Some(val) = rd.value.filter(|_| rd.q != Q::Failed)
        {
            d.value = Some(Value::String(if val < 1.2 { "sans" } else if val > 1.6 { "serif" } else { "uncertain" }.into()));
        }
        axes.push(d);
    }
    for extra in ["terminal", "a_storeys", "g_storeys"] {
        axes.push(reading_json(extra, an.get(extra).unwrap_or(&Value::Null), None));
    }

    let outline = opt_str(&m, "outline_format").unwrap_or_else(|| "none".into());
    let marks = v.get("marks").cloned().unwrap_or(Value::Null);
    let width_cv = profile.get(AxisId::WidthCv);
    let classification = if width_cv.q == Q::Measured && width_cv.value.is_some_and(|x| x.abs() < 0.005) {
        Some("Mono".to_string())
    } else {
        let sf = profile.get(AxisId::SerifFoot);
        match (sf.q, sf.value) {
            (Q::Failed, _) | (_, None) => None,
            (_, Some(x)) if x < 1.2 => Some("Sans".into()),
            (_, Some(x)) if x > 1.6 => Some("Serif".into()),
            _ => None,
        }
    };
    let rows = rows_of(state, &b, std::slice::from_ref(&key))?;
    let row = rows.get(&key).cloned().ok_or_else(|| CmdError::not_found("hàng của face"))?;
    let has_image = b.index.has_image(&key);
    let similar_top =
        similar(state, face, &SimilarOptsDto { k: 3, ..Default::default() }).unwrap_or_default();

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
                                .filter_map(|p| Some(AxisCoordDto { tag: p.get(0)?.as_str()?.into(), value: p.get(1)?.as_f64()? as f32 }))
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
    let opsz = m.get("os2_optical_size_pt").and_then(Value::as_array).and_then(|x| Some([x.first()?.as_f64()? as f32, x.get(1)?.as_f64()? as f32]));
    let flag = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);

    Ok(FaceDetailDto {
        row,
        summary: summarize_face(profile, reference, l),
        classification,
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
            features: FeaturesDto { gsub: strs(m.get("gsub_features")), gpos: strs(m.get("gpos_features")) },
        },
        vietnamese: VietnameseDto {
            status: opt_str(&v, "status").unwrap_or_default(),
            supports: flag("supports_vietnamese"),
            missing: strs(v.get("missing")),
            // required_total gồm 52 chữ ASCII a–z/A–Z; "134" là số chữ tiếng Việt (có dấu hoặc đ)
            total: v.get("required_total").and_then(Value::as_u64).map_or(134, |t| t.saturating_sub(52).max(1)) as u32,
            precomposed_composite: v.get("precomposed_composite").and_then(Value::as_u64).is_some_and(|n| n > 0),
            gpos_mark: flag("gpos_mark"),
            gpos_mkmk: flag("gpos_mkmk"),
            gsub_ccmp: flag("gsub_ccmp"),
            dong_sign: flag("dong_sign"),
            stacked_marks_top_em: v.get("stacked_marks_top_em").and_then(Value::as_f64).map(|x| x as f32),
            stacked_marks_overflow: flag("stacked_marks_overflow"),
            stack_style: marks.get("stack_style").and_then(Value::as_str).map(String::from),
            mark_weight_ratio: reading_json("mark_weight_ratio", marks.get("mark_weight_ratio").unwrap_or(&Value::Null), None),
        },
        axes,
        similar_top,
    })
}

