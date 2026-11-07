//! Compare và "Pair these": so nhiều face cạnh nhau (không có điểm tổng) và thử mọi cách gán vai heading/body.
use std::collections::HashSet;

use fontatlas_index::{Role, rank_key, summarize_face};

use crate::dto::analysis::*;
use crate::dto::library::*;
use crate::error::CmdError;
use crate::services::analysis::{bundle, candidate_dto, get_face, ix_of, lang_of, rows_of};
use crate::state::AppState;

const MIN_FACES: usize = 2;
/// compareFaces: 2–8 cột (màn Compare hỗ trợ tới 8; >4 chỉ là gợi ý ở UI). pairThese giữ 2–4.
const MAX_COMPARE: usize = 8;
const MAX_PAIR: usize = 4;

/// Ngưỡng mốc gần 0: dưới đây % thay đổi không có nghĩa.
const BASE_EPS: f64 = 1e-3;

fn validate(faces: &[FaceKeyDto], max: usize) -> Result<(), CmdError> {
    if !(MIN_FACES..=max).contains(&faces.len()) {
        return Err(CmdError::invalid_arg(format!("cần {MIN_FACES}–{max} face, nhận {}", faces.len())));
    }
    let mut seen = HashSet::new();
    for f in faces {
        if !seen.insert((f.hash_hex.trim().to_ascii_lowercase(), f.face_index)) {
            return Err(CmdError::invalid_arg("các face phải khác nhau"));
        }
    }
    Ok(())
}

/// (Δ tuyệt đối, Δ%). Trục góc ("deg") và mốc ≈ 0 chỉ có Δ tuyệt đối.
fn delta(base: Option<f64>, v: Option<f64>, unit: Option<&str>) -> (Option<f64>, Option<f64>) {
    match (base, v) {
        (Some(b), Some(v)) if b.is_finite() && v.is_finite() => {
            let d = v - b;
            let pct = (unit != Some("deg") && b.abs() >= BASE_EPS).then(|| d / b.abs() * 100.0);
            (Some(d), pct)
        }
        _ => (None, None),
    }
}

fn family_rows(state: &AppState, family: &str) -> Vec<FaceRowDto> {
    let mut out = Vec::new();
    let mut offset = 0u32;
    loop {
        let q = FaceQueryDto { text: Some(family.to_string()), limit: 1000, offset, ..Default::default() };
        let page = match crate::services::library::query_faces(state, &q) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("compare: không lấy được các face của họ {family:?}: {e:?}");
                break;
            }
        };
        let n = page.items.len() as u32;
        out.extend(page.items.into_iter().filter(|r| r.family == family));
        offset = offset.saturating_add(n);
        if n == 0 || u64::from(offset) >= page.total {
            break;
        }
    }
    out
}

fn family_structure(state: &AppState, d: &FaceDetailDto) -> FamilyStructureDto {
    let rows = family_rows(state, &d.row.family);
    let mut weights: Vec<u16> = rows.iter().filter_map(|r| r.weight_class).collect();
    weights.sort_unstable();
    weights.dedup();
    let mut widths: Vec<u16> = rows.iter().filter_map(|r| r.width_class).collect();
    widths.sort_unstable();
    widths.dedup();
    let obl = |r: &FaceRowDto| r.subfamily.as_deref().is_some_and(|s| s.to_lowercase().contains("oblique") || s.to_lowercase().contains("slanted"));
    FamilyStructureDto {
        family: d.row.family.clone(),
        faces: rows.len() as u32,
        weights,
        widths,
        has_italic: rows.iter().any(|r| r.is_italic && !obl(r)) || d.extras.style.italic_kind == "italic",
        has_oblique: rows.iter().any(obl) || d.extras.style.italic_kind == "oblique",
        has_variable: rows.iter().any(|r| r.is_variable),
        has_opsz: d.extras.style.opsz_range.is_some(),
    }
}

fn vietnamese_of(d: &FaceDetailDto, asc_em: Option<f32>) -> CompareVietnameseDto {
    let cap = d.axes.iter().find(|a| a.key == "cap_height_em").and_then(|a| a.raw_value).map(|v| v as f32);
    let top = d.vietnamese.stacked_marks_top_em;
    let over = |base: Option<f32>| top.zip(base).map(|(t, b)| ((t - b) * 100.0).max(0.0));
    CompareVietnameseDto {
        supports: d.vietnamese.supports,
        missing: d.vietnamese.missing.len() as u32,
        total: d.vietnamese.total,
        stacked_marks_top_em: top,
        hhea_ascender_em: asc_em,
        cap_height_em: cap,
        over_ascender_pct_em: over(asc_em),
        over_cap_pct_em: over(cap),
        overflow: d.vietnamese.stacked_marks_overflow,
        stack_style: d.vietnamese.stack_style.clone(),
    }
}

/// hhea.ascender (em) từ `vietnamese_json`.
fn hhea_ascender(state: &AppState, face: &FaceKeyDto) -> Option<f32> {
    let k = fontatlas_store::FaceKey::try_from(face).ok()?;
    let a = state.with_store(|s| Ok(s.analysis_of(&k)?)).ok()??;
    let v: serde_json::Value = serde_json::from_str(&a.vietnamese_json).ok()?;
    v.get("hhea_ascender_em")?.as_f64().map(|x| x as f32)
}

