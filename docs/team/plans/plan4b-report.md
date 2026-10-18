# Báo cáo Plan 4b — encoder candle + tải thẳng từ repo chính thức

Nhánh `feat/fontatlas-embed-candle`, tách từ `feat/fontatlas-embed` @ 7a79a50. Chưa push, chưa merge.
- Không tự host gì: model tải thẳng từ `google/siglip2-base-patch16-224` @ `75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2`.
- Không sửa crate store/core/index.
- Máy đo: Apple M4 Pro, macOS 27.2, rustc 1.96.0, candle 0.11.0. Trong lúc đo, máy đang tải nặng (load average 11–13).

## Kết quả
- `cargo test --workspace`: **186 test qua, 0 lỗi**. Test cần model thật bị bỏ qua mặc định; đã chạy bằng `--include-ignored` (không cần mạng) và qua.
- `ort` đã bị gỡ khỏi `Cargo.lock`. `cargo clippy -p fontatlas-embed --all-targets`: sạch.
- Hai test `cancel_mid_run_writes_nothing` và `pause_then_resume_deterministic` chạy lặp 20 lần với encoder candle: 20/20 qua.

## Cổng (Task 1): QUA
- **Parity** với torch, 6 ảnh vàng, dùng `candle_transformers::models::siglip::VisionModel` có head MAP (đúng `get_image_features`):

| | cos min với torch |
|---|---|
| candle CPU (Accelerate) | 1,000000 |
| candle Metal | 0,9999996 |
| bản ort trước đây (để so) | 0,99997 |

- **Tốc độ** (ms/face, release, lô 4, cùng ảnh, chạy 2 lượt):

| | ms/face | Nạp model | RAM đỉnh |
|---|---|---|---|
| candle CPU | 29,8–30,7 | 0,7–1,0 s | 0,69–0,76 GB |
| candle Metal | 27,4–28,2 | 0,06 s | 0,77 GB |
| ort 4 luồng | 27,7–29,7 | 6,5–8,3 s | 0,74–0,77 GB |
| ort 8 luồng | 23,5–25,6 | 6,9–9,7 s | 0,82 GB |

  → candle chậm hơn ort tối đa khoảng 1,3×, xa ngưỡng dừng 3×. Riêng Metal để CPU gần như rảnh: whitenfit trên 3.744 face chỉ tốn 18,7 s CPU user.

## Tải model (Task 3)
- **Bố cục file nguồn:**
  - `model.safetensors`: 1.500.800.904 B, sha256 `612923…a0b`, header JSON 49.016 B.
  - 208 tensor `vision_model.*` nằm thành **một đoạn liền** 371.536.896 B ở cuối file.
- **Cách tải:** 2 request Range (header + đoạn vision), kiểm sha256 **từng đoạn**, rồi ghép thành file safetensors cục bộ tất định (371.561.688 B, sha256 `9c0ed27…` được ghim).
  - Đã kiểm tay với HF thật (chỉ lấy 8 byte): HF trả 302 sang CDN rồi 206. Có test cho trường hợp Range đi qua redirect.
- **Server không hỗ trợ Range (trả 200):** stream cả thân nhưng chỉ ghi header + đoạn vision, rồi kiểm sha256 toàn file. Không cần file tạm 1,5 GB.
- **Test không cần mạng:** server tiny_http giả có Range, cộng file safetensors giả. Các trường hợp được kiểm:
  - Range thường, fallback 200, sai sha đoạn, sai `Content-Range`, thân quá cỡ;
  - huỷ giữa chừng, file bị sửa, chạy đồng thời, redirect;
  - file tải về nạp được bằng candle.
- **Test end-to-end** (ignored): phục vụ file HF thật qua Range cục bộ → file ghép trùng sha ghim → parity với torch đạt.
- Manifest v2: `data/model-manifest.json`, tạo bằng `examples/make_manifest.rs`. NOTICE ghi rõ nguồn, revision, sha và phần đã thay đổi (chỉ trích tensor, không đổi trọng số); kèm LICENSE Apache-2.0.

## Version và whitening (Task 4)
- `raw_version` = `siglip2-base-p16-224@75de2d5|st-vision@9c0ed2787f99|candle0.11|render-v2`
- **Dùng chung version cho CPU và Metal.** Lý do: cos min CPU↔Metal là 0,9999996 trên 6 ảnh vàng và 0,9999993 trên 200 face Google Fonts, trên ngưỡng 0,9999. Test `cpu_and_metal_agree` giữ ngưỡng này.
- `white-v2`: fit lại trên nửa tune (1.729 face, Google Fonts @47d54ca, OFL/Apache); sha256 `284c91b5…`.

**Similar chỉ bằng ảnh, chưa kết hợp trục số đo.** Nửa test, 1.284 truy vấn:

| | R@1 | R@5 | R@10 |
|---|---|---|---|
| candle, whiten | **0,763** | **0,896** | 0,922 |
| ort (Plan 4), whiten | 0,764 | 0,895 | 0,921 |
| candle, thô | 0,690 | 0,833 | 0,872 |

→ Chất lượng tương đương ort. R@5 vẫn **chưa đạt** spec S4 (0,9) nếu chỉ dùng ảnh; còn chờ Task 10 kết hợp trục.

- Hiệu năng whitenfit (Metal): 28,4 ms/face, tổng 125 s cho 3.744 face. RAM đỉnh 1,18 GB, gồm cả encoder CPU nạp thêm để so sánh.

## Review cuối (subagent): đã sửa
- **Important — header file cục bộ phụ thuộc feature `serde_json/preserve_order`.**
  - Hậu quả: nếu một dependency bất kỳ trong app bật feature này, mọi lần tải đều lệch sha.
  - Sửa: ghi header bằng struct tường minh. Test `vision_header_bytes_are_pinned` ghim từng byte; đỏ khi bật `preserve_order`, xanh ở cả hai chế độ. `make_manifest` cho cùng sha ở cả hai chế độ.
- **Important — Metal chỉ được kiểm lúc nạp, không kiểm lúc chạy.**
  - Sửa: `CandleEncoder::verified()` chạy thử một ảnh qua mọi kernel và kiểm đầu ra hữu hạn; `load_best` rơi về CPU nếu thất bại. Test dùng model có trọng số NaN.

## Hoãn lại (minor)
- `Content-Range` sai trả lỗi `Io` thay vì `ChecksumMismatch`.
- Header nhận 206 nhưng vision nhận 200 thì báo lỗi, không chuyển sang tải cả file.
- Luồng đọc có thể rò khi kết nối TCP nửa mở (ureq không có timeout đọc thân).
- Huỷ không được kiểm trong lúc connect hoặc chờ header (tối đa khoảng 60 s).
- ureq bật gzip mặc định; nên gửi `Accept-Encoding: identity`.
- Lỗi parse header mang mã "inference".
- `whitenfit` không từ chối file model khác manifest.
- `extract_vision_file` ghi đè tại chỗ (chỉ là tool dev).

## Chưa kiểm
- **Chưa build thử trên Windows.** candle trên Windows chạy CPU thuần Rust (không Accelerate), nên tốc độ có thể chậm hơn số đo ở trên.
- **Task 10** (index/eval kết hợp trục): vẫn chờ Plan 3.
