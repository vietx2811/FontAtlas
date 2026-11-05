# Plan 4 — `fontatlas-embed` Implementation Plan (SigLIP 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dựng crate `fontatlas-embed` cho chế độ "Số đo + Hình ảnh" của Similar. Crate làm các việc sau:
- render ảnh chuỗi mẫu của font bằng Rust;
- chạy encoder ảnh **SigLIP 2 base** (ONNX qua `ort`, CPU) theo lô 4 trên luồng nền;
- lưu vector thô 768 chiều và vector đã whiten 128 chiều vào `library.db`;
- chạy hàng đợi có tạm dừng, huỷ, và chạy tiếp sau khi khởi động lại;
- tải model khi cần, có kiểm checksum;
- đưa vector vào `fontatlas-index` qua đúng một đường là `FaceProfile.embedding`; chỉ mục int8 trong RAM, xếp lại bằng f32 đọc từ SQLite.

Model thiếu, hỏng hoặc bị tắt thì app vẫn chạy chế độ "Số đo".

**Architecture:**
- `fontatlas-embed` phụ thuộc `fontatlas-core` (đọc font, vị trí đo chuẩn) và `fontatlas-store` (hàng đợi, vector); không phụ thuộc Tauri.
- `fontatlas-store` có thêm module `embed.rs`: hàng đợi trên `pipeline_state` (stage `embedding`), đọc/ghi `embeddings`, cài đặt.
- Task 10 sửa `fontatlas-index` (Plan 3): chỉ mục int8 + rerank f32 nằm **trong** `Index`, dữ liệu vào chỉ qua `FaceProfile.embedding`. Task 10 cũng nối `fontatlas-eval` để đo S4.
- Số kỹ thuật lấy từ spike đã chạy thật:
  - `docs/team/spikes/03b_model_choice.md`: chọn SigLIP 2 vì giấy phép, kèm số đo;
  - `03_embedding_eval.md`: chuỗi render, whitening;
  - `02_data_vector.md`: int8 + rerank.

**Tech Stack:**
- Rust edition 2024, rustc ≥ 1.92 (`ort` rc.14 yêu cầu; máy có 1.96).
- `ort = { version = "=2.0.0-rc.14", default-features = false, features = ["std", "download-binaries", "tls-native", "api-31"] }` (ORT 1.31, liên kết tĩnh).
- `skrifa = "=0.48.0"`, `read-fonts = "=0.45.1"` (giống core).
- `tiny-skia = { version = "=0.12.0", default-features = false, features = ["std", "simd"] }` (giống Plan 5).
- `ureq = "3"`, `sha2 = "0.10"`, `blake3 = "=1.8.7"`, `rayon = "1.12"`, `serde`, `serde_json`, `thiserror = "2"`.
- Feature `fit` (tắt mặc định): `nalgebra`, ghim `=` bản 0.x mới nhất lúc thực thi; chỉ tool `whitenfit` dùng.
- dev: `tempfile = "3.27"`, `tiny_http = "0.12"`, `png = "=0.18.1"` (đọc ảnh vàng).
- Script Python **offline**: chạy một lần lúc thực thi, không chạy trong CI hay lúc app chạy.

**Spec:** `docs/superpowers/specs/2026-10-16-fontatlas-mvp-design.md`, mục 5 (đã cập nhật 18/10: embed mọi font), 6.1–6.2, 10, 11.

**Nhánh:**
- Base `dev/mvp`, nhánh thực thi `feat/fontatlas-embed`.
- **Gộp với Plan 3** (`feat/fontatlas-index`):
  - `[workspace].members` giữ mọi crate;
  - `standard_location` của core: Plan 3 tạo bản `pub(crate)`, Plan 4 cần `pub`; giữ đúng một định nghĩa `pub`;
  - fixture `public_sans_vf.ttf` tải đúng cùng URL/SHA với Plan 3 nên trùng byte.
- Task 1–9 không cần Plan 3. **Task 10 chỉ bắt đầu khi `feat/fontatlas-index` đã gộp vào `dev/mvp`.**
- **Phụ thuộc nhóm 03 (store):** `Store::gc()` phải xoá `embeddings` của hash không còn trong `files` (Lead đã giao). Plan này chỉ dựa vào hành vi đó; ngoài ra `finish_embeddings` tự bỏ qua hash đã mất.

## Global Constraints

- **Model:**
  - SigLIP 2 base patch16-224: `google/siglip2-base-patch16-224` @ revision `75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2`, giấy phép **Apache-2.0** (đã xác minh, spike 03b);
  - encoder ảnh = `get_image_features` (pooler MAP), fp32, **lô tĩnh 4**: input `pixel_values` `[4,3,224,224]`, output `image_embeds` `[4,768]`, opset 17, khoảng 372 MB;
  - **tự export**, không dùng bản ONNX bên thứ ba (các bản đó không ghi license);
  - kèm `NOTICE` và `LICENSE` Apache-2.0 cạnh file model;
  - lô cuối pad bằng ảnh trắng, bỏ kết quả của phần pad.
- **Runtime:** `ort` 2.0.0-rc.14 / ORT 1.31, CPU, liên kết tĩnh, `GraphOptimizationLevel::Level3`, `intra_threads = 4`. **Không CoreML, không model int8.**
- **Ảnh vào (`RENDER_ID = "render-v2"`):**
  - 224×224 xám, nền trắng, chữ đen, khử răng cưa, không resize;
  - 3 dòng `"Hamburg"`, `"Người Việt"`, `"Rgaeo 0123"`, cùng một cỡ; mỗi dòng căn giữa ngang, cả khối căn giữa dọc; lề 12 px;
  - chuẩn hoá SigLIP: `(v/255 − 0.5)/0.5`, R=G=B.
- **Vector lưu trong `embeddings`** (`source='local'`), mỗi face hai dòng:
  - thô: `model_version = RAW_VERSION`, `dim = 768`, chưa L2. Nhờ dòng này, đổi whitening không phải chạy lại model;
  - đã whiten: `model_version = MODEL_VERSION`, `dim = 128`, L2.
  - `RAW_VERSION = "siglip2-base-p16-224@75de2d5|onnx-fp32-b4@<sha256[:12] model>|ort1.31-cpu|render-v2"`.
  - `MODEL_VERSION = "{RAW_VERSION}|{whitening.version}@<sha256[:12] file whitening>"`.
  - Đổi model, runtime, render hoặc whitening là đổi version.
  - *Lệch spec có chủ ý:* spec 5.2/6.1 nói vector 512 chiều, int8 (CLIP). Với SigLIP 2, vector thô 768 chiều; chỉ mục dùng 128 chiều sau whitening. Lead cập nhật spec.
- **Whitening (`white-v1`):**
  - các bước: L2(thô) → trừ μ → chiếu lên 128 trục PCA, chia mỗi trục cho `√λᵢ + 0,1·√λ₁` → L2;
  - **bắt buộc**: SigLIP 2 thô kém rõ, R@5 0,814 so với 0,898 sau whiten;
  - fit trên **nửa tune** của kho Google Fonts OFL/Apache, SHA `47d54ca1ebb14fd864695f1d39dc734f39017c33` (cùng Plan 3), đo ở vị trí chuẩn;
  - nửa họ theo `family_split(family)`: byte đầu của `blake3(lowercase(family))` chẵn → `tune`, lẻ → `test`. **Không** dùng nửa test, để S4 sạch.
- **Chính sách (spec 5.1, quyết định 18/10):**
  - embed **mọi** font trong thư viện, không lọc theo `license_class`;
  - cài đặt `u.settings['embed_enabled']` mặc định **bật** (thiếu khoá = `'1'`). Giá trị `'0'` tắt toàn cục: worker không chạy, vector cũ giữ nguyên, Similar dùng chế độ "Số đo";
  - cờ `u.settings['license_notice_ack']` để UI (Plan 6) biết đã hiện thông báo trách nhiệm giấy phép chưa; plan này chỉ cung cấp hằng số và get/set;
  - embedding không bao giờ rời máy; crate không có mã gửi đi.
- **Model không nằm trong bộ cài:**
  - URL tải là **hằng cấu hình** trong `data/model-manifest.json` (url, sha256, size). Việc host công khai **chờ chủ app duyệt** (Task 9 Step 2);
  - tải vào `<app_data>/models/<file>.part`, kiểm size + sha256, `fsync` file và thư mục, rồi đổi tên nguyên tử;
  - khoá `<file>.lock` (`std::fs::File::lock`) để hai lần tải không chạy chồng;
  - sidecar `<file>.ok` (sha256, size, mtime) để kiểm nhanh ở các lần sau.
- **Chế độ:** model thiếu, dở, hỏng, nạp lỗi, hoặc `embed_enabled = '0'` → `EmbedMode::MeasuresOnly(reason)`; UI ghi "Số đo".
- **Hàng đợi:**
  - luồng nền riêng; render song song trên rayon `ThreadPool` riêng 4 luồng;
  - mỗi job bọc `catch_unwind`; panic → `failed`/`render_failed`;
  - tạm dừng có hiệu lực sau lô hiện tại;
  - huỷ dừng giữa lô (`RunOptions::terminate`), **không ghi một phần lô**, trả face về `pending`;
  - khởi động lại thì `running` → `pending`;
  - `claim` chạy trong transaction `Immediate`;
  - khi hàng rỗng, gọi `enqueue` thêm một lần (bắt face mới sau rescan) rồi mới kết luận xong.
- **Mã lỗi stage `embedding`:**
  - `failed`: `missing_file`, `stale_file` (hash file đã đổi), `render_failed`, `inference`;
  - `skipped`: `no_glyphs` (chuỗi mẫu có < 50% ký tự trong cmap);
  - huỷ không phải lỗi.
- **`finish_embeddings`** chỉ ghi khi dòng `pipeline_state` còn `running`, đúng version, **và** hash còn trong `files`; nếu không thì bỏ qua, không lỗi.
- **Chỉ mục ảnh** (nằm trong `fontatlas-index`, Task 10):
  - int8 đối xứng theo từng vector (`scale = 127/max|x|`), tính cho **mọi** ứng viên trục;
  - lấy 400 ứng viên tốt nhất theo khoảng cách kết hợp (trục + cos int8), xếp lại bằng cos f32 đọc từ SQLite qua **kết nối đọc riêng**.
