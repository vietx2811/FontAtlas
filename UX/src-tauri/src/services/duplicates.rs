//! Trùng lặp và phiên bản: chỉ phát hiện và báo (tool, không gatekeeper) — không xoá, không di chuyển file nào.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use fontatlas_index::FaceKey as IxKey;
use fontatlas_store::{DupRow, FaceKey, FaceQuery, MAX_LIMIT, SortKey};

use crate::dto::analysis::*;
use crate::dto::library::*;
use crate::error::CmdError;
use crate::index_bundle::{FaceMeta, IndexBundle, ix_key};
use crate::services::analysis::bundle;
use crate::state::AppState;

/// Số face mỗi lần lấy hàng từ Store (không vượt trần của Store).
const CHUNK: usize = MAX_LIMIT as usize;

/// Một thành viên trước khi gắn `FaceRowDto`.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub face_id: i64,
    pub key: FaceKey,
    pub path: Option<String>,
    pub format: String,
    pub version: Option<String>,
    /// Chỉ điền cho nhóm `versions` (đọc lười từ bảng head).
    pub revision: Option<f32>,
    pub modified: Option<i64>,
    pub is_newest: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawGroup {
    pub kind: DuplicateKindDto,
    pub key: String,
    pub members: Vec<Member>,
    pub newest_known: bool,
}

pub fn format_of(path: &str, container: Option<&str>) -> String {
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        let e = ext.to_ascii_lowercase();
        if ["ttf", "otf", "ttc", "otc", "woff", "woff2"].contains(&e.as_str()) {
            return e;
        }
    }
    match container {
        Some("woff1") => "woff",
        Some("woff2") => "woff2",
        Some("collection") => "ttc",
        _ => "sfnt",
    }
    .into()
}

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Thuần: nhóm các dòng (face × vị trí) thành nhóm trùng lặp, chưa có số liệu head.
/// `version` tra chuỗi phiên bản (name ID 5) theo dòng.
pub fn group_rows(rows: &[DupRow], version: &dyn Fn(&DupRow) -> Option<String>) -> Vec<RawGroup> {
    let mut groups = Vec::new();
    let member_of = |r: &DupRow| Member {
        face_id: r.face_id,
        key: r.key,
        path: Some(r.path.clone()),
        format: format_of(&r.path, r.container.as_deref()),
        version: version(r),
        revision: None,
        modified: None,
        is_newest: false,
    };

    // (a) cùng nội dung ở nhiều đường dẫn (TTC: theo face đầu tiên của tệp)
    let mut by_file: BTreeMap<[u8; 32], Vec<&DupRow>> = BTreeMap::new();
    for r in rows {
        by_file.entry(r.key.hash).or_default().push(r);
    }
    for (hash, rs) in &by_file {
        let first_face = rs.iter().map(|r| r.key.face_index).min().unwrap_or(0);
        let mut paths: Vec<&DupRow> = rs.iter().copied().filter(|r| r.key.face_index == first_face).collect();
        paths.sort_by(|a, b| a.path.cmp(&b.path));
        paths.dedup_by(|a, b| a.path == b.path);
        if paths.len() >= 2 {
            groups.push(RawGroup { kind: DuplicateKindDto::SameFile, key: hex(hash), members: paths.iter().map(|r| member_of(r)).collect(), newest_known: false });
        }
    }

    // một đại diện cho mỗi (hash, face_index): đường dẫn nhỏ nhất
    let mut primary: BTreeMap<([u8; 32], u32), &DupRow> = BTreeMap::new();
    for r in rows {
        primary.entry((r.key.hash, r.key.face_index)).and_modify(|cur| {
            if r.path < cur.path {
                *cur = r;
            }
        }).or_insert(r);
    }

    // (b) cùng PostScript name (chuẩn hoá hoa/thường, bỏ trống), khác nội dung
    let mut by_ps: BTreeMap<String, Vec<&DupRow>> = BTreeMap::new();
    for r in primary.values() {
        if let Some(ps) = r.postscript.as_deref().map(norm).filter(|s| !s.is_empty()) {
            by_ps.entry(ps).or_default().push(r);
        }
    }
    let mut grouped: HashSet<([u8; 32], u32)> = HashSet::new();
    for (ps, rs) in by_ps {
        if rs.len() < 2 {
            continue;
        }
        let mut members: Vec<Member> = rs.iter().map(|r| member_of(r)).collect();
        members.sort_by_key(|m| m.face_id);
        let formats: HashSet<&str> = members.iter().map(|m| m.format.as_str()).collect();
        let kind = if formats.len() == members.len() { DuplicateKindDto::Formats } else { DuplicateKindDto::Versions };
        for r in &rs {
            grouped.insert((r.key.hash, r.key.face_index));
        }
        groups.push(RawGroup { kind, key: ps, members, newest_known: false });
    }

    // (c) cùng family+style khác định dạng (tên PostScript khác nhau, vd TTF vs OTF); bỏ qua family rỗng
    let mut by_style: BTreeMap<String, Vec<&DupRow>> = BTreeMap::new();
    for r in primary.values() {
        if grouped.contains(&(r.key.hash, r.key.face_index)) || norm(&r.family).is_empty() {
            continue;
        }
        let k = format!("{}|{}|{}|{}", norm(&r.family), norm(r.subfamily.as_deref().unwrap_or("")), r.weight.unwrap_or(0), u8::from(r.italic));
        by_style.entry(k).or_default().push(r);
    }
    for (k, rs) in by_style {
        if rs.len() < 2 {
            continue;
        }
        let mut members: Vec<Member> = rs.iter().map(|r| member_of(r)).collect();
        let formats: HashSet<&str> = members.iter().map(|m| m.format.as_str()).collect();
        // chỉ khi mỗi thành viên một định dạng riêng (nhiều bản cùng định dạng có thể là static vs variable, không phải trùng)
        if formats.len() != members.len() {
            continue;
        }
        members.sort_by(|a, b| a.format.cmp(&b.format));
        groups.push(RawGroup { kind: DuplicateKindDto::Formats, key: k, members, newest_known: false });
    }

    groups.sort_by(|a, b| (kind_rank(a.kind), &a.key).cmp(&(kind_rank(b.kind), &b.key)));
    groups
}

