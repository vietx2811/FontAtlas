//! DDL v1 của hai file và bộ chạy migration theo `PRAGMA user_version`.
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::StoreError;
use crate::open::now_ms;

pub(crate) const LIBRARY_V1: &str = r#"
CREATE TABLE files (
  file_id       INTEGER PRIMARY KEY,
  hash          BLOB NOT NULL UNIQUE CHECK (length(hash) = 32),
  size          INTEGER NOT NULL,
  container     TEXT CHECK (container IN ('sfnt','collection','woff1','woff2')), -- NULL: nạp lỗi
  num_faces     INTEGER NOT NULL DEFAULT 0,
  first_seen_at INTEGER NOT NULL
);
CREATE TABLE file_locations (
  location_id  INTEGER PRIMARY KEY,
  file_id      INTEGER NOT NULL REFERENCES files ON DELETE CASCADE,
  source_id    INTEGER NOT NULL,              -- u.sources (khác file, không FK)
  path         TEXT NOT NULL UNIQUE,
  size         INTEGER NOT NULL,
  mtime_ns     INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  missing      INTEGER NOT NULL DEFAULT 0,
  scan_gen     INTEGER NOT NULL DEFAULT 0     -- thế hệ quét gần nhất chạm tới vị trí này
);
CREATE INDEX file_locations_file   ON file_locations(file_id);
CREATE INDEX file_locations_source ON file_locations(source_id, missing);
CREATE TABLE faces (
  face_id         INTEGER PRIMARY KEY,
  file_id         INTEGER NOT NULL REFERENCES files ON DELETE CASCADE,
  face_index      INTEGER NOT NULL,
  family_name     TEXT NOT NULL,              -- typographic_family ?? family ?? postscript ?? '(không tên)'
  subfamily       TEXT,                       -- typographic_subfamily ?? subfamily
  full_name       TEXT,
  postscript_name TEXT,
  manufacturer    TEXT,
  designer        TEXT,
  weight_class    INTEGER,
  width_class     INTEGER,
  is_italic       INTEGER NOT NULL DEFAULT 0,
  is_variable     INTEGER NOT NULL DEFAULT 0,
  outline_format  TEXT NOT NULL,              -- glyf|cff|cff2|none
  license_class   TEXT NOT NULL,              -- ofl|apache|other|unknown
  vi_status       TEXT NOT NULL,              -- full|precomposed_only|marks_only|none_or_partial
  supports_vi     INTEGER NOT NULL DEFAULT 0,
  vi_missing      INTEGER NOT NULL DEFAULT 0, -- số ký tự thiếu
  num_glyphs      INTEGER,
  family_key      TEXT NOT NULL,              -- vi_fold(family_name)
  search_text     TEXT NOT NULL,              -- vi_fold(các tên), tính ở Rust
  UNIQUE (file_id, face_index)
);
CREATE INDEX faces_family ON faces(family_key, weight_class, is_italic);
CREATE INDEX faces_weight ON faces(weight_class, family_key);
CREATE INDEX faces_vi     ON faces(vi_status);
CREATE VIRTUAL TABLE faces_fts USING fts5(
  search_text, content='faces', content_rowid='face_id',
  tokenize = 'trigram remove_diacritics 1'
);
CREATE TRIGGER faces_ai AFTER INSERT ON faces BEGIN
  INSERT INTO faces_fts(rowid, search_text) VALUES (new.face_id, new.search_text);
END;
CREATE TRIGGER faces_ad AFTER DELETE ON faces BEGIN
  INSERT INTO faces_fts(faces_fts, rowid, search_text) VALUES ('delete', old.face_id, old.search_text);
END;
CREATE TRIGGER faces_au AFTER UPDATE OF search_text ON faces BEGIN
  INSERT INTO faces_fts(faces_fts, rowid, search_text) VALUES ('delete', old.face_id, old.search_text);
  INSERT INTO faces_fts(rowid, search_text) VALUES (new.face_id, new.search_text);
