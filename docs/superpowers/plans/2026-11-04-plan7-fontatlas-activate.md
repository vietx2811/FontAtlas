# Plan 7 — `fontatlas-activate` (kích hoạt font) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Kích hoạt và tắt font của thư viện cho mọi app trên máy, không copy file. Chạy được theo face và theo collection, phát hiện xung đột tên trước khi kích hoạt, và đối soát với hệ điều hành mỗi lần app mở hoặc thoát. Bản này làm macOS; Windows chỉ có stub.

**Architecture:**
- **Store.** Crate `fontatlas-store` nhận migration user.db v2 cho bảng `activation_state`, cùng API có kiểu. Store giữ trạng thái mong muốn và trạng thái thật.
- **Crate `fontatlas-activate` (mới)** có hai lớp:
  1. **Engine.** Trait `ActivationEngine` chỉ nói chuyện với hệ điều hành, gồm 3 bản:
     - `MacCoreTextEngine`: objc2-core-text, đăng ký tại chỗ theo scope.
     - `WindowsGdiEngine`: stub, trả `Unsupported`.
     - `FakeEngine`: trong bộ nhớ, mô phỏng đúng ngữ nghĩa CoreText đã đo ở spike 04. Mọi test logic dùng bản này.
  2. **Service.** `ActivationService<E>` điều phối Store + engine:
     - preflight xung đột
     - activate/deactivate theo face, collection, tất cả
     - đổi scope
     - `reconcile` lúc khởi động, `shutdown` lúc thoát
- **Không phụ thuộc Tauri.** Plan 6a (lớp lệnh) gọi service trong `spawn_blocking` và nối `shutdown` vào `RunEvent::Exit`.

**Tech Stack:**
- Rust edition 2024, rustc ≥ 1.85, rusqlite =0.31.0 bundled (qua store), thiserror 2, serde 1, unicode-normalization 0.1.25.
- macOS: `objc2-core-text = "=0.3.2"`, `objc2-core-foundation = "=0.3.2"`. Cả hai đã có trong `Cargo.lock` qua Tauri, đúng bản spike đã chạy.
- dev: tempfile 3.27, serde_json 1.

**Spec:** `docs/superpowers/specs/2026-10-16-fontatlas-mvp-design.md`, mục 1.3 (cập nhật 2026-11-04: kích hoạt vào bản đầu, macOS trước, có ở onboarding và Cài đặt), 3.1 (`activation_state` trong `user.db`, không FK sang `library.db`), 10, 11.
**Bằng chứng kỹ thuật:** `docs/team/spikes/04_activation.md`. Mọi hành vi CoreText trong plan này lấy từ bảng 1.1 của spike, gọi tắt là "spike #n".
**Ghi nhớ của chủ app:** công cụ, không gác cổng. Xung đột chỉ cảnh báo và cho người dùng quyết; không chặn cứng.

**Nhánh:** `feat/activate`, tạo từ `dev/mvp` (HEAD `187eb67`). Task 2 thêm `crates/fontatlas-activate` vào `members`. Khi gộp, Lead chạy `cargo update -w`.

**Phối hợp với Plan 6a:** tên và kiểu dưới đây (mục "Hợp đồng cho Plan 6a") là đề xuất. Lead chốt với 02. Nếu đổi tên, chỉ đổi ở Task 2 và Task 8; các task khác dùng lại qua Interfaces.

## Global Constraints

