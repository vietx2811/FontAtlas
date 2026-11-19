//! Điều tra lỗi beta.3 (library trống thoáng qua, sidebar không hiện số ngay, library tự nạp lại): mô phỏng một nguồn cloud
//! đang tải font bằng thư mục tạm (đặt bằng `FA_SIM_DIR`, nên trỏ vào VXData). KHÔNG chạm thư mục cloud/Library thật.
//! Các test in số liệu ra stderr (`--nocapture`); khẳng định chỉ ghi lại hành vi hiện có để làm bằng chứng.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app_lib::dto::events::ScanEvent;
use app_lib::dto::library::*;
use app_lib::services::activation::NoActivation;
use app_lib::services::cloud_watch::CloudWatch;
use app_lib::services::{boot, duplicates, library, sources};
use app_lib::state::{AppDirs, AppState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/fontatlas-core/tests/fixtures").join(name)
}

fn sim_root() -> tempfile::TempDir {
    match std::env::var("FA_SIM_DIR") {
        Ok(d) => {
            std::fs::create_dir_all(&d).unwrap();
            tempfile::tempdir_in(d).unwrap()
        }
        Err(_) => tempfile::tempdir().unwrap(),
    }
}

const BASE: [&str; 6] = ["roboto.ttf", "open_sans.ttf", "bebas_neue.ttf", "chonburi.ttf", "miltonian.ttf", "playfair_display.ttf"];

/// Font "duy nhất" thứ `i`: bytes fixture + đuôi riêng (khác hash, vẫn parse được).
fn unique_font(i: usize) -> Vec<u8> {
    let mut b = std::fs::read(fixture(BASE[i % BASE.len()])).unwrap();
    b.extend(std::iter::repeat_n((i % 251) as u8, 16 + i));
    b
}

fn booted(root: &Path) -> Arc<AppState> {
    let st = AppState::new(AppDirs::under(&root.join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    st
}

fn total(st: &AppState) -> u32 {
    library::query_faces(st, &FaceQueryDto { limit: 1, ..Default::default() }).unwrap().total as u32
}

#[derive(Default)]
struct Counts {
    queued: AtomicU32,
    progress: AtomicU32,
    faces_added: AtomicU32,
    finished: AtomicU32,
    idle: AtomicU32,
    /// Finished mà FE bump nhưng không có gì đổi (analyzed+relinked+failed+missing = 0, seen = unchanged)
    finished_noop: AtomicU32,
}

fn count_events(st: &AppState) -> Arc<Counts> {
    let c = Arc::new(Counts::default());
    let c2 = c.clone();
    st.hubs.scan.subscribe(Box::new(move |e| match e {
        ScanEvent::Queued { .. } => {
            c2.queued.fetch_add(1, Ordering::SeqCst);
        }
        ScanEvent::Progress { .. } => {
            c2.progress.fetch_add(1, Ordering::SeqCst);
        }
        ScanEvent::FacesAdded { .. } => {
            c2.faces_added.fetch_add(1, Ordering::SeqCst);
        }
        ScanEvent::Finished { stats, .. } => {
            c2.finished.fetch_add(1, Ordering::SeqCst);
            if stats.analyzed + stats.relinked + stats.failed + stats.missing == 0 && stats.seen == stats.unchanged {
                c2.finished_noop.fetch_add(1, Ordering::SeqCst);
            }
        }
        ScanEvent::Idle => {
            c2.idle.fetch_add(1, Ordering::SeqCst);
        }
        _ => {}
    }));
    c
}

/// Luồng đọc liên tục như sidebar: ghi min `total` thấy được và độ trễ lớn nhất của một truy vấn.
struct Poller {
    stop: Arc<std::sync::atomic::AtomicBool>,
    min_total: Arc<AtomicU32>,
    max_ms: Arc<AtomicU64>,
    samples: Arc<AtomicU32>,
    h: Option<std::thread::JoinHandle<()>>,
}

fn poll(st: &Arc<AppState>) -> Poller {
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (min_total, max_ms, samples) = (Arc::new(AtomicU32::new(u32::MAX)), Arc::new(AtomicU64::new(0)), Arc::new(AtomicU32::new(0)));
    let (s, m, x, n, st) = (stop.clone(), min_total.clone(), max_ms.clone(), samples.clone(), st.clone());
    let h = std::thread::spawn(move || {
        while !s.load(Ordering::SeqCst) {
            let t = Instant::now();
            let tot = total(&st);
            x.fetch_max(t.elapsed().as_millis() as u64, Ordering::SeqCst);
            m.fetch_min(tot, Ordering::SeqCst);
            n.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(2));
        }
    });
    Poller { stop, min_total, max_ms, samples, h: Some(h) }
}

impl Poller {
    fn finish(mut self) -> (u32, u64, u32) {
        self.stop.store(true, Ordering::SeqCst);
        self.h.take().unwrap().join().unwrap();
        (self.min_total.load(Ordering::SeqCst), self.max_ms.load(Ordering::SeqCst), self.samples.load(Ordering::SeqCst))
    }
}

fn wait_until(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(secs) {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("hết {secs}s chờ: {what}");
}

/// Mô phỏng File Provider: stub `.Tên.ttf.icloud` xuất hiện, rồi bị thay bằng file thật (iCloud) — kèm một lượt
/// ghi tạm `.tmp` rồi đổi tên (kiểu Drive). `gap` = khoảng nghỉ giữa hai file.
fn download_burst(dir: &Path, from: usize, n: usize, gap: Duration) {
    for i in from..from + n {
        let name = format!("dl{i}.ttf");
        let stub = dir.join(format!(".{name}.icloud"));
        std::fs::write(&stub, b"stub").unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let tmp = dir.join(format!("{name}.tmp"));
        std::fs::write(&tmp, unique_font(i)).unwrap();
        std::fs::remove_file(&stub).unwrap();
        std::fs::rename(&tmp, dir.join(&name)).unwrap();
        std::thread::sleep(gap);
    }
}

fn seeded(root: &Path, n: usize) -> (Arc<AppState>, PathBuf) {
    let st = booted(root);
    let dir = root.join("Cloud");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..n {
        std::fs::write(dir.join(format!("base{i}.ttf")), unique_font(1000 + i)).unwrap();
    }
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(120)));
    (st, dir)
}

