//! Giải quyết trùng lặp: xem trước kế hoạch → áp dụng (ẩn / Thùng rác) → hoàn tác. Quy tắc chọn bản giữ ở `dup_pick`;
//! sổ quyết định và chuyển liên kết ở `fontatlas_store::dup_resolve`. KHÔNG xoá vĩnh viễn, KHÔNG đụng nguồn hệ thống.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fontatlas_activate::{ConflictPolicy, DeactivatePolicy, Scope};
use fontatlas_store::{ActivationScope, DesiredState, FaceKey, MemberInfo, NewResolution, ResolutionRow, Source};

use crate::dto::analysis::DuplicateKindDto;
use crate::dto::dup_resolve::*;
use crate::dto::library::{FaceKeyDto, FaceQueryDto, FaceRowDto};
use crate::error::CmdError;
use crate::services::dup_pick::{Cand, Decision, Reason, pick_keeper};
use crate::services::duplicates::{self, Member, RawGroup};
use crate::state::AppState;

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn reason_dto(r: &Reason) -> ReasonDto {
    ReasonDto { code: r.code.to_string(), args: r.args.clone() }
}

fn kind_str(k: DuplicateKindDto) -> &'static str {
    match k {
        DuplicateKindDto::SameFile => "same_file",
        DuplicateKindDto::Versions => "versions",
        DuplicateKindDto::Formats => "formats",
    }
}

fn kind_of(s: &str) -> DuplicateKindDto {
    match s {
        "same_file" => DuplicateKindDto::SameFile,
        "versions" => DuplicateKindDto::Versions,
        _ => DuplicateKindDto::Formats,
    }
}

fn key_dto(k: &FaceKey) -> FaceKeyDto {
    FaceKeyDto { hash_hex: k.hash_hex(), face_index: k.face_index }
}

/// Thuần: tệp có được chuyển vào Thùng rác không. Chỉ tệp trong thư mục do người dùng thêm (nguồn `directory`),
/// không bao giờ /System hay /Library; ~/Library/Fonts cần người dùng chọn rõ từng tệp.
pub fn trash_eligibility(path: &str, source_kind: &str, num_faces: u32, exists: bool, home: Option<&Path>) -> TrashEligibilityDto {
    let blocked = |b: &str| TrashEligibilityDto { allowed: false, needs_explicit: false, blocked: Some(b.into()) };
    if source_kind != "directory" {
        return blocked("managed_source");
    }
    if path.starts_with("/System/") || path.starts_with("/Library/") {
        return blocked("system_path");
    }
    if num_faces > 1 {
        return blocked("collection_file");
    }
    if !exists {
        return blocked("missing_file");
    }
    let user_fonts = home.map(|h| h.join("Library/Fonts"));
    let needs_explicit = user_fonts.is_some_and(|f| Path::new(path).starts_with(f));
    TrashEligibilityDto { allowed: true, needs_explicit, blocked: None }
}

struct Ctx {
    rows: HashMap<i64, FaceRowDto>,
    sources: Vec<Source>,
    home: Option<PathBuf>,
}

fn ctx_for(state: &AppState, groups: &[RawGroup]) -> Result<Ctx, CmdError> {
    let ids: Vec<i64> = groups.iter().flat_map(|g| g.members.iter().map(|m| m.face_id)).collect::<std::collections::HashSet<_>>().into_iter().collect();
    let rows = duplicates::rows_by_id(state, &ids)?;
    let sources = state.with_store(|s| Ok(s.sources()?))?;
    Ok(Ctx { rows, sources, home: std::env::var_os("HOME").map(PathBuf::from) })
}

struct Built {
    member: Member,
    cand: Cand,
    info: MemberInfo,
    source: Option<Source>,
    row: FaceRowDto,
    active: bool,
}

