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
    storm("gap 350ms > quiet 200ms", 350);
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
    let dir = root.path().join("Big");
    std::fs::create_dir_all(&dir).unwrap();
    let n: usize = std::env::var("FA_SIM_N").ok().and_then(|v| v.parse().ok()).unwrap_or(600);
    for i in 0..n {
        std::fs::write(dir.join(format!("f{i}.ttf")), unique_font(i)).unwrap();
    }
    let lat: Arc<Mutex<Vec<(&'static str, u64)>>> = Arc::default();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (st2, lat2, stop2) = (st.clone(), lat.clone(), stop.clone());
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
            time("query_all", &|| drop(total(&st2)));
            time("query_star", &|| drop(library::query_faces(&st2, &FaceQueryDto { favorites_only: true, limit: 1, ..Default::default() })));
            time("duplicate_summary", &|| drop(duplicates::duplicate_summary(&st2, &FaceQueryDto::default())));
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
    eprintln!("[sidebar-vs-scan] {n} font quét trong {scan_s:.1}s; tổng thư viện sau quét = {}", total(&st));
    for nm in names {
        let mut xs: Vec<u64> = v.iter().filter(|x| x.0 == nm).map(|x| x.1).collect();
        xs.sort_unstable();
        eprintln!("  {nm}: {} lần, trung vị {} ms, p95 {} ms, max {} ms", xs.len(), xs[xs.len() / 2], xs[(xs.len() * 95 / 100).min(xs.len() - 1)], xs[xs.len() - 1]);
    }
}