END;
CREATE TABLE community_snapshots (
  snapshot_id      INTEGER PRIMARY KEY,
  name             TEXT NOT NULL,
  version          TEXT NOT NULL,
  format_version   INTEGER NOT NULL,
  analyzer_version TEXT,
  model_version    TEXT,
  sha256           BLOB NOT NULL,
  imported_at      INTEGER NOT NULL,
  UNIQUE (name, version)
);
CREATE TABLE analyses (
  hash             BLOB NOT NULL,
  face_index       INTEGER NOT NULL,
  analyzer_version TEXT NOT NULL,
  source           TEXT NOT NULL CHECK (source IN ('local','community','legacy')),
  snapshot_id      INTEGER REFERENCES community_snapshots ON DELETE CASCADE,
  metadata_json    TEXT NOT NULL,
  vietnamese_json  TEXT NOT NULL,
  anatomy_json     TEXT NOT NULL,
  analyzed_at      INTEGER NOT NULL,
  UNIQUE (hash, face_index, analyzer_version, source),
  CHECK ((source = 'community') = (snapshot_id IS NOT NULL))
);
CREATE TABLE embeddings (
  hash          BLOB NOT NULL,
  face_index    INTEGER NOT NULL,
  model_version TEXT NOT NULL,
  source        TEXT NOT NULL CHECK (source IN ('local','community','legacy')),
  snapshot_id   INTEGER REFERENCES community_snapshots ON DELETE CASCADE,
  dim           INTEGER NOT NULL,
  dtype         TEXT NOT NULL CHECK (dtype IN ('f32','f16','i8')),
  vec           BLOB NOT NULL,
  created_at    INTEGER NOT NULL,
  UNIQUE (hash, face_index, model_version, source),
  CHECK ((source = 'community') = (snapshot_id IS NOT NULL)),
  CHECK (length(vec) = dim * CASE dtype WHEN 'f32' THEN 4 WHEN 'f16' THEN 2 ELSE 1 END)
);
CREATE TABLE pipeline_state (
  hash         BLOB NOT NULL,
  face_index   INTEGER NOT NULL,                -- -1 = cấp file
  stage        TEXT NOT NULL CHECK (stage IN ('load','analysis','embedding')),
  status       TEXT NOT NULL CHECK (status IN ('pending','running','done','failed','skipped')),
  version      TEXT NOT NULL,
  error_code   TEXT,
  error_detail TEXT,
  attempts     INTEGER NOT NULL DEFAULT 0,
  updated_at   INTEGER NOT NULL,
  PRIMARY KEY (hash, face_index, stage)
) WITHOUT ROWID;
CREATE INDEX pipeline_failed ON pipeline_state(status) WHERE status IN ('pending','failed');
CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value INTEGER NOT NULL
) WITHOUT ROWID;
INSERT INTO meta(key, value) VALUES ('scan_generation', 0);
"#;