fn build(state: &AppState, ctx: &Ctx, g: &RawGroup) -> Result<Option<Vec<Built>>, CmdError> {
    let mut out = Vec::new();
    for m in &g.members {
        let (Some(path), Some(row)) = (m.path.clone(), ctx.rows.get(&m.face_id)) else { return Ok(None) };
        let Some(info) = state.with_store(|s| Ok(s.member_info(&m.key, &path)?))? else { return Ok(None) };
        let rank = ctx.sources.iter().position(|s| s.id == info.source_id).unwrap_or(usize::MAX);
        let source = ctx.sources.iter().find(|s| s.id == info.source_id).cloned();
        let active = state.with_store(|s| Ok(s.activation_row(&m.key.hash)?))?.is_some_and(|a| a.state == DesiredState::Active);
        let cand = Cand {
            path,
            format: m.format.clone(),
            version: m.version.clone(),
            revision: m.revision,
            modified: m.modified,
            is_newest: m.is_newest,
            num_glyphs: info.num_glyphs,
            vi_missing: row.vi_missing,
            is_variable: row.is_variable,
            file_size: row.file_size,
            source_rank: rank,
            license: row.license_class.clone(),
            num_faces: info.num_faces,
        };
        out.push(Built { member: m.clone(), cand, info, source, row: row.clone(), active });
    }
    Ok(Some(out))
}

fn member_dto(b: &Built, ctx: &Ctx) -> PlanMemberDto {
    let kind = b.source.as_ref().map(|s| s.kind.as_str()).unwrap_or("directory");
    let exists = Path::new(&b.cand.path).is_file();
    PlanMemberDto {
        face: key_dto(&b.member.key),
        family: b.row.family.clone(),
        style: b.row.subfamily.clone(),
        path: b.cand.path.clone(),
        format: b.cand.format.clone(),
        version: b.cand.version.clone(),
        revision: b.cand.revision,
        modified: b.cand.modified,
        num_glyphs: b.cand.num_glyphs,
        file_size: b.cand.file_size,
        vi_missing: b.cand.vi_missing,
        is_variable: b.cand.is_variable,
        source_id: b.info.source_id,
        source_name: b.source.as_ref().map(|s| s.name.clone()).unwrap_or_default(),
        source_kind: kind.into(),
        active: b.active,
        trash: trash_eligibility(&b.cand.path, kind, b.cand.num_faces, exists, ctx.home.as_deref()),
    }
}

fn plan_item(g: &RawGroup, built: &[Built], d: &Decision, ctx: &Ctx) -> PlanItemDto {
    PlanItemDto {
        kind: g.kind,
        key: g.key.clone(),
        keeper: member_dto(&built[d.keeper], ctx),
        losers: d.losers.iter().map(|&i| member_dto(&built[i], ctx)).collect(),
        reason: reason_dto(&d.reason),
        auto_safe: d.auto_safe,
        review: d.review.as_ref().map(reason_dto),
        warnings: d.warnings.iter().map(reason_dto).collect(),
    }
}

/// Kế hoạch giữ / ẩn theo quy tắc. Không đổi gì.
pub fn resolve_preview(state: &AppState, scope: &ResolveScopeDto, q: &FaceQueryDto) -> Result<ResolvePlanDto, CmdError> {
    let mut groups = duplicates::current_groups(state, Some(q))?;
    if let ResolveScopeDto::Group { kind, key } = scope {
        groups.retain(|g| g.kind == *kind && g.key == *key);
        if groups.is_empty() {
            return Err(CmdError::new("not_found", "nhóm không còn tồn tại"));
        }
    }
    let ctx = ctx_for(state, &groups)?;
    let mut plan = ResolvePlanDto { items: vec![], needs_review: 0, hide_count: 0, keep_count: 0, reclaimable_bytes: 0 };
    for g in &groups {
        let Some(built) = build(state, &ctx, g)? else { continue };
        let cands: Vec<Cand> = built.iter().map(|b| b.cand.clone()).collect();
        let Some(d) = pick_keeper(g.kind, g.newest_known, &cands) else { continue };
        if !d.auto_safe {
            plan.needs_review += 1;
            if matches!(scope, ResolveScopeDto::AllSafe) {
                continue;
            }
        }
        let item = plan_item(g, &built, &d, &ctx);
        plan.keep_count += 1;
        plan.hide_count += item.losers.len() as u32;
        plan.reclaimable_bytes += item.losers.iter().map(|l| l.file_size).sum::<u64>();
        plan.items.push(item);
    }
    Ok(plan)
}

