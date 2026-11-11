//! Giải quyết trùng lặp: xem trước kế hoạch → áp dụng (ẩn / Thùng rác) → hoàn tác. Quy tắc chọn bản giữ ở `dup_pick`;
//! sổ quyết định và chuyển liên kết ở `fontatlas_store::dup_resolve`. KHÔNG xoá vĩnh viễn, KHÔNG đụng nguồn hệ thống.
//!
//! Thứ tự an toàn cho MỖI bản thua (mỗi bản độc lập, lỗi một bản không chặn bản khác):
//! kiểm tra → ghi sổ `pending` (đã ẩn) → tắt font đang bật → chuyển Thùng rác → đánh dấu `done` → bật bản giữ.
//! Mọi thay đổi ngoài sổ đều SAU khi sổ đã ghi, và mọi bước lỗi đều đảo lại các bước trước (lỗi đảo lại được báo, không nuốt).
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use fontatlas_activate::{ConflictPolicy, DeactivatePolicy, FileProbe, Scope, probe_file};
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

fn reason_dto(r: &Reason) -> ResolveReasonDto {
    ResolveReasonDto { code: r.code.to_string(), args: r.args.clone() }
}

fn kind_str(k: DuplicateKindDto) -> &'static str {
    duplicates::kind_name(k)
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

// ---------- an toàn tệp ----------

/// Tệp có được chuyển vào Thùng rác không. Chỉ tệp THẬT (không symlink) nằm dưới gốc của một nguồn `directory`, sau khi
/// `canonicalize` cả hai phía; không bao giờ /System hay /Library; ~/Library/Fonts cần người dùng chọn rõ từng tệp.
pub fn check_trash(path: &str, source: Option<&Source>, num_faces: u32, home: Option<&Path>) -> TrashEligibilityDto {
    let blocked = |b: &str| TrashEligibilityDto { allowed: false, needs_explicit: false, blocked: Some(b.into()) };
    let Some(src) = source.filter(|s| s.kind.as_str() == "directory") else { return blocked("managed_source") };
    let p = Path::new(path);
    if std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()) {
        return blocked("symlink");
    }
    let Ok(canon) = std::fs::canonicalize(p) else { return blocked("missing_file") };
    if !canon.is_file() {
        return blocked("missing_file");
    }
    for sys in ["/System", "/Library", "/private/var/db", "/Applications"] {
        if canon.starts_with(sys) || Path::new(path).starts_with(sys) {
            return blocked("system_path");
        }
    }
    let Some(root) = src.path.as_deref().and_then(|r| std::fs::canonicalize(r).ok()) else { return blocked("outside_source") };
    if !canon.starts_with(&root) {
        return blocked("outside_source");
    }
    if num_faces > 1 {
        return blocked("collection_file");
    }
    let user_fonts = home.and_then(|h| std::fs::canonicalize(h.join("Library/Fonts")).ok());
    let needs_explicit = user_fonts.is_some_and(|f| canon.starts_with(f));
    TrashEligibilityDto { allowed: true, needs_explicit, blocked: None }
}

fn mtime_ns_of(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos() as i64)
}

/// Tệp trên đĩa còn đúng là tệp đã quét? `with_hash` thì đọc cả nội dung (BLAKE3 phải khớp hash của face).
fn verify_file(path: &str, hash: &[u8; 32], info: &MemberInfo, with_hash: bool) -> Result<(), &'static str> {
    let p = Path::new(path);
    let Ok(m) = std::fs::metadata(p) else { return Err("missing") };
    if m.len() != info.size || mtime_ns_of(&m) != info.mtime_ns {
        return Err("changed");
    }
    if with_hash {
        let Ok(bytes) = std::fs::read(p) else { return Err("missing") };
        if blake3::hash(&bytes).as_bytes() != hash {
            return Err("changed");
        }
    }
    Ok(())
}

/// Bản giữ phải đọc được ngay trên máy (không phải placeholder cloud), đúng tệp đã quét; với Thùng rác còn kiểm hash.
fn check_keeper(path: &str, key: &FaceKey, info: &MemberInfo, trash: bool) -> Result<(), &'static str> {
    match probe_file(Path::new(path)) {
        FileProbe::Missing => return Err("keeper_missing"),
        FileProbe::NotDownloaded => return Err("keeper_not_local"),
        FileProbe::Ready => {}
    }
    verify_file(path, &key.hash, info, trash).map_err(|e| if e == "missing" { "keeper_missing" } else { "keeper_changed" })
}

