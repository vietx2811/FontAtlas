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
-- collections: dựng lại với AUTOINCREMENT để id đã xoá không bao giờ được dùng lại (lý do kích hoạt
-- `collection:<id>` không thể thừa hưởng nhầm). `migrate` tắt foreign_keys quanh bước này để
-- DROP TABLE không cascade xoá collection_items.
CREATE TABLE collections_v2 (
  collection_id INTEGER PRIMARY KEY AUTOINCREMENT,
  legacy_id     TEXT UNIQUE,
  name          TEXT NOT NULL,
  color         TEXT,
  position      INTEGER NOT NULL DEFAULT 0,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);
INSERT INTO collections_v2 (collection_id, legacy_id, name, color, position, created_at, updated_at)
  SELECT collection_id, legacy_id, name, color, position, created_at, updated_at FROM collections;
DROP TABLE collections;
ALTER TABLE collections_v2 RENAME TO collections;
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

/// v3 (Plan 6a): cặp font đã lưu.
pub(crate) const USER_V3: &str = r#"
CREATE TABLE pairs (
  pair_id       INTEGER PRIMARY KEY,
  a_hash        BLOB NOT NULL CHECK (length(a_hash) = 32), a_face INTEGER NOT NULL,
  a_role        TEXT NOT NULL CHECK (a_role IN ('heading','body','code')),
  b_hash        BLOB NOT NULL CHECK (length(b_hash) = 32), b_face INTEGER NOT NULL,
  b_role        TEXT NOT NULL CHECK (b_role IN ('heading','body','code')),
  mode          TEXT CHECK (mode IN ('safe','controlled','experimental','blocked')),
  preset        TEXT,
  sample_text   TEXT,
  collection_id INTEGER REFERENCES collections ON DELETE SET NULL,
  note          TEXT,
  created_at    INTEGER NOT NULL
);
CREATE INDEX pairs_collection ON pairs(collection_id);
"#;

/// v4 (Font Desk D1): ghi đè phân loại của người dùng.
pub(crate) const USER_V4: &str = r#"
CREATE TABLE classification_overrides (
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  class      TEXT NOT NULL CHECK (class IN ('serif','sans','mono','script','display','blackletter','symbol')),
  subclass   TEXT,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (hash, face_index)
) WITHOUT ROWID;
"#;

/// v5 (Font Desk D1, quyết định 2026-11-08): Display là tag mục đích dùng, không phải class. Dựng lại bảng không còn
/// `display` trong CHECK; override `display` cũ bị bỏ (không có nhãn hình dáng để chuyển — class tự động áp dụng lại).
pub(crate) const USER_V5: &str = r#"
CREATE TABLE classification_overrides_v5 (
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  class      TEXT NOT NULL CHECK (class IN ('serif','sans','mono','script','blackletter','symbol')),
  subclass   TEXT,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (hash, face_index)
) WITHOUT ROWID;
INSERT INTO classification_overrides_v5 (hash, face_index, class, subclass, updated_at)
  SELECT hash, face_index, class, subclass, updated_at FROM classification_overrides WHERE class <> 'display';
DROP TABLE classification_overrides;
ALTER TABLE classification_overrides_v5 RENAME TO classification_overrides;
"#;

