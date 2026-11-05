# Plan 6a — Backend tích hợp Tauri Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Thay lớp lệnh Tauri cũ (Python sidecar + Qdrant + 28 lệnh) bằng một lớp lệnh mỏng gọi các crate `fontatlas-*`, đủ cho một bản dùng được thật. Lớp mới gồm:
- thư viện, quét và tiến độ;
- hồ sơ font, Similar, Pairing, cặp đã lưu;
- tìm bằng mô tả;
- model và hàng đợi embedding (tải tiếp được);
- onboarding và di chuyển DB cũ;
- lớp lệnh kích hoạt font gọi service của Plan 7, cộng vòng đời mở/thoát app;
- scheme preview/font cùng CSP chặt;
- hợp đồng TypeScript sinh tự động cho Plan 6b.

**Architecture:**
- **Lệnh mỏng:** mỗi `#[tauri::command]` chỉ chuyển tham số rồi gọi một hàm trong `src/services/*`. Hàm service nhận `&Arc<AppState>`, trả `Result<Dto, CmdError>`, test được không cần webview.
- **Năm kết nối SQLite** (WAL, đọc song song; mọi transaction ghi là IMMEDIATE):
  - `AppState.store` (`OnceLock<Mutex<Store>>`): đọc thư viện (`query_faces`, `get_face`…) và service kích hoạt của Plan 7 (nhận `&Mutex<Store>`, khoá theo từng file). **Không** dùng để ghi dữ liệu người dùng;
  - `AppState.user` (`OnceLock<Mutex<Store>>`, mở bằng `Store::open_user_only`: **chỉ user.db, không attach library.db**): mọi ghi dữ liệu người dùng (yêu thích, collection, tag, setting, pairs, nguồn), để không tranh khoá với lô ingest;
  - kết nối riêng của luồng quét (ingest theo lô ngắn, callback tạm dừng nằm ngoài transaction);
  - kết nối riêng của worker embed (sẵn có ở Plan 4);
  - kết nối **đọc riêng** (`Store::open_reader`, `query_only`), mở tạm cho mỗi lần dựng Index; không giữ khoá Store chính.
- **Index:** `RwLock<Option<Arc<IndexBundle>>>`; dựng bằng kết nối đọc riêng, ngoài mọi khoá, rồi mới đổi con trỏ. Dựng lại sau embed bị debounce (Task 15, 16).
- **Lệnh:** mọi lệnh đụng Store/Index/embed là `async` + `spawn_blocking` (hoặc `#[tauri::command(async)]`); không lệnh đồng bộ nào khoá `Mutex<Store>` trên luồng chính. Test tĩnh ở Task 11.
- **DTO mirror:** crate lõi không thêm dependency `ts-rs`; mỗi kiểu lõi đi qua IPC có một DTO trong app crate + `From<>` (Task 19).
- **Tiến độ:** đi qua `ProgressHub`. Mỗi lệnh `*_subscribe(onEvent: Channel)` là một đăng ký; một sự kiện gửi tới mọi đăng ký.
- **Kích hoạt font thuộc Plan 7 (06).** Plan 7 sở hữu `user.db` v2, trait `ActivationEngine`, `FakeEngine`, `ActivationService` và `reconcile`/`shutdown` (nhánh `feat/fontatlas-activate`, plan `docs/plan7-activate` @ d8eec6f). 6a chỉ:
  - bọc service thành lệnh Tauri (Task 17);
  - nối vòng đời: reconcile trước khi UI hiện trạng thái, gỡ session khi thoát.

**Tech Stack:**
- Tauri 2.9.5 (dev: feature `test`), `tauri-plugin-dialog` 2, `tauri-plugin-log` 2, `tauri-plugin-opener` 2 (chỉ gọi từ Rust).
- `ts-rs =12.0.1` (`cargo search ts-rs` ngày 2026-11-05 cho 12.0.1; cache cargo trên máy chỉ có 10.1.0, nên khi thực thi chạy lại `cargo search ts-rs` và ghi kết quả vào ledger trước khi ghim), `fs2 =0.4.3` (`disk_free`; `cargo search fs2` cho 0.4.3), rusqlite `=0.31.0` (bundled).
- Các crate workspace (gồm `fontatlas-activate` của Plan 7); crate mới `fontatlas-describe`.

**Spec:** `docs/superpowers/specs/2026-10-16-fontatlas-mvp-design.md`, mục 2, 3.4, 5.1, 7, 8, 10 (checkout gốc). Cùng với:
- `docs/team/plans/plan6-notes.md` và các mục "cho Plan 6" trong `plan2-report.md`, `plan4-report.md`, `plan5-report.md`;
- `docs/design/01_ui_hien_tai_va_tinh_nang.md` phần B;
- `docs/design/03_ux_flows.md` (nhánh `docs/ux-flows` @ 47ccf03, mục 8 "[Thiếu BE]");
- `docs/team/spikes/04_activation.md` §0, §4;
- `docs/team/research/10_descriptor_dictionary.json` + `_README.md`.

**Mở rộng phạm vi** (chủ app, qua Lead, 2026-11-04): kích hoạt font quay lại MVP (macOS trước); bản phải dùng được thật.

**Nhánh và thứ tự gộp:** `feat/tauri-backend` tách từ **`dev/mvp` mới nhất (3b52f6b trở lên, đã có Plan 6b T1: cửa sổ trong suốt, vibrancy, `macos-private-api`) sau khi Plan 7 đã gộp** (Lead gộp Plan 7 trước). Không push, không merge.
- **Chạy được trước khi Plan 7 gộp** (tách tạm từ `dev/mvp` 3b52f6b, rồi rebase khi Plan 7 vào): Task 1–4, 6–16, 18–19.
- **Phải đợi Plan 7:** Task 5 (`USER_V3` nối sau `USER_V2`), Task 17 (lớp lệnh kích hoạt), Task 20 (smoke toàn bộ).
- Task 12 và 14 có điểm móc tới Task 17 (reconcile lúc khởi động, gỡ kích hoạt khi xoá collection). Trước khi có Plan 7, các điểm móc trả `Ok`.

## Global Constraints

- **Không Python, không Qdrant lúc chạy** (spec 2):
  - xoá `UX/python-sidecar/`, `UX/binaries/`, `UX/rust-backend-plan.rs`;
  - xoá `src-tauri/src/{qdrant,db,logger,lib_logger_snippet}.rs`;
  - bỏ `bundle.externalBin`;
  - bỏ dependency `qdrant-client`, `reqwest`, `uuid`, `chrono`, `tauri-plugin-shell`, `tauri-plugin-fs`.
- **Mọi lệnh đụng Store/Index/embed là `async` + `spawn_blocking`** (`tauri::async_runtime::spawn_blocking`), hoặc `#[tauri::command(async)]`; luồng nền riêng cho quét/embed. Không lệnh đồng bộ nào khoá `Mutex<Store>` trên luồng chính.
  - Ngoại lệ đồng bộ (không đụng Store/Index): `startup_status`, `scan_status`, `preview_info`, `activation_caps` (danh sách `SYNC_OK` trong test).
  - Test tĩnh (Task 11): `commands_touching_store_are_async` duyệt `COMMANDS`, mọi lệnh ngoài `SYNC_OK` phải có `is_async`.
  - Gồm cả: quét, dựng Index, Similar, Pairing, `query_faces` có lọc trục, tải model, kích hoạt hàng loạt, xử lý scheme.
- **IPC:**
  - tên lệnh snake_case; tham số và trường JSON camelCase (spec 7.2);
  - enum serde `snake_case`;
  - sự kiện `#[serde(tag = "type", rename_all = "camelCase")]`.
- **Lỗi lệnh:** mọi lệnh trả `Result<T, CmdError>`, với `CmdError { code, message, detail: Option<serde_json::Value> }`.
  - Mã: `store_unavailable`, `not_found`, `not_comparable`, `invalid_arg`, `nested_source`, `busy`, `model_missing`, `cancelled`, `io`, `insufficient_disk`, `activation_unsupported`, `internal`;
  - cộng nguyên mã của `StoreError::code()` và `activation_<ActivationError::code()>`.
  - Không panic trong setup hay trong lệnh; không `unwrap`/`expect` trên dữ liệu ngoài.
  - Mã `io` có hai nguồn: `StoreError::Io` (kèm `detail {"source":"store"}`) và lỗi I/O của chính 6a (kèm `detail {"source":"app","op":"…"}`). Frontend phân biệt bằng `detail.source`.
- **Khoá face trên IPC:** `FaceKeyDto { hashHex: String, faceIndex: u32 }`.
- **Không chặn theo giấy phép** (spec 5.1 và quyết định của chủ app):
  - `license_class` chỉ để hiển thị;
  - embedding và kích hoạt chạy cho mọi font;
  - `license_notice_ack` chỉ lưu cờ đã hiện thông báo, không gate gì.
- **Không ghi đè `user.db` khi migration hỏng** (spec 10): `Store::open` lỗi thì app vào trạng thái `failed`, không ghi gì thêm.
- **Thứ tự khởi động:**
  1. `Store::open`;
  2. mở `PreviewService`;
  3. kiểm DB cũ (chỉ phát hiện; nhập khi người dùng chọn ở onboarding O2);
  4. quét lại mọi nguồn thư mục nếu `rescan_on_launch` **hoặc** `analyzer_version_seen` khác `ANALYZER_VERSION` (đổi version tự phân tích lại, bất kể `rescanOnLaunch`);
  5. `resolve_legacy` (nếu đã nhập);
  6. dựng Index (kết nối đọc riêng; dùng bản phân tích mới nhất sẵn có, bất kể version, cho tới khi phân tích lại xong);
  7. `activation.reconcile` (Plan 7), xong **trước** phase `ready`, tức trước khi UI hiện trạng thái kích hoạt;
  8. mở worker embed (luôn **sau** Store chính). `enqueue_embeddings` + `ensure_started` cũng chạy sau mỗi `Idle` của quét và sau khi model chuyển `Ready`.
- **Vòng đời worker embed** (plan6-notes):
  - `join` worker cũ trước khi mở worker mới;
  - tắt `embed_enabled` thì `cancel()` rồi `join`;
  - thoát app: `lifecycle::on_exit(&AppState)` (Task 16) cancel và join cả quét lẫn embed (≤ 5 s), rồi `activation.shutdown` (gỡ scope `session`). `RunEvent::Exit` chỉ gọi hàm này.
- **Kích hoạt:** mọi quy tắc (đơn vị file, scope, lý do kích hoạt, xung đột, đường dẫn canonical, `Busy`) nằm trong Plan 7 và được service của Plan 7 thực thi.
  - 6a **không** tự đăng ký hay gỡ font;
  - 6a **không** định nghĩa engine;
  - 6a **không** động vào `activation_state` hay `activation_reasons`.
- **`user.db`:** Plan 7 sở hữu v2. 6a chỉ nối **v3** (`pairs`, Task 5) qua `migrate()` có sẵn (sao lưu, BEGIN IMMEDIATE/COMMIT, ROLLBACK khi lỗi). Cấu hình khác của 6a dùng bảng `settings`, không thêm migration.
- **CSP** (plan5-report, nguyên văn), cộng `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'`:

  ```
  img-src 'self' data: blob: fontatlas-preview: http://fontatlas-preview.localhost
  font-src 'self' fontatlas-font: http://fontatlas-font.localhost
  connect-src 'self' ipc: http://ipc.localhost fontatlas-preview: http://fontatlas-preview.localhost
  ```

- **Capabilities:** chỉ `core:default`, `core:window:allow-start-dragging`, `dialog:allow-open`.
  - Không `fs:*`, `shell:*`, `opener:*`, `log:*`. Log chỉ từ Rust (`tauri-plugin-log` đăng ký ở Rust); frontend không có lệnh log và không có quyền plugin log.
  - "Hiện trong Finder" là lệnh Rust lấy đường dẫn từ Store (frontend không truyền path).
- **TS:**
  - `ts-rs =12.0.1`, DTO mirror trong app crate (crate lõi không có `ts-rs`), xuất `UX/bindings/` (env `TS_RS_EXPORT_DIR`);
  - bản đồ lệnh `UX/bindings/commands.ts`;
  - fixture `UX/bindings/fixtures/*.json`.