fn kind_rank(k: DuplicateKindDto) -> u8 {
    match k {
        DuplicateKindDto::SameFile => 0,
        DuplicateKindDto::Versions => 1,
        DuplicateKindDto::Formats => 2,
    }
}

/// (fontRevision, head.modified) đọc được từ tệp.
type Heads = (Option<f32>, Option<i64>);

/// Điền (revision, modified) cho nhóm `versions` rồi đánh dấu bản mới nhất. `head` trả (None, None) khi không đọc được.
pub fn apply_heads(g: &mut RawGroup, head: &dyn Fn(&Member) -> Heads) {
    if g.kind != DuplicateKindDto::Versions {
        return;
    }
    for m in &mut g.members {
        (m.revision, m.modified) = head(m);
    }
    mark_newest(g);
}

/// Bản mới nhất = lớn nhất theo (fontRevision, head.modified). Chỉ đánh dấu khi MỌI thành viên có đủ dữ liệu so sánh
/// (không để bản đọc được head thắng bản không đọc được) và chỉ có một bản lớn nhất.
fn mark_newest(g: &mut RawGroup) {
    for m in &mut g.members {
        m.is_newest = false;
    }
    g.newest_known = false;
    let all_rev = g.members.iter().all(|m| m.revision.is_some());
    let all_mod = g.members.iter().all(|m| m.modified.is_some());
    let score = |m: &Member| -> Option<(Option<i64>, Option<i64>)> {
        if all_rev {
            Some((m.revision.map(|r| (r * 1000.0).round() as i64), all_mod.then_some(m.modified).flatten()))
        } else if all_mod {
            Some((None, m.modified))
        } else {
            None
        }
    };
    let scores: Vec<_> = g.members.iter().map(score).collect();
    if scores.iter().any(Option::is_none) {
        return;
    }
    let Some(best) = scores.iter().max().cloned().flatten() else { return };
    let tops: Vec<usize> = scores.iter().enumerate().filter(|(_, s)| **s == Some(best)).map(|(i, _)| i).collect();
    if tops.len() == 1 {
        g.members[tops[0]].is_newest = true;
        g.newest_known = true;
    }
}

