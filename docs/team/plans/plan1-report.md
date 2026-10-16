# Plan 1 — Báo cáo thực thi `fontatlas-core`

Nhánh `feat/fontatlas-core` (cắt từ `chore/sprint0-hygiene` @ 212aba4), worktree `claude/gracious-dijkstra-011ba7`. Chưa push, chưa merge.
Máy: Apple M4 Pro, 12 luồng, rustc 1.96. Ký hiệu: **[ĐÃ CHẠY]** = số đo thật.

## 1. Kết quả

- Crate `UX/src-tauri/crates/fontatlas-core` có đủ Task 1–11. Riêng trục đầu nét (terminal) hoãn sang plan sau, theo quyết định của Lead (mục "Trục hoãn").
- [ĐÃ CHẠY] `cargo test -p fontatlas-core`: **47 passed, 0 failed, 1 ignored** (`perf_system_fonts`, chạy tay).
  - load 10, metadata 5, geom 5, vietnamese 5, axes 13, anatomy 4, analyze 4 (+1 ignored), robustness 1.
  - Các test phụ thuộc font hệ thống macOS tự bỏ qua trên máy khác.
- [ĐÃ CHẠY] `cargo clippy -p fontatlas-core --all-targets -- -D warnings`: sạch.
- [ĐÃ CHẠY] `cargo fmt -p fontatlas-core` đã áp dụng.
- [ĐÃ CHẠY] `cargo check -p app`: app gốc vẫn biên dịch. 11 cảnh báo có sẵn từ trước, không thuộc crate mới.
- [ĐÃ CHẠY] Độ bền:
  - `robustness`: 360 biến thể hỏng ngẫu nhiên (6 file × 60), không panic ở cả bản debug (có kiểm tra tràn số) lẫn release.
  - Không phải sửa hàm nào.

## 2. Commit

| Commit | Nội dung |
|---|---|
| 5838ebd | crate + `FontFile` (nạp, băm BLAKE3 file gốc, giải nén WOFF1/WOFF2) |
| d31ed0a | `faces()` (TTC), `validate()` (biên bảng, bảng bắt buộc, loca/glyf) |
| 5902584 | metadata + phân loại giấy phép |
| 917fde4 | `Metric`/`Quality`/`FailReason`, `GlyphCtx`, `ink_runs` |
| 9dc9097 | phủ tiếng Việt chi tiết + độ cao dấu chồng |
| d5380ba | x-height, cap, asc/desc, độ đậm |
| d43086e | contrast, stress, aperture c, width, slant, serif, counter |
| 4c0cfc6 | aperture e, a/g một hay hai tầng, spacing, width cv (terminal hoãn) |
| c62745d | `Anatomy`, `measure()`, vị trí chuẩn VF, nhánh chữ hoa, không outline |
| d3a7385 | `analyze_bytes/path/paths` (rayon), example `dump` |
| 06b9b11 | sửa: chấp nhận font bitmap Apple (`bhed`) trong `validate` |
| (commit cuối) | test độ bền, `cargo fmt`, báo cáo này |

## 3. Hiệu năng [ĐÃ CHẠY] (release, page cache đã nóng)

| Tập | File / face | Đơn luồng | rayon |
|---|---|---|---|
| `/System/Library/Fonts` (không đệ quy, test `perf_system_fonts`) | 83 / 307 | — | 129–235 ms (3 lần chạy) |
| Bộ dữ liệu của spike (`/System/Library/Fonts` đệ quy + `/Library/Fonts` + mẫu) | 375 / 791 | 1 151–1 161 ms (1.46 ms/face) | 188–200 ms (0.24 ms/face), ×5.8–6.1 |

- Mỗi lượt gồm đọc file, băm, giải nén, `validate`, metadata, tiếng Việt và `Anatomy` ở vị trí chuẩn.
- Nhanh hơn spike (1.6–1.8 s đơn luồng), vì mỗi VF giờ chỉ đo ở 1 vị trí thay vì 4.
- Ở bản đầu, 791 face (spike có 792) vì `NISC18030.ttf` bị `validate` loại. Commit 06b9b11 đã sửa lỗi này. Tôi chưa đo lại bộ đầy đủ sau khi sửa: thêm một font bitmap thì không ảnh hưởng thời gian.

## 4. Giá trị trục trên 5 mẫu + Helvetica (cho 04 ML hiệu chỉnh) [ĐÃ CHẠY]

