//! Gợi ý đối tác / nguồn cột 2 / combo (G4–G6). Mô hình ở `fontatlas_core::pairing`; ở đây chỉ quét, lọc, xếp hạng tất định.
//! Xếp hạng lexicographic (không điểm số): cổng Pairs → mức phong cách tốt nhất → mức hợp vai → số phong cách khớp → tên.
use std::collections::{HashMap, HashSet};

use fontatlas_core::pairing::{self, Fit, PairCtx, PairStyle, Reason, Role, Traits};
use fontatlas_index::{FaceKey as IxKey, FaceProfile, Gate, Role as IxRole};

use crate::dto::library::FaceKeyDto;
use crate::dto::suggest::*;
use crate::error::CmdError;
use crate::index_bundle::IndexBundle;
use crate::services::analysis::{bundle, dto_key, ix_of, rows_of};
use crate::services::library::{EffMap, effective_classes, matching_keys};
use crate::services::suggest::{fit_dto, reason_dto, resolve_role, traits_of, with_tnum_note};
use crate::state::AppState;

fn ix_role(r: Role) -> IxRole {
    match r {
        Role::Body | Role::Caption | Role::Label | Role::Numerals => IxRole::Body,
        Role::Heading | Role::Display | Role::Quote => IxRole::Heading,
    }
}

fn parse_styles(ids: &[String]) -> Result<Vec<PairStyle>, CmdError> {
    if ids.is_empty() {
        return Ok(PairStyle::ALL.to_vec());
    }
    ids.iter().map(|s| PairStyle::from_id(s).ok_or_else(|| CmdError::invalid_arg(format!("phong cách lạ: {s}")))).collect()
}

fn gate_rank(g: Gate) -> u8 {
    match g {
        Gate::Safe => 0,
        Gate::Controlled => 1,
        Gate::Experimental => 2,
        Gate::Blocked => 3,
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

struct Cand<'a> {
    p: &'a FaceProfile,
    gate: Gate,
    role_fit: Fit,
    role_reasons: Vec<Reason>,
    style_fit: Fit,
    styles: Vec<PairStyle>,
    style_reasons: Vec<Reason>,
    warnings: Vec<Reason>,
}

struct Scan<'a> {
    b: &'a IndexBundle,
    eff: &'a EffMap,
}

impl Scan<'_> {
    fn ctx(&self, a: &FaceProfile, c: &FaceProfile) -> PairCtx {
        PairCtx {
            same_family_root: pairing::family_root(&a.family) == pairing::family_root(&c.family),
            same_foundry: matches!((&a.manufacturer, &c.manufacturer), (Some(x), Some(y)) if x == y)
                || matches!((&a.designer, &c.designer), (Some(x), Some(y)) if x == y),
            embed_cos: match (&a.embedding, &c.embedding) {
                (Some(x), Some(y)) => fontatlas_index::embed_cos(Some(x), Some(y)),
                _ => None,
            },
        }
    }

    /// Mọi đối tác đạt (hợp vai ≥ ok, ≥ 1 phong cách ≥ ok, không bị chặn), đã xếp hạng.
    #[allow(clippy::too_many_arguments)]
    fn partners(
        &self,
        anchor: &FaceProfile,
        anchor_role: Role,
        target: Role,
        styles: &[PairStyle],
        project_vi: bool,
        allowed: Option<&HashSet<IxKey>>,
        exclude: &HashSet<IxKey>,
    ) -> Vec<Cand<'_>> {
        let at = traits_of(self.b, anchor, self.eff);
        let anchor_latin = !self.b.meta.get(&anchor.key).is_some_and(|m| m.non_latin_primary);
        let mut out = Vec::new();
        for c in self.b.index.profiles() {
            if c.key == anchor.key || exclude.contains(&c.key) {
                continue;
            }
            let Some(m) = self.b.meta.get(&c.key) else { continue };
            if m.missing || (anchor_latin && m.non_latin_primary) || (project_vi && c.vi_missing > 0) {
                continue;
            }
            if allowed.is_some_and(|s| !s.contains(&c.key)) {
                continue;
            }
            let ct: Traits = traits_of(self.b, c, self.eff);
            let rf = pairing::role_fit(&ct, target);
            if !matches!(rf.fit, Fit::Good | Fit::Ok) {
                continue;
            }
            let ctx = self.ctx(anchor, c);
            let mut hit: Vec<PairStyle> = Vec::new();
            let (mut best, mut best_reasons, mut warns) = (Fit::Unknown, Vec::new(), Vec::new());
            for s in styles {
                let x = pairing::pair_style(&at, &ct, &ctx, *s);
                if matches!(x.fit, Fit::Good | Fit::Ok) {
                    hit.push(*s);
                    if x.fit > best {
                        best = x.fit;
                        best_reasons = x.reasons;
                        warns = x.warnings;
                    }
                }
            }
            if hit.is_empty() {
                continue;
            }
            let Ok(a) = self.b.index.assess_pair(&anchor.key, ix_role(anchor_role), &c.key, ix_role(target), project_vi) else { continue };
            if a.gate == Gate::Blocked {
                continue;
            }
            out.push(Cand { p: c, gate: a.gate, role_fit: rf.fit, role_reasons: rf.reasons, style_fit: best, styles: hit, style_reasons: best_reasons, warnings: warns });
        }
        out.sort_by(|x, y| {
            gate_rank(x.gate)
                .cmp(&gate_rank(y.gate))
                .then(y.style_fit.cmp(&x.style_fit))
                .then(y.role_fit.cmp(&x.role_fit))
                .then(y.styles.len().cmp(&x.styles.len()))
                .then_with(|| x.p.family.cmp(&y.p.family))
                .then_with(|| x.p.style.cmp(&y.p.style))
                .then_with(|| x.p.key.cmp(&y.p.key))
        });
        out
    }
}