pub(crate) const USER_V1: &str = r#"
CREATE TABLE sources (
  source_id  INTEGER PRIMARY KEY,
  legacy_id  TEXT UNIQUE,
  kind       TEXT NOT NULL CHECK (kind IN ('directory','system','google_fonts','adobe_fonts','community')),
  name       TEXT NOT NULL,
  path       TEXT,
  enabled    INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX sources_kind_path ON sources(kind, COALESCE(path, ''));
CREATE TABLE favorites (
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (hash, face_index)
) WITHOUT ROWID;
CREATE TABLE collections (
  collection_id INTEGER PRIMARY KEY,
  legacy_id     TEXT UNIQUE,
  name          TEXT NOT NULL,
  color         TEXT,
  position      INTEGER NOT NULL DEFAULT 0,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);
CREATE TABLE collection_items (
  collection_id INTEGER NOT NULL REFERENCES collections ON DELETE CASCADE,
  hash          BLOB NOT NULL,
  face_index    INTEGER NOT NULL,
  added_at      INTEGER NOT NULL,
  PRIMARY KEY (collection_id, hash, face_index)
) WITHOUT ROWID;
CREATE INDEX collection_items_face ON collection_items(hash, face_index);
CREATE TABLE tags (
  tag_id     INTEGER PRIMARY KEY,
  name       TEXT NOT NULL,
  name_key   TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL
);
CREATE TABLE face_tags (
  tag_id     INTEGER NOT NULL REFERENCES tags ON DELETE CASCADE,
  hash       BLOB NOT NULL,
  face_index INTEGER NOT NULL,
  added_at   INTEGER NOT NULL,
  PRIMARY KEY (tag_id, hash, face_index)
) WITHOUT ROWID;
CREATE INDEX face_tags_face ON face_tags(hash, face_index);
CREATE TABLE activation_state (
  hash        BLOB PRIMARY KEY CHECK (length(hash) = 32),
  state       TEXT NOT NULL CHECK (state IN ('active','inactive')),
  scope       TEXT NOT NULL CHECK (scope IN ('session','persistent')),
  method      TEXT NOT NULL,
  active_path TEXT,
  updated_at  INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE legacy_ids (
  legacy_id   TEXT PRIMARY KEY,
  legacy_path TEXT NOT NULL,
  payload     TEXT NOT NULL,                  -- JSON hàng fonts cũ đã rút gọn (xem Task 7)
  hash        BLOB,                           -- NULL = chưa nối được
  face_index  INTEGER,
  resolved_at INTEGER
);
CREATE INDEX legacy_ids_unresolved ON legacy_ids(legacy_path) WHERE hash IS NULL;
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) WITHOUT ROWID;
"#;

/// v2 (Plan 7): activation_state có trạng thái thật + bảng lý do kích hoạt.
/// Đổi tên bảng để giữ dữ liệu cũ, vì SQLite không sửa được `CHECK`.
pub(crate) const USER_V2: &str = r#"
CREATE TABLE activation_state_v2 (
  hash           BLOB PRIMARY KEY CHECK (length(hash) = 32),
  state          TEXT NOT NULL CHECK (state IN ('active','inactive')),
  scope          TEXT NOT NULL CHECK (scope IN ('session','persistent')),
  method         TEXT NOT NULL,              -- 'coretext' | 'gdi' | 'fake'
  active_path    TEXT,                       -- chuỗi canonical đã đưa cho OS
  status         TEXT NOT NULL DEFAULT 'unknown' CHECK (status IN
                 ('registered','unregistered','conflict','missing_file','not_downloaded','error','unknown')),
  conflict_ps    TEXT,
  last_error     TEXT,
  activated_at   INTEGER,
  verified_at    INTEGER,
  updated_at     INTEGER NOT NULL,
  engine_version INTEGER NOT NULL DEFAULT 1
) WITHOUT ROWID;
INSERT INTO activation_state_v2 (hash,state,scope,method,active_path,updated_at)
  SELECT hash,state,scope,method,active_path,updated_at FROM activation_state;
DROP TABLE activation_state;
ALTER TABLE activation_state_v2 RENAME TO activation_state;
CREATE INDEX activation_state_status ON activation_state(scope, status);
CREATE TABLE activation_reasons (
  hash     BLOB NOT NULL REFERENCES activation_state(hash) ON DELETE CASCADE,
  reason   TEXT NOT NULL CHECK (reason = 'manual'
           OR (reason GLOB 'collection:[0-9]*' AND reason NOT GLOB 'collection:*[^0-9]*')),
  added_at INTEGER NOT NULL,
  PRIMARY KEY (hash, reason)
) WITHOUT ROWID;
CREATE INDEX activation_reasons_reason ON activation_reasons(reason);
INSERT INTO activation_reasons (hash, reason, added_at)
  SELECT hash, 'manual', updated_at FROM activation_state WHERE state = 'active';
"#;

/// Phần tử `i` đưa DB từ v`i` lên v`i+1`. Chỉ nối thêm, không sửa bản đã phát hành.
pub(crate) const LIBRARY_MIGRATIONS: &[&str] = &[LIBRARY_V1];
pub(crate) const USER_MIGRATIONS: &[&str] = &[USER_V1, USER_V2];

/// Chỉ cho test: tạo user.db ở v1 (trước Plan 7) để kiểm migration.
#[doc(hidden)]
pub fn create_user_v1_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user v1");
    c.execute_batch(USER_V1).expect("USER_V1");
    c.execute_batch("PRAGMA user_version = 1;").expect("user_version");
}
pub const LIBRARY_VERSION: u32 = LIBRARY_MIGRATIONS.len() as u32;
pub const USER_VERSION: u32 = USER_MIGRATIONS.len() as u32;