/// Q1 + Q2: 50 font tải xong lần lượt. `quiet` thu 1/6 so với app (200 ms ↔ 2 s) để test chạy nhanh;
/// `gap` > quiet = mỗi file một lượt quét (xấu nhất), `gap` < quiet = gom lại.
fn storm(label: &str, gap_ms: u64) {
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 40);
    let base = total(&st);
    assert!(base >= 40, "thư viện nền {base}");
    let w = CloudWatch::with_timing(Duration::from_millis(200), Duration::from_secs(5));
    w.sync(&st);
    std::thread::sleep(Duration::from_millis(800));
    let c = count_events(&st);
    let p = poll(&st);
    let t0 = Instant::now();
    download_burst(&dir, 0, 50, Duration::from_millis(gap_ms));
    wait_until("đủ 50 font", 90, || total(&st) >= base + 50);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    std::thread::sleep(Duration::from_millis(1200)); // lượt quét muộn do sự kiện đuôi
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let (min_total, max_ms, samples) = p.finish();
    w.shutdown();
    eprintln!(
        "[{label}] 50 file/{:.1}s · queued={} progress={} facesAdded={} finished={} (không đổi gì={}) idle={} · \
         total nền={base} min thấy được={min_total} ({samples} mẫu, truy vấn chậm nhất {max_ms} ms)",
        t0.elapsed().as_secs_f32(),
        c.queued.load(Ordering::SeqCst),
        c.progress.load(Ordering::SeqCst),
        c.faces_added.load(Ordering::SeqCst),
        c.finished.load(Ordering::SeqCst),
        c.finished_noop.load(Ordering::SeqCst),
        c.idle.load(Ordering::SeqCst),
    );
    assert!(min_total >= base, "library không được trống/ít đi trong lúc quét: min={min_total} < nền={base}");
}

#[test]
fn q1_fifty_downloads_slower_than_quiet_each_trigger_a_whole_source_scan() {
    storm("gap 350ms > quiet 200ms", 350); // quiet thu nhỏ; stub .icloud → chờ ×4 (BE-4): gom lại
}

/// BE-4 với thời gian THẬT của app (quiet 2 s, cloud 8 s, max 30 s): 50 file cách 350 ms → tối đa 3 lượt quét.
#[test]
fn be4_fifty_icloud_downloads_350ms_apart_make_at_most_three_scans_with_real_timings() {
    use app_lib::services::cloud_watch::{MAX_WAIT, QUIET};
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 20);
    let base = total(&st);
    let w = CloudWatch::with_timing(QUIET, MAX_WAIT);
    w.sync(&st);
    std::thread::sleep(Duration::from_millis(800));
    let c = count_events(&st);
    download_burst(&dir, 0, 50, Duration::from_millis(330)); // + 20 ms stub = 350 ms/file
    wait_until("đủ 50 font", 90, || total(&st) >= base + 50);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let q = c.queued.load(Ordering::SeqCst);
    w.shutdown();
    eprintln!("[be4 real] queued={q} finished={}", c.finished.load(Ordering::SeqCst));
    assert!((1..=3).contains(&q), "50 file/350 ms phải ra ≤ 3 lượt, ra {q}");
}

#[test]
fn q1_fifty_downloads_faster_than_quiet_coalesce() {
    storm("gap 80ms < quiet 200ms", 80);
}

