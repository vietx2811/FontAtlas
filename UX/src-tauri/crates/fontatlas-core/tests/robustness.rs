mod common;
use common::*;
use fontatlas_core::analyze_bytes;

/// LCG có seed cố định để test lặp lại được.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn random_byte_corruption_never_panics() {
    let mut rng = Lcg(0x0F0A_71A5);
    for s in SAMPLES.iter().chain(["open_sans.woff2"].iter()) {
        let orig = fixture_bytes(s);
        for _ in 0..60 {
            let mut d = orig.clone();
            let flips = 1 + (rng.next() % 32) as usize;
            for _ in 0..flips {
                let i = (rng.next() as usize) % d.len();
                d[i] = (rng.next() & 0xFF) as u8;
            }
            if rng.next() % 4 == 0 {
                let cut = (rng.next() as usize) % d.len();
                d.truncate(cut.max(1));
            }
            let res = std::panic::catch_unwind(|| analyze_bytes(d));
            assert!(res.is_ok(), "{s}: panic trên dữ liệu hỏng");
        }
    }
}