/// Backend không trả câu cho người dùng (app có vi + en, UI dịch theo `code`). `note` chỉ được giữ làm chẩn đoán thô
/// cho vài mã; các mã khác bỏ nó.
fn fail(path: &str, code: &str, note: impl Into<String>) -> ResolveFailureDto {
    let detail = matches!(code, "trash_failed" | "db_error" | "not_allowed").then(|| note.into());
    ResolveFailureDto { path: path.into(), code: code.into(), detail }
}

fn to_dto(r: &ResolutionRow, name: Option<(String, Option<String>)>, trash_still_there: bool) -> ResolutionDto {
    let action = if r.action == "trash" { ResolveActionDto::Trash } else { ResolveActionDto::Hide };
    let reason: serde_json::Value = serde_json::from_str(&r.reason).unwrap_or_default();
    let args = reason["args"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect()).unwrap_or_default();
    ResolutionDto {
        id: r.id,
        batch_id: r.batch_id.clone(),
        kind: kind_of(&r.kind),
        group_key: r.group_key.clone(),
        keeper: key_dto(&r.keeper),
        keeper_path: r.keeper_path.clone(),
        loser: key_dto(&r.loser),
        loser_path: r.loser_path.clone(),
        action,
        trash_path: r.trash_path.clone(),
        reason: ReasonDto { code: reason["code"].as_str().unwrap_or("unknown").to_string(), args },
        was_active: r.was_active,
        created_at: r.created_at,
        undone_at: r.undone_at,
        links_moved: r.links_moved,
        family: name.as_ref().map(|n| n.0.clone()),
        style: name.and_then(|n| n.1),
        can_undo: r.undone_at.is_none() && (action == ResolveActionDto::Hide || trash_still_there),
    }
}

fn names_of(state: &AppState, key: &FaceKey) -> Option<(String, Option<String>)> {
    state
        .with_store(|s| {
            let q = fontatlas_store::FaceQuery { include_missing: true, include_hidden: true, limit: 1000, ..Default::default() };
            Ok(s.query_faces(&q)?.items.into_iter().find(|r| r.hash_hex == key.hash_hex() && r.face_index == key.face_index).map(|r| (r.family, r.subfamily)))
        })
        .ok()
        .flatten()
}

fn scope_of(s: ActivationScope) -> Scope {
    match s {
        ActivationScope::Session => Scope::Session,
        ActivationScope::Persistent => Scope::Persistent,
    }
}