- **`FaceRowDto`** giữ đúng 19 khoá của `FaceRow`: parser Plan 5 từ chối khoá lạ (fixture `crates/fontatlas-store/tests/fixtures/face_row.json`).
- **`ANALYZER_VERSION` đổi đúng một lần**, thành `"core-0.3.0"` (Task 6, vì thêm trường Metadata). `reference-v1.json` dựng lại từ kho có sẵn **`/Users/vietx/fontatlas-ref/google-fonts`** (google/fonts @ `47d54ca`), không tải lại. Bản phân tích cũ vẫn được Index dùng (mới nhất mỗi face) cho tới khi phân tích lại xong.
- **Cấu hình cửa sổ của Plan 6b T1 giữ nguyên:** `tauri` giữ feature `macos-private-api`; `tauri.conf.json` giữ `app.macOSPrivateApi`, `transparent`, `windowEffects`. Task 11 chỉ đổi CSP và `externalBin`; `tests/config.rs` kiểm các khoá này.
- **Nền tảng:** macOS kiểm thật. **Windows chưa kiểm**, ghi trong báo cáo.
- Comment tiếng Việt, ngắn. Mỗi commit kết thúc bằng `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Ổ ngoài rút giữa lúc quét:**
   - không vị trí nào bị đánh `missing`;
   - sự kiện cuối `finished` có `status = "source_unavailable"`;
   - `list_sources` báo `available = false`;
   - font đang kích hoạt trên ổ đó không bị gỡ khỏi `desired`;
   - kiểm `root_available` sau mỗi lô; lỗi `io` không lưu thành `Analysis::Failed` (file để `pending`).

   Test: Task 2 `unplugged_mid_scan_marks_nothing_missing`, `io_error_stays_pending_not_failed`, Task 13 `unavailable_source_reported`, Task 17 `unplugged_active_font_stays_desired` (nối dây; logic ở Plan 7 `reconcile_missing_then_back`).
2. **App tắt giữa hàng đợi** (quét, embed, kích hoạt hoặc tải model):
   - quét huỷ thì không đánh `missing`, trạng thái `cancelled`;
   - worker embed cancel + join ≤ 5 s; mở lại thì job `running` về `pending`;
   - model tải dở giữ `.part` để tải tiếp;
   - font scope `session` được gỡ, `persistent` giữ.

   Test: Task 2 `cancelled_scan_marks_nothing_missing`, Task 16 `shutdown_cancels_and_joins_workers`, Task 16 `restart_resumes_running_jobs`, Task 9 `cancel_keeps_part_and_resumes`, Task 17 `exit_shuts_down_session_only`.
3. **DB cũ hỏng hoặc `user.db` mới hơn app:**
   - không panic, không sửa file nào;
   - trạng thái nêu mã lỗi;
   - app vẫn dùng được (DB cũ rác) hoặc vào `failed` (user.db mới hơn).

   Test: Task 12 `corrupt_legacy_db_is_reported_and_untouched`, `newer_user_db_fails_without_writing`.
4. **Hai cửa sổ hoặc hai lệnh quét/kích hoạt cùng lúc:**
   - cùng nguồn chỉ một lần quét;
   - mọi đăng ký nhận cùng chuỗi sự kiện;
   - yêu thích/collection/tag ghi qua kết nối `user` riêng, trễ < 200 ms trong lúc quét (không dựa vào `busy_timeout`);
   - hai lệnh kích hoạt cùng hash: một lệnh nhận `Busy` (Plan 7 `concurrent_activation_same_hash_is_busy`); lớp lệnh không giữ khoá Store quanh service.

   Test: Task 13 `duplicate_rescan_is_single_flight`, `two_subscribers_get_same_events`, Task 1 `concurrent_writers_do_not_fail_busy`, `user_only_handle_unaffected_by_ingest_lock`, Task 13 `favorite_during_scan_latency_under_200ms`, Task 17 `activate_collection_via_command_reports_progress`.
5. **Model chưa tải, tải dở, sai checksum hoặc URL rỗng:**
   - Similar và Pairing chạy theo trục, `source = "axes"`;
   - `model_status` báo đúng trạng thái;
   - worker không mở; không lỗi nào lọt lên lệnh Similar;
   - server bỏ qua `Range` thì tải lại từ đầu, UI nhận `resumed:false`; lỗi cuối (vd `checksum_mismatch`) lưu vào settings nên trạng thái suy ra được sau khi mở lại app.

   Test: Task 15 `similar_works_without_model`, Task 16 `worker_not_started_without_model`, Task 9 `checksum_mismatch_reported`, `server_ignoring_range_restarts_with_resumed_false`, Task 16 `model_last_error_survives_restart`.

---

## Thay đổi sau review độc lập (2026-11-05)

| # | Nội dung | Nơi sửa |
|---|---|---|
| 1 | DTO mirror trong app crate, lõi không thêm `ts-rs`; ghim `12.0.1` đã kiểm `cargo search` | Tech Stack, Task 19 |
| 2 | Lệnh `async` + `spawn_blocking`; test tĩnh duyệt `COMMANDS` | Global, Task 11 |
| 3 | Dựng Index bằng kết nối đọc riêng; debounce ≥ 30 s hoặc khi hàng đợi rỗng; mô tả 5 kết nối | Architecture, Task 1, 15, 16 |
| 4 | Ghi dữ liệu người dùng qua kết nối chỉ user.db; lô ingest ngắn, callback ngoài transaction; test độ trễ < 200 ms | Task 1, 2, 13 |
| 5 | Ổ rút giữa lô: `root_available` mỗi lô; `io` không thành `Failed` | Task 2 |
| 6 | Index dùng bản phân tích mới nhất bất kể version; đổi version tự phân tích lại | Task 3, 6, 12, 15 |
| 7 | `enqueue_embeddings` + `ensure_started` sau Idle của quét và sau model `Ready` | Task 13, 16 |
| 8 | `faceKeys`, `get_setting`/`set_setting` (`ui.*`, ≤ 64 KB); `rebuild_library` hoãn | Task 14 |
| 9 | `resumed:false` khi server bỏ qua Range; lỗi cuối lưu settings; `disk_free` dùng `fs2` | Task 9, 16 |
| 10 | Base dev/mvp 3b52f6b; giữ `macos-private-api`, `macOSPrivateApi`, `transparent`, `windowEffects`; test config | Global, Task 11 |
| 11 | Log chỉ từ Rust; capabilities tối thiểu | Global, Task 11 |
| 12 | `lifecycle::on_exit(&AppState)` tách riêng. **Chữ ký API Plan 7 chờ cập nhật sau Plan 7 Task 8** | Task 16, 17 |
| m | `create_user_v1_for_test` chỉ có sau Plan 7; `existingPath` tra thêm; `PairOptsDto.role` có `code`; mã `io` hai nguồn | Task 5, 13, 15, Global |

---

## File Structure

```
UX/src-tauri/
├── Cargo.toml                  # edition 2024; bỏ dep cũ; thêm crate, ts-rs, opener; members + describe
├── .cargo/config.toml          # [env] TS_RS_EXPORT_DIR = "../bindings" (relative)
├── tauri.conf.json             # bỏ externalBin, CSP mới
├── capabilities/default.json   # core + dialog:allow-open
├── src/
│   ├── lib.rs                  # Builder, plugin, scheme, generate_handler từ macro commands!, RunEvent::Exit
│   ├── error.rs  state.rs  hub.rs  index_bundle.rs  commands.rs  bindings.rs
│   ├── dto/{mod,face,library,analysis,settings,events,activation,model}.rs
│   └── services/{boot,onboarding,sources,scan,library,analysis,describe,pairs,settings,model,embed,activation,lifecycle,preview,system}.rs
├── tests/{config,boot,scan,library,ipc,analysis,embed,activation,preview}.rs, smoke.sh
├── crates/fontatlas-core/       # Metadata + copyright, vendor_url, named_instance_coords; core-0.3.0
├── crates/fontatlas-store/      # IMMEDIATE, open_user_only, open_reader, rescan tiến độ/huỷ, đọc analyses, scope query, user v3 (pairs, sau v2 của Plan 7), legacy_preview, backup
├── crates/fontatlas-index/      # storeys, phân vị độ lớn, summarize_face, similar_with, similar chỉ ảnh, lọc ứng viên; reference-v1 dựng lại
├── crates/fontatlas-embed/      # tải model tiếp được (Range), tốc độ
├── crates/fontatlas-describe/   # MỚI: từ điển mô tả → bộ lọc
└── crates/fontatlas-activate/   # của Plan 7 — 6a chỉ dùng, không sửa
UX/bindings/                     # sinh: *.ts, commands.ts, fixtures/*.json, README.md
.github/workflows/ci.yml         # kiểm bindings không lệch
docs/team/plans/plan6a-report.md
```

**Lệnh cũ (28):** gỡ hết, không giữ shim. Frontend cũ (`UX/services/*.ts`, `hooks/useFontLoader.ts`, `PreferencesPanel.tsx`) sẽ hỏng lúc chạy cho tới 6b, nhưng `npm run build` vẫn qua. Lưới Plan 5 đã khớp hợp đồng mới. Ánh xạ cũ → mới nằm ở bảng lệnh cuối plan (cột "thay cho").

---

### Task 1: Store — IMMEDIATE, kết nối chỉ-user.db, kết nối đọc riêng

**Files:** Modify `crates/fontatlas-store/src/{ingest,legacy,rescan,user,open}.rs` (12 chỗ `self.conn.transaction()`), `src/lib.rs`. Test `tests/concurrency.rs`.

**Interfaces — Produces:**
- Mọi transaction ghi dùng `transaction_with_behavior(TransactionBehavior::Immediate)`.
- `Store::open_user_only(dir: &Path) -> Result<Store, StoreError>`: main = DB trong bộ nhớ, `ATTACH user.db AS u` (SQL tiền tố `u.` của `user.rs` chạy nguyên), **không mở library.db**.
  - Không migrate: đòi `Store::open` đã chạy trước; `user_version` lệch `USER_VERSION` thì trả `StoreError` mã sẵn có của migration, không ghi.
  - Chỉ gọi hàm user: `set_favorite`, collection, tag, `setting`/`set_setting`, `pairs`, nguồn. Hàm user nào JOIN `faces`/`file_locations` thì tách phần JOIN ra khỏi đường user-only (khoá theo hash); hàm cần bảng library lỗi `no such table`.
- `Store::open_reader(dir: &Path) -> Result<Store, StoreError>`: library.db + attach user.db, `PRAGMA query_only=ON`, không migrate. Dùng để dựng Index.
- Chữ ký công khai khác không đổi.

- [ ] **Step 1: Test thất bại**
  - `concurrent_writers_do_not_fail_busy`:
    - hai `Store` cùng thư mục chạy trên hai luồng;
    - luồng A đổi yêu thích 200 lần; luồng B tạo rồi đổi tên 200 collection;
    - không lần gọi nào lỗi; cuối cùng `collections().len() == 200`.
  - `user_only_handle_unaffected_by_ingest_lock`: luồng A giữ một transaction IMMEDIATE ghi library.db 1 s (ingest giả); `open_user_only` gọi `set_favorite` 50 lần, mỗi lần < 200 ms, không lỗi.
  - `open_user_only_does_not_touch_library_db`: sha256 library.db không đổi; vẫn chạy khi library.db bị xoá.
  - `reader_is_query_only`: `open_reader` rồi ghi → lỗi readonly; đọc `faces` được khi A đang ingest (WAL).
- [ ] **Step 2: Chạy, xác nhận lỗi `database is locked` / thiếu hàm.**
  - Run: `cd UX/src-tauri && cargo test -p fontatlas-store --test concurrency`
  - Nếu `concurrent_writers…` chạy 10 lần không fail: ghi ledger, giữ test làm hồi quy.
- [ ] **Step 3: Đổi 12 chỗ sang IMMEDIATE; cài `open_user_only`, `open_reader`.**
- [ ] **Step 4: Chạy** `cargo test -p fontatlas-store`. Expected: PASS.
- [ ] **Step 5: Commit** — `fix(store): IMMEDIATE write transactions; user-only and reader connections`

---

### Task 2: Store — quét có tiến độ, huỷ, phân tích theo lô

**Files:** Modify `crates/fontatlas-store/src/rescan.rs`, `src/lib.rs`. Test `tests/rescan_progress.rs`.

**Interfaces — Produces:**
- `pub enum ScanPhase { Walking, Hashing, Analyzing, MarkingMissing }`
- `pub struct ScanProgress { pub phase: ScanPhase, pub done: usize, pub total: usize, pub faces_written: usize }` (camelCase). `faces_written` là số face ghi trong lô vừa xong; UI dùng để hiện font dần.
- `RescanStatus::Cancelled`
- `pub fn rescan_source_with(&mut self, source_id: i64, on_progress: &mut dyn FnMut(&ScanProgress), cancel: &AtomicBool) -> Result<RescanStats, StoreError>`
- `rescan_source(id)` = bản `_with` với callback rỗng và cờ `false`.

Hành vi:
- **Sự kiện:**
  - `Walking` một lần;
  - `Hashing` một lần;
  - `Analyzing` sau mỗi lô `INGEST_BATCH`: lô = `analyze_paths` rồi `ingest` ngay (spec S1, font hiện dần);
  - `MarkingMissing`.
- **Kiểm `cancel`:** sau Walking, sau Hashing, sau mỗi lô.
  - Bị huỷ thì `status = Cancelled`, **không** đánh missing.
  - Callback có thể chặn (dùng cho tạm dừng ở Task 13).
- **Ổ rút giữa lúc quét:** kiểm `root_available` sau Walking, sau Hashing và sau **mỗi lô**. Mất thì `SourceUnavailable`, không đánh missing.
  - Lỗi `io` khi đọc/hash file (NotFound, Other…) **không** ghi thành `Analysis::Failed`: file giữ `pending`, lần quét sau thử lại. Chỉ lỗi phân tích thật (font hỏng) mới thành `Failed`.
- **Lô ngắn, callback ngoài transaction:** `INGEST_BATCH` ≤ 64 file; `on_progress` luôn gọi **ngoài** transaction, không giữ khoá ghi khi chờ (tạm dừng, huỷ).

- [ ] **Step 1: Test thất bại.**
  - Fixture: 600 bản sao `roboto.ttf`, mỗi bản đổi 4 byte cuối để có hash khác nhau.
  - `progress_phases_in_order`: thứ tự `[Walking, Hashing, Analyzing×3, MarkingMissing]`; lô Analyzing cuối có `done == total == 600`; tổng `faces_written == 600`.
  - `cancelled_scan_marks_nothing_missing`: quét đủ; xoá 10 file; quét lại, đặt cancel ở lô Analyzing đầu → `Cancelled`; `missing=1` đếm được 0.
  - `unplugged_mid_scan_marks_nothing_missing`: callback đổi tên thư mục gốc ở lô Analyzing đầu → `SourceUnavailable`; `missing=1` đếm được 0.
  - `io_error_stays_pending_not_failed`: callback đổi tên thư mục gốc ở lô giữa → các file chưa xử lý không có dòng `failed` (đếm 0 dòng `Failed` trong `analyses`/`pipeline_state`); sau khi trả thư mục và quét lại, mọi file có bản phân tích.
  - `progress_callback_runs_outside_transaction`: callback mở kết nối thứ hai (`busy_timeout` 0) làm `BEGIN IMMEDIATE; COMMIT` trên library.db, thành công.
- [ ] **Step 2: Chạy, xác nhận fail biên dịch.**
  - Run: `cargo test -p fontatlas-store --test rescan_progress`
- [ ] **Step 3: Cài đặt.**
- [ ] **Step 4: Chạy** `cargo test -p fontatlas-store`. Expected: PASS; số đếm `RescanStats` của test cũ không đổi.
- [ ] **Step 5: Commit** — `feat(store): rescan progress callback, cancellation, batched analyze+ingest`

---

### Task 3: Store — đọc bản phân tích, vị trí file, xem trước DB cũ, sao lưu

**Files:** Modify `crates/fontatlas-store/src/{query,legacy,open}.rs` (hoặc tạo `src/analysis.rs`), `src/lib.rs`. Test `tests/analysis_read.rs`.

**Interfaces — Produces:**

```rust
pub struct StoredAnalysis { pub face_id: i64, pub key: FaceKey, pub metadata_json: String, pub vietnamese_json: String,
    pub anatomy_json: String, pub missing: bool, pub source_ids: Vec<i64>, pub container: Option<String>, pub file_size: u64,
    pub analyzer_version: String }
impl Store {
    /// Mỗi face một dòng: bản phân tích source='local' MỚI NHẤT có sẵn, bất kể analyzer_version
    /// (Index vẫn dùng được sau khi đổi ANALYZER_VERSION, cho tới khi phân tích lại xong). Trả số dòng.
    pub fn for_each_latest_analysis(&self, f: &mut dyn FnMut(StoredAnalysis)) -> Result<usize, StoreError>;
    /// Số face có bản phân tích mới nhất khác ANALYZER_VERSION (0 = đã phân tích lại xong).
    pub fn stale_analysis_count(&self) -> Result<u64, StoreError>;
    /// Tra face_id theo khoá (bỏ khoá không có); dùng cho `faceKeys` (Task 14).
    pub fn face_ids_of(&self, keys: &[FaceKey]) -> Result<Vec<(FaceKey, i64)>, StoreError>;
    pub fn analysis_of(&self, key: &FaceKey) -> Result<Option<StoredAnalysis>, StoreError>;
    /// vị trí missing=0 có location_id nhỏ nhất
    pub fn face_path(&self, key: &FaceKey) -> Result<Option<PathBuf>, StoreError>;
    /// sao lưu user.db bằng VACUUM INTO vào dir/backups/user-<ms>.db; trả đường dẫn
    pub fn backup_user_db(&self) -> Result<PathBuf, StoreError>;
}
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct LegacyPreview { pub fonts: usize, pub sources: usize, pub collections: usize, pub favorites: usize, pub tags: usize }
/// Đọc chỉ-đọc DB cũ để đếm, không ghi gì.
pub fn legacy_preview(old_db: &Path) -> Result<LegacyPreview, StoreError>;
```

- [ ] **Step 1: Test thất bại.**
  - `reads_latest_analyses_with_face_id_and_sources`: 5 fixture → 5 dòng; `metadata_json` parse được thành `Metadata`; `source_ids == [id]`; `container == Some("sfnt")`.
  - `face_path_skips_missing_locations`
  - `latest_analysis_used_until_reanalyzed`: face chỉ có dòng `analyzer_version` cũ vẫn được đọc; sau khi thêm dòng mới, chỉ dòng mới được trả (một dòng mỗi face).
  - `stale_analysis_count_counts_old_versions`
  - `face_ids_of_skips_unknown_keys`
  - `legacy_preview_counts_without_writing`: dùng `fontatlas.db` mẫu trong test legacy; sha256 của file không đổi sau khi gọi.
  - `backup_user_db_creates_readable_copy`: bản sao mở được, có cùng số collection.
- [ ] **Step 2: Chạy, xác nhận fail.**
- [ ] **Step 3: Cài đặt.**
- [ ] **Step 4: Chạy** `cargo test -p fontatlas-store`. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(store): read analyses, face path, legacy preview, on-demand user backup`

---

### Task 4: Store — `query_faces` có phạm vi và xếp theo điểm khớp

**Files:** Modify `crates/fontatlas-store/src/query.rs`. Test `tests/query_scope.rs`.

**Interfaces — Produces:**
- `SortKey::Match` (serde `"match"`).
- `pub fn query_faces_scoped(&self, q: &FaceQuery, scope: Option<&[(i64, f32)]>) -> Result<FacePage, StoreError>`
  - `scope = (face_id, score)`; `total` đếm trong scope;
  - `Match` xếp `score DESC, family, face_id`; không có scope thì `Match` xếp như `Family`.
- `query_faces(q)` = `query_faces_scoped(q, None)`.

Cách cài: bảng `temp.scope(face_id INTEGER PRIMARY KEY, score REAL NOT NULL)` trên chính kết nối, nạp lại trong một transaction mỗi lần gọi, rồi JOIN. Không dùng `IN (…)`, vì 50k tham số vượt giới hạn.

- [ ] **Step 1: Test thất bại**
  - `scope_limits_rows_and_total`
  - `match_sort_orders_by_score_desc`
  - `scope_combines_with_text_and_vi_filters`
  - `large_scope_50k_ids_ok`
- [ ] **Step 2–4:** fail → cài → `cargo test -p fontatlas-store` PASS.
- [ ] **Step 5: Commit** — `feat(store): scoped face query with match sort`

---

### Task 5: Store — user.db v3: bảng `pairs` (cặp đã lưu)

**Phụ thuộc Plan 7.** Plan 7 sở hữu v2 (`activation_state` định nghĩa lại, `activation_reasons`; commit `c59523c` trên `feat/fontatlas-activate`). Task này chỉ **nối thêm** `USER_V3` sau `USER_V2`, nên phải chạy trên `dev/mvp` **đã gộp Plan 7**. Không sửa `USER_V1`, `USER_V2` hay `activation_state`.

**Files:**
- Modify `crates/fontatlas-store/src/schema.rs`:
  - thêm `USER_V3`;
  - `USER_MIGRATIONS = &[USER_V1, USER_V2, USER_V3]`;
  - thêm `#[doc(hidden)] pub fn create_user_v2_for_test(path)`, chạy `USER_V1` + `USER_V2` + `PRAGMA user_version = 2`, cùng mẫu với `create_user_v1_for_test`. Hàm này **chỉ có sau khi gộp Plan 7** (commit `c59523c`); trước đó không tồn tại, nên Task 5 không chạy sớm hơn.
- Create `src/pairs.rs`; modify `src/lib.rs`.
- Test `tests/user_v3.rs`.

**Interfaces — Produces:**

```sql
-- USER_V3 (chạy qua `migrate()` có sẵn: sao lưu VACUUM INTO một lần trước chuỗi, mỗi bước BEGIN IMMEDIATE…COMMIT, lỗi thì ROLLBACK)
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
```

```rust
pub struct SavedPair { pub id: i64, pub a: FaceKey, pub a_role: String, pub b: FaceKey, pub b_role: String,
    pub mode: Option<String>, pub preset: Option<String>, pub sample_text: Option<String>,
    pub collection_id: Option<i64>, pub note: Option<String>, pub created_at: i64 }   // Serialize camelCase
pub struct NewPair { /* các trường SavedPair trừ id, created_at */ }
impl Store {
    pub fn save_pair(&mut self, p: &NewPair) -> Result<i64, StoreError>;
    pub fn pairs(&self, collection_id: Option<i64>) -> Result<Vec<SavedPair>, StoreError>;
    pub fn delete_pair(&mut self, id: i64) -> Result<(), StoreError>;
}
```

- [ ] **Step 1: Test thất bại**
  - `v1_to_v3_migrates`: `create_user_v1_for_test` + collection mẫu → `Store::open`:
    - `USER_VERSION == 3`;
    - collection còn;
    - có bảng `activation_reasons` (v2 đã chạy) và `pairs`;
    - `OpenReport.user_backup` có file `user.v1.*.db`.
  - `v2_to_v3_migrates_keeps_activation_rows`: `create_user_v2_for_test`, chèn một dòng `activation_state` cộng lý do `manual` → mở → `user_version == 3`; dòng và lý do còn nguyên; backup `user.v2.*.db`.
  - `v3_failure_rolls_back_to_v2`: DB v2 đã có sẵn một bảng `pairs(x)` lạ.
    - `Store::open` trả `StoreError::Migration { from: 2, to: 3, .. }`;
    - `PRAGMA user_version` vẫn 2; bảng lạ không đổi; file backup đã tạo.
  - `pairs_roundtrip_and_collection_delete_sets_null`
- [ ] **Step 2–4:** fail → cài → `cargo test -p fontatlas-store` PASS (gồm test v2 của Plan 7 trong `tests/activation.rs`).
- [ ] **Step 5: Commit** — `feat(store): user.db v3 saved pairs (after Plan 7 v2)`

---
### Task 6: Core — `copyright`, `vendor_url`, toạ độ named instance; `core-0.3.0`; dựng lại reference

**Files:**
- Modify `crates/fontatlas-core/src/{metadata,lib}.rs`.
- Dựng lại `crates/fontatlas-index/data/reference-v1.json`.
- Test `crates/fontatlas-core/tests/metadata.rs`.

**Interfaces — Produces:**
- `Metadata` thêm:
  - `copyright: Option<String>` (nameID 0);
  - `vendor_url: Option<String>` (nameID 11);
  - `named_instance_coords: Vec<NamedInstanceInfo>`, với `pub struct NamedInstanceInfo { pub name: Option<String>, pub coords: Vec<(String, f32)> }` (toạ độ user-space theo thứ tự trục `fvar`).
  - `named_instances: Vec<String>` giữ nguyên.
- Ba trường mới của `Metadata` có `#[serde(default)]`, để đọc được JSON cũ.
- `ANALYZER_VERSION = "core-0.3.0"`. Boot tự phân tích lại các dòng cũ bất kể `rescanOnLaunch` (Task 12); Index dùng bản cũ cho tới khi xong (Task 3, 15).

