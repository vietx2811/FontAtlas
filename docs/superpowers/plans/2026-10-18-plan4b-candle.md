# Plan 4b — chuyển encoder sang candle (SigLIP 2 safetensors chính thức)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps dùng checkbox (`- [ ]`).

**Goal:** Bỏ ONNX Runtime. Encoder ảnh chạy bằng candle (`candle-transformers::models::siglip`), trọng số tải thẳng từ repo chính thức `google/siglip2-base-patch16-224` @ `75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2`. Không tự host gì.

**Architecture:**
- Giữ nguyên API crate (`Encode`, worker, store, whitening). Chỉ thay phần thân encoder và phần tải model.
- Tải model: chỉ lấy phần `vision_model.*` bằng HTTP Range, rồi ghép thành một file safetensors cục bộ hợp lệ.
- Thiết bị: Metal trên macOS, lỗi thì rơi về CPU; Windows dùng CPU.

**Tech Stack:** `candle-core`/`candle-nn`/`candle-transformers =0.11.0` (macOS: features `accelerate`, `metal`); `ureq 3`; `sha2`.

**Spec:** `docs/superpowers/specs/2026-10-16-fontatlas-mvp-design.md` §5, §6.1–6.2, §10–11. Kế thừa Plan 4 (`2026-10-17-plan4-fontatlas-embed.md`). Quyết định chủ app 18/10 qua Lead: candle, không tự host.

**Nhánh:** `feat/fontatlas-embed-candle`, tách từ `feat/fontatlas-embed` @ 7a79a50. Không push, không merge. Không sửa crate store/core/index.

## Sự thật đã kiểm (18/10)
- `model.safetensors`: 1.500.800.904 B, sha256 `612923381c76ec5a9bed335d1c48827e3f2e506ac31b044b63b2031fadee6a0b` (blob trong cache HF). Header JSON dài 49.016 B, toàn bộ tensor là F32.
- 208 tensor `vision_model.*` nằm thành **một đoạn liền**: offset dữ liệu 1.129.214.984 → 1.500.751.880 (371.536.896 B), ở cuối file. Tháp text nằm trước.
- Kiến trúc là SigLIP v1: conv patch 16, 196 position embedding, head MAP (`probe`, `in_proj`). config.json chỉ ghi `model_type`, các giá trị khác là mặc định.
  - Khớp `siglip::Config::base_patch16_224().vision_config`;
  - `VisionModel::new(cfg, use_head = true, vb.pp("vision_model"))` cho ra pooler output = `get_image_features`.

## Global Constraints
- Chuẩn hoá đầu vào: mean = std = 0,5; render-v2 giữ nguyên; lô 4; vector thô 768 chiều.
- `model_version` phải đổi khi đổi runtime: `raw_version` = `siglip2-base-p16-224@75de2d5|st-vision@<sha12>|candle0.11-<dev>|render-v2`.
- Embedding không rời máy. Kèm NOTICE + LICENSE Apache-2.0 (đã có trong `data/`).
- Test mặc định không cần mạng. Test cần model thật thì `#[ignore]` và đọc biến môi trường `FONTATLAS_SIGLIP`.

## Review Focus
1. Server không hỗ trợ Range (trả 200): phải tải cả file, kiểm sha256 toàn file, rồi trích phần vision. Không được ghép nhầm từ body 200 như thể là 206.
2. Range trả sai đoạn (`Content-Range` lệch) hoặc dữ liệu hỏng: sha256 của đoạn sai → xoá `.part`, trả `ChecksumMismatch`.
3. Huỷ giữa lô trên Metal/CPU: không ghi gì, face về `pending` (test worker cũ vẫn phải xanh với encoder mới).
4. Metal không khởi tạo được (máy ảo, GPU cũ): phải rơi về CPU, và `model_version` phản ánh đúng thiết bị thật đã chạy.
5. File safetensors cục bộ bị sửa: phát hiện bằng sha256 và tải lại (giữ cơ chế `.ok` của Plan 4).

---

### Task 1: CỔNG — candle khớp torch và đủ nhanh
**Files:** `Cargo.toml` (thêm candle), `tests/candle_gate.rs`, `examples/bench_backends.rs`, `src/candle_enc.rs` (bản tối thiểu).

**Interfaces — Produces:**
- `pub enum Backend { Cpu, Metal }`
- `pub struct CandleEncoder`, gồm:
  - `load(path: &Path, backend: Backend) -> Result<Self, EmbedError>`;
  - `device_id(&self) -> &'static str` (`"cpu"` | `"metal"`);
  - `impl Encode`.

- [ ] Viết test `#[ignore]` `candle_matches_torch_on_golden_pngs`:
  - nạp `FONTATLAS_SIGLIP` (file HF đầy đủ cũng được, vì VarBuilder chỉ đọc `vision_model.*`) cho cả Cpu và Metal;
  - so với `tests/data/torch_ref.json`;
  - assert cos min ≥ 0,999 (ngưỡng cổng); in cos min/mean.
- [ ] Viết test `cpu_and_metal_agree`: cos min giữa vector CPU và Metal trên 6 ảnh vàng; in số.
- [ ] Chạy → FAIL (chưa có `CandleEncoder`).
- [ ] Cài `CandleEncoder` tối thiểu: `VarBuilder::from_mmaped_safetensors`, `VisionModel` có head, forward lô ≤ 4.
- [ ] Chạy → in số parity.
- [ ] `examples/bench_backends.rs`:
  - 64 ảnh render-v2 từ fixtures, lô 4, 1 lô khởi động;
  - đo ms/face release cho candle-cpu, candle-metal, ort (`FONTATLAS_MODEL`);
  - in JSON.
