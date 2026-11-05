# Báo cáo Plan 5 — `fontatlas-preview` + lưới thư viện mới

Nhánh `feat/preview-grid` (tách từ `dev/mvp` @ 7840126). Chưa push, chưa merge, chưa rebase lên `dev/mvp` mới.
Máy đo: Apple M4 Pro, 24 GB, macOS 27.2, rustc 1.96.0, Node 22.

## Kết quả
- `cargo test -p fontatlas-core -p fontatlas-store -p fontatlas-preview`: **163 test qua**.
  - Riêng preview: `text` 10, `render` 8 (+1 test hiệu năng chạy bằng `--ignored`), `cache` 11, `service` 10, `http` 7.
- `cargo clippy` cho 3 crate trên, `--all-targets -D warnings`: không cảnh báo.
- `npm test` (Vitest): **50/50**, chạy 3 lần liền đều qua. `npx tsc --noEmit` cho toàn UX: sạch. `vite build`: qua.
- Không sửa `fontatlas-core` và lưới cũ trong `UX/components/`.

## Số đo
| Đo | Kết quả |
|---|---|
| `render_face` (release, một luồng, 5 font mẫu) | **0,35–0,37 ms/face** (spec: ~0,67, ngưỡng plan 1 ms) |
| `dump_previews` trên `/System/Library/Fonts` + `/Library/Fonts` | 781 face, 6 lỗi (emoji/bitmap → `no_outlines`), 0,67 ms/face song song gồm đọc file |
| PNG 624×112 (px 48), grayscale+alpha | trung bình 5,4 KB, trung vị 5,0 KB, p90 8,0 KB → 10k font ≈ 54 MB |

### S2 — harness WKWebView, 5.000 ô, trung vị 3 lần
| Lượt | Mode | FPS cuộn nhanh | p95 khung | Ô trống nhanh / chậm | Bộ nhớ WebContent | Đổi chữ mẫu | Hiển thị đầu |
|---|---|---|---|---|---|---|---|
| Task 9 (trước review) | mask | 60,2 | 19 ms | 0% / 0% | 208 MB | 134 ms | 145 ms |
| Task 9 (trước review) | img | 60,2 | 18 ms | 0% / 0% | 229 MB | 116 ms | 120 ms |
| Sau sửa review, lượt 1 (máy tải nặng) | mask | 58,0 | 18 ms | 0,5% / 0% | 226 MB | 216 ms | 146 ms |
| Sau sửa review, lượt 1 (máy tải nặng) | img | 59,2 | 18 ms | 0% / 0% | 221 MB | 166 ms | 103 ms |
| Sau sửa review, lượt 2 | mask | **60,0** | **17 ms** | **0,3% / 0%** | **220 MB** | 166 ms | 123 ms |

- **Cả hai mode đều đạt S2** (≥ 58 FPS, p95 ≤ 20 ms, 0% ô trống khi cuộn chậm, < 400 MB). Giữ `DEFAULT_PREVIEW_MODE = 'mask'`.
- Lượt 1 sau review chạy khi máy đang tải nặng (load average 12,4: Illustrator ~110% CPU, một tiến trình Vitest của phiên khác ~100%, OneDrive). Lần chạy lạnh đầu tiên của mode mask chỉ đạt 57,3 FPS với 19% ô trống khi cuộn nhanh, kéo trung vị xuống đúng ngưỡng 58,0. Lượt 2 lúc máy nhẹ hơn đạt 60,0 FPS. Các thay đổi lúc sửa review không chạm đường cuộn (chỉ thêm callback ổn định và trạng thái lỗi).
- Ghi chú cho người đọc số: S2 đo với backend giả (Vite phục vụ PNG dựng sẵn), không qua IPC Tauri. Plan 6 cần đo lại trong Tauri thật và trên Windows/WebView2.
- `fontfaces = 0` trong mọi lượt: lưới không tạo FontFace khi chỉ cuộn.

## Lệch so với plan (đều có trong ledger, Lead đã đồng ý các mục Task 1–9)
1. `familyName` dùng đủ 64 ký tự hash thay vì 16: test lộ trùng khoá khi hai hash có chung tiền tố.
2. Môi trường test:
   - `test/setup.ts` thêm `afterEach(cleanup)`, vì Vitest không bật `globals` nên Testing Library không tự dọn DOM.
   - Ép `requestAnimationFrame = setTimeout(0)`, vì jsdom có sẵn rAF 16 ms theo đồng hồ thật.
   - Helper `settle` chờ 5 lượt. Không nới assertion nào.
3. Lỗi mở file dùng chung trong một nhóm `prewarm` được nhân bản thành `Core(Parse)` (vẫn 422), vì `PreviewError` không `Clone`.
4. Tên file trong URL không percent-decode (chỉ gồm hex, `_`, số); có `%` thì trả 400.
5. `PreviewCache::is_empty` thêm theo clippy.
6. **Lỗi của plan**: chữ live dùng `px·3/4` css px, to gấp 1,5 lần ảnh preview (ảnh vẽ ở px trên canvas 2× → px/2 css px). Đã sửa thành `px/2`.
7. Mock `createMockLibraryApi({ fail })` nhận query (`fail(q)`) để giả lỗi theo trang; tương thích ngược.
8. CI thêm `npx tsc --noEmit` (spec mục 11 yêu cầu, plan bỏ sót).

## Review toàn nhánh (subagent Opus, sau Task 9)
Kết luận gốc: 0 Critical, 4 Important, 9 Minor, "With fixes". Tôi nâng hai Minor lên Important theo tác động thực tế:
- mục 7: mở cache lỗi có thể làm Plan 6 không dựng được service, tức mất toàn bộ preview;
- mục 12: thiếu `tsc` trong CI.