/// Số lần thực sự mở tệp để đọc head (cho test cache; không dùng ở logic).
static HEAD_READS: AtomicUsize = AtomicUsize::new(0);

pub fn head_reads() -> usize {
    HEAD_READS.load(Ordering::SeqCst)
}

/// head.fontRevision / head.modified đọc từ tệp khi cần, nhớ theo (hash, face): nội dung theo hash là bất biến nên cache
/// không bao giờ cũ. Tệp mất / iCloud chưa tải (không đọc nội dung để khỏi kích hoạt tải ngầm) / không đọc được thì
/// KHÔNG nhớ và trả (None, None).
type HeadCache = Mutex<HashMap<([u8; 32], u32), Heads>>;
static HEAD_CACHE: OnceLock<HeadCache> = OnceLock::new();

fn head_of(key: &FaceKey, path: &str) -> Heads {
    let cache = HEAD_CACHE.get_or_init(Default::default);
    let k = (key.hash, key.face_index);
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(&k).copied()) {
        return v;
    }
    let p = std::path::Path::new(path);
    if fontatlas_activate::probe_file(p) != fontatlas_activate::FileProbe::Ready {
        return (None, None);
    }
    HEAD_READS.fetch_add(1, Ordering::SeqCst);
    match fontatlas_core::extras::head_info(p, key.face_index) {
        Some(h) => {
            let v = (h.revision, h.modified);
            if let Ok(mut c) = cache.lock() {
                c.insert(k, v);
            }
            v
        }
        None => (None, None),
    }
}

/// Nhóm cơ sở (chưa có head) nhớ theo Index hiện tại: dựng lại khi Index được dựng lại (sau scan/ingest/embed). Summary
/// và groups dùng chung.
type BaseCache = Mutex<Option<(Weak<IndexBundle>, Arc<Vec<RawGroup>>)>>;
static BASE_CACHE: OnceLock<BaseCache> = OnceLock::new();

fn base_groups(state: &AppState, b: &Arc<IndexBundle>) -> Result<Arc<Vec<RawGroup>>, CmdError> {
    let cache = BASE_CACHE.get_or_init(Default::default);
    if let Ok(g) = cache.lock()
        && let Some((w, groups)) = g.as_ref()
        && w.upgrade().is_some_and(|a| Arc::ptr_eq(&a, b))
    {
        return Ok(groups.clone());
    }
    let mut rows = state.with_store(|s| Ok(s.duplicate_rows()?))?;
    // bản CHÉP trong Organized Library (B7) không phải "trùng lặp" của file gốc: bỏ khỏi phân nhóm
    if let Some(prefix) = crate::services::organized::library_prefix(state) {
        rows.retain(|r| !r.path.strip_prefix(&prefix).is_some_and(|rest| rest.starts_with('/')));
    }
    // bản sao đổi tên của VF (G7b) cùng họ với bản gốc nhưng cố ý tách riêng: không phải "phiên bản trùng"
    if let Ok(derived) = state.with_store(|s| Ok(s.derived_hashes()?))
        && !derived.is_empty()
    {
        rows.retain(|r| !derived.contains(&r.key.hash));
    }
    let meta: &HashMap<IxKey, FaceMeta> = &b.meta;
    let groups = Arc::new(group_rows(&rows, &|r| meta.get(&ix_key(&r.key)).and_then(|m| m.version.clone())));
    if let Ok(mut g) = cache.lock() {
        *g = Some((Arc::downgrade(b), groups.clone()));
    }
    Ok(groups)
}

/// `q` có bộ lọc nào ngoài mặc định không (bỏ qua sắp xếp/phân trang): so với `FaceQueryDto::default()`.
fn is_filtered(q: &FaceQueryDto) -> bool {
    let mut base = q.clone();
    let d = FaceQueryDto::default();
    base.sort = d.sort;
    base.descending = d.descending;
    base.offset = d.offset;
    base.limit = d.limit;
    base.include_missing = d.include_missing;
    serde_json::to_value(&base).ok() != serde_json::to_value(&d).ok()
}

