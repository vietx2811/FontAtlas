# Plan 2 — `fontatlas-store` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dựng crate Rust `fontatlas-store`. Crate này giữ dữ liệu của app trong hai file SQLite: `library.db` (dẫn xuất, dựng lại được) và `user.db` (dữ liệu người dùng). Nó ghi kết quả `FileReport` của `fontatlas-core`, quét lại thư mục theo diff, tìm tên tiếng Việt gập dấu, truy vấn lưới có lọc/sắp xếp/phân trang, quản lý favorite/collection/tag và chuyển dữ liệu từ DB cũ.

**Architecture:** Một `Store` giữ **một** `rusqlite::Connection` mở `library.db` làm schema `main` và `ATTACH` `user.db` làm schema `u`. Migration của mỗi file chạy trên kết nối riêng **trước** khi attach (để DDL rơi đúng file). Bảng người dùng tham chiếu font bằng `(hash, face_index)`, không có khoá ngoại sang `library.db`. Quét lại: đi cây thư mục → so `(path, size, mtime_ns)` → băm BLAKE3 phần đổi → chỉ chạy `fontatlas_core::analyze_paths` cho hash chưa có kết quả ở `ANALYZER_VERSION` hiện tại → ghi theo lô trong transaction. DDL đi từ spike đã chạy thật (`docs/team/spikes/02_data_vector.md`, `spikes/data-vector/schema/*.sql` ở nhánh `claude/gracious-euler-ca23d9`), chỉnh theo API thật của core.

**Tech Stack:** Rust edition 2024 (rustc ≥ 1.85), rusqlite **=0.31.0** bundled (SQLite 3.45.0), blake3 =1.8.7, unicode-normalization 0.1.25, rayon 1.12, serde 1 + serde_json 1, thiserror 2; dev: tempfile 3.27. Phụ thuộc `fontatlas-core` (path).

**Spec:** `docs/superpowers/specs/2026-10-16-fontatlas-mvp-design.md`, mục 2, 3, 9, 10. Plan này là Plan 2 trong lộ trình cuối Plan 1 (`docs/superpowers/plans/2026-10-16-plan1-fontatlas-core.md`).

## Global Constraints

- `rusqlite = { version = "=0.31.0", features = ["bundled"] }` — **phải trùng** phiên bản app (`UX/src-tauri/Cargo.toml`: `rusqlite = "0.31"`). Hai bản `libsqlite3-sys` khác nhau trong cùng workspace sẽ xung đột `links = "sqlite3"`. SQLite 3.45.0 đủ cho `trigram remove_diacritics` (spike đã chạy thật trên đúng bản này).
- Ghim: `blake3 = "=1.8.7"` (cùng core), `unicode-normalization = "0.1.25"`.
- Hai file trong thư mục dữ liệu app: `library.db`, `user.db`; sao lưu ở `backups/`. Schema attach của `user.db` tên là `u`.
- `library.db`: `page_size=16384` (đặt khi tạo, trước bảng đầu tiên), WAL. `user.db`: WAL. Mỗi kết nối: `foreign_keys=ON`, `busy_timeout=5000`.
- `LIBRARY_VERSION = 1`, `USER_VERSION = 1`, quản lý bằng `PRAGMA user_version`. Mỗi bước migration trong một transaction `BEGIN IMMEDIATE`; lỗi thì rollback và giữ phiên bản cũ. Sao lưu `user.db` **một lần** trước cả chuỗi migration.
- **`user.db` không bao giờ bị ghi đè:** sao lưu `VACUUM INTO backups/user.v{N}.{unix_ms}.db` trước mỗi migration của `user.db` có phiên bản > 0; `user.db` mới hơn app → từ chối mở (`newer_schema`), không ghi gì. `library.db` mới hơn hoặc migration lỗi → đổi tên thành `library.broken-{unix_ms}.db` rồi tạo mới.
- **Rescan và mọi thao tác trên `library.db` không ghi vào `user.db`.** Chỉ API người dùng (Task 4) và `resolve_legacy` (Task 7, chỉ `INSERT OR IGNORE`, không xoá/ghi đè) ghi `user.db`.
- Danh tính: `hash` = BLAKE3 32 byte của file gốc (giống `FileReport::hash_hex`), face = `(hash, face_index)`.
- `vi_fold`: NFD → bỏ ký tự combining (General Category Mn) → `đ/Đ → d` → `to_lowercase` → gộp khoảng trắng, trim. Dùng cho `faces.search_text`, `faces.family_key`, `u.tags.name_key` và chuỗi truy vấn.
- Tìm tên: FTS5 `trigram remove_diacritics 1` khi truy vấn đã gập ≥ 3 ký tự; ngắn hơn thì `LIKE` tiền tố từ. Người dùng không gõ được cú pháp FTS hoặc ký tự đại diện `LIKE`.
- `source` của `analyses`/`embeddings` ∈ `local | community | legacy`; `source='community'` ⇔ có `snapshot_id`.
- Mã lỗi `StoreError::code()`: `sqlite`, `io`, `newer_schema`, `migration`, `bad_hash`, `legacy`, `not_found`, `nested_source`. Mã lỗi file lưu trong `pipeline_state.error_code` là `CoreError::code()` (`io`, `not_a_font`, `truncated`, `missing_table`, `woff_decode`, `parse`).
- Thời gian: `i64` mili giây Unix.
- Không `unwrap()`/`expect()` trên dữ liệu từ DB, file hoặc DB cũ trong code thư viện (test được dùng). Code không panic trên DB cũ hỏng.
- Comment trong code tiếng Việt ngắn gọn. Mỗi commit kết thúc bằng `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **File bị đổi tên hoặc di chuyển giữa hai lần quét:** cùng hash → chỉ thêm `file_locations` mới, không phân tích lại; đường cũ đánh `missing=1`; favorite/collection/tag vẫn gắn (khoá theo hash). Test ở Task 6 (`rename_relinks_without_reanalysis_and_keeps_favorite`).
2. **Cùng một font ở 2 thư mục (hoặc 2 nguồn):** một `files`, hai `file_locations`, lưới chỉ hiện **một** face; xoá một nguồn thì face vẫn còn. Test ở Task 6 (`duplicate_in_two_sources_is_one_face`).
3. **Migration bị gián đoạn hoặc lỗi giữa chừng, `user.db` mới hơn app:** phiên bản giữ nguyên, dữ liệu nguyên vẹn, có bản sao lưu; `user.db` mới hơn thì từ chối và byte file không đổi. Import DB cũ lỗi giữa chừng không để lại dòng dở. Test ở Task 2 và Task 7.
4. **Tìm "Đẹp" bằng "dep":** cùng với input NFD, chữ hoa, truy vấn 1–2 ký tự, và truy vấn chứa `"`, `%`, `_`, `*`. Test ở Task 1 và Task 5.
5. **`user.db` không bao giờ bị ghi đè:** rescan và dựng lại `library.db` không đổi kích thước/mtime của `user.db` và `user.db-wal`; favorite sống sót khi `library.db` bị xoá. Test ở Task 2 và Task 6.

Thêm (không tính vào 5 mục trên nhưng có test): đường dẫn có tên tiếng Việt NFD vs NFC khi nối dữ liệu cũ (Task 7); DB cũ thiếu bảng `collections` hoặc thiếu cột (Task 7); TTC nhiều face (Task 3).


## Sửa sau review (Lead, 2026-10-16) — có hiệu lực cao hơn phần task bên dưới

Các mục sau thay thế chỗ tương ứng trong task. Người thực thi làm theo mục này khi có khác biệt.

- **A1 (Task 2) `library.db` hỏng** (không phải SQLite, `SQLITE_CORRUPT`, `SQLITE_NOTADB`) được xử lý như `NewerSchema`/`Migration`: đổi tên `library.broken-{ms}.db` rồi tạo mới. Test `garbage_library_is_recreated`: ghi `b"rac rac rac"` vào `library.db`, `Store::open` → `library_recreated == true`, dữ liệu `u.settings` còn nguyên.
- **A2 (Task 2, 3, 6) `missing` theo thế hệ quét, không theo đồng hồ.** `library.db` có thêm bảng `meta(key TEXT PRIMARY KEY, value INTEGER NOT NULL) WITHOUT ROWID` với khoá `scan_generation`; `file_locations` có thêm cột `scan_gen INTEGER NOT NULL DEFAULT 0`. `rescan_source` tăng `scan_generation` lên 1 trước khi quét; mọi vị trí chạm tới (không đổi, nối lại, phân tích) ghi `scan_gen = thế hệ hiện tại`; cuối lần quét `UPDATE file_locations SET missing=1 WHERE source_id=? AND missing=0 AND scan_gen < ?thế_hệ`. `Store::ingest` ghi `scan_gen` = giá trị `scan_generation` hiện tại (không tăng). `last_seen_at` vẫn ghi để hiển thị, không dùng để quyết định.
- **A3 (Task 3, 6) Test FTS thật.** Bỏ `SELECT count(*) FROM faces_fts` (đọc từ `faces`). Helper `fts_ok(&Store)` chạy `INSERT INTO faces_fts(faces_fts) VALUES('integrity-check')` (lỗi = hỏng). Sau re-ingest: `integrity-check` qua và `SELECT count(*) FROM faces_fts WHERE faces_fts MATCH '"roboto"'` = 1. Sau `remove_source` + `gc` ở Task 6: `integrity-check` qua và MATCH tên font đã xoá = 0.
- **A4 (Task 7) Lỗi giữa transaction import.** Test `failure_inside_import_transaction_leaves_nothing`: tạo `CREATE TEMP TRIGGER boom BEFORE INSERT ON u.legacy_ids WHEN (SELECT count(*) FROM u.legacy_ids) >= 2 BEGIN SELECT RAISE(ABORT, 'boom'); END;` qua `s.conn()`, `import_legacy` trả lỗi, rồi `u.sources`, `u.collections`, `u.legacy_ids`, `u.tags` đều rỗng.
- **A5 (Task 2) Kiểm phiên bản `user.db` bằng kết nối chỉ đọc** (`SQLITE_OPEN_READ_ONLY`) **trước** khi mở đọc-ghi. Test `newer_user_db_in_wal_is_untouched`: tạo `user.db` ở WAL, ghi dữ liệu + `user_version=99`, giữ kết nối không đóng (`std::mem::forget`) để `user.db-wal` còn khung; `Store::open` → `newer_schema`; byte của `user.db` **và** `user.db-wal` không đổi.
- **A6 (Task 6) Review Focus 5 chặt hơn.** `fingerprint` = (độ dài, mtime, BLAKE3 nội dung). Test `user_db_bytes_unchanged_by_rescan_and_library_rebuild`: chụp fingerprint `user.db` + `user.db-wal` sau checkpoint; rescan, xoá file, rescan, `gc`, đóng store, ghi rác vào `library.db`, mở lại (dựng lại library) → fingerprint không đổi.
- **A7 (Task 4) Chặn nguồn lồng nhau** (quyết định của Lead). `add_source` với `path` nằm trong hoặc chứa đường dẫn của một nguồn đã có → `StoreError::NestedSource { existing_id: i64 }`, mã `nested_source`. So sánh: `canonicalize` (lỗi thì giữ nguyên) → NFC → so theo thành phần đường dẫn (`/a/b` không chứa `/a/bc`). Trùng đúng đường dẫn và cùng `kind` → trả id cũ; trùng đường dẫn khác `kind` → `NestedSource`. Đường dẫn lưu nguyên như người dùng đưa (không canonicalize). Test hai chiều: cha có trước rồi thêm con; con có trước rồi thêm cha; và `/a/b` + `/a/bc` được phép.
- **A8 (Task 4) Nguồn không có path:** bỏ `UNIQUE(kind, path)`; thay bằng `CREATE UNIQUE INDEX sources_kind_path ON sources(kind, COALESCE(path, ''))`. `add_source` tìm trước bằng `kind = ? AND path IS ?` trong cùng transaction. Test: `add_source(GoogleFonts, "G", None)` hai lần trả cùng id.
- **A9 (Task 6) Gốc nguồn không đọc được** (ổ ngoài đã rút, thư mục bị xoá/đổi tên): `rescan_source` **không** đánh `missing` gì, không tăng thế hệ, trả `RescanStats { status: RescanStatus::SourceUnavailable, .. }`. `RescanStatus { Ok, SourceUnavailable }` (serde `snake_case`, chuỗi `source_unavailable`). Test `unplugged_root_marks_nothing_missing`: quét, đổi tên thư mục gốc, quét lại → `SourceUnavailable`, `missing == 0`, tổng face không đổi; đổi tên lại → quét bình thường. Thay test `unreadable_root_yields_empty_scan_not_panic`.
- **A10 (Task 7) `tags` cũ NULL hoặc sai kiểu** xử lý theo từng hàng: NULL → không tag, không đếm; không phải chuỗi JSON mảng chuỗi (số, BLOB, JSON khác) → không tag, `bad_tags += 1`. Đọc bằng `get_ref` + `ValueRef`, không `get::<String>`. Thêm hàng NULL và hàng số nguyên vào test `old_db_missing_newer_columns_still_imports`.
- **A11 (Task 3) Ánh xạ cột rõ ràng:** `supports_vi` = `vietnamese.supports_vietnamese`; `weight_class` = `metadata.weight_class`; `width_class` = `metadata.width_class`; `num_glyphs` = `metadata.num_glyphs`; `full_name` = `metadata.full_name`; `postscript_name` = `metadata.postscript_name`; `manufacturer` = `metadata.manufacturer`; `designer` = `metadata.designer`.
- **A12 (mọi task)** Mỗi `assert!(xs.iter().all(..))` hoặc `windows(2)` đi kèm `assert!(!xs.is_empty())`.
- **A13 (Global Constraints)** Sao lưu `user.db` **một lần** trước cả chuỗi migration (không phải trước từng bước). Báo cáo Task 8 ghi: lỗi chỉ ở cấp file (`face_index=-1`) vì core trả `Result` theo file; rescan đọc file mới hai lần (băm rồi phân tích) — để tối ưu sau.

