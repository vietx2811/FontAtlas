# Plan 5 — `fontatlas-preview` + lưới thư viện mới — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dựng crate Rust `fontatlas-preview` để vẽ preview PNG alpha-mask đúng face, có cache trên đĩa và hai handler custom protocol (preview, font). Song song, dựng module lưới thư viện mới trong `UX/` gồm react-window v2, preview bằng `mask-image` tải một lần, bộ nạp FontFace có ngân sách theo byte và phân trang qua `query_faces`. Harness hiệu năng của spike 05 được đưa vào thành test hồi quy cho S2.

**Architecture:**
- **Rust.** Crate `fontatlas-preview` nhận `fontatlas_core::Face` và dùng skrifa để lấy outline theo cmap + advance. Nó co chữ theo hộp mực thật để dấu chồng tiếng Việt không bị cắt, vẽ bằng tiny-skia, rồi xuất PNG grayscale+alpha.
  - Cache là một file SQLite (`previews.db`) gồm hai vùng LRU theo byte: chữ mẫu mặc định và chữ mẫu tuỳ chỉnh.
  - `PreviewService` **sở hữu thế hệ (generation)**: frontend xin thế hệ mới, không tự đếm.
  - Hai hàm thuần `handle_preview_uri` và `handle_font_uri` biến URL của scheme `fontatlas-preview` / `fontatlas-font` thành phản hồi HTTP, kèm CORS theo origin của app. Plan 6 chỉ việc nối chúng vào `register_asynchronous_uri_scheme_protocol`.
- **Frontend.** Module mới `UX/features/library-grid/` không đụng lưới cũ. Thứ tự lớp:
  1. `PagedFaces`: phân trang, lọc ở backend.
  2. `PreviewBlobs`: tải một lần mỗi URL → blob URL dùng cho `mask-image`.
  3. `FontFaceLoader`: chính sách b2, nạp bằng `FontFace(name, 'url(...)')`.
  4. `LibraryGrid` / `FaceCell`.

  Lệnh Tauri thật thuộc Plan 6. Plan này định nghĩa hợp đồng TS, đọc fixture JSON do store sinh ra, và dùng mock trong Vitest.

**Tech Stack:**
- Rust edition 2024 (rustc ≥ 1.85): skrifa =0.48.0, read-fonts =0.45.1 (qua core), tiny-skia =0.12.0, png =0.18.1, rusqlite =0.31.0 bundled, blake3 =1.8.7, unicode-normalization 0.1.25, url 2.5, rayon 1.12, thiserror 2; dev: tempfile 3.27, serde_json 1.
- Frontend: React 19, react-window 2.3.4, Vitest 3.2.4, jsdom 26.1.0, @testing-library/react 16.3.0, @testing-library/dom 10.4.0, @testing-library/jest-dom 6.6.3.
- Harness: Swift (WKWebView), Python 3.
- CI: GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-16-fontatlas-mvp-design.md`, mục 7.1, 1.5 (S2), 11. Số liệu và lý do lựa chọn lấy từ `docs/team/spikes/05_frontend_perf.md` (mode c, bộ nạp b2). Đây là Plan 5 trong lộ trình ở cuối Plan 1.

**Nhánh:** `feat/preview-grid`, tạo từ **`dev/mvp`** (HEAD `7840126`, đã có `fontatlas-core` và `fontatlas-store`). `members` của workspace đã có core và store; Task 1 chỉ thêm `crates/fontatlas-preview`. Khi gộp ngược vào `dev/mvp`, Lead tạo lại `Cargo.lock` bằng `cargo update -w`.

**Hợp đồng `FaceRow` với store:**
- `FaceRow` của Plan 2 đã có `fileSize: u64` (byte của file gốc). Bộ nạp FontFace dùng giá trị này để tính ngân sách.
- Fixture hợp đồng có sẵn ở `UX/src-tauri/crates/fontatlas-store/tests/fixtures/face_row.json`, sinh từ serde thật bởi test `face_row_json_matches_contract_fixture` của store. Task 6 cho TS đọc **đúng file này**.
- Khi `FaceRow` đổi, test Rust của store và test TS của plan này cùng đỏ.

## Global Constraints

- Ghim phiên bản Rust:
  - `skrifa = "=0.48.0"`, `read-fonts = "=0.45.1"`: trùng core.
  - `tiny-skia = { version = "=0.12.0", default-features = false, features = ["std", "simd"] }`: tắt `png-format`.
  - `png = "=0.18.1"`. Đây là **bản png thứ hai** trong cây phụ thuộc, vì app Tauri đã kéo một bản 0.17.x qua tauri/image. Chấp nhận: crate nhỏ, không có `links`.
  - `rusqlite = { version = "=0.31.0", features = ["bundled"] }`: phải trùng app và store (`links = "sqlite3"`).
  - `blake3 = "=1.8.7"`, `unicode-normalization = "0.1.25"`, `url = "2.5"`.
- `RENDERER_VERSION = "preview-0.1.0"`. Tăng giá trị này khi đổi cách vẽ: khoá cache đổi theo, và URL mang `r=` nên cache của WebView cũng đổi.
- `PX_BUCKETS = [32, 48, 64, 96]`, mặc định 48. Canvas `W = 13·px`, `H = 7·px/3` (chia nguyên): ở px 48 là 624×112. Ảnh hiển thị ở **một nửa** kích thước (mật độ 2×): px 48 → 312×56 css px. Px ngoài danh sách → `bad_size`.
- Chữ mẫu: NFC, `trim`, tối đa `MAX_TEXT_CHARS = 64` ký tự. Chuỗi rỗng → `DEFAULT_SAMPLE = "Hamburgefonts Ăn phở"`. Chữ dự phòng Latin: `LATIN_FALLBACK = "Hamburgefonts"`. Mẫu lấy từ cmap: tối đa `CMAP_SAMPLE_CHARS = 12` ký tự.
- **Không shaping phức tạp:** chỉ cmap + advance, không kerning, không GSUB/GPOS, không bidi, không nối chữ Ả Rập/Ấn. Không dùng kerning bảng `kern` cũ: font hiện đại kern bằng GPOS, nên `kern` cũ phủ ít font mà lại thêm một nhánh code. Dấu tiếng Việt chỉ đúng với ký tự dựng sẵn (NFC). Giới hạn này được ghi trong doc của crate và trong báo cáo.
- PNG `GrayscaleAlpha` 8 bit: gray = 0, alpha = độ phủ.
- Cache:
  - File `previews.db` trong thư mục cache của app.
  - Hai vùng `Pool::Default` (chữ mẫu đã chuẩn hoá == `DEFAULT_SAMPLE`) và `Pool::Custom`. `Budgets::DEFAULT = { default_bytes: 64 MiB, custom_bytes: 32 MiB }`.
  - Mỗi vùng tự đẩy LRU xuống ≤ 90% ngân sách của nó. Entry lớn hơn ngân sách vùng thì không lưu.
  - File hỏng lúc mở → đổi tên `previews.broken-{unix_ms}.db` (cùng `-wal`/`-shm`) rồi tạo mới.
  - Lỗi cache lúc chạy (`get`/`put`) → coi như miss / bỏ qua ghi và vẫn render. Cache không bao giờ làm hỏng việc hiển thị.
- **Thế hệ preview do backend sở hữu.**
  - `PreviewService::bump_generation() -> u64` trả giá trị mới; `generation()` trả giá trị hiện tại; giá trị chỉ tăng.
  - Frontend lấy giá trị ban đầu qua `preview_info` và giá trị mới qua `preview_bump_generation`. Không bao giờ tự đếm.
  - Yêu cầu có `g` < hiện tại → 410.
- Font variable được vẽ ở instance mặc định (`LocationRef::default()`).
- Lỗi `PreviewError::code()`:

  | Mã | HTTP |
  |---|---|
  | `bad_size` | 400 |
  | `bad_request` | 400 |
  | `not_found` | 404 |
  | `stale_file` | 409 |
  | `stale_generation` | 410 |
  | `no_outlines` | 422 |
  | `nothing_fits` | 422 |
  | `ttc_face` | 422 |
  | `core` | 422; riêng `io` → 404 |
  | `cache` | 500 |
  | `encode` | 500 |

- Custom protocol, hỗ trợ cả hai dạng URL: macOS `{scheme}://localhost/{name}`, Windows `http://{scheme}.localhost/{name}`.
  - Preview: scheme `fontatlas-preview`, tên `{hash_hex}_{face_index}_{px}.png`, query `t` (chữ mẫu), `g` (thế hệ, tuỳ chọn), `r` (renderer version). 200 → `image/png`, `Cache-Control: private, max-age=86400`.
  - Font: scheme `fontatlas-font`, tên `{hash_hex}_{face_index}.font`. Chỉ `face_index = 0`; face ≥ 1 → 422 `ttc_face`. 200 → content-type theo container (`font/ttf`, `font/otf`, `font/collection`, `font/woff`, `font/woff2`).
  - **CORS:** mọi phản hồi có `Vary: Origin`. `Access-Control-Allow-Origin` = đúng origin của yêu cầu, chỉ khi origin đó nằm trong `CorsPolicy`; **không bao giờ `*`**.
  - `CorsPolicy::app()` = `["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost"]`. Plan 6 thêm origin của dev server (`http://localhost:3000`) khi chạy dev.
- CSP cho Plan 6 áp dụng (giữ nguyên phần `default-src`, `style-src`, `script-src` hiện có):
  - `img-src 'self' data: blob: fontatlas-preview: http://fontatlas-preview.localhost`: `mask-image` dùng blob URL
  - `font-src 'self' fontatlas-font: http://fontatlas-font.localhost`
  - `connect-src 'self' ipc: http://ipc.localhost fontatlas-preview: http://fontatlas-preview.localhost`: `fetch` preview
- **Font không đi qua asset scope theo đường dẫn, không fetch → ArrayBuffer.** Frontend dùng `new FontFace(name, 'url("fontatlas-font://…")')`; backend tra đường dẫn theo `(hash, face_index)` qua `FaceLocator`.
- Frontend:
  - Code mới **chỉ** nằm trong `UX/features/library-grid/` (kể cả `perf/`). Không sửa `UX/components/`, `UX/hooks/`, `UX/App.tsx`: Plan 6 sẽ thay lưới cũ.
  - Thay `react-window` bằng đúng `2.3.4` và gỡ `@types/react-window`.
  - Kiểu trả về của component là `React.JSX.Element`.
- Preview trong ô:
  - `DEFAULT_PREVIEW_MODE = 'mask'`: mỗi URL được `fetch` **một lần** → blob URL → `mask-image` của một `div` có `background-color: var(--foreground)`. Ô `ready` sau khi blob đã gắn vào style **và** qua một khung (rAF).
  - Khi URL đổi, giữ ảnh cũ cho tới khi ảnh mới sẵn sàng (không nháy).
  - **Phương án dự phòng** `previewMode = 'img'` (một cờ): `<img src>` thường, theme tối dùng `filter: invert(1)`. Chỉ bật nếu S2 trượt với `mask`.
  - `PreviewBlobs`: dedupe theo URL, đếm tham chiếu, giữ tối đa `MAX_BLOBS = 800` blob không còn ai dùng, `revokeObjectURL` khi đẩy ra.
- Bộ nạp FontFace:
  - `BUDGET_BYTES = 128 * 1024 * 1024` (cộng `row.fileSize`), `MAX_ACTIVE = 6`, `V_MAX = 1500` px/s, `IDLE_MS = 120`, `HOVER_DELAY_MS = 150`.
  - **Face có `faceIndex ≥ 1` không bao giờ dùng FontFace.**
  - Lỗi tải font → `null`, không thử lại trong phiên.
- Lọc, tìm và phân trang: **luôn ở backend**. `PAGE_SIZE = 200` (≤ `MAX_LIMIT` 1000 của store).
  - Chỉ đổi `text` → debounce 150 ms. Đổi bộ lọc khác → áp dụng ngay **và huỷ timer debounce đang chờ**.
  - Không lọc phía client.
  - Mọi đối tượng có vòng đời (`PagedFaces`, timer, blob) được tạo và huỷ trong **cùng một effect** để an toàn với `<StrictMode>`.
- Lưới: hàng cao `cssH + 64` (ở px 48 là 120), ô rộng tối thiểu `cssW + 24`, `overscanCount = 2`.
- S2 (harness, trung vị 3 lần, n = 5000, WKWebView, `previewMode = 'mask'`):
  - cuộn nhanh: FPS ≥ 58 và p95 khung ≤ 20 ms
  - cuộn chậm: % ô trống = 0
  - bộ nhớ WebContent: `phys_footprint` < 400 MB
- Lệnh Rust **luôn giới hạn** `-p fontatlas-core -p fontatlas-store -p fontatlas-preview`. Không dùng `--workspace`, vì crate app cũ cần thư viện hệ thống của Tauri.
- CI (`.github/workflows/preview-grid.yml`):
  - job `rust` (ubuntu): test + clippy cho ba crate trên
  - job `ux` (ubuntu): `npm ci`, `npm test`, `vite build`
  - job `s2` (`macos-14`): `continue-on-error: true`, chỉ báo cáo, không chặn merge
- Không `unwrap()`/`expect()` trên dữ liệu từ font, file, DB hoặc URL trong code thư viện (test thì được). Không panic trên input bất kỳ. Sort số thực dùng `total_cmp`.
- Comment trong code viết tiếng Việt ngắn gọn. Mỗi commit kết thúc bằng `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Remount hoặc `<StrictMode>`:** các ô không được rơi vào 410 hàng loạt (thế hệ phải lấy từ backend), và `PagedFaces` không bị dùng sau khi đã dispose. Test `remount_uses_backend_generation`, `works_under_strict_mode` (Task 8), `disposed_instance_ignores_responses` (Task 7), `bump_generation_is_monotonic` (Task 4).
2. **Face ở vị trí ≥ 1 trong TTC:** preview phải đúng face đó. Không bao giờ có FontFace hay URL font cho face đó. Test `ttc_face_one_matches_standalone` (Task 2), `font_uri_refuses_ttc_face` (Task 5), `refuses_face_index_above_zero` (Task 7), `hover_on_ttc_face_keeps_preview` (Task 8).
3. **Font không có glyph cho chữ mẫu, hoặc không có outline:**
   - thiếu glyph → chữ dự phòng Latin, sau đó mẫu từ cmap
   - không có outline → 422
   - ô hiện "Không có preview", không trắng trơn, không panic

   Test `pick_text_*` (Task 1), `no_outline_font_is_no_outlines` (Task 2), `error_state_shows_placeholder` (Task 8).
4. **Dấu chồng tiếng Việt chữ hoa ("Ỗ Ẫ Ậ") không bị cắt mép trên/dưới.** Test `fit_shrinks_when_ink_exceeds_canvas` (Task 1), `stacked_marks_are_not_clipped` (Task 2).
5. **Cache đầy, cache hỏng lúc mở hoặc lúc chạy, nhiều luồng cùng gọi:**
   - vẫn phục vụ preview
   - chữ tuỳ chỉnh không đẩy chữ mặc định ra
   - không vượt ngân sách

   Test `custom_churn_keeps_default_pool`, `oversized_entry_not_stored`, `corrupt_cache_is_renamed_and_recreated` (Task 3); `cache_errors_degrade_to_render`, `tiny_budget_still_serves`, `concurrent_get_and_prewarm` (Task 4).
6. **Đổi theme, đổi chữ mẫu, cuộn rất nhanh:**
   - đổi theme không tải lại ảnh
   - đổi chữ không nháy (giữ ảnh cũ tới khi có ảnh mới)
   - không tải FontFace cho ô lướt qua khi tốc độ > 1.500 px/s

   Test `theme_toggle_keeps_preview` và `text_change_keeps_old_image_until_new_ready` (Task 8), `fast_scroll_defers_until_idle` (Task 7), S2 (Task 9).

---

## File Structure

```
.github/workflows/preview-grid.yml               # Mới (Task 9)
UX/src-tauri/
├── Cargo.toml                                  # Sửa: thêm member crates/fontatlas-preview
├── crates/fontatlas-store/tests/fixtures/face_row.json   # Có sẵn (Plan 2) — chỉ đọc
└── crates/fontatlas-preview/
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs        # doc (giới hạn shaping), hằng số, re-export
    │   ├── error.rs      # PreviewError, code(), http_status()
    │   ├── text.rs       # normalize_text, pick_text, TextSource, canvas_size, check_px
    │   ├── render.rs     # fit, truncate_at, require_fit, render_face, cmap_sample
    │   ├── cache.rs      # PreviewCache, Pool, Budgets, cache_key
    │   ├── service.rs    # PreviewRequest, FaceLocator, PreviewService
    │   └── http.rs       # CorsPolicy, PreviewHttp, parse/handle_preview_uri, parse/handle_font_uri
    ├── examples/dump_previews.rs
    └── tests/
        ├── common/mod.rs
        ├── fixtures/make_ttc.py, pair.ttc
        ├── text.rs  render.rs  cache.rs  service.rs  http.rs