- [ ] **Step 1: Test thất bại.**
  - `copyright_and_vendor_url_read`: Open Sans có `copyright` chứa "Copyright"; `vendor_url` là `Some` hoặc `None` đúng theo bảng name thật (in ra rồi chốt bằng giá trị thật, không đoán).
  - `public_sans_named_instances_have_wght`: mọi instance có `coords` chứa `("wght", w)` với 100 ≤ w ≤ 900; có instance "Bold" với `wght == 700`.
  - `analyzer_version_is_core_0_3_0`
  - `old_metadata_json_parses_with_defaults`: JSON `Metadata` không có ba trường mới vẫn parse, giá trị mặc định rỗng/`None`.
- [ ] **Step 2–4:** fail → cài → `cargo test -p fontatlas-core -p fontatlas-store` PASS.
- [ ] **Step 5: Dựng lại reference.**
  - Kho có sẵn: `/Users/vietx/fontatlas-ref/google-fonts`. Kiểm SHA trước: `git -C /Users/vietx/fontatlas-ref/google-fonts rev-parse HEAD` phải bằng `47d54ca1ebb14fd864695f1d39dc734f39017c33`. Không tải lại.
  - Run: `cargo run --release -p fontatlas-eval --bin refbuild -- --fonts /Users/vietx/fontatlas-ref/google-fonts --out crates/fontatlas-index/data/reference-v1.json --version ref-v1 --source "google/fonts@$(cat ../../tools/reference/GOOGLE_FONTS_SHA) ofl+apache"`
  - Expected:
    - `cargo test -p fontatlas-index` PASS (`bundled_reference_is_valid` đọc `ANALYZER_VERSION`);
    - `git diff` chỉ đổi `analyzer_version`, vì Anatomy không đổi nên quantiles và `d_fam` phải y hệt. Lệch thì dừng và báo Lead.
- [ ] **Step 6: Commit** — `feat(core): copyright, vendor_url, named instance coordinates; core-0.3.0; rebuild reference-v1`

---

### Task 7: Index — storeys, phân vị độ lớn, câu tóm tắt, Similar theo yêu cầu, lọc ứng viên Pairing

**Files:** Modify `crates/fontatlas-index/src/{profile,reference,explain,index}.rs`, `src/lib.rs`. Test thêm vào `tests/{profile,reference,explain,similar,pairing}.rs`.

**Interfaces — Produces:**
- `FaceProfile` thêm `a_storeys: Reading`, `g_storeys: Reading`.
  - Thiếu thì `Failed`; không vào 16 chiều Similar.
- `ReferenceTable::magnitude_percentile(&self, axis: AxisId, method: &str, v: f32) -> Option<f32>`
  - `= percentile(|v|) − percentile(−|v|)`, kẹp [0, 1];
  - chỉ cho `StressDeg`, `SlantDeg`, trục khác trả `None`.
- `pub fn summarize_face(p: &FaceProfile, r: &ReferenceTable, lang: Lang) -> String`
- `pub struct SimilarOptions { pub weights: GroupWeights, pub embed_weight: Option<f32>, pub k: usize }`
- `Index::similar_with(&self, q: &FaceKey, o: &SimilarOptions, filter: &dyn Fn(&FaceProfile) -> bool) -> Result<Vec<SimilarHit>, IndexError>`
  - `Some(0.0)` → chỉ trục; `similar()` cũ gọi bản này.
- `Index::pair_candidates_filtered(&self, q, q_role, cand_role, project_vi, k, filter: &(dyn Fn(&FaceProfile) -> bool + Sync)) -> Result<…>`
  - `filter` áp **trước** `assess` (lọc theo collection hoặc face missing);
  - `pair_candidates` cũ = filter luôn true.