fn need_role(r: &RoleRefDto) -> Result<Role, CmdError> {
    resolve_role(r).ok_or_else(|| CmdError::invalid_arg(format!("vai không nhận ra: {}", r.name)))
}

pub fn suggest_partners(state: &AppState, anchor: &FaceKeyDto, req: &SuggestPartnersReqDto) -> Result<PartnerPageDto, CmdError> {
    let (ar, tr) = (resolve_role(&req.anchor_role), resolve_role(&req.target_role));
    let styles = parse_styles(&req.styles)?;
    let b = bundle(state)?;
    let key = ix_of(&b, anchor)?;
    let (Some(ar), Some(tr)) = (ar, tr) else {
        return Ok(PartnerPageDto { items: vec![], total: 0 });
    };
    let eff = effective_classes(state)?;
    let allowed = req.filter.as_ref().map(|q| matching_keys(state, q)).transpose()?;
    let ap = b.index.profile(&key).ok_or_else(|| CmdError::not_found("face"))?;
    let scan = Scan { b: &b, eff: &eff };
    let all = scan.partners(ap, ar, tr, &styles, req.project_vi, allowed.as_ref(), &HashSet::new());
    let total = all.len() as u64;
    let page: Vec<&Cand> = all.iter().skip(req.offset as usize).take(req.limit.clamp(1, 100) as usize).collect();
    let keys: Vec<IxKey> = page.iter().map(|c| c.p.key.clone()).collect();
    let rows = rows_of(state, &b, &keys)?;
    let items = page
        .into_iter()
        .filter_map(|c| {
            let mut reasons: Vec<WhyDto> = c.style_reasons.iter().map(reason_dto).collect();
            reasons.extend(c.role_reasons.iter().map(reason_dto));
            reasons.truncate(4);
            Some(PartnerDto {
                face: dto_key(&c.p.key),
                row: rows.get(&c.p.key)?.clone(),
                gate: gate_str(c.gate).into(),
                styles: c.styles.iter().map(|s| s.id().to_string()).collect(),
                role_fit: fit_dto(c.role_fit),
                style_fit: fit_dto(c.style_fit),
                reasons: with_tnum_note(tr, reasons),
                warnings: c.warnings.iter().map(reason_dto).collect(),
            })
        })
        .collect();
    Ok(PartnerPageDto { items, total })
}

// ---- cột 2 ----