UX/
├── package.json                                # Sửa: react-window 2.3.4, devDeps test, scripts
├── vitest.config.ts  test/setup.ts             # Mới
└── features/library-grid/
    ├── types.ts            # FaceQuery, FaceRow, FacePage, parseFaceRow, parseFacePage, PX_BUCKETS, cssSize
    ├── api.ts              # LibraryApi, TauriCommandMap, makePreviewUrl, makeFontUrl, mock, makeRow
    ├── pagedFaces.ts       # PagedFaces, usePagedFaces
    ├── previewBlobs.ts     # PreviewBlobs, PreviewHttpError
    ├── fontFaceLoader.ts   # ScrollVelocity, FontFaceLoader, browserLoaderDeps
    ├── FaceCell.tsx  LibraryGrid.tsx  libraryGrid.css  index.ts
    ├── __tests__/ types.test.ts api.test.ts pagedFaces.test.ts previewBlobs.test.ts fontFaceLoader.test.ts LibraryGrid.test.tsx
    └── perf/ README.md vite.config.ts index.html bench.tsx wk.swift run_s2.py
docs/team/plans/plan5-report.md                 # Task 10
```

---

### Task 1: Crate, lỗi, chuẩn hoá chữ mẫu, chọn chữ dự phòng, phép co vừa canvas

**Files:**
- Modify: `UX/src-tauri/Cargo.toml` (`[workspace] members` thêm `"crates/fontatlas-preview"` sau core và store)
- Create: `UX/src-tauri/crates/fontatlas-preview/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/text.rs`, `src/render.rs` (chỉ `fit`, `truncate_at`, `require_fit`)
- Test: `tests/text.rs`, `tests/common/mod.rs`

**Interfaces:**
- Consumes: `fontatlas_core::CoreError` (`code()`).
- Produces:
  - Hằng số: `RENDERER_VERSION`, `PX_BUCKETS: [u32; 4]`, `DEFAULT_PX: u32 = 48`, `DEFAULT_SAMPLE`, `LATIN_FALLBACK`, `MAX_TEXT_CHARS: usize = 64`, `CMAP_SAMPLE_CHARS: usize = 12`
  - `pub enum PreviewError { BadSize(u32), BadRequest(String), NotFound, StaleFile, StaleGeneration, NoOutlines, NothingFits, TtcFace, Core(CoreError), Cache(rusqlite::Error), Encode(String) }`, kèm `code(&self) -> &'static str` và `http_status(&self) -> u16`
  - `pub fn check_px(px: u32) -> Result<u32, PreviewError>`
  - `pub fn canvas_size(px: u32) -> (u32, u32)`
  - `pub fn normalize_text(raw: &str) -> String`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)] #[serde(rename_all = "snake_case")] pub enum TextSource { Requested, LatinFallback, CmapSample, Nothing }`
  - `pub fn pick_text(requested: &str, has: impl Fn(char) -> bool, cmap_sample: impl FnOnce() -> String) -> (String, TextSource)`
  - `render::fit(ascent: f32, descent: f32, ink_top: f32, ink_bottom: f32, height: u32, vpad: f32) -> (f32, f32)`: trả `(k, baseline)`
  - `render::truncate_at(right_edges: &[f32], k: f32, width: u32, hpad: f32) -> usize`
  - `render::require_fit(keep: usize) -> Result<usize, PreviewError>`: `0` → `NothingFits`

- [ ] **Step 1: Tạo nhánh làm việc**

```bash
git switch -c feat/preview-grid dev/mvp
```

- [ ] **Step 2: Khai báo crate**

`UX/src-tauri/Cargo.toml`: `members = [".", "crates/fontatlas-core", "crates/fontatlas-store", "crates/fontatlas-preview"]`.

`UX/src-tauri/crates/fontatlas-preview/Cargo.toml`:
```toml
[package]
name = "fontatlas-preview"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
description = "Vẽ preview PNG alpha-mask cho lưới thư viện; cache SQLite LRU; handler custom protocol"

[dependencies]
fontatlas-core = { path = "../fontatlas-core" }
blake3 = "=1.8.7"
png = "=0.18.1"
rayon = "1.12"
read-fonts = "=0.45.1"
rusqlite = { version = "=0.31.0", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
skrifa = "=0.48.0"
thiserror = "2"
tiny-skia = { version = "=0.12.0", default-features = false, features = ["std", "simd"] }
unicode-normalization = "0.1.25"
url = "2.5"

[dev-dependencies]
serde_json = "1"
tempfile = "3.27"
```

- [ ] **Step 3: Viết test thất bại** — `tests/common/mod.rs` và `tests/text.rs`

`tests/common/mod.rs`:
```rust
#![allow(dead_code)]
use std::path::PathBuf;

/// Fixture dùng chung với core (không chép).
pub fn core_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fontatlas-core/tests/fixtures").join(name)
}
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}
pub fn file(p: &std::path::Path) -> fontatlas_core::FontFile {
    fontatlas_core::FontFile::open(p).unwrap()
}
pub fn rename_table(mut d: Vec<u8>, from: &[u8; 4], to: &[u8; 4]) -> Vec<u8> {
    let n = u16::from_be_bytes([d[4], d[5]]) as usize;
    for i in 0..n {
        let p = 12 + 16 * i;
        if &d[p..p + 4] == from { d[p..p + 4].copy_from_slice(to); }
    }
    d
}
```

`tests/text.rs`:
```rust
mod common;
use fontatlas_preview::*;

#[test]
fn normalize_text_nfc_trims_and_limits() {
    assert_eq!(normalize_text("  A\u{0306}\u{0301}n  "), "\u{1EAE}n"); // NFD → NFC "Ắn"
    assert_eq!(normalize_text("   "), DEFAULT_SAMPLE);
    assert_eq!(normalize_text(&"ơ".repeat(100)).chars().count(), MAX_TEXT_CHARS);
}

#[test]
fn canvas_size_and_buckets() {
    assert_eq!(canvas_size(48), (624, 112));
    assert_eq!(canvas_size(32), (416, 74));
    assert!(check_px(48).is_ok());
    assert_eq!(check_px(50).unwrap_err().code(), "bad_size");
    assert_eq!(check_px(50).unwrap_err().http_status(), 400);
}

#[test]
fn pick_text_keeps_requested_when_covered() {
    let (t, s) = pick_text("Ăn phở", |_| true, || unreachable!());
    assert_eq!((t.as_str(), s), ("Ăn phở", TextSource::Requested));
}

#[test]
fn pick_text_ignores_spaces_when_checking_coverage() {
    let (_, s) = pick_text("A B", |c| c != ' ', || unreachable!());
    assert_eq!(s, TextSource::Requested);
}

#[test]
fn pick_text_falls_back_to_latin() {
    let (t, s) = pick_text("Ăn phở", |c| c.is_ascii(), || unreachable!());
    assert_eq!((t.as_str(), s), (LATIN_FALLBACK, TextSource::LatinFallback));
}

#[test]
fn pick_text_uses_cmap_sample_for_non_latin_font() {
    let (t, s) = pick_text("Ăn phở", |c| !c.is_ascii(), || "αβγ".into());
    assert_eq!((t.as_str(), s), ("αβγ", TextSource::CmapSample));
    let (t, s) = pick_text("Ăn phở", |_| false, String::new);
    assert_eq!((t.as_str(), s), ("", TextSource::Nothing));
}

#[test]
fn fit_keeps_scale_when_ink_inside() {
    let (k, base) = render::fit(40.0, -10.0, 35.0, -8.0, 112, 4.0);
    assert_eq!((k, base), (1.0, 44.0));
}

#[test]
fn fit_shrinks_when_ink_exceeds_canvas() {
    // dấu chồng đưa mực lên 150 → co để (top - bottom)·k = 104
    let (k, base) = render::fit(40.0, -10.0, 150.0, -10.0, 112, 4.0);
    assert!((k - 104.0 / 160.0).abs() < 1e-5, "{k}");
    assert!((base - (4.0 + 150.0 * k)).abs() < 1e-4);
}

#[test]
fn truncate_at_never_keeps_partial_glyph() {
    // canvas 100, đệm 6 → mép phải tối đa 94
    assert_eq!(render::truncate_at(&[30.0, 60.0, 90.0, 120.0], 1.0, 100, 6.0), 2); // 6+90 = 96 > 94
    assert_eq!(render::truncate_at(&[30.0, 60.0], 0.5, 100, 6.0), 2);
    assert_eq!(render::truncate_at(&[200.0], 1.0, 100, 6.0), 0);
}

#[test]
fn nothing_fits_is_an_explicit_error() {
    let e = render::require_fit(0).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("nothing_fits", 422));
    assert_eq!(render::require_fit(3).unwrap(), 3);
}
```

- [ ] **Step 4: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test text`
Expected: FAIL, lỗi biên dịch `unresolved import fontatlas_preview`.

- [ ] **Step 5: Viết implementation**

- `lib.rs`:
  - Doc của crate nêu giới hạn: không shaping/kerning/bidi, dấu chỉ đúng với ký tự dựng sẵn, VF vẽ ở instance mặc định.
  - `mod error; mod text; pub mod render;`; các module của task sau thêm dần.
  - Re-export hằng số, `PreviewError`, các hàm của `text`.
- `error.rs`: `thiserror`, `#[from]` cho `CoreError` và `rusqlite::Error`. Mã và HTTP theo bảng ở Global Constraints. Lỗi `Core(e)` trả 404 khi `e.code() == "io"`, ngược lại 422.
- `text.rs`:
  - `normalize_text`: `nfc()` → `trim` → rỗng thì `DEFAULT_SAMPLE` → lấy tối đa `MAX_TEXT_CHARS` ký tự.
  - `canvas_size(px) = (13 * px, 7 * px / 3)`.
  - `pick_text` theo thứ tự:
    1. mọi ký tự không phải khoảng trắng qua `has` → `Requested`
    2. mọi ký tự của `LATIN_FALLBACK` qua `has` → `LatinFallback`
    3. `cmap_sample()` khác rỗng → `CmapSample`
    4. còn lại → `("", Nothing)`
- `render.rs`: đúng thuật toán sau (toạ độ font hướng y lên, đơn vị px ở cỡ `px`):

```rust
/// Co cho vừa: top/bottom là biên lớn nhất của metric và mực thật (dấu chồng có thể vượt ascent).
pub fn fit(ascent: f32, descent: f32, ink_top: f32, ink_bottom: f32, height: u32, vpad: f32) -> (f32, f32) {
    let top = ascent.max(ink_top);
    let bottom = descent.min(ink_bottom);
    let avail = height as f32 - 2.0 * vpad;
    let k = (avail / (top - bottom).max(1e-3)).min(1.0);
    (k, vpad + top * k) // baseline tính từ mép trên canvas (y hướng xuống)
}

/// Số glyph giữ lại: dừng trước glyph đầu tiên có mép phải vượt (width - hpad); không cắt giữa glyph.
pub fn truncate_at(right_edges: &[f32], k: f32, width: u32, hpad: f32) -> usize {
    let limit = width as f32 - hpad;
    right_edges.iter().take_while(|&&r| hpad + r * k <= limit).count()
}

pub fn require_fit(keep: usize) -> Result<usize, PreviewError> {
    if keep == 0 { Err(PreviewError::NothingFits) } else { Ok(keep) }
}
```

- [ ] **Step 6: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test text`
Expected: `ok. 10 passed`

- [ ] **Step 7: Commit**

```bash
git add UX/src-tauri/Cargo.toml UX/src-tauri/Cargo.lock UX/src-tauri/crates/fontatlas-preview
git commit -m "feat(preview): crate skeleton, sample text normalization, fallback, fit

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `render_face` — outline → tiny-skia → PNG alpha-mask, đúng face TTC

**Files:**
- Modify: `src/render.rs`, `src/lib.rs`
- Create: `tests/fixtures/make_ttc.py`, `tests/fixtures/pair.ttc`
- Test: `tests/render.rs`

**Interfaces:**
- Consumes: `fontatlas_core::{FontFile, Face}` (`Face::font() -> &FontRef`, `Face::index()`); `fit`, `truncate_at`, `require_fit`, `pick_text`, `normalize_text`, `canvas_size`, `check_px` (Task 1)
- Produces:
  - `#[derive(Debug, Clone)] pub struct Rendered { pub png: Vec<u8>, pub width: u32, pub height: u32, pub source: TextSource, pub text: String, pub truncated: bool }`
  - `pub fn render_face(face: &fontatlas_core::Face<'_>, text: &str, px: u32) -> Result<Rendered, PreviewError>`
  - `pub fn cmap_sample(font: &read_fonts::FontRef<'_>) -> String`

- [ ] **Step 1: Sinh fixture TTC trong venv tạm (giống cách Plan 1 sinh WOFF)**

`tests/fixtures/make_ttc.py`:
```python
"""Sinh pair.ttc: face 0 = roboto, face 1 = playfair_display. Cần fontTools."""
import pathlib
import sys

from fontTools.ttLib import TTFont
from fontTools.ttLib.ttCollection import TTCollection

core = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
c = TTCollection()
c.fonts = [TTFont(core / "roboto.ttf"), TTFont(core / "playfair_display.ttf")]
c.save(out / "pair.ttc")
```

```bash
python3 -m venv /tmp/fa-venv && /tmp/fa-venv/bin/pip install -q fonttools
cd UX/src-tauri/crates/fontatlas-preview
/tmp/fa-venv/bin/python tests/fixtures/make_ttc.py ../fontatlas-core/tests/fixtures tests/fixtures
```
`pair.ttc` được commit vào repo; CI không cần fontTools.

- [ ] **Step 2: Viết test thất bại** — `tests/render.rs`

```rust
mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_preview::*;

/// Giải mã PNG GrayscaleAlpha → (w, h, kênh alpha).
fn alpha(r: &Rendered) -> (u32, u32, Vec<u8>) {
    let dec = png::Decoder::new(std::io::Cursor::new(&r.png));
    let mut rd = dec.read_info().unwrap();
    let mut buf = vec![0; rd.output_buffer_size().unwrap()];
    let info = rd.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::GrayscaleAlpha);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    (info.width, info.height, buf[..info.buffer_size()].chunks(2).map(|p| p[1]).collect())
}
fn render(path: &std::path::Path, idx: u32, text: &str, px: u32) -> Rendered {
    let f = file(path);
    let faces = f.faces().unwrap();
    render_face(&faces[idx as usize], text, px).unwrap()
}

#[test]
fn png_has_bucket_size_and_ink() {
    for px in PX_BUCKETS {
        let r = render(&core_fixture("open_sans.ttf"), 0, DEFAULT_SAMPLE, px);
        let (w, h, a) = alpha(&r);
        assert_eq!((w, h), canvas_size(px));
        assert_eq!((r.width, r.height), (w, h));
        assert!(a.iter().any(|&v| v > 128), "phải có mực ở px {px}");
        assert_eq!(r.source, TextSource::Requested);
    }
}

#[test]
fn stacked_marks_are_not_clipped() {
    let r = render(&core_fixture("open_sans.ttf"), 0, "ỖẪẬ Ấp", 48);
    let (w, h, a) = alpha(&r);
    let row = |y: u32| &a[(y * w) as usize..((y + 1) * w) as usize];
    for y in [0, 1, h - 2, h - 1] {
        assert!(row(y).iter().all(|&v| v == 0), "hàng {y} phải trống (không cắt mép)");
    }
    assert!(a.iter().any(|&v| v > 128));
}

#[test]
fn ttc_face_one_matches_standalone() {
    let ttc = render(&fixture("pair.ttc"), 1, DEFAULT_SAMPLE, 48);
    let solo = render(&core_fixture("playfair_display.ttf"), 0, DEFAULT_SAMPLE, 48);
    let face0 = render(&fixture("pair.ttc"), 0, DEFAULT_SAMPLE, 48);
    assert_eq!(alpha(&ttc).2, alpha(&solo).2, "face 1 trong TTC phải vẽ đúng Playfair");
    assert_ne!(alpha(&ttc).2, alpha(&face0).2, "face 0 và face 1 phải khác nhau");
}

#[test]
fn missing_glyphs_fall_back_not_blank() {
    let r = render(&core_fixture("open_sans.ttf"), 0, "漢字かな", 48);
    assert_eq!((r.source, r.text.as_str()), (TextSource::LatinFallback, LATIN_FALLBACK));
    assert!(alpha(&r).2.iter().any(|&v| v > 128));
}

#[test]
fn cmap_sample_skips_controls_spaces_and_pua() {
    let f = file(&core_fixture("roboto.ttf"));
    let faces = f.faces().unwrap();
    let s = cmap_sample(faces[0].font());
    assert_eq!(s.chars().count(), CMAP_SAMPLE_CHARS);
    assert!(s.chars().all(|c| !c.is_control() && !c.is_whitespace() && !('\u{E000}'..='\u{F8FF}').contains(&c)), "{s:?}");
}

#[test]
fn long_text_is_truncated_without_partial_glyph() {
    let r = render(&core_fixture("roboto.ttf"), 0, &"W".repeat(60), 48);
    assert!(r.truncated);
    let (w, _, a) = alpha(&r);
    assert!(a.chunks(w as usize).all(|row| row[w as usize - 1] == 0), "vùng đệm phải không có mực");
}

#[test]
fn no_outline_font_is_no_outlines() {
    let raw = std::fs::read(core_fixture("roboto.ttf")).unwrap();
    let raw = rename_table(rename_table(raw, b"glyf", b"glyX"), b"loca", b"locX");
    let f = FontFile::from_bytes(raw).unwrap();
    let faces = f.faces().unwrap();
    let e = render_face(&faces[0], DEFAULT_SAMPLE, 48).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("no_outlines", 422));
}

#[test]
fn bad_size_rejected_before_render() {
    let f = file(&core_fixture("roboto.ttf"));
    let faces = f.faces().unwrap();
    assert_eq!(render_face(&faces[0], "A", 50).unwrap_err().code(), "bad_size");
}

#[test]
#[ignore = "hiệu năng: cargo test -p fontatlas-preview --release --test render -- --ignored --nocapture"]
fn render_under_1ms_per_face() {
    let files: Vec<FontFile> = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "libre_baskerville.ttf", "source_code_pro.ttf"]
        .iter().map(|n| file(&core_fixture(n))).collect();
    let faces: Vec<_> = files.iter().map(|f| f.faces().unwrap().remove(0)).collect();
    let n = 200;
    let t = std::time::Instant::now();
    for _ in 0..n {
        for f in &faces { render_face(f, DEFAULT_SAMPLE, 48).unwrap(); }
    }
    let per = t.elapsed().as_secs_f64() * 1e3 / (n * faces.len()) as f64;
    eprintln!("render_face: {per:.3} ms/face (đơn luồng)");
    assert!(per <= 1.0, "spec 7.1: ~0,67 ms, ngưỡng 1 ms");
}
```