**`summarize_face`** ghép tối đa 4 mệnh đề, theo thứ tự, ngăn bằng dấu phẩy, kết thúc bằng dấu chấm, rồi nối câu tiếng Việt.

Mệnh đề hình học, chỉ khi `Measured` và có phân vị `p`:
- **Lớp** theo `serif_foot`: < 1,2 → "Sans"; > 1,6 → "Serif".
- **Tương phản:** `p ≥ 0,65` → "tương phản cao" / "high contrast"; `p ≤ 0,35` → "tương phản thấp" / "low contrast".
- **x-height:** `p ≥ 0,65` → "x-height cao" / "tall x-height"; `p ≤ 0,35` → "x-height thấp" / "short x-height".
- **Aperture:** `p ≥ 0,65` → "aperture mở" / "open aperture"; `p ≤ 0,35` → "aperture hẹp" / "narrow aperture".

Câu tiếng Việt: "Đủ 134 ký tự Việt." / "All 134 Vietnamese letters.", hoặc "Thiếu {n} ký tự Việt." / "Missing {n} Vietnamese letters.". Các chuỗi này vào `all_templates()`.

- [ ] **Step 1: Test thất bại.**
  - `profile_reads_storeys`: Roboto `a = 2`, `g = 1`.
  - `magnitude_percentile_symmetric`: q tuyến tính −30..30 → ±15 cho ≈ 0,5; 0 cho 0.
  - `summary_playfair_vi`: bắt đầu "Serif, tương phản cao" và kết thúc "Đủ 134 ký tự Việt.".
  - `summary_libre_missing`: chứa "Thiếu 68 ký tự Việt.".
  - `summary_non_latin_only_vi_sentence`
  - `similar_with_zero_image_weight_is_axes`
  - `pair_candidates_filtered_respects_filter`: filter chỉ nhận 3 family → kết quả ⊆ 3 family đó.
- [ ] **Step 2–4:** fail → cài → chạy:
  - `cargo test -p fontatlas-index`
  - `cargo test -p fontatlas-index --release --test pair_golden --test similar -- --include-ignored`
  - Expected: PASS; golden không đổi.
- [ ] **Step 5: Commit** — `feat(index): storeys, magnitude percentile, face summary, similar options, filtered pair candidates`

---

### Task 8: Crate `fontatlas-describe` — từ điển mô tả thành bộ lọc

**Files:** Create `crates/fontatlas-describe/{Cargo.toml, src/lib.rs, data/descriptor-dictionary.json, tests/describe.rs}`; thêm vào members.

**Interfaces:**
- Consumes: `fontatlas_store::vi_fold`.
- Produces:

```rust
pub enum Strength { Required, Preferred }                       // serde snake_case
pub struct AxisFilter { pub axis: String, pub range: Option<(f32, f32)>, pub equals: Option<String>,
    pub strength: Strength, pub weight: f32, pub term_id: String }
pub struct MatchedTerm { pub id: String, pub text: String, pub group: String, pub subjectivity: String,
    pub culture: String, pub rationale_vi: String, pub rationale_en: String, pub explanation_banned: bool }
pub struct Conflict { pub axis: String, pub term_a: String, pub term_b: String, pub range_a: (f32, f32), pub range_b: (f32, f32) }
pub enum HardConstraint { ViSupported, ViComplete }   // "vi_status in [full, precomposed_only]" / "missing = 0"
pub struct Parsed { pub dictionary_version: String, pub matched: Vec<MatchedTerm>, pub filters: Vec<AxisFilter>,
    pub hard: Vec<HardConstraint>, pub conflicts: Vec<Conflict>, pub unmappable: Vec<(String, String)>,
    pub partial: Vec<(String, String)>, pub unknown_words: Vec<String> }
pub trait FaceView {
    fn percentile(&self, axis: &str) -> Option<f32>;   // None → bỏ qua filter (README §4.4)
    fn class(&self, axis: &str) -> Option<String>;
    fn vi_missing(&self) -> usize;
    fn supports_vi(&self) -> bool;
}
impl Dictionary { pub fn bundled() -> Dictionary; pub fn from_json(s: &str) -> Result<Dictionary, DescribeError>; pub fn parse(&self, text: &str) -> Parsed; }
pub fn evaluate(filters: &[AxisFilter], hard: &[HardConstraint], v: &dyn FaceView) -> Option<f32>;   // None = loại
```

Quy tắc (README từ điển §4):
- **Khớp cụm:** `vi_fold` cả truy vấn lẫn `terms.vi/en`; lấy cụm dài nhất khớp trọn từ, từ trái sang phải. Phần còn lại vào `unknown_words`.
- **Gộp hai range cùng trục:**
  - giao khác rỗng → thay bằng phần giao; `Required` thắng; `weight` lấy max;
  - giao rỗng → **cả hai rời `filters`**, thêm `Conflict` (app không tự chọn).
- **`evaluate`:**
  - `hard` hoặc `Required` trượt → `None`;
  - `Preferred` thoả thì `+weight`; ngoài range thì `−weight × khoảng cách`;
  - trục `None` thì bỏ qua.
- **Kiểm khi nạp:** `axis` hợp lệ; `0 ≤ lo < hi ≤ 1`; `Preferred` có `weight`; `equals` là chuỗi hoặc số, chuẩn về chuỗi.

- [ ] **Step 1: Chép từ điển.** `cp "<checkout gốc>/docs/team/research/10_descriptor_dictionary.json" crates/fontatlas-describe/data/descriptor-dictionary.json`. Ghi `0.1.0-draft` vào commit message.
- [ ] **Step 2: Test thất bại**
  - `bundled_dictionary_valid`: 58 mục.
  - `warm_maps_to_aperture_terminal_contrast_stress`
  - `diacritic_free_input_matches`: "am ap" khớp `warm`.
  - `longest_phrase_wins`: "tối giản Hàn" khớp `korean_minimal`.
  - `luxury_vs_casual_conflict_reported`: "sang bình dân" có conflict trên `x_height_em` và `contrast`; `filters` không còn hai trục này.
  - `unmappable_reason_returned`: "đương đại".
  - `required_beats_preferred_and_null_axis_skipped`
  - `vietnamese_ready_hard_constraint`
- [ ] **Step 3–5:** fail → cài → `cargo test -p fontatlas-describe && cargo clippy -p fontatlas-describe --all-targets -- -D warnings` PASS.
- [ ] **Step 6: Commit** — `feat(describe): descriptor dictionary to visible axis filters, conflicts, evaluation`

---

### Task 9: Embed — tải model tiếp được (HTTP Range), tốc độ, kiểm dung lượng

`ModelStore::ensure` hiện xoá `.part` khi bắt đầu và khi lỗi hoặc huỷ (UX 03 §5: "[Thiếu BE — P1]").

**Files:** Modify `crates/fontatlas-embed/src/model.rs`, `src/lib.rs`. Test `tests/model.rs` (thêm; dùng server HTTP giả sẵn có trong test Plan 4).

**Interfaces — Produces:**
- `ModelStatus::Partial` thêm dữ liệu: `Partial { bytes: u64, total: u64 }`.
- `pub fn ensure_resumable(&self, progress: &dyn Fn(DownloadProgress), cancel: &AtomicBool) -> Result<PathBuf, EmbedError>`
  - `pub struct DownloadProgress { pub done: u64, pub total: u64, pub bytes_per_sec: f64, pub resumed: bool }`;
  - giữ `.part` cùng `.part.json` (`{manifest_sha256, segments_done, bytes}`) khi huỷ hoặc lỗi mạng;
  - lần sau tiếp từ đoạn và byte đã ghi (fsync mỗi 8 MiB);
  - `manifest_sha256` khác (manifest mới) thì bắt đầu lại;
  - checksum cuối sai → xoá `.part`, trả `ChecksumMismatch`;
  - gửi `Range` mà server trả 200 (bỏ qua Range) → cắt `.part` về 0, tải lại từ đầu, `resumed = false` trong mọi `DownloadProgress` của lần đó (`true` khi tiếp được bằng 206). Không coi là lỗi.
- `ensure` cũ = `ensure_resumable`.
- `ModelStore::dir(&self) -> &Path`, `ModelStore::expected_size(&self) -> u64`.
- `pub fn disk_free(path: &Path) -> std::io::Result<u64>` = `fs2::available_space(path)`. Dependency `fs2 = "=0.4.3"` thêm vào `crates/fontatlas-embed/Cargo.toml` (một lời gọi cho unix và Windows; Windows chưa kiểm).

- [ ] **Step 1: Test thất bại**
  - `cancel_keeps_part_and_resumes`: huỷ ở 40% → `status() == Partial { bytes ≈ 40% }`; gọi lại → server giả ghi nhận request đầu có `Range: bytes=<đúng offset>`; kết quả `Ready`; sha256 khớp.
  - `manifest_change_restarts`
  - `checksum_mismatch_reported`: server trả byte sai → `ChecksumMismatch`, không còn `.part`.
  - `server_ignoring_range_restarts_with_resumed_false`: server giả luôn trả 200 → kết quả `Ready`, sha256 khớp, mọi `DownloadProgress` của lần tiếp có `resumed == false`, `.part` không vượt `total`.
  - `progress_reports_speed`: `bytes_per_sec > 0`.
  - `disk_free_positive`
- [ ] **Step 2–4:** fail → cài → `cargo test -p fontatlas-embed` PASS.
- [ ] **Step 5: Commit** — `feat(embed): resumable model download with Range, speed, disk free`

---

### Task 10: Index — Similar chỉ dùng ảnh khi font gốc không so được theo trục

Lead quyết 2026-11-05: font chỉ có chữ hoa hoặc không phải Latin (`NotComparable`) vẫn cần Similar. Khi font gốc có embedding, kết quả lấy theo ảnh, và mọi hit ghi rõ nguồn "Hình ảnh".

**Files:** Modify `crates/fontatlas-index/src/{similar,index}.rs`, `src/lib.rs`. Test thêm vào `tests/similar.rs`.

**Interfaces — Produces:**
- `SimilarSource::Image` (serde `"image"`).
- `Index::similar_image_only(&self, q: &FaceKey, k: usize, filter: &dyn Fn(&FaceProfile) -> bool) -> Result<Vec<SimilarHit>, IndexError>`
  - xếp theo `1 − cos` (int8, rồi xếp lại f32 khi có `exact`, như `similar`);
  - chỉ face có ảnh; bỏ chính nó;
  - `same`/`differ` rỗng;
  - font gốc không có ảnh → `NotComparable`.
- `Index::similar_with` không đổi. App (Task 15) gọi `similar_image_only` khi `similar_with` trả `NotComparable` và `has_image(q)`.

- [ ] **Step 1: Test thất bại**
  - `image_only_for_non_comparable_query`: profile mọi trục Failed nhưng có embedding → mọi hit `source == Image`, xếp theo cos giảm dần.
  - `image_only_without_embedding_is_not_comparable`
  - `image_only_excludes_self_and_faces_without_image`
- [ ] **Step 2–4:** fail → cài → chạy:
  - `cargo test -p fontatlas-index`
  - `cargo test -p fontatlas-index --release --test similar --test pair_golden -- --include-ignored`
  - Expected: PASS; golden không đổi.
- [ ] **Step 5: Commit** — `feat(index): image-only similar for non-comparable queries`

---
### Task 11: Khung app mới — gỡ phần cũ, AppState, CmdError, ts-rs, cấu hình

**Files:**
- Delete như Global Constraints.
- Rewrite `src/lib.rs`.
- Create `src/{error,state,hub,commands,bindings}.rs`, `src/dto/mod.rs`, `src/services/mod.rs`, `.cargo/config.toml`.
- Modify `Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`.
- Test `tests/config.rs`, unit test trong `error.rs`.

**Interfaces — Produces:**

```rust
pub struct CmdError { pub code: String, pub message: String, pub detail: Option<serde_json::Value> }   // TS, camelCase
impl From<StoreError|IndexError|EmbedError|PreviewError> for CmdError;  // NestedSource → detail {existingId}; service add_source tra thêm existingPath (Task 13)
pub struct AppDirs { pub data: PathBuf, pub cache: PathBuf, pub models: PathBuf }   // debug build: FONTATLAS_DATA_DIR ghi đè data/cache/models
pub struct AppState {
    pub dirs: AppDirs,
    pub store: OnceLock<Mutex<Store>>,            // đặt một lần khi Store::open thành công; đọc thư viện; service Plan 7 nhận &Mutex<Store>
    pub user: OnceLock<Mutex<Store>>,             // Store::open_user_only; mọi ghi dữ liệu người dùng
    pub boot: RwLock<BootStatus>,
    pub index: RwLock<Option<Arc<IndexBundle>>>,
    pub hubs: Hubs,
    pub scans: ScanManager,
    pub embed: EmbedSupervisor,
    pub activation: Box<dyn ActivationApi>,       // Task 17; trước đó là bản trả activation_unsupported
    pub preview: OnceLock<Arc<PreviewService<StoreLocator>>>,
    pub dictionary: fontatlas_describe::Dictionary,
}
impl AppState {
    pub fn new(dirs: AppDirs, activation: Box<dyn ActivationApi>, encoder: EncoderFactory) -> Arc<AppState>;
    pub fn store(&self) -> Result<&Mutex<Store>, CmdError>;                                                  // chưa mở → store_unavailable
    pub fn with_store<T>(&self, f: impl FnOnce(&mut Store) -> Result<T, CmdError>) -> Result<T, CmdError>;  // khoá ngắn quanh f
    pub fn with_user<T>(&self, f: impl FnOnce(&mut Store) -> Result<T, CmdError>) -> Result<T, CmdError>;   // cùng kiểu, trên kết nối user-only
}
// commands.rs: macro commands!{ [async] name(args) -> Result; … } sinh cả generate_handler! lẫn COMMANDS: &[CommandSig]
pub struct CommandSig { pub name: &'static str, pub args: &'static [(&'static str, &'static str)], pub result: &'static str, pub is_async: bool }
pub const SYNC_OK: &[&str] = &["startup_status", "scan_status", "preview_info", "activation_caps"];
```