- **S4:**
  - spec đòi R@1 ≥ 0,6, R@5 ≥ 0,9;
  - spike 03b (SigLIP 2, ảnh PIL, 400 face macOS, whiten): R@1 0,693, R@5 **0,898 — sát ngưỡng, chưa đạt**; fit chéo nửa họ: 0,721 / 0,926;
  - báo số thật của Rust ở Task 10; **không chỉnh renderer hay whitening để khớp ngưỡng**.
- Không Python lúc chạy app và trong CI. Test không cần mạng lúc **chạy** (server cục bộ, model giả commit sẵn). CI **cần mạng lúc build**: `ort` `download-binaries` tải ORT; nên cache `~/.cache/ort.pyke.io`.
- Không `unwrap()`/`expect()` trên dữ liệu từ font, file, DB hoặc model trong code thư viện. Sort số thực dùng `total_cmp`. Mọi phép chia đều kiểm mẫu số 0.
- Comment trong code viết tiếng Việt, ngắn gọn. Mỗi commit kết thúc bằng `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Model tải dở** (mạng đứt, app tắt giữa chừng): còn `.part` → trạng thái `Partial`, không bao giờ dùng file dở, lần sau tải lại. Test ở Task 5 (`interrupted_download_is_cancelled_and_partial`).
2. **Checksum sai** (lúc tải, hoặc file bị sửa sau đó): xoá file, trả `ChecksumMismatch` / `Corrupt`, chế độ về `MeasuresOnly`. Test ở Task 5 (`bad_checksum_rejected`, `tampered_ready_file_is_corrupt`, `oversize_body_aborted`).
3. **Huỷ giữa lô:** encoder chậm được tiêm qua trait, bảo đảm `terminate` chạy giữa lúc `run`. Không có dòng `embeddings` nào của lô; face về `pending`. Test ở Task 7 (`cancel_mid_run_writes_nothing`).
4. **App tắt giữa hàng đợi:** `running` → `pending`, chạy tiếp không trùng, không mất; `finish` muộn của phiên cũ không ghi được. Test ở Task 6 (`running_rows_reset_on_start`, `finish_ignores_non_running_or_lost_hash`) và Task 7 (`resume_after_restart_completes`).
5. **Đổi `model_version`:**
   - model, runtime hoặc render mới → embed lại toàn bộ;
   - chỉ whitening mới → whiten lại từ vector thô, **không** chạy model;
   - chỉ mục chỉ nạp version hiện tại.

   Test ở Task 6 (`new_model_version_requeues_all`, `rewhiten_from_raw_without_model`) và Task 8 (`other_versions_invisible`).
6. **Tắt embedding toàn cục:** worker không chạy, Similar về "Số đo", vector giữ nguyên; bật lại thì chạy tiếp. Test ở Task 6 (`disabled_enqueues_nothing`) và Task 7 (`worker_refuses_when_disabled`).

---

## File Structure

```
UX/src-tauri/
├── Cargo.toml                                    # Sửa: thêm "crates/fontatlas-embed" vào members
├── crates/fontatlas-core/src/anatomy.rs, lib.rs  # Sửa: pub fn standard_location (Task 1)
├── crates/fontatlas-core/tests/fixtures/public_sans_vf.ttf  # nếu dev/mvp chưa có (Task 2), cùng nguồn Plan 3
├── crates/fontatlas-store/
│   ├── src/embed.rs                              # Mới: hàng đợi, vector, cài đặt (Task 6)
│   ├── src/lib.rs                                # Sửa: pub use embed::*
│   └── tests/embed.rs                            # Mới
├── crates/fontatlas-embed/
│   ├── Cargo.toml                                # feature "fit" cho nalgebra + bin whitenfit
│   ├── data/model-manifest.json                  # url (hằng cấu hình), sha256, size, license (Task 9)
│   ├── data/model-NOTICE.txt, data/model-LICENSE-Apache-2.0.txt   # đi kèm file model (Task 9)
│   ├── data/whitening-v1.bin                     # μ[768] + W[768×128] (Task 9)
│   ├── src/lib.rs                                # hằng số, EmbedError, versions, EmbedMode, family_split
│   ├── src/render.rs                             # render_input, to_tensor (Task 2)
│   ├── src/encoder.rs                            # trait Encode + OrtEncoder (Task 3)
│   ├── src/whiten.rs                             # Whitening load/apply; fit (feature "fit") (Task 4)
│   ├── src/model.rs                              # ModelStore: tải, checksum, khoá (Task 5)
│   ├── src/worker.rs                             # EmbedWorker (Task 7)
│   ├── src/source.rs                             # StoreEmbeddingSource (Task 8)
│   ├── src/bin/whitenfit.rs                      # required-features = ["fit"] (Task 9)
│   ├── examples/render_fixtures.rs
│   └── tests/
│       ├── common/mod.rs
│       ├── data/tiny_encoder.onnx, tiny_bad_shape.onnx   # model giả (Task 3)
│       ├── data/render/*.png                     # ảnh vàng (Task 2)
│       ├── data/torch_ref.json                   # vector 768 chiều từ PyTorch cho ảnh vàng (Task 9)
│       └── render.rs encoder.rs whiten.rs model.rs worker.rs source.rs parity.rs
├── crates/fontatlas-index/src/{similar.rs,index.rs,embed_index.rs}   # Task 10
└── crates/fontatlas-eval/src/{metrics.rs,bin/eval.rs}                  # Task 10
tools/embed/
├── export_siglip2.py                             # export HF @75de2d5 → ONNX lô 4 (Task 9)
├── make_tiny_onnx.py                             # model giả, opset 17, ir_version 8 (Task 3)
└── torch_ref.py                                  # vector tham chiếu từ ảnh vàng (Task 9)
docs/team/plans/plan4-report.md                   # Task 10
```

Renderer để riêng trong crate này. Sau khi gộp với Plan 5 (`fontatlas-preview`), tách phần pen dùng chung (skrifa `OutlinePen` → `tiny_skia::PathBuilder`). Chuỗi mẫu, lề và chuẩn hoá vẫn ở đây.

---

### Task 1: Crate, hằng số, version, `standard_location` công khai

**Files:**
- Modify: `UX/src-tauri/Cargo.toml`, `crates/fontatlas-core/src/anatomy.rs`, `crates/fontatlas-core/src/lib.rs`
- Create: `crates/fontatlas-embed/Cargo.toml`, `src/lib.rs`
- Test: `src/lib.rs` (`#[cfg(test)]`), `crates/fontatlas-core/tests/anatomy.rs` (thêm)

**Interfaces:**
- Produces:
  - core: `pub fn standard_location(font: &FontRef) -> skrifa::instance::Location` (cùng vị trí với `MeasureAt::Standard`; `measure_with_filter` gọi lại hàm này).
  - Hằng số: `pub const MODEL_NAME: &str = "siglip2-base-p16-224@75de2d5";`, `pub const RUNTIME_ID: &str = "ort1.31-cpu";`, `pub const RENDER_ID: &str = "render-v2";`.
  - Hằng số: `pub const SAMPLE_LINES: [&str; 3] = ["Hamburg", "Người Việt", "Rgaeo 0123"];`, `pub const CANVAS: u32 = 224;`, `pub const MARGIN: f32 = 12.0;`, `pub const BATCH: usize = 4;`, `pub const RAW_DIM: usize = 768;`, `pub const DIM: usize = 128;`, `pub const NORM_MEAN: f32 = 0.5;`, `pub const NORM_STD: f32 = 0.5;`.
  - `pub fn raw_version(model_sha256: &str) -> String`, `pub fn model_version(raw_version: &str, whitening_version: &str, whitening_sha256: &str) -> String`.
  - `pub fn family_split(family: &str) -> &'static str` (`"tune"` hoặc `"test"`, theo Global Constraints).
  - `#[derive(Debug, thiserror::Error)] pub enum EmbedError { Io(#[from] std::io::Error), Font(String), NoGlyphs, Render(String), Model(String), ChecksumMismatch { expected: String, got: String }, Cancelled, Disabled, Store(#[from] fontatlas_store::StoreError) }`, với `fn code(&self) -> &'static str` trả `io`, `font`, `no_glyphs`, `render_failed`, `inference`, `checksum`, `cancelled`, `disabled`, `store`.
  - `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub enum EmbedMode { MeasuresOnly(ModeReason), Images { model_version: String } }`, `pub enum ModeReason { Disabled, ModelMissing, ModelPartial, ModelCorrupt, LoadFailed }`.

- [ ] **Step 1: Viết test thất bại**

```rust
#[test]
fn versions_compose() {
    let raw = raw_version(&"ab".repeat(32));
    assert_eq!(raw, format!("siglip2-base-p16-224@75de2d5|onnx-fp32-b4@{}|ort1.31-cpu|render-v2", "ab".repeat(6)));
    assert_eq!(model_version(&raw, "white-v1", &"cd".repeat(32)), format!("{raw}|white-v1@{}", "cd".repeat(6)));
}

#[test]
fn any_component_changes_version() {
    let r1 = raw_version(&"ab".repeat(32));
    assert_ne!(r1, raw_version(&"ac".repeat(32)));
    assert_ne!(model_version(&r1, "white-v1", &"cd".repeat(32)), model_version(&r1, "white-v2", &"cd".repeat(32)));
}

#[test]
fn family_split_deterministic_balanced_case_insensitive() {
    let t = (0..1000).filter(|i| family_split(&format!("Family {i}")) == "tune").count();
    assert!((400..600).contains(&t), "{t}");
    assert_eq!(family_split("Roboto"), family_split("roboto"));
}

#[test]
fn error_codes_stable() {
    assert_eq!(EmbedError::NoGlyphs.code(), "no_glyphs");
    assert_eq!(EmbedError::Cancelled.code(), "cancelled");
    assert_eq!(EmbedError::Disabled.code(), "disabled");
}
```

Thêm vào `crates/fontatlas-core/tests/anatomy.rs`:

```rust
#[test]
fn standard_location_is_public_and_consistent() {
    for name in SAMPLES {
        let f = fontatlas_core::FontFile::from_bytes(fixture_bytes(name)).unwrap();
        let faces = f.faces().unwrap();
        let _ = fontatlas_core::standard_location(faces[0].font());
        let a = faces[0].measure(fontatlas_core::MeasureAt::Standard);
        assert_eq!(a.location.is_empty(), faces[0].font().axes().is_empty(), "{name}");
    }
}
```

- [ ] **Step 2: Chạy** — `cd UX/src-tauri && cargo test -p fontatlas-embed -p fontatlas-core` → FAIL.
- [ ] **Step 3: Cài.** `Cargo.toml` của embed theo Tech Stack (ghim `=`, `ort` default-features = false). `[features] fit = ["dep:nalgebra"]`.
- [ ] **Step 4: Chạy** — PASS. Lần build đầu `ort` tải ORT binaries (cần mạng lúc build).
- [ ] **Step 5: Commit** — `feat(embed): crate skeleton, versions (model+runtime+render+whitening), family_split, core standard_location pub`

---

### Task 2: Render ảnh đầu vào (`render.rs`)

**Files:**
- Create: `crates/fontatlas-embed/src/render.rs`, `examples/render_fixtures.rs`, `tests/common/mod.rs`, `tests/render.rs`, `tests/data/render/*.png`
- Create (nếu `dev/mvp` chưa có): `crates/fontatlas-core/tests/fixtures/public_sans_vf.ttf` và dòng tương ứng trong README fixture. Lệnh tải và nguồn **giống hệt** Plan 3 Task 1 Step 0 (`google/fonts@47d54ca…:ofl/publicsans/PublicSans[wght].ttf`).

**Interfaces:**
- Consumes: skrifa (`charmap`, `glyph_metrics`, `metrics`, `outline_glyphs`), `standard_location`, hằng số Task 1
- Produces:
  - `pub struct Gray224(pub Vec<u8>)` — 224·224 byte, 0 = đen
  - `pub fn coverage_ratio(font: &FontRef) -> f32`
  - `pub fn render_input(font: &FontRef, loc: &Location) -> Result<Gray224, EmbedError>`
  - `pub fn to_tensor(img: &Gray224, out: &mut [f32])` — `out.len() == 3·224·224`, thứ tự CHW, giá trị `(v/255 − 0.5)/0.5`

Thuật toán:
1. `coverage_ratio < 0.5` → `Err(NoGlyphs)`. Ký tự thiếu thì vẽ glyph 0.
2. Đơn vị font: `m = font.metrics(Size::unscaled(), loc)`, `lh = m.ascent − m.descent`, `wᵢ = Σ advance`. Không kerning, không shaping; chữ Việt dùng ký tự dựng sẵn.
3. `s = min((224−24)/max wᵢ, (224−24)/(3·lh))`. Nếu `lh ≤ 0` hoặc `max wᵢ ≤ 0` → `Err(Render)`.
4. Baseline dòng i: `y = (224 − 3·lh·s)/2 + i·lh·s + ascent·s`; gốc dòng: `x = (224 − wᵢ·s)/2`.
5. Vẽ outline (pen → `PathBuilder`, lật trục y), fill đen bằng `FillRule::Winding` có anti-alias trên `Pixmap` trắng; lấy kênh R.

- [ ] **Step 1: Viết test thất bại**

```rust
#[test]
fn renders_with_ink_inside_margins() {
    for f in SAMPLES_LATIN {
        let img = render_fixture(f).unwrap();
        assert_eq!(img.0.len(), 224 * 224);
        let ink: Vec<usize> = (0..224 * 224).filter(|i| img.0[*i] < 128).collect();
        assert!(ink.len() > 2000, "{f}: {}", ink.len());
        let xs = ink.iter().map(|i| i % 224);
        assert!(xs.clone().min().unwrap() >= 10 && xs.max().unwrap() <= 213, "{f}");
    }
}

#[test]
fn tensor_normalisation_is_siglip() {
    let mut img = Gray224(vec![255; 224 * 224]); img.0[0] = 0;
    let mut t = vec![0f32; 3 * 224 * 224]; to_tensor(&img, &mut t);
    assert_eq!(t[0], -1.0);
    assert_eq!(t[1], 1.0);
    assert_eq!(t[224 * 224], -1.0); // kênh G cùng giá trị
}

#[test]
fn golden_png_matches() {
    for f in SAMPLES_LATIN {
        let img = render_fixture(f).unwrap();
        let gold = load_golden_png(f); // crate png =0.18.1, kênh R
        let bad = img.0.iter().zip(&gold).filter(|(a, b)| (**a as i16 - **b as i16).abs() > 2).count();
        assert!(bad * 200 <= 224 * 224, "{f}: {bad} pixel lệch > 2");
    }
}

#[test]
fn missing_vietnamese_still_renders() {
    assert!(render_fixture("libre_baskerville.ttf").is_ok()); // > 50% ký tự có
}

#[test]
fn no_cmap_is_no_glyphs() {
    // đổi tên bảng cmap → font không còn ký tự nào (helper tự viết, không phụ thuộc Plan 5)
    let bytes = rename_table(fixture_bytes("roboto.ttf"), b"cmap", b"cmaq");
    assert!(matches!(render_bytes(bytes, 0), Err(EmbedError::NoGlyphs)));
}
```

Helper trong `tests/common/mod.rs`:
- `SAMPLES_LATIN = ["roboto.ttf", "open_sans.ttf", "playfair_display.ttf", "source_code_pro.ttf", "public_sans_vf.ttf", "libre_baskerville.ttf"]`;
- `fixture`, `fixture_bytes` trỏ `../fontatlas-core/tests/fixtures`;
- `render_fixture(name)`, `render_bytes(Vec<u8>, index)`: mở font, lấy face, gọi `render_input(font, &standard_location(font))`;
- `rename_table(bytes, from, to)`: chép logic của `fontatlas-core/tests/common/mod.rs` — đổi tag trong table directory, giữ thứ tự (`cmap` → `cmaq` vẫn đúng thứ tự sort);
- `load_golden_png(name)`: dùng `png::Decoder`, lấy kênh R;
- `data(name)`, `tensor_of_gray(v)`, `norm`, `cos`, `pearson`;
- `lcg_clusters(n, dim, centers, seed) -> Vec<Vec<f32>>`. Mỗi crate test tự có bản riêng; không dùng chung qua dev-dep, để tránh vòng phụ thuộc.

- [ ] **Step 2: Chạy** — FAIL.
- [ ] **Step 3: Cài `render.rs` và `examples/render_fixtures.rs`.** Example ghi `tests/data/render/<fixture>.png` cho 6 fixture bằng crate `png`.
- [ ] **Step 4: Sinh ảnh vàng, kiểm bằng mắt, chạy test.** Chạy `cargo run -p fontatlas-embed --example render_fixtures`, mở 6 PNG và xác nhận: 3 dòng căn giữa, dấu tiếng Việt đúng chỗ, Libre Baskerville có ô `.notdef`. Chạy test → PASS.
- [ ] **Step 5: Commit** (kèm PNG) — `feat(embed): render-v2 sample image (skrifa + tiny-skia 0.12) with golden PNGs`

---

### Task 3: Encoder (`encoder.rs`) — trait để tiêm, `OrtEncoder`, model giả

**Files:**
- Create: `crates/fontatlas-embed/src/encoder.rs`, `tools/embed/make_tiny_onnx.py`, `tests/data/tiny_encoder.onnx`, `tests/data/tiny_bad_shape.onnx`, `tests/encoder.rs`

**Interfaces:**
- Produces:
  - `pub trait Encode: Send { fn encode(&mut self, tensors: &[Vec<f32>], cancel: &AtomicBool) -> Result<Vec<Vec<f32>>, EmbedError>; fn raw_dim(&self) -> usize; }`
  - `pub struct OrtEncoder`; `impl OrtEncoder { pub fn load(path: &Path, intra_threads: usize) -> Result<Self, EmbedError> }`; `impl Encode for OrtEncoder`
  - `encode` nhận 1..=`BATCH` tensor:
    - thiếu thì pad bằng ảnh trắng; trả đúng số vector bằng số tensor vào;
    - mảng rỗng → `Ok(vec![])`; nhiều hơn `BATCH` → `Err(Model)`.
  - `load` kiểm input `[4,3,224,224]` và output `[4, RAW_DIM]`; sai → `Err(Model("shape"))`.
  - Huỷ:
    - luồng phụ chờ cờ `cancel` (park 20 ms), rồi gọi `RunOptions::terminate()`;
    - `run` trả lỗi → `Err(Cancelled)`;
    - cờ đã bật trước khi chạy → `Err(Cancelled)` ngay.
  - `Environment` của ort dùng chung qua `OnceLock`.

`tools/embed/make_tiny_onnx.py`: chạy offline bằng `uv run --with onnx==1.17.0`; **ghim** `opset_imports=[17]`, `ir_version=8`. Script tạo hai file:
- `tiny_encoder.onnx`: `pixel_values[4,3,224,224]` → `ReduceMean(axes=[2,3], keepdims=0)` → `[4,3]` → `MatMul W[3,768]` → `image_embeds[4,768]`, với `W[c][j] = (j%3==c) ? (j+1)/768 : 0`;
- `tiny_bad_shape.onnx`: giống trên nhưng output `[4,512]`.

- [ ] **Step 1: Viết test thất bại**

```rust
fn expected(t: &[f32]) -> Vec<f32> {
    let hw = 224 * 224;
    let m: Vec<f32> = (0..3).map(|c| t[c * hw..(c + 1) * hw].iter().sum::<f32>() / hw as f32).collect();
    (0..768).map(|j| m[j % 3] * (j + 1) as f32 / 768.0).collect()
}

#[test]
fn tiny_model_matches_formula_and_pads() {
    let mut e = OrtEncoder::load(&data("tiny_encoder.onnx"), 2).unwrap();
    let ts: Vec<Vec<f32>> = (0..3).map(|i| tensor_of_gray((i * 80) as u8)).collect(); // 3 < 4 → pad
    let out = e.encode(&ts, &AtomicBool::new(false)).unwrap();
    assert_eq!(out.len(), 3);
    for (t, o) in ts.iter().zip(&out) { assert!(o.iter().zip(expected(t)).all(|(a, b)| (a - b).abs() < 1e-4)); }
}

#[test]
fn batch_limits() {
    let mut e = OrtEncoder::load(&data("tiny_encoder.onnx"), 1).unwrap();
    assert!(e.encode(&vec![tensor_of_gray(255); 5], &AtomicBool::new(false)).is_err());
    assert!(e.encode(&[], &AtomicBool::new(false)).unwrap().is_empty());
}

#[test]
fn cancelled_before_run() {
    let mut e = OrtEncoder::load(&data("tiny_encoder.onnx"), 1).unwrap();
    assert!(matches!(e.encode(&[tensor_of_gray(255)], &AtomicBool::new(true)), Err(EmbedError::Cancelled)));
}

#[test]
fn wrong_output_shape_rejected() {
    assert!(matches!(OrtEncoder::load(&data("tiny_bad_shape.onnx"), 1), Err(EmbedError::Model(_))));
}

#[test]
fn garbage_file_rejected() {
    let p = tempfile::NamedTempFile::new().unwrap(); std::fs::write(p.path(), b"not onnx").unwrap();
    assert!(matches!(OrtEncoder::load(p.path(), 1), Err(EmbedError::Model(_))));
}
```

- [ ] **Step 2: Chạy** — FAIL.
- [ ] **Step 3: Chạy script tạo hai model giả, rồi cài `encoder.rs`.**
- [ ] **Step 4: Chạy** — PASS.
- [ ] **Step 5: Commit** — `feat(embed): Encode trait + OrtEncoder (static batch 4, padding, terminate on cancel); tiny models opset 17`

---

### Task 4: Whitening (`whiten.rs`)

**Files:**
- Create: `crates/fontatlas-embed/src/whiten.rs`, `tests/whiten.rs`

**Interfaces:**
- Produces:
  - Định dạng file, theo thứ tự:
    - 8 byte `b"FAWHITEN"`;
    - `u32` LE: độ dài header;
    - header JSON `{"version":"white-v1","raw_dim":768,"dim":128,"shrink":0.1,"source":"…","split":"tune","n":…}`;
    - μ: `raw_dim` số f32 LE;
    - W: `raw_dim×dim` số f32 LE, hàng-trước; cột i = vᵢ/(√λᵢ + shrink·√λ₁).
  - `pub struct Whitening { pub version: String, pub sha256: String, pub raw_dim: usize, pub dim: usize, mean: Vec<f32>, w: Vec<f32> }`
  - `impl Whitening { pub fn from_bytes(b: &[u8]) -> Result<Self, EmbedError>; pub fn bundled() -> Self /* Task 9 */; pub fn apply(&self, raw: &[f32]) -> Option<Vec<f32>>; pub fn to_bytes(&self, header: &serde_json::Value) -> Vec<u8> }`
  - `#[cfg(feature = "fit")] pub fn fit(raw: &[Vec<f32>], dim: usize, shrink: f32) -> Result<Whitening, EmbedError>`: dùng nalgebra `SymmetricEigen`. Ít hơn `dim + 1` mẫu, hoặc các vector không cùng số chiều → `Err`.
  - `apply`: `raw.len() != raw_dim` → `None`; các bước L2 → trừ μ → nhân W → L2; chuẩn bằng 0 ở bất kỳ bước nào → `None`.

