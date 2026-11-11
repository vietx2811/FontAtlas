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
    assert_eq!(f.axes.len(), 22);
    let ch = f.axes.iter().find(|a| a.key == "contrast_h").expect("contrast_h");
    assert_eq!(ch.unit.as_deref(), Some("ratio"));
    assert!(ch.raw_value.is_some_and(|v| v > 1.0), "Playfair H stem/bar > 1");
    assert_eq!(f.classification.as_deref(), Some("Serif"));
    assert_eq!(f.classification_info.class, "serif");
    assert_eq!(f.classification_info.source, "auto");
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

#[test]
fn class_filter_facets_and_override_use_effective_class() {
    let td = tempfile::tempdir().unwrap();
    let st = lib6(td.path());
    let total = library::query_faces(&st, &FaceQueryDto { limit: 50, ..Default::default() }).unwrap().total;

    // facet đếm đủ mọi face; mỗi class lọc ra đúng số facet báo
    let facets = analysis::class_facets(&st, &FaceQueryDto::default()).unwrap();
    assert_eq!(facets.iter().map(|f| u64::from(f.count)).sum::<u64>(), total);
    let mono = facets.iter().find(|f| f.class == "mono").expect("Source Code Pro là Mono (tín hiệu cứng, không bị hạ về Uncertain)");
    let q = FaceQueryDto { classes: vec!["mono".into()], limit: 50, ..Default::default() };
    assert_eq!(library::query_faces(&st, &q).unwrap().total, u64::from(mono.count));

    // thư viện khoẻ: không có face thiếu hồ sơ → không có bucket unclassified, và lọc "unclassified" rỗng
    assert!(facets.iter().all(|f| f.class != "unclassified"));
    let unc = FaceQueryDto { classes: vec!["unclassified".into()], limit: 50, ..Default::default() };
    assert_eq!(library::query_faces(&st, &unc).unwrap().total, 0);

    // class lạ bị từ chối
    let bad = FaceQueryDto { classes: vec!["wingdings".into()], ..Default::default() };
    assert!(library::query_faces(&st, &bad).is_err());

    // facet bỏ qua chính bộ lọc class (không tự triệt tiêu)
    let f2 = analysis::class_facets(&st, &q).unwrap();
    assert_eq!(f2, facets);

    // override chuyển Roboto sang Script/formal; lọc và facet phản ánh ngay
    let roboto = find(&st, "Roboto");
    analysis::set_classification_override(
        &st,
        &ClassificationOverrideDto { face: key(&roboto), class: "script".into(), subclass: Some("formal".into()) },
    )
    .unwrap();
    // nhãn v1 `formal` được lưu dạng v2 `formal_copperplate`
    let qd = FaceQueryDto { classes: vec!["script".into()], subclasses: vec!["formal_copperplate".into()], limit: 50, ..Default::default() };
    let hit = library::query_faces(&st, &qd).unwrap();
    assert!(hit.items.iter().any(|r| r.family == "Roboto"));
    let disp = analysis::class_facets(&st, &FaceQueryDto::default()).unwrap();
    assert!(disp.iter().find(|f| f.class == "script").unwrap().subclasses.iter().any(|s| s.subclass == "formal_copperplate"));
    // taxonomy v2: Display là class (lọc/ghi đè được); class lạ vẫn bị từ chối
    assert!(library::query_faces(&st, &FaceQueryDto { classes: vec!["display".into()], ..Default::default() }).is_ok());
    assert!(library::query_faces(&st, &FaceQueryDto { classes: vec!["wiggly".into()], ..Default::default() }).is_err());
    assert!(analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key(&roboto), class: "display".into(), subclass: None }).is_ok());
    analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key(&roboto), class: "script".into(), subclass: Some("formal".into()) }).unwrap();
    // lọc traits: giá trị lạ bị từ chối, giá trị hợp lệ chạy được
    assert!(library::query_faces(&st, &FaceQueryDto { traits: vec!["wiggly".into()], ..Default::default() }).is_err());
    assert!(library::query_faces(&st, &FaceQueryDto { traits: vec!["stencil".into()], ..Default::default() }).is_ok());

    // xoá override → trở lại tự động
    analysis::clear_classification_override(&st, &key(&roboto)).unwrap();
    assert!(!library::query_faces(&st, &qd).unwrap().items.iter().any(|r| r.family == "Roboto"));

    // face không tồn tại bị từ chối khi ghi đè
    let ghost = ClassificationOverrideDto { face: FaceKeyDto { hash_hex: "00".repeat(32), face_index: 0 }, class: "sans".into(), subclass: None };
    assert!(analysis::set_classification_override(&st, &ghost).is_err());

    // usage tính lại theo class ghi đè (Display không có "text")
    analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key(&roboto), class: "script".into(), subclass: None }).unwrap();
    let d = analysis::get_face(&st, &key(&roboto), LangDto::Vi).unwrap();
    assert_eq!(d.classification_info.source, "override");
    assert_eq!(d.classification_info.usage, vec!["display".to_string()]);
    analysis::clear_classification_override(&st, &key(&roboto)).unwrap();

    // subclass không thuộc class bị từ chối khi ghi đè
    let invalid = ClassificationOverrideDto { face: key(&roboto), class: "sans".into(), subclass: Some("didone".into()) };
    assert!(analysis::set_classification_override(&st, &invalid).is_err());
}

