//! Trùng lặp và phiên bản: chỉ phát hiện và báo (tool, không gatekeeper) — không xoá, không di chuyển file nào.
use std::collections::{BTreeMap, HashMap, HashSet};

use fontatlas_index::FaceKey as IxKey;
use fontatlas_store::{DupRow, FaceKey, FaceQuery, SortKey};

use crate::dto::analysis::*;
use crate::dto::library::*;
use crate::error::CmdError;
use crate::index_bundle::{FaceMeta, ix_key};
use crate::services::analysis::bundle;
use crate::state::AppState;

/// Một thành viên trước khi gắn `FaceRowDto`.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub face_id: i64,
    pub key: FaceKey,
    pub path: Option<String>,
    pub format: String,
    pub version: Option<String>,
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

/// Thông tin phiên bản của một face (từ phân tích; thiếu với phân tích cũ).
#[derive(Debug, Clone, Default)]
pub struct VersionInfo {
    pub version: Option<String>,
    pub revision: Option<f32>,
    pub modified: Option<i64>,
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

/// Thuần: nhóm các dòng (face × vị trí) thành nhóm trùng lặp. `info` tra phiên bản theo (hash, face_index).
pub fn group_rows(rows: &[DupRow], info: &dyn Fn(&FaceKey) -> VersionInfo) -> Vec<RawGroup> {
    let mut groups = Vec::new();

    // (a) cùng nội dung ở nhiều đường dẫn
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
            let members = paths
                .iter()
                .map(|r| {
                    let i = info(&r.key);
                    Member { face_id: r.face_id, key: r.key, path: Some(r.path.clone()), format: format_of(&r.path, r.container.as_deref()), version: i.version, revision: i.revision, modified: i.modified, is_newest: false }
                })
                .collect();
            groups.push(RawGroup { kind: DuplicateKindDto::SameFile, key: hex(hash), members, newest_known: false });
        }
    }

    // một đại diện cho mỗi (hash, face_index): đường dẫn đầu tiên
    let mut primary: BTreeMap<([u8; 32], u32), &DupRow> = BTreeMap::new();
    for r in rows {
        primary.entry((r.key.hash, r.key.face_index)).and_modify(|cur| {
            if r.path < cur.path {
                *cur = r;
            }
        }).or_insert(r);
    }
    let member_of = |r: &DupRow| {
        let i = info(&r.key);
        Member { face_id: r.face_id, key: r.key, path: Some(r.path.clone()), format: format_of(&r.path, r.container.as_deref()), version: i.version, revision: i.revision, modified: i.modified, is_newest: false }
    };

    // (b) cùng PostScript name, khác nội dung
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
        let mut g = RawGroup { kind, key: ps, members, newest_known: false };
        if kind == DuplicateKindDto::Versions {
            mark_newest(&mut g);
        }
        for r in &rs {
            grouped.insert((r.key.hash, r.key.face_index));
        }
        groups.push(g);
    }

    // (c) cùng family+style khác định dạng (tên PostScript khác nhau, vd TTF vs OTF)
    let mut by_style: BTreeMap<String, Vec<&DupRow>> = BTreeMap::new();
    for r in primary.values() {
        if grouped.contains(&(r.key.hash, r.key.face_index)) {
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

/// Bản mới nhất = lớn nhất theo (fontRevision, head.modified); chỉ đánh dấu khi duy nhất và có ít nhất một số liệu.
fn mark_newest(g: &mut RawGroup) {
    let score = |m: &Member| (m.revision.map(|r| (r * 1000.0).round() as i64), m.modified);
    let best = g.members.iter().map(score).max();
    let Some(best) = best else { return };
    if best == (None, None) {
        return;
    }
    let tops: Vec<usize> = g.members.iter().enumerate().filter(|(_, m)| score(m) == best).map(|(i, _)| i).collect();
    if tops.len() == 1 {
        g.members[tops[0]].is_newest = true;
        g.newest_known = true;
    }
}

fn info_of(meta: &HashMap<IxKey, FaceMeta>, k: &FaceKey) -> VersionInfo {
    meta.get(&ix_key(k)).map(|m| VersionInfo { version: m.version.clone(), revision: m.revision, modified: m.modified_ts }).unwrap_or_default()
}

fn groups_for(state: &AppState, q: &FaceQueryDto) -> Result<Vec<RawGroup>, CmdError> {
    let b = bundle(state)?;
    let rows = state.with_store(|s| Ok(s.duplicate_rows()?))?;
    let mut groups = group_rows(&rows, &|k| info_of(&b.meta, k));
    // chỉ giữ nhóm có ít nhất một thành viên khớp bộ lọc hiện tại
    let filtered = q.text.is_some()
        || q.vi.is_some()
        || !q.license.is_empty()
        || q.variable.is_some()
        || q.italic.is_some()
        || !q.source_ids.is_empty()
        || q.favorites_only
        || q.collection_id.is_some()
        || !q.tag_ids.is_empty()
        || !q.classes.is_empty()
        || !q.subclasses.is_empty()
        || !q.traits.is_empty()
        || !q.axis_filters.is_empty()
        || !q.raw_axis_ranges.is_empty()
        || !q.hard.is_empty()
        || q.face_keys.is_some();
    if filtered {
        let allowed = crate::services::library::matching_keys(state, q)?;
        groups.retain(|g| g.members.iter().any(|m| allowed.contains(&ix_key(&m.key))));
    }
    Ok(groups)
}

pub fn duplicate_summary(state: &AppState, q: &FaceQueryDto) -> Result<DuplicateSummaryDto, CmdError> {
    let mut s = DuplicateSummaryDto::default();
    for g in groups_for(state, q)? {
        match g.kind {
            DuplicateKindDto::SameFile => s.same_file += 1,
            DuplicateKindDto::Versions => s.versions += 1,
            DuplicateKindDto::Formats => s.formats += 1,
        }
    }
    s.total = s.same_file + s.versions + s.formats;
    Ok(s)
}

fn rows_by_id(state: &AppState, ids: &[i64]) -> Result<HashMap<i64, FaceRowDto>, CmdError> {
    let mut out = HashMap::new();
    for chunk in ids.chunks(1000) {
        let scope: Vec<(i64, f32)> = chunk.iter().map(|id| (*id, 1.0)).collect();
        let q = FaceQuery { include_missing: true, sort: SortKey::Match, limit: 1000, ..Default::default() };
        let page = state.with_store(|s| Ok(s.query_faces_scoped(&q, Some(&scope))?))?;
        for r in page.items {
            out.insert(r.face_id, FaceRowDto::from(r));
        }
    }
    Ok(out)
}

pub fn duplicate_groups(state: &AppState, q: &FaceQueryDto) -> Result<Vec<DuplicateGroupDto>, CmdError> {
    let groups = groups_for(state, q)?;
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

    fn no_info(_: &FaceKey) -> VersionInfo {
        VersionInfo::default()
    }

    #[test]
    fn same_file_lists_every_path_and_ignores_single_copies() {
        let rows = vec![row(1, 1, "A-Regular", "A", "Regular", "/x/a.ttf"), row(1, 1, "A-Regular", "A", "Regular", "/y/a.ttf"), row(2, 2, "B-Regular", "B", "Regular", "/x/b.ttf")];
        let g = group_rows(&rows, &no_info);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::SameFile);
        assert_eq!(g[0].members.iter().filter_map(|m| m.path.as_deref()).collect::<Vec<_>>(), vec!["/x/a.ttf", "/y/a.ttf"]);
    }

    #[test]
    fn versions_mark_newest_by_revision_then_modified_only_when_unique() {
        let rows = vec![row(1, 1, "A-Regular", "A", "Regular", "/old/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/new/a.ttf")];
        let info = |k: &FaceKey| match k.hash[0] {
            1 => VersionInfo { version: Some("Version 1.0".into()), revision: Some(1.0), modified: Some(2_000) },
            _ => VersionInfo { version: Some("Version 2.0".into()), revision: Some(2.0), modified: Some(1_000) },
        };
        let g = group_rows(&rows, &info);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Versions);
        assert!(g[0].newest_known);
        // revision thắng ngày sửa
        assert_eq!(g[0].members.iter().filter(|m| m.is_newest).map(|m| m.key.hash[0]).collect::<Vec<_>>(), vec![2]);
        // cùng revision, khác ngày → ngày quyết định
        let same_rev = |k: &FaceKey| VersionInfo { revision: Some(1.5), modified: Some(i64::from(k.hash[0])), ..Default::default() };
        let g = group_rows(&rows, &same_rev);
        assert_eq!(g[0].members.iter().filter(|m| m.is_newest).map(|m| m.key.hash[0]).collect::<Vec<_>>(), vec![2]);
        // hoà hoàn toàn hoặc không có số liệu → không đánh dấu ai
        let tie = |_: &FaceKey| VersionInfo { revision: Some(1.5), modified: Some(5), ..Default::default() };
        let g = group_rows(&rows, &tie);
        assert!(!g[0].newest_known && g[0].members.iter().all(|m| !m.is_newest));
        let g = group_rows(&rows, &no_info);
        assert!(!g[0].newest_known && g[0].members.iter().all(|m| !m.is_newest));
    }

    #[test]
    fn formats_when_each_member_has_its_own_format() {
        // cùng PostScript name, hai định dạng → formats
        let g = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/a.otf")], &no_info);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Formats);
        assert!(g[0].members.iter().all(|m| !m.is_newest) && !g[0].newest_known);
        // PostScript name khác nhau nhưng cùng family+style+weight → vẫn formats
        let g = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-Reg", "A", "Regular", "/a.woff2")], &no_info);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].kind, DuplicateKindDto::Formats);
        assert_eq!(g[0].members.iter().map(|m| m.format.as_str()).collect::<Vec<_>>(), vec!["ttf", "woff2"]);
    }

    #[test]
    fn same_format_with_different_postscript_is_not_a_duplicate() {
        // static vs variable cùng family+style, cùng .ttf: không báo trùng
        let g = group_rows(&[row(1, 1, "A-Regular", "A", "Regular", "/a.ttf"), row(2, 2, "A-VF", "A", "Regular", "/a-vf.ttf")], &no_info);
        assert!(g.is_empty(), "{g:?}");
    }

    #[test]
    fn a_face_is_in_at_most_one_name_based_group() {
        // 3 bản cùng PS name: 2 TTF + 1 WOFF2 → một nhóm versions (không bị nhóm lại ở pass family/style)
        let g = group_rows(
            &[row(1, 1, "A-Regular", "A", "Regular", "/1/a.ttf"), row(2, 2, "A-Regular", "A", "Regular", "/2/a.ttf"), row(3, 3, "A-Regular", "A", "Regular", "/3/a.woff2")],
            &no_info,
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