- [ ] **Step 1: Viết test thất bại** (`tests/whiten.rs`; các test cần `fit` đánh dấu `#[cfg(feature = "fit")]`)

```rust
#[cfg(feature = "fit")]
#[test]
fn fit_apply_unit_and_decorrelated() {
    let data = lcg_clusters(3000, 768, 10, 20261018);
    let w = fit(&data, 128, 0.1).unwrap();
    let out: Vec<Vec<f32>> = data.iter().filter_map(|x| w.apply(x)).collect();
    assert_eq!(out.len(), data.len());
    assert!(out.iter().all(|v| v.len() == 128 && (norm(v) - 1.0).abs() < 1e-4));
    assert!(pearson(&col(&out, 0), &col(&out, 1)).abs() < 0.1);
}

#[cfg(feature = "fit")]
#[test]
fn roundtrip_bytes_and_sha() {
    let w = fit(&lcg_clusters(500, 768, 10, 1), 128, 0.1).unwrap();
    let b = w.to_bytes(&serde_json::json!({"version":"white-test","raw_dim":768,"dim":128,"shrink":0.1,"source":"t","split":"tune","n":500}));
    let r = Whitening::from_bytes(&b).unwrap();
    assert_eq!((r.version.as_str(), r.sha256.len()), ("white-test", 64));
    let x = &lcg_clusters(1, 768, 1, 9)[0];
    assert_eq!(w.apply(x), r.apply(x));
}

#[test]
fn rejects_bad_files_wrong_dim_and_zero() {
    assert!(Whitening::from_bytes(b"FAWHITEN\x05\x00\x00\x00{bad}").is_err());
    let w = Whitening::from_bytes(&identity_whitening_bytes(768, 128)).unwrap(); // helper: μ=0, W=[I;0]
    assert!(w.apply(&vec![0.0; 768]).is_none());
    assert!(w.apply(&vec![1.0; 512]).is_none());
}

#[cfg(feature = "fit")]
#[test]
fn too_few_samples() { assert!(fit(&lcg_clusters(100, 768, 3, 3), 128, 0.1).is_err()); }
```