- [ ] **Step 3: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test render`
Expected: FAIL, `cannot find function render_face`.

- [ ] **Step 4: Viết implementation**

`render_face(face, text, px)` theo thứ tự:

1. `check_px(px)?`; `(w, h) = canvas_size(px)`; `hpad = px as f32 / 8.0`; `vpad = (h as f32 / 28.0).max(2.0)`.
2. `font = face.font()`; `size = Size::new(px as f32)`; `loc = LocationRef::default()`; `cmap = font.charmap()`.
3. `(text, source) = pick_text(&normalize_text(text), |c| cmap.map(c).is_some(), || cmap_sample(font))`. `Nothing` → `Err(NoOutlines)`.
4. Dàn chữ: với mỗi ký tự map được:
   - vẽ outline (`DrawSettings::unhinted(size, loc)`) vào bộ thu `Vec<Seg>` (move/line/quad/cubic, toạ độ đã cộng `x` hiện tại);
   - cập nhật `ink_top`/`ink_bottom` từ **mọi điểm, kể cả điểm điều khiển** (hộp bao chặn trên, an toàn cho việc không cắt);
   - `right_edge = x + advance`, `x += advance`. Advance từ `glyph_metrics(size, loc).advance_width(gid)`, không có thì dùng `px * 0.5`.
5. Không có đoạn nào được vẽ → `Err(NoOutlines)`.
6. `m = font.metrics(size, loc)`; `(k, base) = fit(m.ascent, m.descent, ink_top, ink_bottom, h, vpad)`; `keep = require_fit(truncate_at(&right_edges, k, w, hpad))?`; `truncated = keep < số glyph`.
7. Đưa `keep` glyph đầu vào `tiny_skia::PathBuilder` với phép biến đổi `(x, y) ↦ (hpad + x·k, base − y·k)`. Fill `FillRule::Winding`, `anti_alias = true`, màu đen, alpha 255 vào `Pixmap::new(w, h)` (`None` → `Encode`).
8. Kênh alpha → `[0, a]` cho mỗi pixel → `png::Encoder` (`GrayscaleAlpha`, `Eight`, `Compression::Fast`). Lỗi → `Encode(e.to_string())`.

`cmap_sample`: duyệt `font.charmap().mappings()` theo thứ tự codepoint, giữ ký tự hợp lệ, `>= U+0021`, không phải control/khoảng trắng, không thuộc PUA `E000–F8FF`, tối đa `CMAP_SAMPLE_CHARS` ký tự.

- [ ] **Step 5: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test render`
Expected: `ok. 8 passed; 1 ignored`

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --release --test render -- --ignored --nocapture`
Expected: dòng `render_face: 0.xxx ms/face`, test PASS. Spike đo được 0,67 ms/face, gồm cả đọc file.

- [ ] **Step 6: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-preview
git commit -m "feat(preview): render_face to grayscale+alpha PNG with fit-to-canvas and TTC faces

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: `PreviewCache` — SQLite, khoá BLAKE3, hai vùng LRU theo byte

**Files:**
- Create: `src/cache.rs`
- Modify: `src/lib.rs`
- Test: `tests/cache.rs`

**Interfaces:**
- Consumes: `normalize_text`, `RENDERER_VERSION`, `DEFAULT_SAMPLE`, `PreviewError`
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct CacheKey(pub [u8; 32])`
  - `pub fn cache_key(hash_hex: &str, face_index: u32, px: u32, text: &str) -> CacheKey`: chuẩn hoá `text` trước khi băm
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Pool { Default, Custom }`
  - `pub fn pool_for(text: &str) -> Pool`: `Default` khi `normalize_text(text) == DEFAULT_SAMPLE`
  - `#[derive(Debug, Clone, Copy)] pub struct Budgets { pub default_bytes: u64, pub custom_bytes: u64 }`, `impl Budgets { pub const DEFAULT: Budgets = Budgets { default_bytes: 64 << 20, custom_bytes: 32 << 20 }; }`
  - `pub struct PreviewCache`
  - `PreviewCache::open(path: &Path, budgets: Budgets) -> Result<PreviewCache, PreviewError>`
  - `get(&mut self, k: &CacheKey) -> Result<Option<Vec<u8>>, PreviewError>`: lần trúng làm mới thứ tự LRU
  - `put(&mut self, k: &CacheKey, pool: Pool, png: &[u8]) -> Result<(), PreviewError>`
  - `total_bytes(&self, pool: Pool) -> u64`, `len(&self) -> Result<u64, PreviewError>`

- [ ] **Step 1: Viết test thất bại** — `tests/cache.rs`

```rust
use fontatlas_preview::*;

fn k(i: u32) -> CacheKey { cache_key(&"ab".repeat(32), i, 48, "Ăn") }
fn b(d: u64, c: u64) -> Budgets { Budgets { default_bytes: d, custom_bytes: c } }

#[test]
fn round_trip_and_miss() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("previews.db"), Budgets::DEFAULT).unwrap();
    assert_eq!(c.get(&k(0)).unwrap(), None);
    c.put(&k(0), Pool::Custom, b"png0").unwrap();
    assert_eq!(c.get(&k(0)).unwrap().as_deref(), Some(&b"png0"[..]));
    assert_eq!((c.total_bytes(Pool::Custom), c.total_bytes(Pool::Default)), (4, 0));
}

#[test]
fn key_covers_every_component_and_folds_nfc() {
    let h = "ab".repeat(32);
    let nfc = cache_key(&h, 0, 48, "\u{1EAE}n");          // "Ắn" dựng sẵn
    let nfd = cache_key(&h, 0, 48, "A\u{0306}\u{0301}n"); // cùng chữ, dạng tách dấu
    assert_eq!(nfc, nfd, "NFD và NFC cùng khoá");
    assert_ne!(nfc, cache_key(&"cd".repeat(32), 0, 48, "\u{1EAE}n"));
    assert_ne!(nfc, cache_key(&h, 1, 48, "\u{1EAE}n"));
    assert_ne!(nfc, cache_key(&h, 0, 64, "\u{1EAE}n"));
    assert_ne!(nfc, cache_key(&h, 0, 48, "\u{1EAE}m"));
}

#[test]
fn pool_for_default_sample_only() {
    assert_eq!(pool_for(DEFAULT_SAMPLE), Pool::Default);
    assert_eq!(pool_for("  "), Pool::Default); // rỗng → mặc định
    assert_eq!(pool_for("Khác"), Pool::Custom);
}

#[test]
fn eviction_keeps_total_under_budget() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), b(1 << 20, 10_000)).unwrap();
    for i in 0..3 { c.put(&k(i), Pool::Custom, &vec![1u8; 3000]).unwrap(); }
    assert!(c.get(&k(0)).unwrap().is_some()); // làm mới k(0)
    for i in 3..5 { c.put(&k(i), Pool::Custom, &vec![1u8; 3000]).unwrap(); }
    assert!(c.total_bytes(Pool::Custom) <= 10_000);
    assert!(c.get(&k(0)).unwrap().is_some(), "vừa dùng → giữ");
    assert!(c.get(&k(1)).unwrap().is_none(), "cũ nhất → bị đẩy");
    assert!(c.get(&k(4)).unwrap().is_some());
}

#[test]
fn custom_churn_keeps_default_pool() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), b(10_000, 1_000)).unwrap();
    c.put(&k(0), Pool::Default, &[1; 500]).unwrap();
    for i in 1..50 { c.put(&k(i), Pool::Custom, &[2; 400]).unwrap(); }
    assert!(c.get(&k(0)).unwrap().is_some(), "chữ tuỳ chỉnh không được đẩy chữ mặc định");
    assert!(c.total_bytes(Pool::Custom) <= 1_000);
}

#[test]
fn oversized_entry_not_stored() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), b(1 << 20, 1000)).unwrap();
    c.put(&k(0), Pool::Custom, &vec![1u8; 500]).unwrap();
    c.put(&k(1), Pool::Custom, &vec![1u8; 5000]).unwrap();
    assert!(c.get(&k(1)).unwrap().is_none());
    assert!(c.get(&k(0)).unwrap().is_some(), "entry quá lớn không được đẩy entry khác");
}

#[test]
fn put_same_key_replaces_and_recounts() {
    let d = tempfile::tempdir().unwrap();
    let mut c = PreviewCache::open(&d.path().join("p.db"), Budgets::DEFAULT).unwrap();
    c.put(&k(0), Pool::Custom, &[1; 100]).unwrap();
    c.put(&k(0), Pool::Custom, &[2; 40]).unwrap();
    assert_eq!(c.total_bytes(Pool::Custom), 40);
    assert_eq!(c.len().unwrap(), 1);
}

#[test]
fn reopen_keeps_entries_and_totals_and_applies_smaller_budget() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("p.db");
    { let mut c = PreviewCache::open(&p, Budgets::DEFAULT).unwrap(); for i in 0..10 { c.put(&k(i), Pool::Default, &[1; 100]).unwrap(); } }
    let mut c = PreviewCache::open(&p, Budgets::DEFAULT).unwrap();
    assert_eq!(c.total_bytes(Pool::Default), 1000);
    assert!(c.get(&k(9)).unwrap().is_some());
    drop(c);
    let c = PreviewCache::open(&p, b(450, 450)).unwrap();
    assert!(c.total_bytes(Pool::Default) <= 450);
}

#[test]
fn corrupt_cache_is_renamed_and_recreated() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("previews.db");
    std::fs::write(&p, b"rac rac rac rac rac rac").unwrap();
    let mut c = PreviewCache::open(&p, Budgets::DEFAULT).unwrap();
    assert_eq!(c.len().unwrap(), 0);
    c.put(&k(0), Pool::Custom, b"x").unwrap();
    assert!(c.get(&k(0)).unwrap().is_some());
    let broken: Vec<_> = std::fs::read_dir(d.path()).unwrap()
        .filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("previews.broken-") && n.ends_with(".db")).collect();
    assert_eq!(broken.len(), 1, "{broken:?}");
}
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test cache`
Expected: FAIL, `cannot find type PreviewCache`.

- [ ] **Step 3: Viết implementation** — `src/cache.rs`

- `cache_key`: `blake3::hash` của `format!("{RENDERER_VERSION}\0{hash_hex}\0{face_index}\0{px}\0{}", normalize_text(text))`.
- `open`:
  - Mở `Connection`, đặt `journal_mode=WAL`, `synchronous=NORMAL`, chạy DDL:

    ```sql
    CREATE TABLE IF NOT EXISTS previews(
      id INTEGER PRIMARY KEY,
      key BLOB NOT NULL UNIQUE,
      pool INTEGER NOT NULL,      -- 0 mặc định, 1 tuỳ chỉnh
      png BLOB NOT NULL,
      bytes INTEGER NOT NULL,
      tick INTEGER NOT NULL);
    CREATE INDEX IF NOT EXISTS previews_lru ON previews(pool, tick);
    ```

  - Đọc tổng `bytes` theo `pool` và `tick = coalesce(max(tick), 0)`.
  - Bất kỳ bước nào lỗi (file rác → `SQLITE_NOTADB`): đóng kết nối, đổi tên `path`, `path-wal`, `path-shm` (nếu có) thành `previews.broken-{unix_ms}.db[-wal|-shm]`, mở lại **một lần**. Lần hai vẫn lỗi → `Err(Cache)`.
  - Cuối `open`: gọi `evict` cho từng vùng vượt ngân sách.
- **Thứ tự LRU dùng bộ đếm `tick` tăng dần**, không dùng đồng hồ, để test tất định. `get` tăng `tick` và `UPDATE … SET tick=? WHERE key=?`.
- `put`:
  - `png.len() > ngân sách của pool` → bỏ qua, `Ok(())`.
  - Ngược lại: trong một transaction, đọc `(pool, bytes)` cũ của `key` nếu có, rồi `INSERT ... ON CONFLICT(key) DO UPDATE SET pool=excluded.pool, png=excluded.png, bytes=excluded.bytes, tick=excluded.tick`. Cập nhật tổng của pool cũ và pool mới.
  - Pool vượt ngân sách → `evict(pool)`.
- `evict(pool)`: trong một transaction, lặp `SELECT id, bytes FROM previews WHERE pool=? ORDER BY tick LIMIT 64` và xoá cho tới khi tổng ≤ `ngân sách * 9 / 10`.

- [ ] **Step 4: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test cache`
Expected: `ok. 9 passed`

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-preview
git commit -m "feat(preview): SQLite preview cache with per-pool byte LRU and rename-on-corrupt

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `PreviewService` — thế hệ do backend sở hữu, kiểm hash, cache suy giảm an toàn, lô theo viewport

**Files:**
- Create: `src/service.rs`
- Modify: `src/lib.rs`
- Test: `tests/service.rs`

**Interfaces:**
- Consumes: `PreviewCache`, `Pool`, `pool_for`, `cache_key`, `render_face`, `check_px`, `fontatlas_core::FontFile`
- Produces:
  - `pub trait FaceLocator: Send + Sync { fn locate(&self, hash_hex: &str, face_index: u32) -> Option<std::path::PathBuf>; }`: Plan 6 cài bằng `Store` (vị trí `missing = 0` đầu tiên)
  - `#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)] #[serde(rename_all = "camelCase")] pub struct PreviewRequest { pub hash_hex: String, pub face_index: u32, pub px: u32, pub text: String, pub generation: Option<u64> }`
  - `pub struct PreviewService<L: FaceLocator>` (`Send + Sync`)
  - `PreviewService::new(cache: PreviewCache, locator: L) -> Self`
  - `bump_generation(&self) -> u64`, `generation(&self) -> u64`
  - `locator(&self) -> &L`
  - `get(&self, req: &PreviewRequest) -> Result<Vec<u8>, PreviewError>`
  - `prewarm(&self, reqs: &[PreviewRequest]) -> Vec<Result<(), PreviewError>>`
  - `render_count(&self) -> u64`, `file_reads(&self) -> u64`

- [ ] **Step 1: Viết test thất bại** — `tests/service.rs`

