# Plan 6a — báo cáo

Nhánh `feat/tauri-backend` (từ `dev/mvp` 50b042f, đã gộp Plan 7). Plan: `docs/superpowers/plans/2026-11-04-plan6a-tauri-backend.md`. Chưa push, chưa merge.

## Kết quả

- `cargo test --workspace`: **620 test qua**, 22 bị `ignored` (đo hiệu năng, test sống CoreText, 50k golden). `cargo clippy --workspace --all-targets -- -D warnings` sạch.
- Frontend: `npx tsc --noEmit` sạch, `vitest run` 231/231 (gồm `bindings/__tests__/contract.test.ts`), `npm run build` qua.
- Smoke macOS: `cargo tauri build --debug --bundles app` ra `Font Atlas.app`; mở với `FONTATLAS_DATA_DIR` tạm, sau 12 s tiến trình còn sống, có `library.db`, `user.db`, `cache/`, không panic (`UX/src-tauri/tests/smoke.sh`).
- **Windows: chưa kiểm.** Engine Windows là stub của Plan 7 (`activation_unsupported`).
- **Kích hoạt font thật trên CoreText chưa thử tay** (cần chủ app đồng ý, theo quy tắc test sống của Plan 7). Toàn bộ đường lệnh được kiểm bằng `FakeEngine`.

## Thời gian trên thư viện thật

Thư viện: google/fonts @47d54ca (3 886 face), `--release`, máy phát triển (12 luồng).

| Việc | Thời gian |
|---|---|
| Quét + phân tích + dựng Index lần đầu | 5,4 s |
| Dựng lại Index (kết nối đọc riêng) | 88 ms |
| `query_faces` trang 100, không lọc | 1,1 ms |
| `query_faces` text = "sans" | 0,8 ms |
| `query_faces` lọc trục (`describe` "ấm tối giản") | 4,4 ms |
| `similar` k = 12 | 0,5 ms |
| `pair_candidates` k = 20, `projectVi` | 1,1 ms |
| `get_face` | 1,6 ms |

Tái lập: `FONTATLAS_PERF_DIR=<thư mục font> cargo test -p app --release --test perf -- --ignored --nocapture`.

## Commit

- 963a7d7 feat(app): ts-rs bindings, generated TauriCommandMap, JSON fixtures, contract tests, CI drift check
- beebb46 feat(app): preview/font schemes with CORS, preview and cache commands
- 814521a feat(app): activation commands over Plan 7 service, reconcile-before-ready, exit shutdown
- 9580927 feat(app): settings, resumable model download, embed queue status/ETA/device, pause-all, lifecycle on_exit, rescan-on-launch with analyzer-version trigger
- cbf848e feat(app): face detail, similar, pairing (collection scope), saved pairs, describe-to-filters; index bundle with debounced rebuild
- 58eaa1e feat(app): library, family styles, reveal in Finder, backup, disk free, UI state, ui.* settings commands
- 7c8ce7f feat(app): sources, single-flight scan manager with pause/resume, progress hub over Channel
- df54ff2 feat(app): boot sequence, onboarding state, user-driven legacy import
- 578bfee refactor(app): remove Python sidecar, Qdrant and legacy commands; new app skeleton, CSP, minimal capabilities
- 317a0d5 feat(index): image-only similar for non-comparable queries
- 20eb54e feat(embed): resumable model download with Range, speed, disk free
- 58994aa feat(describe): descriptor dictionary 0.1.0-draft to visible axis filters, conflicts, evaluation
- b78cbdc feat(index): storeys, magnitude percentile, face summary, similar options, filtered pair candidates
- 0dcacbd feat(core): copyright, vendor_url, named instance coordinates; core-0.3.0; rebuild reference-v1
- 11ccb1c feat(store): user.db v3 saved pairs (after Plan 7 v2)
- f020cb4 style(store): clippy clean tests
- 8d38a7e feat(store): scoped face query with match sort
- f18e998 feat(store): read latest analyses, face path, legacy preview, on-demand user backup
- e9c7a11 feat(store): rescan progress callback, cancellation, batched analyze+ingest; io errors stay pending
- 2dd1842 fix(store): IMMEDIATE write transactions; user-only and reader connections
- 119cc7c docs(plan6a): bring approved plan b2cc0fd onto feat/tauri-backend

## Sau review độc lập

Rebase lên `dev/mvp` d53a653 (bản sửa Plan 7). Luồng quét dùng `Store::open_library_only` (không attach `user.db`); test yêu thích trong lúc quét từng đỏ 231 ms với kết nối attach. Reviewer (Sonnet) tìm 9 mục Important; đã sửa (mỗi mục có test hồi quy ở `tests/review_fixes.rs` hoặc crate liên quan):

