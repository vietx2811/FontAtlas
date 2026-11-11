use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use app_lib::dto::library::*;
use app_lib::index_bundle::wait_rebuilt;
use app_lib::services::activation::NoActivation;
use app_lib::services::{analysis, boot, library, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

const FILES: [&str; 4] = ["roboto.ttf", "bebas_neue.ttf", "oswald_vf.ttf", "public_sans_vf.ttf"];

fn state(root: &Path) -> Arc<AppState> {
    state_with(root, &FILES)
}

fn state_with(root: &Path, files: &[&str]) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let dir = root.join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::copy(fixture(f), dir.join(f)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(wait_rebuilt(&st, Duration::from_secs(60)));
    st
}

fn names(st: &Arc<AppState>, q: &FaceQueryDto) -> Vec<String> {
    let mut q = q.clone();
    q.limit = 100;
    let mut v: Vec<String> = library::query_faces(st, &q).unwrap().items.into_iter().map(|r| r.family).collect();
    v.sort();
    v.dedup();
    v
}

fn weight(range: [u16; 2], wght: bool) -> FaceQueryDto {
    FaceQueryDto {
        weight_class: Some(range),
        var_axis_match: VarAxisMatchDto { wght, wdth: true },
        ..Default::default()
    }
}

fn hist(st: &Arc<AppState>, q: &FaceQueryDto, axis: &str) -> AxisHistogramDto {
    analysis::axis_histograms(st, q, 8).unwrap().into_iter().find(|h| h.axis == axis).unwrap_or_else(|| panic!("{axis}"))
}

use app_lib::dto::analysis::AxisHistogramDto;

#[test]
fn weight_class_filter_matches_variable_axis_range() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    assert_eq!(names(&st, &weight([800, 900], false)), Vec::<String>::new(), "instance mặc định không ai ≥ 800");
    assert_eq!(names(&st, &weight([800, 900], true)), vec!["Public Sans"], "wght 100–900 giao; Oswald (200–700) thì không");
    assert_eq!(names(&st, &weight([100, 100], false)), vec!["Public Sans"], "usWeightClass 100");
    assert_eq!(names(&st, &weight([500, 600], true)), vec!["Oswald", "Public Sans"]);
    assert_eq!(names(&st, &weight([400, 400], false)), vec!["Bebas Neue", "Oswald", "Roboto"]);
    assert_eq!(names(&st, &weight([400, 400], true)), vec!["Bebas Neue", "Oswald", "Public Sans", "Roboto"]);
}

#[test]
fn width_class_filter_and_invalid_ranges() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let q = FaceQueryDto { width_class: Some([5, 5]), ..Default::default() };
    assert_eq!(names(&st, &q).len(), 4);
    let q = FaceQueryDto { width_class: Some([1, 3]), ..Default::default() };
    assert!(names(&st, &q).is_empty());
    for bad in [[0, 3], [5, 3], [1, 10]] {
        let q = FaceQueryDto { width_class: Some(bad), ..Default::default() };
        assert!(library::query_faces(&st, &q).is_err(), "{bad:?}");
    }
    let q = FaceQueryDto { weight_class: Some([0, 9]), ..Default::default() };
    assert!(library::query_faces(&st, &q).is_err());
    let q = FaceQueryDto { weight_class: Some([900, 100]), ..Default::default() };
    assert!(analysis::axis_histograms(&st, &q, 8).is_err());
}