Cách đo: `MeasureAt::Standard`, đầu ra của `examples/dump`. Helvetica lấy face 0 của `Helvetica.ttc`.

| font | contrast | contrast_h | stress_deg | aperture c (°) | aperture_e (÷xh) | stem | darkness | width | spacing | width_cv | a tầng | g tầng | serif_foot |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Roboto Regular | 1.23 | 1.23 | low_contrast | 53 | 0.209 | 0.171 | 0.376 | 0.91 | 0.171 | 0.284 | 2 | 1 | 1 |
| Open Sans Regular | 1.24 | 1.13 | low_contrast | 105 | 0.308 | 0.151 | 0.322 | 0.918 | 0.181 | 0.289 | 2 | 2 | 1 |
| Playfair Display Regular | 4.89 | 4.65 | 0 | 83 | 0.355 | 0.175 | 0.338 | 0.882 | 0.0867 | 0.264 | 2 | 2 | 2.69 |
| Libre Baskerville Regular | 2.28 | 2.1 | 0 | 92 | 0.389 | 0.179 | 0.353 | 1.05 | 0.0889 | 0.277 | 2 | 2 | 2.93 |
| Source Code Pro Regular | 1.25 | 1.17 | low_contrast | 85 | 0.258 | 0.169 | 0.333 | 0.988 | 0.298 | 0 | 2 | 2 | 1 |
| Helvetica | 1.22 | 1.15 | low_contrast | 49 | 0.133 | 0.168 | 0.385 | 0.932 | 0.148 | 0.297 | 2 | 1 | 1 |

Nhận xét:
- **Aperture `e`** xếp đúng chiều humanist mở / grotesque khép: Helvetica 0.13 < Roboto 0.21 < Open Sans 0.31.
- **Stress** vẫn ra 0° cho Playfair và Libre Baskerville. Điều này khớp nghi vấn của spike, nên giữ `Estimated` chờ nhãn tay.
- **Số tầng của `g`** khớp hình thật: Open Sans và Source Code Pro có `g` hai tầng; Roboto và Helvetica có `g` một tầng.

## 5. Chỗ lệch plan và API fontations

**API fontations:**
- Khớp plan ở mọi chỗ.
- `FontRef::table_directory`, `TableRecord::{tag, offset, length}`, `Loca::get_raw/len`, `hhea().ascender().to_i16()` và `feature_list()` qua `and_then` đều biên dịch nguyên bản.
- `wuff::decompress_woff1/woff2` đúng tên.

**Lệch plan, đã ghi ledger:**
1. Task 1: thêm `impl Debug for FontFile` thủ công (không in bytes), vì `unwrap_err` trong test cần `Debug`. `tests/common/mod.rs` thêm `#![allow(dead_code)]`.
2. Task 5: test `regression_avenir_next_supports_vietnamese` trong plan kỳ vọng mọi face đều đủ. Dữ liệu thật (khớp spike) cho thấy Heavy và Heavy Italic thiếu 94 ký tự. Test nay kiểm chính xác: 10 face đủ và 2 face Heavy thiếu đúng 94, chặt hơn chứ không nới.
   - Nguồn sai: báo cáo spike mục 2.3 ghi "Avenir Next (mọi độ đậm)" là không chính xác.
3. Task 7: thêm `use kurbo::Shape` (cần cho `winding`).
4. Task 8: **terminal hoãn** theo quyết định Lead (B). Đã xoá `terminal_c`, test `terminal_ball_vs_sans` và trường `Anatomy::terminal`.
5. Task 9: test `non_latin_or_no_outline_does_not_panic` dùng Hiragino Sans GB và PingFang.
   - Hiragino có chữ Latin nên x-height đo được (0.545, Measured), tức hành vi đúng, tiền đề test sai. PingFang không có trên máy.
   - Đã thay bằng GeezaPro (Ả Rập) và NISC18030 (bitmap), thêm assert `Failed(NoOutlines)` cho font bitmap.
6. Ngoài plan: `validate` loại font bitmap Apple `NISC18030.ttf` (bảng `bhed bdat bloc`, không có `head/hhea/hmtx`). Điều này trái spec 4.1 và Review Focus 5.
   - Đã thêm bộ bảng bắt buộc riêng khi có `bhed` (`bhed, maxp, cmap`).
   - Test `apple_bitmap_font_with_bhed_is_accepted` đi từ RED sang GREEN.
7. Task 11: seed `0xF0A7_1A5` đổi cách viết thành `0x0F0A_71A5` (cùng giá trị) để qua lint `unusual_byte_groupings` của clippy.

