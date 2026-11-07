# Font Desk D1 — báo cáo phân loại — Vòng 2b (sau review độc lập, core-0.6.0)

Vòng 2b thay các số của "Vòng 2" bên dưới (giữ lại để đối chiếu). Cấu hình chốt: `CONF_MIN = 0.88`, hiệu chỉnh fit lại trên tune sau khi sửa
tín hiệu cứng Mono / từ khoá / italic.

## Đã sửa theo review

1. **Mono không còn bị hiệu chỉnh hạ về Uncertain** khi có tín hiệu cứng (`post.isFixedPitch` hoặc `width_cv < 0.005`) và không có bằng chứng Script/Display
   ngược lại. Playfair Display = Serif và Source Code Pro = Mono được gán thật (test `tests/classify.rs`, `tests/analysis.rs` khôi phục assert cụ thể).
   Trường `classification` (chuỗi cũ) vẫn `None` khi Uncertain; UI dùng `classificationInfo.candidate`.
   Tên font chứa "Display" (Playfair **Display**) là nhãn optical size, không phải class → bỏ "display" khỏi từ khoá Display.
2. **Facet/lọc không bỏ sót face**: face thoả bộ lọc nhưng không có hồ sơ trong Index (chưa phân tích, JSON hỏng) vào bucket `unclassified`
   (facet + giá trị lọc `classes: ["unclassified"]`, tách khỏi `uncertain`) → tổng facet = tổng thư viện. Chưa có test với thư viện có face lỗi thật
   (test hiện chỉ kiểm thư viện khoẻ: không có bucket, lọc rỗng).
3. **Eval trên mọi mặt** (`classify_eval --faces all`): Bold/Italic/Condensed… Bảng theo nhóm style bên dưới. Phát hiện: cue nghiêng (`slant ≥ 5`) đẩy
   italic của Serif/Sans sang Script (sai 23% trên phần đã gán). Sửa: `ClassifyInput.italic` (italic_angle ≠ 0 hoặc tên Italic/Oblique) → bỏ cue nghiêng.
   Sau sửa: italic sai 2,7% (test).
4. **Minor**: usage tính lại cho class ghi đè; `vote` dùng `Class::snake()`; từ khoá khớp nguyên từ (hậu tố `*` = tiền tố; Monoton/Monofett không khớp `mono`,
   Romanesco không khớp `roman`); tên tách từ một lần mỗi face; cache phân loại hiệu lực theo (bundle, thế hệ ghi đè); ghi đè kiểm face tồn tại;
   test migration v3→v4 có dữ liệu.
5. `capabilities_minimal` (tests/config.rs) lỗi sẵn trên dev/mvp d452d2d (thêm `core:window:allow-set-theme` nhưng không cập nhật test): đã cập nhật test cho khớp.

## Số liệu thật (test split, cấu hình cuối)

| Phép đo | Số mặt | Sai/đã gán | Độ phủ | Subclass đúng/đã gán |
|---|---|---|---|---|
| Chỉ mặt Regular (như vòng 2) | 1045 | **5.0%** (23/456) | 43.6% | 70.9% (bỏ hàng Mono: **70.1%**) |
| **Mọi mặt của họ** (Bold, Italic, Condensed…) | 2001 | **3.6%** (30/825) | 41.2% | **62.6%** (bỏ Mono: 61.1%) |
| Tune, chỉ Regular | 973 | 4.9% | 39.7% | 75.0% (bỏ Mono: 74.1%) |
| Tune, mọi mặt | 1742 | 4.8% | 37.3% | 65.2% |

Nói thẳng:
- **Subclass 70,9% là sát ngưỡng** (bỏ hàng Mono còn 70,1%; trên mọi mặt chỉ 62,6% — luật subclass Sans/Serif dựa vào aperture/stress/contrast
  lệch ở Bold/Italic, **chưa đạt 70% khi tính mọi mặt**).