```rust
mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_preview::*;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

struct MapLocator(HashMap<(String, u32), PathBuf>);
impl FaceLocator for MapLocator {
    fn locate(&self, h: &str, i: u32) -> Option<PathBuf> { self.0.get(&(h.to_string(), i)).cloned() }
}
fn hash_of(p: &std::path::Path) -> String { FontFile::open(p).unwrap().hash_hex() }
fn svc_with(budgets: Budgets, entries: &[(PathBuf, u32)]) -> (tempfile::TempDir, PreviewService<MapLocator>) {
    let d = tempfile::tempdir().unwrap();
    let cache = PreviewCache::open(&d.path().join("p.db"), budgets).unwrap();
    let map = entries.iter().map(|(p, i)| ((hash_of(p), *i), p.clone())).collect();
    (d, PreviewService::new(cache, MapLocator(map)))
}
fn svc(entries: &[(PathBuf, u32)]) -> (tempfile::TempDir, PreviewService<MapLocator>) { svc_with(Budgets::DEFAULT, entries) }
fn req(p: &std::path::Path, i: u32, g: Option<u64>) -> PreviewRequest {
    PreviewRequest { hash_hex: hash_of(p), face_index: i, px: 48, text: "Ăn phở".into(), generation: g }
}

#[test]
fn get_renders_once_then_hits_cache() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0)]);
    let a = s.get(&req(&p, 0, None)).unwrap();
    let b = s.get(&req(&p, 0, None)).unwrap();
    assert_eq!(a, b);
    assert_eq!(s.render_count(), 1);
    assert_eq!(&a[1..4], b"PNG");
}

#[test]
fn bump_generation_is_monotonic() {
    let (_d, s) = svc(&[]);
    assert_eq!(s.generation(), 0);
    assert_eq!(s.bump_generation(), 1);
    assert_eq!(s.bump_generation(), 2);
    assert_eq!(s.generation(), 2);
}

#[test]
fn stale_generation_rejected_before_render() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0)]);
    for _ in 0..5 { s.bump_generation(); }
    let e = s.get(&req(&p, 0, Some(4))).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("stale_generation", 410));
    assert_eq!(s.render_count(), 0);
    assert!(s.get(&req(&p, 0, Some(5))).is_ok());
    assert!(s.get(&req(&p, 0, Some(6))).is_ok(), "thế hệ mới hơn (đã xin nhưng chưa thấy) vẫn hợp lệ");
    assert!(s.get(&req(&p, 0, None)).is_ok(), "không có g → không kiểm thế hệ");
}

#[test]
fn unknown_face_is_not_found() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[]);
    assert_eq!(s.get(&req(&p, 0, None)).unwrap_err().code(), "not_found");
    let (_d2, s2) = svc(&[(p.clone(), 7)]); // locator trả path nhưng file chỉ có face 0
    let mut r = req(&p, 0, None);
    r.face_index = 7;
    assert_eq!(s2.get(&r).unwrap_err().code(), "not_found");
}

#[test]
fn changed_file_is_stale_file() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("f.ttf");
    std::fs::copy(core_fixture("roboto.ttf"), &path).unwrap();
    let (_d, s) = svc(&[(path.clone(), 0)]);
    let r = req(&path, 0, None);
    std::fs::copy(core_fixture("open_sans.ttf"), &path).unwrap(); // file đổi sau khi quét
    let e = s.get(&r).unwrap_err();
    assert_eq!((e.code(), e.http_status()), ("stale_file", 409));
}

#[test]
fn prewarm_reads_each_file_once_and_fills_cache() {
    let p = fixture("pair.ttc");
    let (_d, s) = svc(&[(p.clone(), 0), (p.clone(), 1)]);
    let rs = s.prewarm(&[req(&p, 0, None), req(&p, 1, None)]);
    assert!(!rs.is_empty() && rs.iter().all(|r| r.is_ok()));
    assert_eq!((s.file_reads(), s.render_count()), (1, 2));
    s.get(&req(&p, 1, None)).unwrap();
    assert_eq!(s.render_count(), 2, "đã có trong cache");
}

#[test]
fn prewarm_skips_cached_and_stale_and_keeps_order() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0)]);
    s.get(&req(&p, 0, None)).unwrap();
    for _ in 0..9 { s.bump_generation(); }
    let rs = s.prewarm(&[req(&p, 0, None), req(&p, 0, Some(1))]);
    assert!(rs[0].is_ok());
    assert_eq!(rs[1].as_ref().unwrap_err().code(), "stale_generation");
    assert_eq!((s.render_count(), s.file_reads()), (1, 1));
}

#[test]
fn tiny_budget_still_serves() {
    let p = core_fixture("roboto.ttf");
    let (_d, s) = svc_with(Budgets { default_bytes: 1, custom_bytes: 1 }, &[(p.clone(), 0)]);
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert_eq!(s.render_count(), 2, "cache không giữ được → render lại, vẫn phục vụ");
}

#[test]
fn cache_errors_degrade_to_render() {
    let p = core_fixture("roboto.ttf");
    let (d, s) = svc(&[(p.clone(), 0)]);
    // làm hỏng cache lúc đang chạy: bảng biến mất dưới chân service
    rusqlite::Connection::open(d.path().join("p.db")).unwrap().execute_batch("DROP TABLE previews;").unwrap();
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert!(s.get(&req(&p, 0, None)).is_ok());
    assert_eq!(s.render_count(), 2);
}

#[test]
fn concurrent_get_and_prewarm() {
    let p = fixture("pair.ttc");
    let q = core_fixture("roboto.ttf");
    let (_d, s) = svc(&[(p.clone(), 0), (p.clone(), 1), (q.clone(), 0)]);
    let s = Arc::new(s);
    let reqs = vec![req(&p, 0, None), req(&p, 1, None), req(&q, 0, None)];
    let handles: Vec<_> = (0..8).map(|t| {
        let (s, reqs) = (Arc::clone(&s), reqs.clone());
        std::thread::spawn(move || {
            if t % 2 == 0 { s.prewarm(&reqs).into_iter().all(|r| r.is_ok()) }
            else { reqs.iter().map(|r| s.get(r).map(|png| png.len())).all(|r| r.is_ok()) }
        })
    }).collect();
    assert!(handles.into_iter().all(|h| h.join().unwrap()));
    let a = s.get(&reqs[1]).unwrap();
    assert_eq!(a, s.get(&reqs[1]).unwrap());
}
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test service`
Expected: FAIL, `cannot find trait FaceLocator`.

- [ ] **Step 3: Viết implementation** — `src/service.rs`

- Trường:
  - `cache: Mutex<PreviewCache>`
  - `locator: L`
  - `generation: AtomicU64`: `bump_generation` = `fetch_add(1) + 1`
  - `renders`, `reads: AtomicU64`
  - **Không giữ lock của cache trong lúc đọc file hay render.** Mutex bị poison → coi như miss / bỏ qua ghi (dùng `lock().ok()`).
- `get(req)`:
  1. `req.generation` có giá trị và `< generation()` → `StaleGeneration`.
  2. `check_px`; `hash_hex` phải 64 ký tự hex thường, ngược lại `BadRequest`. Tính `key`, `pool = pool_for(&req.text)`.
  3. `cache.get`: trúng → trả kết quả. Lỗi → `eprintln!("preview cache get: {e}")` rồi coi như miss.
  4. Gọi `locate` (`None` → `NotFound`).
  5. `FontFile::open` (tăng `reads`).
  6. `hash_hex()` khác → `StaleFile`.
  7. Face theo `index()`, không có → `NotFound`.
  8. `render_face` (tăng `renders`), rồi `cache.put(key, pool, png)`. Lỗi `put` chỉ log; vẫn trả PNG.
- `prewarm(reqs)`:
  1. Bước lọc trước (thế hệ, `BadRequest`, cache trúng) giống `get`.
  2. Phần còn lại nhóm theo `hash_hex`, chạy `rayon::par_iter` theo nhóm: `locate` theo request đầu, `FontFile::open` một lần, kiểm hash, render từng face, `put`.
  3. Trả kết quả theo **thứ tự đầu vào**.

- [ ] **Step 4: Chạy test, xác nhận qua**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test service`
Expected: `ok. 10 passed`

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-preview
git commit -m "feat(preview): PreviewService with backend-owned generations, hash check, degrading cache

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Handler custom protocol (preview + font) có CORS, ví dụ `dump_previews`, clippy

**Files:**
- Create: `src/http.rs`, `examples/dump_previews.rs`
- Modify: `src/lib.rs`
- Test: `tests/http.rs`

**Interfaces:**
- Consumes: `PreviewService`, `PreviewRequest`, `FaceLocator`, `PreviewError::http_status`, `fontatlas_core::FontFile`
- Produces:
  - `pub const PREVIEW_SCHEME: &str = "fontatlas-preview"`, `pub const FONT_SCHEME: &str = "fontatlas-font"`
  - `#[derive(Debug, Clone)] pub struct CorsPolicy { pub allowed: Vec<String> }`, kèm `CorsPolicy::app() -> CorsPolicy` và `CorsPolicy::with(mut self, origin: &str) -> CorsPolicy`
  - `#[derive(Debug)] pub struct PreviewHttp { pub status: u16, pub content_type: &'static str, pub cache_control: &'static str, pub allow_origin: Option<String>, pub vary: &'static str, pub body: Vec<u8> }`
  - `pub fn parse_preview_uri(uri: &str) -> Result<PreviewRequest, PreviewError>`
  - `pub fn handle_preview_uri<L: FaceLocator>(svc: &PreviewService<L>, uri: &str, origin: Option<&str>, cors: &CorsPolicy) -> PreviewHttp`
  - `pub fn parse_font_uri(uri: &str) -> Result<(String, u32), PreviewError>`
  - `pub fn handle_font_uri<L: FaceLocator>(locator: &L, uri: &str, origin: Option<&str>, cors: &CorsPolicy) -> PreviewHttp`
  - Ví dụ `dump_previews`:
    - `dump_previews --out <dir> <font dir>...` ghi `<dir>/<id>.png` và `<dir>/manifest.json` = `[{ "id": u32, "path": String, "faceIndex": u32, "family": String, "hashHex": String }]`
    - `dump_previews one <path> <face_index> <px> <text>` in PNG ra stdout

- [ ] **Step 1: Viết test thất bại** — `tests/http.rs`

```rust
mod common;
use common::*;
use fontatlas_core::FontFile;
use fontatlas_preview::*;
use std::path::PathBuf;

struct One(String, PathBuf);
impl FaceLocator for One {
    fn locate(&self, h: &str, _i: u32) -> Option<PathBuf> { (h == self.0).then(|| self.1.clone()) }
}
fn setup(p: PathBuf) -> (tempfile::TempDir, PreviewService<One>, String) {
    let h = FontFile::open(&p).unwrap().hash_hex();
    let d = tempfile::tempdir().unwrap();
    let c = PreviewCache::open(&d.path().join("p.db"), Budgets::DEFAULT).unwrap();
    (d, PreviewService::new(c, One(h.clone(), p)), h)
}
const APP: Option<&str> = Some("tauri://localhost");

#[test]
fn parses_macos_and_windows_forms() {
    let h = "ab".repeat(32);
    for base in ["fontatlas-preview://localhost/", "http://fontatlas-preview.localhost/"] {
        let r = parse_preview_uri(&format!("{base}{h}_2_64.png?t=%C4%82n%20ph%E1%BB%9F&g=7&r=preview-0.1.0")).unwrap();
        assert_eq!(r, PreviewRequest { hash_hex: h.clone(), face_index: 2, px: 64, text: "Ăn phở".into(), generation: Some(7) });
    }
    let r = parse_preview_uri(&format!("fontatlas-preview://localhost/{h}_0_48.png")).unwrap();
    assert_eq!((r.text.as_str(), r.generation), (DEFAULT_SAMPLE, None));
    assert_eq!(parse_font_uri(&format!("http://fontatlas-font.localhost/{h}_0.font")).unwrap(), (h.clone(), 0));
}

#[test]
fn bad_uris_are_400() {
    let h = "ab".repeat(32);
    for u in [
        format!("fontatlas-preview://localhost/{h}_0_50.png"),
        format!("fontatlas-preview://localhost/{h}_0_48.jpg"),
        "fontatlas-preview://localhost/zz_0_48.png".to_string(),
        format!("fontatlas-preview://localhost/{h}_x_48.png"),
        format!("fontatlas-preview://localhost/{h}_0_48.png?g=abc"),
        "không phải url".to_string(),
    ] {
        assert_eq!(parse_preview_uri(&u).unwrap_err().http_status(), 400, "{u}");
    }
    assert_eq!(parse_font_uri(&format!("fontatlas-font://localhost/{h}.font")).unwrap_err().http_status(), 400);
}

#[test]
fn preview_ok_with_cors_for_app_origin_only() {
    let (_d, s, h) = setup(core_fixture("roboto.ttf"));
    let uri = format!("fontatlas-preview://localhost/{h}_0_48.png?t=Ab");
    let ok = handle_preview_uri(&s, &uri, APP, &CorsPolicy::app());
    assert_eq!((ok.status, ok.content_type, ok.cache_control), (200, "image/png", "private, max-age=86400"));
    assert_eq!(ok.allow_origin.as_deref(), Some("tauri://localhost"));
    assert_eq!(ok.vary, "Origin");
    assert_eq!(&ok.body[1..4], b"PNG");
    assert_eq!(handle_preview_uri(&s, &uri, Some("https://evil.example"), &CorsPolicy::app()).allow_origin, None);
    assert_eq!(handle_preview_uri(&s, &uri, None, &CorsPolicy::app()).allow_origin, None);
    let dev = CorsPolicy::app().with("http://localhost:3000");
    assert_eq!(handle_preview_uri(&s, &uri, Some("http://localhost:3000"), &dev).allow_origin.as_deref(), Some("http://localhost:3000"));
}

#[test]
fn preview_errors_map_to_http() {
    let (_d, s, h) = setup(core_fixture("roboto.ttf"));
    let nf = handle_preview_uri(&s, &format!("fontatlas-preview://localhost/{}_0_48.png", "cd".repeat(32)), APP, &CorsPolicy::app());
    assert_eq!((nf.status, nf.content_type, nf.cache_control), (404, "text/plain; charset=utf-8", "no-store"));
    assert_eq!(String::from_utf8(nf.body).unwrap(), "not_found");
    assert_eq!(nf.allow_origin.as_deref(), Some("tauri://localhost"), "lỗi cũng cần CORS để fetch đọc được mã");
    for _ in 0..3 { s.bump_generation(); }
    assert_eq!(handle_preview_uri(&s, &format!("fontatlas-preview://localhost/{h}_0_48.png?g=2"), APP, &CorsPolicy::app()).status, 410);
}

#[test]
fn font_uri_serves_file_with_type_and_cors() {
    let (_d, s, h) = setup(core_fixture("roboto.ttf"));
    let r = handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{h}_0.font"), APP, &CorsPolicy::app());
    assert_eq!((r.status, r.content_type), (200, "font/ttf"));
    assert_eq!(r.body, std::fs::read(core_fixture("roboto.ttf")).unwrap());
    assert_eq!(r.allow_origin.as_deref(), Some("tauri://localhost"));
    let (_d2, s2, h2) = setup(fixture("pair.ttc"));
    let c = handle_font_uri(s2.locator(), &format!("fontatlas-font://localhost/{h2}_0.font"), APP, &CorsPolicy::app());
    assert_eq!((c.status, c.content_type), (200, "font/collection"));
}

#[test]
fn font_uri_refuses_ttc_face() {
    let (_d, s, h) = setup(fixture("pair.ttc"));
    let r = handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{h}_1.font"), APP, &CorsPolicy::app());
    assert_eq!(r.status, 422);
    assert_eq!(String::from_utf8(r.body).unwrap(), "ttc_face");
}

#[test]
fn font_uri_not_found_and_stale() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("f.ttf");
    std::fs::copy(core_fixture("roboto.ttf"), &path).unwrap();
    let (_d, s, h) = setup(path.clone());
    assert_eq!(handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{}_0.font", "cd".repeat(32)), APP, &CorsPolicy::app()).status, 404);
    std::fs::copy(core_fixture("open_sans.ttf"), &path).unwrap();
    assert_eq!(handle_font_uri(s.locator(), &format!("fontatlas-font://localhost/{h}_0.font"), APP, &CorsPolicy::app()).status, 409);
}
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX/src-tauri && cargo test -p fontatlas-preview --test http`
Expected: FAIL, `cannot find function parse_preview_uri`.

- [ ] **Step 3: Viết implementation**

- Phân tích URL (dùng chung cho hai scheme):
  1. `url::Url::parse` (lỗi → `BadRequest`).
  2. Lấy `path_segments().last()` và percent-decode.
  3. Preview: tên phải kết thúc `.png`; phần còn lại tách `_` thành đúng 3 phần: `hash` (64 hex thường), `face_index: u32`, `px` qua `check_px`. `t` → `normalize_text` (thiếu → `DEFAULT_SAMPLE`). `g` → `u64` (có nhưng sai → `BadRequest`). `r` bỏ qua.
  4. Font: tên `.font`, tách thành 2 phần `hash` và `face_index`.