/// Đổi tên họ trong bảng `name` (cùng độ dài, cả ASCII/Mac và UTF-16BE) để font có từ khoá trait trong tên.
fn rename_in_font(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
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

fn lib_with_traits(root: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for n in SIX {
        let raw = std::fs::read(fixture(n)).unwrap();
        let raw = match n {
            // "Open Sans" → "Rounded  " (token `rounded`); "Libre Baskerville" → "Stencil" + đệm
            "open_sans.ttf" => rename_in_font(&raw, "Open Sans", "Rounded  "),
            "libre_baskerville.ttf" => rename_in_font(&raw, "Libre Baskerville", "Stencil          "),
            _ => raw,
        };
        std::fs::write(dir.join(n), raw).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

#[test]
fn traits_filter_or_and_facets_with_named_fonts() {
    let td = tempfile::tempdir().unwrap();
    let st = lib_with_traits(td.path());
    let page = |q: FaceQueryDto| library::query_faces(&st, &FaceQueryDto { limit: 50, ..q }).unwrap();
    let total = page(FaceQueryDto::default()).total;
    assert_eq!(total, 6);

    let stencil = page(FaceQueryDto { traits: vec!["stencil".into()], ..Default::default() });
    let rounded = page(FaceQueryDto { traits: vec!["rounded".into()], ..Default::default() });
    assert_eq!(stencil.total, 1, "{:?}", stencil.items.iter().map(|r| &r.family).collect::<Vec<_>>());
    assert_eq!(rounded.total, 1);
    assert_ne!(stencil.items[0].hash_hex, rounded.items[0].hash_hex);
    // không có trait pixel → rỗng
    assert_eq!(page(FaceQueryDto { traits: vec!["pixel".into()], ..Default::default() }).total, 0);
    // OR trong traits
    let both = page(FaceQueryDto { traits: vec!["stencil".into(), "rounded".into()], ..Default::default() });
    assert_eq!(both.total, 2);

    // traits được báo trong chi tiết face
    let d = analysis::get_face(&st, &key(&stencil.items[0]), LangDto::Vi).unwrap();
    assert_eq!(d.classification_info.traits, vec!["stencil".to_string()]);

    // AND với classes/subclasses: dùng class hiệu lực của face stencil
    let cls = d.classification_info.class.clone();
    let other = if cls == "mono" { "script" } else { "mono" };
    assert_eq!(page(FaceQueryDto { traits: vec!["stencil".into()], classes: vec![cls.clone()], ..Default::default() }).total, 1);
    assert_eq!(page(FaceQueryDto { traits: vec!["stencil".into()], classes: vec![other.into()], ..Default::default() }).total, 0);
    // ghi đè class đổi kết quả AND nhưng không đổi traits
    analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key(&stencil.items[0]), class: other.into(), subclass: None }).unwrap();
    assert_eq!(page(FaceQueryDto { traits: vec!["stencil".into()], classes: vec![other.into()], ..Default::default() }).total, 1);
    assert_eq!(page(FaceQueryDto { traits: vec!["stencil".into()], classes: vec![cls.clone()], ..Default::default() }).total, 0);
    let d2 = analysis::get_face(&st, &key(&stencil.items[0]), LangDto::Vi).unwrap();
    assert_eq!(d2.classification_info.traits, vec!["stencil".to_string()]);
    // subclass không khớp → 0
    assert_eq!(page(FaceQueryDto { traits: vec!["stencil".into()], classes: vec![other.into()], subclasses: vec!["didone".into()], ..Default::default() }).total, 0);

    // facet: tổng = tổng thư viện; với traits thì tổng = số face có trait
    let sum = |f: &[ClassFacetDto]| f.iter().map(|x| u64::from(x.count)).sum::<u64>();
    assert_eq!(sum(&analysis::class_facets(&st, &FaceQueryDto::default()).unwrap()), total);
    let with = analysis::class_facets(&st, &FaceQueryDto { traits: vec!["stencil".into(), "rounded".into()], ..Default::default() }).unwrap();
    assert_eq!(sum(&with), 2);
    // facet bỏ qua classes/subclasses nhưng giữ traits
    let ign = analysis::class_facets(&st, &FaceQueryDto { traits: vec!["stencil".into()], classes: vec!["mono".into()], ..Default::default() }).unwrap();
    assert_eq!(sum(&ign), 1);
}

#[test]
fn confident_only_and_include_secondary_filters() {
    let td = tempfile::tempdir().unwrap();
    let st = lib6(td.path());
    let q = |f: &dyn Fn(&mut FaceQueryDto)| {
        let mut q = FaceQueryDto { limit: 500, ..Default::default() };
        f(&mut q);
        library::query_faces(&st, &q)
    };
    let all = q(&|_| {}).unwrap().total;
    let confident = q(&|q| q.confident_only = true).unwrap();
    assert!(confident.total <= all);
    // face uncertain không bao giờ "tự tin"
    assert_eq!(q(&|q| { q.confident_only = true; q.classes = vec!["uncertain".into()]; }).unwrap().total, 0);
    // ngưỡng cao hơn chỉ giữ ít hơn hoặc bằng
    assert!(q(&|q| q.min_confidence = Some(0.99)).unwrap().total <= confident.total.max(q(&|q| q.min_confidence = Some(0.88)).unwrap().total));
    assert!(q(&|q| q.min_confidence = Some(2.0)).is_err());
    // includeSecondary chỉ mở rộng
    let base = q(&|q| q.classes = vec!["serif".into()]).unwrap().total;
    let wide = q(&|q| { q.classes = vec!["serif".into()]; q.include_secondary = true; }).unwrap().total;
    assert!(wide >= base);
    // subclass của class mới (slab/display/handwriting) hợp lệ khi lọc
    for s in ["clarendon", "stencil", "casual_print", "formal_copperplate", "coding"] {
        assert!(q(&|q| q.subclasses = vec![s.into()]).is_ok(), "{s}");
    }
    assert!(q(&|q| q.subclasses = vec!["wiggly".into()]).is_err());
    // ghi đè của người dùng = tin cậy tuyệt đối: face đó có trong confidentOnly khi khớp class
    let roboto = find(&st, "Roboto");
    analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key(&roboto), class: "display".into(), subclass: Some("stencil".into()) }).unwrap();
    let hit = q(&|q| { q.confident_only = true; q.classes = vec!["display".into()]; }).unwrap();
    assert!(hit.items.iter().any(|r| r.family == "Roboto"));
}

