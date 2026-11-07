use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::analysis::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{analysis, boot, duplicates, library, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

/// Thay chuỗi cùng độ dài ở cả dạng ASCII/Mac và UTF-16BE (bảng `name`).
fn patch_name(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let u16be = |s: &str| s.encode_utf16().flat_map(|c| c.to_be_bytes()).collect::<Vec<u8>>();
    let mut out = bytes.to_vec();
    for (a, b) in [(from.as_bytes().to_vec(), to.as_bytes().to_vec()), (u16be(from), u16be(to))] {
        let mut i = 0;
        while i + a.len() <= out.len() {
            if out[i..i + a.len()] == a[..] {
                out[i..i + a.len()].copy_from_slice(&b);
                i += a.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// Ghi đè head.fontRevision (Fixed 16.16).
fn patch_revision(bytes: &[u8], revision: f64) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let n = u16::from_be_bytes([out[4], out[5]]) as usize;
    for i in 0..n {
        let rec = 12 + i * 16;
        if &out[rec..rec + 4] == b"head" {
            let off = u32::from_be_bytes(out[rec + 8..rec + 12].try_into().unwrap()) as usize;
            out[off + 4..off + 8].copy_from_slice(&((revision * 65536.0).round() as u32).to_be_bytes());
            return out;
        }
    }
    panic!("không có bảng head");
}

fn state_with(root: &Path, files: &[(&str, Vec<u8>)]) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    for (rel, bytes) in files {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

fn groups(st: &AppState) -> Vec<DuplicateGroupDto> {
    duplicates::duplicate_groups(st, &FaceQueryDto::default()).unwrap()
}

#[test]
fn same_content_in_two_places_is_one_face_and_one_same_file_group() {
    let td = tempfile::tempdir().unwrap();
    let roboto = std::fs::read(fixture("roboto.ttf")).unwrap();
    let st = state_with(td.path(), &[("a/roboto.ttf", roboto.clone()), ("b/copy-of-roboto.ttf", roboto), ("c/playfair.ttf", std::fs::read(fixture("playfair_display.ttf")).unwrap())]);
    // thư viện chỉ có 2 face dù có 3 tệp
    assert_eq!(library::query_faces(&st, &FaceQueryDto { limit: 50, ..Default::default() }).unwrap().total, 2);
    let g = groups(&st);
    assert_eq!(g.len(), 1, "{g:?}");
    assert_eq!(g[0].kind, DuplicateKindDto::SameFile);
    assert_eq!(g[0].faces.len(), 2);
    let paths: Vec<&str> = g[0].faces.iter().filter_map(|f| f.path.as_deref()).collect();
    assert!(paths.iter().any(|p| p.ends_with("a/roboto.ttf")) && paths.iter().any(|p| p.ends_with("b/copy-of-roboto.ttf")), "{paths:?}");
    assert!(g[0].faces.iter().all(|f| f.row.family == "Roboto" && !f.is_newest));
    let s = duplicates::duplicate_summary(&st, &FaceQueryDto::default()).unwrap();
    assert_eq!((s.same_file, s.versions, s.formats, s.total), (1, 0, 0, 1));

    // Similar/Pairing/Compare: cùng nội dung ở đường dẫn khác không bao giờ thành "một font khác"
    let roboto_row = library::query_faces(&st, &FaceQueryDto { text: Some("Roboto".into()), limit: 10, ..Default::default() }).unwrap().items.remove(0);
    let key = FaceKeyDto { hash_hex: roboto_row.hash_hex.clone(), face_index: roboto_row.face_index };
    let sim = analysis::similar(&st, &key, &SimilarOptsDto { k: 10, ..Default::default() }).unwrap();
    assert!(sim.iter().all(|h| h.face != key), "không tự gợi ý chính nó");
    let mut seen = std::collections::HashSet::new();
    assert!(sim.iter().all(|h| seen.insert((h.face.hash_hex.clone(), h.face.face_index))), "mỗi face một lần");
    let pc = analysis::pair_candidates(&st, &key, &PairOptsDto { role: PairRoleDto::Heading, mode: None, project_vi: false, k: 50, collection_id: None, lang: LangDto::Vi }).unwrap();
    assert!(pc.iter().all(|c| c.face != key), "Pairing không gợi ý chính font ở đường dẫn khác");
    let mut seen = std::collections::HashSet::new();
    assert!(pc.iter().all(|c| seen.insert((c.face.hash_hex.clone(), c.face.face_index))));
}

#[test]
fn two_versions_are_grouped_and_newest_is_known_only_when_unique() {
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    let v_old = patch_revision(&base, 1.203);
    let v_new = patch_revision(&patch_name(&base, "Version 1.203", "Version 1.204"), 1.204);
    let st = state_with(td.path(), &[("old/playfair.ttf", v_old.clone()), ("new/playfair.ttf", v_new), ("other/roboto.ttf", std::fs::read(fixture("roboto.ttf")).unwrap())]);
    let g = groups(&st);
    assert_eq!(g.len(), 1, "{g:?}");
    assert_eq!(g[0].kind, DuplicateKindDto::Versions);
    assert!(g[0].newest_known);
    assert_eq!(g[0].faces.len(), 2);
    let newest: Vec<&DuplicateMemberDto> = g[0].faces.iter().filter(|f| f.is_newest).collect();
    assert_eq!(newest.len(), 1);
    assert_eq!(newest[0].version.as_deref(), Some("Version 1.204"));
    assert!(newest[0].path.as_deref().unwrap().ends_with("new/playfair.ttf"));
    assert!(g[0].faces.iter().all(|f| f.revision.is_some_and(|r| (r - 1.2).abs() < 0.1) && f.modified.is_some()));
    // lọc: nhóm chỉ hiện khi có thành viên khớp bộ lọc
    let hit = duplicates::duplicate_groups(&st, &FaceQueryDto { text: Some("Playfair".into()), ..Default::default() }).unwrap();
    assert_eq!(hit.len(), 1);
    let miss = duplicates::duplicate_groups(&st, &FaceQueryDto { text: Some("Roboto".into()), ..Default::default() }).unwrap();
    assert!(miss.is_empty());
    assert_eq!(duplicates::duplicate_summary(&st, &FaceQueryDto { text: Some("Roboto".into()), ..Default::default() }).unwrap().total, 0);
}

#[test]
fn identical_revision_and_date_means_no_newest() {
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("playfair_display.ttf")).unwrap();
    // chỉ đổi chuỗi phiên bản (name 5): revision và head.modified giống hệt → không xác định được bản mới
    let st = state_with(td.path(), &[("a/playfair.ttf", base.clone()), ("b/playfair.ttf", patch_name(&base, "Version 1.203", "Version 1.204"))]);
    let g = groups(&st);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].kind, DuplicateKindDto::Versions);
    assert!(!g[0].newest_known && g[0].faces.iter().all(|f| !f.is_newest));
    assert_eq!(g[0].faces.iter().filter_map(|f| f.version.clone()).collect::<std::collections::BTreeSet<_>>().len(), 2);
}

#[test]
fn same_font_in_different_formats_is_a_formats_group() {
    let td = tempfile::tempdir().unwrap();
    let base = std::fs::read(fixture("open_sans.ttf")).unwrap();
    // cùng PostScript name, nội dung khác (đổi chuỗi phiên bản), đuôi .otf: coi như định dạng khác
    let st = state_with(td.path(), &[("a/OpenSans-Regular.ttf", base.clone()), ("b/OpenSans-Regular.otf", patch_name(&base, "Version", "Vers1on"))]);
    let g = groups(&st);
    assert_eq!(g.len(), 1, "{g:?}");
    assert_eq!(g[0].kind, DuplicateKindDto::Formats);
    let mut fm: Vec<&str> = g[0].faces.iter().map(|f| f.format.as_str()).collect();
    fm.sort();
    assert_eq!(fm, ["otf", "ttf"]);
    assert!(g[0].faces.iter().all(|f| !f.is_newest));
}

#[test]
fn no_duplicates_in_a_clean_library_and_nothing_is_ever_deleted() {
    let td = tempfile::tempdir().unwrap();
    let files: Vec<(&str, Vec<u8>)> = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf"].iter().map(|n| (*n, std::fs::read(fixture(n)).unwrap())).collect();
    let st = state_with(td.path(), &files);
    assert!(groups(&st).is_empty());
    assert_eq!(duplicates::duplicate_summary(&st, &FaceQueryDto::default()).unwrap(), DuplicateSummaryDto::default());
    // chỉ đọc: các tệp vẫn còn nguyên
    for (n, b) in &files {
        assert_eq!(&std::fs::read(td.path().join("Fonts").join(n)).unwrap(), b);
    }
}