- `CorsPolicy::app()`: ba origin ở Global Constraints. `allow_origin` = `origin` nếu nằm trong `allowed` (so khớp nguyên chuỗi), ngược lại `None`. Áp dụng cho **cả** phản hồi lỗi.
- `handle_preview_uri`:
  - Thành công → 200, `image/png`, `private, max-age=86400`.
  - Lỗi → `e.http_status()`, `text/plain; charset=utf-8`, `no-store`, `body = e.code()`.
  - `vary = "Origin"`.
- `handle_font_uri`:
  1. Parse; `face_index > 0` → `TtcFace`.
  2. `locate` → `NotFound`. `std::fs::read` (`NotFound` khi lỗi io).
  3. `blake3::hash(&raw).to_hex() != hash` → `StaleFile`.
  4. Content-type theo 4 byte đầu: `ttcf` → `font/collection`, `OTTO` → `font/otf`, `wOFF` → `font/woff`, `wOF2` → `font/woff2`, còn lại `font/ttf`.
  5. Trả nguyên byte, `private, max-age=86400`.
- `examples/dump_previews.rs`:
  - Duyệt thư mục đệ quy lấy `ttf/otf/ttc/otc/woff/woff2`.
  - `FontFile::open` → `faces()` → `render_face(face, DEFAULT_SAMPLE, 48)`, chạy song song bằng `rayon`. Lỗi thì bỏ qua và đếm.
  - Ghi PNG + manifest, in ra stderr: tổng face, số lỗi, ms/face.
  - Lệnh `one` in PNG của một face ra stdout.

- [ ] **Step 4: Chạy test, clippy, ví dụ**

Run: `cd UX/src-tauri && cargo test -p fontatlas-core -p fontatlas-store -p fontatlas-preview && cargo clippy -p fontatlas-core -p fontatlas-store -p fontatlas-preview --all-targets -- -D warnings`
Expected: mọi test qua. Riêng preview: `text` 10, `render` 8 + 1 ignored, `cache` 9, `service` 10, `http` 7. Clippy không cảnh báo.

Run (macOS): `cd UX/src-tauri && cargo run --release -p fontatlas-preview --example dump_previews -- --out /tmp/fa-previews /System/Library/Fonts /Library/Fonts`
Expected: khoảng 780 face. Lỗi ≤ 10 (emoji/bitmap → `no_outlines`). `du -sh /tmp/fa-previews` ≈ 5 MB.

- [ ] **Step 5: Commit**

```bash
git add UX/src-tauri/crates/fontatlas-preview
git commit -m "feat(preview): preview and font protocol handlers with app-origin CORS; dump_previews

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Hạ tầng test frontend, kiểu dữ liệu + bộ đọc hợp đồng, API, mock

**Files:**
- Modify: `UX/package.json`, `UX/tsconfig.json` (thêm `"resolveJsonModule": true` vào `compilerOptions` để test import fixture JSON)
- Create: `UX/vitest.config.ts`, `UX/test/setup.ts`, `UX/features/library-grid/types.ts`, `api.ts`, `index.ts`
- Test: `__tests__/types.test.ts`, `__tests__/api.test.ts`

**Interfaces:**
- Consumes: fixture `UX/src-tauri/crates/fontatlas-store/tests/fixtures/face_row.json` (Plan 2; sinh từ serde thật)
- Produces (`types.ts`):
  - `FaceQuery` (gương `fontatlas-store` Task 5, camelCase):

    ```ts
    export type FaceQuery = {
      text?: string; vi?: 'full' | 'supported'; license?: string[];
      variable?: boolean; italic?: boolean; sourceIds?: number[];
      favoritesOnly?: boolean; collectionId?: number; tagIds?: number[];
      includeMissing?: boolean; sort?: 'family' | 'weight' | 'added';
      descending?: boolean; offset?: number; limit?: number;
    }
    ```

  - `FaceRow`: gương **đúng** `fontatlas_store::FaceRow` trên `dev/mvp`:

    ```ts
    export type FaceRow = {
      faceId: number; hashHex: string; faceIndex: number; family: string;
      subfamily: string | null; fullName: string | null;
      weightClass: number | null; widthClass: number | null;
      isItalic: boolean; isVariable: boolean; viStatus: string;
      supportsVi: boolean; viMissing: number; licenseClass: string;
      path: string | null; fileSize: number; missing: boolean;
      isFavorite: boolean; addedAt: number;
    }
    ```

  - `export type FacePage = { total: number; offset: number; items: FaceRow[] }`
  - `export type GridQuery = Omit<FaceQuery, 'offset' | 'limit'>`
  - `export class ContractError extends Error {}`
  - `export function parseFaceRow(x: unknown, path?: string): FaceRow` và `export function parseFacePage(x: unknown): FacePage`: kiểm từng trường và kiểu (`null` chỉ ở trường `Option`). **Khoá thiếu, sai kiểu hoặc thừa** → `ContractError` kèm đường dẫn trường.
  - `export const PX_BUCKETS = [32, 48, 64, 96] as const; export type PxBucket = typeof PX_BUCKETS[number]`
  - `export const DEFAULT_SAMPLE = 'Hamburgefonts Ăn phở'`
  - `export function cssSize(px: PxBucket): { width: number; height: number }` = `{ width: 13*px/2, height: Math.floor(7*px/3)/2 }`
- Produces (`api.ts`):
  - `export type PreviewInfo = { rendererVersion: string; generation: number; buckets: number[] }`
  - `LibraryApi`:

    ```ts
    export interface LibraryApi {
      queryFaces(q: FaceQuery): Promise<FacePage>;
      previewInfo(): Promise<PreviewInfo>;
      bumpGeneration(): Promise<number>;
      previewUrl(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>, o: { text: string; px: PxBucket; generation: number; rendererVersion: string }): string;
      fontUrl(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string | null;
    }
    ```

  - Hợp đồng lệnh cho Plan 6:

    ```ts
    export interface TauriCommandMap {
      query_faces: { args: { q: FaceQuery }; result: FacePage };
      preview_info: { args: Record<string, never>; result: PreviewInfo };
      preview_bump_generation: { args: Record<string, never>; result: number };
      preview_prewarm: { args: { items: Array<{ hashHex: string; faceIndex: number; px: number; text: string; generation: number }> }; result: null };
    }
    ```

  - `export function makePreviewUrl(toUrl: (fileName: string) => string): LibraryApi['previewUrl']`
  - `export function makeFontUrl(toUrl: (fileName: string) => string): LibraryApi['fontUrl']`: `null` khi `faceIndex > 0`
  - `export type MockApi = LibraryApi & { calls: FaceQuery[]; currentGeneration(): number }`
  - `export function createMockLibraryApi(rows: FaceRow[], o?: { latencyMs?: number; toUrl?: (n: string) => string; fail?: () => Error | null }): MockApi`
  - `export function makeRow(i: number, over?: Partial<FaceRow>): FaceRow`

- [ ] **Step 1: Cài đặt và cấu hình**

```bash
cd UX
npm install --save-exact react-window@2.3.4
npm uninstall @types/react-window
npm install --save-dev --save-exact vitest@3.2.4 jsdom@26.1.0 @testing-library/react@16.3.0 @testing-library/dom@10.4.0 @testing-library/jest-dom@6.6.3
npm pkg set scripts.test="vitest run"
```

`UX/tsconfig.json`: thêm `"resolveJsonModule": true` vào `compilerOptions`.

`UX/vitest.config.ts`:
```ts
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { '@': path.resolve(__dirname, '.') } },
  test: { environment: 'jsdom', setupFiles: ['./test/setup.ts'], include: ['features/**/__tests__/**/*.test.{ts,tsx}'] },
});
```

`UX/test/setup.ts`:
- import `@testing-library/jest-dom/vitest`
- polyfill `ResizeObserver` (lớp rỗng `observe/unobserve/disconnect`) nếu chưa có
- `globalThis.requestAnimationFrame ??= cb => setTimeout(() => cb(performance.now()), 0) as unknown as number` và `cancelAnimationFrame` tương ứng
- `URL.createObjectURL ??= () => 'blob:mock'`, `URL.revokeObjectURL ??= () => {}`

- [ ] **Step 2: Viết test thất bại** — `__tests__/types.test.ts` và `__tests__/api.test.ts`

`types.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
// Fixture do store sinh từ serde thật (test face_row_json_matches_contract_fixture của Plan 2).
import row from '../../../src-tauri/crates/fontatlas-store/tests/fixtures/face_row.json';
import { ContractError, cssSize, parseFacePage, parseFaceRow } from '../types';

describe('hợp đồng FaceRow với store', () => {
  it('accepts the Rust-generated fixture', () => {
    const r = parseFaceRow(row);
    expect(r.fileSize).toBe(12345);
    expect(r.hashHex).toHaveLength(64);
    expect(parseFacePage({ total: 1, offset: 0, items: [row] }).items).toHaveLength(1);
  });
  it('accepts null where Rust has Option', () => {
    expect(() => parseFaceRow({ ...row, subfamily: null, fullName: null, weightClass: null, widthClass: null, path: null })).not.toThrow();
  });
  it('rejects missing, mistyped or unexpected fields with their path', () => {
    const { fileSize: _, ...missing } = row;
    expect(() => parseFacePage({ total: 1, offset: 0, items: [missing] })).toThrow(ContractError);
    expect(() => parseFacePage({ total: 1, offset: 0, items: [missing] })).toThrow(/items\[0\]\.fileSize/);
    expect(() => parseFaceRow({ ...row, faceIndex: '0' })).toThrow(/faceIndex/);
    expect(() => parseFaceRow({ ...row, family: null })).toThrow(/family/);
    expect(() => parseFaceRow({ ...row, newField: 1 })).toThrow(/unexpected key newField/);
  });
});

describe('cssSize', () => {
  it('is half of the Rust canvas', () => {
    expect(cssSize(48)).toEqual({ width: 312, height: 56 });
    expect(cssSize(32)).toEqual({ width: 208, height: 37 });
  });
});
```

`api.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import { createMockLibraryApi, makeFontUrl, makePreviewUrl, makeRow } from '../api';

const toUrl = (n: string) => `fontatlas-preview://localhost/${encodeURIComponent(n)}`;
const H = 'ab'.repeat(32);

describe('previewUrl', () => {
  it('builds file name, NFC text, generation and renderer version', () => {
    const url = makePreviewUrl(toUrl)({ hashHex: H, faceIndex: 1 }, { text: 'Ắn phở', px: 48, generation: 3, rendererVersion: 'preview-0.1.0' });
    expect(url).toBe(`fontatlas-preview://localhost/${H}_1_48.png?t=${encodeURIComponent('Ắn phở')}&g=3&r=preview-0.1.0`);
  });
  it('NFD and NFC text give the same url (no double fetch)', () => {
    const p = makePreviewUrl(toUrl);
    const o = { px: 48 as const, generation: 0, rendererVersion: 'v' };
    expect(p({ hashHex: H, faceIndex: 0 }, { ...o, text: 'Ắn' })).toBe(p({ hashHex: H, faceIndex: 0 }, { ...o, text: 'Ắn' }));
  });
});

describe('fontUrl', () => {
  it('only for face 0', () => {
    const f = makeFontUrl(n => `fontatlas-font://localhost/${n}`);
    expect(f({ hashHex: H, faceIndex: 0 })).toBe(`fontatlas-font://localhost/${H}_0.font`);
    expect(f({ hashHex: H, faceIndex: 1 })).toBeNull();
  });
});

describe('mock api', () => {
  const rows = Array.from({ length: 1500 }, (_, i) => makeRow(i));
  it('pages with total and clamps limit to 1000', async () => {
    const api = createMockLibraryApi(rows);
    const p = await api.queryFaces({ offset: 1400, limit: 200 });
    expect(p.total).toBe(1500);
    expect(p.items).toHaveLength(100);
    expect(p.items[0].faceId).toBe(rows[1400].faceId);
    expect((await api.queryFaces({ limit: 5000 })).items).toHaveLength(1000);
  });
  it('filters by folded text like the backend (đ → d, no diacritics)', async () => {
    const api = createMockLibraryApi([...rows.slice(0, 10), makeRow(999, { family: 'Đẹp Sans' })]);
    const p = await api.queryFaces({ text: 'dep' });
    expect(p.items.map(r => r.family)).toEqual(['Đẹp Sans']);
    expect(api.calls.at(-1)).toEqual({ text: 'dep' });
  });
  it('owns the generation like the backend', async () => {
    const api = createMockLibraryApi(rows);
    expect((await api.previewInfo()).generation).toBe(0);
    expect(await api.bumpGeneration()).toBe(1);
    expect((await api.previewInfo()).generation).toBe(1);
    expect(api.currentGeneration()).toBe(1);
  });
});
```

- [ ] **Step 3: Chạy test, xác nhận thất bại**

Run: `cd UX && npx vitest run features/library-grid/__tests__/types.test.ts features/library-grid/__tests__/api.test.ts`
Expected: FAIL, lỗi resolve `../types` / `../api`.

- [ ] **Step 4: Viết implementation**

- `parseFaceRow` / `parseFacePage`:
  - Hàm thuần không dùng thư viện. Bảng mô tả `const ROW_SCHEMA: Record<keyof FaceRow, 'number' | 'string' | 'boolean' | 'string|null' | 'number|null'>`. Kiểu `Record<keyof FaceRow, …>` buộc bảng đủ khoá.
  - Kiểm mọi khoá trong bảng (thiếu/sai kiểu → lỗi) và mọi khoá của object (khoá không có trong bảng → `unexpected key {k}`). Lỗi ném `new ContractError(`${path}${k}: …`)`, với `path` dạng `items[0].`.
  - `parseFacePage` kiểm `total`, `offset` là số, `items` là mảng, rồi gọi `parseFaceRow(item, `items[${i}].`)`.
- `makePreviewUrl`: `${toUrl(`${hashHex}_${faceIndex}_${px}.png`)}?t=${encodeURIComponent(text.normalize('NFC').trim())}&g=${generation}&r=${rendererVersion}`.
- `makeFontUrl`: `faceIndex > 0 ? null : toUrl(`${hashHex}_${faceIndex}.font`)`.
- `createMockLibraryApi`:
  - `fold(s)` = `s.normalize('NFD').replace(/\p{Mn}/gu, '').replace(/đ/gi, 'd').toLowerCase()`.
  - Lọc `text` bằng `includes` trên `fold(family)`, sắp theo `fold(family)` rồi `faceId`. `limit` = 100 nếu 0 hoặc không có, kẹp ≤ 1000.
  - Đợi `latencyMs` nếu có. `fail?.()` khác `null` → reject bằng lỗi đó. Ghi `calls`.
  - Bộ đếm thế hệ nằm trong closure của API (sống qua nhiều lần mount).
  - `previewInfo` → `{ rendererVersion: 'mock', generation, buckets: [...PX_BUCKETS] }`.
  - `previewUrl = makePreviewUrl(o.toUrl ?? (n => `/preview/${n}`))`, `fontUrl = makeFontUrl(n => `/font/${n}`)`.
- `makeRow(i)`:
  - `faceId = i + 1`, `hashHex = i.toString(16).padStart(64, '0')`, `faceIndex = 0`, `family = `Family ${String(i).padStart(5, '0')}``.
  - `viStatus 'full'`, `supportsVi true`, `viMissing 0`, `licenseClass 'ofl'`, `path = `/fonts/${i}.ttf``, `fileSize = 200_000`.
  - Các trường còn lại mang giá trị trung tính.
- `index.ts`: re-export public API.

- [ ] **Step 5: Chạy test, typecheck**

Run: `cd UX && npx vitest run features/library-grid/__tests__/types.test.ts features/library-grid/__tests__/api.test.ts && npx tsc --noEmit -p . 2>&1 | grep 'features/library-grid' ; echo done`
Expected: `10 passed` (`types` 4, `api` 6). Không có dòng lỗi nào thuộc `features/library-grid`.

- [ ] **Step 6: Commit**

```bash
git add UX/package.json UX/package-lock.json UX/tsconfig.json UX/vitest.config.ts UX/test UX/features/library-grid
git commit -m "feat(ux-grid): vitest setup, FaceRow contract guard on store fixture, IPC contract, mock api

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: `PagedFaces`, `PreviewBlobs`, `FontFaceLoader`

**Files:**
- Create: `UX/features/library-grid/pagedFaces.ts`, `previewBlobs.ts`, `fontFaceLoader.ts`
- Test: `__tests__/pagedFaces.test.ts`, `__tests__/previewBlobs.test.ts`, `__tests__/fontFaceLoader.test.ts`