- [ ] **Step 2: Chạy** — `cargo test -p fontatlas-embed --features fit --test whiten` → FAIL.
- [ ] **Step 3: Cài.** `sha256` tính trên toàn bộ byte file.
- [ ] **Step 4: Chạy** — PASS, cả khi có và không có `--features fit`.
- [ ] **Step 5: Commit** — `feat(embed): whitening file format + apply; PCA fit behind feature "fit"`

---

### Task 5: Tải và kiểm model (`model.rs`)

**Files:**
- Create: `crates/fontatlas-embed/src/model.rs`, `tests/model.rs`

**Interfaces:**
- Produces:
  - `#[derive(Deserialize)] pub struct Manifest { pub file: String, pub url: String, pub sha256: String, pub size: u64, pub license: String, pub notice_files: Vec<String> }`
  - `pub enum ModelStatus { Missing, Partial, Ready(PathBuf), Corrupt }`
  - `pub struct ModelStore`, với:
    - `pub fn new(dir: PathBuf, m: Manifest) -> Self`;
    - `pub fn file_name(&self) -> &str`;
    - `pub fn status(&self) -> ModelStatus`;
    - `pub fn ensure(&self, progress: &dyn Fn(u64, u64), cancel: &AtomicBool) -> Result<PathBuf, EmbedError>`;
    - `pub fn remove(&self) -> std::io::Result<()>`.
  - `pub fn sha256_file(p: &Path) -> std::io::Result<String>`
  - `pub const READ_TIMEOUT: Duration = Duration::from_secs(30);`

Quy tắc:
- **`status`:**
  - `<file>` có và `.ok` khớp (sha256 bằng manifest, size và mtime bằng file) → `Ready`;
  - `<file>` có nhưng `.ok` thiếu hoặc lệch → băm lại; khớp thì ghi `.ok` và trả `Ready`, không khớp → `Corrupt`;
  - chỉ có `.part` → `Partial`; không có gì → `Missing`.
- **`ensure`:**
  1. Lấy khoá `<file>.lock` (`File::lock`, chờ).
  2. `Ready` → trả luôn. Ngược lại xoá `.part` và file `Corrupt` nếu có.
  3. GET bằng `ureq`: timeout kết nối 30 s, timeout đọc `READ_TIMEOUT`. HTTP khác 200 → `Err(Io)`.
  4. Đọc từng chunk 64 KB. **Mỗi chunk**:
     - kiểm `cancel`: bật thì xoá `.part`, trả `Err(Cancelled)`;
     - cộng dồn sha256 và số byte; số byte vượt `size` → dừng ngay, xoá `.part`, trả `Err(ChecksumMismatch)`.
  5. Gọi `progress` sau mỗi 1 MB.
  6. Xong: size hoặc sha256 sai → xoá, `Err(ChecksumMismatch)`; đúng → `sync_all` file, rename, `sync_all` thư mục, ghi `.ok`.

- [ ] **Step 1: Viết test thất bại** (server `tiny_http` ở `127.0.0.1:0`)

Helper:
- `serve_bytes(n)`;
- `serve_slow(total, chunk, delay)`: gửi `Content-Length: total`, rồi từng chunk cách nhau `delay`;
- `serve_oversize(declared, actual)`;
- `serve_status(code)`;
- `serve_bytes_counting(n)`: như `serve_bytes`, có `requests()` đếm số request;
- `manifest(url, sha, size)`, `sha_hex`.

```rust
#[test]
fn downloads_verifies_and_marks_ready() {
    let (_s, url, body) = serve_bytes(3 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&body), body.len() as u64));
    assert!(matches!(st.status(), ModelStatus::Missing));
    let p = st.ensure(&|_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), body);
    assert!(matches!(st.status(), ModelStatus::Ready(_)));
}

#[test]
fn bad_checksum_rejected() {
    let (_s, url, body) = serve_bytes(1 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &"00".repeat(32), body.len() as u64));
    assert!(matches!(st.ensure(&|_, _| {}, &AtomicBool::new(false)), Err(EmbedError::ChecksumMismatch { .. })));
    assert!(matches!(st.status(), ModelStatus::Missing));
}

#[test]
fn oversize_body_aborted() {
    let (_s, url) = serve_oversize(1 << 20, 4 << 20); // khai 1 MB, gửi 4 MB
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &"00".repeat(32), 1 << 20));
    assert!(matches!(st.ensure(&|_, _| {}, &AtomicBool::new(false)), Err(EmbedError::ChecksumMismatch { .. })));
}

#[test]
fn interrupted_download_is_cancelled_and_partial() {
    let (_s, url, body) = serve_slow(4 << 20, 64 << 10, Duration::from_millis(20));
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&body), body.len() as u64));
    let cancel = AtomicBool::new(false);
    let seen = AtomicU64::new(0);
    let r = std::thread::scope(|sc| {
        sc.spawn(|| { while seen.load(Ordering::SeqCst) < (1 << 20) { std::thread::sleep(Duration::from_millis(5)); } cancel.store(true, Ordering::SeqCst); });
        st.ensure(&|done, _| seen.store(done, Ordering::SeqCst), &cancel)
    });
    assert!(matches!(r, Err(EmbedError::Cancelled)), "{r:?}");
    // app chết giữa chừng: tự tạo .part
    std::fs::write(d.path().join(format!("{}.part", st.file_name())), &body[..1000]).unwrap();
    assert!(matches!(st.status(), ModelStatus::Partial));
}

#[test]
fn tampered_ready_file_is_corrupt() {
    let (_s, url, body) = serve_bytes(1 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&body), body.len() as u64));
    let p = st.ensure(&|_, _| {}, &AtomicBool::new(false)).unwrap();
    let mut b = std::fs::read(&p).unwrap(); b[10] ^= 0xff; std::fs::write(&p, b).unwrap();
    assert!(matches!(st.status(), ModelStatus::Corrupt));
}

#[test]
fn http_error_is_io() {
    let (_s, url) = serve_status(404);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &"00".repeat(32), 10));
    assert!(matches!(st.ensure(&|_, _| {}, &AtomicBool::new(false)), Err(EmbedError::Io(_))));
}

#[test]
fn concurrent_ensure_downloads_once() {
    let (srv, url, body) = serve_bytes_counting(1 << 20);
    let d = tempfile::tempdir().unwrap();
    let st = ModelStore::new(d.path().into(), manifest(&url, &sha_hex(&body), body.len() as u64));
    std::thread::scope(|sc| { for _ in 0..3 { sc.spawn(|| st.ensure(&|_, _| {}, &AtomicBool::new(false)).unwrap()); } });
    assert_eq!(srv.requests(), 1);
}
```