- **Luật subclass Mono được chỉnh sau khi đã thấy ma trận test** (dữ liệu tune 19/19 là sans_mono nên chỉnh theo tune, nhưng thứ tự thời gian là như vậy).
- **Sai 5,0% ở Regular chỉ vừa chạm ngưỡng**; sau khi Mono được bỏ qua hiệu chỉnh, sai tăng từ 4,1% lên 5,0% (Mono precision thấp vì CJK fixed-pitch, barcode, bitmap).
- Italic: vẫn chỉ ~32% độ phủ; Bold ~28%; Black ~15%; Condensed ~51%.

## Theo nhóm style (test, mọi mặt)

## Theo nhóm style (class cuối, tại CONF_MIN)

| nhóm | số mặt | đúng | sai | Uncertain | sai/đã gán | độ phủ |
|---|---|---|---|---|---|---|
| black/heavy | 66 | 10 | 0 | 56 | 0.0% | 15.2% |
| bold | 170 | 45 | 3 | 122 | 6.2% | 28.2% |
| italic | 352 | 109 | 3 | 240 | 2.7% | 31.8% |
| light/thin | 129 | 81 | 2 | 46 | 2.4% | 64.3% |
| medium/semibold | 132 | 63 | 0 | 69 | 0.0% | 47.7% |
| regular | 1020 | 420 | 22 | 578 | 5.0% | 43.3% |
| width (condensed/extended) | 132 | 67 | 0 | 65 | 0.0% | 50.8% |

Tune (mọi mặt):

## Theo nhóm style (class cuối, tại CONF_MIN)

| nhóm | số mặt | đúng | sai | Uncertain | sai/đã gán | độ phủ |
|---|---|---|---|---|---|---|
| black/heavy | 56 | 6 | 0 | 50 | 0.0% | 10.7% |
| bold | 137 | 37 | 3 | 97 | 7.5% | 29.2% |
| italic | 308 | 84 | 5 | 219 | 5.6% | 28.9% |
| light/thin | 108 | 65 | 3 | 40 | 4.4% | 63.0% |
| medium/semibold | 101 | 47 | 0 | 54 | 0.0% | 46.5% |
| regular | 960 | 363 | 19 | 578 | 5.0% | 39.8% |
| width (condensed/extended) | 72 | 16 | 1 | 55 | 5.9% | 23.6% |

## Precision/coverage (test, mặt Regular, cấu hình cuối)

## Precision/coverage — toàn bộ (1045 họ)

| ngưỡng confidence | đã gán | độ phủ | đúng | precision (đúng/đã gán) |
|---|---|---|---|---|
| 0.00 | 997 | 95.4% | 789 | 79.1% |
| 0.50 | 922 | 88.2% | 764 | 82.9% |
| 0.60 | 872 | 83.4% | 746 | 85.6% |
| 0.70 | 787 | 75.3% | 699 | 88.8% |
| 0.75 | 731 | 70.0% | 658 | 90.0% |
| 0.80 | 641 | 61.3% | 595 | 92.8% |
| 0.85 | 523 | 50.0% | 502 | 96.0% |
| 0.88 | 414 | 39.6% | 402 | 97.1% |
| 0.90 | 314 | 30.0% | 305 | 97.1% |
| 0.93 | 169 | 16.2% | 166 | 98.2% |
| 0.95 | 114 | 10.9% | 114 | 100.0% |
| class cuối (CONF_MIN) | 456 | 43.6% | 433 | 95.0% |

Nhóm văn bản (Serif/Sans/Mono):

## Precision/coverage — chỉ nhóm văn bản (đáp án Serif/Sans/Mono) (609 họ)

| ngưỡng confidence | đã gán | độ phủ | đúng | precision (đúng/đã gán) |
|---|---|---|---|---|
| 0.00 | 581 | 95.4% | 544 | 93.6% |
| 0.50 | 567 | 93.1% | 533 | 94.0% |
| 0.60 | 559 | 91.8% | 528 | 94.5% |
| 0.70 | 538 | 88.3% | 514 | 95.5% |
| 0.75 | 518 | 85.1% | 497 | 95.9% |
| 0.80 | 487 | 80.0% | 472 | 96.9% |
| 0.85 | 433 | 71.1% | 426 | 98.4% |
| 0.88 | 349 | 57.3% | 345 | 98.9% |
| 0.90 | 275 | 45.2% | 272 | 98.9% |
| 0.93 | 154 | 25.3% | 152 | 98.7% |
| 0.95 | 110 | 18.1% | 110 | 100.0% |
| class cuối (CONF_MIN) | 390 | 64.0% | 375 | 96.2% |