**Interfaces:**
- Consumes: `LibraryApi`, `FaceRow`, `FaceQuery`, `GridQuery`, `makeRow`, `createMockLibraryApi` (Task 6)
- Produces (`pagedFaces.ts`):
  - `export const PAGE_SIZE = 200; export const SEARCH_DEBOUNCE_MS = 150`
  - `export type PagedSnapshot = { total: number | null; version: number; error: string | null }`
  - `export class PagedFaces`:
    - `constructor(api: Pick<LibraryApi, 'queryFaces'>, query: GridQuery)`
    - `subscribe(cb: () => void): () => void`
    - `getSnapshot(): PagedSnapshot`
    - `setQuery(q: GridQuery): void`
    - `ensureRange(start: number, stop: number): void`
    - `rowAt(i: number): FaceRow | undefined`
    - `dispose(): void`
  - `export function usePagedFaces(api: LibraryApi, query: GridQuery): { paged: PagedFaces | null; snap: PagedSnapshot }`: tạo **và** dispose trong cùng một `useEffect([api])`
- Produces (`previewBlobs.ts`):
  - `export const MAX_BLOBS = 800`
  - `export class PreviewHttpError extends Error { constructor(public status: number, public code: string) }`
  - `export type BlobDeps = { fetch(url: string, init: { signal: AbortSignal }): Promise<{ ok: boolean; status: number; blob(): Promise<Blob>; text(): Promise<string> }>; createObjectURL(b: Blob): string; revokeObjectURL(u: string): void }`
  - `export class PreviewBlobs`:
    - `constructor(deps?: BlobDeps, max?: number)`
    - `acquire(url: string): Promise<string>`
    - `release(url: string): void`
    - `stats(): { entries: number; fetches: number; revoked: number }`
- Produces (`fontFaceLoader.ts`):
  - `export const LOADER = { BUDGET_BYTES: 128 * 1024 * 1024, MAX_ACTIVE: 6, V_MAX: 1500, IDLE_MS: 120 } as const`
  - `export class ScrollVelocity { sample(scrollTop: number, tMs: number): void; value(tMs: number): number }`
  - `export type LoaderRow = Pick<FaceRow, 'hashHex' | 'faceIndex' | 'fileSize'>`
  - `export type LoaderDeps = { createFace(family: string, source: string): { load(): Promise<unknown> }; fonts: { add(f: unknown): void; delete(f: unknown): boolean }; fontUrl(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string | null; now(): number; velocity(): number; setTimer(fn: () => void, ms: number): unknown; clearTimer(h: unknown): void }`
  - `export class FontFaceLoader`:
    - `constructor(deps: LoaderDeps, opts?: Partial<typeof LOADER>)`
    - `acquire(row: LoaderRow): Promise<string | null>`
    - `release(row: LoaderRow): void`
    - `onScroll(): void`
    - `stats(): { active: number; readyBytes: number; loads: number; evictions: number; cancels: number }`
  - `export function familyName(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string` = `fa-${hashHex.slice(0, 16)}-${faceIndex}`
  - `export function browserLoaderDeps(api: LibraryApi, velocity: ScrollVelocity): LoaderDeps`: `createFace = (f, src) => new FontFace(f, src)`, `fonts = document.fonts`, `fontUrl = api.fontUrl`, `now = performance.now`

- [ ] **Step 1: Viết test thất bại** — `__tests__/pagedFaces.test.ts`

```ts
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { PAGE_SIZE, PagedFaces, SEARCH_DEBOUNCE_MS } from '../pagedFaces';
import type { FacePage, FaceQuery } from '../types';

const rows = Array.from({ length: 1000 }, (_, i) => makeRow(i));
/** Chạy hết microtask (mock trả Promise đã resolve; không dùng setTimeout vì đồng hồ giả). */
const flush = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); };
function deferredApi() {
  const calls: FaceQuery[] = []; const resolvers: Array<(p: FacePage) => void> = [];
  return { calls, resolvers, queryFaces: (q: FaceQuery) => { calls.push(q); return new Promise<FacePage>(r => resolvers.push(r)); } };
}

beforeEach(() => vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] }));
afterEach(() => vi.useRealTimers());

describe('PagedFaces', () => {
  it('fetches page 0 immediately and learns total', async () => {
    const api = createMockLibraryApi(rows);
    const p = new PagedFaces(api, {});
    await flush();
    expect(api.calls).toEqual([{ offset: 0, limit: PAGE_SIZE }]);
    expect(p.getSnapshot().total).toBe(1000);
    expect(p.rowAt(0)?.faceId).toBe(1);
    expect(p.rowAt(PAGE_SIZE)).toBeUndefined();
  });

  it('only fetches missing pages around the visible range', async () => {
    const api = createMockLibraryApi(rows);
    const p = new PagedFaces(api, {});
    await flush();
    p.ensureRange(0, 40); p.ensureRange(0, 40);
    p.ensureRange(150, 190);
    await flush();
    expect(api.calls.map(c => c.offset)).toEqual([0, 200]);
    expect(p.rowAt(250)?.faceId).toBe(251);
  });

  it('drops responses from an older query', async () => {
    const api = deferredApi();
    const p = new PagedFaces(api, { text: 'a' });
    p.setQuery({ text: 'b' });
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    api.resolvers[1]({ total: 1, offset: 0, items: [makeRow(42)] });
    api.resolvers[0]({ total: 9, offset: 0, items: [makeRow(1)] });
    await flush();
    expect(p.getSnapshot().total).toBe(1);
    expect(p.rowAt(0)?.faceId).toBe(43);
  });

  it('debounces text; other filters apply immediately and cancel the pending debounce', async () => {
    const api = createMockLibraryApi(rows);
    const p = new PagedFaces(api, {});
    await flush();
    p.setQuery({ text: 'f' }); p.setQuery({ text: 'fa' });
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS - 1);
    expect(api.calls).toHaveLength(1);
    p.setQuery({ text: 'fa', favoritesOnly: true }); // áp dụng ngay, huỷ timer của 'fa'
    await flush();
    expect(api.calls).toHaveLength(2);
    expect(api.calls.at(-1)).toMatchObject({ text: 'fa', favoritesOnly: true, offset: 0 });
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    expect(api.calls).toHaveLength(2);
  });

  it('surfaces errors and retries the page on next ensureRange', async () => {
    let fail = true;
    const api = { queryFaces: vi.fn(async (q: FaceQuery) => { if (fail) throw new Error('db'); return { total: 1, offset: q.offset ?? 0, items: [makeRow(0)] }; }) };
    const p = new PagedFaces(api, {});
    await flush();
    expect(p.getSnapshot().error).toBe('db');
    fail = false;
    p.ensureRange(0, 10);
    await flush();
    expect(p.getSnapshot()).toMatchObject({ error: null, total: 1 });
  });

  it('disposed_instance_ignores_responses', async () => {
    const api = deferredApi();
    const p = new PagedFaces(api, {});
    const cb = vi.fn(); p.subscribe(cb);
    p.dispose();
    api.resolvers[0]({ total: 5, offset: 0, items: [makeRow(0)] });
    await flush();
    expect(cb).not.toHaveBeenCalled();
    expect(p.getSnapshot().total).toBeNull();
    p.setQuery({ text: 'x' }); p.ensureRange(0, 10);
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    expect(api.calls).toHaveLength(1);
  });
});
```

- [ ] **Step 2: Viết test thất bại** — `__tests__/previewBlobs.test.ts`

```ts
import { describe, expect, it, vi } from 'vitest';
import { PreviewBlobs, PreviewHttpError, type BlobDeps } from '../previewBlobs';

function deps(status = 200) {
  let n = 0; const revoked: string[] = []; const aborted: string[] = [];
  const d: BlobDeps = {
    fetch: vi.fn(async (url: string, init: { signal: AbortSignal }) => {
      init.signal.addEventListener('abort', () => aborted.push(url));
      return { ok: status === 200, status, blob: async () => new Blob(['x']), text: async () => 'stale_generation' };
    }),
    createObjectURL: () => `blob:${++n}`,
    revokeObjectURL: u => void revoked.push(u),
  };
  return { d, revoked, aborted };
}

describe('PreviewBlobs', () => {
  it('fetches each url once and shares the blob url', async () => {
    const { d } = deps(); const b = new PreviewBlobs(d);
    const [x, y] = await Promise.all([b.acquire('/a'), b.acquire('/a')]);
    expect(x).toBe(y);
    expect(d.fetch).toHaveBeenCalledTimes(1);
  });

  it('keeps referenced blobs and revokes only unreferenced beyond the cap', async () => {
    const { d, revoked } = deps(); const b = new PreviewBlobs(d, 2);
    const a = await b.acquire('/a');                               // giữ /a
    for (const u of ['/b', '/c', '/d']) { await b.acquire(u); b.release(u); }
    expect(revoked).not.toContain(a);
    expect(revoked).toHaveLength(1);                               // /b bị đẩy; /c /d còn trong 2 chỗ
    expect(b.stats().entries).toBe(3);
  });

  it('non-ok responses reject with status and code and are not cached', async () => {
    const { d } = deps(410); const b = new PreviewBlobs(d);
    await expect(b.acquire('/a')).rejects.toEqual(new PreviewHttpError(410, 'stale_generation'));
    await b.acquire('/a').catch(() => {});
    expect(d.fetch).toHaveBeenCalledTimes(2);
  });

  it('aborts the fetch when released before it finishes', async () => {
    const { d, aborted } = deps(); const b = new PreviewBlobs(d);
    const p = b.acquire('/a'); b.release('/a');
    await p.catch(() => {});
    expect(aborted).toEqual(['/a']);
  });
});
```

- [ ] **Step 3: Viết test thất bại** — `__tests__/fontFaceLoader.test.ts`

```ts
import { describe, expect, it } from 'vitest';
import { makeRow } from '../api';
import { FontFaceLoader, LOADER, ScrollVelocity, familyName, type LoaderDeps } from '../fontFaceLoader';

/** Phụ thuộc giả: đồng hồ tay, load() giữ lại được, đếm add/delete và nguồn đã dùng. */
function fake(o: { velocity?: number; fail?: boolean } = {}) {
  let t = 0; let v = o.velocity ?? 0;
  const timers: Array<{ at: number; fn: () => void }> = [];
  const pending: Array<() => void> = [];
  const added = new Set<unknown>(); const sources: string[] = [];
  const deps: LoaderDeps = {
    createFace: (family, source) => { sources.push(source); return { family, load: () => new Promise((res, rej) => pending.push(() => (o.fail ? rej(new Error('x')) : res(null)))) }; },
    fonts: { add: f => void added.add(f), delete: f => added.delete(f) },
    fontUrl: r => (r.faceIndex > 0 ? null : `fontatlas-font://localhost/${r.hashHex}_0.font`),
    now: () => t, velocity: () => v,
    setTimer: (fn, ms) => { const h = { at: t + ms, fn }; timers.push(h); return h; },
    clearTimer: h => { const i = timers.indexOf(h as never); if (i >= 0) timers.splice(i, 1); },
  };
  const settle = async () => { while (pending.length) pending.shift()!(); for (let i = 0; i < 10; i++) await Promise.resolve(); };
  const advance = (ms: number) => { t += ms; for (const h of [...timers]) if (h.at <= t) { timers.splice(timers.indexOf(h), 1); h.fn(); } };
  return { deps, settle, advance, added, sources, setV: (x: number) => (v = x) };
}

describe('FontFaceLoader', () => {
  it('dedupes, loads by url (no ArrayBuffer) and returns the family name', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    const r = makeRow(1);
    const a = l.acquire(r); const b = l.acquire(r);
    await f.settle();
    expect(await a).toBe(familyName(r));
    expect(await b).toBe(familyName(r));
    expect(f.sources).toEqual([`url("fontatlas-font://localhost/${r.hashHex}_0.font")`]);
  });

  it('refuses_face_index_above_zero', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    expect(await l.acquire(makeRow(1, { faceIndex: 1 }))).toBeNull();
    expect(f.sources).toHaveLength(0);
  });

  it('caps concurrency at MAX_ACTIVE', () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    for (let i = 0; i < 10; i++) l.acquire(makeRow(i));
    expect(l.stats().active).toBe(LOADER.MAX_ACTIVE);
    expect(f.sources).toHaveLength(LOADER.MAX_ACTIVE);
  });

  it('fast_scroll_defers_until_idle and cancels rows released meanwhile', () => {
    const f = fake({ velocity: 4000 }); const l = new FontFaceLoader(f.deps);
    const a = makeRow(1), b = makeRow(2);
    l.acquire(a); l.acquire(b); l.onScroll();
    expect(f.sources).toHaveLength(0);
    l.release(b);
    f.setV(0); f.advance(LOADER.IDLE_MS);
    expect(f.sources).toHaveLength(1);
    expect(l.stats().cancels).toBe(1);
  });

  it('slow scroll below V_MAX loads immediately', () => {
    const f = fake({ velocity: LOADER.V_MAX - 1 }); const l = new FontFaceLoader(f.deps);
    l.onScroll(); l.acquire(makeRow(1));
    expect(f.sources).toHaveLength(1);
  });

  it('evicts unreferenced faces by fileSize, oldest first, never in-use ones', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps, { BUDGET_BYTES: 1000 });
    const rs = [0, 1, 2, 3].map(i => makeRow(i, { fileSize: 400 }));
    l.acquire(rs[0]); await f.settle(); l.release(rs[0]); f.advance(1);
    l.acquire(rs[1]); await f.settle(); l.release(rs[1]); f.advance(1);
    l.acquire(rs[2]); await f.settle();
    l.acquire(rs[3]); await f.settle();
    expect(l.stats()).toMatchObject({ evictions: 2, readyBytes: 800 });
    expect(f.added.size).toBe(2);
  });

  it('load error resolves null and is not retried', async () => {
    const f = fake({ fail: true }); const l = new FontFaceLoader(f.deps);
    const r = makeRow(1);
    const p = l.acquire(r); await f.settle();
    expect(await p).toBeNull();
    l.release(r);
    expect(await l.acquire(r)).toBeNull();
    expect(f.sources).toHaveLength(1);
  });
});

describe('ScrollVelocity', () => {
  it('measures px/s and decays to 0 after idle', () => {
    const v = new ScrollVelocity();
    v.sample(0, 0); v.sample(400, 100); v.sample(800, 200);
    expect(v.value(200)).toBeGreaterThan(LOADER.V_MAX);
    expect(v.value(200 + LOADER.IDLE_MS + 1)).toBe(0);
  });
});
```

- [ ] **Step 4: Chạy test, xác nhận thất bại**

Run: `cd UX && npx vitest run features/library-grid/__tests__/pagedFaces.test.ts features/library-grid/__tests__/previewBlobs.test.ts features/library-grid/__tests__/fontFaceLoader.test.ts`
Expected: FAIL, lỗi resolve module.

- [ ] **Step 5: Viết implementation** — `pagedFaces.ts`

- Trạng thái nội bộ:
  - `epoch`, `disposed`, `query`, `total`, `error`
  - `pages: Map<number, FaceRow[]>`, `inflight: Set<number>`, `timer`
  - `snapshot` object, tạo mới mỗi lần `emit()` (tăng `version`)
- `constructor` gọi `fetchPage(0)`.
- `setQuery(q)`:
  - `disposed` → bỏ qua.
  - Chỉ `text` khác (so `JSON.stringify` của phần còn lại) → huỷ timer cũ, đặt timer `SEARCH_DEBOUNCE_MS` để áp dụng.
  - Ngược lại → **huỷ timer đang chờ** rồi áp dụng ngay.
  - Áp dụng = `epoch++`, xoá `pages`/`inflight`, `total = null`, `error = null`, `fetchPage(0)`, `emit()`.
- `ensureRange(start, stop)`: `disposed` → bỏ. Trang từ `floor(max(0, start - PAGE_SIZE/2) / PAGE_SIZE)` tới `floor((stop + PAGE_SIZE/2) / PAGE_SIZE)`; khi biết `total`, kẹp ≤ `floor((total-1)/PAGE_SIZE)`. Trang chưa có và không đang tải → `fetchPage`.
- `fetchPage(n)`:
  - Ghi nhớ `e = epoch`. Gọi `queryFaces({ ...query, offset: n*PAGE_SIZE, limit: PAGE_SIZE })`.
  - Khi xong: `disposed || e !== epoch` → bỏ.
  - Thành công → `pages.set`, `total`, `error = null`. Lỗi → `error = err.message ?? String(err)`.
  - Luôn `inflight.delete(n)`, `emit()`.
- `dispose()`: `disposed = true`, huỷ timer, xoá listener.
- `usePagedFaces`:
  - `const [paged, setPaged] = useState<PagedFaces | null>(null)`.
  - `useEffect(() => { const p = new PagedFaces(api, queryRef.current); setPaged(p); return () => p.dispose(); }, [api])`.
  - Một effect khác gọi `paged?.setQuery(query)` khi `JSON.stringify(query)` đổi.
  - `useSyncExternalStore` với `subscribe`/`getSnapshot` an toàn khi `paged === null`: snapshot hằng `{ total: null, version: 0, error: null }`.

- [ ] **Step 6: Viết implementation** — `previewBlobs.ts`

- Entry theo URL: `{ refs, promise, blobUrl?, ctrl, lastRelease }`.
- `acquire(url)`:
  - Có entry → `refs++`, trả `promise`.
  - Chưa có → `AbortController` → `fetch`.
    - `!ok` → đọc `text()` làm code, xoá entry, reject `PreviewHttpError(status, code)`.
    - `ok` → `createObjectURL(await blob())`.
- `release(url)`:
  - `refs--`.
  - `refs === 0` và chưa xong → `ctrl.abort()`, xoá entry.
  - Đã xong → ghi `lastRelease` (bộ đếm tăng dần) rồi `trim()`.
- `trim()`: số entry `refs === 0` vượt `max` → đẩy theo `lastRelease` tăng dần, gọi `revokeObjectURL`.
- `deps` mặc định: `globalThis.fetch`, `URL.createObjectURL`, `URL.revokeObjectURL`.

- [ ] **Step 7: Viết implementation** — `fontFaceLoader.ts` (chính sách b2 của spike; thuật toán cố định)

```ts
// ScrollVelocity: EWMA như spike (0,7 cũ / 0,3 mới); quá IDLE_MS không có mẫu → 0.
sample(top, t) { if (this.lastT !== null) { const dt = Math.max(1, t - this.lastT);
  this.v = 0.7 * this.v + 0.3 * Math.abs(top - this.lastTop) / dt * 1000; }
  this.lastTop = top; this.lastT = t; }