/// Một lượt quét không có gì đổi vẫn phát Queued/Progress/Finished/Idle → FE bump (tải lại lưới + sidebar).
#[test]
fn q1_scan_with_nothing_changed_still_emits_finished_which_the_ui_bumps_on() {
    let root = sim_root();
    let (st, _dir) = seeded(root.path(), 20);
    let c = count_events(&st);
    for _ in 0..3 {
        assert!(st.scans.enqueue_follow_up(&st, &[1]));
        assert!(st.scans.wait_idle(Duration::from_secs(60)));
    }
    eprintln!(
        "[noop] 3 lượt không đổi gì: queued={} progress={} facesAdded={} finished={} (không đổi gì={}) idle={}",
        c.queued.load(Ordering::SeqCst), c.progress.load(Ordering::SeqCst), c.faces_added.load(Ordering::SeqCst),
        c.finished.load(Ordering::SeqCst), c.finished_noop.load(Ordering::SeqCst), c.idle.load(Ordering::SeqCst),
    );
    assert_eq!(c.finished.load(Ordering::SeqCst), 3);
    assert_eq!(c.finished_noop.load(Ordering::SeqCst), 3);
}

/// Q2: file bị ghi đè tại chỗ (tải dở trên cùng đường dẫn): face cũ có bị ẩn trước khi bản đủ về không?
#[test]
fn q2_partial_overwrite_in_place_hides_the_face_until_the_complete_file_is_scanned() {
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 5);
    let base = total(&st);
    let victim = dir.join("base0.ttf");
    let full = std::fs::read(&victim).unwrap();
    std::fs::write(&victim, &full[..full.len() / 2]).unwrap(); // đang tải dở
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let mid = total(&st);
    std::fs::write(&victim, &full).unwrap(); // tải xong
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let end = total(&st);
    eprintln!("[partial] total: nền={base} · lúc file dở={mid} · sau khi đủ={end}");
    assert_eq!(end, base);
}

/// Q2: file biến mất khỏi thư mục trong lúc quét (stub → file thật qua hai bước có khoảng hở) bị đánh missing → ẩn?
#[test]
fn q2_file_absent_during_walk_is_hidden_then_restored_by_next_scan() {
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 5);
    let base = total(&st);
    let f = dir.join("base1.ttf");
    let keep = std::fs::read(&f).unwrap();
    std::fs::remove_file(&f).unwrap(); // khoảng hở: không stub, không file thật
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let gap = total(&st);
    std::fs::write(&f, &keep).unwrap();
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let back = total(&st);
    eprintln!("[absent] total: nền={base} · lúc file vắng={gap} · sau khi có lại={back}");
    assert_eq!(back, base);
}

/// Q2 (ngược lại): stub .icloud đứng một mình (file cloud chưa tải) KHÔNG bị coi là đã xoá.
#[test]
fn q2_icloud_stub_of_a_known_font_is_not_treated_as_deleted() {
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 5);
    let base = total(&st);
    let f = dir.join("base2.ttf");
    std::fs::remove_file(&f).unwrap();
    std::fs::write(dir.join(".base2.ttf.icloud"), b"stub").unwrap(); // iCloud đã dời nội dung lên cloud
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let stubbed = total(&st);
    eprintln!("[stub] total: nền={base} · khi chỉ còn stub={stubbed}");
    assert_eq!(stubbed, base);
}