pub fn desk_suggest(state: &AppState, req: &DeskSuggestReqDto) -> Result<DeskPageDto, CmdError> {
    use crate::dto::analysis::SimilarOptsDto;
    let limit = req.limit.clamp(1, 100) as usize;
    let (items, total) = match req.source.as_str() {
        "fits_role" => {
            let o = RoleFitListOptsDto { project_vi: req.project_vi, limit: 100, offset: 0, filter: req.filter.clone() };
            let page = crate::services::suggest::role_fit_faces(state, &req.target_role, &o)?;
            let anchor = req.anchor.as_ref().map(|a| (a.hash_hex.clone(), a.face_index));
            let all: Vec<DeskItemDto> = page
                .items
                .into_iter()
                .filter(|h| anchor.as_ref().is_none_or(|a| *a != (h.face.hash_hex.clone(), h.face.face_index)))
                .map(|h| DeskItemDto { face: h.face, row: h.row, fit: Some(h.fit), reasons: h.reasons, pinned: false })
                .collect();
            let t = page.total;
            (all.into_iter().skip(req.offset as usize).take(limit).collect::<Vec<_>>(), t)
        }
        "similar" => {
            let a = req.anchor.as_ref().ok_or_else(|| CmdError::invalid_arg("nguồn similar cần anchor"))?;
            let o = SimilarOptsDto { k: 100, vi_only: req.project_vi, ..Default::default() };
            let allowed = req.filter.as_ref().map(|q| matching_keys(state, q)).transpose()?;
            let hits = crate::services::analysis::similar(state, a, &o)?;
            let all: Vec<_> = hits
                .into_iter()
                .filter(|h| allowed.as_ref().is_none_or(|s| s.contains(&IxKey { hash: h.face.hash_hex.clone(), index: h.face.face_index })))
                .collect();
            let t = all.len() as u64;
            let items = all
                .into_iter()
                .skip(req.offset as usize)
                .take(limit)
                .map(|h| {
                    let mut why: Vec<WhyDto> = h.same.iter().take(3).map(|ax| WhyDto { code: format!("similar_{ax}"), params: vec![] }).collect();
                    if matches!(h.source, crate::dto::analysis::SimilarSourceDto::Image | crate::dto::analysis::SimilarSourceDto::AxesAndImage) && why.len() < 3 {
                        why.push(WhyDto { code: "image_close".into(), params: vec![] });
                    }
                    DeskItemDto { face: h.face, row: h.row, fit: None, reasons: why, pinned: false }
                })
                .collect();
            (items, t)
        }
        "pinned" => {
            let id = req.case_id.ok_or_else(|| CmdError::invalid_arg("nguồn pinned cần caseId"))?;
            let case = crate::services::cases::get_case(state, id)?;
            let b = bundle(state)?;
            let eff = effective_classes(state)?;
            let role = resolve_role(&req.target_role);
            let allowed = req.filter.as_ref().map(|q| matching_keys(state, q)).transpose()?;
            let mut v = Vec::new();
            for f in &case.faces {
                let Some(row) = f.row.clone() else { continue };
                if f.missing {
                    continue;
                }
                let k = IxKey { hash: f.key.hash_hex.clone(), index: f.key.face_index };
                if allowed.as_ref().is_some_and(|s| !s.contains(&k)) {
                    continue;
                }
                let (fit, reasons) = match (role, b.index.profile(&k)) {
                    (Some(r), Some(p)) => {
                        let x = pairing::role_fit(&traits_of(&b, p, &eff), r);
                        (Some(fit_dto(x.fit)), with_tnum_note(r, x.reasons.iter().map(reason_dto).collect()))
                    }
                    _ => (None, vec![]),
                };
                v.push(DeskItemDto { face: f.key.clone(), row, fit, reasons, pinned: true });
            }
            let t = v.len() as u64;
            (v.into_iter().skip(req.offset as usize).take(limit).collect(), t)
        }
        other => return Err(CmdError::invalid_arg(format!("nguồn lạ: {other}"))),
    };
    Ok(DeskPageDto { source: req.source.clone(), items, total })
}

// ---- combo ----

fn mix(seed: u64, k: &IxKey) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ seed;
    for b in k.hash.bytes().chain(k.index.to_le_bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^ (h >> 31)
}

const MAX_ANCHORS: usize = 12;