/// v6 (Font Desk 0.3.0): Case và Combo. Face khoá theo (hash, face_index) như collection/pairs; font không còn trong
/// thư viện vẫn nằm trong Case (DTO báo `missing`). Xoá Case là xoá mềm (`deleted_at`), dọn sau 30 ngày.
pub(crate) const USER_V6: &str = r#"
CREATE TABLE cases (
  id         INTEGER PRIMARY KEY,
  name       TEXT NOT NULL,
  brief      TEXT,                          -- JSON BriefDto (NULL = không có)
  settings   TEXT NOT NULL DEFAULT '{}',    -- JSON {vi, mode}
  view_state TEXT NOT NULL DEFAULT '{}',    -- JSON: cột/ghim/sắp/nhóm đóng của Pairing và Compare
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  opened_at  INTEGER NOT NULL,
  deleted_at INTEGER
);
CREATE INDEX cases_opened ON cases(opened_at) WHERE deleted_at IS NULL;
CREATE TABLE case_faces (
  case_id    INTEGER NOT NULL REFERENCES cases ON DELETE CASCADE,
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  origin     TEXT NOT NULL CHECK (origin IN ('added','pinned','brief','peek')),
  note       TEXT,
  added_at   INTEGER NOT NULL,
  PRIMARY KEY (case_id, hash, face_index)
) WITHOUT ROWID;
CREATE TABLE combos (
  id         INTEGER PRIMARY KEY,
  case_id    INTEGER NOT NULL REFERENCES cases ON DELETE CASCADE,
  idx        INTEGER NOT NULL,
  name       TEXT NOT NULL,
  is_main    INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (case_id, idx)
);
CREATE TABLE combo_slots (
  combo_id   INTEGER NOT NULL REFERENCES combos ON DELETE CASCADE,
  role       TEXT NOT NULL CHECK (role IN ('display','heading','subheading','body','caption','accent','quote')),
  ord        INTEGER NOT NULL DEFAULT 0,
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  note       TEXT,
  style      TEXT NOT NULL DEFAULT '{}',    -- JSON: weight, italic, case, size_step, tracking, leading
  PRIMARY KEY (combo_id, role)
) WITHOUT ROWID;
"#;

/// Phần tử `i` đưa DB từ v`i` lên v`i+1`. Chỉ nối thêm, không sửa bản đã phát hành.
/// Library v2 (0.3.0): độ phủ ngôn ngữ (Hyperglot) theo face. Khoá theo (hash, face_index) như `analyses`.
/// `face_langs` chỉ giữ ngôn ngữ đạt ≥ near; `face_lang_state` đánh dấu face đã tính với `db_version` nào
/// (không có hàng ở `face_langs` + có dấu = "đã tính, không đạt"; không có dấu = chưa tính).
pub(crate) const LIBRARY_V2: &str = r#"
CREATE TABLE face_langs (
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  lang       TEXT NOT NULL,
  have       INTEGER NOT NULL,
  total      INTEGER NOT NULL,
  level      TEXT NOT NULL CHECK (level IN ('full','near')),
  PRIMARY KEY (hash, face_index, lang)
) WITHOUT ROWID;
CREATE INDEX face_langs_lang ON face_langs(lang, level);
CREATE TABLE face_lang_state (
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  db_version TEXT NOT NULL,
  PRIMARY KEY (hash, face_index)
) WITHOUT ROWID;
"#;


/// Library v3 (0.3.0): dải trục `wght`/`wdth` của font Variable (từ `fvar`) để lọc giao khoảng bằng SQL.
/// NULL = không phải Variable hoặc không có trục đó. Backfill từ bản phân tích MỚI NHẤT của mỗi face
/// (min và max luôn lấy từ cùng một hàng). Face Variable mà bản phân tích mới nhất hỏng JSON, không có
/// `axes` hoặc không có bản phân tích thì không thể điền: đánh dấu `pipeline_state.version = 'axes-backfill'`
/// ở dòng `load` của file để lần quét sau phân tích lại (xem `AXES_REANALYZE_MARK`).
pub(crate) const LIBRARY_V3: &str = r#"
ALTER TABLE faces ADD COLUMN wght_min REAL;
ALTER TABLE faces ADD COLUMN wght_max REAL;
ALTER TABLE faces ADD COLUMN wdth_min REAL;
ALTER TABLE faces ADD COLUMN wdth_max REAL;
CREATE INDEX faces_wght ON faces(wght_min, wght_max) WHERE wght_min IS NOT NULL;
CREATE TEMP TABLE latest_meta AS
  SELECT hash, face_index, mj FROM (
    SELECT hash, face_index, CASE WHEN json_valid(metadata_json) THEN metadata_json END AS mj,
           row_number() OVER (PARTITION BY hash, face_index ORDER BY analyzed_at DESC, rowid DESC) AS rn
    FROM analyses)
  WHERE rn = 1;
CREATE TEMP TABLE face_axes AS
  SELECT f.face_id AS face_id,
         json_extract(j.value, '$.tag') AS tag,
         json_extract(j.value, '$.min') AS mn,
         json_extract(j.value, '$.max') AS mx
  FROM faces f
  JOIN files fi ON fi.file_id = f.file_id
  JOIN latest_meta l ON l.hash = fi.hash AND l.face_index = f.face_index
  JOIN json_each(l.mj, '$.axes') j
  WHERE f.is_variable = 1;