Đã sửa 6 mục (commit effc0a1..9d8d484). Mỗi mục có test đỏ trước rồi xanh; suite cuối 163 Rust + 50 Vitest.

| # | Vấn đề | Sửa | Test |
|---|---|---|---|
| I1 | Một trang lỗi thay cả lưới bằng thông báo, không có cách thử lại. Khi sửa còn lộ thêm **vòng lặp vô hạn** lỗi → render → fetch nếu lỗi kéo dài (test treo). | Giữ lưới và hiện banner "Thử lại" khi đã có dữ liệu. `PagedFaces` đánh dấu trang lỗi, chỉ fetch lại qua `retry()` hoặc khi đổi query. | `a failed page keeps the grid and can be retried`, `surfaces errors, does not refetch a failed page on its own…` |
| I2 | 410 bị nuốt với giả định "URL mới sắp tới": nếu thế hệ backend tăng vì nguồn khác thì ô trắng vĩnh viễn. Lỗi `previewInfo`/`bumpGeneration` cũng bị nuốt im lặng. | Ô gặp 410 gọi `onStale` → grid lấy lại thế hệ hiện tại (gộp nhiều ô thành một lần gọi). Lỗi IPC hiện "Không tải được preview" kèm nút Thử lại. | `recovers when the backend generation moved on elsewhere (410)`, `shows an error when preview info cannot be loaded` |
| I3 | Cỡ chữ live sai 1,5× (lỗi plan). | `fontSize: px/2` | `live font renders at the same size as the preview` |
| I4 | Loader tự tạo không gỡ FontFace khi grid unmount; `release` không evict. | `FontFaceLoader.dispose()`. Grid tạo và huỷ loader trong cùng một effect. `release` gọi evict khi vượt ngân sách. | `dispose removes every face…`, `release evicts when over budget…`, `disposes its own FontFaceLoader on unmount` |
| 7→I | Đổi tên file cache khi gặp *bất kỳ* lỗi mở nào; mở lại lỗi thì trả `Err`. | Chỉ đổi tên khi `SQLITE_CORRUPT`/`NOTADB`, giữ một file broken gần nhất; còn lại dùng cache trong bộ nhớ. | `unopenable_cache_falls_back_to_memory_without_renaming`, `keeps_at_most_one_broken_file` |
| 12→I | CI thiếu `tsc --noEmit`. | Thêm bước vào job `ux`. | Kiểm bằng lệnh trên máy (exit 0). |

**Minor để lại** (chưa sửa, ghi để Lead quyết):
- `FaceCell`: ô bị bỏ chọn trong lúc font đang tải vẫn có thể hiện font live mà không giữ ref.
- `FontFaceLoader`: nếu `fonts.add` throw thì promise không resolve.
- `render.rs`:
  - font không map khoảng trắng làm các từ dính nhau;
  - glyph có LSB âm hoặc nét vươn có thể bị cắt mép trái/phải;
  - baseline bám mép trên, nên baseline giữa các font trong cùng hàng không thẳng.
- Lỗi preview (422) không được nhớ ở cả frontend lẫn backend, nên font lỗi bị render lại mỗi lần ô mount.
- Đổi query làm lưới trắng tới khi trang 0 về.
- Chữ mẫu so sánh trước khi trim nên thêm/bớt khoảng trắng cuối gây bump thừa. `prewarm` không dedupe request trùng. `run_s2.py` báo `KeyError` nếu `footprint` không trả `phys_footprint`.

**Mục reviewer để ngoài phạm vi** (đã ghi quyết định trong ledger): CORS thực tế khi custom scheme gửi/không gửi `Origin`; FontFace nạp `font/collection`; `preview_prewarm`; S2 trên WebView2 và trong Tauri thật. Cả bốn đều thuộc Plan 6.

## Ghi chú cho Plan 6
- Cài `TauriCommandMap` (`UX/features/library-grid/api.ts`):
  - `query_faces` trả `FacePage`; gọi `parseFacePage` lên kết quả.
  - `preview_info` và `preview_bump_generation` đọc/tăng `PreviewService::generation()` / `bump_generation()`.
  - `preview_prewarm` gọi `PreviewService::prewarm` sau `onRowsRendered`.
- Đăng ký hai scheme `fontatlas-preview` và `fontatlas-font`, gọi `handle_preview_uri` / `handle_font_uri` trong `spawn_blocking`.
  - Truyền header `Origin` của request vào handler.
  - Gắn `Access-Control-Allow-Origin`, `Vary`, `Cache-Control` từ `PreviewHttp`.
  - Dev thêm `CorsPolicy::app().with("http://localhost:3000")`.
- `FaceLocator` cài bằng `Store`: lấy vị trí `missing = 0` đầu tiên của `(hash, face_index)`.
- CSP:

  ```
  img-src 'self' data: blob: fontatlas-preview: http://fontatlas-preview.localhost
  font-src 'self' fontatlas-font: http://fontatlas-font.localhost
  connect-src 'self' ipc: http://ipc.localhost fontatlas-preview: http://fontatlas-preview.localhost
  ```

- Mở `PreviewCache` ở thư mục cache của app với `Budgets::DEFAULT` (64 MiB chữ mẫu mặc định + 32 MiB chữ tuỳ chỉnh). Cache tự rơi về bộ nhớ nếu không mở được file.
- Đo lại S2 trong Tauri thật (`npm run perf:s2` hiện dùng backend giả) và trên Windows/WebView2. Nếu mode `mask` trượt ở đó thì bật `previewMode="img"`.
- Khi gộp: Lead tạo lại `Cargo.lock` bằng `cargo update -w`. `tiny-skia =0.12.0` phải chỉ còn một bản (crate embed cũng dùng).