value(t) { return this.lastT === null || t - this.lastT > LOADER.IDLE_MS ? 0 : this.v; }
```

Mỗi entry (khoá `familyName`) có trạng thái `waiting | queued | loading | ready | error`, kèm `refs`, `used`, `bytes = row.fileSize`, `face`, `promise`, `resolve`.

- `acquire(row)`:
  - `fontUrl(row) === null` (gồm `faceIndex > 0`) → `null`.
  - Đã có entry → `refs++`, `used = now()`, trả `promise`.
  - Mới:
    - `velocity() < V_MAX` → `queued` + `pump()`.
    - Ngược lại → `waiting`.
- `release(row)`:
  - `refs--`.
  - `refs ≤ 0` và `waiting`/`queued` → xoá, `cancels++`, resolve `null`.
- `onScroll()`: đặt lại timer `IDLE_MS` → `flushWaiting()`. Nếu `velocity() < V_MAX` thì flush ngay.
- `flushWaiting()`: entry `waiting` có `refs > 0` → `queued`, rồi `pump()`.
- `pump()`: khi `active < MAX_ACTIVE` và còn `queued` → `loading`, `active++`, rồi `createFace(family, `url("${fontUrl}")`).load()` → `fonts.add`.
  - Thành công: `ready`, `readyBytes += bytes`, `loads++`, resolve family, `evict()`.
  - Lỗi: `error`, resolve `null`.
  - `finally`: `active--`, `pump()`.
- `evict()`: khi `readyBytes > BUDGET_BYTES`, xét entry `ready` có `refs === 0` theo `used` tăng dần: `fonts.delete(face)`, trừ byte, xoá entry, `evictions++`.

- [ ] **Step 8: Chạy test, xác nhận qua**

Run: `cd UX && npx vitest run features/library-grid/__tests__/pagedFaces.test.ts features/library-grid/__tests__/previewBlobs.test.ts features/library-grid/__tests__/fontFaceLoader.test.ts`
Expected: `18 passed` (6 + 4 + 8).

- [ ] **Step 9: Commit**

```bash
git add UX/features/library-grid
git commit -m "feat(ux-grid): backend paging (StrictMode-safe), single-fetch preview blobs, b2 FontFace loader by url

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: `LibraryGrid` và `FaceCell`

**Files:**
- Create: `UX/features/library-grid/FaceCell.tsx`, `LibraryGrid.tsx`, `libraryGrid.css`
- Modify: `UX/features/library-grid/index.ts`
- Test: `__tests__/LibraryGrid.test.tsx`

**Interfaces:**
- Consumes: `usePagedFaces`, `PreviewBlobs`, `PreviewHttpError`, `FontFaceLoader`, `ScrollVelocity`, `browserLoaderDeps`, `LibraryApi`, `cssSize`, `PxBucket`, `DEFAULT_SAMPLE` (Task 6–7)
- Produces:
  - `export const HOVER_DELAY_MS = 150`
  - `export type PreviewMode = 'mask' | 'img'; export const DEFAULT_PREVIEW_MODE: PreviewMode = 'mask'`
  - `export type CellState = 'loading' | 'ready' | 'error' | 'live'`
  - `export function FaceCell(p: { row: FaceRow | undefined; url: string | null; px: PxBucket; text: string; selected: boolean; mode: PreviewMode; blobs: PreviewBlobs; loader: FontFaceLoader; onSelect(row: FaceRow): void }): React.JSX.Element`
  - `export type LibraryGridProps = { api: LibraryApi; query: GridQuery; sampleText?: string; px?: PxBucket; previewMode?: PreviewMode; selectedFaceId?: number | null; onSelect?(row: FaceRow): void; loader?: FontFaceLoader; blobs?: PreviewBlobs; width?: number; height?: number }`
  - `export function LibraryGrid(p: LibraryGridProps): React.JSX.Element`
  - Thuộc tính DOM mà harness dựa vào:
    - mỗi ô: `role="gridcell"`, `data-face-id`, `data-state={CellState}`, `data-shown-url` (URL nguồn của ảnh đang hiển thị)
    - phần tử cuộn: `data-testid="library-grid-scroller"`

- [ ] **Step 1: Viết test thất bại** — `__tests__/LibraryGrid.test.tsx`

```tsx
import { StrictMode } from 'react';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow, type MockApi } from '../api';
import { FontFaceLoader } from '../fontFaceLoader';
import { HOVER_DELAY_MS, LibraryGrid, type LibraryGridProps } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';

const rows = Array.from({ length: 5000 }, (_, i) => makeRow(i));
const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
/** rAF trong setup là setTimeout(0) → cần cho chạy cả timer. */
const settle = async () => { await flush(); await act(async () => { await new Promise(r => setTimeout(r, 0)); }); await flush(); };

function stubLoader() {
  return { acquire: vi.fn(async (r: { faceIndex: number }) => (r.faceIndex > 0 ? null : 'fa-live')), release: vi.fn(), onScroll: vi.fn() } as unknown as FontFaceLoader & { acquire: ReturnType<typeof vi.fn>; release: ReturnType<typeof vi.fn> };
}
function blobs(status: (url: string) => number = () => 200) {
  const fetched: string[] = []; let n = 0;
  const d: BlobDeps = {
    fetch: async (url) => { fetched.push(url); const s = status(url); return { ok: s === 200, status: s, blob: async () => new Blob(['p']), text: async () => 'no_outlines' }; },
    createObjectURL: () => `blob:${++n}`, revokeObjectURL: () => {},
  };
  return { b: new PreviewBlobs(d), fetched };
}
function ui(api: MockApi, extra: Partial<LibraryGridProps>, deps: { loader: ReturnType<typeof stubLoader>; b: PreviewBlobs }) {
  return <LibraryGrid api={api} query={{}} width={1300} height={800} loader={deps.loader} blobs={deps.b} {...extra} />;
}
async function mount(extra: Partial<LibraryGridProps> = {}, data = rows, status?: (u: string) => number) {
  const api = createMockLibraryApi(data);
  const loader = stubLoader(); const { b, fetched } = blobs(status);
  const r = render(ui(api, extra, { loader, b }));
  await settle();
  return { ...r, api, loader, b, fetched, rerenderWith: (e: Partial<LibraryGridProps>) => r.rerender(ui(api, { ...extra, ...e }, { loader, b })) };
}
const cells = () => screen.getAllByRole('gridcell');

afterEach(() => { vi.useRealTimers(); document.documentElement.classList.remove('dark'); });

it('mask stylesheet tints with the theme foreground; img fallback inverts in dark', () => {
  const css = readFileSync(resolve(__dirname, '../libraryGrid.css'), 'utf8');
  expect(css).toMatch(/\.fa-preview-mask\s*{[^}]*background-color:\s*var\(--foreground\)/);
  expect(css).toMatch(/-webkit-mask-image:\s*var\(--preview-url\)/);
  expect(css).toMatch(/\.dark\s+\.fa-preview-img\s*{[^}]*filter:\s*invert\(1\)/);
});

describe('LibraryGrid', () => {
  it('virtualizes: only a few rows of 5000 are in the DOM', async () => {
    await mount();
    expect(cells().length).toBeGreaterThan(0);
    expect(cells().length).toBeLessThan(100);
    expect(screen.getByRole('grid')).toHaveAttribute('aria-rowcount', String(Math.ceil(5000 / 3)));
  });

  it('mask mode: one fetch per url, blob url in the mask, ready after a frame', async () => {
    const { fetched } = await mount();
    const c = cells()[0];
    expect(c).toHaveAttribute('data-state', 'ready');
    expect(c.querySelector('img')).toBeNull();
    const mask = c.querySelector('[data-mask]') as HTMLElement;
    expect(mask).toHaveClass('fa-preview-mask');
    expect(mask.style.getPropertyValue('--preview-url')).toMatch(/^url\("blob:\d+"\)$/);
    expect(new Set(fetched).size).toBe(fetched.length);
  });

  it('theme_toggle_keeps_preview', async () => {
    const { fetched } = await mount();
    const before = fetched.length;
    document.documentElement.classList.add('dark');
    await settle();
    expect(fetched.length).toBe(before);
    expect(cells()[0]).toHaveAttribute('data-state', 'ready');
  });

  it('text_change_keeps_old_image_until_new_ready and uses a backend generation', async () => {
    const { api, rerenderWith } = await mount({ sampleText: 'Một' });
    const c = () => cells()[0];
    const oldShown = c().getAttribute('data-shown-url')!;
    expect(oldShown).toMatch(/g=0/);
    rerenderWith({ sampleText: 'Hai' });
    expect(c().getAttribute('data-shown-url')).toBe(oldShown); // chưa đổi ảnh ngay
    await settle();
    expect(api.currentGeneration()).toBe(1);
    expect(c().getAttribute('data-shown-url')).toMatch(/g=1/);
    expect(c().getAttribute('data-shown-url')).toContain(encodeURIComponent('Hai'));
    expect(c()).toHaveAttribute('data-state', 'ready');
  });

  it('remount_uses_backend_generation', async () => {
    const api = createMockLibraryApi(rows);
    await api.bumpGeneration(); await api.bumpGeneration(); // backend đã ở thế hệ 2 (phiên trước)
    const loader = stubLoader(); const { b } = blobs();
    const first = render(ui(api, {}, { loader, b })); await settle();
    first.unmount();
    render(ui(api, {}, { loader, b })); await settle();
    expect(cells()[0].getAttribute('data-shown-url')).toMatch(/g=2/);
  });

  it('works_under_strict_mode', async () => {
    const api = createMockLibraryApi(rows);
    const loader = stubLoader(); const { b } = blobs();
    render(<StrictMode>{ui(api, {}, { loader, b })}</StrictMode>);
    await settle();
    expect(cells().length).toBeGreaterThan(0);
    expect(cells()[0]).toHaveAttribute('data-state', 'ready');
    expect(cells()[0].getAttribute('data-shown-url')).toMatch(/g=0/);
  });

  it('error_state_shows_placeholder', async () => {
    await mount({}, rows.slice(0, 3), () => 422);
    const c = cells()[0];
    expect(c).toHaveAttribute('data-state', 'error');
    expect(c).toHaveTextContent('Không có preview');
  });

  it('img fallback mode renders a plain img with the protocol url', async () => {
    const { fetched } = await mount({ previewMode: 'img' }, rows.slice(0, 3));
    const img = cells()[0].querySelector('img.fa-preview-img') as HTMLImageElement;
    expect(img.getAttribute('src')).toMatch(/^\/preview\/0{64}_0_48\.png\?/);
    expect(fetched).toHaveLength(0);
    fireEvent.load(img);
    expect(cells()[0]).toHaveAttribute('data-state', 'ready');
  });

  it('hover loads live font after delay and releases on leave', async () => {
    const { loader } = await mount();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const c = cells()[0];
    fireEvent.pointerEnter(c);
    expect(loader.acquire).not.toHaveBeenCalled();
    await act(async () => { await vi.advanceTimersByTimeAsync(HOVER_DELAY_MS); });
    expect(loader.acquire).toHaveBeenCalledTimes(1);
    expect(c).toHaveAttribute('data-state', 'live');
    fireEvent.pointerLeave(c);
    expect(loader.release).toHaveBeenCalledTimes(1);
  });

  it('hover_on_ttc_face_keeps_preview', async () => {
    const { loader } = await mount({}, [makeRow(0, { faceIndex: 1 })]);
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const c = cells()[0];
    fireEvent.pointerEnter(c);
    await act(async () => { await vi.advanceTimersByTimeAsync(HOVER_DELAY_MS); });
    expect(loader.acquire).not.toHaveBeenCalled();
    expect(c).toHaveAttribute('data-state', 'ready');
  });

  it('shows missing Vietnamese badge only for non-ASCII sample on fonts lacking it', async () => {
    await mount({ sampleText: 'Ăn phở' }, [makeRow(0, { supportsVi: false, viMissing: 12 }), makeRow(1)]);
    const [a, b] = cells();
    expect(a).toHaveTextContent('Thiếu 12 ký tự tiếng Việt');
    expect(b).not.toHaveTextContent('Thiếu');
  });

  it('click and Enter select the face', async () => {
    const onSelect = vi.fn();
    await mount({ onSelect });
    const c = cells()[1];
    fireEvent.click(c);
    fireEvent.keyDown(c, { key: 'Enter' });
    expect(onSelect).toHaveBeenCalledTimes(2);
    expect(onSelect.mock.calls[0][0].faceId).toBe(2);
  });

  it('shows the empty state', async () => {
    await mount({ query: { text: 'khong co font nao' } });
    expect(screen.getByText('Không có font nào khớp')).toBeInTheDocument();
  });

  it('shows the backend error state', async () => {
    const api = createMockLibraryApi(rows, { fail: () => new Error('db hỏng') });
    const { b } = blobs();
    render(<LibraryGrid api={api} query={{}} width={1300} height={800} loader={stubLoader()} blobs={b} />);
    await settle();
    expect(screen.getByRole('alert')).toHaveTextContent('Lỗi tải thư viện: db hỏng');
  });
});
```

- [ ] **Step 2: Chạy test, xác nhận thất bại**

Run: `cd UX && npx vitest run features/library-grid/__tests__/LibraryGrid.test.tsx`
Expected: FAIL, lỗi resolve `../LibraryGrid`.

- [ ] **Step 3: Viết implementation**

`LibraryGrid.tsx`:
- Kích thước ô và lưới:
  - `{width: cw, height: ch} = cssSize(px ?? 48)`
  - `cellW = cw + 24`, `rowH = ch + 64`
  - `cols = max(1, floor(width / cellW))`
  - `rowCount = ceil((snap.total ?? 0) / cols)`
- **Thế hệ từ backend:**
  - Một `useEffect([api])` gọi `api.previewInfo()` → state `{ rendererVersion, generation }` (cờ `alive` chống set sau unmount).
  - Khi `sampleText`/`px` đổi so với lần render trước (lần đầu không tính) → `api.bumpGeneration()` → set `generation` và chữ/cỡ "đã áp dụng".
  - Trong lúc chờ, URL của các ô dùng **giá trị cũ** (text cũ + thế hệ cũ), nên ảnh cũ không bị thay bằng một yêu cầu sẽ bị 410.
  - Chưa có `previewInfo` → `url = null`; ô ở trạng thái `loading`.
- `loader`, `blobs`, `velocity` lấy từ prop. Không có thì **tạo trong `useEffect` và huỷ ở cleanup của chính effect đó**, giữ trong state.
- `<div role="grid" aria-rowcount={rowCount} aria-colcount={cols}>` bọc `<List>`:
  - `rowCount`, `rowHeight={rowH}`, `overscanCount={2}`, `defaultHeight={height}`, `style={{ height, width }}`
  - `onRowsRendered={({ startIndex, stopIndex }) => paged?.ensureRange(startIndex*cols, (stopIndex+1)*cols - 1)}`
  - `rowProps` gồm `version` để hàng render lại khi trang về
- Phần tử cuộn có `data-testid="library-grid-scroller"`. `onScroll` → `velocity.sample(el.scrollTop, performance.now())` rồi `loader.onScroll()`.
- `snap.total === 0` → `<p>Không có font nào khớp</p>`. `snap.error` → `<p role="alert">Lỗi tải thư viện: {error}</p>`.

