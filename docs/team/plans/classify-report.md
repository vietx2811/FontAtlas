# Font Desk D1 — báo cáo phân loại (classify, core-0.5.0)

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