/// Nhóm hiện có sau bộ lọc, TRỪ các nhóm người dùng đã bỏ qua.
fn filtered_groups(state: &AppState, q: &FaceQueryDto) -> Result<Vec<RawGroup>, CmdError> {
    let mut groups = filtered_groups_raw(state, q)?;
    let ignored: HashSet<(String, String)> = state.with_store(|s| Ok(s.ignored_groups()?))?.into_iter().map(|(k, key, _)| (k, key)).collect();
    if !ignored.is_empty() {
        groups.retain(|g| !ignored.contains(&(kind_name(g.kind).to_string(), g.key.clone())));
    }
    Ok(groups)
}

pub(crate) fn kind_name(k: DuplicateKindDto) -> &'static str {
    match k {
        DuplicateKindDto::SameFile => "same_file",
        DuplicateKindDto::Versions => "versions",
        DuplicateKindDto::Formats => "formats",
    }
}

fn filtered_groups_raw(state: &AppState, q: &FaceQueryDto) -> Result<Vec<RawGroup>, CmdError> {
    let b = bundle(state)?;
    let mut groups: Vec<RawGroup> = base_groups(state, &b)?.as_ref().clone();
    // chỉ giữ nhóm có ít nhất một thành viên khớp bộ lọc hiện tại
    if is_filtered(q) {
        let allowed = crate::services::library::matching_keys(state, q)?;
        groups.retain(|g| g.members.iter().any(|m| allowed.contains(&ix_key(&m.key))));
    }
    Ok(groups)
}

/// Sau khi giải quyết / hoàn tác: nhóm cơ sở phải dựng lại (Index không đổi nên cache theo Index không tự mất).
pub fn invalidate_cache() {
    if let Some(c) = BASE_CACHE.get()
        && let Ok(mut g) = c.lock()
    {
        *g = None;
    }
}

/// Mọi nhóm hiện có (không lọc), đã điền head cho nhóm `versions`. Dùng cho giải quyết trùng lặp.
pub(crate) fn current_groups(state: &AppState, q: Option<&FaceQueryDto>) -> Result<Vec<RawGroup>, CmdError> {
    let mut groups: Vec<RawGroup> = match q {
        Some(q) => filtered_groups(state, q)?,
        None => base_groups(state, &bundle(state)?)?.as_ref().clone(),
    };
    for g in &mut groups {
        apply_heads(g, &|m| m.path.as_deref().map_or((None, None), |p| head_of(&m.key, p)));
    }
    Ok(groups)
}

/// Chỉ đếm — KHÔNG đọc head hay mở tệp nào.
pub fn duplicate_summary(state: &AppState, q: &FaceQueryDto) -> Result<DuplicateSummaryDto, CmdError> {
    let mut s = DuplicateSummaryDto::default();
    for g in filtered_groups(state, q)? {
        match g.kind {
            DuplicateKindDto::SameFile => s.same_file += 1,
            DuplicateKindDto::Versions => s.versions += 1,
            DuplicateKindDto::Formats => s.formats += 1,
        }
    }
    s.total = s.same_file + s.versions + s.formats;
    Ok(s)
}

pub(crate) fn rows_by_id(state: &AppState, ids: &[i64]) -> Result<HashMap<i64, FaceRowDto>, CmdError> {
    let mut out = HashMap::new();
    for chunk in ids.chunks(CHUNK) {
        let scope: Vec<(i64, f32)> = chunk.iter().map(|id| (*id, 1.0)).collect();
        let q = FaceQuery { include_missing: true, include_hidden: true, sort: SortKey::Match, limit: MAX_LIMIT, ..Default::default() };
        let page = state.with_store(|s| Ok(s.query_faces_scoped(&q, Some(&scope))?))?;
        for r in page.items {
            out.insert(r.face_id, FaceRowDto::from(r));
        }
    }
    Ok(out)
}