// ---------- dựng kế hoạch ----------

struct Ctx {
    rows: HashMap<i64, FaceRowDto>,
    sources: Vec<Source>,
    home: Option<PathBuf>,
}

fn ctx_for(state: &AppState, groups: &[RawGroup]) -> Result<Ctx, CmdError> {
    let ids: Vec<i64> = groups.iter().flat_map(|g| g.members.iter().map(|m| m.face_id)).collect::<HashSet<_>>().into_iter().collect();
    let rows = duplicates::rows_by_id(state, &ids)?;
    // thứ tự ưu tiên người dùng đặt (mặc định: nguồn thêm trước); `rank` = vị trí trong danh sách này
    let (mut sources, order) = state.with_store(|s| Ok((s.sources()?, s.source_priority()?)))?;
    sources.sort_by_key(|s| order.iter().position(|i| *i == s.id).unwrap_or(usize::MAX));
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
        trash: check_trash(&b.cand.path, b.source.as_ref(), b.cand.num_faces, ctx.home.as_deref()),
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
        member_paths: built.iter().map(|b| b.cand.path.clone()).collect(),
    }
}

/// Thứ tự nguồn ưu tiên giữ bản trùng (id, ưu tiên nhất trước).
pub fn source_priority(state: &AppState) -> Result<Vec<i64>, CmdError> {
    state.with_store(|s| Ok(s.source_priority()?))
}