UPDATE faces SET
  wght_min = (SELECT mn FROM face_axes a WHERE a.face_id = faces.face_id AND a.tag = 'wght' LIMIT 1),
  wght_max = (SELECT mx FROM face_axes a WHERE a.face_id = faces.face_id AND a.tag = 'wght' LIMIT 1),
  wdth_min = (SELECT mn FROM face_axes a WHERE a.face_id = faces.face_id AND a.tag = 'wdth' LIMIT 1),
  wdth_max = (SELECT mx FROM face_axes a WHERE a.face_id = faces.face_id AND a.tag = 'wdth' LIMIT 1)
WHERE is_variable = 1;
UPDATE pipeline_state SET version = 'axes-backfill'
WHERE stage = 'load' AND face_index = -1 AND hash IN (
  SELECT fi.hash FROM faces f JOIN files fi ON fi.file_id = f.file_id
  LEFT JOIN latest_meta l ON l.hash = fi.hash AND l.face_index = f.face_index
  WHERE f.is_variable = 1
    AND (l.mj IS NULL OR json_type(l.mj, '$.axes') IS NOT 'array'));
DROP TABLE latest_meta;
DROP TABLE face_axes;
"#;

/// Giá trị `pipeline_state.version` mà migration library v3 đặt để buộc phân tích lại.
pub const AXES_REANALYZE_MARK: &str = "axes-backfill";

/// Library v4 (0.2.1): `file_locations.cloud_only` = vị trí hiện là placeholder trên ổ cloud (dataless, chưa tải về).
/// Quét lại đặt 1 khi thấy cờ dataless, 0 khi file đã nằm trên máy. Dữ liệu phân tích giữ nguyên.
pub(crate) const LIBRARY_V4: &str = r#"
ALTER TABLE file_locations ADD COLUMN cloud_only INTEGER NOT NULL DEFAULT 0;
"#;

/// Library v5 (Library view options R2): năm phát hành ước lượng và ngày tạo file.
/// - `faces.release_year`/`release_year_src`: `release_year_src` = `copyright|head_created|head_modified|none`; NULL = chưa tính
///   (bản phân tích cũ chưa có `extras`) và được điền ở lần mở sau / khi phân tích lại. Backfill chạy ở Rust (`release::backfill`).
/// - `file_locations.created_ns` (birth time) và `created_src` (`disk` | `cloud_sync`): NULL tới lần quét kế tiếp.
pub(crate) const LIBRARY_V5: &str = r#"
ALTER TABLE faces ADD COLUMN release_year INTEGER;
ALTER TABLE faces ADD COLUMN release_year_src TEXT;
ALTER TABLE file_locations ADD COLUMN created_ns INTEGER;
ALTER TABLE file_locations ADD COLUMN created_src TEXT;
CREATE INDEX faces_release ON faces(release_year);
"#;

pub(crate) const LIBRARY_MIGRATIONS: &[&str] = &[LIBRARY_V1, LIBRARY_V2, LIBRARY_V3, LIBRARY_V4, LIBRARY_V5];
/// Chỉ cho test: library.db ở v4 (trước năm phát hành/ngày tạo file).
#[doc(hidden)]
pub fn create_library_v4_for_test(c: &Connection) {
    for m in [LIBRARY_V1, LIBRARY_V2, LIBRARY_V3, LIBRARY_V4] {
        c.execute_batch(m).expect("LIBRARY_Vn");
    }
    c.execute_batch("PRAGMA user_version = 4;").expect("user_version");
}
/// Chỉ cho test: library.db ở v3 (trước cờ cloud_only).
pub fn create_library_v3_for_test(c: &Connection) {
    c.execute_batch(LIBRARY_V1).expect("LIBRARY_V1");
    c.execute_batch(LIBRARY_V2).expect("LIBRARY_V2");
    c.execute_batch(LIBRARY_V3).expect("LIBRARY_V3");
    c.execute_batch("PRAGMA user_version = 3;").expect("user_version");
}
/// Chỉ cho test: library.db ở v2 (trước dải trục).
pub fn create_library_v2_for_test(c: &Connection) {
    c.execute_batch(LIBRARY_V1).expect("LIBRARY_V1");
    c.execute_batch(LIBRARY_V2).expect("LIBRARY_V2");
    c.execute_batch("PRAGMA user_version = 2;").expect("user_version");
}
/// Chỉ cho test: library.db ở v1 (trước độ phủ ngôn ngữ).
pub fn create_library_v1_for_test(c: &Connection) {
    c.execute_batch(LIBRARY_V1).expect("LIBRARY_V1");
    c.execute_batch("PRAGMA user_version = 1;").expect("user_version");
}