Các trường `scans`, `embed`, `activation`, `preview`, `index`, `hubs` dùng kiểu tạm có `Default` ở task này; các task sau thay bằng kiểu thật.

- [ ] **Step 1: Test thất bại**
  - `csp_has_required_sources`
  - `no_external_bin_and_no_sidecar`
  - `capabilities_minimal`: tập quyền đúng bằng {`core:default`, `core:window:allow-start-dragging`, `dialog:allow-open`}.
  - `cmd_error_from_nested_source`: `detail.existingId` có giá trị.
  - `commands_touching_store_are_async`: mọi phần tử `COMMANDS` ngoài `SYNC_OK` có `is_async == true`; thêm quét văn bản `src/commands.rs`, không còn `#[tauri::command]` trần (không `async`) cho lệnh ngoài `SYNC_OK`.
  - `window_chrome_settings_preserved`: `tauri.conf.json` có `app.macOSPrivateApi == true`, cửa sổ chính `transparent == true` và có `windowEffects`; `Cargo.toml` của app có feature `macos-private-api` của `tauri`.
  - `error_io_code_carries_source`: `StoreError::Io` → `code == "io"`, `detail.source == "store"`.
- [ ] **Step 2: Chạy, xác nhận fail.**
  - Run: `cd UX/src-tauri && cargo test -p app --test config`
- [ ] **Step 3: Cài đặt.**
  - Cargo app: `edition = "2024"`, `rust-version = "1.85"`; dep path tới các crate `fontatlas-*` (gồm `fontatlas-activate` khi Plan 7 đã gộp; trước đó `ActivationApi` là trait cục bộ có bản trả `activation_unsupported`); `ts-rs = { version = "=12.0.1", features = ["serde-json-impl"] }`; `tauri-plugin-opener = "2"`.
  - Dev-dep: `tauri` với feature `test`, `tempfile = "3.27"`.
  - `tauri.conf.json`: chỉ đổi `app.security.csp` và bỏ `bundle.externalBin`; giữ nguyên `macOSPrivateApi`, `transparent`, `windowEffects`. `tauri` giữ feature `macos-private-api`.
  - `tauri-plugin-log` đăng ký ở Rust; không thêm quyền `log:*`; không lệnh log cho frontend.
  - `.cargo/config.toml`: `[env] TS_RS_EXPORT_DIR = { value = "../bindings", relative = true }`.
- [ ] **Step 4: Chạy.**
  - Run: `cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && (cd .. && npm run build)`
  - Expected: PASS.
- [ ] **Step 5: Commit** — `refactor(app): remove Python sidecar, Qdrant and legacy commands; new app skeleton, CSP, minimal capabilities`

---

### Task 12: Khởi động, onboarding, di chuyển DB cũ

**Files:** Create `src/services/{boot,onboarding}.rs`, `src/dto/settings.rs` (phần boot/onboarding). Modify `src/commands.rs`, `src/lib.rs`. Test `tests/boot.rs`.

**Interfaces:**
- Consumes:
  - `Store::open`, `OpenReport`, `legacy_preview`, `import_legacy`, `resolve_legacy`, `LEGACY_DB_FILE`;
  - các điểm móc của task sau (trả `Ok` ở task này): `ScanManager::enqueue_all_directories`, `rebuild_index`, `services::activation::reconcile_on_boot` (Task 17), `EmbedSupervisor::ensure_started`.
- Produces:
  - `pub fn start(state: Arc<AppState>)` (luồng `fontatlas-boot`);
  - `pub fn run_boot(state: &Arc<AppState>) -> BootStatus` (đồng bộ, để test).
  - `BootStatusDto { phase: "starting"|"scanning"|"indexing"|"activation"|"ready"|"failed", storeError: Option<CmdError>, libraryRecreated, userBackup: Option<String>, legacy: LegacyStateDto }`
  - `LegacyStateDto { found: bool, preview: Option<LegacyPreview>, imported: Option<LegacyImport>, resolved: Option<LegacyResolve>, error: Option<CmdError> }`
  - `OnboardingDto { step: Option<String>, hasSources: bool, legacy: LegacyStateDto, licenseNoticeAck: bool, embedEnabled: bool, modelState: String, activationSupported: bool, activationOnboarded: bool, keepActiveOnExit: bool, fontManagers: Vec<FontManagerDto> }`. Trong đó:
    - `activationSupported` = `state.activation.supported()`;
    - `activationOnboarded` / `keepActiveOnExit` đọc `SETTING_ACTIVATION_ONBOARDED` / `SETTING_ACTIVATION_KEEP` của Plan 7 (`"1"` = true);
    - `fontManagers` từ `system::detect_font_managers` (Task 14).
  - Lệnh:
    - `startup_status() -> BootStatusDto`
    - `boot_subscribe(onEvent: Channel<BootEvent>)`
    - `onboarding_state() -> OnboardingDto`
    - `set_onboarding_step(step: Option<String>)` (khoá setting `onboarding_step`; `None` = xong)
    - `legacy_import() -> LegacyStateDto`: nhập rồi enqueue quét; `resolve_legacy` chạy ở `on_idle`.
    - `legacy_retry() -> LegacyStateDto` (cùng hàm, dùng sau lỗi)

Hành vi:
- `Store::open` lỗi → `failed` + mã; mọi lệnh cần Store trả `store_unavailable`; không ghi thêm.
- Có `fontatlas.db` và `setting("legacy_import")` là `None` → `legacy.found = true`, có `preview`. **Không tự nhập**: người dùng chọn ở O2 (UX 03 §8.6).
- Nhập lỗi → `legacy.error`, không đặt `legacy_import`.
- Quét lại các nguồn lúc mở app khi setting `rescan_on_launch` khác `"false"` (mặc định bật; Task 16 có DTO) **hoặc** setting `analyzer_version_seen` khác `ANALYZER_VERSION`. Đổi version tự kích hoạt phân tích lại, bất kể `rescan_on_launch`.
  - `analyzer_version_seen` được ghi khi quét toàn bộ về `Idle` mà `stale_analysis_count() == 0`.
  - Tắt quét thì vẫn dựng Index từ bản phân tích mới nhất sẵn có (kể cả version cũ).

- [ ] **Step 1: Test thất bại**
  - `fresh_boot_reaches_ready`
  - `legacy_detected_not_imported_until_command`: có preview; collection chưa có; gọi `legacy_import` → có.
  - `corrupt_legacy_db_is_reported_and_untouched`: 4 KB rác → `preview` là `None`, `error.code == "legacy"`; sha256 không đổi; boot vẫn `ready`.
  - `newer_user_db_fails_without_writing`: `user_version = USER_VERSION + 1` → `failed`, mã `newer_schema`; byte `user.db` không đổi; `backups/` không có file mới.
  - `onboarding_step_roundtrip`
  - `analyzer_version_change_forces_rescan_even_if_rescan_on_launch_false`: `analyzer_version_seen` cũ, `rescan_on_launch=false` → boot enqueue quét mọi nguồn thư mục; sau `Idle`, `analyzer_version_seen == ANALYZER_VERSION`.
  - `index_built_from_stale_analyses`: DB có dòng `analyzer_version` cũ → Index vẫn có đủ face lúc `ready`.
- [ ] **Step 2–4:** fail → cài → `cargo test -p app --test boot` PASS.
- [ ] **Step 5: Commit** — `feat(app): boot sequence, onboarding state, user-driven legacy import`

---

### Task 13: Nguồn font, quản lý quét (tạm dừng/tiếp tục/huỷ), ProgressHub

**Files:** Create `src/hub.rs`, `src/services/{sources,scan}.rs`, `src/dto/{library,events}.rs`. Modify `src/commands.rs`, `src/state.rs`. Test `tests/scan.rs`.

**Interfaces — Produces:**

```rust
pub struct ProgressHub<E: Clone + Send> { /* … */ }
impl<E> ProgressHub<E> { pub fn subscribe(&self, sink: Box<dyn Fn(E) + Send + Sync>) -> u64; pub fn unsubscribe(&self, id: u64); pub fn emit(&self, e: E); }
pub struct Hubs { pub boot: ProgressHub<BootEvent>, pub scan: ProgressHub<ScanEvent>, pub embed: ProgressHub<EmbedEvent>,
                  pub model: ProgressHub<ModelEvent>, pub activation: ProgressHub<ActivationEvent> }
pub enum ScanEvent {   // tag "type"
    Queued { source_id: i64 }, Progress { source_id: i64, progress: ScanProgress },
    FacesAdded { source_id: i64, count: usize },          // sau mỗi lô có faces_written > 0
    Paused, Resumed,
    Finished { source_id: i64, stats: RescanStats }, Failed { source_id: i64, error: CmdError }, Idle,
}
impl ScanManager {
    pub fn enqueue(&self, state: &Arc<AppState>, ids: &[i64]);     // bỏ id đang chạy hoặc đã trong hàng
    pub fn enqueue_all_directories(&self, state: &Arc<AppState>) -> Result<(), CmdError>;
    pub fn pause(&self); pub fn resume(&self); pub fn cancel_current(&self);
    pub fn shutdown(&self);                                         // cancel + join ≤ 5 s
    pub fn wait_idle(&self, timeout: Duration) -> bool;
    pub fn on_idle(&self, f: Box<dyn Fn() + Send + Sync>);
    pub fn status(&self) -> ScanStatusDto;                          // {running, paused, currentSourceId, queued: Vec<i64>}
}
pub struct SourceDto { pub id: i64, pub kind: String, pub name: String, pub path: Option<String>, pub enabled: bool,
    pub available: bool, pub face_count: u32, pub last_scan: Option<RescanStats>, pub scanning: bool }
```

Lệnh:
- `list_sources`
- `add_source(path) -> SourceDto` (enqueue sau khi thêm); lỗi `nested_source` kèm `detail {existingId, existingPath}`, với `existingPath` tra thêm từ nguồn `existingId`
- `remove_source(sourceId)`: `remove_source` → bump preview → dựng lại Index → điểm móc `reconcile` (Task 17). Font kích hoạt mà file biến mất do Plan 7 xử lý (`missing_file`, giữ mong muốn); 6a không tự gỡ kích hoạt.
- `rescan_sources(sourceIds: Option<Vec<i64>>)`
- `scan_pause`, `scan_resume`, `scan_cancel`
- `scan_status() -> ScanStatusDto`
- `scan_subscribe(onEvent)`

Tạm dừng: callback tiến độ của Task 2 chặn khi `paused` (park có timeout 100 ms, vẫn kiểm `cancel`). Callback chạy ngoài transaction nên không giữ khoá ghi khi chờ. Mọi ghi dữ liệu người dùng của task này (nguồn, yêu thích) đi qua `with_user`.

- [ ] **Step 1: Test thất bại**
  - `add_and_scan_reports_progress_then_finished` (có `FacesAdded`)
  - `nested_source_error_has_existing_id_and_path`
  - `duplicate_rescan_is_single_flight`
  - `two_subscribers_get_same_events`
  - `unavailable_source_reported`
  - `favorite_during_scan_latency_under_200ms`: quét 600 file; trong lúc quét gọi `set_favorite` mỗi 20 ms qua lệnh; từng lần < 200 ms, không lỗi.
  - `pause_then_resume_completes`: pause ở lô đầu → 300 ms không có `Progress` mới → resume → `Finished`.
- [ ] **Step 2–4:** fail → cài → `cargo test -p app --test scan` PASS.
- [ ] **Step 5: Commit** — `feat(app): sources, single-flight scan manager with pause/resume, progress hub over Channel`

---

### Task 14: Lệnh thư viện và hệ thống

**Files:** Create `src/services/{library,system}.rs`, `src/dto/{face,library}.rs`. Modify `src/commands.rs`. Test `tests/library.rs`, `tests/ipc.rs`.

**Interfaces:**
- Consumes: `query_faces_scoped` (Task 4), `IndexBundle::scope_for` (Task 15; ở đây trả `None` khi `axisFilters` rỗng), `face_path`, `backup_user_db`, `disk_free`.
- Produces — DTO:
  - `FaceKeyDto`, kèm `TryFrom` sang `fontatlas_store::FaceKey` (sai hex → `invalid_arg`).
  - `FaceRowDto`, `FacePageDto`: đúng 19 khoá.
  - `FaceQueryDto` = mọi trường `FaceQuery` + `axisFilters: Vec<AxisFilterDto>` + `faceKeys: Option<Vec<FaceKeyDto>>` + `sort` thêm `"match"`.
    - `faceKeys` (≤ 2000, quá thì `invalid_arg`) dùng cho "Gần đây" / "Đang bật": app tra `Store::face_ids_of`, tạo scope điểm giảm dần theo thứ tự đưa vào, giao với scope của `axisFilters`; `sort = "match"` giữ nguyên thứ tự đưa vào.
  - `AxisFilterDto { axis, range: Option<[f32; 2]>, equals: Option<String>, strength: "required"|"preferred", weight: f32 }`