---

## File Structure

```
UX/src-tauri/
├── Cargo.toml                           # Sửa: thêm member crates/fontatlas-store
└── crates/fontatlas-store/
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs       # khai báo module, re-export
    │   ├── error.rs     # StoreError + code()
    │   ├── fold.rs      # vi_fold, fts_query
    │   ├── schema.rs    # DDL v1 hai file, migrate()
    │   ├── open.rs      # Store, OpenReport, open(), PRAGMA, sao lưu, dựng lại library
    │   ├── ingest.rs    # FaceKey, ScannedFile, IngestItem, Analysis, ingest(), pipeline_state, problem_files
    │   ├── user.rs      # sources, favorites, collections, tags
    │   ├── query.rs     # FaceQuery, FacePage, FaceRow, query_faces
    │   ├── rescan.rs    # walk_fonts, rescan_source, remove_source + gc
    │   └── legacy.rs    # import_legacy, resolve_legacy, legacy_id
    └── tests/
        ├── common/mod.rs   # tmp dir, fixture từ fontatlas-core, report giả, DDL DB cũ
        ├── fold.rs  open.rs  ingest.rs  user.rs  query.rs  rescan.rs  legacy.rs
```

Fixture font dùng lại từ `crates/fontatlas-core/tests/fixtures/` (5 TTF OFL/Apache + WOFF/WOFF2), không chép.

---

### Task 1: Crate, `StoreError`, `vi_fold`

**Files:**
- Modify: `UX/src-tauri/Cargo.toml` (mảng `members`)
- Create: `UX/src-tauri/crates/fontatlas-store/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/fold.rs`
- Test: `tests/common/mod.rs`, `tests/fold.rs`

**Interfaces:**
- Produces:
  - `pub enum StoreError { Sqlite(rusqlite::Error), Io(std::io::Error), NewerSchema { db: &'static str, found: u32, supported: u32 }, Migration { db: &'static str, from: u32, to: u32, source: rusqlite::Error }, BadHash(String), Legacy(String), NotFound(String) }` cùng `StoreError::code(&self) -> &'static str`
  - `pub fn vi_fold(s: &str) -> String`
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub(crate) enum TextMatch { Fts(String), Prefix { start: String, word: String } }` và `pub(crate) fn text_match(query: &str) -> Option<TextMatch>` (None khi truy vấn rỗng sau khi gập)

- [ ] **Step 1: Tạo nhánh làm việc**

```bash
git switch -c feat/fontatlas-store feat/fontatlas-core
```
Nhánh gốc phải đã có commit đổi `units_per_em`, `italic_angle`, `num_glyphs` sang `Option` (đang làm ở Plan 1). Kiểm tra: `grep -n "italic_angle: Option<f32>" UX/src-tauri/crates/fontatlas-core/src/metadata.rs` phải ra 1 dòng; nếu không, dừng và báo Lead.

- [ ] **Step 2: Khai báo crate**

Sửa `UX/src-tauri/Cargo.toml`: `members = [".", "crates/fontatlas-core", "crates/fontatlas-store"]`.

Tạo `crates/fontatlas-store/Cargo.toml`:
```toml
[package]
name = "fontatlas-store"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
description = "Kho SQLite của FontAtlas: library.db + user.db, migration, FTS, rescan, di chuyển DB cũ"

[dependencies]
blake3 = "=1.8.7"
fontatlas-core = { path = "../fontatlas-core" }
rayon = "1.12"
rusqlite = { version = "=0.31.0", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
unicode-normalization = "0.1.25"

[dev-dependencies]
tempfile = "3.27"
```

`src/lib.rs` lúc này chỉ có `mod error; mod fold; pub use error::StoreError; pub use fold::vi_fold;` — các task sau thêm module và re-export của mình.

- [ ] **Step 3: Viết test thất bại** — `tests/common/mod.rs` (khởi tạo, các task sau bổ sung) và `tests/fold.rs`

`tests/common/mod.rs`:
```rust
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures").join(name)
}

pub fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// (độ dài, mtime) để chứng minh một file không bị ghi; None nếu chưa tồn tại.
pub fn fingerprint(p: &Path) -> Option<(u64, std::time::SystemTime)> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.len(), m.modified().ok()?))
}
```

`tests/fold.rs`:
```rust
use fontatlas_store::vi_fold;

#[test]
fn folds_vietnamese_including_d_stroke() {
    assert_eq!(vi_fold("Đẹp Sans"), "dep sans");
    assert_eq!(vi_fold("ĐƯỜNG PHỐ"), "duong pho");
    assert_eq!(vi_fold("Quốc Ngữ"), "quoc ngu");
    assert_eq!(vi_fold("đđĐĐ"), "dddd");
}

#[test]
fn nfc_and_nfd_inputs_fold_the_same() {
    let nfd = "Ho\u{0302}\u{0300}ng"; // "Hồng" dạng NFD
    assert_eq!(vi_fold(nfd), vi_fold("Hồng"));
    assert_eq!(vi_fold(nfd), "hong");
}

#[test]
fn collapses_whitespace_and_trims() {
    assert_eq!(vi_fold("  Phở \t  Bò\n"), "pho bo");
    assert_eq!(vi_fold(""), "");
}

#[test]
fn non_latin_is_kept_lowercased() {
    assert_eq!(vi_fold("東京 Gothic"), "東京 gothic");
    assert_eq!(vi_fold("Ελληνικά"), "ελληνικα");
}
```

Unit test trong `src/fold.rs` (cuối file, `#[cfg(test)] mod tests`):
```rust
#[test]
fn text_match_picks_fts_or_prefix() {
    assert_eq!(text_match("Đẹp"), Some(TextMatch::Fts("\"dep\"".into())));
    assert_eq!(text_match("PL"), Some(TextMatch::Prefix { start: "pl%".into(), word: "% pl%".into() }));
    assert_eq!(text_match("   "), None);
}

#[test]
fn text_match_neutralises_syntax() {
    // dấu " bị bỏ, % _ \ được escape cho LIKE ... ESCAPE '\'
    assert_eq!(text_match("a\"b*c"), Some(TextMatch::Fts("\"ab*c\"".into())));
    assert_eq!(text_match("_%"), Some(TextMatch::Prefix { start: "\\_\\%%".into(), word: "% \\_\\%%".into() }));
}
```

- [ ] **Step 4: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store`
Expected: FAIL, lỗi biên dịch `cannot find function vi_fold`.

- [ ] **Step 5: Viết implementation**

`src/error.rs`: enum như Interfaces, `#[derive(Debug, thiserror::Error)]`, `#[from]` cho `rusqlite::Error` và `std::io::Error`, `#[source]` cho `Migration.source`. Thông điệp tiếng Việt, ví dụ `"{db} có schema v{found}, app chỉ hỗ trợ tới v{supported}"`.

`src/fold.rs`:
- `vi_fold`: `s.nfd()` → bỏ ký tự có `unicode_normalization::char::is_combining_mark(c)` → `'đ' | 'Đ' => 'd'` → `flat_map(char::to_lowercase)` → `split_whitespace().collect::<Vec<_>>().join(" ")`.
- `text_match(q)`: `f = vi_fold(q)`, bỏ mọi `"`; rỗng → `None`; `f.chars().count() >= 3` → `Fts(format!("\"{f}\""))`; ngắn hơn → escape `\`→`\\`, `%`→`\%`, `_`→`\_` rồi `Prefix { start: e + "%", word: "% " + e + "%" }`.

- [ ] **Step 6: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store`
Expected: `tests/fold.rs` 4 passed; unit test 2 passed.

- [ ] **Step 7: Commit**

