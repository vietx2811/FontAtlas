use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::analysis::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{boot, compare, library, sources};
use app_lib::state::{AppDirs, AppState};

const SIX: [&str; 6] = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf", "public_sans_vf.ttf"];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn lib6(root: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in SIX {
        std::fs::copy(fixture(n), dir.join(n)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

fn key_of(st: &AppState, family: &str) -> FaceKeyDto {
    let q = FaceQueryDto { text: Some(family.into()), limit: 50, ..Default::default() };
    let r = library::query_faces(st, &q).unwrap().items.into_iter().find(|r| r.family == family).unwrap_or_else(|| panic!("{family}"));
    FaceKeyDto { hash_hex: r.hash_hex, face_index: r.face_index }
}

fn opts() -> CompareOptsDto {
    CompareOptsDto { lang: LangDto::Vi }
}

#[test]
fn compare_deltas_extras_and_vietnamese() {
    let td = tempfile::tempdir().unwrap();
    let st = lib6(td.path());
    let faces = vec![key_of(&st, "Roboto"), key_of(&st, "Playfair Display"), key_of(&st, "Libre Baskerville")];
    let c = compare::compare_faces(&st, &faces, &opts()).unwrap();
    assert_eq!(c.baseline, faces[0]);
    assert_eq!(c.faces.len(), 3);
    assert!(c.faces[0].is_baseline && !c.faces[1].is_baseline);
    // mốc không có delta; các cột khác có delta khớp công thức
    assert!(c.faces[0].axes.iter().all(|a| a.delta.is_none() && a.delta_pct.is_none()));
    let contrast = |i: usize| c.faces[i].axes.iter().find(|a| a.key == "contrast").unwrap().clone();
    let (b, p) = (contrast(0), contrast(1));
    let (bv, pv) = (b.raw_value.unwrap(), p.raw_value.unwrap());
    assert!((p.delta.unwrap() - (pv - bv)).abs() < 1e-9);
    assert!((p.delta_pct.unwrap() - (pv - bv) / bv.abs() * 100.0).abs() < 1e-9);
    assert!(pv > bv, "Playfair tương phản cao hơn Roboto");
    // cờ ước lượng khớp quality
    for f in &c.faces {
        assert!(f.axes.iter().all(|a| a.estimated == (a.quality == "estimated")));
        assert!(!f.axes.is_empty());
    }
    // D2: dữ liệu mới có ở font vừa quét
    let r = &c.faces[0];
    assert_eq!(r.extras.vendor_id.as_deref(), Some("GOOG"));
    assert_eq!(r.extras.years.created, Some(2008));
    assert_eq!(r.extras.kerning.source, "gpos");
    assert!(r.extras.kerning.pairs > 0);
    assert_eq!(r.license.kind, "ofl");
    assert_eq!(r.extras.embedding.level, "installable");
    assert!(r.extras.ot_groups.iter().any(|g| g.group == "ligatures"));
    // phân loại có mặt, không có điểm tổng
    assert!(!r.classification.class.is_empty());
    // tiếng Việt: số % vượt không âm, và nhất quán với đỉnh dấu chồng
    for f in &c.faces {
        let v = &f.vietnamese;
        assert_eq!(v.total, 134);
        assert_eq!(v.supports, v.missing == 0, "{v:?}");
        if let (Some(top), Some(cap), Some(over)) = (v.stacked_marks_top_em, v.cap_height_em, v.over_cap_pct_em) {
            assert!((over - ((top - cap) * 100.0).max(0.0)).abs() < 1e-3, "{v:?}");
            assert!(over >= 0.0);
        }
        if let (Some(top), Some(asc), Some(over)) = (v.stacked_marks_top_em, v.hhea_ascender_em, v.over_ascender_pct_em) {
            assert!((over - ((top - asc) * 100.0).max(0.0)).abs() < 1e-3);
            assert_eq!(v.overflow, over > 0.0, "{v:?}");
        }
    }
    // cấu trúc family từ thư viện
    assert_eq!(c.faces[1].family.family, "Playfair Display");
    assert!(c.faces[1].family.faces >= 1 && !c.faces[1].family.weights.is_empty());
}

#[test]
fn compare_rejects_bad_input() {
    let td = tempfile::tempdir().unwrap();
    let st = lib6(td.path());
    let a = key_of(&st, "Roboto");
    let b = key_of(&st, "Playfair Display");
    assert!(compare::compare_faces(&st, std::slice::from_ref(&a), &opts()).is_err(), "1 face");
    assert!(compare::compare_faces(&st, &[a.clone(), a.clone()], &opts()).is_err(), "trùng");
    let five: Vec<FaceKeyDto> = ["Roboto", "Playfair Display", "Open Sans", "Libre Baskerville", "Source Code Pro"].iter().map(|f| key_of(&st, f)).collect();
    // compareFaces nhận tới 8 (UI chỉ gợi ý > 4); pairThese vẫn giới hạn 4
    assert_eq!(compare::compare_faces(&st, &five, &opts()).unwrap().faces.len(), 5);
    let po = PairTheseOptsDto { project_vi: false, lang: LangDto::Vi, max_combos: 3 };
    let err = compare::pair_these(&st, &five, &po).unwrap_err();
    assert!(err.code == "invalid_arg" && err.message.contains("2–4"), "{err:?}");
    let nine: Vec<FaceKeyDto> = (0..9u8).map(|i| FaceKeyDto { hash_hex: format!("{i:02x}").repeat(32), face_index: 0 }).collect();
    let err = compare::compare_faces(&st, &nine, &opts()).unwrap_err();
    assert!(err.code == "invalid_arg" && err.message.contains("2–8"), "{err:?}");
    let ghost = FaceKeyDto { hash_hex: "00".repeat(32), face_index: 0 };
    assert!(compare::compare_faces(&st, &[a.clone(), ghost.clone()], &opts()).is_err());
    assert!(compare::compare_faces(&st, &[a, b], &opts()).is_ok());
    assert!(compare::pair_these(&st, &[ghost.clone(), ghost], &PairTheseOptsDto { project_vi: false, lang: LangDto::Vi, max_combos: 3 }).is_err());
}

fn gate_rank(g: &str) -> u8 {
    match g {
        "safe" => 0,
        "controlled" => 1,
        "experimental" => 2,
        _ => 3,
    }
}

#[test]
fn pair_these_tries_every_role_assignment_and_ranks() {
    let td = tempfile::tempdir().unwrap();
    let st = lib6(td.path());
    let po = |max| PairTheseOptsDto { project_vi: false, lang: LangDto::Vi, max_combos: max };
    let (a, b) = (key_of(&st, "Playfair Display"), key_of(&st, "Roboto"));

    // 2 font → đúng 2 cách gán (A→H/B→B và ngược lại)
    let two = compare::pair_these(&st, &[a.clone(), b.clone()], &po(3)).unwrap();
    assert_eq!(two.len(), 2);
    assert_eq!(two.iter().map(|c| c.rank).collect::<Vec<_>>(), vec![1, 2]);
    let roles: std::collections::HashSet<(String, String)> = two.iter().map(|c| (c.heading.family.clone(), c.body.family.clone())).collect();
    assert!(roles.contains(&("Playfair Display".into(), "Roboto".into())) && roles.contains(&("Roboto".into(), "Playfair Display".into())));
    for c in &two {
        assert_ne!(c.heading.face_id, c.body.face_id);
        assert_eq!(c.assessment.row.face_id, c.body.face_id);
        assert!(!c.assessment.sentences.is_empty());
        assert!(!c.assessment.labels.is_empty());
    }
    // xếp hạng: cổng không tăng dần ngược (an toàn trước)
    assert!(gate_rank(&two[0].assessment.gate) <= gate_rank(&two[1].assessment.gate));
    // trùng với assess_pair đơn lẻ (cùng bộ máy với Pairing)
    let direct = app_lib::services::analysis::assess_pair(
        &st,
        &two[0].heading_key(),
        &two[0].body_key(),
        &PairOptsDto { role: PairRoleDto::Heading, mode: None, project_vi: false, k: 1, collection_id: None, lang: LangDto::Vi },
    )
    .unwrap();
    assert_eq!(direct.gate, two[0].assessment.gate);
    assert_eq!(direct.clashes, two[0].assessment.clashes);

    // 4 font → 12 cách gán, chỉ trả tối đa 3, và max_combos giới hạn
    let four: Vec<FaceKeyDto> = ["Roboto", "Playfair Display", "Open Sans", "Libre Baskerville"].iter().map(|f| key_of(&st, f)).collect();
    let top = compare::pair_these(&st, &four, &po(3)).unwrap();
    assert_eq!(top.len(), 3);
    assert!(top.windows(2).all(|w| gate_rank(&w[0].assessment.gate) <= gate_rank(&w[1].assessment.gate)));
    assert_eq!(compare::pair_these(&st, &four, &po(1)).unwrap().len(), 1);
    assert_eq!(compare::pair_these(&st, &four, &po(99)).unwrap().len(), 3, "kẹp ở 3");
    // xác định: gọi lại cho cùng kết quả
    let again = compare::pair_these(&st, &four, &po(3)).unwrap();
    assert_eq!(top, again);
}

trait Keys {
    fn heading_key(&self) -> FaceKeyDto;
    fn body_key(&self) -> FaceKeyDto;
}
impl Keys for PairComboDto {
    fn heading_key(&self) -> FaceKeyDto {
        FaceKeyDto { hash_hex: self.heading.hash_hex.clone(), face_index: self.heading.face_index }
    }
    fn body_key(&self) -> FaceKeyDto {
        FaceKeyDto { hash_hex: self.body.hash_hex.clone(), face_index: self.body.face_index }
    }
}