- [ ] **Step 2: Chạy** — FAIL.
- [ ] **Step 3: Cài `model.rs`.**
- [ ] **Step 4: Chạy** — PASS (chỉ dùng localhost).
- [ ] **Step 5: Commit** — `feat(embed): model download (.part, per-chunk cancel, size guard, sha256, fsync, lock, .ok)`

---

### Task 6: Store — hàng đợi, vector, cài đặt (`fontatlas-store/src/embed.rs`)

**Files:**
- Create: `UX/src-tauri/crates/fontatlas-store/src/embed.rs`, `tests/embed.rs`
- Modify: `src/lib.rs` (`pub use embed::*`)

**Interfaces** (trên `impl Store`; mọi transaction ghi dùng `TransactionBehavior::Immediate`):
- Cài đặt:
  - `pub const SETTING_EMBED_ENABLED: &str = "embed_enabled";`, `pub const SETTING_LICENSE_NOTICE_ACK: &str = "license_notice_ack";`
  - `pub fn setting(&self, key: &str) -> Result<Option<String>, StoreError>`, `pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError>`
  - `pub fn embed_enabled(&self) -> Result<bool, StoreError>` (thiếu khoá → `true`)
- Hàng đợi:
  - `pub struct EnqueueStats { pub pending: usize, pub already_done: usize }`
  - `pub fn enqueue_embeddings(&mut self, raw_version: &str) -> Result<EnqueueStats, StoreError>` — đang tắt thì trả `Ok` rỗng
  - `pub fn reset_running_embeddings(&mut self) -> Result<usize, StoreError>`
  - `pub struct EmbedJob { pub key: FaceKey, pub path: PathBuf }`
  - `pub fn claim_embedding_batch(&mut self, raw_version: &str, n: usize) -> Result<Vec<EmbedJob>, StoreError>`
  - `pub enum JobResult { Done { raw: Vec<f32>, white: Option<Vec<f32>> }, Failed { code: &'static str, detail: String }, Skipped { code: &'static str } }`
  - `pub fn finish_embeddings(&mut self, raw_version: &str, model_version: &str, results: &[(FaceKey, JobResult)]) -> Result<usize /* số dòng đã ghi */, StoreError>`
  - `pub fn release_embeddings(&mut self, raw_version: &str, keys: &[FaceKey]) -> Result<(), StoreError>`
- Vector:
  - `pub fn rewhiten(&mut self, raw_version: &str, model_version: &str, apply: &dyn Fn(&[f32]) -> Option<Vec<f32>>) -> Result<usize, StoreError>`
  - `pub fn gc_embeddings(&mut self, keep: &[&str]) -> Result<usize, StoreError>`
  - `pub fn count_embeddings(&self, version: &str) -> Result<usize, StoreError>`
  - `pub fn for_each_embedding(&self, version: &str, f: &mut dyn FnMut(FaceKey, &[f32])) -> Result<(), StoreError>`
  - `pub fn embeddings_f32(&self, version: &str, keys: &[FaceKey]) -> Result<Vec<Option<Vec<f32>>>, StoreError>`
- `pub fn open_read_only(dir: &Path) -> Result<Store, StoreError>`: kết nối đọc riêng cho truy vấn (`SQLITE_OPEN_READ_ONLY`, ATTACH `user.db` read-only).

Quy tắc:
- `pipeline_state` stage `embedding` dùng `version = raw_version`; whitening không nằm trong hàng đợi.
- **`enqueue`** (một transaction), với mỗi face đã phân tích `done`:
  - đã có dòng `embeddings(.., raw_version)` → tính vào `already_done`;
  - chưa có dòng, khác version, hoặc đang `failed` → `pending`;
  - `skipped` (`no_glyphs`) cùng version thì giữ nguyên.
- **`claim`:**
  - chọn tối đa n dòng `pending` với `version = raw_version`, sắp theo `updated_at`;
  - JOIN `files` + `file_locations` (`missing = 0`) để lấy path; không có vị trí nào → `failed`/`missing_file`;
  - đánh `running` trong cùng transaction.
- **`finish`** (một transaction), với mỗi kết quả:
  - **chỉ ghi khi** dòng còn `status = 'running'` ∧ `version = raw_version` ∧ `EXISTS (SELECT 1 FROM files WHERE hash = ?)`; nếu không thì bỏ qua;
  - `Done`: `INSERT OR REPLACE` dòng thô (`dim = raw.len()`) và, nếu có, dòng whiten (`model_version`, `dim = 128`); rồi đánh `done`;
  - `Failed` → `failed` + mã; `Skipped` → `skipped` + mã.
- **`rewhiten`:** với mọi dòng thô của `raw_version` chưa có dòng `model_version`, gọi `apply` rồi ghi. Không đụng `pipeline_state`.
- **`gc_embeddings`:** `DELETE FROM embeddings WHERE source = 'local' AND model_version NOT IN (keep…)`.

- [ ] **Step 1: Viết test thất bại**

Helper:
- `store_with(&[fixture])`: thư mục tạm + `add_source` + `rescan_source`;
- `reopen(Store)`;
- `state_of(&Store, FaceKey, raw) -> (String, Option<String>)`;
- `done(raw_dim) -> JobResult`;
- `finish_all(&mut Store, raw, mv)`: enqueue + claim + finish `done(768)` cho mọi face.

```rust
const RAW: &str = "raw1"; const MV: &str = "raw1|white-a";

#[test]
fn enqueues_all_fonts_regardless_of_license() {
    let mut s = store_with(&["roboto.ttf", "open_sans.ttf"]);
    s.conn().execute("UPDATE faces SET license_class='other'", []).unwrap(); // font "thương mại" vẫn được embed
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 2);
}

#[test]
fn disabled_enqueues_nothing() {
    let mut s = store_with(&["roboto.ttf"]);
    assert!(s.embed_enabled().unwrap()); // mặc định bật
    s.set_setting(SETTING_EMBED_ENABLED, "0").unwrap();
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 0);
    s.set_setting(SETTING_EMBED_ENABLED, "1").unwrap();
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 1);
}

#[test]
fn license_notice_flag_roundtrip() {
    let mut s = store_with(&[]);
    assert_eq!(s.setting(SETTING_LICENSE_NOTICE_ACK).unwrap(), None);
    s.set_setting(SETTING_LICENSE_NOTICE_ACK, "1").unwrap();
    assert_eq!(s.setting(SETTING_LICENSE_NOTICE_ACK).unwrap().as_deref(), Some("1"));
}

#[test]
fn running_rows_reset_on_start() {
    let mut s = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap(); assert_eq!(s.claim_embedding_batch(RAW, 4).unwrap().len(), 1);
    let mut s2 = reopen(s);
    assert_eq!(s2.reset_running_embeddings().unwrap(), 1);
    assert_eq!(s2.claim_embedding_batch(RAW, 4).unwrap().len(), 1);
}

#[test]
fn finish_ignores_non_running_or_lost_hash() {
    let mut s = store_with(&["roboto.ttf", "playfair_display.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 4).unwrap();
    s.release_embeddings(RAW, &[j[0].key]).unwrap(); // không còn running
    s.conn().execute("DELETE FROM files WHERE hash = ?1", [&j[1].key.hash[..]]).unwrap(); // hash mất
    let n = s.finish_embeddings(RAW, MV, &[(j[0].key, done(768)), (j[1].key, done(768))]).unwrap();
    assert_eq!(n, 0);
    assert_eq!(s.count_embeddings(RAW).unwrap(), 0);
}

#[test]
fn new_model_version_requeues_all() {
    let mut s = store_with(&["roboto.ttf", "playfair_display.ttf"]);
    finish_all(&mut s, RAW, MV);
    assert_eq!(s.enqueue_embeddings("raw2").unwrap().pending, 2);
}

#[test]
fn rewhiten_from_raw_without_model() {
    let mut s = store_with(&["roboto.ttf", "playfair_display.ttf"]);
    finish_all(&mut s, RAW, MV);
    assert_eq!(s.rewhiten(RAW, "raw1|white-b", &|r| Some(r[..128].to_vec())).unwrap(), 2);
    assert_eq!(s.count_embeddings("raw1|white-b").unwrap(), 2);
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 0); // không phải chạy lại model
    assert_eq!(s.gc_embeddings(&[RAW, "raw1|white-b"]).unwrap(), 2); // xoá white-a
}

#[test]
fn no_glyphs_is_skipped_and_not_requeued() {
    let mut s = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 4).unwrap();
    s.finish_embeddings(RAW, MV, &[(j[0].key, JobResult::Skipped { code: "no_glyphs" })]).unwrap();
    assert_eq!(state_of(&s, j[0].key, RAW), ("skipped".into(), Some("no_glyphs".into())));
    assert_eq!(s.enqueue_embeddings(RAW).unwrap().pending, 0);
}

#[test]
fn f32_roundtrip_stream_and_read_only_connection() {
    let mut s = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 1).unwrap();
    let raw: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
    s.finish_embeddings(RAW, MV, &[(j[0].key, JobResult::Done { raw: raw.clone(), white: Some(raw[..128].to_vec()) })]).unwrap();
    let ro = Store::open_read_only(s.dir()).unwrap();
    assert_eq!(ro.embeddings_f32(RAW, &[j[0].key]).unwrap()[0].as_deref(), Some(&raw[..]));
    let mut n = 0; ro.for_each_embedding(MV, &mut |_, v| { assert_eq!(v.len(), 128); n += 1 }).unwrap(); assert_eq!(n, 1);
    assert!(ro.conn().execute("DELETE FROM embeddings", []).is_err()); // chỉ đọc
}

#[test]
fn retry_failed_requeues_embedding_failures() {
    let mut s = store_with(&["roboto.ttf"]);
    s.enqueue_embeddings(RAW).unwrap();
    let j = s.claim_embedding_batch(RAW, 1).unwrap();
    s.finish_embeddings(RAW, MV, &[(j[0].key, JobResult::Failed { code: "inference", detail: "x".into() })]).unwrap();
    s.retry_failed().unwrap();
    assert_eq!(s.claim_embedding_batch(RAW, 1).unwrap().len(), 1);
}
```