**Chú thích định nghĩa:** các hàng theo ngưỡng đếm theo `confidence ≥ θ` và lớp đoán thô (`candidate` hoặc `class`). Hàng cuối "class cuối (CONF_MIN)" đếm theo class
trả về thực tế — gồm cả Mono/Symbol/Blackletter bỏ qua hiệu chỉnh nên số khác hàng 0.88 (ví dụ 456/433 so với 414/402) và khớp bảng Class.
Bảng Class dùng định nghĩa "class cuối".

## Tune/test chia theo tên họ

`family_split` băm **tên họ**, nên các họ cùng nguồn gốc (Noto Sans X, Noto Serif X, Bitcount…) có thể rơi vào hai phía; hiệu chỉnh/luật chọn trên tune có thể
"nhìn thấy" họ gần giống của test. Số trên test vì vậy hơi lạc quan; chưa đo lại với chia theo nhóm họ.

## Chi phí hiệu năng

`connect` + `bounce` đo thêm ~39 glyph/font: trên 300 font Regular, `analyze_path` 5,7 ms/font (một luồng, file đã cache) trong đó đo riêng hai chỉ số
này 0,5 ms/font (≤ 8% — thực tế thấp hơn vì phép đo riêng còn mở lại file). (`classify_eval --time 300`.)

## Ghi chú về `reference-v1.json`

Tôi sửa tay đúng một chỗ: `analyzer_version` (core-0.5.0 → core-0.6.0). Phần phân vị giữ nguyên vì không có trục nào của Index đổi
(connect/bounce chỉ dùng cho classify). Nội dung không được tái tạo bằng `refbuild`; cách bền hơn là cho test tham chiếu so tương thích thay vì so version (việc còn lại).

---

# Vòng 2 (số liệu trước review — thay bằng Vòng 2b ở trên)


Nhánh `feat/classify`. Đáp án và chia tune/test giữ nguyên như vòng 1 (xem phần "Vòng 1" bên dưới). Mọi hằng số
(ngưỡng cue, hệ số hiệu chỉnh, luật subclass Sans/Mono) được chọn trên **tune**; **test** chạy với cấu hình đã chốt.
Ngoại lệ cần nói rõ: sau lần chạy test đầu, tôi thấy ma trận subclass của test có 19 họ `sans_mono` bị gán `serif_mono`;
luật Mono được sửa theo dữ liệu **tune** (19/19 họ Mono có nhãn con là sans_mono, `serif_foot` trung vị 2.8 vì i/l/j có chân)
chứ không fit lại trên test, rồi chạy test lần cuối.

## Tóm tắt

| Chỉ tiêu | Vòng 1 (test) | Vòng 2 tune | Vòng 2 test | Mục tiêu |
|---|---|---|---|---|
| Sai trên phần đã gán (class) | 22.5% | 4.2% | **4.1%** | ≤ 5% — **đạt** |
| Phần đã gán / tổng (độ phủ) | 94.8% | 36.4% | 39.7% | chấp nhận giảm |
| Uncertain | 5.2% | 63.6% | 60.3% | — |
| Subclass đúng/đã gán (họ đúng class, đo trên class thô) | 64.9% | 74.9% | **70.7%** | ≥ 70% — đạt sát ngưỡng |

**Đánh đổi phải biết:** để sai ≤ 5% thì ~60% họ bị hạ về `Uncertain`. Máy vẫn trả `candidate` (lớp đoán thô) và `confidence`
để UI hiện "có lẽ là Sans" thay vì để trống. Toàn bộ đường cong precision/coverage ở dưới; muốn nhiều nhãn hơn thì hạ
`CONF_MIN` (ví dụ 0.80 → ~94% precision, 60% độ phủ trên test).

## Thay đổi vòng 2