/// Áp dụng. Mỗi nhóm độc lập: lỗi một nhóm không chặn nhóm khác; mọi lỗi nằm trong `failed`.
/// `trasher` chuyển tệp vào Thùng rác và trả đường dẫn mới (tiêm để test).
pub fn resolve_apply(
    state: &AppState,
    req: &ResolveRequestDto,
    trasher: &dyn Fn(&Path) -> Result<PathBuf, String>,
    now: i64,
) -> Result<ResolveResultDto, CmdError> {
    let batch_id = format!("dup-{now}-{}", std::process::id());
    let mut groups = duplicates::current_groups(state, None)?;
    groups.retain(|g| req.items.iter().any(|i| i.kind == g.kind && i.key == g.key));
    let ctx = ctx_for(state, &groups)?;
    let mut done = Vec::new();
    let mut failed = Vec::new();

    for item in &req.items {
        let Some(g) = groups.iter().find(|g| g.kind == item.kind && g.key == item.key) else {
            failed.push(fail(&item.keeper_path, "not_found", "nhóm không còn tồn tại"));
            continue;
        };
        let Some(built) = build(state, &ctx, g)? else {
            failed.push(fail(&item.keeper_path, "not_found", "nhóm đã thay đổi"));
            continue;
        };
        let Some(keeper) = built.iter().find(|b| b.cand.path == item.keeper_path) else {
            failed.push(fail(&item.keeper_path, "keeper_not_in_group", "bản giữ không thuộc nhóm"));
            continue;
        };
        if !Path::new(&keeper.cand.path).is_file() {
            failed.push(fail(&keeper.cand.path, "keeper_missing", "bản giữ không còn trên đĩa — không ẩn bản nào"));
            continue;
        }
        let reason = {
            let cands: Vec<Cand> = built.iter().map(|b| b.cand.clone()).collect();
            let kidx = built.iter().position(|b| b.cand.path == item.keeper_path).unwrap_or(0);
            pick_keeper(g.kind, g.newest_known, &cands)
                .filter(|d| d.keeper == kidx)
                .map(|d| d.reason)
                .unwrap_or_else(|| Reason::new("chosen_by_user"))
        };
        for lp in &item.loser_paths {
            if *lp == item.keeper_path {
                failed.push(fail(lp, "not_allowed", "bản giữ không thể đồng thời là bản bị ẩn"));
                continue;
            }
            let Some(loser) = built.iter().find(|b| b.cand.path == *lp) else {
                failed.push(fail(lp, "loser_not_in_group", "bản này không thuộc nhóm"));
                continue;
            };
            if loser.cand.num_faces > 1 {
                failed.push(fail(lp, "collection_file", "tệp có nhiều face (TTC/OTC): không tự ẩn để khỏi ẩn nhầm face khác"));
                continue;
            }
            let same_hash = loser.member.key == keeper.member.key;
            let row = state.with_store(|s| Ok(s.activation_row(&loser.member.key.hash)?))?;
            let was_active = row.as_ref().is_some_and(|r| r.state == DesiredState::Active);
            if same_hash && was_active && row.as_ref().and_then(|r| r.active_path.as_deref()) == Some(lp.as_str()) {
                failed.push(fail(lp, "active_copy_in_use", "font đang bật bằng chính bản sao này; tắt font rồi giải quyết lại"));
                continue;
            }
            let do_trash = req.action == ResolveActionDto::Trash;
            if do_trash {
                let t = trash_eligibility(lp, loser.source.as_ref().map(|s| s.kind.as_str()).unwrap_or(""), loser.cand.num_faces, Path::new(lp).is_file(), ctx.home.as_deref());
                if !t.allowed || (t.needs_explicit && !req.explicit_trash_paths.contains(lp)) {
                    failed.push(fail(lp, "not_allowed", t.blocked.unwrap_or_else(|| "needs_explicit".into())));
                    continue;
                }
            }
            // tắt bản thua đang bật (khác hash) rồi bật bản giữ cùng phạm vi
            if !same_hash && was_active && state.activation.supported() {
                let _ = state.activation.deactivate_faces(state.store()?, &[loser.member.key], DeactivatePolicy::Force, &mut |_| {});
                if let Some(r) = &row {
                    let _ = state.activation.activate_faces(state.store()?, &[keeper.member.key], Some(scope_of(r.scope)), ConflictPolicy::Proceed, &mut |_| {});
                }
            }
            let moved = if do_trash {
                match trasher(Path::new(lp)) {
                    Ok(p) => Some(p.to_string_lossy().into_owned()),
                    Err(e) => {
                        failed.push(fail(lp, "trash_failed", e));
                        continue;
                    }
                }
            } else {
                None
            };
            let new = NewResolution {
                kind: kind_str(g.kind).into(),
                group_key: g.key.clone(),
                keeper: keeper.member.key,
                keeper_path: keeper.cand.path.clone(),
                loser: loser.member.key,
                loser_path: lp.clone(),
                action: if moved.is_some() { "trash" } else { "hide" }.into(),
                trash_path: moved.clone(),
                reason: reason.json(),
                was_active,
            };
            match state.with_store(|s| Ok(s.apply_resolutions(&batch_id, std::slice::from_ref(&new), now)?)) {
                Ok(ids) => {
                    let r = state.with_store(|s| Ok(s.resolution(ids[0])?))?;
                    done.push(to_dto(&r, Some((loser.row.family.clone(), loser.row.subfamily.clone())), true));
                }
                Err(e) => {
                    // sổ không ghi được: đưa tệp về chỗ cũ để không để trạng thái nửa vời
                    if let Some(t) = &moved {
                        let _ = std::fs::rename(t, lp);
                    }
                    failed.push(fail(lp, "db_error", e.message));
                }
            }
        }
    }
    duplicates::invalidate_cache();
    Ok(ResolveResultDto { batch_id, done, failed, needs_rescan: false })
}