pub fn suggest_combos(state: &AppState, req: &SuggestCombosReqDto) -> Result<Vec<ComboSuggestionDto>, CmdError> {
    if req.roles.is_empty() || req.roles.len() > 6 {
        return Err(CmdError::invalid_arg("cần 1–6 vai"));
    }
    let styles = parse_styles(&req.styles)?;
    let k_out = req.k.clamp(3, 6) as usize;
    let roles: Vec<Role> = req.roles.iter().map(need_role).collect::<Result<_, _>>()?;
    let b = bundle(state)?;
    let eff = effective_classes(state)?;
    let scan = Scan { b: &b, eff: &eff };
    let name_of = |r: &RoleRefDto| r.name.trim().to_lowercase();
    let mut locked: HashMap<usize, IxKey> = HashMap::new();
    for l in &req.locked {
        let i = req.roles.iter().position(|r| name_of(r) == name_of(&l.role)).ok_or_else(|| CmdError::invalid_arg(format!("khoá vai không có trong roles: {}", l.role.name)))?;
        locked.insert(i, ix_of(&b, &l.face)?);
    }
    let allowed: Option<HashSet<IxKey>> = match (req.scope.as_str(), req.case_id) {
        ("case", Some(id)) => Some(
            crate::services::cases::get_case(state, id)?
                .faces
                .iter()
                .map(|f| IxKey { hash: f.key.hash_hex.clone(), index: f.key.face_index })
                .collect(),
        ),
        ("case", None) => return Err(CmdError::invalid_arg("scope=case cần caseId")),
        ("library", _) | ("", _) => None,
        (o, _) => return Err(CmdError::invalid_arg(format!("scope lạ: {o}"))),
    };

    // Neo = vai đầu: tối đa 1 face mỗi family, good trước ok, trong cùng mức xáo theo seed (tất định).
    let anchors: Vec<&FaceProfile> = if let Some(k) = locked.get(&0) {
        vec![b.index.profile(k).ok_or_else(|| CmdError::not_found("face"))?]
    } else {
        let mut pool: Vec<(Fit, u64, &FaceProfile)> = Vec::new();
        for p in b.index.profiles() {
            let Some(m) = b.meta.get(&p.key) else { continue };
            if m.missing || m.non_latin_primary || (req.project_vi && p.vi_missing > 0) || allowed.as_ref().is_some_and(|s| !s.contains(&p.key)) {
                continue;
            }
            let f = pairing::role_fit(&traits_of(&b, p, &eff), roles[0]).fit;
            if matches!(f, Fit::Good | Fit::Ok) {
                pool.push((f, mix(req.seed, &p.key), p));
            }
        }
        pool.sort_by(|a, c| c.0.cmp(&a.0).then(a.1.cmp(&c.1)).then_with(|| a.2.key.cmp(&c.2.key)));
        let mut seen = HashSet::new();
        pool.into_iter().filter(|(_, _, p)| seen.insert(p.family.clone())).map(|x| x.2).take(MAX_ANCHORS).collect()
    };

    let mut out: Vec<(Fit, Fit, u64, ComboSuggestionDto)> = Vec::new();
    for a in anchors {
        let mut used: HashSet<IxKey> = HashSet::from([a.key.clone()]);
        let mut picks: Vec<(usize, &FaceProfile, Fit, bool)> = vec![(0, a, pairing::role_fit(&traits_of(&b, a, &eff), roles[0]).fit, locked.contains_key(&0))];
        let (mut style, mut reasons, mut warnings, mut style_fit) = (None, Vec::<WhyDto>::new(), Vec::<WhyDto>::new(), Fit::Good);
        let mut ok = true;
        for (i, role) in roles.iter().enumerate().skip(1) {
            if let Some(k) = locked.get(&i) {
                let p = b.index.profile(k).ok_or_else(|| CmdError::not_found("face"))?;
                used.insert(p.key.clone());
                picks.push((i, p, pairing::role_fit(&traits_of(&b, p, &eff), *role).fit, true));
                continue;
            }
            // Đối tác của neo cho vai này; không có thì bỏ cả combo.
            let c = scan.partners(a, roles[0], *role, &styles, req.project_vi, allowed.as_ref(), &used).into_iter().next();
            let Some(c) = c else {
                ok = false;
                break;
            };
            if i == 1 {
                style = c.styles.first().map(|s| s.id().to_string());
                reasons = c.style_reasons.iter().map(reason_dto).collect();
            }
            style_fit = style_fit.min(c.style_fit);
            warnings.extend(c.warnings.iter().map(reason_dto));
            if c.gate != Gate::Safe {
                warnings.push(WhyDto { code: format!("gate_{}", gate_str(c.gate)), params: vec![] });
            }
            used.insert(c.p.key.clone());
            picks.push((i, c.p, c.role_fit, false));
        }
        if !ok {
            continue;
        }
        let min_fit = picks.iter().map(|p| p.2).min().unwrap_or(Fit::Unknown);
        let keys: Vec<IxKey> = picks.iter().map(|p| p.1.key.clone()).collect();
        let rows = rows_of(state, &b, &keys)?;
        let slots: Vec<ComboSlotSuggestionDto> = picks
            .iter()
            .filter_map(|(i, p, f, lk)| {
                Some(ComboSlotSuggestionDto { role: req.roles[*i].clone(), face: dto_key(&p.key), row: rows.get(&p.key)?.clone(), fit: fit_dto(*f), locked: *lk })
            })
            .collect();
        if slots.len() != picks.len() {
            continue;
        }
        warnings.truncate(4);
        let vi_missing: u32 = picks.iter().map(|p| p.1.vi_missing as u32).sum();
        out.push((min_fit, style_fit, mix(req.seed, &a.key), ComboSuggestionDto { slots, style, reasons, warnings, vi_missing, seed: req.seed }));
    }
    out.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.cmp(&x.1)).then(x.2.cmp(&y.2)));
    Ok(out.into_iter().take(k_out).map(|x| x.3).collect())
}