```bash
git add UX/src-tauri/Cargo.toml UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): crate skeleton, StoreError, vi_fold with đ→d

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Schema v1, migration, `Store::open`

**Files:**
- Create: `src/schema.rs`, `src/open.rs`
- Modify: `src/lib.rs`
- Test: `tests/open.rs` (+ unit test trong `src/schema.rs`)

**Interfaces:**
- Consumes: `StoreError` (Task 1)
- Produces:
  - `pub const LIBRARY_FILE: &str = "library.db"`, `pub const USER_FILE: &str = "user.db"`, `pub const BACKUP_DIR: &str = "backups"`
  - `pub const LIBRARY_VERSION: u32 = 1`, `pub const USER_VERSION: u32 = 1`
  - `pub(crate) const LIBRARY_MIGRATIONS: &[&str]`, `pub(crate) const USER_MIGRATIONS: &[&str]` (phần tử `i` đưa DB từ v`i` lên v`i+1`)
  - `pub(crate) fn migrate(conn: &Connection, db: &'static str, migs: &[&str], backup_dir: Option<&Path>) -> Result<Option<PathBuf>, StoreError>` — trả đường dẫn bản sao lưu nếu có tạo
  - `pub struct Store { conn: Connection, dir: PathBuf }`
  - `pub struct OpenReport { pub library_recreated: bool, pub user_backup: Option<PathBuf> }`
  - `Store::open(dir: &Path) -> Result<(Store, OpenReport), StoreError>`
  - `Store::conn(&self) -> &Connection` (chẩn đoán và test), `Store::dir(&self) -> &Path`
  - `pub(crate) fn now_ms() -> i64`

- [ ] **Step 1: Viết test thất bại** — `tests/open.rs`

```rust
mod common;
use common::*;
use fontatlas_store::{LIBRARY_VERSION, Store, USER_VERSION};

fn pragma_i64(s: &Store, sql: &str) -> i64 {
    s.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}
fn pragma_str(s: &Store, sql: &str) -> String {
    s.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn fresh_open_creates_both_files_with_pragmas() {
    let d = tmp();
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(!rep.library_recreated && rep.user_backup.is_none());
    assert!(d.path().join("library.db").exists() && d.path().join("user.db").exists());
    assert_eq!(pragma_i64(&s, "PRAGMA main.page_size"), 16384);
    assert_eq!(pragma_str(&s, "PRAGMA main.journal_mode"), "wal");
    assert_eq!(pragma_str(&s, "PRAGMA u.journal_mode"), "wal");
    assert_eq!(pragma_i64(&s, "PRAGMA main.user_version"), LIBRARY_VERSION as i64);
    assert_eq!(pragma_i64(&s, "PRAGMA u.user_version"), USER_VERSION as i64);
    assert_eq!(pragma_i64(&s, "PRAGMA foreign_keys"), 1);
    for t in ["files", "file_locations", "faces", "faces_fts", "analyses", "embeddings", "pipeline_state", "community_snapshots"] {
        let n: i64 = s.conn().query_row("SELECT count(*) FROM main.sqlite_schema WHERE name=?1", [t], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "library thiếu {t}");
    }
    for t in ["sources", "favorites", "collections", "collection_items", "tags", "face_tags", "activation_state", "legacy_ids", "settings"] {
        let n: i64 = s.conn().query_row("SELECT count(*) FROM u.sqlite_schema WHERE name=?1", [t], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "user thiếu {t}");
    }
}

#[test]
fn reopen_is_idempotent_and_makes_no_backup() {
    let d = tmp();
    drop(Store::open(d.path()).unwrap());
    let (_s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.user_backup.is_none() && !rep.library_recreated);
    let n = std::fs::read_dir(d.path().join("backups")).map(|r| r.count()).unwrap_or(0);
    assert_eq!(n, 0);
}

#[test]
fn newer_user_db_is_refused_and_untouched() {
    let d = tmp();
    let up = d.path().join("user.db");
    {
        let c = rusqlite::Connection::open(&up).unwrap();
        c.execute_batch("CREATE TABLE keep(x); INSERT INTO keep VALUES (42); PRAGMA user_version = 99;").unwrap();
    }
    let before = std::fs::read(&up).unwrap();
    let err = Store::open(d.path()).err().expect("phải từ chối");
    assert_eq!(err.code(), "newer_schema");
    assert_eq!(std::fs::read(&up).unwrap(), before);
}

#[test]
fn newer_or_broken_library_is_recreated_and_user_data_kept() {
    let d = tmp();
    {
        let (s, _) = Store::open(d.path()).unwrap();
        s.conn().execute("INSERT INTO u.settings(key, value) VALUES ('probe', '1')", []).unwrap();
    }
    {
        let c = rusqlite::Connection::open(d.path().join("library.db")).unwrap();
        c.execute_batch("PRAGMA user_version = 99;").unwrap();
    }
    let (s, rep) = Store::open(d.path()).unwrap();
    assert!(rep.library_recreated);
    let v: String = s.conn().query_row("SELECT value FROM u.settings WHERE key='probe'", [], |r| r.get(0)).unwrap();
    assert_eq!(v, "1");
    let broken = std::fs::read_dir(d.path()).unwrap().filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with("library.broken-"));
    assert!(broken, "phải giữ file library cũ dưới tên library.broken-*");
}
```

Unit test cuối `src/schema.rs`:
```rust
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
        let b: i64 = c.query_row("SELECT count(*) FROM sqlite_schema WHERE name='b'", [], |r| r.get(0)).unwrap();
        assert_eq!(b, 0, "bảng của bước lỗi phải bị rollback");
        let backups: Vec<_> = std::fs::read_dir(d.path()).unwrap().filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("user.v1.")).collect();
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
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test open`
Expected: FAIL, lỗi biên dịch `cannot find ... Store`.

- [ ] **Step 3: Viết DDL v1** — `src/schema.rs`

DDL là dữ liệu do spec chốt (mục 3.1, 3.2, 9) — chép nguyên văn. `LIBRARY_MIGRATIONS = &[LIBRARY_V1]`, `USER_MIGRATIONS = &[USER_V1]`.

```rust
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
  missing      INTEGER NOT NULL DEFAULT 0
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
"#;

pub(crate) const USER_V1: &str = r#"
CREATE TABLE sources (
  source_id  INTEGER PRIMARY KEY,
  legacy_id  TEXT UNIQUE,
  kind       TEXT NOT NULL CHECK (kind IN ('directory','system','google_fonts','adobe_fonts','community')),
  name       TEXT NOT NULL,
  path       TEXT,
  enabled    INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  UNIQUE (kind, path)
);
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
```

Ghi chú khác spike: `analyses` lưu JSON nguyên văn của `Metadata`/`VietnameseCoverage`/`Anatomy` (trục chuẩn hoá thuộc Plan 3); `pipeline_state` khoá theo `(hash, face_index)` để ghi được lỗi cấp file (`face_index = -1`) khi chưa có face; `files.container` thay `format` vì core trả `Container`.

- [ ] **Step 4: Viết `migrate`** — `src/schema.rs`

```rust
/// Chạy migration trên schema main của `conn`. Trả về đường dẫn bản sao lưu nếu có.
pub(crate) fn migrate(conn: &Connection, db: &'static str, migs: &[&str], backup_dir: Option<&Path>)
    -> Result<Option<PathBuf>, StoreError>
{
    let latest = migs.len() as u32;
    let cur: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
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
```

- [ ] **Step 5: Viết `Store::open`** — `src/open.rs`

Thứ tự (quyết định thiết kế, không đổi):
1. `create_dir_all(dir)`.
2. **user.db trước** (quý hơn): `Connection::open(dir/user.db)` → `migrate(&uc, "user", USER_MIGRATIONS, Some(&dir/backups))` → **sau đó** mới `PRAGMA journal_mode=WAL` (đổi journal mode là ghi vào header file; làm trước thì `user.db` mới hơn app sẽ bị sửa byte). Lỗi → trả lỗi ngay, chưa đụng `library.db`. Drop `uc`. Lưu `user_backup`.
3. `library.db`: hàm `open_library(path) -> Result<Connection, StoreError>`: mở; nếu `SELECT count(*) FROM sqlite_schema` = 0 thì `PRAGMA page_size=16384` **rồi mới** `PRAGMA journal_mode=WAL`; `migrate(&c, "library", LIBRARY_MIGRATIONS, None)`. Nếu `open_library` trả `NewerSchema` hoặc `Migration`: đóng kết nối, đổi tên `library.db`, `library.db-wal`, `library.db-shm` (nếu có) thành `library.broken-{ms}.db[-wal|-shm]`, gọi lại `open_library` một lần, `library_recreated = true`.
4. Trên kết nối library: `ATTACH DATABASE ?1 AS u` (đường dẫn user.db), `PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000; PRAGMA synchronous=NORMAL; PRAGMA u.synchronous=NORMAL;`.

`now_ms()` dùng `SystemTime::now().duration_since(UNIX_EPOCH)`, lỗi đồng hồ → 0.

`lib.rs`: `mod schema; mod open; pub use open::{Store, OpenReport, LIBRARY_FILE, USER_FILE, BACKUP_DIR}; pub use schema::{LIBRARY_VERSION, USER_VERSION};` (hằng `LIBRARY_VERSION = LIBRARY_MIGRATIONS.len() as u32` đặt trong `schema.rs`).

- [ ] **Step 6: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store`
Expected: `tests/open.rs` 4 passed; unit `schema` 2 passed; test Task 1 vẫn qua.

- [ ] **Step 7: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): schema v1 for library.db/user.db, versioned migration with backup

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Ghi kết quả phân tích (`ingest`), `pipeline_state`, file có vấn đề

**Files:**
- Create: `src/ingest.rs`
- Modify: `src/lib.rs`, `tests/common/mod.rs`
- Test: `tests/ingest.rs`

**Interfaces:**
- Consumes: `Store` (Task 2), `vi_fold` (Task 1), `fontatlas_core::{FileReport, FaceReport, Container, ANALYZER_VERSION}`
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct FaceKey { pub hash: [u8; 32], pub face_index: u32 }`; `FaceKey::from_hex(hex: &str, face_index: u32) -> Result<FaceKey, StoreError>` (`BadHash` nếu không đúng 64 ký tự hex); `FaceKey::hash_hex(&self) -> String`
  - `pub(crate) fn parse_hash(hex: &str) -> Result<[u8; 32], StoreError>`
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct ScannedFile { pub path: PathBuf, pub size: u64, pub mtime_ns: i64 }`
  - `pub enum Analysis { Report(FileReport), Failed { code: String, detail: String }, Known }`
  - `pub struct IngestItem { pub source_id: i64, pub file: ScannedFile, pub hash: [u8; 32], pub analysis: Analysis }`
  - `#[derive(Debug, Default, PartialEq, Eq)] pub struct IngestSummary { pub faces_written: usize, pub failed: usize, pub relinked: usize }`
  - `Store::ingest(&mut self, items: Vec<IngestItem>, seen_at: i64) -> Result<IngestSummary, StoreError>` — **một** transaction
  - `#[derive(Debug, Clone, Serialize)] pub struct ProblemFile { pub hash_hex: String, pub face_index: Option<u32>, pub stage: String, pub code: String, pub detail: Option<String>, pub attempts: u32, pub path: Option<String> }`
  - `Store::problem_files(&self) -> Result<Vec<ProblemFile>, StoreError>`
  - `Store::retry_failed(&mut self) -> Result<usize, StoreError>` — chuyển `failed` → `pending`

- [ ] **Step 1: Bổ sung helper test** — thêm vào `tests/common/mod.rs`

```rust
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
```
`blake3`, `fontatlas-core`, `rusqlite`, `serde_json`, `unicode-normalization` là dependency thường nên test tích hợp dùng trực tiếp, không cần khai báo lại. `FileReport` đã `#[derive(Clone)]` ở core.

- [ ] **Step 2: Viết test thất bại** — `tests/ingest.rs`

```rust
mod common;
use common::*;
use fontatlas_core::ANALYZER_VERSION;
use fontatlas_store::{Analysis, FaceKey, IngestItem, ScannedFile};

#[test]
fn report_writes_file_face_analysis_and_state() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("open_sans.ttf");
    let sum = s.ingest(vec![item(1, "/fonts/OpenSans.ttf", &r)], 1000).unwrap();
    assert_eq!(sum.faces_written, 1);
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations WHERE missing=0"), 1);
    let (ver, src, meta): (String, String, String) = s.conn().query_row(
        "SELECT analyzer_version, source, metadata_json FROM analyses", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(ver, ANALYZER_VERSION);
    assert_eq!(src, "local");
    assert!(meta.contains("Open Sans"));
    let (fam, key, vi): (String, String, String) = s.conn().query_row(
        "SELECT family_name, family_key, vi_status FROM faces", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(fam, "Open Sans");
    assert_eq!(key, "open sans");
    assert_eq!(vi, "full");
    let st: String = s.conn().query_row(
        "SELECT status FROM pipeline_state WHERE face_index=-1 AND stage='load'", [], |r| r.get(0)).unwrap();
    assert_eq!(st, "done");
}

#[test]
fn reingest_is_idempotent_and_face_id_stable() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 1).unwrap();
    let id1 = count(&s, "SELECT face_id FROM faces");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 2).unwrap();
    assert_eq!(count(&s, "SELECT face_id FROM faces"), id1);
    assert_eq!(count(&s, "SELECT count(*) FROM analyses"), 1);
    assert_eq!(count(&s, "SELECT count(*) FROM faces_fts"), 1);
}

#[test]
fn old_local_analysis_versions_are_replaced() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 1).unwrap();
    s.conn().execute("UPDATE analyses SET analyzer_version='core-0.0.0'", []).unwrap();
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 2).unwrap();
    let vers: Vec<String> = s.conn().prepare("SELECT analyzer_version FROM analyses").unwrap()
        .query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect();
    assert_eq!(vers, vec![ANALYZER_VERSION.to_string()]);
}

#[test]
fn collection_report_gives_one_row_per_face() {
    let d = tmp();
    let mut s = open_store(d.path());
    let mut r = fake_report(&report("roboto.ttf"), "ttc", "Duo");
    let mut second = r.faces[0].clone();
    second.index = 1;
    second.metadata.subfamily = Some("Bold".into());
    r.faces.push(second);
    r.container = fontatlas_core::Container::Collection;
    s.ingest(vec![item(1, "/a/Duo.ttc", &r)], 1).unwrap();
    let idx: Vec<i64> = s.conn().prepare("SELECT face_index FROM faces ORDER BY face_index").unwrap()
        .query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect();
    assert_eq!(idx, vec![0, 1]);
    assert_eq!(count(&s, "SELECT num_faces FROM files"), 2);
    assert_eq!(count(&s, "SELECT count(*) FROM files WHERE container='collection'"), 1);
}

#[test]
fn failure_is_recorded_with_code_path_and_attempts() {
    let d = tmp();
    let mut s = open_store(d.path());
    let h = *blake3::hash(b"rac").as_bytes();
    let fail = || IngestItem {
        source_id: 1,
        file: ScannedFile { path: "/a/broken.ttf".into(), size: 3, mtime_ns: 1 },
        hash: h,
        analysis: Analysis::Failed { code: "not_a_font".into(), detail: "không phải file font".into() },
    };
    assert_eq!(s.ingest(vec![fail()], 1).unwrap().failed, 1);
    s.ingest(vec![fail()], 2).unwrap();
    let p = s.problem_files().unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].code, "not_a_font");
    assert_eq!(p[0].stage, "load");
    assert_eq!(p[0].face_index, None);
    assert_eq!(p[0].attempts, 2);
    assert_eq!(p[0].path.as_deref(), Some("/a/broken.ttf"));
    assert_eq!(count(&s, "SELECT count(*) FROM faces"), 0);
    assert_eq!(s.retry_failed().unwrap(), 1);
    assert!(s.problem_files().unwrap().is_empty());
}

#[test]
fn known_relinks_and_unknown_known_is_error() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    s.ingest(vec![item(1, "/a/Roboto.ttf", &r)], 1).unwrap();
    let known = IngestItem { analysis: Analysis::Known, ..item(1, "/b/Roboto copy.ttf", &r) };
    assert_eq!(s.ingest(vec![known], 2).unwrap().relinked, 1);
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2);
    assert_eq!(count(&s, "SELECT count(*) FROM files"), 1);
    let ghost = IngestItem { hash: [7; 32], analysis: Analysis::Known, ..item(1, "/c/x.ttf", &r) };
    assert_eq!(s.ingest(vec![ghost], 3).unwrap_err().code(), "not_found");
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 2, "lô lỗi phải rollback");
}

#[test]
fn report_hash_must_match_item_hash() {
    let d = tmp();
    let mut s = open_store(d.path());
    let r = report("roboto.ttf");
    let bad = IngestItem { hash: [1; 32], ..item(1, "/a/R.ttf", &r) };
    assert_eq!(s.ingest(vec![bad], 1).unwrap_err().code(), "bad_hash");
}

#[test]
fn face_key_hex_roundtrip_and_validation() {
    let k = FaceKey::from_hex(&"ab".repeat(32), 3).unwrap();
    assert_eq!(k.hash_hex(), "ab".repeat(32));
    assert_eq!(FaceKey::from_hex("xyz", 0).unwrap_err().code(), "bad_hash");
}
```

- [ ] **Step 3: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test ingest`
Expected: FAIL, lỗi biên dịch (`Analysis`, `IngestItem`, `ingest` chưa có).

- [ ] **Step 4: Viết implementation** — `src/ingest.rs`

`Store::ingest`: mở `self.conn.transaction()`; với mỗi item:
1. `file_id = upsert_file(tx, hash, size, now)` — `INSERT INTO files(hash,size,first_seen_at) VALUES (?,?,?) ON CONFLICT(hash) DO NOTHING`, rồi `SELECT file_id`. Riêng `Analysis::Known`: nếu không có `files` → `StoreError::NotFound(hex)`.
2. Upsert location: `INSERT INTO file_locations(file_id,source_id,path,size,mtime_ns,last_seen_at,missing) VALUES (...,0) ON CONFLICT(path) DO UPDATE SET file_id=excluded.file_id, source_id=excluded.source_id, size=excluded.size, mtime_ns=excluded.mtime_ns, last_seen_at=excluded.last_seen_at, missing=0`. Đường dẫn lưu bằng `path.to_string_lossy()`.
3. `Report(r)`: kiểm tra `parse_hash(&r.hash_hex)? == item.hash` (khác → `BadHash`); `UPDATE files SET container=?, num_faces=?`; với mỗi `FaceReport`: upsert `faces` theo `UNIQUE(file_id, face_index)` bằng `ON CONFLICT DO UPDATE SET` toàn bộ cột dẫn xuất (giữ `face_id`); `INSERT OR REPLACE INTO analyses(... 'local', NULL, json...)` rồi `DELETE FROM analyses WHERE hash=? AND face_index=? AND source='local' AND analyzer_version<>?`; `set_state(hash, face.index, 'analysis', 'done', ANALYZER_VERSION, None)`. Cuối cùng `DELETE FROM faces WHERE file_id=? AND face_index >= num_faces` và `set_state(hash, -1, 'load', 'done', ANALYZER_VERSION, None)`.
4. `Failed{code,detail}`: `set_state(hash, -1, 'load', 'failed', ANALYZER_VERSION, Some((code, detail)))` với `attempts = attempts + 1`.

Ánh xạ `FaceReport` → cột `faces` (hàm `fn face_row(f: &FaceReport) -> FaceCols`):
- `family_name` = `typographic_family ?? family ?? postscript_name ?? "(không tên)"`; `subfamily` = `typographic_subfamily ?? subfamily`.
- `is_italic` = `italic_angle.is_some_and(|a| a.abs() >= 0.5)` **hoặc** `vi_fold(subfamily)` chứa `"italic"` hay `"oblique"`.
- `is_variable` = `!axes.is_empty()`.
- `outline_format`, `license_class`, `vi_status`: chuỗi `serde_json::to_value(x)` dạng snake_case (trùng tên serde của core).
- `vi_missing` = `vietnamese.missing.len()`.
- `family_key` = `vi_fold(family_name)`.
- `search_text` = `vi_fold` của các giá trị khác rỗng, bỏ trùng, nối bằng `" "`, theo thứ tự: typographic_family, family, typographic_subfamily, subfamily, full_name, postscript_name, manufacturer, designer.
- JSON: `serde_json::to_string(&f.metadata)`, `(&f.vietnamese)`, `(&f.anatomy)`. Các kiểu này chỉ gồm struct/enum/Vec/String/số nên serialize không thể lỗi; viết `.unwrap_or_else(|_| "null".to_string())` để giữ quy tắc không `unwrap`.

`set_state`: `INSERT INTO pipeline_state(hash,face_index,stage,status,version,error_code,error_detail,attempts,updated_at) VALUES (?,?,?,?,?,?,?,?,?) ON CONFLICT(hash,face_index,stage) DO UPDATE SET status=excluded.status, version=excluded.version, error_code=excluded.error_code, error_detail=excluded.error_detail, attempts=CASE WHEN excluded.status='failed' THEN pipeline_state.attempts+1 ELSE 0 END, updated_at=excluded.updated_at` (lần chèn đầu `attempts` = 1 nếu `failed`, 0 nếu không).

`problem_files`: `SELECT p.hash, p.face_index, p.stage, p.error_code, p.error_detail, p.attempts, (SELECT l.path FROM files fi JOIN file_locations l USING(file_id) WHERE fi.hash=p.hash ORDER BY l.missing, l.location_id LIMIT 1) FROM pipeline_state p WHERE p.status='failed' ORDER BY p.updated_at DESC`. `face_index = -1` → `None`.

`retry_failed`: `UPDATE pipeline_state SET status='pending' WHERE status='failed'` → số dòng.

`lib.rs`: `mod ingest; pub use ingest::{Analysis, FaceKey, IngestItem, IngestSummary, ProblemFile, ScannedFile};`.

- [ ] **Step 5: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test ingest`
Expected: `ok. 8 passed`.

- [ ] **Step 6: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): ingest FileReport into files/faces/analyses with pipeline_state

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Dữ liệu người dùng — nguồn, favorite, collection, tag

