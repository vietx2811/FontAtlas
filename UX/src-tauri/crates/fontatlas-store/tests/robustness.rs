mod common;
use common::*;
use fontatlas_store::{FaceQuery, SourceKind};

/// LCG seed cố định như Plan 1.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn corrupted_legacy_db_never_panics() {
    let d = tmp();
    let src = d.path().join("f");
    std::fs::create_dir_all(&src).unwrap();
    let old = d.path().join("old.db");
    make_legacy_db(&old, true, &src, &[LegacyFont {
        id: "A", path: "/x/a.ttf", family: "A", subfamily: "R", tags: "[]", collection: None, favorite: true, source: None,
    }]);
    let orig = std::fs::read(&old).unwrap();
    let mut rng = Lcg(0x0005_702E);
    for i in 0..40 {
        let mut b = orig.clone();
        for _ in 0..(1 + rng.next() % 64) {
            let k = (rng.next() as usize) % b.len();
            b[k] = (rng.next() & 0xFF) as u8;
        }
        let p = d.path().join(format!("bad{i}.db"));
        std::fs::write(&p, &b).unwrap();
        let mut s = open_store(&d.path().join(format!("db{i}")));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.import_legacy(&p)));
        assert!(r.is_ok(), "panic khi import DB cũ hỏng #{i}");
    }
}

#[test]
fn corrupted_fonts_in_scan_are_isolated() {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    let orig = std::fs::read(core_fixture("roboto.ttf")).unwrap();
    let mut rng = Lcg(0xF0A7);
    for i in 0..20 {
        let mut b = orig.clone();
        for _ in 0..16 {
            let k = (rng.next() as usize) % b.len();
            b[k] = (rng.next() & 0xFF) as u8;
        }
        b.truncate(((rng.next() as usize) % b.len()).max(1));
        std::fs::write(fonts.join(format!("x{i}.ttf")), b).unwrap();
    }
    std::fs::copy(core_fixture("open_sans.ttf"), fonts.join("ok.ttf")).unwrap();
    let mut s = open_store(&d.path().join("db"));
    let src = s.add_source(SourceKind::Directory, "F", Some(&fonts)).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.seen, 21);
    assert_eq!(st.analyzed + st.failed + st.relinked, 21);
    assert!(s.query_faces(&FaceQuery::default()).unwrap().items.iter().any(|r| r.family == "Open Sans"));
}

#[test]
#[ignore = "đo: cargo test -p fontatlas-store --release --test robustness -- --ignored --nocapture"]
fn perf_rescan_system_fonts() {
    let root = std::path::Path::new("/System/Library/Fonts");
    if !root.exists() {
        return;
    }
    let d = tmp();
    let mut s = open_store(d.path());
    let src = s.add_source(SourceKind::System, "System", Some(root)).unwrap();
    for lan in 1..=2 {
        let t = std::time::Instant::now();
        let st = s.rescan_source(src).unwrap();
        eprintln!("lần {lan}: {:?} trong {:?}", st, t.elapsed());
    }
}