pub(crate) fn user_version(conn: &Connection) -> rusqlite::Result<u32> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
}

/// Chạy migration trên schema main của `conn`. Sao lưu một lần trước cả chuỗi
/// (khi DB đã có dữ liệu). Trả về đường dẫn bản sao lưu nếu có tạo.
pub(crate) fn migrate(
    conn: &Connection,
    db: &'static str,
    migs: &[&str],
    backup_dir: Option<&Path>,
) -> Result<Option<PathBuf>, StoreError> {
    let latest = migs.len() as u32;
    let cur = user_version(conn)?;
    if cur > latest {
        return Err(StoreError::NewerSchema { db, found: cur, supported: latest });
    }
    if cur == latest {
        return Ok(None);
    }
    let backup = match backup_dir {
        Some(dir) if cur > 0 => {
            std::fs::create_dir_all(dir)?;
            let p = dir.join(format!("{db}.v{cur}.{}.db", now_ms()));
            conn.execute("VACUUM INTO ?1", [p.to_string_lossy()])?;
            Some(p)
        }
        _ => None,
    };
    for v in cur..latest {
        let step = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            conn.execute_batch(migs[v as usize])?;
            conn.execute_batch(&format!("PRAGMA user_version = {}; COMMIT;", v + 1))
        };
        if let Err(e) = step() {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(StoreError::Migration { db, from: v, to: v + 1, source: e });
        }
    }
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    const M1: &str = "CREATE TABLE a(x); INSERT INTO a VALUES (1);";
    const M2_BAD: &str = "CREATE TABLE b(y); INSERT INTO khong_co VALUES (1);";

    #[test]
    fn failed_step_rolls_back_keeps_version_and_backs_up() {
        let d = tempfile::tempdir().unwrap();
        let c = Connection::open(d.path().join("t.db")).unwrap();
        assert!(migrate(&c, "user", &[M1], Some(d.path())).unwrap().is_none()); // từ v0: không sao lưu
        let err = migrate(&c, "user", &[M1, M2_BAD], Some(d.path())).unwrap_err();
        assert!(matches!(err, StoreError::Migration { from: 1, to: 2, .. }), "{err:?}");
        let v: u32 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 1);
        let b: i64 = c
            .query_row("SELECT count(*) FROM sqlite_schema WHERE name='b'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(b, 0, "bảng của bước lỗi phải bị rollback");
        let backups: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("user.v1."))
            .collect();
        assert_eq!(backups.len(), 1);
        let bc = Connection::open(backups[0].path()).unwrap();
        let x: i64 = bc.query_row("SELECT x FROM a", [], |r| r.get(0)).unwrap();
        assert_eq!(x, 1);
    }

    #[test]
    fn newer_version_is_refused_without_change() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA user_version = 7;").unwrap();
        let err = migrate(&c, "user", &[M1], None).unwrap_err();
        assert!(matches!(err, StoreError::NewerSchema { found: 7, supported: 1, .. }));
    }
}