/// v10 (0.3.0 auto-activation; số do vị trí trong USER_MIGRATIONS, hiện sau USER_REPLACED = v9): lý do `auto:<app>` (CHECK của `activation_reasons` nới ra, dựng lại bảng
/// vì SQLite không sửa được CHECK) và nhật ký các lần app thiết kế hỏi font.
pub(crate) const USER_AUTO_ACTIVATION: &str = r#"
ALTER TABLE activation_reasons RENAME TO activation_reasons_v7;
CREATE TABLE activation_reasons (
  hash     BLOB NOT NULL REFERENCES activation_state(hash) ON DELETE CASCADE,
  reason   TEXT NOT NULL CHECK (reason = 'manual'
           OR (reason GLOB 'collection:[0-9]*' AND reason NOT GLOB 'collection:*[^0-9]*')
           OR (reason GLOB 'auto:[a-z]*' AND reason NOT GLOB 'auto:*[^a-z]*')),
  added_at INTEGER NOT NULL,
  PRIMARY KEY (hash, reason)
) WITHOUT ROWID;
INSERT INTO activation_reasons (hash, reason, added_at) SELECT hash, reason, added_at FROM activation_reasons_v7;
DROP TABLE activation_reasons_v7;
DROP INDEX IF EXISTS activation_reasons_reason;
CREATE INDEX activation_reasons_reason ON activation_reasons(reason);
CREATE TABLE auto_activation_log (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  at        INTEGER NOT NULL,
  app       TEXT NOT NULL,
  doc       TEXT,
  requested TEXT NOT NULL,             -- PostScript name hoặc "family / style" như app gửi
  outcome   TEXT NOT NULL,             -- activated | already_active | available | missing | conflict | failed
  hash      BLOB
);
"#;


/// Taxonomy v2 (Font Desk 0.3.x): ghi đè phân loại của người dùng chuyển sang 8 class mới.
/// SỐ PHIÊN BẢN do vị trí trong `USER_MIGRATIONS` quyết định (không hard-code ở đâu khác): nếu nhánh khác lấy cùng số
/// (feat/auto-activation cũng thêm một migration user.db), chỉ cần đổi thứ tự mục trong danh sách khi merge.
/// Ánh xạ SQL này phải khớp `fontatlas_core::classify::migrate_legacy` (có test đối chiếu).
pub(crate) const USER_TAXONOMY_V2: &str = r#"
CREATE TABLE classification_overrides_tx (
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  class      TEXT NOT NULL CHECK (class IN ('serif','slab','sans','mono','script','handwriting','display','blackletter','symbol')),
  subclass   TEXT,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (hash, face_index)
) WITHOUT ROWID;
INSERT INTO classification_overrides_tx (hash, face_index, class, subclass, updated_at)
  SELECT hash, face_index,
    CASE WHEN class='serif'  AND subclass='slab'        THEN 'slab'
         WHEN class='script' AND subclass='handwritten' THEN 'handwriting'
         ELSE class END,
    CASE WHEN class='serif'  AND subclass='slab'        THEN NULL
         WHEN class='script' AND subclass='handwritten' THEN NULL
         WHEN class='script' AND subclass='casual_brush' THEN 'brush'
         WHEN class='script' AND subclass='formal'       THEN 'formal_copperplate'
         WHEN class='serif'  AND subclass='humanist_oldstyle' THEN 'venetian_oldstyle'
         ELSE subclass END,
    updated_at
  FROM classification_overrides;