- Produces — Lệnh:
  - `query_faces(q) -> FacePageDto`
  - `set_favorite(face, on)`
  - `list_collections`, `create_collection(name, color)`, `rename_collection(id, name)`, `delete_collection(id)`, `add_to_collection(id, faces)`, `remove_from_collection(id, faces)`
    - `delete_collection` gọi điểm móc `services::activation::before_delete_collection(id)` **trước** `Store::delete_collection`: Task 17 nối vào `deactivate_collection`; trước Plan 7 trả `Ok`.
  - `list_tags`, `tags_of(face)`, `create_tag(name)`, `tag_faces(tagId, faces)`, `untag_faces(tagId, faces)`
  - `list_problems() -> Vec<ProblemDto>`, `retry_problems() -> u32`
  - `get_family_styles(family: String) -> Vec<FamilyStyleDto>`
    - `FamilyStyleDto { face: FaceKeyDto, subfamily: Option<String>, weight: Option<u16>, italic: bool, instance: Option<{ name, coords: Vec<{tag, value}> }> }`;
    - face tĩnh: một dòng, `instance = None`;
    - VF: một dòng cho mỗi named instance, `weight` = toạ độ `wght` làm tròn, `italic` = `ital == 1` hoặc tên chứa "Italic".
  - `reveal_in_finder(face)`: đường dẫn lấy từ `face_path`, gọi `tauri_plugin_opener::reveal_item_in_dir`; không có path → `not_found`.
  - `backup_user_db() -> String`
  - `disk_free(target: "models"|"data") -> u64` (frontend không truyền path tuỳ ý)
  - `detect_font_managers() -> Vec<FontManagerDto>`
    - `FontManagerDto { name, installed: bool, activeCount: Option<u32> }`;
    - bảng cố định RightFont, FontBase, Typeface, Suitcase Fusion, FontExplorer X; kiểm bundle trong `/Applications` và `~/Applications`;
    - `activeCount = None` cho tới khi Plan 7 cung cấp số đăng ký CoreText (UX 03 O7).
  - `get_ui_state() -> UiStateDto`, `set_ui_state(state: UiStateDto)`
    - `UiStateDto { sidebarCollapsed: bool, openSections: Vec<String>, layout: String, sampleSize: u32, infoExpanded: bool }`, serde `default`;
    - lưu JSON dưới khoá setting `ui.state`, tối đa 64 KB, quá thì `invalid_arg`;
    - không nâng user.db.
  - `get_setting(key) -> Option<String>`, `set_setting(key, value)`: chỉ khoá khớp `^ui\.[a-z0-9_.-]{1,60}$`; giá trị ≤ 64 KB (65 536 byte UTF-8). Sai khoá hoặc quá cỡ → `invalid_arg`. Khoá hệ thống (`embed_enabled`…) chỉ đổi qua `set_settings` có kiểm.
  - Mọi lệnh ghi của task này (yêu thích, collection, tag, setting) đi qua `with_user`.
  - `rebuild_library` **không có** trong 6a (hoãn); đổi `ANALYZER_VERSION` đã tự phân tích lại.

- [ ] **Step 1: Test thất bại**
  - `face_row_json_matches_store_fixture`: tập khoá bằng tập khoá trong `face_row.json`.
  - `favorites_collections_tags_roundtrip`
  - `bad_hash_is_invalid_arg`
  - `retry_problems_requeues`
  - `family_styles_static_and_variable`: Roboto → 1 dòng; Public Sans VF → ≥ 9 dòng, có dòng `weight == 700`.
  - `ui_state_roundtrip_and_size_cap` (trần 64 KB)
  - `get_set_setting_only_ui_keys_and_64k_cap`
  - `face_keys_filter_keeps_given_order`: 5 khoá đưa vào theo thứ tự đảo → `sort = "match"` trả đúng thứ tự đó; khoá không tồn tại bị bỏ.
  - `detect_font_managers_lists_known_names`: 5 tên, `activeCount` là `None`.
  - `reveal_missing_path_is_not_found`
  - `tests/ipc.rs` `ipc_query_faces_uses_camel_case_args` (`tauri::test::mock_builder`, `get_ipc_response`)
- [ ] **Step 2–4:** fail → cài → `cargo test -p app --test library --test ipc` PASS.
- [ ] **Step 5: Commit** — `feat(app): library, family styles, reveal in Finder, backup, disk free, UI state commands`

---

### Task 15: Index trong app — chi tiết face, Similar, Pairing, cặp đã lưu, tìm bằng mô tả

**Files:** Create `src/index_bundle.rs`, `src/services/{analysis,describe,pairs}.rs`, `src/dto/analysis.rs`. Modify `src/commands.rs`, `src/services/{boot,scan}.rs` (`rebuild_index` khi `Idle`). Test `tests/analysis.rs`.

**Interfaces:**
- Consumes: Task 3, 5, 6, 7, 8, 10; `StoreEmbeddingSource` (Plan 4).
- Produces — bundle:

```rust
pub struct FaceMeta { pub face_id: i64, pub source_ids: Vec<i64>, pub missing: bool, pub container: Option<String>, pub file_size: u64 }
pub struct IndexBundle { pub index: Index, pub meta: HashMap<fontatlas_index::FaceKey, FaceMeta>, pub model_version: Option<String> }
impl IndexBundle {
    /// `store` là kết nối từ `Store::open_reader` (Task 1), không bao giờ là `AppState.store`. Dùng `for_each_latest_analysis`.
    pub fn build(store: &Store, model_version: Option<&str>) -> Result<IndexBundle, CmdError>;
    pub fn scope_for(&self, filters: &[AxisFilterDto], hard: &[HardConstraint]) -> Option<Vec<(i64, f32)>>;  // None = không lọc
}
pub fn rebuild_index(state: &Arc<AppState>);   // mở reader riêng, dựng ngoài mọi khoá, đổi con trỏ; lỗi giữ bundle cũ
pub enum RebuildWhy { ScanIdle, QueueEmpty, EmbedProgress }
pub struct RebuildPolicy { pub min_interval: Duration }   // mặc định 30 s; test dùng 200 ms
/// ScanIdle, QueueEmpty: dựng ngay. EmbedProgress (mỗi 512 face): chỉ dựng nếu ≥ min_interval kể từ lần dựng trước.
/// Một lần dựng tại một thời điểm; yêu cầu đến khi đang dựng chỉ đặt cờ dirty, chạy lại đúng một lần.
pub fn request_rebuild(state: &Arc<AppState>, why: RebuildWhy);
```

`FaceView` của bundle:
- `percentile(axis)` theo `reference.percentile` với method của reading;
- `stress_deg`/`slant_deg` dùng `magnitude_percentile`; `terminal` trả `None`;
- `class("serif_foot")` → `sans`/`serif`/`uncertain`; `class("a_storeys"|"g_storeys")` → `"1"`/`"2"`.

- Produces — Lệnh (không có face → `not_found`; Index chưa dựng → `busy`):
  - `get_face(face, lang) -> FaceDetailDto`. Gồm:
    - `row`, `summary` (`summarize_face`), `source: "axes"|"axes_and_image"`, `hasImage`;
    - `file: { format: "ttf"|"otf"|"ttc"|"otc"|"woff"|"woff2", container, outline: "glyf"|"cff"|"cff2"|"none", numGlyphs, sizeBytes, path }`;
    - `metadata: { license, licenseUrl, designer, manufacturer, version, copyright, vendorUrl, variableAxes: [{tag, min, default, max}], namedInstances: [{name, coords}], scripts, opszRange, features: {gsub, gpos} }`;
    - `vietnamese: { status, supports, missing: Vec<String>, precomposedComposite, gposMark, gposMkmk, gsubCcmp, dongSign, stackedMarksTopEm, stackedMarksOverflow, stackStyle, markWeightRatio: AxisReadingDto }`;
    - `axes: Vec<AxisReadingDto>` (16 + terminal + aStoreys + gStoreys);
    - `similarTop` (k = 3).
  - `AxisReadingDto { key, value, quality: "measured"|"estimated"|"failed", reason, method, percentile }`
  - `similar(face, opts) -> Vec<SimilarHitDto>`
    - `SimilarOptsDto { weights: {proportion, weight, structure, detail}, useImage: bool, k ≤ 100, viOnly: bool, sourceIds: Vec<i64>, weightRange: Option<[u16; 2]> }`;
    - bỏ face `missing`;
    - `similar_with` trả `NotComparable` mà font gốc có ảnh (và `useImage`) → gọi `similar_image_only` (Task 10); mọi hit `source = "image"`. Không có ảnh → `not_comparable`;
    - `SimilarHitDto { face, row, distance, source: "axes"|"axes_and_image"|"image", same: Vec<String>, differ: Vec<String> }`.
  - `pair_candidates(face, opts) -> Vec<PairCandidateDto>`
    - `PairOptsDto { role: "heading"|"body"|"code", mode: Option<"safe"|"controlled"|"experimental">, projectVi: bool, k ≤ 100, collectionId: Option<i64>, lang }`;
    - ứng viên mang vai trò còn lại;
    - `collectionId` → chỉ ứng viên trong collection (dùng `pair_candidates_filtered`).
  - `assess_pair(a, b, opts) -> PairCandidateDto`
  - `PairCandidateDto`:
    - `face`, `row`, `gate`;
    - `labels: [{axis, relation, symbol, delta, a, b, roleReversed}]`, với `symbol`: ● hoà hợp, ◐ tương phản / tương phản có serif, ◆ va chạm, ○ chưa chắc, ◑ gợi ý độ đậm;
    - `clashes`, `bodyWarnings`, `headingOverflow`, `insufficientData`, `weightHint`, `block`, `sentences`, `summary`;
    - **không trường điểm số nào.**
  - `save_pair(pair: NewPairDto) -> i64`, `list_pairs(collectionId: Option<i64>) -> Vec<SavedPairDto>`, `delete_pair(id)`
  - `describe(text) -> DescribeDto`: 1-1 với `Parsed`; `filters` là `AxisFilterDto` để gửi thẳng vào `query_faces.axisFilters`.
- Preset bố cục (Menu, Poster…) là frontend (6b); backend chỉ lưu tên `preset` của cặp.

- [ ] **Step 1: Test thất bại** (thư viện 6 fixture đã quét, đã `rebuild_index`)
  - `get_face_playfair`: summary bắt đầu "Serif, tương phản cao"; `file.format == "ttf"`; `numGlyphs > 0`; `copyright` có giá trị; 19 trục.
  - `get_face_libre_missing_list`: 68 ký tự, có "ơ".
  - `similar_works_without_model`
  - `similar_falls_back_to_image_for_caps_only`: face mọi trục Failed nhưng có embedding giả → hit `source == "image"`; tắt `useImage` → `not_comparable`.
  - `pair_candidates_mode_filter_and_no_score_field`
  - `pair_candidates_collection_scope`
  - `pair_project_vi_blocks_missing`
  - `saved_pairs_roundtrip`
  - `describe_then_query`: "tương phản cao" → `query_faces(sort = "match")` → Playfair đứng đầu.
  - `index_rebuilt_after_scan`
  - `index_build_does_not_hold_main_store_lock`: giữ khoá `state.store` ở luồng khác 1 s; `rebuild_index` vẫn xong trong thời gian đó.
  - `rebuild_after_embed_is_debounced`: policy 200 ms, 10 yêu cầu `EmbedProgress` dồn trong 100 ms → số lần dựng ≤ 2; `QueueEmpty` dựng ngay.
  - `index_uses_stale_version_analyses`: dòng `analyzer_version` cũ vẫn có trong Index.
- [ ] **Step 2–4:** fail → cài → `cargo test -p app --test analysis` PASS.
- [ ] **Step 5: Commit** — `feat(app): face detail, similar, pairing (collection scope), saved pairs, describe-to-filters`

---

### Task 16: Cài đặt, model, hàng đợi embed

**Files:** Create `src/services/{settings,model,embed,lifecycle}.rs`, `src/dto/{settings,model}.rs`. Modify `src/commands.rs`, `src/lib.rs` (`RunEvent::Exit` chỉ gọi `lifecycle::on_exit`), `src/services/boot.rs`. Test `tests/embed.rs`.

**Interfaces:**
- Consumes:
  - `ModelStore` + `ensure_resumable`, `disk_free`, `expected_size`, `dir` (Task 9);
  - `EmbedWorker`, `WorkerConfig`, `Progress`, `Whitening::bundled`, `CandleEncoder::{load_best, device_id}`, `raw_version`;
  - các hàm setting và embedding của Store.
