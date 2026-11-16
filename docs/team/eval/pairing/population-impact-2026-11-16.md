# Ảnh hưởng dân số của P1–P4 (hợp vai), 04, 2026-11-16

Sau kết quả bộ mù (`blind/RESULTS-2026-11-16.md`): Lead duyệt P1–P4. Tài liệu này ghi **cái đã đổi, ảnh hưởng lên 2014 họ OFL, bộ vàng, và những gì chưa giải quyết**.
Không chỉnh theo bộ mù: mọi ngưỡng lấy từ dân số 1259 họ chữ đọc (gt Sans+Serif) của `classify-small-serif/after_core-0.9.0_regular.csv`.

## 1. Đã đổi (`fontatlas-core/src/pairing.rs`)

| # | Thay đổi | Ngưỡng và nguồn |
|---|---|---|
| P1 | `x_height_em > 0,8` → `fit = unknown` + lý do `measure_doubtful` (tham số `x_height`), mọi vai, kể cả `ViPolicy::Note`. Danh sách chỉ hiện good/ok nên font bị loại khỏi gợi ý | 53/53 họ "Guides" vượt 0,8; 0/51 họ Playwrite khác |
| P2 | Body và Caption có thêm tiêu chí **width** (dải p10–p90 chữ đọc: 0,80–1,06; Label đã có sẵn). Width chỉ **phạt** (xem 1.1) | p10/p90 width chữ đọc |
| P2 | Chữ thân (Body/Caption/Label): một tiêu chí lệch xa (ngoài p1/p99 chữ đọc) → trần **ok**; hai trở lên → trần **weak** | bảng `far_bounds`: x_height 0,36–0,72; aperture 25–131; aperture_e 0,035–0,42; contrast 1,02–8,4; spacing 0,027–0,294; stem 0,048–0,478; width 0,52–1,38 |
| P3 | Lý do của tiêu chí lệch xa mang tham số `far = 1` và đứng đầu danh sách lý do; **mức không bao giờ là good khi có lý do `far`** (test bất biến) | — |
| P4 | Heading (và Quote, xem 1.2): một hay nhiều tiêu chí lệch xa → trần **ok**, không hạ thêm | cùng bảng |

### 1.1 Width là tiêu chí "chỉ phạt"
Bản đầu thêm width vào trung bình có trọng số: làm **47 họ Body và 64 họ Caption được nâng mức** (ví dụ Cormorant Garamond weak → ok), trái mục đích. Nay width chỉ tham gia trung bình khi điểm của nó thấp hơn điểm trung bình của các tiêu chí còn lại; không đổi ngưỡng "quá thiếu dữ kiện → unknown". Kết quả: **0 lượt nâng mức ở mọi vai** (bảng 3).

### 1.2 Quote được áp luật như Heading
Lead duyệt Heading. Quote dùng chung kiểu tiêu chí (x-height, contrast, spacing) và P3 đòi "lý do lệch xa không đi cùng good", nên Quote cũng bị trần ok khi có tiêu chí lệch xa. Không áp cho Display (cần cá tính) và Numerals (chỉ hai tiêu chí).

## 2. Bộ vàng

`pairing_eval` trên `docs/team/eval/pairing_gold.csv`: **cặp 16/16, vai 11/11**, giống trước khi đổi.
Lưu ý: bộ vàng là bộ đã chỉnh, không phải bằng chứng chất lượng độc lập.

## 3. Ảnh hưởng lên 2014 họ (trước → sau, % của nhóm; `vi_missing` không tính vì CSV không có)

Công cụ: `fontatlas-eval/src/bin/rolefit_pop.rs`, chạy ở commit trước (`3b594691`) và sau.

**Chữ đọc (gt Sans+Serif, n = 1259)**

| vai | good | ok | weak | unknown |
|---|---|---|---|---|
| body | 80,7 → 77,1 | 13,2 → 15,5 | 3,7 → 4,8 | 2,4 → 2,5 |
| caption | 70,3 → 68,2 | 21,3 → 22,4 | 6,1 → 6,9 | 2,3 → 2,5 |
| heading | 90,7 → 88,5 | 5,8 → 7,9 | 1,2 → 1,2 | 2,3 → 2,5 |
| display | 10,4 → 10,3 | 5,2 → 5,1 | 82,1 → 82,1 | 2,3 → 2,5 |
| quote | 87,2 → 85,5 | 9,4 → 11,0 | 1,2 → 1,1 | 2,2 → 2,4 |
| label | 78,9 → 78,4 | 11,4 → 11,7 | 7,5 → 7,5 | 2,3 → 2,5 |
| numerals | 82,5 → 82,4 | 5,4 → 5,3 | 9,8 → 9,8 | 2,3 → 2,5 |

**Khác (Script, Mono, Blackletter, Uncertain…, n = 755)**