-- subclass không thuộc class (vd sans/slab, blackletter/x) bị bỏ, class giữ: cùng quy tắc `migrate_legacy`
UPDATE classification_overrides_tx SET subclass = NULL
 WHERE subclass IS NOT NULL AND class || '/' || subclass NOT IN (
  'serif/venetian_oldstyle','serif/garalde','serif/transitional','serif/didone','serif/glyphic','serif/wedge',
  'slab/clarendon','slab/egyptian','slab/humanist_slab','slab/typewriter',
  'sans/grotesque','sans/neo_grotesque','sans/geometric','sans/humanist','sans/flared','sans/rounded','sans/squarish',
  'mono/sans_mono','mono/serif_mono','mono/coding','mono/typewriter',
  'script/formal_copperplate','script/brush','script/monoline_script','script/signature','script/upright_script','script/calligraphic_unjoined',
  'handwriting/casual_print','handwriting/marker','handwriting/childlike',
  'display/stencil','display/inline_outline','display/shaded_3d','display/pixel_bitmap');
DROP TABLE classification_overrides;
ALTER TABLE classification_overrides_tx RENAME TO classification_overrides;
"#;


/// Font vừa xem (0.4.0, G5 nguồn "Recent"): cục bộ, tối đa `RECENT_MAX` dòng, mỗi face một dòng (xem lại thì đẩy lên đầu).
/// SỐ PHIÊN BẢN do vị trí trong `USER_MIGRATIONS` quyết định. Phân số 0.4.0 (Lead): v11 = G3 combo roles (03), v12 = bảng này;
/// trên nhánh này (chưa có v11 của 03) nó đang đứng ở vị trí 11, khi gom phải chèn migration của 03 TRƯỚC mục này.
pub(crate) const USER_RECENT_FACES: &str = r#"
CREATE TABLE recent_faces (
  hash       BLOB NOT NULL CHECK (length(hash) = 32),
  face_index INTEGER NOT NULL,
  viewed_at  INTEGER NOT NULL,
  PRIMARY KEY (hash, face_index)
) WITHOUT ROWID;
CREATE INDEX recent_faces_viewed ON recent_faces(viewed_at DESC);
"#;

/// Giải quyết trùng lặp (0.3.0): sổ quyết định THEO VỊ TRÍ TỆP (`loser_path`), không theo hash — hai đường dẫn cùng nội dung
/// (`same_file`) chung hash nên ẩn theo hash sẽ ẩn cả bản giữ. Một face bị ẩn khi MỌI vị trí còn thấy của nó đều nằm trong sổ.
/// `dup_link_moves` ghi những gì đã chuyển từ bản thua sang bản giữ để hoàn tác đúng từng dòng (không đụng dữ liệu có sẵn).
/// SỐ PHIÊN BẢN do vị trí trong `USER_MIGRATIONS` quyết định.
pub(crate) const USER_DUP_RESOLVE: &str = r#"
CREATE TABLE dup_resolutions (
  id          INTEGER PRIMARY KEY,
  batch_id    TEXT NOT NULL,
  kind        TEXT NOT NULL CHECK (kind IN ('same_file','versions','formats')),
  group_key   TEXT NOT NULL,
  keeper_hash BLOB NOT NULL CHECK (length(keeper_hash) = 32),
  keeper_face INTEGER NOT NULL,
  keeper_path TEXT NOT NULL,
  loser_hash  BLOB NOT NULL CHECK (length(loser_hash) = 32),
  loser_face  INTEGER NOT NULL,
  loser_path  TEXT NOT NULL,
  action      TEXT NOT NULL CHECK (action IN ('hide','trash')),
  trash_path  TEXT,
  reason      TEXT NOT NULL,
  was_active  INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL,
  undone_at   INTEGER,
  -- ghi sổ TRƯỚC khi đụng tệp / activation: pending = đang làm dở (đã ẩn); done = xong
  status      TEXT NOT NULL DEFAULT 'done' CHECK (status IN ('pending','done')),
  -- phạm vi activation của bản thua trước khi tắt (session|persistent), để hoàn tác bật lại đúng
  loser_scope TEXT CHECK (loser_scope IN ('session','persistent')),
  -- ta đã bật bản giữ giúp người dùng (nó chưa bật trước đó): hoàn tác tắt lại
  keeper_activated INTEGER NOT NULL DEFAULT 0
);
-- nhóm người dùng chọn "Bỏ qua": nhớ theo (loại, khoá nhóm), có thể bỏ qua → hiện lại
CREATE TABLE dup_ignored (
  kind       TEXT NOT NULL CHECK (kind IN ('same_file','versions','formats')),
  group_key  TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (kind, group_key)
) WITHOUT ROWID;
-- thứ tự nguồn ưu tiên giữ bản trùng (nhỏ = ưu tiên hơn); NULL = chưa sắp, xếp sau theo thứ tự tạo
ALTER TABLE sources ADD COLUMN priority INTEGER;
CREATE UNIQUE INDEX dup_resolutions_live ON dup_resolutions(loser_path) WHERE undone_at IS NULL;
CREATE INDEX dup_resolutions_batch ON dup_resolutions(batch_id);
CREATE TABLE dup_link_moves (
  resolution_id INTEGER NOT NULL REFERENCES dup_resolutions ON DELETE CASCADE,
  tbl           TEXT NOT NULL CHECK (tbl IN ('favorites','collection_items','face_tags','case_faces','pairs','combo_slots')),
  op            TEXT NOT NULL CHECK (op IN ('insert','update')),
  ref_id        INTEGER NOT NULL DEFAULT 0,
  side          TEXT NOT NULL DEFAULT ''
);
CREATE INDEX dup_link_moves_res ON dup_link_moves(resolution_id);
"#;