/// Q3: các lệnh sidebar (đếm all/⭐/trùng lặp, nguồn) chạy song song với một lượt quét lớn + vòng bump: độ trễ.
#[test]
fn q3_sidebar_commands_while_a_large_scan_is_running() {
    let root = sim_root();
    let st = booted(root.path());
    // FA_SIM_SRC: quét một thư mục font có sẵn (chỉ ĐỌC, vd bản sao thư viện QA); mặc định sinh FA_SIM_N font tổng hợp
    let n: usize = std::env::var("FA_SIM_N").ok().and_then(|v| v.parse().ok()).unwrap_or(600);
    let dir = match std::env::var("FA_SIM_SRC") {
        Ok(d) => PathBuf::from(d),
        Err(_) => {
            let dir = root.path().join("Big");
            std::fs::create_dir_all(&dir).unwrap();
            for i in 0..n {
                std::fs::write(dir.join(format!("f{i}.ttf")), unique_font(i)).unwrap();
            }
            dir
        }
    };
    let lat: Arc<Mutex<Vec<(&'static str, u64)>>> = Arc::default();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let errs = Arc::new(AtomicU32::new(0));
    let (st2, lat2, stop2, errs2) = (st.clone(), lat.clone(), stop.clone(), errs.clone());
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    let h = std::thread::spawn(move || {
        let time = |name: &'static str, f: &dyn Fn()| {
            let t = Instant::now();
            f();
            lat2.lock().unwrap().push((name, t.elapsed().as_millis() as u64));
        };
        while !stop2.load(Ordering::SeqCst) {
            // = Sidebar.refresh() (Promise.all: sidebar chờ cái chậm nhất)
            let t = Instant::now();
            time("list_sources", &|| drop(sources::list_sources(&st2)));
            time("name_clashes", &|| drop(library::name_clashes(&st2)));
            time("query_all", &|| { let _ = total(&st2); });
            time("query_star", &|| drop(library::query_faces(&st2, &FaceQueryDto { favorites_only: true, limit: 1, ..Default::default() })));
            time("duplicate_summary", &|| {
                if duplicates::duplicate_summary(&st2, &FaceQueryDto::default()).is_err() {
                    errs2.fetch_add(1, Ordering::SeqCst); // vd "Index chưa dựng xong"
                }
            });
            lat2.lock().unwrap().push(("SIDEBAR_REFRESH_TOTAL", t.elapsed().as_millis() as u64));
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    let t0 = Instant::now();
    assert!(st.scans.wait_idle(Duration::from_secs(300)));
    let scan_s = t0.elapsed().as_secs_f32();
    stop.store(true, Ordering::SeqCst);
    h.join().unwrap();
    let v = lat.lock().unwrap().clone();
    let mut names: Vec<&str> = v.iter().map(|x| x.0).collect();
    names.sort();
    names.dedup();
    eprintln!("[sidebar-vs-scan] duplicate_summary trả lỗi (Index chưa dựng) {} lần trong lúc quét", errs.load(Ordering::SeqCst));
    assert!(app_lib::index_bundle::wait_rebuilt(&st, Duration::from_secs(120)));
    let t = Instant::now();
    let first = duplicates::duplicate_summary(&st, &FaceQueryDto::default());
    eprintln!("  [lúc yên] duplicate_summary lần ĐẦU (cache lạnh): {} ms, ok={}", t.elapsed().as_millis(), first.is_ok());
    for (nm, f) in [
        ("list_sources", Box::new(|| drop(sources::list_sources(&st))) as Box<dyn Fn()>),
        ("duplicate_summary", Box::new(|| drop(duplicates::duplicate_summary(&st, &FaceQueryDto::default())))),
        ("name_clashes", Box::new(|| drop(library::name_clashes(&st)))),
        ("query_all", Box::new(|| { let _ = total(&st); })),
    ] {
        let mut xs: Vec<u128> = (0..5).map(|_| { let t = Instant::now(); f(); t.elapsed().as_millis() }).collect();
        xs.sort_unstable();
        eprintln!("  [lúc yên] {nm}: trung vị {} ms, max {} ms", xs[2], xs[4]);
    }
    eprintln!("[sidebar-vs-scan] nguồn {} quét trong {scan_s:.1}s; tổng thư viện sau quét = {}", dir.display(), total(&st));
    for nm in names {
        let mut xs: Vec<u64> = v.iter().filter(|x| x.0 == nm).map(|x| x.1).collect();
        xs.sort_unstable();
        eprintln!("  {nm}: {} lần, trung vị {} ms, p95 {} ms, max {} ms", xs.len(), xs[xs.len() / 2], xs[(xs.len() * 95 / 100).min(xs.len() - 1)], xs[xs.len() - 1]);
    }
}

fn record(st: &AppState) -> Arc<Mutex<Vec<ScanEvent>>> {
    let v: Arc<Mutex<Vec<ScanEvent>>> = Arc::default();
    let v2 = v.clone();
    st.hubs.scan.subscribe(Box::new(move |e| v2.lock().unwrap().push(e)));
    v
}

#[test]
fn be1_finished_reports_changed_false_for_an_unchanged_scan_and_true_when_a_font_arrives() {
    use app_lib::dto::events::ScanTrigger;
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 8);
    let ev = record(&st);
    assert!(st.scans.enqueue_follow_up_as(&st, &[1], ScanTrigger::Watcher));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    std::fs::write(dir.join("new.ttf"), unique_font(77)).unwrap();
    assert!(st.scans.enqueue_follow_up_as(&st, &[1], ScanTrigger::Watcher));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let fin: Vec<(bool, u32)> = ev
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| if let ScanEvent::Finished { changed, stats, .. } = e { Some((*changed, stats.purged)) } else { None })
        .collect();
    assert_eq!(fin, vec![(false, 0), (true, 0)], "quét không đổi → changed=false; có font mới → true");
}

/// Case D của QA: ghi đè file bằng đúng nội dung cũ (chỉ mtime đổi) → thư viện không đổi → changed=false;
/// một bản sao mới của font đã có (vị trí mới, cùng hash) là thay đổi thật → changed=true.
#[test]
fn be1_rewriting_a_font_with_identical_bytes_is_not_a_change_but_a_new_copy_is() {
    use app_lib::dto::events::ScanTrigger;
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 6);
    let ev = record(&st);
    let first = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "ttf")).unwrap();
    let bytes = std::fs::read(&first).unwrap();
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(30));
        std::fs::write(&first, &bytes).unwrap(); // cùng nội dung, mtime mới
        assert!(st.scans.enqueue_follow_up_as(&st, &[1], ScanTrigger::Watcher));
        assert!(st.scans.wait_idle(Duration::from_secs(60)));
    }
    std::fs::write(dir.join("copy-of-first.ttf"), &bytes).unwrap();
    assert!(st.scans.enqueue_follow_up_as(&st, &[1], ScanTrigger::Watcher));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let fin: Vec<bool> = ev.lock().unwrap().iter().filter_map(|e| if let ScanEvent::Finished { changed, .. } = e { Some(*changed) } else { None }).collect();
    for e in ev.lock().unwrap().iter() {
        if let ScanEvent::Finished { .. } = e {
            println!("[d-sample] {}", serde_json::to_string(e).unwrap());
        }
    }
    assert_eq!(fin, vec![false, false, false, true], "ghi đè cùng byte ×3 → false; bản sao mới → true");
}

