# Phân loại serif chân nhỏ (mục D, core-0.9.0)

Nhánh `fix/classify-small-serif`. Báo cáo chỉ có con số: không font, ảnh hay đường dẫn của chủ app.

## 1. Nguyên nhân (systematic debugging)

Giả thuyết ban đầu: 9 họ BeauType có `serif_foot` đúng 1.00 vì phép đo hỏng và rơi về giá trị mặc định.

**Bị bác bỏ.** Đo bề rộng mực của `i`, `l`, `n` ở 6 độ cao từ 1.25% tới 35% x-height trên file thật
(Nouvoty, Rohbeton, Grotesk, Beau Sans, Elf50, Oateria, Solisiert, Widesonik, Wifting, Suave): bề rộng
**không đổi từ baseline tới 35% x-height** (ví dụ 172/172/172/172/172/172 đơn vị em). Chúng thật sự không có chân,
nên 1.00 là số đo đúng. Phép đo không rơi về mặc định.

Lỗi thật nằm ở hai chỗ khác:

1. **Chân nhọn/bracket đo thấp.** `serif_foot` chỉ lấy MỘT dòng quét ở 2.5% x-height. Chân nhọn (rộng ~0 ngay tại
   baseline, loe ra trong vài % tiếp theo) cho bề rộng rất nhỏ ở đó: Annam Neue đo 0.61, Noirist 0.26
   (bề rộng theo độ cao của `i`: 55 → 110 → 174 → 271 rồi về thân 180). Giá trị < 0.9 bị coi là "Sans" (phiếu 2.0).
2. **Vùng chết 1.2–1.6.** `classify.rs` chỉ bỏ phiếu khi `serif_foot > 1.6` hoặc `< 1.2`. Chân nhỏ rơi vào giữa, nên
   không có phiếu nào, và từ khoá tên "Neue"/"Grotesk" (Sans 0.8) quyết định.

Ngoài ra, các họ này có **cờ serif trên-trái** (phần mực nhô sang trái thân ở đỉnh `i`/`l`/`n`) mà không có chân slab:
phép đo cũ không nhìn ở đỉnh.

## 2. Thay đổi (analyzer `core-0.8.0` → `core-0.9.0`)

Hai metric mới, `serif_foot` giữ nguyên để không làm lệch dữ liệu đã có:

| Metric | Method | Ý nghĩa |
|---|---|---|
| `serif_flare` | `peak_foot_band_over_stem_median_iln` | Độ loe ĐỐI XỨNG lớn nhất trong dải 1.25–15% x-height ÷ thân ở 35% (chỉ phần loe có ở cả hai bên thân; nét vào/ra một phía của chữ viết tay không tính) |
| `serif_head` | `top_left_flag_over_stem_median_iln` | Phần mực nhô sang trái thân trong dải 3–15% x-height dưới đỉnh ÷ thân |

Quy tắc phân loại (`classify.rs`, `small_foot_weight`):

- `serif_foot` trong 1.2–1.6: bằng chứng Serif theo `serif_flare`/`serif_head` (1.0 / 1.5 / 2.0; ngưỡng 1.3, 1.5, >1.6 và 0.15, 0.25, 0.4). Nặng hơn từ khoá tên (0.8).
- `serif_foot` < 0.9 (chân nhọn) mà có cờ hoặc loe: không bỏ phiếu Sans 2.0, chỉ Serif 1.0. Không có dấu hiệu serif thì giữ nguyên Sans như trước.
- `serif_foot` > 1.6 mà chân chỉ nhô một phía (`serif_flare` < 1.15) và không có cờ trên (`serif_head` < 0.05), như đuôi chữ l của Mooli/Bellota, không còn được tính là Serif.
- Bản phân tích cũ (không có hai metric mới) phân loại y hệt trước (test `old_analyses_without_flare_classify_exactly_as_before`).
- Face đã phân tích với `core-0.8.0` tự được phân tích lại (cơ chế `analyzer_version` sẵn có của store). `reference-v1.json` dựng lại: chỉ đổi `analyzer_version` (16 trục tham chiếu không đổi, 3813 face).

## 3. Kiểm thử

- Tổng hợp (`axes/stroke.rs`): thân thẳng = 1.0; slab > 1.6; bracket 1.3–1.7; **chân nhọn** tái hiện lỗi (phép đo cũ < 0.6, phép đo mới 1.4–1.6); nét vào một phía ≈ 1.0.
- Fixture OFL (`tests/fixtures`, DM Serif Display và Chonburi, `google/fonts@47d54ca1`): foot trong 1.2–1.7, flare ≥ 1.5, ra Serif; Roboto/Open Sans/Fira Sans: flare < 1.15, cờ < 0.1, vẫn Sans.
- Tên không lấn số đo: "… neue" / "… grotesk" với foot 1.54, loe 1.63, cờ 0.30 vẫn Serif.

## 4. Eval phân loại (google/fonts, mặt Regular, 1722 họ có đáp án, `--split all`)

| | đúng | Uncertain | sai | đúng/đã gán |
|---|---|---|---|---|
| Trước (core-0.8.0) | 1208 | 451 | 63 | 95.0% |
| Sau (core-0.9.0) | 1216 | 449 | 57 | 95.5% |

Không tụt. Đổi trạng thái: thêm đúng 6 do chân nhỏ (Alegreya, Aref Ruqaa ×2, Cormorant Upright, DM Serif Display, Sahitya) và 5 chữ viết tay (Licorice, Ole, Over the Rainbow, The Girl Next Door, Warnes) nhờ bỏ phiếu Serif cho chân một phía; mất đúng 3 (Bilbo Swash Caps, KoHo, Tsukimi Rounded → Uncertain).

## 5. Mười lăm họ BeauType Regular (số đo, trước → sau)

Chỉ thay đổi 2 họ; 13 họ còn lại giữ nguyên (nhãn và độ tin cậy).

| Họ | foot | flare | head | trước | sau |
|---|---|---|---|---|---|
| Glyphius | 1.54 | 1.63 | 0.25 | Uncertain | **Serif 0.89** |
| Annam Neue | 0.61 | 1.00 | 0.42 | Sans 0.94 | **Uncertain** (tên "Neue" không còn quyết định) |
| Noirist | 0.26 | 1.00 | 0.52 | Uncertain | Uncertain (không còn candidate Sans) |
| Amibus | 1.93 | 1.93 | 0.46 | Serif 0.91 | Serif 0.91 |
| 11 họ còn lại | 1.00 | 1.00 | 0.00–0.59 | Sans/Uncertain | không đổi |

Chín họ foot = 1.00 là Sans thật theo hình (không chân, thân đều); không phải lỗi đo.