#[test]
fn legacy_override_labels_are_stored_as_v2_and_usage_matches_auto() {
    let td = tempfile::tempdir().unwrap();
    let st = lib6(td.path());
    let roboto = find(&st, "Roboto");
    let set = |c: &str, s: Option<&str>| analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key(&roboto), class: c.into(), subclass: s.map(String::from) });
    let stored = || {
        let k = fontatlas_store::FaceKey::try_from(&key(&roboto)).unwrap();
        st.with_user(|u| Ok(u.classification_override(&k)?)).unwrap().map(|o| (o.class, o.subclass))
    };
    // nhãn v1 vẫn được nhận nhưng chỉ lưu dạng v2
    for (c, s, want) in [
        ("serif", Some("slab"), ("slab", None)),
        ("script", Some("handwritten"), ("handwriting", None)),
        ("script", Some("casual_brush"), ("script", Some("brush"))),
        ("script", Some("formal"), ("script", Some("formal_copperplate"))),
        ("serif", Some("humanist_oldstyle"), ("serif", Some("venetian_oldstyle"))),
        ("sans", Some("rounded"), ("sans", Some("rounded"))),
    ] {
        set(c, s).unwrap();
        assert_eq!(stored(), Some((want.0.to_string(), want.1.map(String::from))), "{c}/{s:?}");
    }
    // rác vẫn bị từ chối, và không ghi gì
    set("sans", Some("slab")).unwrap_err();
    set("wiggly", None).unwrap_err();
    // usage của slab ghi đè = usage của Serif cùng face (không rơi vào nhánh mặc định)
    set("slab", None).unwrap();
    let slab = analysis::get_face(&st, &key(&roboto), LangDto::Vi).unwrap().classification_info.usage;
    set("serif", None).unwrap();
    let serif = analysis::get_face(&st, &key(&roboto), LangDto::Vi).unwrap().classification_info.usage;
    assert_eq!(slab, serif);
    set("display", None).unwrap();
    assert_eq!(analysis::get_face(&st, &key(&roboto), LangDto::Vi).unwrap().classification_info.usage, vec!["display".to_string()]);
}