- **Đơn vị kích hoạt là FILE (khoá `hash`), không phải face.** CoreText và GDI đều đăng ký theo file. Kích hoạt một face trong TTC sẽ kích hoạt mọi face của file đó; báo cáo trả `extra_faces` để UI nói rõ.
- **Scope:**
  - `ActivationScope::Session` (mặc định) = CoreText `kCTFontManagerScopeSession` (3).
  - `ActivationScope::Persistent` = `kCTFontManagerScopePersistent` (2).
  - **Không bao giờ dùng `Process` (1)** trong code sản phẩm, vì app khác không thấy (spike #1).
- **"Theo phiên app":** dòng `Session` được đăng ký lại ở `reconcile` khi app mở và gỡ ở `shutdown` khi app thoát. `state` (mong muốn) vẫn là `active` sau khi thoát. Dòng `Persistent` không bị `shutdown` đụng tới.
- **Lý do kích hoạt (Lead chốt 2026-11-04):**
  - Mỗi file có một **tập lý do** trong bảng `activation_reasons`: `manual` và/hoặc `collection:<id>`.
  - **Bất biến:** `state=Active` ⇔ tập lý do không rỗng.
  - Activate **thêm** lý do. Deactivate **gỡ đúng** lý do đó.
  - Chỉ gọi `engine.unregister` khi tập lý do **rỗng**.
  - Tắt thủ công một file mà collection còn giữ → trả `HeldByCollections`, không đụng gì, trừ khi `DeactivatePolicy::Force`.
  - Mục đích: tắt dự án A không làm mất font của dự án B.
- **Plan 7 là nơi DUY NHẤT nâng `user.db` lên v2.** Plan khác (6a, 6b) cần thêm cấu hình thì dùng bảng `settings` có sẵn, không thêm migration `user.db`.
- **Khi mở app:** `reconcile` chạy xong **trước** khi UI hiện trạng thái kích hoạt.
- **Setting:**
  - `SETTING_ACTIVATION_KEEP = "activation_keep_after_quit"`: `"1"` → scope mặc định `Persistent`; không có hoặc giá trị khác → `Session`.
  - `SETTING_ACTIVATION_ONBOARDED = "activation_onboarded"`: `"1"` sau khi onboarding đã hỏi.
- **Mã lỗi CoreText** (domain `com.apple.CoreText.CTFontManagerErrorDomain`):
  - `105` → `EngineError::AlreadyRegistered`, coi là thành công khi đăng ký.
  - `201` → `EngineError::NotRegistered`, coi là thành công khi gỡ.
  - Mã khác → `EngineError::Rejected { code, message }`.
- **Đường dẫn:**
  - Đường dẫn đưa cho hệ điều hành luôn là `std::fs::canonicalize(p)`.
  - Chuỗi đó lưu nguyên văn vào `activation_state.active_path`.
  - Khi gỡ, **chỉ dùng `active_path` + `scope` đã lưu**, kể cả khi file đã mất (spike #6, #8).
  - Không chuẩn hoá Unicode (NFC/NFD) chuỗi trước khi gọi OS; giữ đúng byte `canonicalize` trả về.
- **Trạng thái file trước khi đăng ký** (`probe_file`):
  - Không tồn tại → `Missing`.
  - macOS có cờ `SF_DATALESS = 0x4000_0000` trong `st_flags`, hoặc tồn tại placeholder `.<tên>.icloud` cùng thư mục → `NotDownloaded`.
  - **Không đọc nội dung file** để tránh kích hoạt tải iCloud ngầm.
- **Xung đột** = một PostScript name của file đang được hệ thống cung cấp từ một đường dẫn ≠ `active_path` của chính file này.
  - Phân loại chủ sở hữu theo `classify_owner` (bảng ở Task 2), là suy đoán theo đường dẫn; UI ghi "có thể là …".
  - `ConflictPolicy::Ask` (mặc định): không đăng ký, trả `NeedsConfirmation`, không ghi DB.
  - `ConflictPolicy::Proceed`: vẫn đăng ký (CoreText chấp nhận trùng, spike #4).
- **Phạm vi file được phép đụng:**
  - Không bao giờ ghi, copy hay xoá trong `~/Library/Fonts`, `/Library/Fonts`, `/System/Library/Fonts`.
  - Không bao giờ gỡ đăng ký một đường dẫn không có trong `activation_state`.
- **Test an toàn:**
  - Test gọi CoreText thật có `#[ignore = "live CoreText: chạy khi người dùng đồng ý"]` **và** chỉ chạy khi `FONTATLAS_LIVE_ACTIVATION=1`; thiếu biến thì `return` sớm, in lý do.
  - Chỉ dùng fixture OFL trong `crates/fontatlas-core/tests/fixtures/`, copy vào `tempfile::TempDir`.
  - Luôn gỡ qua guard RAII (`LiveGuard`) kể cả khi panic.
  - Nếu PostScript name của fixture đã có sẵn trên máy thì bỏ qua test, để không đụng font thật của người dùng.
- **Windows:** chỉ stub + tài liệu. Không thêm crate `windows`, không ghi registry.
- `ENGINE_VERSION: u32 = 1`, ghi vào mỗi dòng `activation_state`.
- Mã lỗi `ActivationError::code()`: `store`, `engine`, `unsupported`, `not_found`, `bad_hash`.
- **Khoá Store:** mọi hàm service nhận `store: &std::sync::Mutex<Store>` và khoá **theo từng file**, nhả khoá trước khi gọi engine và trước khi gọi `progress`. Không giữ khoá suốt lô. Tauri state của Plan 6a là `Mutex<Store>`.
- Mọi kiểu đi qua IPC: `#[serde(rename_all = "camelCase")]`. Enum có dữ liệu dùng `#[serde(tag = "kind", rename_all = "snake_case")]`.

## Review Focus

1. **Tên file tiếng Việt có dấu (NFC vs NFD).** "Phở Đậm.ttf" đăng ký, rồi gỡ khi file đã bị xoá, phải gỡ được. Test: `live_vietnamese_filename_roundtrip` (Task 8).
2. **Thư viện lớn, 1.000+ file kích hoạt cùng lúc.** Spike đo 10–70 ms mỗi file, nên một collection lớn mất hàng chục giây. Service phải báo tiến độ và không giữ khoá Store suốt quá trình. Test: `activate_reports_progress_per_file` và `activate_releases_store_lock_between_files` (Task 5).
3. **App crash rồi mở lại:** dòng session vẫn còn trong registry của macOS. `reconcile` gặp lỗi 105 phải coi là đã đăng ký, không báo lỗi. Test: `reconcile_treats_already_registered_as_ok` (Task 7).
4. **Font đang được RightFont/FontBase kích hoạt (trùng PS name):** phải ra `NeedsConfirmation` với owner `RightFont`, không đăng ký ngầm. Test: `preflight_flags_rightfont_duplicate` (Task 5).
5. **Ổ ngoài rút ra giữa phiên, rồi cắm lại:** đăng ký cũ phải được gỡ (không để font "ma"), mong muốn `active` được giữ, và tự kích hoạt lại khi file trở lại. Test: `reconcile_missing_then_back` (Task 7).

---

## File Structure

```
UX/src-tauri/crates/fontatlas-store/
  src/schema.rs            (sửa) USER_V2 + USER_MIGRATIONS
  src/activation.rs        (mới) ActivationRow, enum, API Store
  src/lib.rs               (sửa) re-export
  tests/activation.rs      (mới)
  tests/open.rs            (không sửa: so với hằng USER_VERSION)
UX/src-tauri/crates/fontatlas-activate/
  Cargo.toml
  src/lib.rs               re-export, platform_engine()
  src/types.rs             EngineError, ActivationError, Owner, Conflict, FileOutcome, FileResult, ActivationReport, Progress, ConflictPolicy
  src/engine.rs            trait ActivationEngine
  src/fake.rs              FakeEngine (pub, dùng cho test của crate và của Plan 6a)
  src/paths.rs             canonical_path, probe_file, FileProbe, classify_owner
  src/mac.rs               MacCoreTextEngine (cfg macos)
  src/windows.rs           WindowsGdiEngine stub (biên dịch mọi nền tảng)
  src/service.rs           ActivationService: preflight, activate*, deactivate*, set_scope
  src/reconcile.rs         reconcile, shutdown
  tests/common/mod.rs      store tạm + nạp fixture vào store
  tests/service.rs  tests/reconcile.rs  tests/mac_live.rs  tests/contract.rs
  tests/fixtures/activation_report.json   (sinh bởi test, hợp đồng cho TS)
.github/workflows/activate.yml (mới)
docs/team/plans/plan7-report.md (Task 8)
```

---

### Task 1: Store — migration v2 `activation_state` + API

**Files:**
- Modify: `UX/src-tauri/crates/fontatlas-store/src/schema.rs` (thêm `USER_V2`, `USER_MIGRATIONS = &[USER_V1, USER_V2]`)
- Create: `UX/src-tauri/crates/fontatlas-store/src/activation.rs`
- Modify: `UX/src-tauri/crates/fontatlas-store/src/lib.rs`
- Test: `UX/src-tauri/crates/fontatlas-store/tests/activation.rs`

**Interfaces:**
- Produces (re-export từ `fontatlas_store`):
  - `enum ActivationScope { Session, Persistent }`, với `as_str()` / `parse()` ↔ `"session"` / `"persistent"`.
  - `enum DesiredState { Active, Inactive }` ↔ `"active"` / `"inactive"`.
  - `enum ActivationStatus { Registered, Unregistered, Conflict, MissingFile, NotDownloaded, Error, Unknown }` ↔ snake_case.
  - Tất cả derive `Debug, Clone, Copy, PartialEq, Eq, Serialize` với `rename_all = "snake_case"`.
  - `enum ActivationReason { Manual, Collection(i64) }`:
    - DB: `"manual"` / `"collection:<id>"`, qua `as_db()` / `parse_db()`.
    - Serde: `#[serde(tag = "kind", rename_all = "snake_case")]`, ra `{"kind":"manual"}` / `{"kind":"collection","collectionId":3}`.
    - `Ord`: `Manual` trước, collection theo id.
  - `struct ActivationRow { reasons: Vec<ActivationReason>, hash: [u8; 32], state: DesiredState, scope: ActivationScope, method: String, active_path: Option<String>, status: ActivationStatus, conflict_ps: Option<String>, last_error: Option<String>, activated_at: Option<i64>, verified_at: Option<i64>, updated_at: i64, engine_version: u32 }`
  - `Store::activation_rows(&self) -> Result<Vec<ActivationRow>, StoreError>`: thứ tự theo `hash`.
  - `Store::activation_row(&self, hash: &[u8; 32]) -> Result<Option<ActivationRow>, StoreError>`
  - `Store::put_activation(&mut self, row: &ActivationRow) -> Result<(), StoreError>`: upsert bằng **`INSERT … ON CONFLICT(hash) DO UPDATE SET …`**. **Cấm `INSERT OR REPLACE`**: REPLACE xoá dòng cũ, và FK `ON DELETE CASCADE` sẽ xoá sạch lý do. `updated_at` lấy từ `now_ms()`, bỏ qua giá trị truyền vào.
  - `Store::delete_activation(&mut self, hash: &[u8; 32]) -> Result<bool, StoreError>`: xoá luôn lý do (FK cascade).
  - `activation_rows` / `activation_row` điền `reasons` (đã sắp xếp). `put_activation` **không** ghi `reasons`; lý do chỉ đổi qua các hàm dưới đây.
  - `Store::add_activation_reason(&mut self, hash: &[u8; 32], reason: ActivationReason) -> Result<bool, StoreError>`: `true` nếu lý do mới. Dòng `activation_state` phải có trước; thiếu thì `NotFound`.
  - `Store::remove_activation_reason(&mut self, hash: &[u8; 32], reason: ActivationReason) -> Result<Vec<ActivationReason>, StoreError>`: trả các lý do còn lại.
  - `Store::clear_activation_reasons(&mut self, hash: &[u8; 32]) -> Result<usize, StoreError>`
  - `Store::hashes_with_reason(&self, reason: ActivationReason) -> Result<Vec<[u8; 32]>, StoreError>`
  - `Store::with_tx<R>(&mut self, f: impl FnOnce(&rusqlite::Transaction) -> Result<R, StoreError>) -> Result<R, StoreError>`: các hàm `*_tx(tx, …)` tương ứng (`put_activation_tx`, `add_activation_reason_tx`, `remove_activation_reason_tx`) để service ghi dòng + lý do trong **một** transaction.
  - `Store::prune_collection_reasons(&mut self) -> Result<Vec<([u8; 32], Vec<ActivationReason>)>, StoreError>`: xoá `collection:<id>` khi collection không còn, hoặc file không còn face nào trong collection đó. Trả `Vec<(hash, Vec<ActivationReason> /*lý do đã xoá*/)>`.
  - `struct FileTarget { hash: [u8; 32], locations: Vec<FileLocation>, faces: Vec<FaceName> }`
  - `struct FileLocation { path: PathBuf, missing: bool }`: `missing = 0` đứng trước, rồi `last_seen_at` giảm dần.
  - `struct FaceName { face_index: u32, postscript_name: Option<String> }`: theo `face_index`.
  - `Store::file_target(&self, hash: &[u8; 32]) -> Result<Option<FileTarget>, StoreError>`: `None` nếu `files` không có hash.
  - `Store::collection_faces(&self, collection_id: i64) -> Result<Vec<FaceKey>, StoreError>`: `NotFound` nếu collection không tồn tại.

- [ ] **Step 1: Viết test thất bại** trong `tests/activation.rs`:
  - `user_v2_migrates_v1_rows`:
    - Tạo `user.db` v1 bằng hàm `#[cfg(test)]`/`#[doc(hidden)] pub fn create_user_v1_for_test(path: &Path)` trong `schema.rs` (chạy `USER_V1` + `PRAGMA user_version=1`, không chép DDL vào test), chèn 1 dòng `activation_state(hash=[7;32], state='active', scope='persistent', method='coretext', active_path='/x.ttf', updated_at=1)`.
    - `Store::open` thành công.
    - `PRAGMA u.user_version == 2`.
    - `activation_row(&[7;32])` trả `state=Active, scope=Persistent, active_path=Some("/x.ttf"), status=Unknown, engine_version=1`.
    - `OpenReport.user_backup.is_some()`.
    - `reasons == [Manual]` (dòng v1 `active` được gán lý do `manual`).
  - `reasons_add_remove_and_check`:
    - add `Manual` → true; add `Collection(3)` → true; add `Collection(3)` lần nữa → false.
    - remove `Collection(3)` → `[Manual]`.
    - `conn().execute("INSERT INTO u.activation_reasons VALUES (?1,'collection:x',1)")` → `Err`.
    - add cho hash chưa có dòng → `NotFound`.
  - `delete_activation_cascades_reasons`.
  - `prune_collection_reasons_drops_deleted_and_removed_items`:
    - Hash H có `Collection(a)` (a đã bị `delete_collection`) và `Collection(b)` (face của H đã bị `remove_from_collection(b)`) → prune trả `[H]`, lý do còn `[]`.
    - Hash K có `Collection(c)` còn hợp lệ → giữ nguyên.
  - `put_activation_roundtrip_and_upsert`: put hai lần cùng hash với status khác nhau → `activation_rows().len()==1`, status là bản sau, `updated_at` > 0. Thêm: add `Manual` trước lần put thứ hai → sau put, `reasons` vẫn là `[Manual]`.
  - `activation_check_constraints`: `conn().execute("INSERT INTO u.activation_state(hash,state,scope,method,status,updated_at,engine_version) VALUES (zeroblob(32),'on','session','x','unknown',1,1)")` phải `Err`.
  - `file_target_lists_locations_and_ps_names`:
    - Ingest `libre_baskerville.ttf` qua helper có sẵn trong `tests/common`.
    - `file_target(hash)` có 1 location `missing=false`, faces `[FaceName{face_index:0, postscript_name:Some("LibreBaskerville-Regular")}]`.
  - `file_target_unknown_hash_is_none`.
  - `collection_faces_returns_items_and_not_found`.

- [ ] **Step 2: Chạy, kỳ vọng FAIL**
  Run: `cargo test -p fontatlas-store --test activation`
  Expected: lỗi biên dịch, chưa có `ActivationRow`.

- [ ] **Step 3: Viết `USER_V2`** trong `schema.rs`. Đổi tên bảng để giữ dữ liệu cũ, vì SQLite không sửa được `CHECK`:

```sql
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
```

- [ ] **Step 4: Viết `activation.rs`** theo Interfaces.
  - Mọi câu SQL tham chiếu `u.activation_state`, `u.collection_items` và `main.file_locations` / `main.faces`, theo cùng cách `user.rs` làm.
  - `file_target` chạy hai truy vấn: một lấy locations theo `file_id`, một lấy faces.

- [ ] **Step 5: Chạy, kỳ vọng PASS**
  Run: `cargo test -p fontatlas-store`
  Expected: mọi test PASS, kể cả `tests/open.rs` (so với hằng `USER_VERSION`, nay bằng 2).

- [ ] **Step 6: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-store
git commit -m "feat(store): user.db v2 activation_state with status + typed API"
```

---

### Task 2: Crate `fontatlas-activate` — kiểu, trait, `paths`, `FakeEngine`

**Files:**
- Create: `crates/fontatlas-activate/{Cargo.toml, src/lib.rs, src/types.rs, src/engine.rs, src/paths.rs, src/fake.rs}`
- Modify: `UX/src-tauri/Cargo.toml` (`members` thêm `"crates/fontatlas-activate"`)
- Test: unit test trong `paths.rs` và `fake.rs`

**Interfaces:**
- Consumes: `fontatlas_store::{ActivationScope, Store, FaceKey, ...}` (Task 1).
- Produces:

```rust
pub use fontatlas_store::ActivationScope as Scope;

pub trait ActivationEngine: Send + Sync {
    fn method(&self) -> &'static str;                                   // "coretext" | "gdi" | "fake"
    fn register(&self, path: &Path, scope: Scope) -> Result<(), EngineError>;
    fn unregister(&self, path: &Path, scope: Scope) -> Result<(), EngineError>;
    /// Mọi PostScript name hệ thống đang cung cấp (một lần mỗi lô).
    fn available_postscript_names(&self) -> Result<HashSet<String>, EngineError>;
    /// Các file đang cung cấp PS name này (có thể >1 khi trùng).
    fn providers(&self, postscript_name: &str) -> Result<Vec<PathBuf>, EngineError>;
    /// PS name đọc từ file, không đăng ký. Dự phòng khi library.db thiếu tên.
    fn postscript_names_in_file(&self, path: &Path) -> Result<Vec<String>, EngineError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError { AlreadyRegistered, NotRegistered, Unsupported, Rejected { code: i64, message: String } }

pub enum FileProbe { Ready, Missing, NotDownloaded }
pub fn canonical_path(p: &Path) -> std::io::Result<PathBuf>;            // = fs::canonicalize
pub fn probe_file(p: &Path) -> FileProbe;
pub enum Owner { System, UserInstalled, AdobeFonts, RightFont, FontBase, FontAtlas, Other }   // serde snake_case
pub fn classify_owner(path: &Path, home: &Path, ours: &HashSet<PathBuf>) -> Owner;

pub struct FakeEngine { /* Mutex<FakeState> */ }
impl FakeEngine {
    pub fn new() -> Self;
    pub fn panic_on_register(&self, path: &Path);                       // giả crash
    pub fn block_register_until(&self, b: Arc<std::sync::Barrier>);    // giả đăng ký chậm
    pub fn with_system_font(self, ps: &str, path: &Path) -> Self;      // font "có sẵn" không gỡ được
    pub fn registrations(&self) -> Vec<(PathBuf, Scope)>;
    pub fn set_names_for(&self, path: &Path, names: &[&str]);          // PS name mà file "chứa"
    pub fn fail_next_register(&self, err: EngineError);
    pub fn simulate_logout(&self);                                      // xoá mọi đăng ký Session
}
```

  - `classify_owner` theo thứ tự (khớp đầu tiên):
    1. `ours` chứa path → `FontAtlas`
    2. bắt đầu bằng `/System/Library/` hoặc `/Library/Fonts/` → `System`
    3. bắt đầu bằng `{home}/Library/Fonts/` → `UserInstalled`
    4. chứa `/Adobe/CoreSync/plugins/livetype/` → `AdobeFonts`
    5. có thành phần đuôi `.rightfontlibrary` → `RightFont`
    6. có thành phần tên đúng `FontBase` (không phân biệt hoa thường) → `FontBase`
    7. còn lại → `Other`
  - `FakeEngine` phải mô phỏng đúng spike: register trùng (path, scope) → `AlreadyRegistered`; unregister không có → `NotRegistered`; hai path khác nhau cùng PS name đều đăng ký được; `providers` trả mọi path đang đăng ký có PS name đó cộng font hệ thống.

- [ ] **Step 1: Viết test thất bại**
  - `paths::tests::classify_owner_table`: một case cho mỗi dòng bảng, gồm `"/Users/u/Library/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/A/x.ttf"` → `RightFont` và `"/Users/u/Library/Application Support/Adobe/CoreSync/plugins/livetype/.r/.123.otf"` → `AdobeFonts`.
  - `paths::tests::probe_missing_and_ready`: file trong tempdir → `Ready`; đường dẫn không tồn tại → `Missing`.
  - `paths::tests::probe_icloud_placeholder` (chạy mọi nền tảng, vì placeholder chỉ là tên file): tạo `.x.ttf.icloud` trong tempdir, probe `x.ttf` → `NotDownloaded`.
  - `paths::tests::canonical_resolves_symlink` **`#[cfg(unix)]`**: symlink → canonical bằng target canonical.
  - Ghi chú: trên Windows `canonicalize` trả dạng `\\?\C:\…`. Chuỗi này được lưu nguyên văn; Plan 7b quyết định có bỏ tiền tố trước khi gọi GDI hay không.
  - `fake::tests::fake_mirrors_coretext`: đăng ký lần 2 → `AlreadyRegistered`; gỡ chưa đăng ký → `NotRegistered`; hai path cùng tên → `providers` trả 2 path; `simulate_logout` chỉ xoá Session.

- [ ] **Step 2: Chạy, kỳ vọng FAIL**
  Run: `cargo test -p fontatlas-activate`
  Expected: lỗi biên dịch.

- [ ] **Step 3: Implement** theo Interfaces.
  - `probe_file`: dùng `std::fs::symlink_metadata` + `std::os::macos::fs::MetadataExt::st_flags()` (cfg macos; `darwin` cần rustc mới hơn MSRV 1.85) để thử cờ dataless.
  - `Cargo.toml`: `fontatlas-store` (path), `serde`, `thiserror`; `[target.'cfg(target_os = "macos")'.dependencies]` để trống ở task này. Dev-deps: `tempfile`, `serde_json`, `fontatlas-core` (path).

- [ ] **Step 4: Chạy, kỳ vọng PASS**
  Run: `cargo test -p fontatlas-activate && cargo clippy -p fontatlas-activate --all-targets -- -D warnings`
  Expected: PASS, không cảnh báo.

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/Cargo.toml UX/src-tauri/crates/fontatlas-activate
git commit -m "feat(activate): engine trait, path probing, owner heuristics, FakeEngine"
```

---

### Task 3: `MacCoreTextEngine`

**Files:**
- Create: `crates/fontatlas-activate/src/mac.rs`
- Modify: `crates/fontatlas-activate/Cargo.toml`, `src/lib.rs`
- Test: `crates/fontatlas-activate/tests/mac_live.rs` (phần chỉ đọc chạy mặc định; phần đăng ký `#[ignore]`)

**Interfaces:**
- Consumes: `ActivationEngine`, `EngineError`, `Scope` (Task 2).
- Produces: `#[cfg(target_os = "macos")] pub struct MacCoreTextEngine;` + `impl ActivationEngine` (`method() = "coretext"`).
- Gọi API:
  - `register` → `CTFontManagerRegisterFontsForURL(url, CTFontManagerScope(scope_raw), &mut err)`; `unregister` → `CTFontManagerUnregisterFontsForURL`. `scope_raw`: Session=3, Persistent=2.
  - `available_postscript_names` → `CTFontManagerCopyAvailablePostScriptNames`.
  - `providers(ps)`: tạo descriptor từ dict `{kCTFontNameAttribute: ps}`, rồi `CTFontDescriptorCreateMatchingFontDescriptors(desc, mandatory = set{kCTFontNameAttribute})`. Với mỗi kết quả, lấy `kCTFontURLAttribute` → path. Lọc lại các kết quả có PS name đúng bằng `ps`.
  - `providers` trả path đã `canonical_path` (bỏ qua path không canonicalize được thì giữ nguyên).
  - `postscript_names_in_file` → `CTFontManagerCreateFontDescriptorsFromURL` + `kCTFontNameAttribute`, giống spike.
  - URL tạo bằng `CFURL::from_file_path(path)` **trên đúng `path` được truyền vào**, không canonicalize lại. Việc canonicalize là của service.
  - Ánh xạ chỉ khi `CFError.domain() == "com.apple.CoreText.CTFontManagerErrorDomain"`: 105 → `AlreadyRegistered`, 201 → `NotRegistered`, khác (hoặc domain khác) → `Rejected { code, message: "<domain>: <description>" }`.

- [ ] **Step 1: Viết test**
  - **Chạy mặc định, chỉ đọc, cfg macos:**
    - `mac_reads_ps_names_from_fixture`: `postscript_names_in_file(fixture libre_baskerville.ttf) == ["LibreBaskerville-Regular"]`.
    - `mac_lists_system_names`: `available_postscript_names()` chứa `"Helvetica"`.
    - `mac_providers_of_system_font`: `providers("Helvetica")` không rỗng và mỗi path bắt đầu bằng `/System/Library/`.
  - **`#[ignore = "live CoreText: chạy khi người dùng đồng ý"]` + cổng env:**
    - `live_session_register_is_visible_and_idempotent`:
      - Copy fixture vào TempDir, canonicalize.
      - In hướng dẫn gỡ thủ công trước khi chạy: Font Book → Remove, kèm đường dẫn tạm và scope in ra màn hình.
      - Bỏ qua nếu `available_postscript_names()` đã chứa `LibreBaskerville-Regular`. Áp dụng cho **mọi** test sống.
      - `register(Session)` OK; lần 2 → `Err(AlreadyRegistered)`.
      - `available_postscript_names()` chứa tên.
      - Gỡ OK; gỡ lần 2 → `Err(NotRegistered)`.
      - Tên biến mất.
    - `live_wrong_scope_unregister_is_not_registered`: đăng ký Persistent, gỡ Session → `NotRegistered`, rồi gỡ Persistent OK.
    - `live_same_url_other_scope`: đăng ký Session, rồi cùng URL ở Persistent. Ghi lại kết quả (Ok hay 105) vào báo cáo; Task 6 `set_scope` chọn thứ tự theo kết quả này (xem Task 6).
    - `live_provider_paths_are_canonical`: đăng ký bản copy trong `/tmp/...` (symlink tới `/private/tmp`), `providers(ps)` sau khi canonicalize bằng đúng path đã đăng ký.
  - `struct LiveGuard { engine, path, scope }` có `Drop` gọi `unregister` và bỏ qua lỗi. Tạo guard **ngay sau** `register` thành công.

- [ ] **Step 2: Chạy, kỳ vọng FAIL**
  Run: `cargo test -p fontatlas-activate --test mac_live`
  Expected: lỗi biên dịch.

- [ ] **Step 3: Implement `mac.rs`**
  - Cargo: `[target.'cfg(target_os = "macos")'.dependencies] objc2-core-text = { version = "=0.3.2", features = ["CTFontManager","CTFontManagerErrors","CTFontDescriptor","CTFont"] }`, `objc2-core-foundation = { version = "=0.3.2", features = ["CFURL","CFArray","CFString","CFError","CFDictionary","CFSet"] }`.
  - Mọi `unsafe` gói trong hàm private nhỏ có comment `// SAFETY:`.

- [ ] **Step 4: Chạy, kỳ vọng PASS**
  Run: `cargo test -p fontatlas-activate --test mac_live`
  Expected: 3 test PASS, 2 ignored.

- [ ] **Step 5 (cần người dùng đồng ý):** hỏi người dùng trong chat. Nói rõ:
  - font: Libre Baskerville Regular, bản copy trong thư mục tạm
  - scope: session rồi persistent
  - cách gỡ: tự động qua guard; thủ công bằng Font Book → Remove

  Chỉ khi người dùng đồng ý mới chạy:
  Run: `FONTATLAS_LIVE_ACTIVATION=1 cargo test -p fontatlas-activate --test mac_live -- --ignored --test-threads=1`
  Expected: 2 PASS. Sau đó `cargo test ... mac_lists_system_names` vẫn PASS, và tên fixture không còn trong `available_postscript_names()`.

- [ ] **Step 6: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-activate
git commit -m "feat(activate): CoreText engine (session/persistent, idempotent 105/201)"
```

---

### Task 4: `WindowsGdiEngine` stub + CI biên dịch Windows

**Files:**
- Create: `crates/fontatlas-activate/src/windows.rs`, `.github/workflows/activate.yml`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct WindowsGdiEngine;`, biên dịch trên **mọi** nền tảng, không `cfg`. Mọi method trả `Err(EngineError::Unsupported)`; `method() = "gdi"`.
  - `pub fn platform_engine() -> Box<dyn ActivationEngine>` trong `lib.rs`: macOS → `MacCoreTextEngine`, nền tảng khác → `WindowsGdiEngine`.
  - `impl<E: ActivationEngine + ?Sized> ActivationEngine for Box<E>`, để `ActivationService<Box<dyn ActivationEngine>>` dùng được.
- Doc comment đầu file ghi phương án đã nghiên cứu (spike 04 §3), để Plan 7b làm theo:
  - **Session:** `AddFontResourceExW(path, 0, None)` (không `FR_PRIVATE`), rồi `SendMessageTimeoutW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0, SMTO_ABORTIFHUNG, 2000)`. Gỡ bằng `RemoveFontResourceExW` + broadcast.
  - **Persistent:** copy file vào `%LOCALAPPDATA%\Microsoft\Windows\Fonts\`, ghi REG_SZ `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts` (tên `"<Family> <Style> (TrueType)"`, dữ liệu là đường dẫn đầy đủ), rồi `AddFontResourceExW` + broadcast. Cần Win10 1809+.
  - **Feature crate `windows`:** `Win32_Graphics_Gdi`, `Win32_UI_WindowsAndMessaging`, `Win32_System_Registry`, `Win32_UI_Shell`, `Win32_System_Com`, `Win32_Foundation`.
  - **Câu hỏi mở:** HKCU trỏ tại chỗ không copy; file bị khoá khi đang nạp; GPO chặn.

- [ ] **Step 1: Viết test thất bại** `windows::tests::stub_is_unsupported`: mọi method trả `Unsupported`, `method()=="gdi"`. Thêm `lib::tests::platform_engine_method`: macOS → `"coretext"`, nền tảng khác → `"gdi"`.
- [ ] **Step 2: Chạy, kỳ vọng FAIL.** Run: `cargo test -p fontatlas-activate windows`
- [ ] **Step 3: Implement stub + `activate.yml`**
  - Job `mac`: `macos-14`, `cargo test -p fontatlas-store -p fontatlas-activate` + `cargo clippy -p fontatlas-activate --all-targets -- -D warnings`.
  - Job `windows-compile`: `windows-latest`, `cargo test --locked -p fontatlas-activate`. Job mac cũng dùng `--locked`. Đây là test biên dịch Windows; test logic chạy bằng FakeEngine nên vẫn PASS.
  - `working-directory: UX/src-tauri`.
  - Không job nào đặt `FONTATLAS_LIVE_ACTIVATION`.
- [ ] **Step 4: Chạy, kỳ vọng PASS.** Run: `cargo test -p fontatlas-activate`
- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-activate .github/workflows/activate.yml
git commit -m "feat(activate): Windows GDI stub + CI compile job; document AddFontResourceExW plan"
```

---

### Task 5: `ActivationService` — preflight + activate theo face/collection

**Files:**
- Create: `crates/fontatlas-activate/src/service.rs`, `tests/common/mod.rs`, `tests/service.rs`
- Modify: `src/types.rs`, `src/lib.rs`

**Interfaces:**
- Consumes: Store API (Task 1), `ActivationEngine` / `FakeEngine` / `paths` (Task 2).
- Produces:

```rust
pub enum ConflictPolicy { Ask, Proceed }
pub struct Conflict { pub postscript_name: String, pub existing_path: String, pub owner: Owner }
pub enum FileOutcome {                       // serde tag = "kind"
    Activated, AlreadyActive, Deactivated, AlreadyInactive, Busy,
    NeedsConfirmation { conflicts: Vec<Conflict> },
    MissingFile, NotDownloaded, Failed { code: String, message: String },
}
pub struct FileResult { pub hash_hex: String, pub path: Option<String>, pub requested_faces: Vec<u32>, pub extra_faces: Vec<u32>, pub outcome: FileOutcome }
pub struct ActivationReport { pub results: Vec<FileResult>, pub changed: usize, pub needs_confirmation: usize, pub failed: usize }
pub struct Progress { pub done: usize, pub total: usize, pub hash_hex: String }

pub struct ActivationService<E: ActivationEngine> { engine: E }
impl<E: ActivationEngine> ActivationService<E> {
    pub fn new(engine: E) -> Self;
    pub fn engine(&self) -> &E;
    pub fn default_scope(store: &Mutex<Store>) -> Result<Scope, ActivationError>;   // đọc SETTING_ACTIVATION_KEEP
    pub fn preflight(&self, store: &Mutex<Store>, keys: &[FaceKey]) -> Result<ActivationReport, ActivationError>; // không đăng ký, không ghi
    pub fn activate_faces(&self, store: &Mutex<Store>, keys: &[FaceKey], scope: Option<Scope>, policy: ConflictPolicy, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    pub fn activate_collection(&self, store: &Mutex<Store>, collection_id: i64, scope: Option<Scope>, policy: ConflictPolicy, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    // nội bộ: fn activate_files(&self, store, keys, reason: ActivationReason, scope, policy, progress)
}
```

- **Thuật toán `activate_faces`** (lặp theo file, giữ thứ tự hash đầu tiên xuất hiện):
  0. **Khoá thao tác theo hash:** service giữ `in_flight: Mutex<HashSet<[u8;32]>>`. Hash đang bận → `FileOutcome::Busy` (biến thể mới), không chờ. Gỡ khỏi tập bằng guard RAII. CoreText luôn gọi **ngoài** khoá `Store`.
  1. Gom `keys` theo `hash`. Với mỗi hash: `file_target` → `None` thì `Failed{code:"not_found"}`.
  2. Chọn location đầu tiên có `probe_file == Ready`. Nếu không có: `NotDownloaded` (khi có ít nhất một location trả `NotDownloaded`) hoặc `MissingFile`.
  3. `path = canonical_path(loc)`. Đọc dòng cũ: nếu `state=Active`, `status=Registered`, `active_path==path` và `scope` khớp → `AlreadyActive` (vẫn thêm lý do, bước 8).
  4. **Preflight trước, gỡ sau.** Xung đột: `names` = PS name từ `FileTarget.faces`; tên nào `None` thì lấy bằng `postscript_names_in_file`. `avail = available_postscript_names()` gọi **một lần mỗi lô**. Với mỗi `name ∈ avail`: `providers(name)`, bỏ các path == `path` **và** == `active_path` cũ của chính file này → mỗi path còn lại là một `Conflict`, owner = `classify_owner(p, home, ours)`; `ours` là tập `active_path` đang Registered.
     - **Trùng trong cùng lô:** hai file của lô có chung PS name → file sau nhận `Conflict{owner: FontAtlas, existing_path: <path file trước>}`.
  5. Có xung đột và `Ask` → `NeedsConfirmation`, không đăng ký, không ghi DB.
  6. Dòng cũ đang `Registered` với path hoặc scope khác → `unregister(old_path, old_scope)` (bỏ qua `NotRegistered`).
  7. **Ghi ý định trước** (một transaction): upsert `state=Active, status=Unknown, active_path=path, scope, method, engine_version` + lý do (bước 8). Rồi `register(path, scope)`: `Ok` hoặc `AlreadyRegistered` → transaction thứ hai đặt `status=Registered, conflict_ps=tên đầu tiên bị trùng (nếu Proceed), activated_at=now, verified_at=now` → `Activated`. Lỗi khác → một transaction: gỡ lý do vừa thêm (nếu là lý do mới); nếu tập lý do rỗng thì `state=Inactive`; rồi `status=Error, last_error="engine.<code>"` → `Failed`.
     - Crash giữa hai transaction để lại dòng `status=Unknown`. `reconcile` xử lý dòng đó (Task 7), nên không có đăng ký Persistent mồ côi.
  8. Lý do: `activate_faces` → `Manual`; `activate_collection(id)` → `Collection(id)`. Với `AlreadyActive` thì thêm lý do trong một transaction riêng. `NeedsConfirmation`, `Failed`, `MissingFile`, `NotDownloaded`, `Busy` **không** để lại lý do mới.
  9. `extra_faces` = face của file không nằm trong `keys`. Gọi `progress` sau mỗi file.
- `scope: None` → `default_scope(store)`.
- `home` lấy từ `std::env::var_os("HOME")`.
- `activate_faces` = `activate_files(keys, Manual, …)`; `activate_collection(id)` = `collection_faces(id)` rồi `activate_files(keys, Collection(id), …)`. **Không** gọi qua `activate_faces`, vì như vậy sẽ gắn nhầm `Manual`.

- [ ] **Step 1: Viết test thất bại** trong `tests/service.rs`. Dùng `FakeEngine` + Store tạm có ingest fixture; helper ở `tests/common`: `store_with(names: &[&str]) -> (TempDir, Mutex<Store>, PathBuf /*thư mục font*/, i64 /*source_id*/)`. Helper copy fixture từ `crates/fontatlas-core/tests/fixtures/` vào TempDir, gọi `add_source(SourceKind::Directory, "t", Some(dir))` rồi `rescan_source(id)`. `rescan(store, source_id)` gọi lại `rescan_source`.
  - `activate_face_registers_canonical_path_session_by_default`: `registrations() == [(canonical, Session)]`; dòng DB `status=Registered`, `active_path` = chuỗi canonical.
  - `activate_releases_store_lock_between_files`: trong callback `progress`, `store.try_lock().is_ok()` với cả 2 file.
  - `activate_uses_persistent_when_keep_setting_on`: `set_setting(SETTING_ACTIVATION_KEEP,"1")`.
  - `activate_twice_is_already_active`.
  - `activate_adds_manual_reason`: `reasons == [Manual]`.
  - `activate_collection_adds_collection_reason`: `reasons == [Collection(id)]`.
  - `already_active_file_gains_new_reason`: thủ công rồi qua collection → `AlreadyActive`, `reasons == [Manual, Collection(id)]`, register đúng 1 lần.
  - `needs_confirmation_adds_no_reason`.
  - `crash_after_intent_leaves_unknown_row`: `FakeEngine::panic_on_register(path)` + `catch_unwind` → dòng `status=Unknown`, `state=Active`, lý do `[Manual]`. Task 7 test `reconcile_resolves_unknown_rows` dùng lại tình huống này.
  - `register_error_rolls_back_new_reason`: `fail_next_register` trên file chưa từng bật → `reasons` rỗng, `state=Inactive`, `status=Error`.
  - `preflight_ignores_own_old_path`: kích hoạt, đổi tên file + rescan → `activate_faces(Ask)` không có conflict nào trỏ path cũ.
  - `same_batch_duplicate_ps_name`: hai bản copy cùng PS name trong một lô `Ask` → file thứ nhất `Activated`, file thứ hai `NeedsConfirmation` owner `FontAtlas`.
  - `concurrent_activation_same_hash_is_busy`: hai thread cùng `activate_faces` một hash; `FakeEngine::block_register_until(barrier)` → một `Activated`, một `Busy`; `registrations().len()==1`.
  - `activate_reports_progress_per_file`: 2 file → `progress` được gọi 2 lần, `done` = 1, 2 và `total` = 2.
  - `preflight_flags_rightfont_duplicate`:
    - `FakeEngine::with_system_font("LibreBaskerville-Regular", "/Users/u/Library/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/L/x.ttf")`.
    - `activate_faces(Ask)` → `NeedsConfirmation`, conflicts[0].owner = `RightFont`, `registrations()` rỗng, `activation_row` = `None`.
  - `proceed_registers_despite_conflict`: như trên với `Proceed` → `Activated`, `conflict_ps=Some("LibreBaskerville-Regular")`.
  - `own_registration_is_not_a_conflict`: kích hoạt, rồi `preflight` cùng face → không có conflict.
  - `missing_file_outcome`: xoá file sau ingest → `MissingFile`, không đăng ký.
  - `not_downloaded_outcome` (mọi nền tảng, dùng placeholder `.icloud`; nhánh `SF_DATALESS` chỉ có ở macOS và không test tự động): thay file bằng placeholder `.<tên>.icloud` → `NotDownloaded`.
  - `ttc_face_reports_extra_faces`:
    - Ingest trực tiếp bằng `Store::ingest` một `IngestItem` cho file `libre_baskerville.ttf` thật, với report sửa thành 2 face (face 0, face 1; PS name `"A-0"`, `"A-1"`).
    - Kích hoạt face 1 → `requested_faces=[1]`, `extra_faces=[0]`, register đúng 1 lần.
  - `activate_collection_activates_each_file_once`: collection chứa 2 face cùng file → 1 lần register.
  - `engine_failure_records_error`: `fail_next_register(Rejected{code:999, message:"x"})` → `Failed{code:"engine.rejected"}`, dòng DB `status=Error`.
  - `moved_file_reregisters_new_path`:
    - Kích hoạt, rồi `std::fs::rename` file và gọi `rescan(store, source_id)`.
    - `activate_faces` lại → unregister path cũ, register path mới.

- [ ] **Step 2: Chạy, kỳ vọng FAIL.** Run: `cargo test -p fontatlas-activate --test service`
- [ ] **Step 3: Implement** theo thuật toán trên.
  - Mã lỗi `FileOutcome::Failed.code` theo dạng `"engine.rejected"`, `"engine.unsupported"`, `"store.<code>"`, `"not_found"`.
  - Lỗi store giữa lô → dừng lô và trả `Err(ActivationError::Store)`. Các file đã xử lý xong vẫn còn trong DB.
- [ ] **Step 4: Chạy, kỳ vọng PASS.** Run: `cargo test -p fontatlas-activate`
- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-activate
git commit -m "feat(activate): service preflight + activate by face/collection with conflict policy"
```

---

### Task 6: Deactivate theo face / collection / tất cả + đổi scope

**Files:**
- Modify: `crates/fontatlas-activate/src/service.rs`, `src/types.rs` (2 biến thể `FileOutcome` mới, `DeactivatePolicy`)
- Test: `crates/fontatlas-activate/tests/service.rs`

**Interfaces:**
- Produces:

```rust
pub enum DeactivatePolicy { Ask, Force }
// FileOutcome (Task 5) thêm 2 biến thể:
//   HeldByCollections { collection_ids: Vec<i64> }    // tắt thủ công nhưng collection còn giữ (Ask)
//   KeptActive { manual: bool, collection_ids: Vec<i64> }  // gỡ lý do xong, vẫn còn lý do khác
pub fn deactivate_faces(&self, store: &Mutex<Store>, keys: &[FaceKey], policy: DeactivatePolicy, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
pub fn deactivate_collection(&self, store: &Mutex<Store>, collection_id: i64, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
/// Nút Cài đặt "Tắt mọi font FontAtlas đã kích hoạt" (dọn trước khi gỡ app).
pub fn deactivate_all(&self, store: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
/// Đổi scope của các file đang active (vd bật "Giữ kích hoạt" ở Cài đặt).
pub fn set_scope(&self, store: &Mutex<Store>, hashes: Option<&[[u8; 32]]>, scope: Scope) -> Result<ActivationReport, ActivationError>; // None = mọi dòng active
```

- **Gỡ lý do, theo file:**
  - `deactivate_faces`, `Ask`:
    - Lý do khác `Manual` còn lại (collection) → `HeldByCollections{collection_ids}`, **không đổi gì** (giữ cả `Manual`).
    - Ngược lại → gỡ `Manual`.
  - `deactivate_faces`, `Force`: `clear_activation_reasons`.
  - `deactivate_collection(id)`: tập file = `hashes_with_reason(Collection(id))`, **không** lấy theo `collection_items` hiện tại, để face đã rời collection vẫn được gỡ. Mỗi file gỡ `Collection(id)`. Còn lý do → `KeptActive{manual, collection_ids}`, không gọi engine.
  - `deactivate_all`: `clear_activation_reasons` mọi dòng.
  - Chỉ khi tập lý do **rỗng** mới chạy bước "gỡ đăng ký" dưới đây.
- **Gỡ đăng ký (khi tập lý do rỗng):**
  - Không có dòng hoặc `state=Inactive` và `status≠Registered` → `AlreadyInactive`.
  - Có `active_path` → `unregister(active_path, row.scope)`; `Ok` hoặc `NotRegistered` → ghi `state=Inactive, status=Unregistered`, giữ `active_path`. Không dùng đường dẫn hiện tại của file.
  - Lỗi khác → `Failed`, `status=Error`.
- `set_scope`, với mỗi dòng `Active`+`Registered` khác scope:
  - Transaction ý định: `status=Unknown`, `scope=new`.
  - Mặc định: `register(path, new)` trước, rồi `unregister(path, old)`, để font không biến mất giữa chừng.
  - **Nếu `live_same_url_other_scope` (Task 3) cho thấy CoreText trả 105:** đổi thứ tự thành gỡ `old` trước, rồi `register(new)`. Ghi lựa chọn vào doc của hàm.
  - Xong → `status=Registered`.
  - Crash giữa chừng → `reconcile` thấy `Unknown` và đăng ký đúng `scope` mới; scope cũ có thể còn thì `reconcile` thử gỡ scope còn lại (bỏ qua `NotRegistered`).

- [ ] **Step 1: Viết test thất bại**
  - `deactivate_uses_stored_path_even_if_file_deleted`: kích hoạt, xoá file → deactivate → `Deactivated`, `registrations()` rỗng.
  - `deactivate_not_registered_is_ok` (đã bị gỡ ngoài app → `NotRegistered` → vẫn `Deactivated`).
  - `deactivate_inactive_is_already_inactive`.
  - `two_overlapping_collections_keep_shared_file`:
    - A = {f1, f2}, B = {f2, f3}. Kích hoạt A rồi B.
    - Tắt A → f1 `Deactivated`, f2 `KeptActive{manual:false, collection_ids:[B]}`; `registrations()` = {f2, f3}.
    - Tắt B → `registrations()` rỗng.
  - `manual_plus_collection`:
    - f1 thủ công, rồi collection A chứa f1.
    - Tắt A → `KeptActive{manual:true, collection_ids:[]}`, f1 vẫn đăng ký.
    - `deactivate_faces(f1, Ask)` → `Deactivated`.
  - `manual_deactivate_held_by_collection_asks`:
    - Kích hoạt collection A {f1}.
    - `deactivate_faces(f1, Ask)` → `HeldByCollections{[A]}`, vẫn đăng ký, `reasons` không đổi.
    - `Force` → `Deactivated`, `reasons` rỗng.
  - `deactivate_collection_after_item_removed`: kích hoạt A {f1}, `remove_from_collection(A, f1)`, tắt A → f1 `Deactivated`.
  - `deactivate_all_clears_every_registration_both_scopes`.
  - `set_scope_crash_midway_recovers`: `panic_on_register` khi chuyển sang Persistent → reconcile → chỉ còn `(p, Persistent)`.
  - `set_scope_session_to_persistent_keeps_font_visible`: sau khi chạy, `registrations()==[(p, Persistent)]`, dòng DB `scope=Persistent`.
- [ ] **Step 2: Chạy, kỳ vọng FAIL.** Run: `cargo test -p fontatlas-activate --test service`
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Chạy, kỳ vọng PASS.** Run: `cargo test -p fontatlas-activate`
- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-activate
git commit -m "feat(activate): reason-counted deactivate by face/collection/all and scope switching"
```

---

### Task 7: `reconcile` (khởi động) + `shutdown` (thoát)

**Files:**
- Create: `crates/fontatlas-activate/src/reconcile.rs`, `tests/reconcile.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces trên `ActivationService<E>`:

```rust
pub struct AutoDeactivated { pub hash_hex: String, pub faces: Vec<u32>, pub removed_reasons: Vec<ActivationReason>, pub cause: String /* "collection_deleted" | "left_collection" */ }
pub struct ReconcileReport { pub auto_deactivated: Vec<AutoDeactivated>, pub reregistered: usize, pub moved: usize, pub missing: usize, pub not_downloaded: usize, pub cleaned: usize, pub failed: usize, pub results: Vec<FileResult> }
pub fn reconcile(&self, store: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError>;
pub fn shutdown(&self, store: &Mutex<Store>) -> Result<usize, ActivationError>;   // số dòng session đã gỡ
```

- **Bước 0 của `reconcile`:** `prune_collection_reasons()`. Lưới an toàn khi collection bị xoá hoặc face rời collection mà không qua service. **Không tắt âm thầm:** mỗi file bị tắt vì bước này có một mục trong `ReconcileReport.auto_deactivated`. Plan 6b hiện một thông báo từ danh sách này.
- **Quy tắc `reconcile`**, mỗi dòng trong `activation_rows()` (đọc sau bước 0), đặt `verified_at=now`:

  | Dòng | Tình huống | Hành động | status |
  |---|---|---|---|
  | `state=Active` | `status=Unknown` (crash giữa ghi ý định và register) | `register(active_path, scope)`; với scope còn lại thì `unregister` (bỏ qua `NotRegistered`) | `Registered` |
  | `state=Inactive` | `status=Unknown` hoặc `Registered` | `unregister(active_path, scope)` | `Unregistered`, `cleaned+=1` |
  | `state=Inactive` | `status=Registered` (crash giữa lúc gỡ) | `unregister(active_path, scope)` | `Unregistered`, `cleaned+=1` |
  | `state=Active` | `reasons` rỗng (sau bước 0) | `unregister` (bỏ qua `NotRegistered`), `state=Inactive` | `Unregistered`, `cleaned+=1` |
  | `state=Active` | không có location `Ready` | nếu `active_path` có: `unregister` (bỏ qua `NotRegistered`), để không còn font "ma" | `MissingFile` hoặc `NotDownloaded`; **giữ `state=Active`** |
  | `state=Active` | canonical hiện tại ≠ `active_path` | gỡ cũ, đăng ký mới | `Registered`, `moved+=1` |
  | `state=Active` | còn lại | `register(path, scope)`; `Ok` hoặc `AlreadyRegistered` | `Registered`; `reregistered+=1` chỉ khi `Ok` |

  - `reconcile` **không** kiểm tra xung đột lại: người dùng đã quyết lúc kích hoạt.
- **`shutdown`:** mọi dòng `scope=Session ∧ status=Registered` → `unregister`; `Ok` hoặc `NotRegistered` → `status=Unregistered`, giữ `state=Active`. Lỗi từng dòng không dừng vòng lặp.
- **Thứ tự gọi ở Plan 6a:**
  - `reconcile` chạy sau `Store::open`, và **sau** rescan nếu app rescan lúc mở, để có location mới.
  - UI chỉ hiện trạng thái kích hoạt khi `reconcile` đã xong.
  - `shutdown` chạy trong `RunEvent::Exit`: gỡ session, giữ persistent.

- [ ] **Step 1: Viết test thất bại** trong `tests/reconcile.rs`:
  - `reconcile_reregisters_session_after_logout`: kích hoạt Session → `simulate_logout()` → reconcile → `registrations()==[(p, Session)]`, `reregistered==1`.
  - `reconcile_treats_already_registered_as_ok`: kích hoạt, không logout (mô phỏng crash) → reconcile → `failed==0`, `reregistered==0`, status `Registered`.
  - `reconcile_missing_then_back`:
    - Kích hoạt Persistent, xoá file (giữ bản sao) → reconcile → `registrations()` rỗng, dòng `state=Active, status=MissingFile`.
    - Trả file về đúng chỗ → reconcile → `Registered`.
  - `reconcile_moved_file`: đổi tên file + rescan → reconcile → `moved==1`, path mới.
  - `reconcile_not_downloaded_unregisters_and_keeps_desire`.
  - `reconcile_cleans_inactive_leftover`: put dòng `Inactive+Registered` và đăng ký trực tiếp vào FakeEngine → reconcile → `cleaned==1`.
  - `shutdown_unregisters_session_only`: một Session, một Persistent → `shutdown()==1`, còn Persistent, dòng Session `state=Active, status=Unregistered`.
  - `shutdown_then_reconcile_restores_session` (vòng đời app đầy đủ).
  - `reconcile_resolves_unknown_rows`: tình huống của `crash_after_intent_leaves_unknown_row` → reconcile → `Registered`, `registrations()==[(p, Session)]`.
  - `reconcile_drops_deleted_collection_reason`:
    - f1 chỉ do collection A giữ, f2 do A + `Manual`.
    - Gọi thẳng `store.delete_collection(A)`, không qua service.
    - reconcile → f1 không còn đăng ký, `state=Inactive`; f2 còn đăng ký, `reasons == [Manual]`.
    - `auto_deactivated` có đúng một mục cho f1, `cause="collection_deleted"`.
- [ ] **Step 2: Chạy, kỳ vọng FAIL.** Run: `cargo test -p fontatlas-activate --test reconcile`
- [ ] **Step 3: Implement.** Dùng lại hàm nội bộ "chọn location Ready + canonical" của Task 5, không viết lại.
- [ ] **Step 4: Chạy, kỳ vọng PASS.** Run: `cargo test -p fontatlas-activate`
- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-activate
git commit -m "feat(activate): startup reconcile and quit-time session cleanup"
```

---

### Task 8: Hợp đồng IPC cho Plan 6a/6b, test sống end-to-end, báo cáo

**Files:**
- Create: `tests/contract.rs`, `tests/fixtures/activation_report.json`, `docs/team/plans/plan7-report.md`
- Modify: `tests/mac_live.rs`, `src/lib.rs` (doc crate)

**Interfaces:**
- Produces:
  - `pub struct ActivationStateDto { hash_hex: String, state: DesiredState, scope: Scope, status: ActivationStatus, reasons: Vec<ActivationReason>, active_path: Option<String>, conflict_ps: Option<String>, last_error: Option<String> }` (camelCase).
  - `ActivationService::state(&self, store: &Mutex<Store>) -> Result<Vec<ActivationStateDto>, ActivationError>`.
  - `tests/fixtures/activation_report.json` và `tests/fixtures/activation_state.json`, do serde thật sinh ra. Plan 6b đọc **đúng file này** cho kiểu TS, cùng cơ chế với `face_row.json` của Plan 5.

- [ ] **Step 1: Viết test**
  - `activation_report_json_matches_contract_fixture`:
    - Dựng `ActivationReport` có đủ mỗi biến thể `FileOutcome` (kể cả `HeldByCollections`, `KeptActive`) và một `Conflict{owner: RightFont}`.
    - Một `ActivationStateDto` có `reasons=[Manual, Collection(3)]` → so với `activation_state.json`.
    - `serde_json::to_string_pretty` == nội dung fixture.
    - Nếu biến `UPDATE_FIXTURES=1` thì ghi lại fixture (giống store).
    - Kiểm khoá camelCase: `hashHex`, `requestedFaces`, `extraFaces`, `"kind":"needs_confirmation"`, `existingPath`, `collectionIds`, `"kind":"collection","collectionId":3`.
  - `#[ignore]` + cổng env, trong `mac_live.rs`:
    - `live_service_roundtrip_session`: `ActivationService::new(MacCoreTextEngine)` + Store tạm có fixture → `activate_faces` → `available_postscript_names()` chứa tên → `shutdown()==1` → tên biến mất → `reconcile` → tên có lại → `deactivate_all` → biến mất.
    - `live_vietnamese_filename_roundtrip`: copy fixture thành `"Phở Đậm.ttf"` (tên nhập ở dạng NFC) → kích hoạt → xoá file → `deactivate_faces` → `Deactivated` và tên biến mất.
    - `live_preflight_of_active_file_has_no_conflict`: kích hoạt fixture, rồi `preflight` cùng face → không có conflict (chứng minh path của provider đã được canonical).
    - `live_duplicate_ps_name_is_flagged`: kích hoạt bản copy A, rồi preflight bản copy B cùng PS name → conflict owner `FontAtlas`.
- [ ] **Step 2: Chạy.**
  Run: `UPDATE_FIXTURES=1 cargo test -p fontatlas-activate --test contract && cargo test -p fontatlas-activate`
  Expected: PASS; fixture được tạo, lần chạy thứ hai không đổi.
- [ ] **Step 3 (cần người dùng đồng ý, như Task 3 Step 5):**
  Run: `FONTATLAS_LIVE_ACTIVATION=1 cargo test -p fontatlas-activate --test mac_live -- --ignored --test-threads=1`
  Expected: 6 PASS. Ghi phiên bản macOS (`sw_vers`) vào báo cáo.
- [ ] **Step 4: Viết `docs/team/plans/plan7-report.md`**, gồm:
  - kết quả test (số PASS / ignored, phiên bản macOS)
  - hợp đồng cho Plan 6a, liệt kê lệnh đề xuất ở mục dưới
  - phần chưa làm: Windows (Plan 7b), checklist Adobe/Figma của spike §1.3
- [ ] **Step 5: Chạy toàn workspace.**
  Run: `cargo test --workspace && cargo clippy -p fontatlas-store -p fontatlas-activate --all-targets -- -D warnings`
  Expected: PASS.
- [ ] **Step 6: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-activate docs/team/plans/plan7-report.md
git commit -m "test(activate): IPC contract fixture, live CoreText end-to-end (ignored), plan 7 report"
```

---

## Hợp đồng cho Plan 6a (đề xuất, Lead chốt)

| Lệnh Tauri | Gọi | Ghi chú |
|---|---|---|
| `activation_preflight(keys)` | `preflight` | Trước khi bấm kích hoạt nhiều font |
| `activation_activate(keys, scope?, proceed: bool, channel)` | `activate_faces` | `proceed=true` → `ConflictPolicy::Proceed`; tiến độ qua `Channel<Progress>` |
| `activation_activate_collection(id, scope?, proceed, channel)` | `activate_collection` | |
| `activation_deactivate(keys, force: bool, channel)` | `deactivate_faces` | `force=false` → có thể nhận `HeldByCollections`; UI hỏi "Font này đang được dùng bởi collection X, Y. Tắt luôn?" rồi gọi lại với `force=true` |
| `activation_deactivate_collection(id, channel)` | `deactivate_collection` | `KeptActive` = font vẫn bật vì dự án khác |
| `delete_collection(id)` (lệnh của 6a) | `deactivate_collection(id)` **rồi** `Store::delete_collection` | `reconcile` bước 0 chỉ là lưới an toàn |
| `activation_deactivate_all(channel)` | `deactivate_all` | Nút trong Cài đặt |
| `activation_set_keep(on: bool)` | `set_setting(SETTING_ACTIVATION_KEEP)` + `set_scope(None, …)` | Onboarding và Cài đặt; đặt `SETTING_ACTIVATION_ONBOARDED="1"` |
| `activation_state()` | `ActivationService::state` | UI đánh dấu face nào đang active theo `hash`, kèm lý do |
| khởi động | `reconcile` (sau `Store::open` + rescan) | Chạy nền, không chặn cửa sổ; UI chỉ hiện trạng thái kích hoạt sau event "reconcile xong" |
| thêm face vào collection đang bật | (không tự động) | Bản đầu không tự kích hoạt face mới thêm; người dùng bấm bật lại collection |
| `RunEvent::Exit` | `shutdown` | |

- Mọi lệnh chạy trong `spawn_blocking`.
- Service nhận `&Mutex<Store>` và tự khoá theo từng file (Global Constraints), nên lệnh khác như `query_faces` vẫn chạy trong lúc kích hoạt một collection lớn. Nếu 6a giữ Store theo kiểu khác (ví dụ pool kết nối), Lead báo để đổi chữ ký ở Task 5 trước khi code.