**Files:**
- Create: `src/user.rs`
- Modify: `src/lib.rs`
- Test: `tests/user.rs`

**Interfaces:**
- Consumes: `Store`, `FaceKey`, `vi_fold`, `now_ms`
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")] pub enum SourceKind { Directory, System, GoogleFonts, AdobeFonts, Community }`; `SourceKind::as_str(&self) -> &'static str`; `SourceKind::parse(&str) -> Option<SourceKind>`
  - `#[derive(Debug, Clone, Serialize)] #[serde(rename_all = "camelCase")] pub struct Source { pub id: i64, pub kind: SourceKind, pub name: String, pub path: Option<String>, pub enabled: bool, pub created_at: i64 }`
  - `Store::add_source(&mut self, kind: SourceKind, name: &str, path: Option<&Path>) -> Result<i64, StoreError>` — trùng `(kind, path)` trả id cũ
  - `Store::sources(&self) -> Result<Vec<Source>, StoreError>`; `Store::source(&self, id: i64) -> Result<Source, StoreError>` (`NotFound`)
  - `Store::set_favorite(&mut self, key: &FaceKey, on: bool) -> Result<(), StoreError>`; `Store::favorites(&self) -> Result<Vec<FaceKey>, StoreError>`
  - `#[derive(Debug, Clone, Serialize)] #[serde(rename_all = "camelCase")] pub struct Collection { pub id: i64, pub name: String, pub color: Option<String>, pub item_count: u32 }`
  - `Store::create_collection(&mut self, name: &str, color: Option<&str>) -> Result<i64, StoreError>`; `rename_collection(&mut self, id: i64, name: &str) -> Result<(), StoreError>` (`NotFound`); `delete_collection(&mut self, id: i64) -> Result<(), StoreError>`; `add_to_collection(&mut self, id: i64, keys: &[FaceKey]) -> Result<usize, StoreError>` (số dòng mới); `remove_from_collection(&mut self, id: i64, keys: &[FaceKey]) -> Result<usize, StoreError>`; `collections(&self) -> Result<Vec<Collection>, StoreError>` (sắp theo `position, name`)
  - `#[derive(Debug, Clone, Serialize)] #[serde(rename_all = "camelCase")] pub struct Tag { pub id: i64, pub name: String, pub face_count: u32 }`
  - `Store::tag(&mut self, name: &str) -> Result<i64, StoreError>` (lấy hoặc tạo theo `vi_fold(name)`; tên rỗng sau gập → `NotFound("tag rỗng")`); `tag_faces(&mut self, tag_id: i64, keys: &[FaceKey]) -> Result<usize, StoreError>`; `untag_faces(...) -> Result<usize, StoreError>`; `tags(&self) -> Result<Vec<Tag>, StoreError>`; `tags_of(&self, key: &FaceKey) -> Result<Vec<Tag>, StoreError>` (cả hai sắp theo `name_key`)
  - `pub(crate) fn tag_in(conn: &Connection, name: &str, now: i64) -> Result<Option<i64>, StoreError>` (dùng lại ở Task 7)

`remove_source` thuộc Task 6 vì chạm `file_locations`.

- [ ] **Step 1: Viết test thất bại** — `tests/user.rs`