pub fn duplicate_groups(state: &AppState, q: &FaceQueryDto) -> Result<Vec<DuplicateGroupDto>, CmdError> {
    group_dtos(state, filtered_groups(state, q)?)
}

/// Bỏ qua nhóm này: ghi nhớ, nhóm biến mất khỏi danh sách, đếm và "Giải quyết tất cả". Hoàn tác bằng `duplicate_unignore`.
pub fn duplicate_ignore(state: &AppState, kind: DuplicateKindDto, key: &str, now: i64) -> Result<(), CmdError> {
    state.with_store(|s| Ok(s.ignore_group(kind_name(kind), key, now)?))
}

pub fn duplicate_unignore(state: &AppState, kind: DuplicateKindDto, key: &str) -> Result<(), CmdError> {
    state.with_store(|s| Ok(s.unignore_group(kind_name(kind), key)?)).map(|_| ())
}

/// Các nhóm đã bỏ qua mà vẫn còn là nhóm trùng (nhóm đã hết trùng thì không hiện), mới bỏ qua trước.
pub fn duplicate_ignored(state: &AppState) -> Result<Vec<IgnoredGroupDto>, CmdError> {
    let ignored = state.with_store(|s| Ok(s.ignored_groups()?))?;
    if ignored.is_empty() {
        return Ok(vec![]);
    }
    let all = filtered_groups_raw(state, &FaceQueryDto::default())?;
    let mut out = Vec::new();
    for (kind, key, at) in ignored {
        if let Some(g) = all.iter().find(|g| kind_name(g.kind) == kind && g.key == key).cloned()
            && let Some(dto) = group_dtos(state, vec![g])?.into_iter().next()
        {
            out.push(IgnoredGroupDto { group: dto, ignored_at: at });
        }
    }
    Ok(out)
}

