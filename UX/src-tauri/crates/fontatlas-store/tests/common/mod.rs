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
        file: ScannedFile { path: path.into(), size: 1, mtime_ns: 1, created_ns: None },
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

pub const LEGACY_FONTS_DDL: &str = "CREATE TABLE fonts (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, file_path TEXT NOT NULL UNIQUE, original_directory TEXT,
    category TEXT, classification TEXT, tags TEXT, collection_id TEXT, date_added TEXT NOT NULL,
    date_modified TEXT NOT NULL, is_synced INTEGER DEFAULT 0, file_exists INTEGER DEFAULT 1,
    x_height REAL, width_variance REAL, stroke_contrast REAL, aperture REAL, terminal_sharpness REAL,
    stress_angle REAL, curvature_ratio REAL, counter_density REAL, foundry TEXT, weight INTEGER DEFAULT 400,
    is_italic INTEGER DEFAULT 0, supports_vietnamese INTEGER DEFAULT 0, family_name TEXT, subfamily TEXT,
    weight_class INTEGER, width_class INTEGER, source_id TEXT, primary_class TEXT, secondary_tags TEXT,
    serif_subtype TEXT, sans_subtype TEXT, script_subtype TEXT, is_favorite INTEGER DEFAULT 0);
CREATE TABLE sources (id TEXT PRIMARY KEY, name TEXT NOT NULL, type TEXT NOT NULL, path TEXT,
    metadata TEXT, date_added TEXT NOT NULL);";
pub const LEGACY_COLLECTIONS_DDL: &str =
    "CREATE TABLE collections (id TEXT PRIMARY KEY, name TEXT NOT NULL, color TEXT, created_at TEXT NOT NULL);";

pub struct LegacyFont<'a> {
    pub id: &'a str, pub path: &'a str, pub family: &'a str, pub subfamily: &'a str,
    pub tags: &'a str, pub collection: Option<&'a str>, pub favorite: bool, pub source: Option<&'a str>,
}

/// Dựng DB cũ giống app hiện tại. `with_collections=false` mô phỏng lỗi B1 (bảng không tồn tại).
pub fn make_legacy_db(path: &Path, with_collections: bool, src_dir: &Path, fonts: &[LegacyFont]) {
    let c = rusqlite::Connection::open(path).unwrap();
    c.execute_batch(LEGACY_FONTS_DDL).unwrap();
    if with_collections {
        c.execute_batch(LEGACY_COLLECTIONS_DDL).unwrap();
        c.execute("INSERT INTO collections VALUES ('col-1','Bộ cũ','#ff0000','2026-01-01T00:00:00Z')", []).unwrap();
    }
    c.execute("INSERT INTO sources VALUES ('src-1','Thư mục cũ','directory',?1,NULL,'2026-01-01T00:00:00Z')",
        [src_dir.to_string_lossy()]).unwrap();
    for f in fonts {
        c.execute(
            "INSERT INTO fonts(id,name,file_path,original_directory,tags,collection_id,date_added,date_modified,
                               family_name,subfamily,source_id,is_favorite)
             VALUES (?1,?2,?3,?4,?5,?6,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z',?7,?8,?9,?10)",
            rusqlite::params![f.id, f.family, f.path, src_dir.to_string_lossy(), f.tags, f.collection,
                              f.family, f.subfamily, f.source, f.favorite as i32],
        ).unwrap();
    }
}