1. **Hiệu chỉnh confidence** (`classify::calibrate`): hồi quy logistic L2 fit trên tune (973 họ, 5-fold CV nội bộ),
   đặc trưng = class đoán, lề điểm, bằng chứng Display/Script còn lại, và vài số đo "văn bản điển hình"
   (width_cv, spacing, aperture_e, counter, contrast, terminal, bounce, connect). `confidence` là P(đúng);
   dưới `CONF_MIN = 0.88` thì `class = Uncertain` và `candidate` giữ lớp đoán. Script fit: `docs/team/eval/classify_calibrate.py`.
2. **Số đo mới cho Script** (core, `anatomy.connect`, `anatomy.bounce`; `ANALYZER_VERSION = core-0.6.0`):
   `connect` = tỉ lệ chữ a–z có mực chạm cả hai mép advance; `bounce` = độ nhảy đỉnh/đáy chữ cao bằng x-height ÷ x-height.
   Trung vị tune: Script connect 0.40 / bounce 0.044, các class khác 0.15–0.19 / 0.013–0.016. Độ nghiêng thân vẫn đo bằng I/T/|.
   Recall Script tune 102 → 136/170 trước khi hiệu chỉnh.
3. **Display chỉ khi có bằng chứng hình học rõ** (điểm Display ≥ 2.0: từ khoá/Panose/sFamilyClass hoặc ≥ 2 số đo cực đoan);
   còn lại để Serif/Sans/Script + tag usage `Display`. GF dán nhãn Display theo mục đích nên Display gần như không còn được gán:
   test chỉ có 1/243 họ Display được gán đúng, 219 → `Uncertain`, 23 gán sang Serif/Sans/Script (một phần được GT chấp nhận khi họ có tag hình dạng).
4. **Subclass Sans** (luật mới fit trên tune): 'a' một tầng → geometric; aperture ≥ 92° hoặc aperture_e > 0.217 → humanist;
   width ≥ 0.97 & contrast ≤ 1.34 → geometric; còn lại grotesque (aperture < 68° hoặc terminal ≥ 1.55) / neo_grotesque.
   Sans (mô phỏng trên 249 họ tune có nhãn con): 72.3% → 76.3%. **grotesque ↔ neo_grotesque vẫn nhập nhằng** (test: neo_grotesque 9/21 đúng, 10 nhầm grotesque).
5. **Subclass Mono**: chỉ gán `serif_mono` khi tên/Panose/sFamilyClass khai báo serif, còn lại `sans_mono` (xem lý do ở trên).
   Ground truth cũng sửa: nhãn con của Mono lấy từ tag họ gốc (Sans → sans_mono, Serif/Slab → serif_mono) thay vì tag Sans/Serif chung chung.
6. **Script formal/handwritten**: thử cây quyết định và quét ngưỡng trên spacing/contrast/x-height/stress; không hơn mức
   "luôn handwritten" (78% trong GF), CV ~76%. Giữ luật cũ. Chưa có tín hiệu đáng tin để tách formal và handwritten.

## Kết quả test (1045 họ) — toàn bộ

## Class — tập `test` (1045 họ có đáp án)

| đúng | Uncertain | sai | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 398 | 630 | 17 | 38.1% | 95.9% |

Hàng = đáp án (primary), cột = dự đoán.

| GT \ dự đoán | Serif | Sans | Mono | Script | Display | Blackletter | Symbol | Uncertain |
|---|---|---|---|---|---|---|---|---|
| Serif | 96 | 0 | 0 | 0 | 0 | 0 | 0 | 84 |
| Sans | 5 | 231 | 1 | 0 | 0 | 0 | 4 | 154 |
| Mono | 0 | 0 | 16 | 0 | 0 | 0 | 0 | 18 |
| Script | 0 | 0 | 0 | 34 | 1 | 0 | 0 | 148 |
| Display | 7 | 14 | 0 | 2 | 1 | 0 | 0 | 219 |
| Blackletter | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 7 |

## Precision/coverage — toàn bộ (1045 họ)