/// Đặt thứ tự (id nêu trước đứng đầu, nguồn không nêu xếp sau); trả thứ tự đầy đủ mới. Sau đó xem trước kế hoạch sẽ khác.
pub fn set_source_priority(state: &AppState, ids: &[i64]) -> Result<Vec<i64>, CmdError> {
    state.with_store(|s| {
        s.set_source_priority(ids)?;
        Ok(s.source_priority()?)
    })
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

// ---------- áp dụng ----------

fn fail(path: &str, code: &str, note: impl Into<String>) -> ResolveFailureDto {
    // Backend không trả câu cho người dùng (app có vi + en, UI dịch theo `code`). `note` chỉ giữ làm chẩn đoán thô.
    let detail = matches!(code, "trash_failed" | "db_error" | "not_allowed" | "rollback_failed" | "deactivate_failed" | "reactivate_failed" | "keeper_activation_failed" | "keeper_deactivate_failed" | "trash_mismatch").then(|| note.into());
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
        reason: ResolveReasonDto { code: reason["code"].as_str().unwrap_or("unknown").to_string(), args },
        was_active: r.was_active,
        created_at: r.created_at,
        undone_at: r.undone_at,
        links_moved: r.links_moved,
        family: name.as_ref().map(|n| n.0.clone()),
        style: name.and_then(|n| n.1),
        status: r.status.clone(),
        keeper_activated: r.keeper_activated,
        can_undo: r.undone_at.is_none() && (action == ResolveActionDto::Hide || trash_still_there || r.status == "pending"),
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

fn scope_str(s: ActivationScope) -> &'static str {
    match s {
        ActivationScope::Session => "session",
        ActivationScope::Persistent => "persistent",
    }
}

fn scope_parse(s: Option<&str>) -> Scope {
    if s == Some("session") { Scope::Session } else { Scope::Persistent }
}

/// Cho worker chạy trên kết nối chỉ-library (không thấy sổ quyết định) biết vị trí đã đi vào / ra khỏi Thùng rác.
fn mark_missing(state: &AppState, path: &str, missing: bool, failed: &mut Vec<ResolveFailureDto>) {
    if let Err(e) = state.with_store(|s| Ok(s.set_location_missing(path, missing).map(|_| ())?)) {
        failed.push(fail(path, "db_error", e.message));
    }
}

/// Chỉ MỘT lệnh resolve chạy tại một thời điểm: `resolve_list` cũng dọn `pending`, không được đụng dòng pending của một
/// `resolve_apply` đang chạy giữa chừng.
static RESOLVE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    RESOLVE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Chuyển tệp KHÔNG ghi đè: hard-link sang đích (lỗi `AlreadyExists` nếu đã có gì ở đó) rồi gỡ nguồn. Nếu gỡ nguồn lỗi thì
/// gỡ liên kết mới để không để tệp ở hai nơi. Cùng volume (Thùng rác của volume chứa tệp), khác volume → lỗi rõ ràng.
fn move_no_clobber(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::hard_link(from, to)?;
    if let Err(e) = std::fs::remove_file(from) {
        let _ = std::fs::remove_file(to);
        return Err(e);
    }
    Ok(())
}

/// Chỉ cho test tích hợp (crate test không thấy hàm private).
#[doc(hidden)]
pub fn move_no_clobber_for_test(from: &Path, to: &Path) -> std::io::Result<()> {
    move_no_clobber(from, to)
}

fn is_active(state: &AppState, key: &FaceKey) -> bool {
    state.with_store(|s| Ok(s.activation_row(&key.hash)?)).ok().flatten().is_some_and(|a| a.state == DesiredState::Active)
}

/// Dọn các quyết định còn `pending` sau sự cố giữa chừng, theo cả trạng thái ACTIVATION đã ghi trong sổ:
/// `hide` → hoàn tất và bảo đảm bản thua đã tắt (sự cố có thể xảy ra trước khi tắt). `trash` mà tệp còn ở chỗ cũ → huỷ và
/// bật lại bản thua nếu nó từng bật mà nay đã tắt. `trash` mà tệp đã đi (không biết đâu) → hoàn tất không có `trash_path`
/// (không hoàn tác tự động) và bảo đảm bản thua đã tắt.
pub fn recover_pending_report(state: &AppState, now: i64) -> Result<(usize, Vec<ResolveFailureDto>), CmdError> {
    let pending = state.with_store(|s| Ok(s.pending_resolutions()?))?;
    let n = pending.len();
    let mut failed = Vec::new();
    for r in pending {
        let revives = r.was_active && r.keeper != r.loser; // bản thua từng bật và khác hash với bản giữ
        if r.action == "trash" && Path::new(&r.loser_path).exists() {
            state.with_store(|s| Ok(s.undo_resolution(r.id, now).map(|_| ())?))?;
            if revives && state.activation.supported() && !is_active(state, &r.loser) {
                reactivate(state, &r.loser, scope_parse(r.loser_scope.as_deref()), &r.loser_path, &mut failed);
            }
        } else {
            state.with_store(|s| Ok(s.complete_resolution(r.id, None)?))?;
            if revives
                && state.activation.supported()
                && is_active(state, &r.loser)
                && let Err(e) = set_active(state, &r.loser, false, Scope::Persistent)
            {
                failed.push(fail(&r.loser_path, "deactivate_failed", e));
            }
        }
    }
    if n > 0 {
        duplicates::invalidate_cache();
    }
    Ok((n, failed))
}

pub fn recover_pending(state: &AppState, now: i64) -> Result<usize, CmdError> {
    Ok(recover_pending_report(state, now)?.0)
}

/// Đảo ghi sổ pending của một bản thua (huỷ giữa chừng). Lỗi đảo là lỗi nặng: báo `rollback_failed`, KHÔNG nuốt.
fn abort(state: &AppState, id: i64, now: i64, path: &str, failed: &mut Vec<ResolveFailureDto>) {
    if let Err(e) = state.with_store(|s| Ok(s.undo_resolution(id, now).map(|_| ())?)) {
        failed.push(fail(path, "rollback_failed", format!("ledger #{id}: {}", e.message)));
    }
}

/// `Ok(report)` của service activation vẫn có thể chứa kết quả THẤT BẠI theo từng tệp: chỉ coi là xong khi mọi tệp
/// đạt đúng trạng thái mong muốn.
fn report_ok(r: &fontatlas_activate::ActivationReport, want_active: bool) -> Result<(), String> {
    use fontatlas_activate::FileOutcome as O;
    for f in &r.results {
        let ok = if want_active {
            matches!(f.outcome, O::Activated | O::AlreadyActive | O::AlreadyAvailable { .. })
        } else {
            matches!(f.outcome, O::Deactivated | O::AlreadyInactive)
        };
        if !ok {
            return Err(format!("{:?}", f.outcome));
        }
    }
    Ok(())
}

fn set_active(state: &AppState, key: &FaceKey, on: bool, scope: Scope) -> Result<(), String> {
    let store = state.store().map_err(|e| e.message)?;
    let r = if on {
        state.activation.activate_faces(store, &[*key], Some(scope), ConflictPolicy::Proceed, &mut |_| {})
    } else {
        state.activation.deactivate_faces(store, &[*key], DeactivatePolicy::Force, &mut |_| {})
    };
    report_ok(&r.map_err(|e| e.to_string())?, on)
}

fn reactivate(state: &AppState, key: &FaceKey, scope: Scope, path: &str, failed: &mut Vec<ResolveFailureDto>) {
    if !state.activation.supported() {
        return;
    }
    if let Err(e) = set_active(state, key, true, scope) {
        failed.push(fail(path, "reactivate_failed", e));
    }
}

/// `joinBatch` hợp lệ? Mẻ do CHÍNH phiên này tạo (id `dup-<ms>-<pid>`, pid hiện tại), có ít nhất một dòng, và còn ít nhất
/// một dòng chưa hoàn tác. Lỗi mang mã cho UI dịch; không đổi gì.
fn check_joinable_batch(state: &AppState, batch: &str) -> Result<String, CmdError> {
    let rows: Vec<ResolutionRow> = state.with_store(|s| Ok(s.resolutions(false)?))?.into_iter().filter(|r| r.batch_id == batch).collect();
    if rows.is_empty() {
        return Err(CmdError::new("batch_not_found", batch));
    }
    let mine = batch.strip_prefix("dup-").and_then(|r| r.rsplit_once('-')).is_some_and(|(_, pid)| pid == std::process::id().to_string());
    if !mine {
        return Err(CmdError::new("batch_other_session", batch));
    }
    if rows.iter().all(|r| r.undone_at.is_some()) {
        return Err(CmdError::new("batch_undone", batch));
    }
    Ok(batch.to_string())
}

/// Áp dụng. Mỗi bản thua độc lập; mọi lỗi nằm trong `failed` (xem `ResolveFailureDto.code`).
/// `trasher` chuyển tệp vào Thùng rác và trả đường dẫn mới (tiêm để test).
pub fn resolve_apply(
    state: &AppState,
    req: &ResolveRequestDto,
    trasher: &dyn Fn(&Path) -> Result<PathBuf, String>,
    now: i64,
) -> Result<ResolveResultDto, CmdError> {
    let _guard = lock();
    let (_, mut failed) = recover_pending_report(state, now)?;
    let batch_id = match &req.join_batch {
        Some(b) => check_joinable_batch(state, b)?,
        None => format!("dup-{now}-{}", std::process::id()),
    };
    let all_groups = duplicates::current_groups(state, None)?;
    let ctx = ctx_for(state, &all_groups)?;
    let trash = req.action == ResolveActionDto::Trash;
    let mut done = Vec::new();
    // trong một request: đường dẫn đã làm bản giữ / bản thua ở mục trước
    let mut kept: HashSet<String> = HashSet::new();
    let mut lost: HashSet<String> = HashSet::new();

    for item in &req.items {
        let Some(g) = all_groups.iter().find(|g| g.kind == item.kind && g.key == item.key) else {
            failed.push(fail(&item.keeper_path, "not_found", ""));
            continue;
        };
        let Some(built) = build(state, &ctx, g)? else {
            failed.push(fail(&item.keeper_path, "not_found", ""));
            continue;
        };
        let current: BTreeSet<&str> = built.iter().map(|b| b.cand.path.as_str()).collect();

        // chọn bản giữ / bản thua: AllSafe tính lại ở backend (bỏ qua mọi thứ UI gửi), Chosen kiểm plan còn mới
        let (keeper_path, loser_paths): (String, Vec<String>) = match req.mode {
            ResolveModeDto::AllSafe => {
                let cands: Vec<Cand> = built.iter().map(|b| b.cand.clone()).collect();
                match pick_keeper(g.kind, g.newest_known, &cands) {
                    Some(d) if d.auto_safe => (built[d.keeper].cand.path.clone(), d.losers.iter().map(|&i| built[i].cand.path.clone()).collect()),
                    _ => {
                        failed.push(fail(&item.keeper_path, "not_auto_safe", ""));
                        continue;
                    }
                }
            }
            ResolveModeDto::Chosen => {
                let seen: BTreeSet<&str> = item.member_paths.iter().map(String::as_str).collect();
                if item.member_paths.is_empty() || seen != current {
                    failed.push(fail(&item.keeper_path, "plan_stale", ""));
                    continue;
                }
                (item.keeper_path.clone(), item.loser_paths.iter().collect::<BTreeSet<_>>().into_iter().cloned().collect())
            }
        };

        let Some(keeper) = built.iter().find(|b| b.cand.path == keeper_path) else {
            failed.push(fail(&keeper_path, "keeper_not_in_group", ""));
            continue;
        };
        if state.with_store(|s| Ok(s.is_hidden_path(&keeper_path)?))? {
            failed.push(fail(&keeper_path, "keeper_is_hidden", ""));
            continue;
        }
        if lost.contains(&keeper_path) || loser_paths.iter().any(|l| kept.contains(l)) {
            failed.push(fail(&keeper_path, "keeper_conflict", ""));
            continue;
        }
        if let Err(code) = check_keeper(&keeper.cand.path, &keeper.member.key, &keeper.info, trash) {
            failed.push(fail(&keeper.cand.path, code, ""));
            continue;
        }
        // chỉ đánh dấu "đã làm bản giữ" khi ít nhất một bản thua thực sự được xử lý (xem cuối vòng lặp)
        let done_before = done.len();

        for lp in &loser_paths {
            if *lp == keeper_path {
                failed.push(fail(lp, "not_allowed", "keeper_is_loser"));
                continue;
            }
            let Some(loser) = built.iter().find(|b| b.cand.path == *lp) else {
                failed.push(fail(lp, "loser_not_in_group", ""));
                continue;
            };
            if loser.cand.num_faces > 1 {
                failed.push(fail(lp, "collection_file", ""));
                continue;
            }
            let same_hash = loser.member.key == keeper.member.key;
            let row = state.with_store(|s| Ok(s.activation_row(&loser.member.key.hash)?))?;
            let was_active = row.as_ref().is_some_and(|r| r.state == DesiredState::Active);
            if same_hash && was_active && row.as_ref().and_then(|r| r.active_path.as_deref()) == Some(lp.as_str()) {
                failed.push(fail(lp, "active_copy_in_use", ""));
                continue;
            }
            if trash {
                let t = check_trash(lp, loser.source.as_ref(), loser.cand.num_faces, ctx.home.as_deref());
                if !t.allowed || (t.needs_explicit && !req.explicit_trash_paths.contains(lp)) {
                    failed.push(fail(lp, "not_allowed", t.blocked.unwrap_or_else(|| "needs_explicit".into())));
                    continue;
                }
                // tệp thua phải còn đúng là tệp đã quét (kể cả nội dung) trước khi đưa vào Thùng rác
                if verify_file(lp, &loser.member.key.hash, &loser.info, true).is_err() {
                    failed.push(fail(lp, "file_changed", ""));
                    continue;
                }
            }
            let loser_scope = row.as_ref().filter(|_| was_active && !same_hash).map(|r| scope_str(r.scope).to_string());
            let reason = {
                let cands: Vec<Cand> = built.iter().map(|b| b.cand.clone()).collect();
                let kidx = built.iter().position(|b| b.cand.path == keeper_path).unwrap_or(0);
                pick_keeper(g.kind, g.newest_known, &cands).filter(|d| d.keeper == kidx).map(|d| d.reason).unwrap_or_else(|| Reason::new("chosen_by_user"))
            };
            let new = NewResolution {
                kind: kind_str(g.kind).into(),
                group_key: g.key.clone(),
                keeper: keeper.member.key,
                keeper_path: keeper_path.clone(),
                loser: loser.member.key,
                loser_path: lp.clone(),
                action: if trash { "trash" } else { "hide" }.into(),
                trash_path: None,
                reason: reason.json(),
                was_active: was_active && !same_hash,
                loser_scope: loser_scope.clone(),
            };

            // 1. ghi sổ pending (từ đây bản thua đã ẩn)
            let id = match state.with_store(|s| Ok(s.begin_resolution(&batch_id, &new, now)?)) {
                Ok(id) => id,
                Err(e) => {
                    failed.push(fail(lp, "db_error", e.message));
                    continue;
                }
            };
            let mut side = Vec::new();
            // 2. tắt font đang bật (khác hash); lỗi → đảo sổ, không ẩn một font còn bật
            let mut deactivated = false;
            if !same_hash && was_active && state.activation.supported() {
                match set_active(state, &loser.member.key, false, Scope::Persistent) {
                    Ok(()) => deactivated = true,
                    Err(e) => {
                        abort(state, id, now, lp, &mut side);
                        failed.push(fail(lp, "deactivate_failed", e));
                        failed.append(&mut side);
                        continue;
                    }
                }
            }
            // 3. Thùng rác; lỗi → đảo sổ + bật lại bản thua đã tắt
            let mut trashed_to: Option<String> = None;
            if trash {
                match trasher(Path::new(lp)) {
                    Ok(p) => {
                        // hash lại tệp NGAY trong Thùng rác: lệch (tệp bị thay giữa kiểm tra và chuyển) → đưa về, đảo hết
                        let intact = std::fs::read(&p).is_ok_and(|b| blake3::hash(&b).as_bytes() == &loser.member.key.hash);
                        if !intact {
                            match move_no_clobber(&p, Path::new(lp)) {
                                Ok(()) => mark_missing(state, lp, false, &mut side),
                                Err(re) => side.push(fail(lp, "rollback_failed", format!("không đưa tệp từ {} về được: {re}", p.display()))),
                            }
                            abort(state, id, now, lp, &mut side);
                            if deactivated {
                                reactivate(state, &loser.member.key, scope_parse(loser_scope.as_deref()), lp, &mut side);
                            }
                            failed.push(fail(lp, "trash_mismatch", p.to_string_lossy()));
                            failed.append(&mut side);
                            continue;
                        }
                        trashed_to = Some(p.to_string_lossy().into_owned());
                        mark_missing(state, lp, true, &mut side);
                    }
                    Err(e) => {
                        abort(state, id, now, lp, &mut side);
                        if deactivated {
                            reactivate(state, &loser.member.key, scope_parse(loser_scope.as_deref()), lp, &mut side);
                        }
                        failed.push(fail(lp, "trash_failed", e));
                        failed.append(&mut side);
                        continue;
                    }
                }
            }
            // 4. đánh dấu xong; lỗi → đưa tệp về chỗ cũ (lỗi đưa về được báo), đảo sổ, bật lại
            if let Err(e) = state.with_store(|s| Ok(s.complete_resolution(id, trashed_to.as_deref())?)) {
                if let Some(t) = &trashed_to
                    && let Err(re) = move_no_clobber(Path::new(t), Path::new(lp))
                {
                    failed.push(fail(lp, "rollback_failed", format!("không đưa tệp từ {t} về được: {re}")));
                } else if trashed_to.is_some() {
                    mark_missing(state, lp, false, &mut side);
                }
                abort(state, id, now, lp, &mut side);
                if deactivated {
                    reactivate(state, &loser.member.key, scope_parse(loser_scope.as_deref()), lp, &mut side);
                }
                failed.push(fail(lp, "db_error", e.message));
                failed.append(&mut side);
                continue;
            }
            // 5. bật bản giữ giúp người dùng nếu nó chưa bật (cùng phạm vi); lỗi chỉ là cảnh báo, sổ đã xong
            if deactivated {
                let keeper_active = state.with_store(|s| Ok(s.activation_row(&keeper.member.key.hash)?))?.is_some_and(|a| a.state == DesiredState::Active);
                if !keeper_active {
                    match set_active(state, &keeper.member.key, true, scope_parse(loser_scope.as_deref())) {
                        Ok(()) => {
                            if let Err(e) = state.with_store(|s| Ok(s.set_keeper_activated(id, true)?)) {
                                failed.push(fail(lp, "db_error", e.message));
                            }
                        }
                        Err(e) => failed.push(fail(lp, "keeper_activation_failed", e)),
                    }
                }
            }
            failed.append(&mut side); // cảnh báo không chặn (vd không đánh dấu missing được) KHÔNG được nuốt
            lost.insert(lp.clone());
            let r = state.with_store(|s| Ok(s.resolution(id)?))?;
            done.push(to_dto(&r, Some((loser.row.family.clone(), loser.row.subfamily.clone())), true));
        }
        if done.len() > done_before {
            kept.insert(keeper_path.clone());
        }
    }
    duplicates::invalidate_cache();
    Ok(ResolveResultDto { batch_id, done, failed, needs_rescan: false })
}

// ---------- hoàn tác ----------

/// Hoàn tác theo thứ tự NGƯỢC (mới nhất trước) nên một mẻ chuỗi A→B→C hoàn tác được; hoàn tác riêng một quyết định cũ
/// khi còn quyết định mới hơn dùng bản giữ của nó làm bản thua bị từ chối (`newer_resolution_depends`).
pub fn resolve_undo(state: &AppState, ids: &[i64], now: i64) -> Result<ResolveResultDto, CmdError> {
    let _guard = lock();
    let (_, mut failed) = recover_pending_report(state, now)?;
    let mut ordered: Vec<i64> = ids.to_vec();
    ordered.sort_unstable_by(|a, b| b.cmp(a));
    ordered.dedup();
    let mut done = Vec::new();
    let mut needs_rescan = false;
    for id in ordered {
        let r = match state.with_store(|s| Ok(s.resolution(id)?)) {
            Ok(r) => r,
            Err(_) => {
                failed.push(fail("", "not_found", ""));
                continue;
            }
        };
        if r.undone_at.is_some() {
            failed.push(fail(&r.loser_path, "already_undone", ""));
            continue;
        }
        if !state.with_store(|s| Ok(s.newer_dependents(id)?))?.is_empty() {
            failed.push(fail(&r.loser_path, "newer_resolution_depends", ""));
            continue;
        }
        let mut moved_back = false;
        if r.action == "trash" && r.trash_path.is_some() {
            let t = r.trash_path.as_deref().unwrap_or_default();
            if !Path::new(t).exists() {
                failed.push(fail(&r.loser_path, "trash_emptied", ""));
                continue;
            }
            if Path::new(&r.loser_path).exists() {
                failed.push(fail(&r.loser_path, "path_occupied", ""));
                continue;
            }
            // chỉ đưa về nếu đúng là tệp đã chuyển đi (tránh đưa nhầm một tệp cùng tên trong Thùng rác)
            match std::fs::read(t) {
                Ok(bytes) if blake3::hash(&bytes).as_bytes() == &r.loser.hash => {}
                Ok(_) => {
                    failed.push(fail(&r.loser_path, "trash_mismatch", t));
                    continue;
                }
                Err(e) => {
                    failed.push(fail(&r.loser_path, "trash_failed", e.to_string()));
                    continue;
                }
            }
            // KHÔNG ghi đè: một tệp mới có thể vừa xuất hiện ở đường dẫn cũ sau lần kiểm tra ở trên
            if let Err(e) = move_no_clobber(Path::new(t), Path::new(&r.loser_path)) {
                let code = if e.kind() == std::io::ErrorKind::AlreadyExists { "path_occupied" } else { "trash_failed" };
                failed.push(fail(&r.loser_path, code, e.to_string()));
                continue;
            }
            moved_back = true;
            needs_rescan = true;
            mark_missing(state, &r.loser_path, false, &mut failed);
        }
        match state.with_store(|s| Ok(s.undo_resolution(id, now)?)) {
            Ok(u) => {
                // khôi phục activation: tắt bản giữ nếu CHÍNH ta đã bật; bật lại bản thua nếu nó từng bật
                if u.keeper_activated
                    && state.activation.supported()
                    && let Err(e) = set_active(state, &u.keeper, false, Scope::Persistent)
                {
                    failed.push(fail(&u.loser_path, "keeper_deactivate_failed", e));
                }
                if u.was_active && u.keeper != u.loser {
                    reactivate(state, &u.loser, scope_parse(u.loser_scope.as_deref()), &u.loser_path, &mut failed);
                }
                done.push(to_dto(&u, names_of(state, &u.loser), false));
            }
            Err(e) => {
                if moved_back && let Some(t) = r.trash_path.as_deref() {
                    match move_no_clobber(Path::new(&r.loser_path), Path::new(t)) {
                        // tệp lại ở Thùng rác → vị trí cũ lại missing (sổ vẫn nói quyết định còn sống)
                        Ok(()) => mark_missing(state, &r.loser_path, true, &mut failed),
                        Err(re) => failed.push(fail(&r.loser_path, "rollback_failed", format!("không đưa tệp về Thùng rác được: {re}"))),
                    }
                }
                failed.push(fail(&r.loser_path, "db_error", e.message));
            }
        }
    }
    duplicates::invalidate_cache();
    Ok(ResolveResultDto { batch_id: String::new(), done, failed, needs_rescan })
}

/// Các quyết định đã ghi, mới nhất trước (trang "Đã giải quyết").
pub fn resolve_list(state: &AppState, include_undone: bool) -> Result<Vec<ResolutionDto>, CmdError> {
    let _guard = lock();
    recover_pending(state, now_ms())?;
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
    use fontatlas_store::SourceKind;

    fn src(kind: SourceKind, root: &Path) -> Source {
        Source { id: 1, kind, name: "S".into(), path: Some(root.to_string_lossy().into_owned()), enabled: true, created_at: 0 }
    }

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn trash_is_limited_to_real_files_under_a_user_source_root() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Fonts");
        let f = root.join("A.ttf");
        touch(&f);
        let s = src(SourceKind::Directory, &root);
        let ok = check_trash(f.to_str().unwrap(), Some(&s), 1, None);
        assert!(ok.allowed && !ok.needs_explicit);
        // nguồn không phải thư mục người dùng
        assert_eq!(check_trash(f.to_str().unwrap(), Some(&src(SourceKind::System, &root)), 1, None).blocked.as_deref(), Some("managed_source"));
        assert_eq!(check_trash(f.to_str().unwrap(), Some(&src(SourceKind::GoogleFonts, &root)), 1, None).blocked.as_deref(), Some("managed_source"));
        assert_eq!(check_trash(f.to_str().unwrap(), None, 1, None).blocked.as_deref(), Some("managed_source"));
        // tệp nhiều face / không tồn tại
        assert_eq!(check_trash(f.to_str().unwrap(), Some(&s), 4, None).blocked.as_deref(), Some("collection_file"));
        assert_eq!(check_trash(root.join("khong.ttf").to_str().unwrap(), Some(&s), 1, None).blocked.as_deref(), Some("missing_file"));
    }

    /// Review #5: symlink lọt qua so sánh chuỗi.
    #[test]
    fn a_symlink_cannot_smuggle_a_system_or_outside_file_into_the_trash() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Fonts");
        std::fs::create_dir_all(&root).unwrap();
        let outside = d.path().join("Elsewhere/B.ttf");
        touch(&outside);
        // symlink nằm trong nguồn nhưng trỏ ra ngoài
        let link = root.join("link.ttf");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let s = src(SourceKind::Directory, &root);
        assert_eq!(check_trash(link.to_str().unwrap(), Some(&s), 1, None).blocked.as_deref(), Some("symlink"));
        // thư mục con là symlink ra ngoài: canonical không còn dưới gốc nguồn
        let dir_link = root.join("dir");
        std::os::unix::fs::symlink(d.path().join("Elsewhere"), &dir_link).unwrap();
        assert_eq!(check_trash(dir_link.join("B.ttf").to_str().unwrap(), Some(&s), 1, None).blocked.as_deref(), Some("outside_source"));
        // đường dẫn ngoài gốc nguồn
        assert_eq!(check_trash(outside.to_str().unwrap(), Some(&s), 1, None).blocked.as_deref(), Some("outside_source"));
        // hệ thống: chặn theo tiền tố canonical dù nguồn có trỏ vào đó
        let sys = src(SourceKind::Directory, Path::new("/System"));
        assert_eq!(check_trash("/System/Library/Fonts/Helvetica.ttc", Some(&sys), 1, None).blocked.as_deref().map(|b| b == "system_path" || b == "missing_file"), Some(true));
    }

    #[test]
    fn user_library_fonts_need_an_explicit_choice() {
        let d = tempfile::tempdir().unwrap();
        let home = d.path();
        let root = home.join("Library/Fonts");
        let f = root.join("A.ttf");
        touch(&f);
        let s = src(SourceKind::Directory, &root);
        let t = check_trash(f.to_str().unwrap(), Some(&s), 1, Some(home));
        assert!(t.allowed && t.needs_explicit);
    }

    #[test]
    fn verify_file_detects_a_replaced_file() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("A.ttf");
        std::fs::write(&f, b"noi dung").unwrap();
        let m = std::fs::metadata(&f).unwrap();
        let hash = *blake3::hash(b"noi dung").as_bytes();
        let info = MemberInfo { source_id: 1, num_glyphs: None, num_faces: 1, size: m.len(), mtime_ns: mtime_ns_of(&m) };
        assert!(verify_file(f.to_str().unwrap(), &hash, &info, true).is_ok());
        // cùng kích thước, nội dung khác, mtime được đặt lại như cũ → vẫn bắt được bằng hash
        std::fs::write(&f, b"noi_dung").unwrap();
        let mt = filetime_set(&f, &m);
        assert!(mt);
        assert_eq!(verify_file(f.to_str().unwrap(), &hash, &info, true), Err("changed"));
        // thay đổi kích thước → bắt ngay cả khi không hash
        std::fs::write(&f, b"dai hon nhieu").unwrap();
        assert_eq!(verify_file(f.to_str().unwrap(), &hash, &info, false), Err("changed"));
    }

    /// Đặt mtime của `p` bằng mtime của `like` (để test hash bắt được thay đổi mà mtime/size không bắt được).
    fn filetime_set(p: &Path, like: &std::fs::Metadata) -> bool {
        let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
        f.set_modified(like.modified().unwrap()).is_ok()
    }
}