- [ ] **Step 2: Chạy** — `cargo test -p fontatlas-store --test embed` → FAIL.
- [ ] **Step 3: Cài `embed.rs`.** Chỉ `set_setting` ghi `user.db`; các API khác chỉ ghi `library.db`, đúng ràng buộc Plan 2. Nếu `retry_failed()` của Plan 2 lọc theo stage, mở rộng thêm stage `embedding`.
- [ ] **Step 4: Chạy** — `cargo test -p fontatlas-store` → PASS, kể cả test cũ.
- [ ] **Step 5: Commit** — `feat(store): embedding queue (immediate tx, guarded finish), raw+whitened vectors, rewhiten, settings embed_enabled/license_notice_ack, read-only connection`

---

### Task 7: Hàng đợi nền (`worker.rs`)

**Files:**
- Create: `crates/fontatlas-embed/src/worker.rs`, `tests/worker.rs`

**Interfaces:**
- Consumes: `Store` (Task 6), `render_input`, `to_tensor`, `standard_location`, `Encode`, `Whitening::apply`, `fontatlas_core::FontFile`
- Produces:
  - `pub struct WorkerConfig { pub store_dir: PathBuf, pub raw_version: String, pub whitening: Option<Whitening>, pub render_threads: usize }`
  - `pub enum Progress { Started { pending: usize }, Batch { done: usize, failed: usize, skipped: usize }, Paused, Resumed, Cancelled, Finished { total_done: usize }, Error(String) }` (`Debug, Clone`)
  - `pub struct EmbedWorker`, với:
    - `pub fn start(cfg: WorkerConfig, encoder: Box<dyn Encode>, on_progress: Box<dyn Fn(Progress) + Send>) -> Result<EmbedWorker, EmbedError>`;
    - `pub fn pause(&self)`, `pub fn resume(&self)`, `pub fn cancel(&self)`;
    - `pub fn join(self) -> Result<(), EmbedError>`;
    - `pub fn model_version(&self) -> Option<&str>`.
  - `start` trả `Err(Disabled)` khi `embed_enabled = false`.

Vòng lặp (chạy trên luồng riêng, mở `Store::open(store_dir)` riêng):
1. `reset_running_embeddings`, `enqueue_embeddings(raw)`, phát `Started`.
2. Lặp:
   - `pause` bật → phát `Paused` **một lần**, park mỗi 50 ms cho tới khi tắt, rồi phát `Resumed`;
   - `cancel` bật → phát `Cancelled`, thoát;
   - `claim(raw, BATCH)`; nếu rỗng thì gọi `enqueue_embeddings(raw)` **một lần**. Vẫn rỗng → `gc_embeddings(&[raw, mv])` (chỉ khi có whitening), phát `Finished`, thoát.
3. Xử lý từng job trên pool riêng, bọc `catch_unwind`; panic → `Failed{render_failed}`:
   - đọc file; `blake3` khác key → `Failed{stale_file}`;
   - mở face, gọi `render_input`: `NoGlyphs` → `Skipped{no_glyphs}`, lỗi khác → `Failed{render_failed}`;
   - `to_tensor`.
4. `encoder.encode(ok_tensors, &cancel)`:
   - `Err(Cancelled)` → `release_embeddings(raw, mọi key của lô)`, phát `Cancelled`, thoát, **không gọi `finish`**;
   - lỗi khác → `Failed{inference}`.
5. `white = whitening.apply(raw)`; gọi `finish_embeddings` một lần cho cả lô; phát `Batch`.

- [ ] **Step 1: Viết test thất bại**

Helper:
- `setup(&[fixture]) -> (TempDir, WorkerConfig)`: `store_with` như Task 6, `whitening = Some(identity_whitening(768, 128))`;
- `setup_many(fixture, n)`: n bản sao, mỗi bản thêm `i` byte 0 vào cuối file;
- `tiny()` = `Box::new(OrtEncoder::load(tiny_encoder.onnx, 1))`;
- `SlowEncoder { inner, in_run: Arc<Barrier>, delay }`: báo vào barrier, rồi gọi `inner` nhưng kiểm `cancel` mỗi 10 ms và trả `Cancelled` khi cờ bật — mô phỏng `terminate` giữa lúc `run`;
- `key_of`, `state_of`, `count_status`, `append_byte_to_font_file(dir, fixture)`, `add_font_and_rescan(dir, fixture)`.

```rust
#[test]
fn processes_all_fonts_and_finishes() {
    let (dir, cfg) = setup(&["roboto.ttf", "playfair_display.ttf", "source_code_pro.ttf", "open_sans.ttf"]);
    let w = EmbedWorker::start(cfg.clone(), tiny(), Box::new(|_| {})).unwrap();
    let mv = w.model_version().unwrap().to_string(); w.join().unwrap();
    let s = Store::open(dir.path()).unwrap().0;
    assert_eq!((s.count_embeddings(&cfg.raw_version).unwrap(), s.count_embeddings(&mv).unwrap()), (4, 4));
}

#[test]
fn worker_refuses_when_disabled() {
    let (dir, cfg) = setup(&["roboto.ttf"]);
    Store::open(dir.path()).unwrap().0.set_setting(SETTING_EMBED_ENABLED, "0").unwrap();
    assert!(matches!(EmbedWorker::start(cfg, tiny(), Box::new(|_| {})), Err(EmbedError::Disabled)));
}

#[test]
fn cancel_mid_run_writes_nothing() {
    let (dir, cfg) = setup_many("roboto.ttf", 8);
    let in_run = Arc::new(Barrier::new(2));
    let enc = Box::new(SlowEncoder::new(tiny(), in_run.clone(), Duration::from_secs(5)));
    let w = EmbedWorker::start(cfg.clone(), enc, Box::new(|_| {})).unwrap();
    in_run.wait(); // chắc chắn đang ở giữa encode
    w.cancel(); w.join().unwrap();
    let s = Store::open(dir.path()).unwrap().0;
    assert_eq!(s.count_embeddings(&cfg.raw_version).unwrap(), 0);
    assert_eq!(count_status(&s, "running"), 0);
    assert_eq!(count_status(&s, "pending"), 8);
}

#[test]
fn resume_after_restart_completes() {
    let (dir, cfg) = setup_many("roboto.ttf", 10);
    { let mut s = Store::open(dir.path()).unwrap().0; s.enqueue_embeddings(&cfg.raw_version).unwrap(); s.claim_embedding_batch(&cfg.raw_version, 4).unwrap(); }
    EmbedWorker::start(cfg.clone(), tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    assert_eq!(Store::open(dir.path()).unwrap().0.count_embeddings(&cfg.raw_version).unwrap(), 10);
}

#[test]
fn pause_then_resume_deterministic() {
    let (_d, cfg) = setup_many("roboto.ttf", 12);
    let (tx, rx) = std::sync::mpsc::channel::<Progress>();
    let gate = Arc::new(Barrier::new(2)); let g = gate.clone();
    let w = EmbedWorker::start(cfg, tiny(), Box::new(move |p| { let started = matches!(p, Progress::Started { .. }); tx.send(p).ok(); if started { g.wait(); } })).unwrap();
    w.pause(); gate.wait(); // pause bật trước khi lô đầu được claim
    assert!(matches!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), Progress::Started { .. }));
    assert!(matches!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), Progress::Paused));
    w.resume(); w.join().unwrap();
    let rest: Vec<Progress> = rx.try_iter().collect();
    assert!(matches!(rest.first(), Some(Progress::Resumed)));
    assert!(matches!(rest.last(), Some(Progress::Finished { total_done: 12 })));
}

#[test]
fn stale_file_failed_others_succeed() {
    let (dir, cfg) = setup(&["roboto.ttf", "playfair_display.ttf"]);
    append_byte_to_font_file(&dir, "roboto.ttf");
    EmbedWorker::start(cfg.clone(), tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    let s = Store::open(dir.path()).unwrap().0;
    assert_eq!(state_of(&s, key_of(&s, "Roboto"), &cfg.raw_version).1.as_deref(), Some("stale_file"));
    assert_eq!(s.count_embeddings(&cfg.raw_version).unwrap(), 1);
}

#[test]
fn picks_up_faces_added_during_run() {
    let (dir, cfg) = setup(&["roboto.ttf"]);
    let d = dir.path().to_owned();
    let w = EmbedWorker::start(cfg.clone(), tiny(), Box::new(move |p| if matches!(p, Progress::Batch { .. }) { add_font_and_rescan(&d, "playfair_display.ttf") })).unwrap();
    w.join().unwrap();
    assert_eq!(Store::open(dir.path()).unwrap().0.count_embeddings(&cfg.raw_version).unwrap(), 2);
}
```