/// Hoàn tác: đưa tệp từ Thùng rác về chỗ cũ (nếu có) rồi gỡ ẩn + liên kết đã chuyển.
pub fn resolve_undo(state: &AppState, ids: &[i64], now: i64) -> Result<ResolveResultDto, CmdError> {
    let mut done = Vec::new();
    let mut failed = Vec::new();
    let mut needs_rescan = false;
    for &id in ids {
        let r = match state.with_store(|s| Ok(s.resolution(id)?)) {
            Ok(r) => r,
            Err(_) => {
                failed.push(fail("", "not_found", format!("quyết định #{id}")));
                continue;
            }
        };
        if r.undone_at.is_some() {
            failed.push(fail(&r.loser_path, "already_undone", "đã hoàn tác rồi"));
            continue;
        }
        if r.action == "trash" {
            let Some(t) = r.trash_path.as_deref().filter(|t| Path::new(t).exists()) else {
                failed.push(fail(&r.loser_path, "trash_emptied", "tệp không còn trong Thùng rác (đã dọn)"));
                continue;
            };
            if Path::new(&r.loser_path).exists() {
                failed.push(fail(&r.loser_path, "path_occupied", "chỗ cũ đã có tệp khác"));
                continue;
            }
            if let Err(e) = std::fs::rename(t, &r.loser_path) {
                failed.push(fail(&r.loser_path, "trash_failed", e.to_string()));
                continue;
            }
            needs_rescan = true;
        }
        match state.with_store(|s| Ok(s.undo_resolution(id, now)?)) {
            Ok(u) => done.push(to_dto(&u, names_of(state, &u.loser), false)),
            Err(e) => failed.push(fail(&r.loser_path, "db_error", e.message)),
        }
    }
    duplicates::invalidate_cache();
    Ok(ResolveResultDto { batch_id: String::new(), done, failed, needs_rescan })
}

/// Các quyết định đã ghi, mới nhất trước (trang "Đã giải quyết").
pub fn resolve_list(state: &AppState, include_undone: bool) -> Result<Vec<ResolutionDto>, CmdError> {
    let rows = state.with_store(|s| Ok(s.resolutions(!include_undone)?))?;
    Ok(rows
        .iter()
        .map(|r| {
            let there = r.trash_path.as_deref().is_some_and(|t| Path::new(t).exists());
            to_dto(r, names_of(state, &r.loser), there)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trash_is_limited_to_user_folders() {
        let home = Path::new("/Users/me");
        let e = |p: &str, k: &str, f: u32, ex: bool| trash_eligibility(p, k, f, ex, Some(home));
        assert!(e("/Users/me/Fonts/A.ttf", "directory", 1, true).allowed);
        assert_eq!(e("/Users/me/Fonts/A.ttf", "system", 1, true).blocked.as_deref(), Some("managed_source"));
        assert_eq!(e("/Users/me/x/A.ttf", "google_fonts", 1, true).blocked.as_deref(), Some("managed_source"));
        assert_eq!(e("/Library/Fonts/A.ttf", "directory", 1, true).blocked.as_deref(), Some("system_path"));
        assert_eq!(e("/System/Library/Fonts/A.ttf", "directory", 1, true).blocked.as_deref(), Some("system_path"));
        assert_eq!(e("/Users/me/Fonts/A.ttc", "directory", 4, true).blocked.as_deref(), Some("collection_file"));
        assert_eq!(e("/Users/me/Fonts/A.ttf", "directory", 1, false).blocked.as_deref(), Some("missing_file"));
        let lib = e("/Users/me/Library/Fonts/A.ttf", "directory", 1, true);
        assert!(lib.allowed && lib.needs_explicit, "~/Library/Fonts cần chọn rõ từng tệp");
    }
}