/// TaskHub: một lượt quét hiện thành việc trong khay (chờ → chạy → gỡ), rev tăng đơn điệu, hết việc thì danh sách rỗng;
/// huỷ việc đang chờ gỡ nó ngay và nguồn đó không bị quét.
#[test]
fn taskhub_a_scan_shows_up_runs_and_disappears_and_a_queued_one_can_be_cancelled() {
    use app_lib::dto::events::ScanTrigger;
    use app_lib::dto::tasks::{TaskEvent, TaskKind, TaskOutcome, TaskState};
    let root = sim_root();
    let (st, _dir) = seeded(root.path(), 6);
    let evs: Arc<Mutex<Vec<TaskEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let e2 = evs.clone();
    st.tasks.events.subscribe(Box::new(move |e| e2.lock().unwrap().push(e)));
    // nguồn 2 giữ luồng quét (đang tạm dừng) để nguồn 1 nằm chờ
    let dir2 = root.path().join("Hold");
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir2.join("h.ttf"), unique_font(901)).unwrap();
    sources::add_source(&st, dir2.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(st.tasks.list().tasks.is_empty(), "hết việc → khay rỗng");
    evs.lock().unwrap().clear();

    st.scans.pause(&st);
    st.scans.enqueue_as(&st, &[2], ScanTrigger::User);
    wait_until("nguồn 2 đang quét", 10, || st.scans.is_current(2));
    assert!(st.scans.enqueue_follow_up_as(&st, &[1], ScanTrigger::Watcher));
    let l = st.tasks.list();
    let states: Vec<(TaskState, ScanTrigger)> = l.tasks.iter().map(|t| (t.state, t.trigger)).collect();
    assert!(states.contains(&(TaskState::Queued, ScanTrigger::Watcher)), "{states:?}");
    assert!(l.tasks.iter().all(|t| t.kind == TaskKind::Scan && t.cancellable));
    // tiêu đề chỉ có tên ngắn, không đường dẫn đầy đủ
    for t in &l.tasks {
        let a = serde_json::to_string(&t.title_args).unwrap();
        assert!(!a.contains('/') && !a.contains("tmp"), "{a}");
    }
    // huỷ việc đang chờ (nguồn 1): gỡ ngay, không quét nguồn 1
    let queued = l.tasks.iter().find(|t| t.state == TaskState::Queued).unwrap().id.clone();
    assert!(st.tasks.cancel(&queued));
    assert!(!st.tasks.list().tasks.iter().any(|t| t.id == queued), "việc chờ bị huỷ phải biến mất ngay");
    assert!(!st.scans.is_queued(1));
    st.scans.resume(&st);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert!(st.tasks.list().tasks.is_empty());
    let ev = evs.lock().unwrap();
    let revs: Vec<u64> = ev.iter().map(|e| match e { TaskEvent::Upsert { task } => task.rev, TaskEvent::Removed { rev, .. } => *rev }).collect();
    assert!(revs.windows(2).all(|w| w[0] < w[1]), "rev đơn điệu: {revs:?}");
    let outcomes: Vec<TaskOutcome> = ev.iter().filter_map(|e| if let TaskEvent::Removed { outcome, .. } = e { Some(*outcome) } else { None }).collect();
    assert!(outcomes.contains(&TaskOutcome::Cancelled) && outcomes.contains(&TaskOutcome::Done), "{outcomes:?}");
}