| ngưỡng confidence | đã gán | độ phủ | đúng | precision (đúng/đã gán) |
|---|---|---|---|---|
| 0.00 | 996 | 95.3% | 793 | 79.6% |
| 0.50 | 934 | 89.4% | 777 | 83.2% |
| 0.60 | 872 | 83.4% | 752 | 86.2% |
| 0.70 | 782 | 74.8% | 699 | 89.4% |
| 0.75 | 717 | 68.6% | 647 | 90.2% |
| 0.80 | 627 | 60.0% | 587 | 93.6% |
| 0.85 | 517 | 49.5% | 495 | 95.7% |
| 0.88 | 408 | 39.0% | 395 | 96.8% |
| 0.90 | 305 | 29.2% | 297 | 97.4% |
| 0.93 | 160 | 15.3% | 157 | 98.1% |
| 0.95 | 107 | 10.2% | 107 | 100.0% |

## Kết quả test — chỉ nhóm văn bản (đáp án Serif/Sans/Mono)

## Precision/coverage — chỉ nhóm văn bản (đáp án Serif/Sans/Mono) (609 họ)

| ngưỡng confidence | đã gán | độ phủ | đúng | precision (đúng/đã gán) |
|---|---|---|---|---|
| 0.00 | 581 | 95.4% | 544 | 93.6% |
| 0.50 | 566 | 92.9% | 537 | 94.9% |
| 0.60 | 554 | 91.0% | 529 | 95.5% |
| 0.70 | 528 | 86.7% | 510 | 96.6% |
| 0.75 | 508 | 83.4% | 492 | 96.9% |
| 0.80 | 477 | 78.3% | 468 | 98.1% |
| 0.85 | 429 | 70.4% | 422 | 98.4% |
| 0.88 | 349 | 57.3% | 343 | 98.3% |
| 0.90 | 274 | 45.0% | 271 | 98.9% |
| 0.93 | 149 | 24.5% | 147 | 98.7% |
| 0.95 | 107 | 17.6% | 107 | 100.0% |

Nhóm văn bản: sai khoảng 1,7% ở độ phủ ~57% (ngưỡng 0.88) — đủ tốt cho các bộ lọc "Serif/Sans/Mono" trong Library.

## Kết quả test — theo hình dạng

## Precision/coverage — theo hình dạng (loại họ GF chỉ có nhãn Display theo mục đích, không tag hình dạng) (898 họ)

| ngưỡng confidence | đã gán | độ phủ | đúng | precision (đúng/đã gán) |
|---|---|---|---|---|
| 0.00 | 856 | 95.3% | 762 | 89.0% |
| 0.50 | 820 | 91.3% | 747 | 91.1% |
| 0.60 | 786 | 87.5% | 730 | 92.9% |
| 0.70 | 733 | 81.6% | 691 | 94.3% |
| 0.75 | 681 | 75.8% | 643 | 94.4% |
| 0.80 | 609 | 67.8% | 586 | 96.2% |
| 0.85 | 506 | 56.3% | 495 | 97.8% |
| 0.88 | 402 | 44.8% | 395 | 98.3% |
| 0.90 | 300 | 33.4% | 297 | 99.0% |
| 0.93 | 159 | 17.7% | 157 | 98.7% |
| 0.95 | 107 | 11.9% | 107 | 100.0% |

Bảng này loại họ GF chỉ mang nhãn Display theo mục đích (không tag hình dạng) — chỉ để tham khảo: app không hứa gán Display cho các font đó.

## Subclass (test)

## Subclass — họ đã đúng class và có nhãn con duy nhất (526)

| đúng | uncertain | sai | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 372 | 0 | 154 | 70.7% | 70.7% |

