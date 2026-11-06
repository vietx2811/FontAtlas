use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::analysis::*;
use app_lib::dto::library::*;
use app_lib::index_bundle::{RebuildWhy, current_model_version, request_rebuild, wait_rebuilt};
use app_lib::services::activation::NoActivation;
use app_lib::services::{analysis, boot, describe, library, pairs, sources};
use app_lib::state::{AppDirs, AppState};

const SIX: [&str; 6] = [
    "roboto.ttf",
    "open_sans.ttf",
    "playfair_display.ttf",
    "libre_baskerville.ttf",
    "source_code_pro.ttf",
    "public_sans_vf.ttf",
];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/fontatlas-core/tests/fixtures")
        .join(name)
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

fn find(st: &AppState, family: &str) -> FaceRowDto {
    let q = FaceQueryDto {
        text: Some(family.into()),
        limit: 50,
        ..Default::default()
    };
    library::query_faces(st, &q)
        .unwrap()
        .items
        .into_iter()
        .find(|r| r.family == family)
        .unwrap_or_else(|| panic!("{family}"))
}

fn key(r: &FaceRowDto) -> FaceKeyDto {
    FaceKeyDto {
        hash_hex: r.hash_hex.clone(),
        face_index: r.face_index,
    }
}

fn popts(role: PairRoleDto) -> PairOptsDto {
    PairOptsDto {
        role,
        mode: None,
        project_vi: false,
        k: 50,
        collection_id: None,
        lang: LangDto::Vi,
    }
}

#[test]
fn index_rebuilt_after_scan() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let b = analysis::bundle(&st).unwrap();
    assert!(b.len() >= 6, "{}", b.len());
}

#[test]
fn get_face_playfair() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let f = analysis::get_face(&st, &key(&find(&st, "Playfair Display")), LangDto::Vi).unwrap();
    assert!(
        f.summary.starts_with("Serif, tương phản cao"),
        "{}",
        f.summary
    );
    assert_eq!(f.file.format, "ttf");
    assert!(f.file.num_glyphs > 0 && f.file.size_bytes > 0 && f.file.path.is_some());
    assert!(f.metadata.copyright.is_some());
    assert_eq!(f.axes.len(), 21);
    assert_eq!(f.classification.as_deref(), Some("Serif"));
    assert_eq!(f.vietnamese.total, 134);
    assert!(f.vietnamese.supports && f.vietnamese.missing.is_empty());
    assert!(
        f.axes
            .iter()
            .find(|a| a.key == "contrast")
            .unwrap()
            .percentile
            .is_some()
    );
    assert_eq!(
        f.axes.iter().find(|a| a.key == "serif_foot").unwrap().value,
        Some(serde_json::json!("serif"))
    );
}

#[test]
fn get_face_libre_missing_list() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let f = analysis::get_face(&st, &key(&find(&st, "Libre Baskerville")), LangDto::Vi).unwrap();
    assert_eq!(f.vietnamese.missing.len(), 68);
    assert!(f.vietnamese.missing.contains(&"ơ".to_string()));
    assert!(f.summary.contains("Thiếu 68 ký tự Việt."));
    let ps = analysis::get_face(&st, &key(&find(&st, "Public Sans")), LangDto::En).unwrap();
    assert!(!ps.metadata.variable_axes.is_empty() && !ps.metadata.named_instances.is_empty());
}

#[test]
fn similar_works_without_model() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let r = find(&st, "Roboto");
    let hits = analysis::similar(
        &st,
        &key(&r),
        &SimilarOptsDto {
            k: 4,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!hits.is_empty() && hits.len() <= 4);
    assert!(
        hits.iter()
            .all(|h| h.source == SimilarSourceDto::Axes && h.face != key(&r))
    );
    let none = analysis::similar(
        &st,
        &FaceKeyDto {
            hash_hex: "00".repeat(32),
            face_index: 0,
        },
        &SimilarOptsDto::default(),
    );
    assert_eq!(none.unwrap_err().code, "not_found");
}