/// Sổ "bản cũ đã gỡ để thay bằng bản của Font Atlas" (nhánh feat/replace-activation). Số phiên bản do vị trí trong
/// `USER_MIGRATIONS` quyết định; khi merge cạnh migration của nhánh khác chỉ cần đổi thứ tự mục trong danh sách.
pub(crate) const USER_REPLACED: &str = r#"
CREATE TABLE activation_replaced (
  id          INTEGER PRIMARY KEY,
  hash        BLOB NOT NULL CHECK (length(hash) = 32),   -- font của Font Atlas đã thay thế
  old_path    TEXT NOT NULL,                              -- URL canonical của bản đã gỡ
  owner       TEXT NOT NULL,                              -- đoán theo đường dẫn: right_font, font_base, ...
  scopes      TEXT NOT NULL,                              -- JSON ["persistent","session"]: scope mà lệnh gỡ trả OK
  postscript_names TEXT NOT NULL DEFAULT '[]',
  created_at  INTEGER NOT NULL,
  restored_at INTEGER
);
CREATE INDEX activation_replaced_pending ON activation_replaced(hash) WHERE restored_at IS NULL;
"#;

pub(crate) const USER_MIGRATIONS: &[&str] = &[USER_V1, USER_V2, USER_V3, USER_V4, USER_V5, USER_V6, USER_TAXONOMY_V2, USER_DUP_RESOLVE, USER_REPLACED, USER_AUTO_ACTIVATION, USER_RECENT_FACES];

/// Chỉ cho test: user.db ngay TRƯỚC migration taxonomy v2 (mọi migration đứng trước nó trong danh sách).
#[doc(hidden)]
pub fn create_user_pre_taxonomy_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user pre-taxonomy");
    let n = USER_MIGRATIONS.iter().position(|m| *m == USER_TAXONOMY_V2).expect("taxonomy migration in list");
    for m in &USER_MIGRATIONS[..n] {
        c.execute_batch(m).expect("USER_Vn");
    }
    c.execute_batch(&format!("PRAGMA user_version = {n};")).expect("user_version");
}

/// Chỉ cho test: user.db ngay TRƯỚC migration sổ "thay thế" (mọi migration đứng trước nó trong danh sách).
#[doc(hidden)]
pub fn create_user_pre_replaced_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user pre-replaced");
    let n = USER_MIGRATIONS.iter().position(|m| *m == USER_REPLACED).expect("replaced migration in list");
    for m in &USER_MIGRATIONS[..n] {
        c.execute_batch(m).expect("USER_Vn");
    }
    c.execute_batch(&format!("PRAGMA user_version = {n};")).expect("user_version");
}

