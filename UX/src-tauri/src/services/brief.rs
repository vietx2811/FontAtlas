//! Brief starter: chip (dùng cho, giọng, ràng buộc) → bộ lọc của từ điển mô tả + trường lọc mới của Library
//! → 3 font neo và 3 Combo khởi đầu. Không LLM, không ghi gì; mọi ánh xạ là bảng cố định bên dưới.
use fontatlas_describe::Strength;

use crate::dto::analysis::{LangDto, PairModeDto, PairOptsDto, PairRoleDto, PairComboDto};
use crate::dto::cases::*;
use crate::dto::langs::{LangFilterDto, LangLevelDto};
use crate::dto::library::{FaceQueryDto, FaceRowDto, SortKeyDto, ViFilterDto, VarAxisMatchDto};
use crate::error::CmdError;
use crate::state::AppState;

pub const MAX_ANCHORS: usize = 3;
/// |value| nhỏ hơn mức này coi như thanh trượt chưa được kéo.
pub const TONE_DEAD_ZONE: f32 = 0.25;
/// |value| từ mức này trở lên → `Required`, còn lại `Preferred`.
pub const TONE_REQUIRED_AT: f32 = 0.6;
pub const DISPLAY_WEIGHT: [u16; 2] = [600, 900];
pub const BODY_WEIGHT: [u16; 2] = [400, 500];

/// Term của từ điển cho một đầu mút thanh trượt (`None` = chưa kéo).
pub fn tone_term(t: &ToneDto) -> Option<(&'static str, Strength)> {
    if t.value.abs() < TONE_DEAD_ZONE || !t.value.is_finite() {
        return None;
    }
    let neg = t.value < 0.0;
    let id = match (t.pair, neg) {
        (TonePairDto::ClassicModern, true) => "classic",
        (TonePairDto::ClassicModern, false) => "modern",
        (TonePairDto::WarmCool, true) => "warm",
        (TonePairDto::WarmCool, false) => "cool",
        (TonePairDto::QuietBold, true) => "delicate",
        (TonePairDto::QuietBold, false) => "strong",
    };
    let st = if t.value.abs() >= TONE_REQUIRED_AT { Strength::Required } else { Strength::Preferred };
    Some((id, st))
}

/// Term của từ điển cho mục đích dùng. `branding` không có term (chỉ ảnh hưởng class); `packaging` có mục
/// trong từ điển nhưng được đánh dấu không ánh xạ được — trả đúng lý do của từ điển, không đoán.
pub fn use_terms(u: BriefUseDto) -> &'static [&'static str] {
    match u {
        BriefUseDto::Editorial => &["editorial"],
        BriefUseDto::UiWeb => &["readable_small", "clean"],
        BriefUseDto::Poster => &["display", "heavy"],
        BriefUseDto::Branding => &[],
        BriefUseDto::Packaging => &["packaging"],
    }
}

fn note(code: &str, detail: Option<String>) -> BriefNoteDto {
    BriefNoteDto { code: code.into(), detail }
}

fn base_query(brief: &BriefDto, d: &crate::dto::analysis::DescribeDto, notes: &mut Vec<BriefNoteDto>, state: &AppState) -> Result<FaceQueryDto, CmdError> {
    let mut q = FaceQueryDto { sort: SortKeyDto::Match, limit: 60, ..Default::default() };
    q.axis_filters = d.filters.clone();
    q.hard = d.hard.clone();
    if brief.needs.vi {
        if !q.hard.iter().any(|h| h == "vi_supported") {
            q.hard.push("vi_supported".into());
        }
        q.vi = Some(ViFilterDto::Supported);
        q.langs = vec![LangFilterDto { code: "vie".into(), level: LangLevelDto::Near }];
        if !crate::services::langs::coverage_complete(state) {
            notes.push(note("langs_incomplete", None));
        }
    }
    if brief.needs.open_license {
        q.license = vec!["ofl".into(), "apache".into()];
    }
    if brief.needs.variable {
        q.variable = Some(true);
    }
    if brief.needs.italic {
        q.italic = Some(true);
    }
    if brief.needs.installed_only {
        let active: Vec<_> = crate::services::activation::state_list(state)?
            .into_iter()
            .filter(|s| s.state == crate::dto::activation::DesiredStateDto::Active)
            .flat_map(|s| s.faces.into_iter().map(move |i| crate::dto::library::FaceKeyDto { hash_hex: s.hash_hex.clone(), face_index: i }))
            .take(crate::services::library::MAX_FACE_KEYS)
            .collect();
        if active.is_empty() {
            notes.push(note("installed_none_active", None));
        } else {
            q.face_keys = Some(active);
        }
    }
    Ok(q)
}

fn with_weight(mut q: FaceQueryDto, w: [u16; 2]) -> FaceQueryDto {
    q.weight_class = Some(w);
    q.var_axis_match = VarAxisMatchDto { wght: true, wdth: true };
    q
}

/// Tối đa `MAX_ANCHORS` font khác family, giữ thứ tự xếp hạng.
fn distinct_families(rows: Vec<FaceRowDto>) -> Vec<FaceRowDto> {
    let mut seen = std::collections::HashSet::new();
    rows.into_iter().filter(|r| seen.insert(r.family.to_lowercase())).take(MAX_ANCHORS).collect()
}