```rust
mod common;
use common::*;
use fontatlas_store::{FaceKey, SourceKind};

fn key(n: u8, i: u32) -> FaceKey {
    FaceKey { hash: [n; 32], face_index: i }
}

#[test]
fn source_dedupes_by_kind_and_path() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.add_source(SourceKind::Directory, "Fonts", Some(std::path::Path::new("/x/Fonts"))).unwrap();
    let b = s.add_source(SourceKind::Directory, "Tên khác", Some(std::path::Path::new("/x/Fonts"))).unwrap();
    assert_eq!(a, b);
    assert_eq!(s.sources().unwrap().len(), 1);
    assert_eq!(s.source(a).unwrap().kind, SourceKind::Directory);
    assert_eq!(s.source(999).unwrap_err().code(), "not_found");
}

#[test]
fn favorites_are_idempotent_and_survive_library_rebuild() {
    let d = tmp();
    {
        let mut s = open_store(d.path());
        s.set_favorite(&key(1, 0), true).unwrap();
        s.set_favorite(&key(1, 0), true).unwrap();
        s.set_favorite(&key(2, 1), true).unwrap();
        s.set_favorite(&key(2, 1), false).unwrap();
        assert_eq!(s.favorites().unwrap(), vec![key(1, 0)]);
    }
    for f in ["library.db", "library.db-wal", "library.db-shm"] {
        let _ = std::fs::remove_file(d.path().join(f));
    }
    let s = open_store(d.path());
    assert_eq!(s.favorites().unwrap(), vec![key(1, 0)]);
}

#[test]
fn collections_crud_and_cascade() {
    let d = tmp();
    let mut s = open_store(d.path());
    let c = s.create_collection("Menu quán phở", Some("#c0392b")).unwrap();
    assert_eq!(s.add_to_collection(c, &[key(1, 0), key(2, 0)]).unwrap(), 2);
    assert_eq!(s.add_to_collection(c, &[key(1, 0)]).unwrap(), 0);
    s.rename_collection(c, "Menu").unwrap();
    assert_eq!(s.rename_collection(777, "x").unwrap_err().code(), "not_found");
    let all = s.collections().unwrap();
    assert_eq!((all[0].name.as_str(), all[0].item_count), ("Menu", 2));
    assert_eq!(s.remove_from_collection(c, &[key(2, 0)]).unwrap(), 1);
    s.delete_collection(c).unwrap();
    assert_eq!(count(&s, "SELECT count(*) FROM u.collection_items"), 0);
}

#[test]
fn tags_dedupe_by_folded_name_and_keep_first_spelling() {
    let d = tmp();
    let mut s = open_store(d.path());
    let a = s.tag("Cổ điển").unwrap();
    let b = s.tag("co dien").unwrap();
    let c = s.tag("  CỔ ĐIỂN ").unwrap();
    assert_eq!((a, a), (b, c));
    assert_eq!(s.tag("   ").unwrap_err().code(), "not_found");
    assert_eq!(s.tag_faces(a, &[key(1, 0), key(1, 1)]).unwrap(), 2);
    let tags = s.tags().unwrap();
    assert_eq!((tags[0].name.as_str(), tags[0].face_count), ("Cổ điển", 2));
    assert_eq!(s.tags_of(&key(1, 1)).unwrap()[0].id, a);
    assert_eq!(s.untag_faces(a, &[key(1, 1)]).unwrap(), 1);
}
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test user`
Expected: FAIL, lỗi biên dịch `no method named add_source`.

- [ ] **Step 3: Viết implementation** — `src/user.rs`

Mọi câu SQL ghi vào `u.<bảng>`. Mỗi hàm nhận `&[FaceKey]` chạy trong một transaction và đếm bằng `execute` (`INSERT OR IGNORE` → 0 hoặc 1). `add_source`: `INSERT INTO u.sources(kind,name,path,created_at) VALUES (...) ON CONFLICT(kind,path) DO NOTHING` rồi `SELECT source_id ... WHERE kind=? AND path IS ?` (dùng `IS` để khớp cả `NULL`). `tag_in`: `k = vi_fold(name)`; rỗng → `Ok(None)`; `INSERT INTO u.tags(name,name_key,created_at) VALUES (trim(name), k, now) ON CONFLICT(name_key) DO NOTHING`; `SELECT tag_id WHERE name_key=?`. `favorites()` sắp theo `created_at, hash, face_index`.

`lib.rs`: `mod user; pub use user::{Collection, Source, SourceKind, Tag};`.

- [ ] **Step 4: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test user`
Expected: `ok. 4 passed`.

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): sources, favorites, collections, tags keyed by (hash, face_index)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: `query_faces` — tìm, lọc, sắp xếp, phân trang phía backend

**Files:**
- Create: `src/query.rs`
- Modify: `src/lib.rs`
- Test: `tests/query.rs`

**Interfaces:**
- Consumes: `Store`, `text_match`/`TextMatch` (Task 1), `FaceKey`, bảng của Task 2–4
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)] #[serde(rename_all = "snake_case")] pub enum SortKey { #[default] Family, Weight, Added }`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)] #[serde(rename_all = "snake_case")] pub enum ViFilter { Full, Supported }` — `Full` = `vi_status='full'`; `Supported` = `supports_vi=1`
  - `#[derive(Debug, Clone, Default, Deserialize)] #[serde(rename_all = "camelCase", default)] pub struct FaceQuery { pub text: Option<String>, pub vi: Option<ViFilter>, pub license: Vec<String>, pub variable: Option<bool>, pub italic: Option<bool>, pub source_ids: Vec<i64>, pub favorites_only: bool, pub collection_id: Option<i64>, pub tag_ids: Vec<i64>, pub include_missing: bool, pub sort: SortKey, pub descending: bool, pub offset: u32, pub limit: u32 }`
  - `#[derive(Debug, Clone, Serialize)] #[serde(rename_all = "camelCase")] pub struct FaceRow { pub face_id: i64, pub hash_hex: String, pub face_index: u32, pub family: String, pub subfamily: Option<String>, pub full_name: Option<String>, pub weight_class: Option<u16>, pub width_class: Option<u16>, pub is_italic: bool, pub is_variable: bool, pub vi_status: String, pub supports_vi: bool, pub vi_missing: u32, pub license_class: String, pub path: Option<String>, pub missing: bool, pub is_favorite: bool, pub added_at: i64 }`
  - `#[derive(Debug, Clone, Serialize)] #[serde(rename_all = "camelCase")] pub struct FacePage { pub total: u64, pub offset: u32, pub items: Vec<FaceRow> }`
  - `pub const DEFAULT_LIMIT: u32 = 100`, `pub const MAX_LIMIT: u32 = 1000`
  - `Store::query_faces(&self, q: &FaceQuery) -> Result<FacePage, StoreError>`

- [ ] **Step 1: Viết test thất bại** — `tests/query.rs`

```rust
mod common;
use common::*;
use fontatlas_store::{FaceKey, FaceQuery, SortKey, Store, ViFilter};

/// 5 font mẫu + 2 face giả tên tiếng Việt.
fn seeded() -> (tempfile::TempDir, Store) {
    let d = tmp();
    let mut s = open_store(d.path());
    let mut items: Vec<_> = SAMPLES.iter().map(|n| item(1, &format!("/lib/{n}"), &report(n))).collect();
    let base = report("open_sans.ttf");
    items.push(item(1, "/lib/dep.ttf", &fake_report(&base, "dep", "Đẹp Sans")));
    items.push(item(1, "/lib/hn.ttf", &fake_report(&base, "hn", "Hà Nội Grotesk")));
    s.ingest(items, 10).unwrap();
    (d, s)
}

fn fams(s: &Store, q: &FaceQuery) -> Vec<String> {
    s.query_faces(q).unwrap().items.into_iter().map(|r| r.family).collect()
}
fn text(t: &str) -> FaceQuery {
    FaceQuery { text: Some(t.into()), ..Default::default() }
}

#[test]
fn search_folds_vietnamese_both_ways() {
    let (_d, s) = seeded();
    assert_eq!(fams(&s, &text("dep")), vec!["Đẹp Sans"]);
    assert_eq!(fams(&s, &text("ĐẸP")), vec!["Đẹp Sans"]);
    assert_eq!(fams(&s, &text("Ha Noi")), vec!["Hà Nội Grotesk"]);
    assert_eq!(fams(&s, &text("oi gro")), vec!["Hà Nội Grotesk"]); // chuỗi con giữa từ
    assert_eq!(fams(&s, &text("No\u{0302}\u{0323}i")), vec!["Hà Nội Grotesk"]); // "Nội" dạng NFD
}

#[test]
fn short_and_hostile_queries_work() {
    let (_d, s) = seeded();
    assert!(fams(&s, &text("pl")).contains(&"Playfair Display".to_string()));
    assert!(fams(&s, &text("ro")).contains(&"Roboto".to_string()));
    for q in ["\"", "%", "_", "*", "a\"b", "NEAR(", "') OR 1=1 --"] {
        assert!(s.query_faces(&text(q)).is_ok(), "truy vấn {q:?} không được lỗi");
    }
    assert_eq!(s.query_faces(&text("%")).unwrap().total, 0);
}

#[test]
fn pagination_total_and_stable_order() {
    let (_d, s) = seeded();
    let all = s.query_faces(&FaceQuery::default()).unwrap();
    assert_eq!(all.total, 7);
    let p2 = s.query_faces(&FaceQuery { offset: 2, limit: 2, ..Default::default() }).unwrap();
    assert_eq!(p2.total, 7);
    assert_eq!(p2.items.len(), 2);
    assert_eq!(p2.items[0].face_id, all.items[2].face_id);
    let keys: Vec<String> = all.items.iter().map(|r| fontatlas_store::vi_fold(&r.family)).collect();
    assert!(keys.windows(2).all(|p| p[0] <= p[1]), "{keys:?}");
    let big = s.query_faces(&FaceQuery { limit: 50_000, ..Default::default() }).unwrap();
    assert_eq!(big.items.len(), 7); // limit bị kẹp, không lỗi
}

#[test]
fn sort_by_weight_descending() {
    let (_d, s) = seeded();
    let rows = s.query_faces(&FaceQuery { sort: SortKey::Weight, descending: true, ..Default::default() }).unwrap().items;
    let w: Vec<u16> = rows.iter().filter_map(|r| r.weight_class).collect();
    assert!(w.windows(2).all(|p| p[0] >= p[1]), "{w:?}");
}

#[test]
fn filters_vi_license_favorites_collection_tags() {
    let (_d, mut s) = seeded();
    let all = s.query_faces(&FaceQuery::default()).unwrap().items;
    let roboto = all.iter().find(|r| r.family == "Roboto").unwrap();
    let k = FaceKey::from_hex(&roboto.hash_hex, roboto.face_index).unwrap();

    let vi = s.query_faces(&FaceQuery { vi: Some(ViFilter::Supported), ..Default::default() }).unwrap();
    assert!(vi.items.iter().all(|r| r.supports_vi));
    let full = fams(&s, &FaceQuery { vi: Some(ViFilter::Full), ..Default::default() });
    assert!(!full.contains(&"Roboto".to_string()), "Roboto là precomposed_only (đã kiểm ở core)");
    assert!(full.contains(&"Open Sans".to_string()));

    let ofl = s.query_faces(&FaceQuery { license: vec!["ofl".into()], ..Default::default() }).unwrap();
    assert!(ofl.items.iter().all(|r| r.license_class == "ofl"));

    s.set_favorite(&k, true).unwrap();
    let fav = s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap();
    assert_eq!(fav.items.len(), 1);
    assert!(fav.items[0].is_favorite);

    let c = s.create_collection("C", None).unwrap();
    s.add_to_collection(c, &[k]).unwrap();
    assert_eq!(s.query_faces(&FaceQuery { collection_id: Some(c), ..Default::default() }).unwrap().total, 1);

    let t1 = s.tag("serif").unwrap();
    let t2 = s.tag("tiêu đề").unwrap();
    s.tag_faces(t1, &[k]).unwrap();
    assert_eq!(s.query_faces(&FaceQuery { tag_ids: vec![t1], ..Default::default() }).unwrap().total, 1);
    assert_eq!(s.query_faces(&FaceQuery { tag_ids: vec![t1, t2], ..Default::default() }).unwrap().total, 0, "nhiều tag = AND");
}

#[test]
fn missing_files_hidden_unless_requested() {
    let (_d, s) = seeded();
    s.conn().execute("UPDATE file_locations SET missing=1 WHERE path='/lib/roboto.ttf'", []).unwrap();
    assert_eq!(s.query_faces(&FaceQuery::default()).unwrap().total, 6);
    let with = s.query_faces(&FaceQuery { include_missing: true, ..Default::default() }).unwrap();
    assert_eq!(with.total, 7);
    assert!(with.items.iter().any(|r| r.family == "Roboto" && r.missing));
}

#[test]
fn query_deserializes_from_camel_case_json() {
    let q: FaceQuery = serde_json::from_str(r#"{"text":"dep","favoritesOnly":true,"sort":"weight","limit":20}"#).unwrap();
    assert!(q.favorites_only && q.sort == SortKey::Weight && q.limit == 20);
}

#[test]
#[ignore = "hiệu năng: cargo test -p fontatlas-store --release --test query -- --ignored --nocapture"]
fn perf_50k_faces_under_50ms() {
    let d = tmp();
    let mut s = open_store(d.path());
    let base = report("open_sans.ttf");
    for chunk in 0..50 {
        let items = (0..1000).map(|i| {
            let n = chunk * 1000 + i;
            item(1, &format!("/big/{n}.ttf"), &fake_report(&base, &format!("f{n}"), &format!("Họ Phông {n}")))
        }).collect();
        s.ingest(items, 1).unwrap();
    }
    let qs = [
        FaceQuery::default(),
        FaceQuery { offset: 40_000, ..Default::default() },
        text("phong 4999"),
        FaceQuery { vi: Some(ViFilter::Full), sort: SortKey::Weight, ..Default::default() },
    ];
    for q in &qs {
        let t = std::time::Instant::now();
        let p = s.query_faces(q).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        eprintln!("{q:?} -> total {} in {ms:.1} ms", p.total);
        assert!(ms < 50.0, "S6: < 50 ms trên 50.000 font");
    }
}
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test query`
Expected: FAIL, lỗi biên dịch `cannot find type FaceQuery`.