- [ ] **Step 2: Chạy** — FAIL.
- [ ] **Step 3: Cài `worker.rs`.**
- [ ] **Step 4: Chạy** — PASS. Chạy lặp 20 lần (`--test-threads 1`, vòng lặp shell) hai test `cancel_mid_run_writes_nothing` và `pause_then_resume_deterministic` để chắc kết quả ổn định.
- [ ] **Step 5: Commit** — `feat(embed): background worker (pause/cancel/resume, catch_unwind, re-enqueue on empty)`

---

### Task 8: Nguồn vector từ store (`source.rs`)

**Files:**
- Create: `crates/fontatlas-embed/src/source.rs`, `tests/source.rs`

**Interfaces:**
- Consumes: `Store::open_read_only`, `Store::embeddings_f32`, `Store::for_each_embedding`
- Produces:
  - `pub struct StoreEmbeddingSource { store: Store /* read-only */, model_version: String }`
  - `impl StoreEmbeddingSource`:
    - `pub fn open(dir: &Path, model_version: &str) -> Result<Self, EmbedError>`;
    - `pub fn load_all(&self) -> Result<Vec<(fontatlas_store::FaceKey, Vec<f32>)>, EmbedError>` — dùng để gán `FaceProfile.embedding` (Task 10);
    - `pub fn fetch(&self, keys: &[fontatlas_store::FaceKey]) -> Result<Vec<Option<Vec<f32>>>, EmbedError>` — dùng cho bước rerank f32.

- [ ] **Step 1: Viết test thất bại**

```rust
#[test]
fn reads_with_separate_connection_while_writer_open() {
    let (dir, cfg) = setup(&["roboto.ttf", "playfair_display.ttf"]);
    let w = EmbedWorker::start(cfg.clone(), tiny(), Box::new(|_| {})).unwrap();
    let mv = w.model_version().unwrap().to_string(); w.join().unwrap();
    let _writer = Store::open(dir.path()).unwrap(); // writer vẫn mở (WAL)
    let src = StoreEmbeddingSource::open(dir.path(), &mv).unwrap();
    let all = src.load_all().unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|(_, v)| v.len() == 128));
    let keys: Vec<_> = all.iter().map(|x| x.0).collect();
    assert_eq!(src.fetch(&keys).unwrap().iter().filter(|v| v.is_some()).count(), 2);
}

#[test]
fn other_versions_invisible() {
    let (dir, cfg) = setup(&["roboto.ttf"]);
    EmbedWorker::start(cfg, tiny(), Box::new(|_| {})).unwrap().join().unwrap();
    assert!(StoreEmbeddingSource::open(dir.path(), "khong-co").unwrap().load_all().unwrap().is_empty());
}
```

- [ ] **Step 2: Chạy** — FAIL.
- [ ] **Step 3: Cài.**
- [ ] **Step 4: Chạy** — PASS.
- [ ] **Step 5: Commit** — `feat(embed): read-only embedding source for index load and f32 rerank`

---

### Task 9: Model thật — export SigLIP 2, NOTICE, manifest, whitening-v1, tham chiếu PyTorch, test khớp

Task này có bước offline (Python, mạng) chạy **một lần**; kết quả được commit, nên CI không cần Python. **File ONNX không commit.**

**Files:**
- Create: `tools/embed/export_siglip2.py`, `tools/embed/torch_ref.py`, `crates/fontatlas-embed/src/bin/whitenfit.rs` (`required-features = ["fit"]`)
- Create (commit): `data/model-manifest.json`, `data/model-NOTICE.txt`, `data/model-LICENSE-Apache-2.0.txt`, `data/whitening-v1.bin`, `tests/data/torch_ref.json`
- Modify: `src/whiten.rs` (`bundled()`), `src/lib.rs` (`pub fn bundled_manifest() -> Manifest`)
- Test: `tests/parity.rs`

**Interfaces:**
- CLI:

  ```
  whitenfit --fonts <DIR> --model <ONNX> --out data/whitening-v1.bin --source "google/fonts@<SHA> ofl+apache"
  ```

  - duyệt thư mục bằng `fontatlas_store::walk_fonts`, phân tích bằng `analyze_path`;
  - chỉ lấy font có `LicenseClass::{Ofl, Apache}` **và** `family_split(family) == "tune"`;
  - render → encode (lô 4) → `fit(k = 128, shrink = 0.1)`;
  - in số face tune, số `no_glyphs`, thời gian; header ghi `"split":"tune"`.
- `data/model-manifest.json`:

  ```json
  {"file":"siglip2_base_p16_224_75de2d5_b4.onnx",
   "url":"<HẰNG CẤU HÌNH — điền sau khi chủ app duyệt nơi host, Task 9 Step 2>",
   "sha256":"<sha256 thật>","size":<byte>,"license":"Apache-2.0",
   "notice_files":["model-NOTICE.txt","model-LICENSE-Apache-2.0.txt"]}
  ```

  Test và worker **không đọc** `url`; chỉ `ModelStore::ensure` của app dùng nó (Plan 6). Khi `url` chưa có, Plan 6 hiển thị "model chưa phát hành".

- [ ] **Step 1: Export** (offline)

```bash
uv venv /tmp/fa-embed && VIRTUAL_ENV=/tmp/fa-embed uv pip install torch transformers onnx huggingface_hub
/tmp/fa-embed/bin/python tools/embed/export_siglip2.py --revision 75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2 \
  --batch 4 --out ~/fontatlas-ref/siglip2_base_p16_224_75de2d5_b4.onnx
shasum -a 256 ~/fontatlas-ref/siglip2_base_p16_224_75de2d5_b4.onnx
```

`export_siglip2.py` (dựng từ `spikes/embedding/export_siglip2.py`):
- `AutoModel.from_pretrained('google/siglip2-base-patch16-224', revision=…, attn_implementation='eager')`;
- wrapper gọi `get_image_features(pixel_values=x)`; nếu trả về object thì lấy `.pooler_output`;
- `torch.onnx.export` với input `[4,3,224,224]` tĩnh, opset 17, `dynamo=False`;
- in phiên bản `torch` và `transformers`;
- tự kiểm khớp ONNX ↔ torch trên 8 ảnh ngẫu nhiên, yêu cầu cos ≥ 0,9999 (spike đo được 0,99993).

Ghi sha256 và size vào manifest.
- `model-NOTICE.txt` ghi:
  - nguồn `google/siglip2-base-patch16-224` @ `75de2d5…` (Google, Apache-2.0);
  - câu "đã chuyển sang ONNX (lô 4, opset 17), không đổi trọng số";
  - lệnh export lại.
- `model-LICENSE-Apache-2.0.txt`: văn bản Apache-2.0 đầy đủ.

- [ ] **Step 2: Hosting — DỪNG, chờ chủ app duyệt.**
  - Không tự upload lên GitHub Release hay nơi nào khác.
  - Báo Lead: tên file, sha256, size, NOTICE. Chủ app quyết nơi host và tài khoản.
  - Khi có URL, **chỉ** sửa trường `url` trong manifest, commit riêng.
  - Các bước sau chạy được với file cục bộ.

- [ ] **Step 3: Dựng whitening-v1 từ nửa tune**

```bash
tools/reference/fetch_google_fonts.sh ~/fontatlas-ref/google-fonts   # SHA 47d54ca… (Plan 3); chưa gộp thì chạy lệnh sparse clone của Plan 3 Task 10 Step 3
cd UX/src-tauri && cargo run --release -p fontatlas-embed --features fit --bin whitenfit -- \
  --fonts ~/fontatlas-ref/google-fonts --model ~/fontatlas-ref/siglip2_base_p16_224_75de2d5_b4.onnx \
  --out crates/fontatlas-embed/data/whitening-v1.bin --source "google/fonts@47d54ca1ebb14fd864695f1d39dc734f39017c33 ofl+apache"
```

Expected: khoảng nửa kho (vài nghìn face), file khoảng 394 KB, mất khoảng 1 phút cho 3.000 face (~21 ms/face).

- [ ] **Step 4: Sinh vector tham chiếu PyTorch từ ảnh vàng**

```bash
/tmp/fa-embed/bin/python tools/embed/torch_ref.py --revision 75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2 \
  crates/fontatlas-embed/tests/data/render > crates/fontatlas-embed/tests/data/torch_ref.json
```

`torch_ref.py`, với mỗi PNG vàng:
- đọc xám → `(v/255−0.5)/0.5` → `[1,3,224,224]` (đúng công thức của `to_tensor`, không dùng processor của HF);
- chạy model HF **cùng revision** với bản ONNX → `get_image_features`;
- ghi JSON `{fixture: [768 số], "_revision": "...", "_transformers": "..."}`.

- [ ] **Step 5: Viết test** (`tests/parity.rs`)

```rust
#[test]
fn bundled_assets_valid() {
    let w = Whitening::bundled();
    assert_eq!((w.raw_dim, w.dim), (768, 128));
    assert!(w.version.starts_with("white-v1"));
    let m = bundled_manifest();
    assert_eq!((m.sha256.len(), m.license.as_str()), (64, "Apache-2.0"));
    assert!(m.size > 300_000_000);
    for f in &m.notice_files { assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(f).exists(), "{f}"); }
}

#[test]
#[ignore = "cần model thật: FONTATLAS_MODEL=<đường dẫn ONNX>"]
fn real_model_matches_pytorch_on_golden_pngs() {
    let mut e = OrtEncoder::load(Path::new(&std::env::var("FONTATLAS_MODEL").unwrap()), 4).unwrap();
    let refs: serde_json::Map<String, serde_json::Value> = serde_json::from_str(include_str!("data/torch_ref.json")).unwrap();
    for chunk in SAMPLES_LATIN.chunks(4) {
        let ts: Vec<Vec<f32>> = chunk.iter().map(|f| { let mut t = vec![0.0; 3 * 224 * 224]; to_tensor(&Gray224(load_golden_png(f)), &mut t); t }).collect();
        for (f, o) in chunk.iter().zip(e.encode(&ts, &AtomicBool::new(false)).unwrap()) {
            let r: Vec<f32> = serde_json::from_value(refs[*f].clone()).unwrap();
            let c = cos(&o, &r);
            assert!(c >= 0.9999, "{f}: cos {c}"); // spike 03b: min 0,99993
        }
    }
}
```

Ảnh vàng đưa thẳng vào `to_tensor`, không render lại, để parity không phụ thuộc renderer.