pub fn brief_starter(state: &AppState, brief: &BriefDto, case_id: Option<i64>) -> Result<BriefStarterDto, CmdError> {
    let mut notes = Vec::new();
    // 1. chip → term
    let mut ids: Vec<(String, Option<Strength>)> = Vec::new();
    for t in &brief.tones {
        if let Some((id, st)) = tone_term(t) {
            ids.push((id.to_string(), Some(st)));
        }
    }
    for u in &brief.uses {
        ids.extend(use_terms(*u).iter().map(|id| ((*id).to_string(), None)));
    }
    if brief.uses.contains(&BriefUseDto::Branding) && brief.uses.len() == 1 {
        notes.push(note("branding_class_only", None));
    }
    if let Some(text) = brief.text.as_deref().filter(|t| !t.trim().is_empty()) {
        for m in state.dictionary.parse(text).matched {
            if !ids.iter().any(|(i, _)| *i == m.id) {
                ids.push((m.id, None));
            }
        }
    }
    let id_refs: Vec<(&str, Option<Strength>)> = ids.iter().map(|(i, s)| (i.as_str(), *s)).collect();
    let parsed = state.dictionary.parse_ids(&id_refs);
    if parsed.matched.is_empty() {
        notes.push(note("no_terms", None));
    }
    let describe = crate::services::describe::from_parsed(parsed);

    // 2. term + ràng buộc → điều kiện Library
    let query = base_query(brief, &describe, &mut notes, state)?;
    let display_query = with_weight(query.clone(), DISPLAY_WEIGHT);
    let body_query = with_weight(query.clone(), BODY_WEIGHT);

    // 3. neo (vai display): nới trọng lượng nếu thiếu
    let mut anchors = Vec::new();
    match crate::services::library::query_faces(state, &display_query) {
        Ok(page) => {
            anchors = distinct_families(page.items);
            if anchors.len() < MAX_ANCHORS
                && let Ok(wide) = crate::services::library::query_faces(state, &query)
            {
                let mut all = anchors.clone();
                all.extend(wide.items);
                let relaxed = distinct_families(all);
                if relaxed.len() > anchors.len() {
                    notes.push(note("weight_relaxed", Some(format!("{}-{}", DISPLAY_WEIGHT[0], DISPLAY_WEIGHT[1]))));
                    anchors = relaxed;
                }
            }
        }
        Err(e) => notes.push(note("index_unavailable", Some(e.message.clone()))),
    }
    if anchors.len() < MAX_ANCHORS {
        notes.push(note("few_anchors", Some(anchors.len().to_string())));
    }

    // 4. Combo: neo + font body hạng 1 (khác family, không lặp body)
    let mode = case_id
        .and_then(|id| crate::services::cases::get_case(state, id).ok())
        .and_then(|c| c.settings.mode)
        .map(|m| match m {
            PairingLevelDto::Safe => PairModeDto::Safe,
            PairingLevelDto::Controlled => PairModeDto::Controlled,
            PairingLevelDto::Experimental => PairModeDto::Experimental,
        })
        .unwrap_or(PairModeDto::Safe);
    let mut combos: Vec<PairComboDto> = Vec::new();
    let mut used_body: Vec<String> = Vec::new();
    for a in &anchors {
        let face = crate::dto::library::FaceKeyDto { hash_hex: a.hash_hex.clone(), face_index: a.face_index };
        let opts = PairOptsDto { role: PairRoleDto::Body, mode: Some(mode), project_vi: brief.needs.vi, k: 20, collection_id: None, lang: LangDto::Vi };
        let Ok(cands) = crate::services::analysis::pair_candidates(state, &face, &opts) else { continue };
        let pick = cands
            .iter()
            .find(|c| c.row.family != a.family && !used_body.contains(&c.row.family))
            .or_else(|| cands.iter().find(|c| !used_body.contains(&c.row.family)));
        match pick {
            Some(c) => {
                used_body.push(c.row.family.clone());
                combos.push(PairComboDto { heading: a.clone(), body: c.row.clone(), assessment: c.clone(), rank: combos.len() as u32 + 1 });
            }
            None => notes.push(note("no_body_pair", Some(a.family.clone()))),
        }
    }
    Ok(BriefStarterDto { describe, query, display_query, body_query, anchors, combos, notes, ai: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(pair: TonePairDto, value: f32) -> ToneDto {
        ToneDto { pair, value }
    }

    #[test]
    fn tone_mapping_table_dead_zone_and_strength() {
        use TonePairDto::*;
        let cases = [
            (ClassicModern, -1.0, Some(("classic", Strength::Required))),
            (ClassicModern, 0.3, Some(("modern", Strength::Preferred))),
            (WarmCool, -0.6, Some(("warm", Strength::Required))),
            (WarmCool, 0.59, Some(("cool", Strength::Preferred))),
            (QuietBold, -0.25, Some(("delicate", Strength::Preferred))),
            (QuietBold, 0.9, Some(("strong", Strength::Required))),
            (QuietBold, 0.24, None),
            (WarmCool, -0.1, None),
            (WarmCool, f32::NAN, None),
        ];
        for (pair, v, want) in cases {
            assert_eq!(tone_term(&tone(pair, v)), want, "{pair:?} {v}");
        }
    }

    #[test]
    fn every_mapped_term_exists_in_the_bundled_dictionary() {
        let d = fontatlas_describe::Dictionary::bundled();
        for id in ["classic", "modern", "warm", "cool", "delicate", "strong"] {
            assert!(d.has_id(id), "{id}");
        }
        for u in [BriefUseDto::Editorial, BriefUseDto::Branding, BriefUseDto::Poster, BriefUseDto::UiWeb, BriefUseDto::Packaging] {
            for id in use_terms(u) {
                assert!(d.has_id(id), "{u:?} → {id}");
            }
        }
    }
}