| GT subclass | dự đoán | số họ |
|---|---|---|
| casual_brush | casual_brush | 1 |
| casual_brush | transitional | 1 |
| didone | didone | 8 |
| didone | humanist_oldstyle | 2 |
| didone | slab | 2 |
| didone | transitional | 8 |
| formal | casual_brush | 1 |
| formal | formal | 19 |
| formal | handwritten | 7 |
| geometric | decorative | 1 |
| geometric | geometric | 32 |
| geometric | humanist | 8 |
| grotesque | geometric | 10 |
| grotesque | grotesque | 17 |
| grotesque | humanist | 5 |
| grotesque | neo_grotesque | 1 |
| handwritten | formal | 32 |
| handwritten | handwritten | 43 |
| humanist | geometric | 13 |
| humanist | grotesque | 3 |
| humanist | humanist | 155 |
| humanist | neo_grotesque | 2 |
| humanist_oldstyle | humanist_oldstyle | 16 |
| humanist_oldstyle | transitional | 8 |
| neo_grotesque | geometric | 5 |
| neo_grotesque | grotesque | 10 |
| neo_grotesque | humanist | 7 |
| neo_grotesque | neo_grotesque | 9 |
| sans_mono | sans_mono | 19 |
| sans_mono | serif_mono | 1 |
| serif_mono | sans_mono | 2 |
| serif_mono | serif_mono | 2 |
| slab | slab | 11 |
| slab | transitional | 1 |
| stencil | geometric | 1 |
| stencil | humanist | 1 |
| transitional | didone | 5 |
| transitional | humanist_oldstyle | 15 |
| transitional | slab | 2 |
| transitional | transitional | 40 |

## Kết quả tune (để so sánh, đã dùng để hiệu chỉnh)

## Class — tập `tune` (973 họ có đáp án)

| đúng | Uncertain | sai | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 339 | 619 | 15 | 34.8% | 95.8% |

Hàng = đáp án (primary), cột = dự đoán.

| GT \ dự đoán | Serif | Sans | Mono | Script | Display | Blackletter | Symbol | Uncertain |
|---|---|---|---|---|---|---|---|---|
| Serif | 80 | 0 | 0 | 0 | 0 | 0 | 0 | 96 |
| Sans | 2 | 202 | 1 | 0 | 1 | 0 | 1 | 132 |
| Mono | 0 | 1 | 9 | 0 | 0 | 0 | 0 | 16 |
| Script | 0 | 0 | 0 | 30 | 0 | 0 | 0 | 140 |
| Display | 6 | 9 | 0 | 7 | 3 | 0 | 0 | 225 |
| Blackletter | 0 | 0 | 0 | 0 | 1 | 1 | 0 | 10 |

## Precision/coverage — toàn bộ (973 họ)

| ngưỡng confidence | đã gán | độ phủ | đúng | precision (đúng/đã gán) |
|---|---|---|---|---|
| 0.00 | 934 | 96.0% | 739 | 79.1% |
| 0.50 | 877 | 90.1% | 726 | 82.8% |
| 0.60 | 816 | 83.9% | 699 | 85.7% |
| 0.70 | 718 | 73.8% | 631 | 87.9% |
| 0.75 | 651 | 66.9% | 581 | 89.2% |
| 0.80 | 568 | 58.4% | 523 | 92.1% |
| 0.85 | 457 | 47.0% | 435 | 95.2% |
| 0.88 | 352 | 36.2% | 338 | 96.0% |
| 0.90 | 262 | 26.9% | 254 | 96.9% |
| 0.93 | 139 | 14.3% | 137 | 98.6% |
| 0.95 | 87 | 8.9% | 87 | 100.0% |

## Giới hạn còn lại

- Độ phủ class chỉ ~38% toàn bộ ở mức sai ≤ 5% (nhóm văn bản ~57%). Script chỉ ~19% họ có nhãn (34/183), Display gần 0.
- Display của GF là mục đích dùng; không có tín hiệu hình học tách chúng khỏi Sans/Serif/Script thường. Hướng khả dĩ: dùng tag usage `Display` + người dùng ghi đè.
- grotesque ↔ neo_grotesque, transitional ↔ humanist_oldstyle, formal ↔ handwritten: nhập nhằng tự nhiên trong ground truth.
- Hiệu chỉnh fit trên 973 họ GF; ngoài GF (font thương mại có Panose/sFamilyClass đầy đủ) chưa đo. CV nội bộ trên tune cho precision ~95% ở 0.88–0.90.
- Font đã phân tích bằng core-0.5.0 sẽ được phân tích lại do `ANALYZER_VERSION` đổi; chúng chưa có `connect`/`bounce` đến lúc đó (cue Script bỏ qua khi thiếu).