/// Tiến độ việc quét đơn điệu: pha đếm total=null (không mượn thang pha phân tích), done không bao giờ lùi trong một việc.
#[test]
fn taskhub_scan_progress_never_goes_backwards_and_counting_has_unknown_total() {
    use app_lib::dto::tasks::TaskEvent;
    let root = sim_root();
    let st = booted(root.path());
    let dir = root.path().join("Fonts");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..600 {
        std::fs::write(dir.join(format!("f{i}.ttf")), unique_font(5000 + i)).unwrap();
    }
    let evs: Arc<Mutex<Vec<TaskEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let e2 = evs.clone();
    st.tasks.events.subscribe(Box::new(move |e| e2.lock().unwrap().push(e)));
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(120)));
    let mut last: Option<(u64, Option<u64>)> = None;
    let mut seen_unknown = false;
    for e in evs.lock().unwrap().iter() {
        if let TaskEvent::Upsert { task } = e {
            if task.total.is_none() {
                seen_unknown = true;
                assert_eq!(task.done, 0, "pha đếm không báo done");
            }
            if let Some((d, t)) = last
                && t == task.total
                && task.total.is_some()
            {
                assert!(task.done >= d, "done lùi: {d} → {} (total {:?})", task.done, task.total);
            }
            last = Some((task.done, task.total));
        }
    }
    assert!(seen_unknown, "phải có giai đoạn total=null");
}