`FaceCell.tsx` (import `./libraryGrid.css`):
- `row === undefined` → khung xám, `data-state="loading"`.
- Phần tử ngoài: `<div role="gridcell" tabIndex={0} aria-selected data-face-id data-state data-shown-url onClick onKeyDown(Enter/Space)>`.
- Dòng tên + badge `Thiếu {viMissing} ký tự tiếng Việt` khi `/[^\x00-\x7F]/.test(text) && !row.supportsVi && row.viMissing > 0`.
- `mode === 'mask'`:
  - State `shown: { src: string; blob: string } | null`.
  - `useEffect([url])`: `url` khác `null` và khác `shown.src` → `state = shown ? state hiện tại : 'loading'` (giữ ảnh cũ) → `blobs.acquire(url)`. Khi xong (ô vẫn mount và `url` chưa đổi): `blobs.release(shown.src)` cũ, `setShown({ src: url, blob })`, rồi `requestAnimationFrame(() => setState('ready'))`.
  - Lỗi `PreviewHttpError` 410 → giữ nguyên (sắp có URL mới). Lỗi khác → `'error'`.
  - Cleanup: huỷ rAF; nếu acquire chưa xong → `blobs.release(url)`.
  - Unmount → `release(shown.src)`.
  - Render: `<div data-mask aria-hidden className="fa-preview-mask" style={{ width: cw, height: ch, ['--preview-url' as string]: `url("${shown.blob}")` }} />` khi có `shown`. `'error'` → `Không có preview`.
- `mode === 'img'`: `<img className="fa-preview-img" src={url} width={cw} height={ch} alt="" onLoad → 'ready' onError → 'error'>`. Không có `key={url}`: trình duyệt giữ ảnh cũ cho tới khi ảnh mới tải xong.
- `live`: `<span style={{ fontFamily: `"${family}"`, fontSize: (px * 3) / 4 }}>{text}</span>` thay cho preview.
- Hover:
  - Nếu `row.faceIndex > 0` → không làm gì.
  - `onPointerEnter` đặt timer `HOVER_DELAY_MS` → `loader.acquire(row)`. Kết quả khác `null` và ô còn hover → `live`.
  - `onPointerLeave` → huỷ timer, `release` nếu đã acquire, quay về trạng thái preview.
  - `selected` giữ `live`.

`libraryGrid.css`:
```css
.fa-preview-mask { background-color: var(--foreground); -webkit-mask-image: var(--preview-url); mask-image: var(--preview-url); -webkit-mask-size: 100% 100%; mask-size: 100% 100%; -webkit-mask-repeat: no-repeat; mask-repeat: no-repeat; }
.fa-preview-img { display: block; }
.dark .fa-preview-img { filter: invert(1); }
```

- [ ] **Step 4: Chạy toàn bộ test frontend và build**

Run: `cd UX && npm test && npx vite build`
Expected: `types` 4, `api` 6, `pagedFaces` 6, `previewBlobs` 4, `fontFaceLoader` 8, `LibraryGrid` 15, tổng 43 passed. `vite build` thành công.

- [ ] **Step 5: Commit**

```bash
git add UX/features/library-grid
git commit -m "feat(ux-grid): virtualized LibraryGrid with single-fetch mask previews, backend generations, img fallback flag

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Harness S2 trong module và CI

**Files:**
- Create: `UX/features/library-grid/perf/README.md`, `vite.config.ts`, `index.html`, `bench.tsx`, `wk.swift`, `run_s2.py`
- Create: `.github/workflows/preview-grid.yml`
- Modify: `UX/package.json` (script `perf:s2`), `.gitignore` (`UX/features/library-grid/perf/.data/`, `UX/features/library-grid/perf/wk`)

**Interfaces:**
- Consumes:
  - `LibraryGrid` (thuộc tính `previewMode`, DOM `data-state`, `data-testid="library-grid-scroller"`)
  - `createMockLibraryApi`, `makeRow` (Task 6–8)
  - ví dụ `dump_previews` (Task 5)
- Produces:
  - `npm run perf:s2 [-- --mode mask|img]` (macOS) → exit 0 nếu đạt S2, 1 nếu không
  - Ghi `perf/.data/runs.jsonl`
  - `window.__result`: `{ mode, n, first_ms, fps, frame_p95, blank_pct, slow_fps, slow_blank_pct, text_change_ms, fontfaces, dom, proc_mb }`. `fontfaces = document.fonts.size`.

Harness chép từ spike 05 vào ngay trong plan (spike không được track). Kịch bản giữ nguyên để số liệu so được với spike:
- cuộn nhanh 4.000 px/s trong 6 s
- cuộn chậm 600 px/s trong 6 s
- dừng 1,5 s rồi đo bộ nhớ
- đổi chữ mẫu

- [ ] **Step 1: Viết `run_s2.py`, chạy để thấy thất bại**

Các bước của `run_s2.py` (`argparse`: `--mode mask|img`, mặc định `mask`; `--n 5000`; `--runs 3`):
1. `cargo run --release -p fontatlas-preview --example dump_previews -- --out UX/features/library-grid/perf/.data/previews /System/Library/Fonts /Library/Fonts` (bỏ qua nếu `manifest.json` đã có).
2. `swiftc -O perf/wk.swift -o perf/wk`.
3. Chạy `npx vite --config features/library-grid/perf/vite.config.ts --port 5199 --strictPort` ở nền; chờ `GET /api/manifest` = 200, tối đa 30 s.
4. Chạy `--runs` lần `wk 'http://localhost:5199/?n={n}&mode={mode}&auto=1'`. Đọc dòng JSON và dòng `WEBCONTENT_PID … phys_footprint: N MB` (đổi GB sang MB), ghi vào `runs.jsonl`.
5. Tính trung vị và so với ngưỡng:

```python
S2 = {"fps": (">=", 58.0), "frame_p95": ("<=", 20.0), "slow_blank_pct": ("==", 0.0), "proc_mb": ("<", 400.0)}
```

6. In bảng trung vị kèm `first_ms`, `text_change_ms`, `fontfaces`, cấu hình máy (`sysctl -n machdep.cpu.brand_string hw.memsize`, `sw_vers -productVersion`).
7. Trượt với `mask` → in gợi ý `npm run perf:s2 -- --mode img` (phương án dự phòng). Dừng Vite, exit theo kết quả.

Run: `cd UX && npm pkg set scripts.perf:s2="python3 features/library-grid/perf/run_s2.py" && npm run perf:s2`
Expected: FAIL (exit 1), vì `wk.swift` và `vite.config.ts` của perf chưa có.

- [ ] **Step 2: Viết harness**

`perf/wk.swift` (từ spike 05):
```swift
// Mở URL trong WKWebView (engine Tauri dùng trên macOS), chờ window.__result, in kết quả + bộ nhớ tiến trình WebContent.
import Cocoa
import WebKit

final class D: NSObject, NSApplicationDelegate {
    var win: NSWindow!
    var wv: WKWebView!
    let started = Date()

    func applicationDidFinishLaunching(_ n: Notification) {
        let frame = NSRect(x: 40, y: 40, width: 1280, height: 800)
        let cfg = WKWebViewConfiguration()
        cfg.websiteDataStore = .nonPersistent() // không dùng HTTP cache giữa các lần chạy
        wv = WKWebView(frame: frame, configuration: cfg)
        // tránh WebKit hãm rAF khi cửa sổ bị che
        let sel = Selector(("_setWindowOcclusionDetectionEnabled:"))
        if wv.responds(to: sel) { wv.perform(sel, with: nil) }
        win = NSWindow(contentRect: frame, styleMask: [.titled], backing: .buffered, defer: false)
        win.level = .floating
        win.contentView = wv
        win.makeKeyAndOrderFront(nil)
        win.orderFrontRegardless()
        guard let url = URL(string: CommandLine.arguments[1]) else { exit(2) }
        wv.load(URLRequest(url: url))
        Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { _ in self.poll() }
    }

    func poll() {
        if Date().timeIntervalSince(started) > 240 { print("{\"error\":\"timeout\"}"); exit(1) }
        wv.evaluateJavaScript("window.__result ? JSON.stringify(window.__result) : ''") { r, _ in
            guard let s = r as? String, !s.isEmpty else { return }
            let pid = (self.wv.value(forKey: "_webProcessIdentifier") as? NSNumber)?.int32Value ?? 0
            let p = Process(); let pipe = Pipe()
            p.executableURL = URL(fileURLWithPath: "/usr/bin/footprint")
            p.arguments = ["-p", String(pid)]
            p.standardOutput = pipe; p.standardError = pipe
            try? p.run(); p.waitUntilExit()
            let out = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            let fp = out.split(separator: "\n").first { $0.contains("phys_footprint:") }.map(String.init) ?? String(out.prefix(300))
            print(s)
            print("WEBCONTENT_PID \(pid) \(fp.trimmingCharacters(in: .whitespaces))")
            exit(0)
        }
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory) // không cướp focus của người dùng
let d = D()
app.delegate = d
app.run()
```

`perf/vite.config.ts`: root `features/library-grid/perf`, plugin react, alias `@` → `UX/`. Middleware:
- `GET /api/manifest` → `manifest.json`.
- `GET /preview/{hash}_{idx}_{px}.png?t=…`:
  - `i = parseInt(hash, 16)`, `entry = manifest[i % manifest.length]`.
  - `t` bằng `DEFAULT_SAMPLE` hoặc không có → stream `{entry.id}.png`.
  - Ngược lại → spawn `dump_previews one <entry.path> <entry.faceIndex> 48 <t>` rồi trả stdout.
  - Header `content-type: image/png`, `cache-control: private, max-age=86400`.
- `POST /api/result` → ghi thêm vào `.data/results.jsonl`.

`perf/bench.tsx`:
- Đọc `n`, `mode`, `auto` từ query. Tạo `rows = Array.from({ length: n }, (_, i) => makeRow(i, { family: manifest[i % m].family }))`, `api = createMockLibraryApi(rows, { toUrl: name => `/preview/${name}` })`.
- Render `<LibraryGrid api={api} query={{}} width={1280} height={800} previewMode={mode} sampleText={text} />`, `text` ở state.
- Với `auto=1`, chạy kịch bản đo (thuật toán cố định, cùng kịch bản spike):

```ts
const raf = () => new Promise<number>(r => requestAnimationFrame(r));
const visibleCells = (el: HTMLElement) => {
  const box = el.getBoundingClientRect();
  return [...el.querySelectorAll<HTMLElement>('[role=gridcell]')].filter(c => {
    const r = c.getBoundingClientRect(); return r.bottom > box.top && r.top < box.bottom;
  });
};
const allReady = (el: HTMLElement) => visibleCells(el).every(c => c.dataset.state === 'ready');
async function waitReady(el: HTMLElement, timeout = 60000) {
  const t0 = performance.now();
  while (performance.now() - t0 < timeout) { if (allReady(el)) { await raf(); await raf(); return performance.now(); } await raf(); }
  return NaN;
}
async function scrollPhase(el: HTMLElement, speed: number, dur: number) {
  const frames: number[] = []; let blank = 0, seen = 0;
  let last = await raf(); const start = last;
  while (last - start < dur) {
    const now = await raf(); frames.push(now - last); last = now;
    el.scrollTop = Math.min(el.scrollTop + speed * frames.at(-1)! / 1000, el.scrollHeight - el.clientHeight);
    for (const c of visibleCells(el)) { seen++; if (c.dataset.state !== 'ready') blank++; }
  }
  const s = [...frames].sort((a, b) => a - b);
  return { fps: frames.length / (dur / 1000), p95: s[Math.floor(s.length * 0.95)], blank_pct: 100 * blank / Math.max(1, seen) };
}
// thứ tự: first_ms = waitReady(scroller) → sleep 500 → fast = scrollPhase(4000, 6000) → slow = scrollPhase(600, 6000)
// → waitReady → sleep 1500 → text_change: t0; setText('Đổi chữ mẫu 2026'); waitReady - t0
// → __result = { mode, n, first_ms, fps: fast.fps, frame_p95: fast.p95, blank_pct: fast.blank_pct,
//               slow_fps: slow.fps, slow_blank_pct: slow.blank_pct, text_change_ms,
//               fontfaces: document.fonts.size, dom: document.getElementsByTagName('*').length }
// → POST /api/result
```

`perf/README.md`: lệnh chạy, ngưỡng S2, cờ `--mode img`, yêu cầu (macOS, Xcode CLT, Rust, Node). Ghi chú: không chạy trên CI Linux.

`.github/workflows/preview-grid.yml`:
```yaml
name: preview-grid
on: [push, pull_request]
jobs:
  rust:
    runs-on: ubuntu-latest
    defaults: { run: { working-directory: UX/src-tauri } }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - run: cargo test -p fontatlas-core -p fontatlas-store -p fontatlas-preview
      - run: cargo clippy -p fontatlas-core -p fontatlas-store -p fontatlas-preview --all-targets -- -D warnings
  ux:
    runs-on: ubuntu-latest
    defaults: { run: { working-directory: UX } }
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: npm, cache-dependency-path: UX/package-lock.json }
      - run: npm ci
      - run: npm test
      - run: npx vite build
  s2:
    runs-on: macos-14
    continue-on-error: true   # chỉ báo cáo, không chặn merge
    defaults: { run: { working-directory: UX } }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: npm, cache-dependency-path: UX/package-lock.json }
      - run: npm ci
      - run: npm run perf:s2
      - if: always()
        uses: actions/upload-artifact@v4
        with: { name: s2-runs, path: UX/features/library-grid/perf/.data/runs.jsonl }
```

- [ ] **Step 3: Chạy harness, xác nhận đạt S2**

Run: `cd UX && npm run perf:s2`
Expected: bảng trung vị với `fps ≥ 58`, `frame_p95 ≤ 20`, `slow_blank_pct = 0`, `proc_mb < 400`, exit 0. Spike (đường `<img>`, n = 5000, M4 Pro) đo được 60 / 18 / 0 / 264 MB. Đường `mask` + blob là đường mới, chưa được đo.

- Nếu `mask` trượt: chạy `npm run perf:s2 -- --mode img`. Ghi cả hai bảng vào báo cáo và đề xuất đổi `DEFAULT_PREVIEW_MODE` trong báo cáo. **Không tự đổi** khi chưa có quyết định của Lead.
- Nếu `proc_mb` vượt: kiểm tra `fontfaces` (phải ≤ 2) và số blob giữ lại. Không nới ngưỡng.

- [ ] **Step 4: Commit**

```bash
git add UX/features/library-grid/perf UX/package.json .gitignore .github/workflows/preview-grid.yml
git commit -m "test(perf): S2 regression harness (mask path) and CI for preview + grid

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Rà soát cuối, báo cáo

**Files:**
- Create: `docs/team/plans/plan5-report.md`

- [ ] **Step 1: Chạy toàn bộ kiểm tra**

```bash
cd UX/src-tauri && cargo test -p fontatlas-core -p fontatlas-store -p fontatlas-preview && cargo clippy -p fontatlas-core -p fontatlas-store -p fontatlas-preview --all-targets -- -D warnings
cargo test -p fontatlas-preview --release --test render -- --ignored --nocapture
cd .. && npm test && npx vite build && npm run perf:s2
```
Expected: mọi lệnh exit 0. Test hợp đồng hai phía cùng qua: `face_row_json_matches_contract_fixture` (store) và `types.test.ts`. Ghi lại `ms/face` và bảng S2.

- [ ] **Step 2: Viết báo cáo** — `docs/team/plans/plan5-report.md`

Báo cáo gồm:
- **Số đo:** ms/face; kích thước trung bình mỗi PNG; S2 trung vị 3 lần (mode `mask`, và `img` nếu đã chạy) kèm cấu hình máy; `text_change_ms`.
- **Giới hạn:**
  - không shaping, không kerning, không bidi
  - VF vẽ ở instance mặc định
  - font bitmap/màu → `no_outlines`
  - badge tiếng Việt theo `viMissing` của bộ 186 ký tự, không theo đúng chữ mẫu
- **Việc cho Plan 6:**
  - cài `TauriCommandMap`, gồm `preview_info` và `preview_bump_generation` trả thế hệ từ `PreviewService`
  - đăng ký hai scheme gọi `handle_preview_uri` / `handle_font_uri` trong `spawn_blocking`, truyền header `Origin`, gắn đủ header CORS
  - cài `FaceLocator` bằng `Store`; áp dụng CSP ở Global Constraints; gọi `parseFacePage` trên kết quả `query_faces`
  - gọi `preview_prewarm` sau `onRowsRendered`
  - thay lưới cũ, đo S2 lại trong Tauri thật và trên Windows/WebView2

- [ ] **Step 3: Commit**

```bash
git add docs/team/plans/plan5-report.md
git commit -m "docs(preview): plan 5 report

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