- Produces:
  - `SettingsDto { embedEnabled, licenseNoticeAck, uiLang: "vi"|"en", sampleText, projectViDefault, keepActiveOnExit, rescanOnLaunch }`. `keepActiveOnExit` đọc từ khoá `activation_keep_after_quit` (`SETTING_ACTIVATION_KEEP` của Plan 7), không tạo khoá riêng.
    - mặc định: `true`, `false`, `"vi"`, `DEFAULT_SAMPLE`, `true`, `false`, `true`.
  - `get_settings`, `set_settings(patch: SettingsPatchDto)`:
    - `sampleText` cắt còn 64 ký tự sau NFC trim;
    - `uiLang` sai → `invalid_arg`;
    - `embedEnabled` false → `stop()`, true → `ensure_started()`;
    - `keepActiveOnExit` → cùng hàm với lệnh `activation_set_keep` (Task 17): ghi `SETTING_ACTIVATION_KEEP`, gọi `set_scope(None, …)`, đặt `SETTING_ACTIVATION_ONBOARDED = "1"`. Trước Plan 7 chỉ ghi setting.
  - `ModelStatusDto`:
    - `{ state: "missing"|"downloading"|"paused"|"ready"|"checksum_mismatch"|"corrupt"|"unpublished", doneBytes, totalBytes, bytesPerSec, etaMs: Option<u64>, location: String, diskFree: u64, requiredBytes: u64, lastError: Option<CmdError> }`;
    - `requiredBytes` = size × 1,1;
    - lỗi cuối (`checksum_mismatch`, `network`, `insufficient_disk`…) lưu vào setting `model_last_error` (JSON `{code, message, atMs}`) khi tải thất bại; xoá khi tải bắt đầu hoặc model `Ready`. Sau khi mở lại app, `state` suy ra từ file + `model_last_error` (không có file và `code == checksum_mismatch` → `checksum_mismatch`);
    - `paused` = có `.part` mà không đang tải;
    - `unpublished` khi URL manifest rỗng.
  - Lệnh model:
    - `model_status`
    - `model_download(onEvent: Channel<ModelEvent>)`: tải tiếp nếu có `.part`; thiếu chỗ thì `insufficient_disk` kèm `detail {required, free}`;
    - `model_cancel` (giữ `.part` → `paused`)
    - `model_redownload` (xoá rồi tải)
    - `model_remove` (dừng worker, xoá file, trạng thái `missing`)
    - `ModelEvent = Progress { done, total, bytesPerSec, etaMs, resumed } | Ready | Paused | Failed { error }` (`resumed:false` khi server bỏ qua Range và tải lại từ đầu; UI hiện thông báo)
  - `EmbedSupervisor`:
    - `ensure_started(&self, state) -> Result<bool, CmdError>`: trước khi mở thì join worker cũ → `reset_running_embeddings` → `enqueue_embeddings`;
    - `stop`, `pause`, `resume`, `shutdown` (≤ 5 s), `status`.
  - `EmbedStatusDto { enabled, running, paused, pending: u64, done: u64, failed: u64, device: Option<"metal"|"cpu">, msPerFace: Option<f32>, etaMs: Option<u64>, modelVersion: Option<String> }`
    - `msPerFace` = trung bình trượt 5 lô; trước lô đầu dùng 28 (Metal) / 30 (CPU) ms theo UX 03;
    - `etaMs = pending × msPerFace`.
  - Lệnh embed: `embed_status`, `embed_pause`, `embed_resume`, `embed_subscribe(onEvent: Channel<EmbedEvent>)`, `embed_failures() -> Vec<{face, code, detail}>` (đọc `pipeline_state` stage `embedding` failed).
  - Dựng lại Index sau embed đi qua `request_rebuild` (Task 15): `EmbedProgress` mỗi 512 face (chỉ chạy nếu ≥ 30 s từ lần dựng trước), `QueueEmpty` khi `Finished` (dựng ngay).
  - **Khởi động embed:** hook `on_idle` của `ScanManager` (Task 13) gọi `request_rebuild(ScanIdle)` rồi `enqueue_embeddings` + `ensure_started` (nếu `embedEnabled`). Khi `model_download` hoặc `model_redownload` chuyển sang `Ready`, gọi cùng cặp `enqueue_embeddings` + `ensure_started`.
  - `pause_all()` / `resume_all()`: lệnh `queue_pause_all`, `queue_resume_all` gọi cả quét lẫn embed (B4).
  - `services/lifecycle.rs`: `pub fn on_exit(state: &AppState)` = `scans.shutdown()`, `embed.shutdown()`, rồi `activation.shutdown()` (Task 17). `lib.rs` `RunEvent::Exit` chỉ gọi hàm này, nên test gọi trực tiếp, không cần webview.
  - `EncoderFactory: Box<dyn Fn(&Path) -> Result<(Box<dyn Encode>, &'static str /*device*/), EmbedError> + Send + Sync>`: app thật dùng Candle, test dùng encoder giả.

- [ ] **Step 1: Test thất bại** (encoder giả; model "ready" giả như test `ModelStore` Plan 4)
  - `worker_not_started_without_model`
  - `toggle_off_cancels_worker`
  - `restart_joins_previous_worker`
  - `shutdown_cancels_and_joins_workers` (qua `lifecycle::on_exit`, không webview, ≤ 5 s)
  - `embed_kicked_after_scan_idle`: sau `Idle` có job embedding `pending` và encoder giả nhận lô.
  - `embed_kicked_when_model_becomes_ready`: model giả chuyển `Ready` → `enqueue_embeddings` + `ensure_started` chạy một lần.
  - `model_last_error_survives_restart`: tải lỗi `checksum_mismatch`, dựng lại `AppState` → `model_status.lastError.code == "checksum_mismatch"`.
  - `restart_resumes_running_jobs`
  - `embed_status_eta_and_device`: sau một lô có `msPerFace`, `etaMs`, `device == "cpu"` (giả).
  - `queue_pause_all_pauses_scan_and_embed`
  - `model_download_insufficient_disk`: `disk_free` giả nhỏ hơn yêu cầu → `insufficient_disk`.
  - `settings_roundtrip_and_validation`
- [ ] **Step 2–4:** fail → cài → `cargo test -p app --test embed` PASS.
- [ ] **Step 5: Commit** — `feat(app): settings, resumable model download, embed queue status/ETA/device, pause-all`

---

### Task 17: Lớp lệnh kích hoạt (gọi service của Plan 7) + vòng đời

**Phụ thuộc Plan 7.** Plan 7 sở hữu:
- trait `ActivationEngine`, `FakeEngine`, `platform_engine()`;
- `ActivationService<E>`: `in_flight` → `FileOutcome::Busy`, dòng intent, lý do kích hoạt;
- `reconcile` với `ReconcileReport.auto_deactivated`, và `shutdown`.

Task này **không** cài logic kích hoạt nào. Nó chỉ:
- bọc service thành lệnh Tauri mỏng theo đúng bảng "Hợp đồng cho Plan 6a" của Plan 7 (`docs/plan7-activate` @ d8eec6f);
- nối vòng đời.

Chữ ký dưới đây chép từ Plan 7. Plan 7 đổi thì đổi theo, không tự chế.

**Chờ cập nhật (review mục 12):** Lead đã nhắn 06 thêm `pub fn set_scope`, `state()` / `ActivationStateDto` và test handle. Trait `ActivationApi` bên dưới là bản d8eec6f; **sửa lại sau khi Plan 7 xong Task 8**, đối chiếu export thực của `fontatlas-activate`. Trước đó không bắt đầu Task 17.

**Files:** Create `src/services/activation.rs`, `src/dto/activation.rs`. Modify `src/state.rs`, `src/services/{boot,library}.rs`, `src/lib.rs`, `src/commands.rs`. Test `tests/activation.rs`.

**Interfaces:**
- Consumes (Plan 7):
  - `fontatlas_activate::{ActivationService, ActivationEngine, FakeEngine, platform_engine, ConflictPolicy, DeactivatePolicy, ActivationReport, Progress, ReconcileReport, AutoDeactivated, ActivationStateDto, ActivationError, Scope}`;
  - các hàm `preflight`, `activate_faces`, `activate_collection`, `deactivate_faces`, `deactivate_collection`, `deactivate_all`, `set_scope`, `state`, `reconcile`, `shutdown`, `default_scope`, `engine().method()`. Mọi hàm nhận `store: &Mutex<Store>`.
  - `fontatlas_store::{SETTING_ACTIVATION_KEEP, SETTING_ACTIVATION_ONBOARDED}`.
- Produces:

```rust
/// Bọc object-safe để AppState giữ engine thật hoặc FakeEngine (test) mà không generic hoá AppState.
pub trait ActivationApi: Send + Sync {
    fn method(&self) -> &'static str;                 // "coretext" | "gdi" | "fake"
    fn supported(&self) -> bool;                      // method() != "gdi"
    fn preflight(&self, s: &Mutex<Store>, keys: &[FaceKey]) -> Result<ActivationReport, ActivationError>;
    fn activate_faces(&self, s: &Mutex<Store>, keys: &[FaceKey], scope: Option<Scope>, p: ConflictPolicy, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn activate_collection(&self, s: &Mutex<Store>, id: i64, scope: Option<Scope>, p: ConflictPolicy, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn deactivate_faces(&self, s: &Mutex<Store>, keys: &[FaceKey], p: DeactivatePolicy, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn deactivate_collection(&self, s: &Mutex<Store>, id: i64, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn deactivate_all(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ActivationReport, ActivationError>;
    fn set_scope(&self, s: &Mutex<Store>, hashes: Option<&[[u8; 32]]>, scope: Scope) -> Result<ActivationReport, ActivationError>;
    fn state(&self, s: &Mutex<Store>) -> Result<Vec<ActivationStateDto>, ActivationError>;
    fn reconcile(&self, s: &Mutex<Store>, progress: &mut dyn FnMut(Progress)) -> Result<ReconcileReport, ActivationError>;
    fn shutdown(&self, s: &Mutex<Store>) -> Result<usize, ActivationError>;
}
impl<E: ActivationEngine> ActivationApi for ActivationService<E> { /* chuyển tiếp từng hàm */ }
impl From<ActivationError> for CmdError;   // code = "activation_" + e.code(); Unsupported → "activation_unsupported"
```

- `AppState.activation: Box<dyn ActivationApi>`.
  - App thật: `Box::new(ActivationService::new(platform_engine()))`.
  - Test: `ActivationService::new(FakeEngine::new())`.
- `ActivationEvent = Progress(Progress) | Reconciled { report: ReconcileReport } | Changed { hashes: Vec<String> }` qua `Hubs.activation`.

Lệnh (tên theo Plan 7; mọi lệnh chạy trong `spawn_blocking`; tiến độ của lệnh dài qua `Channel<Progress>` truyền vào lệnh):
- `activation_caps() -> { method, supported }`
- `activation_state() -> Vec<ActivationStateDto>`
- `activation_preflight(faces) -> ActivationReport`
- `activation_activate(faces, scope: Option<Scope>, proceed: bool, onProgress) -> ActivationReport`
- `activation_activate_collection(collectionId, scope, proceed, onProgress) -> ActivationReport`
- `activation_deactivate(faces, force: bool, onProgress) -> ActivationReport` (`force=false` có thể trả `HeldByCollections`)
- `activation_deactivate_collection(collectionId, onProgress)`
- `activation_deactivate_all(onProgress)`
- `activation_set_keep(on: bool)`: `set_setting(SETTING_ACTIVATION_KEEP, "1"|"0")` + `set_scope(None, scope tương ứng)` + `set_setting(SETTING_ACTIVATION_ONBOARDED, "1")`
- `activation_subscribe(onEvent: Channel<ActivationEvent>)`

Vòng đời và nối dây:
- **Khởi động** (Task 12): sau quét đầu (hoặc ngay sau `Store::open` khi tắt quét lúc mở) gọi `reconcile`.
  - Phase `activation` của boot nằm **trước** `ready`. UI chỉ hiện trạng thái kích hoạt sau `BootEvent::Phase { phase: ready }`.
  - Gửi `ActivationEvent::Reconciled`; UI hiện thông báo từ `auto_deactivated`.
- **Sau mỗi `Idle` của quét:** `reconcile` lần nữa, để kích hoạt lại font có file trở lại ("kích hoạt chờ ổ" của Plan 7).
- **`delete_collection`** (Task 14) gọi `deactivate_collection(id)` **trước** `Store::delete_collection` (bảng hợp đồng Plan 7).
- **Thoát app:** `lifecycle::on_exit(&AppState)` (Task 16) gọi `activation.shutdown()` sau `scans.shutdown()` và `embed.shutdown()` (gỡ scope session). `RunEvent::Exit` chỉ gọi hàm này.
- `supported() == false` (Windows stub) → lệnh ghi trả `activation_unsupported`; `activation_caps`/`activation_state` vẫn chạy.

- [ ] **Step 1: Test thất bại** (`AppState` với `FakeEngine`; dữ liệu là fixture core đã quét)
  - `activate_collection_via_command_reports_progress`: Channel giả nhận ≥ 1 `Progress`; `activation_state` có dòng `state = active`.
  - `boot_runs_reconcile_before_ready`: thứ tự `BootEvent` là `… activation … ready`, và có `ActivationEvent::Reconciled`.
  - `auto_deactivated_pushed_to_ui`: kích hoạt collection, xoá collection **bằng SQL trực tiếp** (bỏ qua service) rồi boot lại → `Reconciled.report.auto_deactivated.len() == 1`.
  - `delete_collection_command_deactivates_first`: `delete_collection` qua lệnh → không còn lý do `collection:<id>`; `auto_deactivated` ở lần reconcile sau rỗng.
  - `unplugged_active_font_stays_desired`: kích hoạt, xoá thư mục nguồn, rescan → `state == active`, `status == missing_file`; trả thư mục, rescan → `status == registered`.
  - `exit_shuts_down_session_only`: một file session, một persistent → sau `on_exit` (gọi trực tiếp, không webview) chỉ file session bị gỡ trong `FakeEngine`.
  - `set_keep_switches_scope_and_marks_onboarded`
  - `gdi_engine_returns_unsupported`: `ActivationService::new(WindowsGdiEngine)` → `activation_activate` trả `activation_unsupported`.
- [ ] **Step 2–4:** fail → cài → `cargo test -p app --test activation` PASS.
- [ ] **Step 5: Commit** — `feat(app): activation commands over Plan 7 service, reconcile-before-ready, exit shutdown`

---
### Task 18: Scheme preview/font và lệnh preview

**Files:** Create `src/services/preview.rs`. Modify `src/lib.rs`, `src/commands.rs`. Test `tests/preview.rs`.