/// Review: huỷ việc đang chờ trả đúng trường hợp đã xử lý; việc "ma" không kẹt khi huỷ/xếp đua nhau; UI luôn nhận Idle cuối.
#[test]
fn taskhub_cancel_outcomes_are_exact_and_racing_enqueue_cancel_leaves_no_ghost_task() {
    use app_lib::dto::events::ScanTrigger;
    use app_lib::services::scan::CancelOutcome;
    let root = sim_root();
    let (st, _dir) = seeded(root.path(), 4);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(st.scans.cancel_source(&st, 1), CancelOutcome::Nothing);
    let ev = record(&st);
    let mut handles = Vec::new();
    for t in 0..4 {
        let st2 = st.clone();
        handles.push(std::thread::spawn(move || {
            for i in 0..60 {
                if (i + t) % 2 == 0 {
                    st2.scans.enqueue_follow_up_as(&st2, &[1], ScanTrigger::Watcher);
                } else {
                    let _ = st2.scans.cancel_source(&st2, 1);
                }
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert!(st.scans.wait_idle(Duration::from_secs(120)));
    assert!(st.tasks.list().tasks.is_empty(), "không còn việc ma: {:?}", st.tasks.list().tasks);
    let evs = ev.lock().unwrap();
    // mỗi Queued của nguồn phải được khép bởi Finished/Failed hoặc bị huỷ rồi có Idle sau cùng
    assert!(matches!(evs.last(), Some(ScanEvent::Idle)) || evs.is_empty() || evs.iter().rev().any(|e| matches!(e, ScanEvent::Idle)), "UI phải nhận Idle");
}

#[test]
fn be2_every_scan_event_carries_who_triggered_it_and_a_user_request_outranks_a_queued_watcher_one() {
    use app_lib::dto::events::ScanTrigger;
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 8);
    std::fs::write(dir.join("new.ttf"), unique_font(78)).unwrap();
    let ev = record(&st);
    assert!(st.scans.enqueue_follow_up_as(&st, &[1], ScanTrigger::Watcher));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let trig = |e: &ScanEvent| match e {
        ScanEvent::Queued { trigger, .. } | ScanEvent::Progress { trigger, .. } | ScanEvent::FacesAdded { trigger, .. } | ScanEvent::Finished { trigger, .. } => Some(*trigger),
        _ => None,
    };
    let got: Vec<ScanTrigger> = ev.lock().unwrap().iter().filter_map(trig).collect();
    assert!(got.len() >= 4 && got.iter().all(|t| *t == ScanTrigger::Watcher), "{got:?}");

    // người dùng xếp thêm khi lượt watcher còn chờ: lượt đó thành của người dùng (không xếp trùng).
    // Nguồn 2 chiếm luồng quét (đang tạm dừng) để nguồn 1 nằm chờ trong hàng.
    let dir2 = root.path().join("Cloud2");
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir2.join("x.ttf"), unique_font(79)).unwrap();
    sources::add_source(&st, dir2.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    st.scans.pause(&st);
    ev.lock().unwrap().clear();
    st.scans.enqueue_as(&st, &[2], ScanTrigger::User);
    wait_until("nguồn 2 đang quét", 10, || st.scans.is_current(2));
    assert!(st.scans.enqueue_follow_up_as(&st, &[1], ScanTrigger::Watcher));
    st.scans.enqueue_as(&st, &[1], ScanTrigger::User);
    st.scans.resume(&st);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let queued: Vec<(i64, ScanTrigger)> = ev.lock().unwrap().iter().filter_map(|e| if let ScanEvent::Queued { source_id, trigger } = e { Some((*source_id, *trigger)) } else { None }).collect();
    assert_eq!(queued, vec![(2, ScanTrigger::User), (1, ScanTrigger::Watcher)], "mỗi nguồn một Queued");
    let fin: Vec<(i64, ScanTrigger)> = ev.lock().unwrap().iter().filter_map(|e| if let ScanEvent::Finished { source_id, trigger, .. } = e { Some((*source_id, *trigger)) } else { None }).collect();
    assert_eq!(fin, vec![(2, ScanTrigger::User), (1, ScanTrigger::User)], "…nhưng lượt chạy của nguồn 1 là của người dùng");
}

/// BE-3a, chiều 1: tạm vắng (stub↔file thật) KHÔNG làm ẩn face; vắng ở lượt xác nhận (và chỉ khi đó) mới ẩn.
#[test]
fn be3a_a_font_absent_for_one_scan_stays_visible_and_a_real_delete_disappears_after_the_second() {
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 5);
    let base = total(&st);
    let f = dir.join("base1.ttf");
    let keep = std::fs::read(&f).unwrap();

    // tạm vắng: lượt 1 thấy vắng, file về trước lượt 2 → không bao giờ ẩn
    std::fs::remove_file(&f).unwrap();
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(total(&st), base, "vắng lần đầu: chưa ẩn");
    std::fs::write(&f, &keep).unwrap();
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(total(&st), base, "file về trước lượt xác nhận");

    // xoá thật: lượt 1 chưa ẩn, lượt 2 (xác nhận) ẩn
    std::fs::remove_file(&f).unwrap();
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(total(&st), base, "xoá thật, lượt 1: chưa ẩn");
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    assert_eq!(total(&st), base - 1, "xoá thật, lượt 2: ẩn");
}

/// BE-3a, chiều 2: không ai xếp lượt thứ hai thì ScanManager tự xếp lượt xác nhận (trigger watcher): xoá thật vẫn biến mất.
#[test]
fn be3a_the_manager_verifies_by_itself_so_a_real_delete_still_disappears_without_another_event() {
    use app_lib::dto::events::ScanTrigger;
    let root = sim_root();
    let (st, dir) = seeded(root.path(), 5);
    let base = total(&st);
    std::fs::remove_file(dir.join("base3.ttf")).unwrap();
    let ev = record(&st);
    assert!(st.scans.enqueue_follow_up(&st, &[1]));
    wait_until("font đã xoá biến mất sau lượt xác nhận", 30, || total(&st) == base - 1);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let v = ev.lock().unwrap();
    let fin: Vec<(bool, ScanTrigger)> = v.iter().filter_map(|e| if let ScanEvent::Finished { changed, trigger, .. } = e { Some((*changed, *trigger)) } else { None }).collect();
    assert_eq!(fin, vec![(false, ScanTrigger::User), (true, ScanTrigger::Watcher)], "lượt 1 không đổi gì; lượt xác nhận ẩn font");
}

/// Đo `duplicate_summary` trên thư viện tổng hợp lớn (mặc định bỏ qua; `FA_SIM_BIG=1` để chạy):
/// 5000 font khác hash, 500 bản sao cùng file (nhóm SameFile); các font chung 6 họ tên nên còn có nhóm Versions lớn.
#[test]
fn perf_duplicate_summary_on_5k_faces_with_500_duplicate_groups() {
    if std::env::var("FA_SIM_BIG").is_err() {
        eprintln!("[bỏ qua] đặt FA_SIM_BIG=1 để đo duplicate_summary trên 5k face");
        return;
    }
    let root = sim_root();
    let st = booted(root.path());
    let dir = root.path().join("Big");
    let dup = root.path().join("BigCopies");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&dup).unwrap();
    for i in 0..5000 {
        let b = unique_font(i);
        std::fs::write(dir.join(format!("f{i}.ttf")), &b).unwrap();
        if i < 500 {
            std::fs::write(dup.join(format!("c{i}.ttf")), &b).unwrap();
        }
    }
    let t = Instant::now();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    sources::add_source(&st, dup.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(1800)));
    eprintln!("[big] quét xong {} face trong {:.1}s", total(&st), t.elapsed().as_secs_f32());
    let mut ms = Vec::new();
    let mut last = None;
    for _ in 0..5 {
        let t = Instant::now();
        let s = duplicates::duplicate_summary(&st, &FaceQueryDto::default()).unwrap();
        ms.push(t.elapsed().as_millis());
        last = Some(s);
    }
    let s = last.unwrap();
    eprintln!("[big] duplicate_summary: {ms:?} ms · sameFile={} versions={} formats={} total={}", s.same_file, s.versions, s.formats, s.total);
    // đồng thời với một lượt quét không đổi gì (bump kéo theo): độ trễ khi tranh khoá
    let st2 = st.clone();
    assert!(st.scans.enqueue_follow_up(&st, &[1, 2]));
    let t = Instant::now();
    let mut worst = 0u128;
    while st2.scans.is_busy() && t.elapsed() < Duration::from_secs(300) {
        let a = Instant::now();
        let _ = duplicates::duplicate_summary(&st2, &FaceQueryDto::default()).unwrap();
        worst = worst.max(a.elapsed().as_millis());
    }
    eprintln!("[big] duplicate_summary trong lúc quét lại không đổi gì: chậm nhất {worst} ms");
}

/// BE-4 mở rộng: nguồn nằm dưới `Library/CloudStorage/<Hãng>` (File Provider) được coi là cloud dù file về không qua stub;
/// cùng kịch bản trên thư mục cục bộ thì mỗi file một lượt quét. Dùng tên thư mục giả, KHÔNG đụng cloud thật.
fn plain_burst_scans(sub: &str) -> u32 {
    let root = sim_root();
    let st = booted(root.path());
    let dir = root.path().join(sub);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("base.ttf"), unique_font(2000)).unwrap();
    sources::add_source(&st, dir.to_str().unwrap()).unwrap();
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    let w = CloudWatch::with_timing(Duration::from_millis(200), Duration::from_secs(10)); // 200 ms ↔ 2 s; cloud ×4 = 800 ms
    w.sync(&st);
    std::thread::sleep(Duration::from_millis(800));
    let c = count_events(&st);
    for i in 0..12 {
        std::fs::write(dir.join(format!("p{i}.ttf")), unique_font(3000 + i)).unwrap(); // không stub, không tmp
        std::thread::sleep(Duration::from_millis(450));
    }
    wait_until("đủ font", 60, || total(&st) >= 13);
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    std::thread::sleep(Duration::from_millis(1500));
    assert!(st.scans.wait_idle(Duration::from_secs(60)));
    w.shutdown();
    c.queued.load(Ordering::SeqCst)
}