fn group_dtos(state: &AppState, mut groups: Vec<RawGroup>) -> Result<Vec<DuplicateGroupDto>, CmdError> {
    // chỉ nhóm `versions` cần head (để biết bản mới nhất)
    for g in &mut groups {
        apply_heads(g, &|m| m.path.as_deref().map_or((None, None), |p| head_of(&m.key, p)));
    }
    let ids: Vec<i64> = groups.iter().flat_map(|g| g.members.iter().map(|m| m.face_id)).collect::<HashSet<_>>().into_iter().collect();
    let rows = rows_by_id(state, &ids)?;
    Ok(groups
        .into_iter()
        .filter_map(|g| {
            let faces: Vec<DuplicateMemberDto> = g
                .members
                .into_iter()
                .filter_map(|m| {
                    Some(DuplicateMemberDto { row: rows.get(&m.face_id)?.clone(), path: m.path, format: m.format, version: m.version, revision: m.revision, modified: m.modified, is_newest: m.is_newest })
                })
                .collect();
            (faces.len() >= 2).then_some(DuplicateGroupDto { kind: g.kind, key: g.key, faces, newest_known: g.newest_known })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(face_id: i64, n: u8, ps: &str, family: &str, style: &str, path: &str) -> DupRow {
        DupRow { face_id, key: FaceKey { hash: [n; 32], face_index: 0 }, family: family.into(), subfamily: Some(style.into()), postscript: Some(ps.into()), weight: Some(400), italic: false, container: Some("sfnt".into()), path: path.into() }
    }

    fn no_ver(_: &DupRow) -> Option<String> {
        None
    }

    #[test]
    fn same_file_lists_every_path_and_ignores_single_copies() {
        let rows = vec![row(1, 1, "A-Regular", "A", "Regular", "/x/a.ttf"), row(1, 1, "A-Regular", "A", "Regular", "/y/a.ttf"), row(2, 2, "B-Regular", "B", "Regular", "/x/b.ttf")];
        let g = group_rows(&rows, &no_ver);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::SameFile);
        assert_eq!(g[0].members.iter().filter_map(|m| m.path.as_deref()).collect::<Vec<_>>(), vec!["/x/a.ttf", "/y/a.ttf"]);
    }

    fn versions_group() -> RawGroup {
        let rows = vec![row(1, 1, "A-Regular", "A", "Regular", "/old/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/new/a.ttf")];
        let mut g = group_rows(&rows, &no_ver);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Versions);
        // group_rows không tự điền head
        assert!(g[0].members.iter().all(|m| m.revision.is_none() && m.modified.is_none() && !m.is_newest));
        g.remove(0)
    }

    fn newest(g: &RawGroup) -> Vec<u8> {
        g.members.iter().filter(|m| m.is_newest).map(|m| m.key.hash[0]).collect()
    }

    #[test]
    fn versions_mark_newest_by_revision_then_modified_only_when_unique_and_complete() {
        let by = |f: fn(u8) -> (Option<f32>, Option<i64>)| {
            let mut g = versions_group();
            apply_heads(&mut g, &|m| f(m.key.hash[0]));
            g
        };
        // revision thắng ngày sửa
        let g = by(|n| if n == 1 { (Some(1.0), Some(2_000)) } else { (Some(2.0), Some(1_000)) });
        assert!(g.newest_known);
        assert_eq!(newest(&g), vec![2]);
        // cùng revision, khác ngày → ngày quyết định
        let g = by(|n| (Some(1.5), Some(i64::from(n))));
        assert_eq!(newest(&g), vec![2]);
        // hoà hoàn toàn → không ai
        let g = by(|_| (Some(1.5), Some(5)));
        assert!(!g.newest_known && newest(&g).is_empty());
        // cùng revision, chỉ một bên có ngày → không đủ để so → không ai
        let g = by(|n| (Some(1.5), (n == 2).then_some(9)));
        assert!(!g.newest_known && newest(&g).is_empty());
        // không có số liệu → không ai
        let g = by(|_| (None, None));
        assert!(!g.newest_known && newest(&g).is_empty());
        // bản đọc được head KHÔNG thắng mặc định bản không đọc được (tệp mất/iCloud)
        let g = by(|n| if n == 1 { (Some(1.0), Some(100)) } else { (None, None) });
        assert!(!g.newest_known && newest(&g).is_empty());
        // đủ ngày nhưng thiếu revision ở một bên → so theo ngày
        let g = by(|n| (None, Some(i64::from(n) * 10)));
        assert!(g.newest_known);
        assert_eq!(newest(&g), vec![2]);
    }

    #[test]
    fn heads_are_only_applied_to_versions_groups() {
        let mut g = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/a.otf")], &no_ver).remove(0);
        assert_eq!(g.kind, DuplicateKindDto::Formats);
        let called = std::cell::Cell::new(0);
        apply_heads(&mut g, &|_| {
            called.set(called.get() + 1);
            (Some(1.0), Some(1))
        });
        assert_eq!(called.get(), 0, "formats không đọc head");
        let mut s = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/x/a.ttf"), row(1, 1, "A-Regular", "A", "Regular", "/y/a.ttf")], &no_ver).remove(0);
        assert_eq!(s.kind, DuplicateKindDto::SameFile);
        apply_heads(&mut s, &|_| {
            called.set(called.get() + 1);
            (Some(1.0), Some(1))
        });
        assert_eq!(called.get(), 0, "same_file không đọc head");
        assert!(s.members.iter().all(|m| m.revision.is_none()));
    }

    #[test]
    fn empty_postscript_is_never_grouped_and_names_are_normalized() {
        let mut a = row(1, 1, "", "A", "Regular", "/1/a.ttf");
        let mut b = row(2, 2, "  ", "B", "Regular", "/2/b.ttf");
        a.postscript = Some(String::new());
        b.postscript = None;
        assert!(group_rows(&[a, b], &no_ver).is_empty());
        // hoa/thường + khoảng trắng: vẫn cùng nhóm; family khác nhau không cản (PostScript name quyết định)
        let g = group_rows(&[row(1, 1, "Foo-Regular", "Foo", "Regular", "/1/a.ttf"), row(2, 2, "  foo-REGULAR ", "Fooo", "Regular", "/2/a.ttf")], &no_ver);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Versions);
        assert_eq!(g[0].key, "foo-regular");
    }

    #[test]
    fn empty_family_is_skipped_in_the_family_style_pass() {
        let mut a = row(1, 1, "X1", "", "Regular", "/a.ttf");
        let mut b = row(2, 2, "X2", "  ", "Regular", "/a.otf");
        a.postscript = None;
        b.postscript = None;
        assert!(group_rows(&[a, b], &no_ver).is_empty());
    }

    #[test]
    fn three_formats_form_one_formats_group() {
        let g = group_rows(
            &[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/a.otf"), row(3, 3, "A-Regular", "A", "Regular", "/a.woff2")],
            &no_ver,
        );
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Formats);
        assert_eq!(g[0].members.len(), 3);
    }

    #[test]
    fn ttc_same_file_is_one_group_listing_each_path_once() {
        // TTC 2 face × 2 đường dẫn → một nhóm same_file, mỗi đường dẫn một lần (theo face đầu)
        let mut rows = Vec::new();
        for (fi, id) in [(0u32, 1i64), (1, 2)] {
            for path in ["/x/f.ttc", "/y/f.ttc"] {
                let mut r = row(id, 7, &format!("F-{fi}"), "F", "Regular", path);
                r.key.face_index = fi;
                r.container = Some("collection".into());
                rows.push(r);
            }
        }
        let g = group_rows(&rows, &no_ver);
        let same: Vec<_> = g.iter().filter(|x| x.kind == DuplicateKindDto::SameFile).collect();
        assert_eq!(same.len(), 1);
        assert_eq!(same[0].members.len(), 2);
        assert!(same[0].members.iter().all(|m| m.key.face_index == 0 && m.format == "ttc"));
    }

    #[test]
    fn default_query_with_paging_only_is_not_filtered() {
        assert!(!is_filtered(&FaceQueryDto::default()));
        assert!(!is_filtered(&FaceQueryDto { limit: 50, offset: 100, descending: true, ..Default::default() }));
        assert!(is_filtered(&FaceQueryDto { text: Some("x".into()), ..Default::default() }));
        assert!(is_filtered(&FaceQueryDto { traits: vec!["stencil".into()], ..Default::default() }));
        assert!(is_filtered(&FaceQueryDto { favorites_only: true, ..Default::default() }));
    }

    #[test]
    fn formats_when_each_member_has_its_own_format() {
        // cùng PostScript name, hai định dạng → formats
        let g = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/a.otf")], &no_ver);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Formats);
        assert!(g[0].members.iter().all(|m| !m.is_newest) && !g[0].newest_known);
        // PostScript name khác nhau nhưng cùng family+style+weight → vẫn formats
        let g = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-Reg", "A", "Regular", "/a.woff2")], &no_ver);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Formats);
        assert_eq!(g[0].members.iter().map(|m| m.format.as_str()).collect::<Vec<_>>(), vec!["ttf", "woff2"]);
    }

    #[test]
    fn same_format_with_different_postscript_is_not_a_duplicate() {
        // static vs variable cùng family+style, cùng .ttf: không báo trùng
        let g = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-VF", "A", "Regular", "/a-vf.ttf")], &no_ver);
        assert!(g.is_empty(), "{g:?}");
    }

    #[test]
    fn a_face_is_in_at_most_one_name_based_group() {
        // 3 bản cùng PS name: 2 TTF + 1 WOFF2 → một nhóm versions (không bị nhóm lại ở pass family/style)
        let g = group_rows(
            &[row(1, 1, "A-Regular", "A", "Regular", "/1/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/2/a.ttf"), row(3, 3, "A-Regular", "A", "Regular", "/3/a.woff2")],
            &no_ver,
        );
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Versions);
        assert_eq!(g[0].members.len(), 3);
    }

    #[test]
    fn format_detection_prefers_extension_then_container() {
        assert_eq!(format_of("/x/A.WOFF2", Some("sfnt")), "woff2");
        assert_eq!(format_of("/x/A", Some("woff1")), "woff");
        assert_eq!(format_of("/x/A", Some("collection")), "ttc");
        assert_eq!(format_of("/x/A.bin", None), "sfnt");
    }
}