pub fn compare_faces(state: &AppState, faces: &[FaceKeyDto], opts: &CompareOptsDto) -> Result<CompareDto, CmdError> {
    validate(faces, MAX_COMPARE)?;
    let details: Vec<FaceDetailDto> = faces.iter().map(|f| get_face(state, f, opts.lang)).collect::<Result<_, _>>()?;
    let base = &details[0];
    let out = details
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let axes = d
                .axes
                .iter()
                .map(|a| {
                    let b = base.axes.iter().find(|x| x.key == a.key).and_then(|x| x.raw_value);
                    let (delta, delta_pct) = if i == 0 { (None, None) } else { delta(b, a.raw_value, a.unit.as_deref()) };
                    CompareAxisDto {
                        key: a.key.clone(),
                        unit: a.unit.clone(),
                        raw_value: a.raw_value,
                        quality: a.quality.clone(),
                        estimated: a.quality == "estimated",
                        percentile: a.percentile,
                        delta,
                        delta_pct,
                    }
                })
                .collect();
            CompareFaceDto {
                face: faces[i].clone(),
                row: d.row.clone(),
                is_baseline: i == 0,
                classification: d.classification_info.clone(),
                axes,
                vietnamese: vietnamese_of(d, hhea_ascender(state, &faces[i])),
                license: LicenseNoteDto {
                    kind: d.extras.license_kind.clone(),
                    description: d.metadata.license.clone(),
                    url: d.metadata.license_url.clone(),
                    embedding: d.extras.embedding.clone(),
                },
                extras: d.extras.clone(),
                family: family_structure(state, d),
            }
        })
        .collect();
    Ok(CompareDto { baseline: faces[0].clone(), faces: out })
}

/// Thử mọi cách gán (heading, body) giữa các face đã chọn; xếp hạng lexicographic như Pairing; trả tối đa 3 Combo.
pub fn pair_these(state: &AppState, faces: &[FaceKeyDto], opts: &PairTheseOptsDto) -> Result<Vec<PairComboDto>, CmdError> {
    validate(faces, MAX_PAIR)?;
    let b = bundle(state)?;
    let keys = faces.iter().map(|f| ix_of(&b, f)).collect::<Result<Vec<_>, _>>()?;
    let rows = rows_of(state, &b, &keys)?;
    let lang = lang_of(opts.lang);
    let max = (opts.max_combos as usize).clamp(1, 3);
    let mut combos = Vec::new();
    for (i, kh) in keys.iter().enumerate() {
        for (j, kb) in keys.iter().enumerate() {
            if i == j {
                continue;
            }
            let x = b.index.assess_pair(kh, Role::Heading, kb, Role::Body, opts.project_vi)?;
            let (Some(ph), Some(pb), Some(rh), Some(rb)) = (b.index.profile(kh), b.index.profile(kb), rows.get(kh), rows.get(kb)) else {
                return Err(CmdError::not_found("face"));
            };
            let key = rank_key(&x, &rb.family, None);
            let mut d = candidate_dto(&x, ph, pb, rb.clone(), lang);
            d.summary = summarize_face(pb, b.index.reference(), lang);
            combos.push((key, rh.clone(), rb.clone(), d));
        }
    }
    combos.sort_by(|a, c| a.0.cmp(&c.0).then_with(|| (a.1.face_id, a.2.face_id).cmp(&(c.1.face_id, c.2.face_id))));
    Ok(combos
        .into_iter()
        .take(max)
        .enumerate()
        .map(|(n, (_, heading, body, assessment))| PairComboDto { heading, body, assessment, rank: n as u32 + 1 })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_pct_skips_angles_and_near_zero_bases() {
        // bình thường
        let (d, p) = delta(Some(2.0), Some(3.0), Some("ratio"));
        assert_eq!((d, p), (Some(1.0), Some(50.0)));
        // mốc âm: % theo |mốc|
        assert_eq!(delta(Some(-2.0), Some(-1.0), Some("ratio")), (Some(1.0), Some(50.0)));
        // trục góc: chỉ Δ tuyệt đối
        assert_eq!(delta(Some(5.0), Some(10.0), Some("deg")), (Some(5.0), None));
        // mốc ≈ 0 (kể cả rất nhỏ): chỉ Δ tuyệt đối, không nổ ra hàng triệu %
        assert_eq!(delta(Some(0.0), Some(1.0), Some("ratio")), (Some(1.0), None));
        let (d, p) = delta(Some(1e-6), Some(0.5), Some("ratio"));
        assert!(d.is_some() && p.is_none());
        // đúng ngưỡng thì có %
        assert!(delta(Some(1e-3), Some(2e-3), None).1.is_some());
        // thiếu số
        assert_eq!(delta(None, Some(1.0), None), (None, None));
        assert_eq!(delta(Some(1.0), Some(f64::NAN), None), (None, None));
    }
}