/// Chỉ cho test: tạo user.db ở v1 (trước Plan 7) để kiểm migration.
#[doc(hidden)]
pub fn create_user_v1_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user v1");
    c.execute_batch(USER_V1).expect("USER_V1");
    c.execute_batch("PRAGMA user_version = 1;").expect("user_version");
}
/// Chỉ cho test: tạo user.db ở v2 (Plan 7) để kiểm migration lên v3.
#[doc(hidden)]
pub fn create_user_v2_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user v2");
    c.execute_batch(USER_V1).expect("USER_V1");
    c.execute_batch(USER_V2).expect("USER_V2");
    c.execute_batch("PRAGMA user_version = 2;").expect("user_version");
}
/// Chỉ cho test: tạo user.db ở v4 để kiểm migration lên v5.
#[doc(hidden)]
pub fn create_user_v4_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user v4");
    for m in [USER_V1, USER_V2, USER_V3, USER_V4] {
        c.execute_batch(m).expect("USER_Vn");
    }
    c.execute_batch("PRAGMA user_version = 4;").expect("user_version");
}
/// Chỉ cho test: tạo user.db ở v5 để kiểm migration lên v6.
#[doc(hidden)]
pub fn create_user_v5_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user v5");
    for m in [USER_V1, USER_V2, USER_V3, USER_V4, USER_V5] {
        c.execute_batch(m).expect("USER_Vn");
    }
    c.execute_batch("PRAGMA user_version = 5;").expect("user_version");
}
/// Chỉ cho test: user.db ngay TRƯỚC migration auto-activation (sau USER_REPLACED), để kiểm migration lên v10.
#[doc(hidden)]
pub fn create_user_pre_auto_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user v7");
    let n = USER_MIGRATIONS.iter().position(|m| *m == USER_AUTO_ACTIVATION).expect("auto-activation migration in list");
    for m in &USER_MIGRATIONS[..n] {
        c.execute_batch(m).expect("USER_Vn");
    }
    c.execute_batch(&format!("PRAGMA user_version = {n};")).expect("user_version");
}
/// Chỉ cho test: tạo user.db ở v3 để kiểm migration lên v4.
#[doc(hidden)]
pub fn create_user_v3_for_test(path: &Path) {
    let c = Connection::open(path).expect("open user v3");
    for m in [USER_V1, USER_V2, USER_V3] {
        c.execute_batch(m).expect("USER_Vn");
    }
    c.execute_batch("PRAGMA user_version = 3;").expect("user_version");
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
            let _ = prune_backups(dir, db, BACKUPS_KEPT);
            Some(p)
        }
        _ => None,
    };
    // Dựng lại bảng có khoá ngoại trỏ tới (vd collections) cần foreign_keys=OFF TRƯỚC khi mở transaction
    // (PRAGMA này không có tác dụng bên trong transaction). Khôi phục giá trị cũ sau khi xong.
    let fk_was_on: bool =
        conn.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0)).map(|v| v == 1).unwrap_or(false);
    if fk_was_on {
        conn.execute_batch("PRAGMA foreign_keys = OFF")?;
    }
    let restore_fk = || {
        if fk_was_on {
            let _ = conn.execute_batch("PRAGMA foreign_keys = ON");
        }
    };
    for v in cur..latest {
        let step = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            conn.execute_batch(migs[v as usize])?;
            conn.execute_batch(&format!("PRAGMA user_version = {}; COMMIT;", v + 1))
        };
        if let Err(e) = step() {
            let _ = conn.execute_batch("ROLLBACK");
            restore_fk();
            return Err(StoreError::Migration { db, from: v, to: v + 1, source: e });
        }
    }
    restore_fk();
    Ok(backup)
}

/// Số bản sao lưu migration giữ lại cho mỗi DB.
pub(crate) const BACKUPS_KEPT: usize = 5;

/// Giữ `keep` bản sao lưu mới nhất của `db` trong `dir` (tên `{db}.v{N}.{ms}.db`, mới = `ms` lớn);
/// trả số bản đã xoá. File khác tên không bị đụng.
pub(crate) fn prune_backups(dir: &Path, db: &str, keep: usize) -> std::io::Result<usize> {
    let prefix = format!("{db}.v");
    let mut found: Vec<(i64, PathBuf)> = vec![];
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_prefix(&prefix).and_then(|r| r.strip_suffix(".db")) else { continue };
        if let Some(ms) = stem.rsplit('.').next().and_then(|s| s.parse::<i64>().ok()) {
            found.push((ms, e.path()));
        }
    }
    found.sort();
    let excess = found.len().saturating_sub(keep);
    for (_, p) in found.iter().take(excess) {
        std::fs::remove_file(p)?;
    }
    Ok(excess)
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