- [ ] **Cổng:** DỪNG và báo Lead kèm số nếu cos min < 0,999, HOẶC nếu cả CPU lẫn Metal đều chậm hơn ort > 3×.
- [ ] Commit `spike(embed): candle gate — parity + backend bench`.

### Task 2: Encoder candle thay ort
**Files:** `src/candle_enc.rs`, `src/encoder.rs`, `src/lib.rs`, `Cargo.toml` (bỏ `ort`), `tests/encoder.rs`, `tools/embed/make_tiny_siglip.py`, `tests/data/tiny_siglip.safetensors` + `tiny_siglip_ref.json`.
- **Model nhỏ cho test:** `SiglipVisionModel` của transformers với config tí hon (hidden 32, 2 lớp, 4 head, patch 16, ảnh 224, có head), seed cố định. Lưu safetensors (tiền tố `vision_model.`) và vector tham chiếu torch cho 2 tensor xác định.
  - Vì `VisionConfig` có trường public, encoder đọc config từ `config.json` cạnh model, mặc định là `base_patch16_224`.
  - Ruling khi làm: nếu không đọc config thì test dùng `VisionConfig` dựng tay.
- Test, thay các test ort:
  - `tiny_matches_torch` (cos ≥ 0,9999);
  - `pads_partial_batch`;
  - `cancel_before_batch_returns_cancelled`;
  - `rejects_wrong_tensor_len`;
  - `missing_tensor_is_model_error`.
- **Huỷ:** kiểm cờ trước forward và sau forward. Lô 4 trên Metal khoảng 50–100 ms, nên độ trễ huỷ ≤ 1 lô. Ghi điều này trong doc.
- **Chọn thiết bị:** `pub fn select_backend() -> Backend`. macOS: thử `Device::new_metal(0)` và chạy một forward nhỏ; lỗi → Cpu. Nơi khác: Cpu.
- Bỏ `ort` khỏi `Cargo.toml`. `OrtEncoder` bị xoá; worker/test dùng `CandleEncoder`. Toàn bộ test worker cũ phải xanh.
- Commit `feat(embed): candle encoder replaces ort`.

### Task 3: Tải phần vision bằng HTTP Range
**Files:** `src/model.rs`, `data/model-manifest.json`, `examples/make_manifest.rs`, `tests/model.rs`.
- Manifest v2:
  - `source { url, size, sha256, header_len, vision_offset, vision_len, header_sha256, vision_sha256 }`;
  - `file`, `sha256` (file cục bộ đã ghép), `size`, `license`, `notice_files`.
  - `examples/make_manifest.rs` tính manifest từ file HF đã có trong máy.
- **File cục bộ (tất định):**
  - header mới chỉ gồm `vision_model.*`, offset trừ đi `vision_offset`, khoá theo thứ tự byte, giữ `__metadata__`;
  - pad space cho chia hết 8 như safetensors;
  - sau đó là đúng đoạn vision.
  - Vì tất định nên sha256 file cục bộ được ghim.
- **Luồng `ensure`:**
  1. GET Range `bytes=0-<8+header_len-1>`. Nếu nhận 206: kiểm `header_sha256`, rồi GET Range đoạn vision vào `.part`, kiểm `vision_sha256`.
  2. Nếu nhận 200 (server bỏ qua Range): stream cả file vào `.full.part`, kiểm `source.sha256`, trích header + đoạn vision.
  3. Ghép file, kiểm `sha256` cục bộ, fsync, rename, ghi `.ok`. Giữ lock, timeout từng chunk và huỷ của Plan 4.
- **Test (server tiny_http giả hỗ trợ Range, file safetensors giả nhỏ có tensor `text_model.*` + `vision_model.*`):**
  - `range_download_extracts_vision_only` — file cục bộ chỉ có tensor vision, đọc lại bằng `safetensors`/VarBuilder được;
  - `server_without_range_falls_back_to_full_file`;
  - `bad_segment_checksum_rejected`;
  - `wrong_content_range_rejected`;
  - các test cũ: interrupted, tampered, oversize, lock.
- Commit `feat(embed): range download of vision tensors from official safetensors`.

### Task 4: model_version theo runtime/thiết bị, fit lại whitening, đo lại
**Files:** `src/lib.rs`, `data/whitening-v1.bin` → `whitening-v2.bin`, `src/bin/whitenfit.rs`, `tests/parity.rs`, `docs/team/plans/plan4b-report.md`.
- **Quyết định thiết bị theo số `cpu_and_metal_agree`:**
  - nếu cos min ≥ 0,9999 trên ảnh vàng **và** trên 200 face thật → dùng chung version, `RUNTIME_ID = "candle0.11"`;
  - ngược lại → `candle0.11-cpu` / `candle0.11-metal`, và worker lấy runtime từ `CandleEncoder::device_id()`.
  - Ghi lý do vào báo cáo.
- `raw_version(model_sha256, device)` tuỳ theo quyết định trên. Cập nhật test version.
- `whitenfit` dùng `CandleEncoder` (thiết bị tốt nhất), fit lại `white-v2` trên nửa tune, rồi `--report-test` R@1/R@5/R@10 (chỉ bằng ảnh). So với bản ort: 0,764 / 0,895.
- Parity test đọc `FONTATLAS_SIGLIP`. Test `bundled_assets_valid` theo manifest v2.
- Báo cáo `plan4b-report.md`: số cổng, ms/face, RAM, R@k, quyết định thiết bị, các ruling.
- Commit, review cuối toàn nhánh (subagent), sửa, rồi báo Lead.