---

# Vòng 1 (giữ nguyên để so sánh)

## Báo cáo vòng 1 (core-0.5.0)

Nhánh `feat/classify`. Đáp án: `google/fonts` @47d54ca — `tags/all/families.csv` (tag score ≥ 50, không mâu thuẫn; bản sao + SHA ở `docs/team/eval/`) cộng `category` trong METADATA.pb. Tune/test chia theo `fontatlas_embed::family_split` (ngưỡng chỉnh trên tune, báo cáo trên test, không chỉnh theo test).

## Kết luận về ngưỡng

| Chỉ tiêu | Ngưỡng | Tune (973) | Test (1045) | Đạt? |
|---|---|---|---|---|
| Class đúng / tổng | ≥ 90% | 74.5% | 73.5% | **KHÔNG** |
| Class đúng / đã gán (bỏ Uncertain) | — | ~77% | 77.5% | — |
| Subclass đúng (trong họ đúng class, có nhãn con duy nhất) | ≥ 70% | 72.3% | 64.9% | **KHÔNG** (test); tune đạt nhẹ |
| Phần sai chuyển thành Uncertain | "thà Uncertain còn hơn gán sai" | 36 | 54 | Một phần: Uncertain chỉ ~5%, còn ~21% gán sai |

Ngưỡng 90% class **không đạt** và theo tôi không đạt được bằng đo hình học thuần với ground truth này (xem Giới hạn). Không nới ngưỡng cho khớp số; số trên là số thật.

## Theo lớp (test)

- Tốt: Serif (159/180 ≈ 88%), Sans (350/395 ≈ 89%), Mono (33/34 ≈ 97%).
- Yếu: Script (91/183 ≈ 50%), Display (76/243 ≈ 31%), Blackletter (3/9, mẫu quá ít).
- Với riêng Serif/Sans/Mono (nhóm "văn bản"), độ chính xác ≈ 89% — gần ngưỡng; phần kéo tụt là Script/Display.

## Test split — kết quả đầy đủ

## Class — tập `test` (1045 họ có đáp án)

| đúng | Uncertain | sai | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 768 | 54 | 223 | 73.5% | 77.5% |

Hàng = đáp án (primary), cột = dự đoán.

| GT \ dự đoán | Serif | Sans | Mono | Script | Display | Blackletter | Symbol | Uncertain |
|---|---|---|---|---|---|---|---|---|
| Serif | 159 | 3 | 1 | 2 | 6 | 0 | 0 | 9 |
| Sans | 5 | 350 | 7 | 3 | 10 | 0 | 4 | 16 |
| Mono | 0 | 0 | 33 | 0 | 0 | 0 | 0 | 1 |
| Script | 2 | 39 | 1 | 91 | 37 | 0 | 0 | 13 |
| Display | 24 | 86 | 7 | 37 | 76 | 0 | 0 | 13 |
| Blackletter | 1 | 2 | 0 | 1 | 1 | 3 | 0 | 2 |

## Subclass — họ đã đúng class và có nhãn con duy nhất (481)

| đúng | uncertain | sai | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 312 | 0 | 169 | 64.9% | 64.9% |