- [ ] **Step 3: Viết implementation** — `src/query.rs`

Dựng `WHERE` động với `Vec<Box<dyn rusqlite::ToSql>>` và tham số đánh số tự tăng. Khung câu:

```sql
SELECT f.face_id, fi.hash, f.face_index, f.family_name, f.subfamily, f.full_name, f.weight_class, f.width_class,
       f.is_italic, f.is_variable, f.vi_status, f.supports_vi, f.vi_missing, f.license_class,
       (SELECT l.path FROM file_locations l WHERE l.file_id = f.file_id ORDER BY l.missing, l.location_id LIMIT 1) AS path,
       NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0) AS missing,
       EXISTS (SELECT 1 FROM u.favorites fv WHERE fv.hash = fi.hash AND fv.face_index = f.face_index) AS fav,
       fi.first_seen_at
FROM faces f JOIN files fi ON fi.file_id = f.file_id
WHERE <điều kiện>
ORDER BY <sắp xếp>
LIMIT ? OFFSET ?
```

Điều kiện (mỗi điều kiện chỉ thêm khi bộ lọc có giá trị):
- text: `TextMatch::Fts(m)` → `f.face_id IN (SELECT rowid FROM faces_fts WHERE faces_fts MATCH ?)`; `TextMatch::Prefix{start, word}` → `(f.search_text LIKE ? ESCAPE '\' OR f.search_text LIKE ? ESCAPE '\')`; `text_match` trả `None` → bỏ qua.
- `vi`: `f.vi_status = 'full'` | `f.supports_vi = 1`.
- `license` khác rỗng: `f.license_class IN (?, …)`.
- `variable`/`italic`: `f.is_variable = ?` / `f.is_italic = ?`.
- `source_ids` khác rỗng: `EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.source_id IN (…))`.
- `favorites_only`: `EXISTS (… u.favorites …)`.
- `collection_id`: `EXISTS (SELECT 1 FROM u.collection_items ci WHERE ci.collection_id = ? AND ci.hash = fi.hash AND ci.face_index = f.face_index)`.
- mỗi `tag_id`: một `EXISTS (SELECT 1 FROM u.face_tags ft WHERE ft.tag_id = ? AND ft.hash = fi.hash AND ft.face_index = f.face_index)` (AND).
- `!include_missing`: `EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = f.file_id AND l.missing = 0)`.

Sắp xếp (`dir` = `DESC` nếu `descending`, luôn có `f.face_id` cuối để ổn định):
- `Family` → `f.family_key {dir}, f.weight_class, f.is_italic, f.face_id`
- `Weight` → `f.weight_class {dir}, f.family_key, f.face_id`
- `Added` → `fi.first_seen_at {dir}, f.face_id`

`limit` = `DEFAULT_LIMIT` nếu 0, kẹp ≤ `MAX_LIMIT`. `total` = `SELECT count(*) FROM faces f JOIN files fi … WHERE <cùng điều kiện>` với cùng tham số. `hash_hex` = `blake3::Hash::from_bytes(arr).to_hex()`.

`lib.rs`: `mod query; pub use query::{DEFAULT_LIMIT, FacePage, FaceQuery, FaceRow, MAX_LIMIT, SortKey, ViFilter};`.

- [ ] **Step 4: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test query`
Expected: `ok. 7 passed; 1 ignored`.

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --release --test query -- --ignored --nocapture`
Expected: 4 dòng thời gian, đều < 50 ms. Nếu `offset 40000` vượt ngưỡng: ghi số vào báo cáo Task 8 và đề xuất phân trang keyset cho Plan 5, **không** đổi API trong plan này.

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): query_faces with folded FTS search, filters, sort, pagination

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Quét lại theo diff, xoá nguồn, dọn rác

**Files:**
- Create: `src/rescan.rs`
- Modify: `src/lib.rs`
- Test: `tests/rescan.rs`

**Interfaces:**
- Consumes: `Store::ingest`, `IngestItem`, `Analysis`, `ScannedFile` (Task 3); `Store::source`, `add_source` (Task 4); `fontatlas_core::{analyze_paths, ANALYZER_VERSION, CoreError}`
- Produces:
  - `pub const FONT_EXTENSIONS: [&str; 6] = ["ttf", "otf", "ttc", "otc", "woff", "woff2"]`
  - `pub fn walk_fonts(root: &Path) -> Vec<ScannedFile>` — đệ quy, không theo symlink, bỏ thư mục/file bắt đầu bằng `.`, đuôi không phân biệt hoa thường, bỏ qua thư mục không đọc được; sắp theo đường dẫn
  - `#[derive(Debug, Default, Clone, Serialize)] #[serde(rename_all = "camelCase")] pub struct RescanStats { pub seen: usize, pub unchanged: usize, pub relinked: usize, pub analyzed: usize, pub failed: usize, pub missing: usize, pub unreadable: Vec<(String, String)> }` — `(đường dẫn, mã lỗi)`
  - `pub const INGEST_BATCH: usize = 256`
  - `Store::rescan_source(&mut self, source_id: i64) -> Result<RescanStats, StoreError>` — `NotFound` nếu nguồn không tồn tại hoặc không có `path`
  - `Store::remove_source(&mut self, source_id: i64) -> Result<(), StoreError>`
  - `Store::gc(&mut self) -> Result<usize, StoreError>` — số `files` đã xoá

- [ ] **Step 1: Viết test thất bại** — `tests/rescan.rs`

```rust
mod common;
use common::*;
use fontatlas_store::{FaceKey, FaceQuery, SourceKind, Store, walk_fonts};
use std::path::Path;

fn setup() -> (tempfile::TempDir, std::path::PathBuf, Store, i64) {
    let d = tmp();
    let fonts = d.path().join("Fonts");
    std::fs::create_dir_all(fonts.join("sub")).unwrap();
    for n in SAMPLES {
        std::fs::copy(core_fixture(n), fonts.join(n)).unwrap();
    }
    std::fs::copy(core_fixture("open_sans.woff2"), fonts.join("sub/OPEN_SANS.WOFF2")).unwrap();
    std::fs::write(fonts.join("notes.txt"), "x").unwrap();
    std::fs::write(fonts.join(".hidden.ttf"), "x").unwrap();
    let db = d.path().join("db");
    let mut s = open_store(&db);
    let src = s.add_source(SourceKind::Directory, "Fonts", Some(&fonts)).unwrap();
    (d, fonts, s, src)
}

fn total(s: &Store) -> u64 {
    s.query_faces(&FaceQuery::default()).unwrap().total
}

#[test]
fn walk_filters_extensions_hidden_and_sorts() {
    let (_d, fonts, _s, _) = setup();
    let w = walk_fonts(&fonts);
    assert_eq!(w.len(), 6);
    assert!(w.windows(2).all(|p| p[0].path < p[1].path));
    assert!(w.iter().all(|f| !f.path.to_string_lossy().contains(".hidden")));
}

#[test]
fn first_scan_then_unchanged_scan() {
    let (_d, _f, mut s, src) = setup();
    let a = s.rescan_source(src).unwrap();
    assert_eq!((a.seen, a.analyzed, a.failed, a.unchanged), (6, 6, 0, 0));
    assert_eq!(total(&s), 6);
    let b = s.rescan_source(src).unwrap();
    assert_eq!((b.unchanged, b.analyzed, b.relinked), (6, 0, 0));
}

#[test]
fn rename_relinks_without_reanalysis_and_keeps_favorite() {
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    let row = s.query_faces(&FaceQuery { text: Some("roboto".into()), ..Default::default() }).unwrap().items.remove(0);
    let k = FaceKey::from_hex(&row.hash_hex, row.face_index).unwrap();
    s.set_favorite(&k, true).unwrap();
    std::fs::create_dir_all(fonts.join("moved")).unwrap();
    std::fs::rename(fonts.join("roboto.ttf"), fonts.join("moved/Roboto Renamed.ttf")).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!((st.relinked, st.analyzed, st.missing), (1, 0, 1));
    let fav = s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap().items;
    assert_eq!(fav.len(), 1);
    assert!(fav[0].path.as_deref().unwrap().ends_with("Roboto Renamed.ttf"));
    assert!(!fav[0].missing);
}

#[test]
fn duplicate_in_two_sources_is_one_face() {
    let (d, _fonts, mut s, src) = setup();
    let other = d.path().join("Other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), other.join("roboto-copy.ttf")).unwrap();
    let src2 = s.add_source(SourceKind::Directory, "Other", Some(&other)).unwrap();
    s.rescan_source(src).unwrap();
    let st = s.rescan_source(src2).unwrap();
    assert_eq!((st.relinked, st.analyzed), (1, 0));
    assert_eq!(total(&s), 6, "cùng hash chỉ là một face");
    assert_eq!(count(&s, "SELECT count(*) FROM file_locations"), 7);
    s.remove_source(src).unwrap();
    let left = s.query_faces(&FaceQuery::default()).unwrap().items;
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].family, "Roboto");
    assert_eq!(count(&s, "SELECT count(*) FROM u.sources"), 1);
}

#[test]
fn removed_file_marked_missing_and_comes_back() {
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    let bytes = std::fs::read(fonts.join("roboto.ttf")).unwrap();
    std::fs::remove_file(fonts.join("roboto.ttf")).unwrap();
    assert_eq!(s.rescan_source(src).unwrap().missing, 1);
    assert_eq!(total(&s), 5);
    std::fs::write(fonts.join("roboto.ttf"), bytes).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.analyzed, 0, "nội dung cũ đã có kết quả");
    assert_eq!(total(&s), 6);
}

#[test]
fn changed_content_same_path_follows_new_hash() {
    let (_d, fonts, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    std::fs::copy(core_fixture("playfair_display.ttf"), fonts.join("roboto.ttf")).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.unchanged, 5);
    assert_eq!(st.relinked, 1, "nội dung mới trùng hash Playfair đã có");
    assert_eq!(total(&s), 5, "Roboto không còn vị trí nào, Playfair ở 2 vị trí");
}

#[test]
fn broken_file_is_reported_once_and_retry_reanalyzes() {
    let (_d, fonts, mut s, src) = setup();
    std::fs::write(fonts.join("broken.ttf"), b"not a font at all").unwrap();
    let a = s.rescan_source(src).unwrap();
    assert_eq!(a.failed, 1);
    let p = s.problem_files().unwrap();
    assert_eq!(p[0].code, "not_a_font");
    assert!(p[0].path.as_deref().unwrap().ends_with("broken.ttf"));
    assert_eq!(s.rescan_source(src).unwrap().failed, 0, "lỗi cùng phiên bản không chạy lại");
    s.retry_failed().unwrap();
    assert_eq!(s.rescan_source(src).unwrap().failed, 1);
}

#[test]
fn analyzer_version_change_reanalyzes_only_stale() {
    let (_d, _f, mut s, src) = setup();
    s.rescan_source(src).unwrap();
    s.conn().execute(
        "UPDATE pipeline_state SET version='core-0.0.0' WHERE face_index=-1 AND hash=(SELECT hash FROM files LIMIT 1)", []).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!((st.analyzed, st.unchanged), (1, 5));
}

#[test]
fn rescan_never_writes_user_db() {
    let (_d, fonts, mut s, src) = setup();
    s.set_favorite(&FaceKey { hash: [3; 32], face_index: 0 }, true).unwrap();
    s.conn().execute_batch("PRAGMA u.wal_checkpoint(TRUNCATE);").unwrap();
    let up = s.dir().join("user.db");
    let wal = s.dir().join("user.db-wal");
    let before = (fingerprint(&up), fingerprint(&wal));
    s.rescan_source(src).unwrap();
    std::fs::remove_file(fonts.join("roboto.ttf")).unwrap();
    s.rescan_source(src).unwrap();
    s.gc().unwrap();
    assert_eq!((fingerprint(&up), fingerprint(&wal)), before);
}

#[test]
fn unknown_or_pathless_source_is_not_found() {
    let (_d, _f, mut s, _src) = setup();
    assert_eq!(s.rescan_source(4242).unwrap_err().code(), "not_found");
    let g = s.add_source(SourceKind::GoogleFonts, "Google", None).unwrap();
    assert_eq!(s.rescan_source(g).unwrap_err().code(), "not_found");
}

#[test]
fn unreadable_root_yields_empty_scan_not_panic() {
    let d = tmp();
    let mut s = open_store(d.path());
    let src = s.add_source(SourceKind::Directory, "Gone", Some(Path::new("/khong/ton/tai"))).unwrap();
    let st = s.rescan_source(src).unwrap();
    assert_eq!(st.seen, 0);
}
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test rescan`
Expected: FAIL, lỗi biên dịch `cannot find function walk_fonts`.