#[test]
fn similar_falls_back_to_image_for_caps_only() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let ver = current_model_version();
    let q = find(&st, "Roboto");
    // mọi trục Failed + embedding giả cho mọi face → so được theo ảnh
    let failed: serde_json::Value = serde_json::Value::Object(
        ["x_height_em", "x_height_cap", "ascender_em", "stem", "darkness", "contrast", "stress_deg", "aperture", "aperture_e", "width", "mean_advance", "width_cv", "spacing", "slant_deg", "serif_foot", "counter", "terminal", "a_storeys", "g_storeys"]
            .iter()
            .map(|k| ((*k).to_string(), serde_json::json!({ "value": null, "quality": { "kind": "failed", "reason": "missing_glyph" }, "method": "x" })))
            .collect(),
    );
    st.with_store(|s| {
        let hash = hex_bytes(&q.hash_hex);
        s.conn().execute("UPDATE analyses SET anatomy_json = ?1 WHERE hash = ?2", rusqlite::params![failed.to_string(), &hash[..]]).unwrap();
        let ids: Vec<(Vec<u8>, i64)> = {
            let mut stm = s.conn().prepare("SELECT fi.hash, f.face_index FROM faces f JOIN files fi USING(file_id)").unwrap();
            stm.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(|x| x.unwrap()).collect()
        };
        for (i, (h, fi)) in ids.iter().enumerate() {
            let v: Vec<f32> = (0..8).map(|j| ((i * 7 + j * 3) as f32).sin()).collect();
            let blob: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
            s.conn()
                .execute(
                    "INSERT INTO embeddings(hash,face_index,model_version,source,dim,dtype,vec,created_at) VALUES (?1,?2,?3,'local',8,'f32',?4,1)",
                    rusqlite::params![h, fi, ver, blob],
                )
                .unwrap();
        }
        Ok(())
    })
    .unwrap();
    app_lib::index_bundle::rebuild_index(&st).unwrap();
    let hits = analysis::similar(
        &st,
        &key(&q),
        &SimilarOptsDto {
            k: 3,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!hits.is_empty() && hits.iter().all(|h| h.source == SimilarSourceDto::Image));
    let off = SimilarOptsDto {
        use_image: false,
        ..Default::default()
    };
    assert_eq!(
        analysis::similar(&st, &key(&q), &off).unwrap_err().code,
        "not_comparable"
    );
}

fn hex_bytes(h: &str) -> Vec<u8> {
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn pair_candidates_mode_filter_and_no_score_field() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let q = key(&find(&st, "Playfair Display"));
    let all = analysis::pair_candidates(&st, &q, &popts(PairRoleDto::Heading)).unwrap();
    assert!(!all.is_empty());
    let json = serde_json::to_string(&all).unwrap();
    assert!(
        !json.to_lowercase().contains("score"),
        "không có trường điểm số"
    );
    assert!(
        all.iter()
            .all(|c| ["safe", "controlled", "experimental"].contains(&c.gate.as_str()))
    );
    assert!(all.iter().all(|c| !c.summary.is_empty() && c.face != q));
    let mut o = popts(PairRoleDto::Heading);
    o.mode = Some(PairModeDto::Experimental);
    let exp = analysis::pair_candidates(&st, &q, &o).unwrap();
    assert!(exp.iter().all(|c| c.gate == "experimental"));
    assert!(exp.len() <= all.len());
    for c in &all {
        for l in &c.labels {
            assert!(["●", "◐", "◆", "○", "◑"].contains(&l.symbol.as_str()));
        }
    }
}

#[test]
fn pair_candidates_collection_scope() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let q = key(&find(&st, "Playfair Display"));
    let col = library::create_collection(&st, "Menu", None).unwrap();
    let inside: Vec<FaceKeyDto> = ["Roboto", "Open Sans"]
        .iter()
        .map(|f| key(&find(&st, f)))
        .collect();
    library::add_to_collection(&st, col.id, &inside).unwrap();
    let mut o = popts(PairRoleDto::Heading);
    o.collection_id = Some(col.id);
    let got = analysis::pair_candidates(&st, &q, &o).unwrap();
    assert!(
        got.iter().all(|c| inside.contains(&c.face)),
        "{:?}",
        got.iter().map(|c| &c.row.family).collect::<Vec<_>>()
    );
}

#[test]
fn pair_project_vi_blocks_missing() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let libre = key(&find(&st, "Libre Baskerville"));
    let mut o = popts(PairRoleDto::Heading);
    o.project_vi = true;
    assert!(
        analysis::pair_candidates(&st, &libre, &o)
            .unwrap()
            .is_empty()
    );
    let x = analysis::assess_pair(&st, &libre, &key(&find(&st, "Roboto")), &o).unwrap();
    assert_eq!(x.gate, "blocked");
    assert!(
        x.block
            .as_deref()
            .unwrap()
            .starts_with("missing_vietnamese:a:68")
    );
    // ứng viên thiếu tiếng Việt không xuất hiện khi query đủ
    let got = analysis::pair_candidates(&st, &key(&find(&st, "Roboto")), &o).unwrap();
    assert!(got.iter().all(|c| c.row.family != "Libre Baskerville"));
}