- [ ] **Step 6: Chạy.** `cargo test -p fontatlas-embed` → PASS. Rồi chạy `FONTATLAS_MODEL=~/fontatlas-ref/siglip2_base_p16_224_75de2d5_b4.onnx cargo test -p fontatlas-embed --release --test parity -- --include-ignored` → PASS; ghi 6 giá trị cos vào commit message.
- [ ] **Step 7: Commit** (không commit ONNX) — `feat(embed): SigLIP 2 export tooling, NOTICE/LICENSE, manifest (url pending owner), whitening-v1 (tune half), torch reference, parity test`

---

### Task 10: Một đường qua `FaceProfile.embedding` trong `fontatlas-index`; đo S4; báo cáo

> **Chỉ bắt đầu khi `feat/fontatlas-index` (Plan 3) đã gộp vào `dev/mvp`.** Rebase trước.

**Files:**
- Create: `crates/fontatlas-index/src/embed_index.rs`
- Modify: `crates/fontatlas-index/src/index.rs`, `src/similar.rs`, `crates/fontatlas-eval/src/metrics.rs`, `src/bin/eval.rs`
- Create: `docs/team/plans/plan4-report.md`
- Test: `crates/fontatlas-index/tests/similar.rs` (thêm)

**Interfaces:**
- Consumes: `FaceProfile.embedding: Option<Vec<f32>>` (Plan 3; 128 chiều, đã whiten, L2), `EMBED_WEIGHT`, `Index::similar`
- Produces:
  - `pub trait ExactEmbeddings: Send + Sync { fn fetch(&self, keys: &[FaceKey]) -> Vec<Option<Vec<f32>>>; }`. App bọc `StoreEmbeddingSource`, đổi `FaceKey` qua `hash_hex()`; test dùng bản trong bộ nhớ.
  - `pub const RERANK_CANDIDATES: usize = 400;`
  - `Index::new`:
    - lượng tử int8 từ `profile.embedding` (scale theo từng vector) vào mảng liền: `q8: Vec<i8>`, `inv: Vec<f32>`, `has: Vec<bool>`;
    - sau đó **bỏ f32 khỏi profile**, giữ RAM ở 128 B/face;
    - vector khác 128 chiều hoặc có chuẩn 0 thì coi như không có.
  - `pub fn set_exact_source(&mut self, src: Box<dyn ExactEmbeddings>)`
  - `similar()` (giữ nguyên chữ ký Plan 3):
    1. tính khoảng cách kết hợp cho **mọi** ứng viên trục; dùng cos int8 khi cả hai bên có ảnh, theo đúng công thức của Plan 3;
    2. lấy `max(k, RERANK_CANDIDATES)` ứng viên tốt nhất;
    3. nếu có exact source: đọc f32 cho query và các ứng viên này, tính lại cos và khoảng cách, sort lại; không có source thì dùng bản int8;
    4. trả k kết quả đầu.
  - Không còn đường thứ hai: bỏ `ImageHits`/`floor` của bản plan cũ.
  - eval:
    - thêm cờ `--embed-model <ONNX> --whitening <BIN>`: render → encode → whiten mọi face của corpus (không qua store), gán `profile.embedding`, dựng `Index` với `ExactEmbeddings` trong bộ nhớ;
    - in thêm khối S4 "Số đo + Hình ảnh" và khối "Số đo": R@1, R@5 cùng họ, **chỉ trên họ `family_split == "test"`**;
    - `EMBED_WEIGHT` chỉ được chỉnh trên họ `tune`.

- [ ] **Step 1: Viết test thất bại**

```rust
struct MemExact(HashMap<FaceKey, Vec<f32>>);
impl ExactEmbeddings for MemExact { fn fetch(&self, k: &[FaceKey]) -> Vec<Option<Vec<f32>>> { k.iter().map(|x| self.0.get(x).cloned()).collect() } }

// LCG: seed = seed·6364136223846793005 + 1, giá trị (seed >> 33)/2³¹ − 0.5 cho 128 chiều, rồi L2
fn unit(seed: u64) -> Vec<f32> { lcg_unit(128, seed) }

#[test]
fn int8_then_exact_rerank_equals_f32_ranking() {
    let ps: Vec<FaceProfile> = (0..2000).map(|i| { let mut p = synth_full(&format!("F{i}"), "Regular", 0.15); p.embedding = Some(unit(i)); p }).collect();
    let exact: HashMap<FaceKey, Vec<f32>> = ps.iter().map(|p| (p.key.clone(), p.embedding.clone().unwrap())).collect();
    let mut ix = index_of(ps); ix.set_exact_source(Box::new(MemExact(exact.clone())));
    let q = key_of("F0", "Regular");
    let got: Vec<FaceKey> = ix.similar(&q, &GroupWeights::default(), 20, &|_| true).unwrap().into_iter().map(|h| h.key).collect();
    let mut truth: Vec<(f32, FaceKey)> = exact.iter().filter(|(k, _)| **k != q).map(|(k, v)| (dot(v, &exact[&q]), k.clone())).collect();
    truth.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    assert_eq!(got, truth.into_iter().take(20).map(|x| x.1).collect::<Vec<_>>());
}

#[test]
fn faces_without_image_use_axes_only() {
    let mut a = synth_full("A", "Regular", 0.15); a.embedding = Some(unit(1));
    let ix = index_of(vec![a, synth_full("B", "Regular", 0.16)]);
    assert_eq!(ix.similar(&key_of("A", "Regular"), &GroupWeights::default(), 1, &|_| true).unwrap()[0].source, SimilarSource::Axes);
}

#[test]
fn wrong_dim_or_zero_embedding_ignored() {
    let mut a = synth_full("A", "Regular", 0.15); a.embedding = Some(vec![0.0; 128]);
    let mut b = synth_full("B", "Regular", 0.16); b.embedding = Some(vec![1.0; 64]);
    let h = index_of(vec![a, b]).similar(&key_of("A", "Regular"), &GroupWeights::default(), 1, &|_| true).unwrap();
    assert!(h[0].distance.is_finite() && h[0].source == SimilarSource::Axes);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "chỉ đo ở --release")]
fn s6_similar_with_images_50k_under_50ms() {
    let mut ps = synthetic_50k(20261016);
    for (i, p) in ps.iter_mut().enumerate() { p.embedding = Some(unit(i as u64)); }
    let exact: HashMap<FaceKey, Vec<f32>> = ps.iter().map(|p| (p.key.clone(), p.embedding.clone().unwrap())).collect();
    let mut ix = index_of(ps); ix.set_exact_source(Box::new(MemExact(exact)));
    let t = Instant::now(); ix.similar(&key_of("F0", "Regular"), &GroupWeights::default(), 20, &|_| true).unwrap();
    let ms = t.elapsed().as_secs_f64() * 1e3; assert!(ms < 50.0, "{ms} ms");
}
```

(Helper `lcg_unit`, `dot` thêm vào `tests/common/mod.rs` của index.)

- [ ] **Step 2: Chạy** — FAIL.
- [ ] **Step 3: Cài `embed_index.rs` và sửa `Index`.**
  - Test hiện có của Plan 3 (`embedding_switches_source`, `mismatched_or_zero_embedding_is_ignored`) phải vẫn PASS.
  - Nếu một test so `distance` chính xác tới 1e-6 với cos f32, thêm `set_exact_source` vào test đó; **không** nới sai số.
- [ ] **Step 4: Đo S4 thật**

```bash
cargo test -p fontatlas-index -p fontatlas-eval -p fontatlas-embed
cargo run --release -p fontatlas-eval --bin eval -- --fonts ~/fontatlas-ref/google-fonts \
  --similar ../../docs/team/eval/similar_sets.csv --pairing ../../docs/team/eval/pairing_judgements.csv \
  --embed-model ~/fontatlas-ref/siglip2_base_p16_224_75de2d5_b4.onnx --whitening crates/fontatlas-embed/data/whitening-v1.bin \
  | tee /tmp/plan4-eval.md
```

Cách đọc kết quả:
- Mốc spec S4: R@1 ≥ 0,6, R@5 ≥ 0,9.
- Mốc spike 03b (PIL, macOS, 400 face): whiten 0,693 / **0,898**; fit chéo nửa họ 0,721 / 0,926.
- Báo đúng số Rust trên họ `test`. R@5 < 0,9 thì ghi **chưa đạt**, để Lead quyết chỉnh spec hay đổi hướng.
- **Không** chỉnh renderer hay whitening để đạt ngưỡng.

- [ ] **Step 5: Viết `docs/team/plans/plan4-report.md`.** Gồm:
  - cos khớp PyTorch của 6 ảnh vàng;
  - số face tune của `whitenfit` và số `no_glyphs`;
  - ms/font của worker (lô 4, 4 luồng ORT) và RAM đỉnh;
  - S6 có ảnh ở 50k;
  - R@1/R@5 trên họ test, ở hai chế độ "Số đo" và "Số đo + Hình ảnh";
  - `EMBED_WEIGHT` đã chọn trên tune;
  - trạng thái hosting model (chờ chủ app);
  - rủi ro còn mở.
- [ ] **Step 6: Commit** — `feat(index,eval): single embedding path (int8 in Index + f32 rerank), S4 with SigLIP 2; plan 4 report`

---

## Ghi chú cho Plan 6 (tích hợp Tauri)

- **Khởi động:**
  - `embed_enabled` (mặc định bật) cộng `ModelStore::status()` → `EmbedMode`;
  - `Ready` → gọi `EmbedWorker::start` sau khi UI đã hiện;
  - `StoreEmbeddingSource::load_all` → gán `FaceProfile.embedding` → `Index::new` + `set_exact_source`.
- **Lần đầu:** hiện thông báo về trách nhiệm giấy phép font (nút "Đã hiểu" → `license_notice_ack = '1'`); Cài đặt có một dòng cố định nhắc lại.
- **Tải model:** hỏi người dùng trước khi tải ~372 MB. Manifest chưa có `url` → hiện "model chưa phát hành", vẫn chạy "Số đo".
- **Tiến độ:** gửi `Progress` qua `tauri::ipc::Channel`; có nút Tạm dừng / Huỷ / Tiếp tục.
- **Giấy phép model:** NOTICE và LICENSE của model đặt cạnh model trong `<app_data>/models/`, và hiện trong màn "Giấy phép bên thứ ba".