**Interfaces:**
- Consumes: `PreviewService`, `PreviewCache::{open, total_bytes, len}`, `Budgets::DEFAULT`, `handle_preview_uri`, `handle_font_uri`, `CorsPolicy`, `PREVIEW_SCHEME`, `FONT_SCHEME`, `RENDERER_VERSION`, `PX_BUCKETS` (Plan 5); `face_path` (Task 3).
- Produces:
  - `StoreLocator { store: Mutex<Store> }`, kèm `impl FaceLocator` (kết nối riêng).
  - `preview_service(state)`: cache ở `dirs.cache/preview.db`.
  - `cors()`: `CorsPolicy::app()`; debug build thêm `.with("http://localhost:3000")`.
  - `to_http(PreviewHttp) -> http::Response<Vec<u8>>`
  - Đăng ký hai scheme bằng `register_asynchronous_uri_scheme_protocol`:
    - đọc header `Origin`; chạy `handle_*_uri` trong `spawn_blocking`;
    - service chưa có (Store chưa mở) thì trả 503.
  - Lệnh:
    - `preview_info() -> { rendererVersion, generation, buckets }` (đúng `PreviewInfo` Plan 5)
    - `preview_bump_generation() -> u64`
    - `preview_prewarm(items)`
    - `preview_cache_info() -> { bytes, entries }`
    - `clear_preview_cache()`: thêm `PreviewCache::clear()` nếu crate chưa có, kèm test trong `fontatlas-preview`.

- [ ] **Step 1: Test thất bại**
  - `preview_png_with_cors`
  - `font_scheme_rejects_ttc_face_1` (422)
  - `preview_info_matches_plan5_shape`
  - `no_store_returns_503`
  - `clear_preview_cache_empties`
- [ ] **Step 2–4:** fail → cài → `cargo test -p app --test preview -p fontatlas-preview` PASS.
- [ ] **Step 5: Commit** — `feat(app): preview/font schemes with CORS, preview and cache commands`

---

### Task 19: Hợp đồng TypeScript — bindings, bản đồ lệnh, fixture, CI

**Lý do chọn ts-rs, không chọn specta:**
- `tauri-specta` vẫn là RC (2.0.0-rc.26 lúc viết plan); ghim RC vào lõi IPC là rủi ro.
- **DTO mirror trong app crate:** các crate lõi `fontatlas-*` không thêm dependency `ts-rs`. Mỗi kiểu lõi đi qua IPC có một DTO trong `src/dto/*` + `impl From<Core> for Dto`, không `#[derive(TS)]` trên kiểu lõi. Gồm: `ScanProgress`, `RescanStats`, `LegacyPreview`, `LegacyImport`, `LegacyResolve`, `ActivationReport`, `Progress`, `ReconcileReport`, `ActivationStateDto`, `Outcome`.
- ts-rs ghim `=12.0.1` (`cargo search ts-rs` 2026-11-05); chạy lại `cargo search ts-rs` khi thực thi, ghi kết quả vào ledger.
- Frontend Plan 5 đã dùng mẫu `TauriCommandMap`; ta sinh đúng mẫu đó.

**Files:** Modify `src/bindings.rs`, `src/commands.rs`, mọi DTO (`#[derive(ts_rs::TS)] #[ts(export)]`). Create `UX/bindings/` (sinh), `UX/bindings/fixtures/*.json`, `UX/bindings/README.md`, `UX/bindings/__tests__/contract.test.ts`. Modify `.github/workflows/ci.yml`.

**Interfaces — Produces:**
- Mỗi DTO, sự kiện, `CmdError`, `ScanProgress`, `RescanStats`, `Outcome`, `Caps`, `FontManager` một file `UX/bindings/<Type>.ts`.
- `UX/bindings/commands.ts`: `export interface TauriCommandMap { <lệnh>: { args: {...}; result: ... } }` cộng `export type CommandName`.
- Fixture JSON cho mỗi kết quả chính:
  - `face_page`, `face_detail_playfair`, `similar_hits`, `pair_candidates`, `saved_pairs`, `describe_warm`;
  - `settings`, `ui_state`, `sources`, `scan_events`, `boot_status`, `onboarding`;
  - `model_status_paused`, `embed_status`, `activation_state`, `activation_report`, `font_managers`, `family_styles_public_sans`, `cmd_error_nested_source`.

- [ ] **Step 1: Test thất bại**
  - `bindings_up_to_date`: `commands.ts` sinh ra phải giống file đã commit.
  - `every_command_in_handler_is_in_map`: cùng nguồn macro `commands!`.
  - `every_core_type_in_ipc_has_mirror`: danh sách tĩnh các kiểu lõi qua IPC trong `bindings.rs` đều có DTO + `From<>`.
  - `dto_serializes_like_core`: với kiểu lõi đã `Serialize`, JSON của lõi và của DTO giống nhau.
  - `fixtures_match_dtos`: mỗi fixture deserialize lại được thành DTO của nó.
  - `contract.test.ts` (Vitest): `parseFacePage(face_page.json)` của Plan 5 không ném lỗi; `tsc` chấp nhận `FaceRowDto` gán cho `FaceRow`.
- [ ] **Step 2: Chạy, xác nhận fail.**
  - Run: `cargo test -p app bindings && (cd .. && npx vitest run bindings)`
- [ ] **Step 3: Cài đặt** + `UX/bindings/README.md` (cách sinh lại; không sửa tay).
- [ ] **Step 4: CI** — job Rust thêm `cargo test -p app` và `git diff --exit-code UX/bindings`.
- [ ] **Step 5: Chạy** `cargo test -p app && (cd .. && npx tsc --noEmit && npx vitest run)`. Expected: PASS.
- [ ] **Step 6: Commit** — `feat(app): ts-rs bindings, generated TauriCommandMap, JSON fixtures, CI drift check`

---

### Task 20: Smoke build macOS và báo cáo

**Files:** Create `docs/team/plans/plan6a-report.md`, `UX/src-tauri/tests/smoke.sh`.

- [ ] **Step 1: Chạy toàn bộ.**
  - Run: `cd UX/src-tauri && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
  - Expected: PASS.
- [ ] **Step 2: Smoke** — `tests/smoke.sh`:
  - `npm ci && npm run build`;
  - `cargo tauri build --debug --bundles app`;
  - kiểm có `Font Atlas.app`;
  - `open -n` với `FONTATLAS_DATA_DIR` tạm, chờ 10 s, tiến trình còn sống;
  - thư mục tạm có `library.db` và `user.db`.
  - Expected: thoát 0.
- [ ] **Step 3: Thử tay 5 mục Review Focus trên bản debug.** Kích hoạt dùng `MacCoreTextEngine` thật (Plan 7 đã gộp). Chỉ thử kích hoạt/tắt khi chủ app đồng ý, dùng fixture OFL chép vào thư mục tạm, theo quy tắc test sống của Plan 7 (`FONTATLAS_LIVE_ACTIVATION=1`, không đụng font thật).
- [ ] **Step 4: `plan6a-report.md`** gồm:
  - danh sách commit, số test;
  - thời gian `query_faces` có lọc trục, `similar`, `pair_candidates` trên thư viện thật;
  - kết quả smoke và thử tay;
  - **"Windows: chưa kiểm"**;
  - mục "cho 6b" (dẫn bảng lệnh dưới);
  - mục "cho Plan 7": chỗ cắm engine, test cần chạy lại.
- [ ] **Step 5: Commit** — `docs(app): plan 6a report, macOS smoke build`

---

## Bảng lệnh cuối cùng (cho 6b)

Đối số camelCase. `face` = `FaceKeyDto`. Mọi lỗi là `CmdError`. Cột "Task" là nơi cài; cột "Thay cho" là lệnh cũ bị gỡ.

| Nhóm | Lệnh | Đối số → Kết quả | Task | Thay cho |
|---|---|---|---|---|
| Khởi động | `startup_status` | → `BootStatusDto` | 12 | |
| | `boot_subscribe` | `onEvent: Channel<BootEvent>` | 12 | |
| Onboarding | `onboarding_state` | → `OnboardingDto` | 12 | |
| | `set_onboarding_step` | `step: string \| null` | 12 | |
| | `legacy_import`, `legacy_retry` | → `LegacyStateDto` | 12 | |
| Nguồn | `list_sources` | → `SourceDto[]` | 13 | `get_sources` |
| | `add_source` | `path` → `SourceDto` \| `nested_source` | 13 | `create_source`, `scan_directory` |
| | `remove_source` | `sourceId` | 13 | `delete_source` |
| | `rescan_sources` | `sourceIds?: number[]` | 13 | `start_background_rescan`, `rescan_font` |
| | `scan_pause`, `scan_resume`, `scan_cancel`, `scan_status` | → `ScanStatusDto` | 13 | |
| | `scan_subscribe` | `onEvent: Channel<ScanEvent>` | 13 | |
| Thư viện | `query_faces` | `q: FaceQueryDto` (có `axisFilters`, `faceKeys`) → `FacePageDto` | 14, 15 | `get_library_fonts`, `get_fonts_by_source`, `search_library`, `get_font_count*` |
| | `get_family_styles` | `family` → `FamilyStyleDto[]` | 14 | |
| | `set_favorite` | `face, on` | 14 | `toggle_favorite` |
| | `list_collections`, `create_collection`, `rename_collection`, `delete_collection`, `add_to_collection`, `remove_from_collection` | | 14 | các lệnh collection cũ |
| | `list_tags`, `tags_of`, `create_tag`, `tag_faces`, `untag_faces` | | 14 | |
| | `list_problems`, `retry_problems` | | 14 | |
| | `reveal_in_finder` | `face` | 14 | |
| Hệ thống | `backup_user_db` | → `string` (đường dẫn) | 14 | |
| | `disk_free` | `target: "models" \| "data"` → `number` | 14 | |
| | `get_ui_state`, `set_ui_state` | `UiStateDto` | 14 | |
| | `get_setting`, `set_setting` | `key` (`ui.*`), `value` ≤ 64 KB → `string \| null` | 14 | |
| Phân tích | `get_face` | `face, lang` → `FaceDetailDto` | 15 | `parse_font_with_sidecar` |
| | `similar` | `face, opts` → `SimilarHitDto[]` | 15 | `find_similar_fonts` |
| | `pair_candidates` | `face, opts` → `PairCandidateDto[]` | 15 | `find_font_pairs`, `find_font_combos` |
| | `assess_pair` | `a, b, opts` → `PairCandidateDto` | 15 | |
| | `save_pair`, `list_pairs`, `delete_pair` | | 15 | |
| | `describe` | `text` → `DescribeDto` | 15 | |
| Cài đặt | `get_settings`, `set_settings` | `patch: SettingsPatchDto` → `SettingsDto` | 16 | |
| Model | `model_status` | → `ModelStatusDto` | 16 | |
| | `model_download` | `onEvent: Channel<ModelEvent>` | 16 | |
| | `model_cancel`, `model_redownload`, `model_remove` | | 16 | |
| Hàng đợi | `embed_status` | → `EmbedStatusDto` | 16 | |
| | `embed_pause`, `embed_resume`, `embed_subscribe`, `embed_failures` | | 16 | `generate_visual_embedding`, `batch_generate_embeddings` |
| | `queue_pause_all`, `queue_resume_all` | | 16 | |
| Kích hoạt (service: Plan 7; tên theo hợp đồng Plan 7) | `activation_caps` | → `{ method, supported }` | 17 | |
| | `activation_state` | → `ActivationStateDto[]` | 17 | |
| | `activation_preflight` | `faces` → `ActivationReport` | 17 | |
| | `activation_activate` | `faces, scope?, proceed, onProgress` → `ActivationReport` | 17 | |
| | `activation_activate_collection` | `collectionId, scope?, proceed, onProgress` → `ActivationReport` | 17 | |
| | `activation_deactivate` | `faces, force, onProgress` → `ActivationReport` (có thể `HeldByCollections`) | 17 | |
| | `activation_deactivate_collection` | `collectionId, onProgress` → `ActivationReport` | 17 | |
| | `activation_deactivate_all` | `onProgress` → `ActivationReport` | 17 | |
| | `activation_set_keep` | `on` | 17 | |
| | `activation_subscribe` | `onEvent: Channel<ActivationEvent>` (`Reconciled` mang `auto_deactivated`) | 17 | |
| | `detect_font_managers` | → `FontManagerDto[]` (`activeCount`: Plan 7) | 14 | |
| Preview | `preview_info`, `preview_bump_generation`, `preview_prewarm` | (như Plan 5) | 18 | |
| | `preview_cache_info`, `clear_preview_cache` | | 18 | |
| Scheme | `fontatlas-preview://`, `fontatlas-font://` | | 18 | `read_font_file`, `file_exists` |
| Gỡ | `add_font_to_library`, `remove_font_from_library`, `clear_library`, `open_folder_dialog`, `open_font_dialog`, `get_app_data_dir`, `write_log`, `get_collections` | dialog chuyển sang `@tauri-apps/plugin-dialog`; log chỉ ghi từ Rust (`tauri-plugin-log`), frontend không gọi log | 11 | |

**Lead đã quyết (2026-11-05):**
- `similar` chỉ dùng ảnh khi font gốc `NotComparable`: **trong 6a** (Task 10, Task 15), nguồn ghi `"image"`.
- `vi_glyph_table` có hộp mực dấu theo glyph: **hoãn sau MVP**. Tab chồng dấu dùng `missing` + `stackedMarksOverflow` + `stackStyle` sẵn có trong `get_face`.
- `rebuild_library` chủ động: **hoãn**. Đổi `ANALYZER_VERSION` đã tự phân tích lại.
- File watcher: **hoãn** (spec 1.3).
- `activeCount` của trình quản lý font: **Plan 7**.

## Lộ trình sau 6a

| Plan | Phạm vi | Phụ thuộc |
|---|---|---|
| 7 `fontatlas-activate` (06, **gộp trước 6a**) | user.db v2, `ActivationEngine` + `MacCoreTextEngine`, `ActivationService` (lý do, `Busy`, reconcile/shutdown), `activeCount`; Windows sau | dev/mvp |
| 6b Giao diện | Thay `UX/services/*.ts` bằng `UX/bindings`; màn theo prototype đã duyệt + UX 03; S2 trong Tauri thật và trên Windows | 6a (đã gồm 7) |