#[test]
fn histograms_cover_18_axes_and_exclude_their_own_filter() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let all = analysis::axis_histograms(&st, &FaceQueryDto::default(), 0).unwrap();
    assert_eq!(all.len(), 18);
    assert_eq!(all.iter().find(|h| h.axis == "x_height_em").unwrap().bins.len(), analysis::HISTOGRAM_BINS_DEFAULT as usize);
    assert_eq!(analysis::axis_histograms(&st, &FaceQueryDto::default(), 500).unwrap()[0].bins.len(), 64, "kẹp tối đa");
    let wc = all.iter().find(|h| h.axis == "weight_class").unwrap();
    assert_eq!((wc.bins.len(), wc.lo, wc.hi, wc.count), (9, 100.0, 900.0, 4));

    // Bộ lọc weight chỉ làm thu histogram của các trục KHÁC; chính histogram weight_class giữ nguyên.
    let q = weight([100, 100], false);
    assert_eq!(hist(&st, &q, "weight_class").bins, wc.bins);
    assert_eq!(hist(&st, &q, "stem").count, 1, "chỉ Public Sans còn lại");
    assert_eq!(hist(&st, &FaceQueryDto::default(), "stem").count, 4);

    // Bộ lọc khoảng của trục đo: histogram trục đó không co, các trục khác co.
    let stem = analysis::axis_stats(&st, &FaceQueryDto::default()).unwrap().into_iter().find(|s| s.axis == "stem").unwrap();
    let q = FaceQueryDto {
        raw_axis_ranges: vec![RawAxisRangeDto { axis: "stem".into(), range: [stem.min.unwrap(), stem.min.unwrap()] }],
        ..Default::default()
    };
    let stem_h = hist(&st, &q, "stem");
    assert_eq!(stem_h.count, 4, "không tự co theo khoảng của chính nó");
    assert_eq!((stem_h.lo, stem_h.hi), (hist(&st, &FaceQueryDto::default(), "stem").lo, hist(&st, &FaceQueryDto::default(), "stem").hi), "miền cố định");
    assert_eq!(hist(&st, &q, "x_height_em").count, 1);
    assert_eq!(hist(&st, &q, "weight_class").count, 1);
    // Hai bộ lọc trục đo: mỗi histogram chịu ảnh hưởng của bộ lọc còn lại.
    let mut q2 = q.clone();
    q2.raw_axis_ranges.push(RawAxisRangeDto { axis: "width".into(), range: [-1.0, 100.0] });
    assert_eq!(hist(&st, &q2, "stem").count, 4);
    assert_eq!(hist(&st, &q2, "width").count, 1);
}

#[test]
fn describe_axis_filters_are_also_excluded_from_their_own_histogram() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let q = FaceQueryDto {
        axis_filters: vec![AxisFilterDto {
            axis: "x_height_em".into(),
            range: Some([0.0, 0.01]),
            equals: None,
            strength: StrengthDto::Required,
            weight: 1.0,
        }],
        ..Default::default()
    };
    let total = library::query_faces(&st, &q).unwrap().total;
    assert!(total < 4, "bộ lọc phải thu hẹp tập ({total})");
    assert_eq!(hist(&st, &q, "x_height_em").count, 4, "trục bị lọc không tự co");
    assert_eq!(hist(&st, &q, "stem").count as u64, total);
}

#[test]
fn histogram_edge_cases_single_font_empty_axis_and_two_series() {
    let td = tempfile::tempdir().unwrap();
    let st = state_with(td.path(), &["roboto.ttf"]);
    let h = analysis::axis_histograms(&st, &FaceQueryDto::default(), 8).unwrap();
    let stem = h.iter().find(|x| x.axis == "stem").unwrap();
    // một font ⇒ mọi giá trị bằng nhau: miền nới ±0.5, vẫn đếm đủ
    assert_eq!(stem.count, 1);
    assert!((stem.hi - stem.lo - 1.0).abs() < 1e-5, "{} {}", stem.lo, stem.hi);
    assert_eq!(stem.bins.iter().sum::<u32>(), 1);
    assert_eq!(stem.bins, stem.span_bins);
    assert!(!stem.empty);
    // trục không có số đo ở font nào
    let stress = h.iter().find(|x| x.axis == "stress_deg").unwrap();
    assert!(stress.empty && stress.count == 0 && stress.bins.iter().all(|n| *n == 0), "{stress:?}");
    // hai chuỗi của weight_class: Roboto tĩnh ⇒ giống nhau
    let wc = h.iter().find(|x| x.axis == "weight_class").unwrap();
    assert_eq!(wc.bins, wc.span_bins);
    assert!(!wc.empty);
}