- [ ] **Step 3: Viết implementation** — `src/rescan.rs`

`walk_fonts`: `std::fs::read_dir` đệ quy bằng ngăn xếp; `entry.file_type()` (không theo symlink: bỏ `is_symlink()`); `mtime_ns` = `modified().duration_since(UNIX_EPOCH).as_nanos() as i64` (lỗi → 0).

`rescan_source` — thuật toán (đây là chỗ test không tự quyết được, làm đúng thứ tự):

```text
src = self.source(id)?; root = src.path ?: NotFound
started = now_ms()
files = walk_fonts(root)
known = SELECT l.path, l.size, l.mtime_ns,
               COALESCE(p.status IN ('done','failed') AND p.version = ANALYZER_VERSION, 0) AS fresh
        FROM file_locations l JOIN files fi USING(file_id)
        LEFT JOIN pipeline_state p ON p.hash = fi.hash AND p.face_index = -1 AND p.stage = 'load'
        WHERE l.source_id = id                                   -> HashMap<path, (size, mtime, fresh)>
unchanged = files có (size, mtime) trùng và fresh = 1
  -> UPDATE file_locations SET last_seen_at = started, missing = 0 WHERE path = ?   (1 transaction)
changed = phần còn lại
hashed = changed.par_iter().map(|f| (f, blake3 của file đọc theo luồng (Hasher::update_reader)))
  lỗi đọc -> stats.unreadable.push((path, "io"))
với mỗi (f, h) đã băm: hash có pipeline_state load fresh  -> IngestItem{Known}       (relinked)
                       ngược lại                          -> cần phân tích
reports = fontatlas_core::analyze_paths(cần phân tích)     // song song, cô lập panic
  Ok(r)  -> IngestItem{hash: parse_hash(&r.hash_hex)?, Report(r)}   // hash của report là chuẩn
  Err(e) -> IngestItem{hash: h, Failed{code: e.code(), detail: e.to_string()}}
ghi theo lô INGEST_BATCH bằng self.ingest(lô, started)
UPDATE file_locations SET missing = 1 WHERE source_id = id AND missing = 0 AND last_seen_at < started
stats.missing = số dòng vừa đổi; stats.seen = files.len()
```
`started` dùng làm `seen_at` cho mọi dòng chạm tới trong lần quét. Đếm `analyzed` = số `Report`, `failed` = số `Failed`, `relinked` = số `Known`.

`remove_source`: một transaction: `DELETE FROM file_locations WHERE source_id=?`, `DELETE FROM u.sources WHERE source_id=?`; sau đó `gc()`. (Ghi chú: đây là thao tác của người dùng nên được ghi `u`.)

`gc`: một transaction: `DELETE FROM files WHERE NOT EXISTS (SELECT 1 FROM file_locations l WHERE l.file_id = files.file_id)` (faces và FTS dọn theo cascade/trigger), rồi `DELETE FROM analyses WHERE source='local' AND hash NOT IN (SELECT hash FROM files)`, `DELETE FROM pipeline_state WHERE hash NOT IN (SELECT hash FROM files)`. File chỉ còn vị trí `missing=1` **không** bị xoá (có thể quay lại). `gc` không đụng `u`.

`lib.rs`: `mod rescan; pub use rescan::{FONT_EXTENSIONS, INGEST_BATCH, RescanStats, walk_fonts};`.

- [ ] **Step 4: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test rescan`
Expected: `ok. 11 passed`.

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): diff-based rescan by mtime/size/hash, relink moves, gc

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Di chuyển từ DB cũ (`fontatlas.db`, bảng `fonts` 35 cột)

**Files:**
- Create: `src/legacy.rs`
- Modify: `src/lib.rs`, `tests/common/mod.rs`
- Test: `tests/legacy.rs`

**Interfaces:**
- Consumes: `Store`, `tag_in` (Task 4), `vi_fold`, `rescan_source` (Task 6, chỉ trong test)
- Produces:
  - `pub const LEGACY_DB_FILE: &str = "fontatlas.db"`
  - `#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)] #[serde(rename_all = "camelCase")] pub struct LegacyImport { pub fonts: usize, pub sources: usize, pub collections: usize, pub recovered_collections: usize, pub bad_tags: usize }`
  - `Store::import_legacy(&mut self, old_db: &Path) -> Result<LegacyImport, StoreError>`
  - `#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)] #[serde(rename_all = "camelCase")] pub struct LegacyResolve { pub resolved: usize, pub unresolved: usize, pub ambiguous_faces: usize }`
  - `Store::resolve_legacy(&mut self) -> Result<LegacyResolve, StoreError>`
  - `Store::legacy_face(&self, legacy_id: &str) -> Result<Option<FaceKey>, StoreError>`

Payload JSON lưu trong `u.legacy_ids.payload` (dạng cố định, chỉ chứa thứ cần để áp dữ liệu người dùng; giá trị thiếu cột → mặc định):
```json
{"name": "...", "family": "...", "subfamily": "...", "favorite": true, "collectionId": "uuid|null", "tags": ["..."], "dateAdded": "..."}
```

- [ ] **Step 1: Bổ sung helper DB cũ** — thêm vào `tests/common/mod.rs`

DDL chép nguyên văn từ `UX/src-tauri/src/db.rs:115-166` (bảng `fonts` 35 cột, `sources`) và struct `Collection` (`db.rs:52-58`):
```rust
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
```

- [ ] **Step 2: Viết test thất bại** — `tests/legacy.rs`

