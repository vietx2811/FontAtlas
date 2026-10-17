#![allow(dead_code)]
use std::path::{Path, PathBuf};

pub const SAMPLES: [&str; 5] = [
    "roboto.ttf",
    "open_sans.ttf",
    "playfair_display.ttf",
    "libre_baskerville.ttf",
    "source_code_pro.ttf",
];

/// Fixture dùng chung với fontatlas-core.
pub fn core_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fontatlas-core/tests/fixtures")
        .join(name)
}

pub fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// (độ dài, mtime, BLAKE3 nội dung) để chứng minh một file không bị ghi; None nếu chưa tồn tại.
pub fn fingerprint(p: &Path) -> Option<(u64, std::time::SystemTime, String)> {
    let m = std::fs::metadata(p).ok()?;
    let h = blake3::hash(&std::fs::read(p).ok()?).to_hex().to_string();
    Some((m.len(), m.modified().ok()?, h))
}

use fontatlas_core::{FileReport, analyze_path};
use fontatlas_store::{Analysis, IngestItem, ScannedFile, Store};

pub fn report(name: &str) -> FileReport {
    analyze_path(&core_fixture(name)).expect("fixture phân tích được")
}

/// Bản sao report với hash giả và tên family mới — để có nhiều face mà không cần file thật.
pub fn fake_report(base: &FileReport, seed: &str, family: &str) -> FileReport {
    let mut r = base.clone();
    r.hash_hex = blake3::hash(seed.as_bytes()).to_hex().to_string();
    for f in &mut r.faces {
        f.metadata.family = Some(family.to_string());
        f.metadata.typographic_family = None;
        f.metadata.full_name = Some(format!("{family} Regular"));
    }
    r
}

pub fn hash_of(r: &FileReport) -> [u8; 32] {
    *blake3::Hash::from_hex(&r.hash_hex).expect("hex").as_bytes()
}

pub fn item(source_id: i64, path: &str, r: &FileReport) -> IngestItem {
    IngestItem {
        source_id,
        file: ScannedFile { path: path.into(), size: 1, mtime_ns: 1 },
        hash: hash_of(r),
        analysis: Analysis::Report(r.clone()),
    }
}

pub fn open_store(dir: &Path) -> Store {
    Store::open(dir).expect("open").0
}

pub fn count(s: &Store, sql: &str) -> i64 {
    s.conn().query_row(sql, [], |r| r.get(0)).expect(sql)
}

/// Chỉ mục FTS khớp bảng nội dung (lỗi = hỏng) — count(*) của FTS external content đọc từ faces nên vô dụng.
pub fn fts_ok(s: &Store) {
    s.conn()
        .execute("INSERT INTO faces_fts(faces_fts) VALUES('integrity-check')", [])
        .expect("FTS integrity-check");
}

pub fn fts_hits(s: &Store, phrase: &str) -> i64 {
    s.conn()
        .query_row("SELECT count(*) FROM faces_fts WHERE faces_fts MATCH ?1", [format!("\"{phrase}\"")], |r| r.get(0))
        .expect("MATCH")
}