#[test]
fn saved_pairs_roundtrip() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let (a, b) = (
        key(&find(&st, "Playfair Display")),
        key(&find(&st, "Roboto")),
    );
    let col = library::create_collection(&st, "Poster", None).unwrap();
    let np = NewPairDto {
        a: a.clone(),
        a_role: PairRoleDto::Heading,
        b: b.clone(),
        b_role: PairRoleDto::Body,
        mode: Some("safe".into()),
        preset: Some("poster".into()),
        sample_text: Some("Phở bò".into()),
        collection_id: Some(col.id),
        note: None,
    };
    let id = pairs::save_pair(&st, &np).unwrap();
    let all = pairs::list_pairs(&st, None).unwrap();
    assert_eq!(
        (
            all.len(),
            all[0].id,
            all[0].a.clone(),
            all[0].b_role.as_str()
        ),
        (1, id, a, "body")
    );
    assert_eq!(pairs::list_pairs(&st, Some(col.id)).unwrap().len(), 1);
    let bad = NewPairDto {
        mode: Some("lung_tung".into()),
        ..np.clone()
    };
    assert_eq!(pairs::save_pair(&st, &bad).unwrap_err().code, "invalid_arg");
    pairs::delete_pair(&st, id).unwrap();
    assert!(pairs::list_pairs(&st, None).unwrap().is_empty());
    assert_eq!(pairs::delete_pair(&st, id).unwrap_err().code, "not_found");
}

#[test]
fn describe_then_query() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let desc = describe::describe(&st, "tương phản cao");
    assert!(!desc.filters.is_empty() && desc.matched[0].id == "high_contrast");
    let q = FaceQueryDto {
        sort: SortKeyDto::Match,
        axis_filters: desc.filters,
        limit: 10,
        ..Default::default()
    };
    let page = library::query_faces(&st, &q).unwrap();
    // `required` loại font tương phản thấp; hai font còn lại cùng thoả khoảng nên hoà điểm → theo tên họ
    let fams: Vec<&str> = page.items.iter().map(|r| r.family.as_str()).collect();
    assert_eq!(fams, ["Libre Baskerville", "Playfair Display"]);
    assert_eq!(page.total, 2);
}

#[test]
fn index_build_does_not_hold_main_store_lock() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let guard = st.store().unwrap().lock().unwrap(); // giữ khoá Store chính
    let t = std::time::Instant::now();
    let st2 = st.clone();
    let h = std::thread::spawn(move || app_lib::index_bundle::rebuild_index(&st2));
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        h.is_finished(),
        "rebuild_index bị chặn bởi khoá Store chính"
    );
    h.join().unwrap().unwrap();
    drop(guard);
    assert!(t.elapsed() < Duration::from_secs(5));
}

#[test]
fn rebuild_after_embed_is_debounced() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    st.rebuild.policy.lock().unwrap().min_interval = Duration::from_millis(400);
    let before = st.rebuild.builds.load(std::sync::atomic::Ordering::SeqCst);
    for _ in 0..10 {
        request_rebuild(&st, RebuildWhy::EmbedProgress);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(wait_rebuilt(&st, Duration::from_secs(30)));
    let after = st.rebuild.builds.load(std::sync::atomic::Ordering::SeqCst);
    assert!(after - before <= 1, "{before} → {after}");
    request_rebuild(&st, RebuildWhy::QueueEmpty);
    assert!(wait_rebuilt(&st, Duration::from_secs(30)));
    assert!(
        st.rebuild.builds.load(std::sync::atomic::Ordering::SeqCst) > after,
        "QueueEmpty dựng ngay"
    );
}

#[test]
fn index_uses_stale_version_analyses() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    st.with_store(|s| {
        s.conn()
            .execute("UPDATE analyses SET analyzer_version = 'core-0.0.1'", [])
            .unwrap();
        Ok(())
    })
    .unwrap();
    app_lib::index_bundle::rebuild_index(&st).unwrap();
    assert!(analysis::bundle(&st).unwrap().len() >= 6);
    let f = analysis::get_face(&st, &key(&find(&st, "Roboto")), LangDto::En).unwrap();
    assert!(!f.summary.is_empty());
}