| GT subclass | dự đoán | số họ |
|---|---|---|
| casual_brush | casual_brush | 1 |
| casual_brush | transitional | 1 |
| didone | didone | 8 |
| didone | humanist_oldstyle | 2 |
| didone | serif_mono | 1 |
| didone | slab | 2 |
| didone | transitional | 8 |
| formal | casual_brush | 1 |
| formal | formal | 17 |
| formal | handwritten | 5 |
| geometric | decorative | 1 |
| geometric | geometric | 28 |
| geometric | grotesque | 3 |
| geometric | humanist | 4 |
| geometric | neo_grotesque | 5 |
| geometric | serif_mono | 1 |
| grotesque | geometric | 10 |
| grotesque | grotesque | 13 |
| grotesque | humanist | 2 |
| grotesque | neo_grotesque | 7 |
| grotesque | serif_mono | 2 |
| handwritten | formal | 8 |
| handwritten | handwritten | 26 |
| humanist | geometric | 11 |
| humanist | grotesque | 2 |
| humanist | humanist | 140 |
| humanist | neo_grotesque | 20 |
| humanist | sans_mono | 1 |
| humanist | serif_mono | 11 |
| humanist_oldstyle | humanist_oldstyle | 17 |
| humanist_oldstyle | slab | 1 |
| humanist_oldstyle | transitional | 8 |
| neo_grotesque | geometric | 5 |
| neo_grotesque | grotesque | 13 |
| neo_grotesque | humanist | 3 |
| neo_grotesque | neo_grotesque | 10 |
| neo_grotesque | serif_mono | 3 |
| slab | serif_mono | 2 |
| slab | slab | 12 |
| slab | transitional | 1 |
| stencil | geometric | 1 |
| stencil | humanist | 1 |
| transitional | didone | 5 |
| transitional | humanist_oldstyle | 15 |
| transitional | serif_mono | 1 |
| transitional | slab | 2 |
| transitional | transitional | 40 |


## Giới hạn

1. **Display không có tín hiệu hình học ổn định.** Nhãn Display của GF là "mục đích dùng" hơn là hình dạng: nhiều font Display thực chất là Sans/Serif/Mono (Bitcount, Big Shoulders, Aubrey…). 86/243 Display bị gán Sans, 24 Serif. Đây là trần của cách tiếp cận, không phải lỗi ngưỡng.
2. **Script ↔ Display ↔ Sans.** 39/183 Script bị gán Sans (script nét đứng, brush đơn nét, Briem Hand, Chilanka); chưa có số đo độ liền nét (connectivity) nên không phân biệt được.
3. **Panose / sFamilyClass hầu như vắng ở font GF**, nên các tín hiệu "khai báo" gần như không tham gia; hệ thống dựa vào số đo + từ khoá tên. Với font thương mại có panose/sFamilyClass đầy đủ, kết quả kỳ vọng tốt hơn nhưng **chưa đo được** vì không có bộ đáp án phù hợp.
4. **Subclass:** Sans humanist/geometric/grotesque/neo_grotesque nhập nhằng tự nhiên (ranh giới do aperture và số tầng của 'a'); ~31 họ Sans bị gán nhãn con mono (serif_mono/sans_mono) khi font Sans có đáp án Mono được chấp nhận — cần xem lại ở vòng sau. Subclass cho Script (formal vs handwritten) lẫn nhau 13 họ.
5. **Blackletter** chỉ 9 họ trong test, không kết luận được.
6. Kết quả mỗi font (class, subclass, confidence, signals[], usage) đều giải thích được qua `signals`; mâu thuẫn mạnh → `Uncertain`, nhưng mức Uncertain hiện thấp (~5%) so với tỉ lệ sai (~21%): có thể nâng ngưỡng Uncertain để đổi recall lấy precision (precision đã gán hiện 77.5%) — đề xuất cho vòng sau nếu app muốn "ít sai hơn".

## Đề xuất vòng sau (không làm trong D1)

- Thêm số đo độ liền nét / số thành phần liên thông của chữ thường để tách Script.
- Nâng ngưỡng Uncertain cho lớp Display↔Sans (ưu tiên precision), hoặc hiện Display như "usage tag" thay vì class.
- Cho phép override hàng loạt theo họ (UI) — API hiện là theo từng face.

## Sản phẩm

- Core: `fontatlas_core::classify` (`classify`, `ClassifyInput`, `subclasses_of`), metadata mới (sFamilyClass, achVendID, fixed-pitch, symbol-cmap), `ANALYZER_VERSION = core-0.5.0`, `reference-v1.json` tái tạo.
- Store: user.db v4 `classification_overrides`.
- App: `FaceDetailDto.classificationInfo` (`ClassificationDto`), commands `set_classification_override` / `clear_classification_override`; override ưu tiên hơn tự động (`source = "override"`, giữ `autoClass`).
- Eval: `cargo run --release -p fontatlas-eval --bin classify_eval -- --gf <google-fonts> --tags docs/team/eval/gf-tags-families.csv --split test`.