#[test]
fn effective_class_is_consistent_across_detail_filter_facets_and_index() {
    let td = tempfile::tempdir().unwrap();
    let st = lib6(td.path());
    let b = analysis::bundle(&st).unwrap();
    // đổi class của một face bằng ghi đè: mọi nơi đọc phải thấy cùng nhãn
    let roboto = find(&st, "Roboto");
    analysis::set_classification_override(&st, &ClassificationOverrideDto { face: key(&roboto), class: "slab".into(), subclass: Some("clarendon".into()) }).unwrap();
    let detail = analysis::get_face(&st, &key(&roboto), LangDto::Vi).unwrap().classification_info;
    assert_eq!((detail.class.as_str(), detail.subclass.as_deref(), detail.source.as_str()), ("slab", Some("clarendon"), "override"));
    let hit = library::query_faces(&st, &FaceQueryDto { classes: vec!["slab".into()], subclasses: vec!["clarendon".into()], limit: 50, ..Default::default() }).unwrap();
    assert!(hit.items.iter().any(|r| r.family == "Roboto"));
    let facets = analysis::class_facets(&st, &FaceQueryDto::default()).unwrap();
    assert!(facets.iter().find(|f| f.class == "slab").unwrap().subclasses.iter().any(|s| s.subclass == "clarendon"));
    // class tự động của Index (nguồn của compare/similar/brief/pairing) không bị ghi đè của người dùng đụng tới
    let key_roboto = fontatlas_index::FaceKey { hash: roboto.hash_hex.clone(), index: roboto.face_index };
    assert_ne!(b.meta[&key_roboto].auto_class, "slab");
    // confidentOnly: ghi đè luôn tự tin ⇒ face vẫn có mặt; face tự động dưới ngưỡng thì không
    let conf = library::query_faces(&st, &FaceQueryDto { confident_only: true, classes: vec!["slab".into()], limit: 50, ..Default::default() }).unwrap();
    assert!(conf.items.iter().any(|r| r.family == "Roboto"));
    // includeSecondary: facet chỉ có thể nhiều hơn
    let with = analysis::class_facets(&st, &FaceQueryDto { include_secondary: true, ..Default::default() }).unwrap();
    let sum = |v: &[ClassFacetDto]| v.iter().map(|f| f.count).sum::<u32>();
    assert!(sum(&with) >= sum(&facets));
}