| vai | good | ok | weak | unknown |
|---|---|---|---|---|
| body | 30,9 → 25,3 | 21,2 → 22,5 | 46,0 → 42,4 | 2,0 → 9,8 |
| caption | 26,9 → 22,4 | 22,3 → 23,8 | 49,3 → 44,4 | 1,6 → 9,4 |
| heading | 67,8 → 51,5 | 17,5 → 29,8 | 13,1 → 9,3 | 1,6 → 9,4 |
| display | 58,5 → 50,7 | 7,8 → 7,8 | 32,1 → 32,1 | 1,6 → 9,4 |
| quote | 53,9 → 46,2 | 26,1 → 33,8 | 18,9 → 11,1 | 1,1 → 8,9 |
| label | 36,6 → 36,0 | 13,1 → 13,2 | 48,6 → 41,2 | 1,7 → 9,5 |
| numerals | 39,1 → 39,1 | 27,2 → 27,0 | 31,9 → 24,2 | 1,9 → 9,7 |

(Nhóm "khác": weak giảm vì 61 họ chuyển thành unknown, không phải vì được nâng mức.)

**Số họ đổi mức (toàn 2014 họ)**

| vai | hạ mức | trong đó chữ đọc | nâng mức | → unknown |
|---|---|---|---|---|
| body | 129 | 57 | **0** | 61 |
| caption | 87 | 37 | **0** | 61 |
| heading | 149 | 26 | **0** | 61 |
| quote | 80 | 22 | **0** | 61 |
| label | 9 | 5 | **0** | 61 |
| display, numerals | 0 | 0 | **0** | 61 |

Chữ đọc bị hạ ở Body: 57/1259 = 4,5%. Ví dụ: Lexend Exa/Giga/Mega/Peta/Tera/Zetta (spacing và width cực đoan) good → ok; Economica, Wire One, Zen Loop (stem rất mảnh/hẹp) good → weak; Orbitron, Jura (contrast cao) good → ok.

## 4. 61 họ chuyển thành unknown: xem tay

61 họ có x-height > 0,8 em: **53 họ Guides** + 8 họ khác. Tám họ khác, đọc theo bản chất họ (chưa render ra hình xem từng glyph; ghi rõ để khỏi nhầm là đã xem mắt):

| họ | gt trong tập | nhận định |
|---|---|---|
| Eater | Uncertain | font trang trí kiểu kinh dị, không phải chữ đọc |
| Jacquarda Bastarda 9 Charted, Jersey 10 Charted, Jersey 25 Charted, Yarndings 12 Charted, Yarndings 20 Charted | Uncertain | họ pixel "Charted" (nền ô lưới); đo bbox lẫn lưới như họ Guides |
| Zilla Slab Highlight | Serif (phân loại tự động: Uncertain) | kiểu highlight có khối nền; bbox cao |
| Boldonse | Sans | font display cực đậm; **có thể là bắt nhầm** (x-height lớn có thật). Chỉ ảnh hưởng vai Display (nay unknown thay vì good); các vai chữ thân vốn weak |

Kết luận: không họ nào trong tám là chữ đọc; một họ (Boldonse) là trường hợp có thể mất oan ở Display. Chấp nhận (1/2014 họ).

## 5. Phát lại 20 thẻ mô hình của bộ mù (chẩn đoán, không dùng để chỉnh)

| nhóm thẻ | trước (7 thẻ Không) | sau |
|---|---|---|
| c22 (Playwrite AR Guides) | Display good | **unknown** (P1), thẻ biến mất |
| c23 (Luxurious Roman làm Body) | good | weak (đã sửa từ `a3a0cf06`, không phải P1–P4) |
| c06, c10 (Lexend Peta/Tera làm Body) | good | **ok** (P2: spacing lệch xa); thẻ vẫn lọt ở mức ok |
| c11 (Handjet Heading) | ok | ok (P4 đã đặt trần ok; Platypi Body vẫn good) |
| c04 (Thasadith, stem 0,072), c21 (Geologica) | good | **không đổi**: không lệch "xa" (stem 0,072 > p1 0,048; Geologica `stem_heavy` nhẹ) |

Tức là P1–P4 loại hẳn 1 thẻ Không (cùng c23 đã loại sẵn) và hạ 3 thẻ xuống ok; **3 thẻ Không (c04, c11, c21) không được xử lý** vì lỗi của chúng không phải "lệch xa". Không kết luận gì về chất lượng từ phát lại này: chỉ bộ mù v2 mới kiểm được.

## 6. Còn lại, chưa làm

- Thasadith (stem rất mảnh làm Body) và các font "lệch vừa" (p1–p10, p90–p99): cần biết chủ app muốn gì trước khi hạ thêm; hiện chỉ có lý do `stem_light` kèm good.
- Kiểm lookalike (hai Sans gần nhau, c10/c21) cần vector ảnh ở bộ v2.
- Chuỗi vi/en cho `measure_doubtful` (05) và cách UI hiển thị tham số `far`.