```rust
mod common;
use common::*;
use fontatlas_store::{FaceQuery, Store};
use unicode_normalization::UnicodeNormalization;

struct Env { _d: tempfile::TempDir, fonts: std::path::PathBuf, old: std::path::PathBuf, s: Store }

fn env(with_collections: bool) -> Env {
    let d = tmp();
    let fonts = d.path().join("Phông chữ"); // tên thư mục tiếng Việt
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(core_fixture("roboto.ttf"), fonts.join("roboto.ttf")).unwrap();
    std::fs::copy(core_fixture("playfair_display.ttf"), fonts.join("playfair.ttf")).unwrap();
    std::fs::copy(core_fixture("open_sans.ttf"), fonts.join("open.ttf")).unwrap();
    let p = |n: &str| fonts.join(n).to_string_lossy().to_string();
    // Đường dẫn của Playfair lưu ở dạng NFD như HFS+ cũ.
    let playfair_nfd: String = p("playfair.ttf").nfd().collect();
    let (pr, po, pm) = (p("roboto.ttf"), p("open.ttf"), p("mat.ttf"));
    let old = d.path().join("fontatlas.db");
    make_legacy_db(&old, with_collections, &fonts, &[
        LegacyFont { id: "A", path: &pr, family: "Roboto", subfamily: "Regular", tags: r#"["Cổ điển","display"]"#, collection: Some("col-1"), favorite: true, source: Some("src-1") },
        LegacyFont { id: "B", path: &pm, family: "Mất", subfamily: "Regular", tags: "[]", collection: None, favorite: true, source: Some("src-1") },
        LegacyFont { id: "C", path: &playfair_nfd, family: "Playfair Display", subfamily: "Regular", tags: "[]", collection: Some("col-ma"), favorite: false, source: None },
        LegacyFont { id: "D", path: &po, family: "Open Sans", subfamily: "Regular", tags: "not json", collection: None, favorite: false, source: Some("src-1") },
    ]);
    let s = open_store(&d.path().join("db"));
    Env { _d: d, fonts, old, s }
}

#[test]
fn import_maps_sources_collections_and_keeps_old_db_intact() {
    let mut e = env(true);
    let before = std::fs::read(&e.old).unwrap();
    let r = e.s.import_legacy(&e.old).unwrap();
    assert_eq!(r.fonts, 4);
    assert_eq!(r.sources, 1);
    assert_eq!(r.collections, 1);
    assert_eq!(r.recovered_collections, 1, "col-ma không tồn tại -> 'Đã khôi phục'");
    assert_eq!(r.bad_tags, 1);
    assert_eq!(std::fs::read(&e.old).unwrap(), before, "DB cũ mở chỉ đọc");
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.legacy_ids WHERE hash IS NULL"), 4);
    assert!(e.s.collections().unwrap().iter().any(|c| c.name.starts_with("Đã khôi phục")));
}

#[test]
fn import_is_idempotent() {
    let mut e = env(true);
    let a = e.s.import_legacy(&e.old).unwrap();
    let b = e.s.import_legacy(&e.old).unwrap();
    assert_eq!(a, b);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.legacy_ids"), 4);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.collections"), 2);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.sources"), 1);
}

#[test]
fn missing_collections_table_is_tolerated() {
    let mut e = env(false);
    let r = e.s.import_legacy(&e.old).unwrap();
    assert_eq!(r.collections, 0);
    assert_eq!(r.recovered_collections, 2, "col-1 và col-ma đều phải khôi phục");
}

#[test]
fn resolve_after_rescan_applies_user_data() {
    let mut e = env(true);
    e.s.import_legacy(&e.old).unwrap();
    let src = e.s.sources().unwrap()[0].id;
    e.s.rescan_source(src).unwrap();
    let r = e.s.resolve_legacy().unwrap();
    assert_eq!(r, fontatlas_store::LegacyResolve { resolved: 3, unresolved: 1, ambiguous_faces: 0 });
    let fav = e.s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap().items;
    assert_eq!(fav.iter().map(|f| f.family.as_str()).collect::<Vec<_>>(), vec!["Roboto"]);
    let roboto = e.s.legacy_face("A").unwrap().expect("A đã nối");
    let tags: Vec<String> = e.s.tags_of(&roboto).unwrap().into_iter().map(|t| t.name).collect();
    assert_eq!(tags, vec!["Cổ điển".to_string(), "display".to_string()]);
    let cols = e.s.collections().unwrap();
    assert_eq!(cols.iter().find(|c| c.name == "Bộ cũ").unwrap().item_count, 1);
    assert!(e.s.legacy_face("C").unwrap().is_some(), "đường dẫn NFD phải khớp file NFC");
    assert!(e.s.legacy_face("B").unwrap().is_none(), "file đã mất vẫn chờ");
    // chạy lại không nhân đôi
    assert_eq!(e.s.resolve_legacy().unwrap().resolved, 0);
    assert_eq!(count(&e.s, "SELECT count(*) FROM u.face_tags"), 2);
}

#[test]
fn lost_file_resolves_when_it_reappears() {
    let mut e = env(true);
    e.s.import_legacy(&e.old).unwrap();
    let src = e.s.sources().unwrap()[0].id;
    std::fs::copy(core_fixture("libre_baskerville.ttf"), e.fonts.join("mat.ttf")).unwrap();
    e.s.rescan_source(src).unwrap();
    e.s.resolve_legacy().unwrap();
    assert!(e.s.legacy_face("B").unwrap().is_some());
    let fav = e.s.query_faces(&FaceQuery { favorites_only: true, ..Default::default() }).unwrap();
    assert_eq!(fav.total, 2);
}

#[test]
fn garbage_or_foreign_db_is_rejected_without_partial_writes() {
    let mut e = env(true);
    let junk = e.fonts.join("junk.db");
    std::fs::write(&junk, b"SQLite format 3\0 nhưng hỏng").unwrap();
    assert_eq!(e.s.import_legacy(&junk).unwrap_err().code(), "legacy");
    let other = e.fonts.join("other.db");
    rusqlite::Connection::open(&other).unwrap().execute_batch("CREATE TABLE x(y);").unwrap();
    assert_eq!(e.s.import_legacy(&other).unwrap_err().code(), "legacy");
    for t in ["legacy_ids", "sources", "collections", "tags"] {
        assert_eq!(count(&e.s, &format!("SELECT count(*) FROM u.{t}")), 0, "{t} phải rỗng");
    }
}

#[test]
fn old_db_missing_newer_columns_still_imports() {
    let d = tmp();
    let old = d.path().join("fontatlas.db");
    {
        let c = rusqlite::Connection::open(&old).unwrap();
        // Bản rất cũ: chưa có is_favorite, family_name, source_id...
        c.execute_batch("CREATE TABLE fonts (id TEXT PRIMARY KEY, name TEXT NOT NULL, file_path TEXT NOT NULL UNIQUE,
            tags TEXT, collection_id TEXT, date_added TEXT NOT NULL, date_modified TEXT NOT NULL);
            INSERT INTO fonts VALUES ('Z','Cũ','/x/cu.ttf','[\"a\"]',NULL,'t','t');").unwrap();
    }
    let mut s = open_store(&d.path().join("db"));
    let r = s.import_legacy(&old).unwrap();
    assert_eq!((r.fonts, r.sources), (1, 0));
}
```

- [ ] **Step 3: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test legacy`
Expected: FAIL, lỗi biên dịch `no method named import_legacy`.

- [ ] **Step 4: Viết `import_legacy`** — `src/legacy.rs`

1. Mở DB cũ bằng `Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)`. Lỗi mở, hoặc `PRAGMA table_info(fonts)` rỗng, hoặc thiếu một trong `id`, `file_path` → `StoreError::Legacy(...)`. Mọi lỗi SQLite khi đọc DB cũ đổi thành `Legacy`.
2. Lấy tập cột của `fonts`, `sources` (nếu có), `collections` (nếu có) qua `PRAGMA table_info`. Câu `SELECT` chỉ gồm cột có thật; cột thiếu → giá trị mặc định (`is_favorite`=false, `tags`="[]", chuỗi khác `None`).
3. **Một transaction trên `u`** cho toàn bộ ghi (lỗi giữa chừng → rollback, không dòng dở):
   - `sources` cũ: `type` → `SourceKind::parse` (không nhận ra → `Directory`); không có `path` mà kind là `Directory` → bỏ. `INSERT INTO u.sources(legacy_id,kind,name,path,created_at) … ON CONFLICT DO NOTHING` (cả `legacy_id` lẫn `(kind,path)`), đếm `sources` bằng số dòng có `legacy_id` sau cùng.
   - `collections` cũ → `u.collections(legacy_id,…)` `ON CONFLICT(legacy_id) DO NOTHING`.
   - Mỗi `collection_id` của `fonts` không có trong bảng cũ (hoặc bảng không tồn tại) → tạo `u.collections(legacy_id = id cũ, name = "Đã khôi phục ({n})")` với `n` tăng dần theo thứ tự id; đếm `recovered_collections`.
   - Mỗi hàng `fonts` → `INSERT INTO u.legacy_ids(legacy_id, legacy_path, payload) … ON CONFLICT(legacy_id) DO NOTHING`. `tags` không parse được thành `Vec<String>` → payload `tags: []`, `bad_tags += 1`.
   - `u.settings('legacy_import')` = JSON `{"source": path, "importedAt": ms, ...LegacyImport}` (`INSERT OR REPLACE` — đây là bản ghi nhật ký, không phải dữ liệu người dùng).
4. Kết quả trả về tính từ dữ liệu DB cũ (không phụ thuộc lần chạy), nên chạy lại cho cùng `LegacyImport` (test `import_is_idempotent`).

Lưu ý: `rusqlite::Error` khi ghi `u` vẫn là `StoreError::Sqlite`.

- [ ] **Step 5: Viết `resolve_legacy` và `legacy_face`** — `src/legacy.rs`

```text
unresolved = SELECT legacy_id, legacy_path, payload FROM u.legacy_ids WHERE hash IS NULL
locs = SELECT l.path, fi.hash, fi.file_id FROM file_locations l JOIN files fi USING(file_id) WHERE l.missing = 0
map: nfc(path) -> (hash, file_id)                      // so khớp sau khi chuẩn hoá NFC hai phía
với mỗi dòng chưa nối mà nfc(legacy_path) có trong map:
   faces = SELECT face_index, family_name, subfamily FROM faces WHERE file_id = ?
   face_index = 0 nếu faces.len() <= 1
             = face có vi_fold(family)==vi_fold(payload.family) và vi_fold(subfamily)==vi_fold(payload.subfamily)
             = ngược lại 0, ambiguous_faces += 1
   trong MỘT transaction trên u:
      favorite  -> INSERT OR IGNORE u.favorites
      collectionId -> collection_id = SELECT collection_id FROM u.collections WHERE legacy_id = ? ; INSERT OR IGNORE u.collection_items
      tags      -> tag_in(..) ; INSERT OR IGNORE u.face_tags
      UPDATE u.legacy_ids SET hash, face_index, resolved_at WHERE legacy_id = ? AND hash IS NULL
resolved = số dòng vừa nối; unresolved = số dòng còn hash IS NULL
```
Không có lệnh `DELETE` hay `UPDATE` nào lên `favorites`/`collection_items`/`face_tags`. `legacy_face(id)`: `SELECT hash, face_index FROM u.legacy_ids WHERE legacy_id=? AND hash IS NOT NULL`.

`lib.rs`: `mod legacy; pub use legacy::{LEGACY_DB_FILE, LegacyImport, LegacyResolve};`.

- [ ] **Step 6: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test legacy`
Expected: `ok. 7 passed`.

- [ ] **Step 7: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): import legacy fonts DB into user.db and resolve by path after rescan

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Độ bền, clippy, báo cáo

**Files:**
- Test: `tests/robustness.rs`
- Create: `docs/team/plans/plan2-report.md` (trong worktree)

**Interfaces:**
- Consumes: mọi API ở trên
- Produces: bảo đảm "không panic trên DB cũ hỏng / file hỏng" (Global Constraints) và báo cáo cho Lead

- [ ] **Step 1: Viết test** — `tests/robustness.rs`

```rust
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
    let mut rng = Lcg(0x5702_E);
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
```

Thêm cuối `tests/robustness.rs` (đo cho báo cáo, chỉ chạy khi gọi rõ):
```rust
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
```

- [ ] **Step 2: Chạy test**

Run: `cd UX/src-tauri && cargo test -p fontatlas-store --release --test robustness`
Expected: `ok. 2 passed; 1 ignored`. Nếu có panic: tìm bằng `RUST_BACKTRACE=1`, sửa ở hàm gây panic (thường là `unwrap` trên giá trị đọc từ DB cũ), thêm test tái hiện, chạy lại.

- [ ] **Step 3: Chạy toàn bộ test và clippy**

```bash
cd UX/src-tauri && cargo test -p fontatlas-store && cargo test -p fontatlas-core \
  && cargo clippy -p fontatlas-store --all-targets -- -D warnings && cargo check -p app
```
Expected: mọi test qua; clippy không cảnh báo; `cargo check -p app` qua (chứng minh không xung đột `libsqlite3-sys` với app).

- [ ] **Step 4: Viết báo cáo ngắn** — `docs/team/plans/plan2-report.md`

Gồm: danh sách commit; output `cargo test -p fontatlas-store` (số test qua/bỏ qua); 4 số đo của `perf_50k_faces_under_50ms`; output của `perf_rescan_system_fonts` (lần 1 so với tiêu chí S1 "1.000 font < 3 s", lần 2 phải gần như chỉ còn `unchanged`); mọi chỗ khác plan (API rusqlite 0.31, hành vi FTS) và cách đã xử lý; câu hỏi mở cho Plan 5/6 (keyset pagination nếu cần, kết nối đọc riêng cho UI).

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store docs/team/plans/plan2-report.md
git commit -m "test(store): corrupted legacy DB and font robustness; plan 2 report

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Ghi chú cho các plan sau

- **Plan 3/4** thêm migration `LIBRARY_V2` (trục chuẩn hoá, cột embedding dùng thật) bằng cách nối vào `LIBRARY_MIGRATIONS` — không sửa `LIBRARY_V1`.
- **Plan 6** gọi theo thứ tự lúc khởi động: `Store::open(app_data_dir)` → nếu có `fontatlas.db` và `u.settings` chưa có `legacy_import` thì `import_legacy` → `rescan_source` cho mọi nguồn `directory` → `resolve_legacy`. `Store` không `Sync`: bọc `Mutex` hoặc mở thêm một `Store` chỉ đọc cho UI (WAL cho phép đọc song song).
- Bỏ Qdrant: plan này không đọc Qdrant (spec 3.4: bỏ dữ liệu Qdrant cũ).
