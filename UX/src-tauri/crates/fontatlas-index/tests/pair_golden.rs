//! Golden: thứ tự và nội dung top-k của pair_candidates trên 50k profile giả (seed cố định)
//! phải giữ nguyên khi tối ưu. Ghi lại golden: FA_BLESS=1 cargo test --release --test pair_golden.
mod common;
use common::*;
use fontatlas_index::{FaceProfile, Role};

/// FNV-1a 64 bit — ổn định giữa các phiên bản Rust.
fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn corpus(vi_holes: bool, embed: bool) -> Vec<FaceProfile> {
    let mut v = synthetic_50k(20261016);
    let mut s: u64 = 7;
    for (i, p) in v.iter_mut().enumerate() {
        if vi_holes && i % 7 == 3 {
            p.vi_missing = 1 + i % 50;
        }
        if embed && i % 3 != 0 {
            p.embedding = Some(
                (0..8)
                    .map(|_| {
                        s = s
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(1442695040888963407);
                        ((s >> 33) as f32 / (1u64 << 31) as f32) - 0.5
                    })
                    .collect(),
            );
        }
        // Vài họ có nhiều face để R2 (related) và family_facts có việc làm.
        if i % 11 == 0 {
            p.family = format!("Fam{}", i % 97);
            p.weight_class = Some(if i % 2 == 0 { 400 } else { 700 });
        }
        if i % 13 == 0 {
            p.designer = Some(format!("D{}", i % 5));
        }
    }
    v
}

fn snapshot() -> String {
    let mut out = String::new();
    for (vi_holes, embed) in [(false, false), (true, false), (false, true)] {
        let ix = index_of(corpus(vi_holes, embed));
        for q in ["F0", "F17", "F4242"] {
            for (rq, rc) in [
                (Role::Heading, Role::Body),
                (Role::Body, Role::Heading),
                (Role::Body, Role::Code),
            ] {
                for project_vi in [false, true] {
                    let qk = ix
                        .profiles()
                        .iter()
                        .find(|p| p.key.hash.starts_with(&format!("{q}/")))
                        .map(|p| p.key.clone())
                        .expect("query");
                    let v = ix
                        .pair_candidates(&qk, rq, rc, project_vi, 20)
                        .expect("pairs");
                    let keys: Vec<String> = v.iter().map(|(k, _)| k.hash.clone()).collect();
                    let full = serde_json::to_string(&v).expect("json");
                    out.push_str(&format!(
                        "{vi_holes},{embed},{q},{rq:?},{rc:?},{project_vi} n={} fnv={:016x} [{}]\n",
                        v.len(),
                        fnv(&full),
                        keys.join(" ")
                    ));
                }
            }
        }
    }
    out
}

#[test]
#[cfg_attr(debug_assertions, ignore = "50k: chạy ở --release")]
fn pair_candidates_top_k_unchanged() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pair_golden.txt");
    let now = snapshot();
    if std::env::var("FA_BLESS").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &now).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&path).expect("chưa có golden: chạy với FA_BLESS=1");
    for (i, (a, b)) in golden.lines().zip(now.lines()).enumerate() {
        assert_eq!(a, b, "dòng {i} khác golden");
    }
    assert_eq!(golden.lines().count(), now.lines().count());
}