#[test]
fn weight_histogram_counts_variable_span_in_second_series() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    let h = analysis::axis_histograms(&st, &FaceQueryDto::default(), 8).unwrap();
    let wc = h.iter().find(|x| x.axis == "weight_class").unwrap();
    assert_eq!(wc.bins.iter().sum::<u32>(), 4, "mỗi font một lần ở instance mặc định");
    // Public Sans wght 100–900 phủ cả chín nấc; Oswald 200–700 phủ nấc 2–7
    assert!(wc.span_bins.iter().sum::<u32>() > wc.bins.iter().sum::<u32>());
    assert!(wc.span_bins[8] >= 1, "nấc 900 có Public Sans");
    assert_eq!(wc.bins[8], 0);
    // kéo 700–900 thì vẫn thấy cột ở chuỗi span dù không face nào mặc định ở đó
    let q = FaceQueryDto { weight_class: Some([700, 900]), ..Default::default() };
    let h = analysis::axis_histograms(&st, &q, 8).unwrap();
    let wc = h.iter().find(|x| x.axis == "weight_class").unwrap();
    assert_eq!(wc.bins.iter().sum::<u32>(), 4, "histogram weight bỏ chính bộ lọc weight");
    // miền của trục đo bỏ qua cả bộ lọc weight/width (cố định khi kéo), dù số đếm co lại
    let all = analysis::axis_histograms(&st, &FaceQueryDto::default(), 8).unwrap();
    let narrow = FaceQueryDto { weight_class: Some([100, 100]), var_axis_match: VarAxisMatchDto { wght: false, wdth: false }, ..Default::default() };
    let n = analysis::axis_histograms(&st, &narrow, 8).unwrap();
    let (a, b) = (all.iter().find(|x| x.axis == "stem").unwrap(), n.iter().find(|x| x.axis == "stem").unwrap());
    assert_eq!((a.lo, a.hi), (b.lo, b.hi));
    assert!(b.count < a.count);
    let stats = analysis::axis_stats(&st, &narrow).unwrap();
    assert_eq!(stats.iter().find(|x| x.axis == "stem").unwrap().count, a.count, "axis_stats cũng bỏ weight/width");
}

#[test]
fn outlier_class_values_validate_and_clamp_consistently() {
    let td = tempfile::tempdir().unwrap();
    let st = state(td.path());
    for bad in [[0, 900], [50, 900], [100, 1000], [900, 100]] {
        let q = FaceQueryDto { weight_class: Some(bad), ..Default::default() };
        assert!(library::query_faces(&st, &q).is_err(), "{bad:?}");
        assert!(analysis::axis_histograms(&st, &q, 8).is_err(), "{bad:?}");
    }
    let all = FaceQueryDto { weight_class: Some([100, 900]), var_axis_match: VarAxisMatchDto { wght: false, wdth: false }, ..Default::default() };
    assert_eq!(library::query_faces(&st, &all).unwrap().total, 4, "kéo hết 100–900 không làm mất font nào có class hợp lệ");
}

/// Đo trên thư viện tổng hợp 20k face: `cargo test -p app --release --test advanced_filters -- --ignored --nocapture perf`.
#[test]
#[ignore = "đo hiệu năng"]
fn perf_axis_histogram_20k_faces() {
    use fontatlas_store::{Analysis, IngestItem, ScannedFile, Store};
    let td = tempfile::tempdir().unwrap();
    let data = td.path().join("data");
    {
        let (mut s, _) = Store::open(&data).unwrap();
        let base = fontatlas_core::analyze_path(&fixture("public_sans_vf.ttf")).unwrap();
        for chunk in (0..20_000).collect::<Vec<u32>>().chunks(500) {
            let items: Vec<IngestItem> = chunk
                .iter()
                .map(|i| {
                    let mut r = base.clone();
                    let mut h = [0u8; 32];
                    h[..4].copy_from_slice(&i.to_le_bytes());
                    h[31] = 0xA5;
                    r.hash_hex = h.iter().map(|b| format!("{b:02x}")).collect();
                    for f in &mut r.faces {
                        f.metadata.family = Some(format!("Fam{}", i % 4000));
                        f.metadata.typographic_family = None;
                        f.metadata.weight_class = Some(100 + (i % 9) as u16 * 100);
                    }
                    IngestItem {
                        source_id: 1,
                        file: ScannedFile { path: format!("/perf/{i}.ttf").into(), size: 1, mtime_ns: 1, created_ns: None },
                        hash: h,
                        analysis: Analysis::Report(r),
                    }
                })
                .collect();
            s.ingest(items, 10).unwrap();
        }
    }
    let st = AppState::new(AppDirs::under(&data), Box::new(NoActivation));
    boot::run_boot(&st);
    assert!(wait_rebuilt(&st, Duration::from_secs(300)));
    let q = FaceQueryDto {
        weight_class: Some([600, 900]),
        raw_axis_ranges: vec![RawAxisRangeDto { axis: "stem".into(), range: [0.0, 10.0] }],
        ..Default::default()
    };
    let t = std::time::Instant::now();
    let h = analysis::axis_histograms(&st, &q, 24).unwrap();
    let ms = t.elapsed().as_millis();
    eprintln!("axis_histogram 20k faces: {ms} ms ({} trục)", h.len());
    assert_eq!(h.len(), 18);
    assert!(ms < 3000);
}