#[test]
fn family_pages_stats_and_raw_filters_use_real_library() {
    let d = tempfile::tempdir().unwrap();
    let st = lib6(d.path());
    let face = key(&find(&st, "Roboto"));
    let opts = SimilarOptsDto::default();
    let all = analysis::similar_families(
        &st,
        &face,
        &opts,
        &FaceQueryDto {
            limit: 100,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(all.total > 0);
    assert!(all.items.iter().all(|g| g.family != "Roboto"
        && g.count as usize == g.faces.len()
        && g.faces.contains(&g.representative.face)));
    let mut paged = Vec::new();
    for off in 0..all.total {
        let p = analysis::similar_families(
            &st,
            &face,
            &opts,
            &FaceQueryDto {
                offset: off,
                limit: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(p.total, all.total);
        paged.extend(p.items);
    }
    assert_eq!(paged, all.items);
    let fav = library::create_collection(&st, "One", None).unwrap();
    let target = all.items[0].representative.face.clone();
    library::add_to_collection(&st, fav.id, std::slice::from_ref(&target)).unwrap();
    let only = analysis::similar_families(
        &st,
        &face,
        &opts,
        &FaceQueryDto {
            collection_id: Some(fav.id),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(only.total, 1);
    assert_eq!(
        only.items[0].faces.as_slice(),
        std::slice::from_ref(&target)
    );
    let stats = analysis::axis_stats(
        &st,
        &FaceQueryDto {
            collection_id: Some(fav.id),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(stats.len(), 16);
    let width = stats.iter().find(|s| s.axis == "width").unwrap();
    assert_eq!(width.unit, "x_height_ratio");
    assert_eq!(width.count, 1);
    let v = width.min.unwrap();
    let range = RawAxisRangeDto {
        axis: "width".into(),
        range: [v, v],
    };
    let q = FaceQueryDto {
        collection_id: Some(fav.id),
        raw_axis_ranges: vec![range],
        ..Default::default()
    };
    assert_eq!(library::query_faces(&st, &q).unwrap().total, 1);
    assert_eq!(
        analysis::similar_families(&st, &face, &opts, &q)
            .unwrap()
            .total,
        1
    );
    let mut q2 = q.clone();
    q2.raw_axis_ranges[0].range = [v + 1.0, v + 2.0];
    assert_eq!(library::query_faces(&st, &q2).unwrap().total, 0);
    assert_eq!(
        analysis::similar_families(&st, &face, &opts, &q2)
            .unwrap()
            .total,
        0
    );
    assert_eq!(
        analysis::axis_stats(&st, &q).unwrap(),
        analysis::axis_stats(&st, &q2).unwrap()
    );
    for range in [
        RawAxisRangeDto {
            axis: "fake".into(),
            range: [0.0, 1.0],
        },
        RawAxisRangeDto {
            axis: "width".into(),
            range: [2.0, 1.0],
        },
        RawAxisRangeDto {
            axis: "width".into(),
            range: [f32::NAN, 1.0],
        },
    ] {
        let q = FaceQueryDto {
            raw_axis_ranges: vec![range],
            ..Default::default()
        };
        assert_eq!(
            library::query_faces(&st, &q).unwrap_err().code,
            "invalid_arg"
        );
        assert_eq!(
            analysis::similar_families(&st, &face, &opts, &q)
                .unwrap_err()
                .code,
            "invalid_arg"
        );
    }
    let detail = analysis::get_face(&st, &target, LangDto::Vi).unwrap();
    let serif = detail.axes.iter().find(|a| a.key == "serif_foot").unwrap();
    assert!(serif.raw_value.is_some());
    assert_eq!(serif.unit.as_deref(), Some("ratio"));
    assert!(
        detail
            .axes
            .iter()
            .find(|a| a.key == "descender_em")
            .unwrap()
            .raw_value
            .unwrap()
            < 0.0
    );
}

#[test]
fn axis_stats_empty_library_and_no_index_readings() {
    let d = tempfile::tempdir().unwrap();
    let st = AppState::new(AppDirs::under(d.path()), Box::new(NoActivation));
    boot::run_boot(&st);
    app_lib::index_bundle::rebuild_index(&st).unwrap();
    let stats = analysis::axis_stats(&st, &FaceQueryDto::default()).unwrap();
    assert_eq!(stats.len(), 16);
    assert!(
        stats
            .iter()
            .all(|s| s.count == 0 && s.min.is_none() && s.p99.is_none())
    );
}