- `ensure_started` không còn race mở hai worker; `remove_source` dừng/loại nguồn khỏi hàng quét trước khi xoá;
- tắt app: cờ huỷ đặt cho quét/model/embed trước rồi mới chờ, bỏ móc sau-quét khi đang tắt, pha băm kiểm huỷ;
- bộ lọc `mode` của `pair_candidates` áp trước khi cắt top-k (`pair_candidates_gated`);
- `boot_subscribe`/`embed_subscribe` đăng ký trước, gửi ảnh chụp sau;
- đi cây thư mục: lỗi từng mục đẩy thư mục cha vào "không đọc được" (không đánh missing oan);
- panic trong job quét không làm kẹt hàng đợi;
- `FaceQueryDto.hard` (ràng buộc cứng của `describe`); `model_redownload`/`model_remove` trả `busy` thay vì xoá file khi lần tải chưa dừng.

Minor chưa làm (ghi để sau): `counts()` embed đếm trên kết nối chính; `enqueue` bỏ qua yêu cầu quét lại nguồn đang chạy (cờ dirty); nạp lại `temp.scope` mỗi trang; `claim_embedding_batch` O(N²); luồng đọc nền của tải model không có timeout thân; `Index::profiles_for_test` dùng ở đường chạy thật (đổi tên); chặn cặp trùng; `VACUUM` cache preview; `on_exit` chạy đồng bộ ở `RunEvent::Exit`.

## Quyết định (Ruling) đã chốt khi thực thi

Đầy đủ trong sổ `.superpowers/sdd/…/progress.md` (git-ignore). Những cái ảnh hưởng 6b/Plan 7:

1. **Khoá khi ingest.** `BEGIN IMMEDIATE` trên kết nối có attach `user.db` khoá cả `user.db` trong lúc giữ. Mọi ghi dữ liệu người dùng vẫn đi qua `AppState.user` (chỉ `user.db`); lô ingest ngắn nên trễ yêu thích khi quét 600 file < 200 ms (test `favorite_during_scan_latency_under_200ms`). Nếu sau này lô dài hơn, chuyển luồng quét sang kết nối chỉ `library.db`.
2. **Quét lại chạy nền sau `ready`**, không chặn khởi động. `ready` đã gồm Index (dựng từ bản phân tích mới nhất, kể cả version cũ) và đối soát kích hoạt.
3. **Đổi `ANALYZER_VERSION`** (nay `core-0.3.0`) tự quét lại bất kể `rescanOnLaunch`; `analyzer_version_seen` ghi khi không còn face cũ.
4. **`keepActiveOnExit` mặc định `false`** (setting chưa có = scope `Session`), khớp `ActivationService::default_scope` của Plan 7.
5. **`vietnamese.total` = 134**: core đếm 186 (134 chữ Việt + 52 chữ ASCII); DTO trừ 52 để khớp câu "Đủ 134 ký tự Việt."
6. **`describe` + `query_faces`**: bộ lọc `required` loại face, `preferred` chỉ xếp hạng; hai face cùng thoả khoảng hoà điểm → xếp theo tên họ.
7. **`model_download` / `model_redownload` trả ngay khi bắt đầu**; tiến độ, `Ready`, `Paused`, `Failed` qua `Channel`. Đang tải thì gắn vào lần tải đó.
8. **`activation_state`** trả hợp đồng Plan 7 + `faces` (các `faceIndex` của file). `reasons` giữ dạng `{kind, collectionId}` của Plan 7, **khác** kiểu giả trong `UX/api/types.ts` (`string | {collection}`): 05 cần chỉnh khi nối bindings thật.
9. `precomposedComposite` là `bool` (số glyph dựng sẵn ghép > 0); core lưu số đếm.

## Cho 6b

- Kiểu: `UX/bindings/index.ts`, bản đồ lệnh `UX/bindings/commands.ts` (`TauriCommandMap`, `CommandName`), fixture `UX/bindings/fixtures/*.json`, README sinh lại.
- Đối số `Channel` nhận qua tham số cuối (`onEvent` / `onProgress`). `EmbedEvent` phân biệt bằng `kind`, các sự kiện khác bằng `type`.
- Dùng `faceKeys` + `sort: "match"` cho "Gần đây" và "Đang bật" (lấy `faces` từ `activation_state`).
- Khoá `ui.*` ≤ 60 ký tự sau tiền tố, giá trị ≤ 64 KB; `ui.state` là khoá của `get_ui_state`/`set_ui_state`.
- Frontend không gọi plugin log; capabilities chỉ `core:default`, `core:window:allow-start-dragging`, `dialog:allow-open`.
- Ghi chú đã làm ngoài `UX/bindings`: `UX/vitest.config.ts` thêm `bindings/__tests__/**` vào `include` (một dòng) để contract test chạy.

## Cho Plan 7

- Chỗ cắm engine: `AppState.activation` (`Box<dyn ActivationApi>`, blanket impl cho `ActivationService<E>`); app thật dùng `platform_api()`.
- Chạy lại khi đổi service: `cargo test -p app --test activation --test bindings` (hợp đồng JSON so với 3 fixture của Plan 7).
- Chưa dùng `activeCount` của trình quản lý font (luôn `null`).
- Hoãn (ngoài MVP, đã chốt với Lead): `vi_glyph_table`, `rebuild_library`, file watcher.