#[test]
fn be4_a_cloudstorage_path_source_waits_longer_than_a_local_folder() {
    let local = plain_burst_scans("Fonts");
    let cloud = plain_burst_scans("Library/CloudStorage/OneDrive-Test/Fonts");
    eprintln!("[be4 path] 12 file cách 450 ms: thư mục cục bộ = {local} lượt, CloudStorage = {cloud} lượt");
    // Không đặt ngưỡng tuyệt đối cho cục bộ: backoff theo cơn bão (≥3 lượt/60 s → quiet giãn) và tải máy làm số này dao động.
    // Ý đồ chỉ là: nguồn CloudStorage gom lại và ít lượt quét hơn hẳn thư mục thường.
    assert!(local >= 2, "cục bộ vẫn quét nhiều lần ({local})");
    assert!(cloud <= 3, "CloudStorage: gom lại ({cloud})");
    assert!(cloud < local, "CloudStorage ({cloud}) phải ít lượt hơn cục bộ ({local})");
}

/// Khởi động với thư viện lớn có sẵn (đã quét một lần): thời gian `run_boot`, rồi tới lúc ba số sidebar đọc được, trong khi
/// lượt quét lúc mở app (rescanOnLaunch) đang chạy. `FA_SIM_SRC` = thư mục font có sẵn (chỉ đọc).
#[test]
fn q4_startup_time_until_the_sidebar_numbers_are_readable_on_a_large_library() {
    let root = sim_root();
    let src = match std::env::var("FA_SIM_SRC") {
        Ok(d) => PathBuf::from(d),
        Err(_) => {
            let d = root.path().join("Big");
            std::fs::create_dir_all(&d).unwrap();
            for i in 0..600 {
                std::fs::write(d.join(format!("f{i}.ttf")), unique_font(i)).unwrap();
            }
            d
        }
    };
    {
        let st = booted(root.path());
        sources::add_source(&st, src.to_str().unwrap()).unwrap();
        assert!(st.scans.wait_idle(Duration::from_secs(600)));
        assert!(app_lib::index_bundle::wait_rebuilt(&st, Duration::from_secs(120)));
        st.scans.shutdown();
    }
    let t0 = Instant::now();
    let st = AppState::new(AppDirs::under(&root.path().join("data")), Box::new(NoActivation));
    boot::run_boot(&st);
    let boot_ms = t0.elapsed().as_millis();
    let mut first = None;
    let mut nums = None;
    while nums.is_none() && t0.elapsed() < Duration::from_secs(60) {
        let t = Instant::now();
        let ok = sources::list_sources(&st).is_ok()
            && library::query_faces(&st, &FaceQueryDto { limit: 1, ..Default::default() }).is_ok()
            && duplicates::duplicate_summary(&st, &FaceQueryDto::default()).is_ok();
        if first.is_none() {
            first = Some((t0.elapsed().as_millis(), t.elapsed().as_millis()));
        }
        if ok {
            nums = Some(t0.elapsed().as_millis());
        } else {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    eprintln!("[startup] run_boot={boot_ms} ms · lệnh sidebar đầu tiên tại {:?} (mốc, độ trễ) ms · đủ 3 số sidebar tại {:?} ms", first, nums);
    assert!(st.scans.wait_idle(Duration::from_secs(600)));
}