**Lưu ý cho plan sau (chưa sửa):**
- Apple Color Emoji có bảng `glyf` rỗng cùng `sbix`, nên `outline_format = Glyf`. Vì vậy `Anatomy` báo `Failed(NoLatinLowercase)` thay vì `NoOutlines`. Kết quả vẫn là Failed và không panic, nhưng lý do chưa chính xác.
- Metadata của font bitmap Apple có `units_per_em = 0`, vì read-fonts không đọc `bhed`.

## Trục hoãn

### Đầu nét (terminal) chữ `c` — hoãn sang Plan 3 (Lead quyết định B, 2026-10-16)

**Định nghĩa trong plan:** `terminal_c` = bề dày đoạn mực đầu nét trên của `c`, đo trên đường dọc ở 92% bề rộng bbox, chia cho bề dày thân trái `c` (đo ở tâm dọc). Test yêu cầu Playfair Display (đầu nét ball) > Roboto.

**Kết quả thật:** Playfair 1.088, Roboto 1.194. Test fail.

**Quét vị trí 70–98% bề rộng** (đầu nét ÷ thân trái). Glyph `c` được dựng thẳng, đo ở instance mặc định:

| Font | thân trái | nét trên ở tâm | 70% | 75% | 80% | 85% | 88% | 90% | 92% | 95% | 98% |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Roboto | 185.0 | 152.3 | 0.97 | 1.12 | 1.52 | 1.57 | 1.43 | 1.32 | 1.19 | 0.95 | 0.59 |
| Open Sans | 172.0 | 149.2 | 0.85 | 0.87 | 0.89 | 0.90 | 0.91 | 0.92 | 0.92 | 0.75 | 0.30 |
| Playfair Display | 97.0 | 22.9 | 0.21 | 0.25 | 0.34 | 1.40 | 1.30 | 1.21 | 1.09 | 0.81 | — |
| Libre Baskerville | 114.1 | 46.4 | 0.55 | 0.72 | 0.91 | 0.97 | 0.94 | 0.89 | 0.83 | 0.69 | 0.42 |
| Source Code Pro | 85.0 | 69.6 | 0.82 | 0.86 | 0.91 | 0.96 | 0.99 | 0.86 | 0.65 | 0.31 | — |
| Helvetica (TTC face 0) | 192.1 | 161.4 | 0.95 | 1.12 | 1.53 | 1.58 | 1.46 | 1.35 | 1.22 | 0.96 | 0.54 |

"—" = đường dọc không cắt đầu nét.

Không vị trí nào cho Playfair > Roboto. Lý do:
- Đầu nét `c` của Roboto và Helvetica cong. Đường dọc cắt xiên qua nét nên đoạn mực dài ra.
- Thân trái của Playfair dày, vì là font tương phản cao, nên tỉ số bị kéo xuống.

Kết luận: tỉ số "đầu nét ÷ thân" không tách được khỏi độ cong.

**Định nghĩa (A), đã đề xuất nhưng không dùng:** max(đoạn mực đầu nét trên, quét 75–95%) ÷ bề dày nét trên ở tâm `c`. Số tính tay từ bảng trên:

| Font | (A) |
|---|---|
| Playfair Display | 1.40×97.0/22.9 ≈ 5.9 |
| Libre Baskerville | 0.97×114.1/46.4 ≈ 2.4 |
| Roboto | 1.57×185.0/152.3 ≈ 1.9 |
| Helvetica | 1.58×192.1/161.4 ≈ 1.9 |
| Source Code Pro | 0.99×85.0/69.6 ≈ 1.2 |
| Open Sans | 0.92×172.0/149.2 ≈ 1.06 |

Thứ tự đúng, nhưng mẫu số là nét mảnh nên (A) chủ yếu lặp lại contrast. Đưa vào pairing thì contrast bị tính hai lần. Vì vậy Lead chọn hoãn: xoá `terminal_c`, không giữ trường luôn `Failed` trong `Anatomy`.

**Hướng cho Plan 3 (04 ML, có nhãn tay):**
- Đo độ dày đầu nét theo **pháp tuyến của đường tâm nét**, không theo đường dọc.
- So với độ dày nét mảnh lân cận.
- Kiểm tra tương quan với `contrast_o` trước khi dùng (ví dụ Spearman trên thư viện tham chiếu). Chỉ đưa vào pairing nếu phần dư sau khi hồi quy theo contrast vẫn mang thông tin.
