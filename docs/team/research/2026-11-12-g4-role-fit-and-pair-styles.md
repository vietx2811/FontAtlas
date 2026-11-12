# G4: điểm "hợp vai" và "phong cách pair" (đề xuất mô hình, chưa code)

Tác giả: 04 (ML/Typography), 2026-11-12. Chờ Lead duyệt trước khi code. 08 làm API trên mô hình này.
Nguồn lý thuyết (kiến thức đã biết, chưa mở lại từng trang trong phiên này): Bringhurst *The Elements of Typographic Style* (mục chọn và ghép chữ: cùng x-height, đủ tương phản hoặc gần như giống hẳn, tránh "gần giống mà không giống"); Typewolf (ghép serif + sans, một font cá tính + một font trung tính, tối đa 2–3 họ); Fontjoy (dùng độ giống bằng embedding để dựng "tương phản vừa phải"); RightFont, Typeface (gợi ý theo phân loại, danh sách yêu thích; không ghi vai). Cần chủ app hoặc 01 xác nhận nếu muốn trích dẫn trực tiếp.

## 1. Số đo sẵn có dùng được (analyzer core-0.9.0, `Anatomy`)

`x_height_em`, `cap_height_em`, `x_height_cap`, `ascender_em`, `descender_em`, `stem`, `darkness`, `contrast`, `contrast_h`, `stress_deg`, `aperture`, `aperture_e`, `width`, `mean_advance`, `width_cv`, `spacing`, `slant_deg`, `serif_foot`, `serif_flare`, `serif_head`, `counter`, `a_storeys`, `g_storeys`, `terminal`, `connect`, `bounce`. Kèm: class/subclass + độ tin cậy (taxonomy v2), phủ tiếng Việt (`vi_missing`), cấu trúc style (`StyleStructure`: weight, italic, width), `Kerning`, nhóm OpenType (`tnum`, `onum`, `smcp`…), trục biến thiên, embedding hình dạng, tên họ/nhà làm.
Mọi `Metric` có `quality`: số đo `Failed` thì không tính, bỏ tiêu chí đó và hạ độ tin cậy của điểm (không bao giờ điền mặc định).

## 2. Điểm hợp vai `fit(font, role) → 0..1` + lý do

Công thức: `fit = gate × Σ wᵢ·sᵢ / Σ wᵢ` với `sᵢ` ∈ [0,1] là hàm "vùng đẹp" của một số đo (1 trong vùng đích, giảm tuyến tính ra ngoài), `gate` là tích các cổng cứng. Điểm được làm tròn 5% và đi kèm tối đa 3 lý do (mã + tham số; UI dịch vi/en, không có câu tiếng Việt trong backend).

| Vai | Cổng (hạ điểm mạnh) | Tiêu chí chính (trọng số) | Ghi chú |
|---|---|---|---|
| **Body** (văn bản) | `vi_missing = 0` (còn thiếu → trần 0.4, mã `vi_incomplete`); class không thuộc script/display/blackletter/handwriting; có weight Regular (400±100) | x-height lớn `x_height_em` 0.46–0.54 (3); mở `aperture`, `aperture_e` cao (2); `contrast` ≤ ~3 (2); `spacing` vừa/rộng (1); `counter` mở (1); `width_cv` thấp (1); có Italic + Bold cùng họ (1) | Bringhurst/Typewolf: văn bản cần ổn định, ít cá tính |
| **Caption / Small** | như Body; `width` không condensed | x-height lớn hơn nữa 0.50+ (3); `spacing` rộng (2); `stem` không mảnh (2); `contrast` thấp (2); aperture mở (2) | cỡ nhỏ hỏng đầu tiên ở nét mảnh và counter kín |
| **Heading / Subhead** | `vi_missing ≤ ngưỡng nhỏ` (cảnh báo, không chặn) | `darkness` vừa–đậm (2); `contrast` 1–6 chấp nhận (1); có weight Semibold/Bold (2); `spacing` không quá thưa (1); class serif/sans/slab đều được | Heading tha thứ hơn Body |
| **Display** | không cổng aperture; nếu `vi_missing > 0` chỉ hạ nhẹ (mã `vi_gaps_display`) | cá tính: `contrast` cao hoặc class display/script/blackletter/slab đậm (3); `spacing` chặt (1); đủ độ lớn mặt (`darkness` cao) (1); phạt font cá tính mà quá trung tính (1) | Display *được phép* tương phản cao, nét đặc trưng |
| **Quote** | như Heading | có Italic hoặc serif tương phản vừa (2); `slant_deg` nếu italic (1); `x_height_em` vừa (1) | |
| **Label / UI** | `vi_missing = 0`, weight ≥ Medium có | `width` condensed–normal (1); caps rõ, aperture mở (2); nét đều `stem` ổn định (2) | |
| **Numerals** | có `tnum` (cổng; thiếu → trần 0.5) | `width_cv` của chữ số thấp; lining/oldstyle theo chọn; nét đều | số đo chữ số riêng là việc thêm nhỏ nếu cần |

Vai tùy ý (G3): nếu tên khớp một vai trên (không phân biệt hoa thường, gợi ý), dùng hồ sơ đó; tên lạ → dùng hồ sơ gần nhất theo kích thước tương đối do UI cung cấp (`text` / `heading` / `display`), mặc định `heading`.

## 3. Phong cách pair `pairScore(a, b, style) → 0..1` + lý do

Ba lớp kiểm chung cho mọi phong cách (đã áp dụng trước khi chấm theo phong cách):

1. **x-height**: Bringhurst, hai font cùng khối văn bản nên có `x_height_em` chênh ≤ 0.04; chênh nhiều thì cảnh báo `xheight_mismatch` (không áp dụng khi một bên là Display).
2. **Vùng "gần giống mà không giống"**: độ giống embedding (cosine) 0.55–0.85 và khác họ → phạt (`too_close_not_same`); ≥ 0.85 coi như "cùng cấu trúc", ≤ 0.35 coi như "đối lập". Ngưỡng cần hiệu chuẩn (mục 5).
3. **Tiếng Việt**: cả hai đạt vai của mình, nếu một bên thiếu thì ghi `vi_incomplete`; không chặn.

| Phong cách | Cách chấm (tóm tắt) |
|---|---|
| **Tương phản (serif + sans)** | một bên class serif/slab, một bên sans; thưởng khi `x_height_em` gần nhau, `width` tương đương, `darkness` cùng bậc; thưởng nhẹ khi subclass cùng thời (xem "cùng thời kỳ"). |
| **Cùng họ / superfamily** | nhận diện theo họ typographic hoặc gốc tên (Source Sans/Serif/Mono, IBM Plex, Noto, Merriweather + Sans…), cùng nhà làm; điểm cao khi cùng họ khác vai (Regular body + Bold heading). Giải thích: "cùng bộ, hợp sẵn". |
| **Cùng cấu trúc** | khoảng cách vector (`x_height_cap`, `contrast`, `stress_deg`, `aperture`, `terminal`, `a_storeys`, `g_storeys`) nhỏ; khác class vẫn được (vd. humanist sans + oldstyle serif). Điểm = 1 − khoảng cách chuẩn hoá. |
| **Cùng thời kỳ** | bảng ánh xạ subclass → thời kỳ (venetian/garalde = Renaissance–Baroque, transitional/didone = Enlightenment–Modern, grotesque/neo_grotesque/geometric = công nghiệp–hiện đại, humanist sans ≈ thủ công thế kỷ 20…); cùng nhóm thì điểm cao. Bảng nhỏ, tay soạn, sửa được. |
| **Đối lập mạnh (Display + trung tính)** | vai Display cho font cá tính cao (fit(Display) cao) + vai Body cho grotesque/neo_grotesque/humanist trung tính (fit(Body) cao); phạt khi cả hai đều cá tính. Giải thích: "một bên nói, một bên đứng yên". |
| **Hài hoà (Fontjoy-like)** | cosine 0.85+ nhưng khác họ, khác weight; dành cho người muốn "gần mà tinh tế". |

Tổng hợp một combo (nhiều vai): `combo = trung bình có trọng số (Body nặng nhất) của fit(role)` × `trung bình các pairScore` của các cặp liền kề, phạt khi quá 3 họ (mã `too_many_families`). Mỗi gợi ý trả `score`, `reasons[]` (≤ 3 mã + tham số), `warnings[]`.

## 4. Giao diện dữ liệu đề xuất cho 08

```
role_fit(face, role)        -> { score, reasons: [{code,args}], warnings, confidence }
pair_score(a, b, style, roles?) -> { score, reasons, warnings }
suggest_partners(anchor, role, style, limit, scope: library|case) -> [{face, score, reasons, warnings}]
suggest_combos(name_hint?, style, library) -> [{ roles: {role:face}, score, reasons }]
```

Mã lý do (ví dụ): `x_height_large`, `aperture_open`, `contrast_low`, `vi_incomplete`, `tnum_missing`, `xheight_match`, `xheight_mismatch`, `class_contrast_serif_sans`, `same_superfamily`, `same_structure`, `same_era`, `display_vs_neutral`, `too_close_not_same`, `too_many_families`. Thuật ngữ font hiển thị bằng tiếng Anh + TermTip (quy tắc chung của dự án).
Mô hình nằm ở crate mới nhỏ hoặc `fontatlas-core` (thuần hàm trên số đo, không I/O) để test được; 08 chỉ gọi.

## 5. Kiểm chứng (đề xuất, làm trước khi gắn UI)

- **Neo dương**: cặp trong cùng superfamily OFL (Source Sans/Serif, IBM Plex Sans/Serif, Merriweather/Merriweather Sans, Noto Sans/Serif…); neo âm: cặp ngẫu nhiên và hai font gần giống nhau khác họ. Đo xếp hạng AUC của `pairScore`.
- **Role-fit**: lấy từ nhãn có sẵn: Body kỳ vọng cao cho họ Roboto/Open Sans/Source Serif/Lora…, thấp cho blackletter/pixel; Display kỳ vọng ngược lại. Đo tương quan thứ hạng với danh sách tay (30–40 họ, 04 soạn, chủ app duyệt).
- **Hiệu chuẩn ngưỡng** "vùng đẹp" trên phân bố số đo thật của 224 họ OFL taxonomy v2 (phân vị 10/90) thay vì số cứng; ghi lại trong eval.
- **Chủ app** xem 20 gợi ý mù (không thấy điểm) và chấm đồng ý/không → số liệu chấp nhận. Mục tiêu bắt đầu: ≥ 70% đồng ý ở top-3.

## 6. Giới hạn và rủi ro

- Số đo là hình dạng, không phải ý nghĩa/thương hiệu; điểm chỉ là gợi ý, luôn kèm lý do và cho đổi.
- Phân loại class còn Uncertain khá nhiều (≈ 19% họ trong taxonomy v2): khi class Uncertain, dùng số đo thô và hạ `confidence`.
- Embedding chỉ có khi thư viện đã chạy embed; thiếu thì bỏ tiêu chí 2 và `too_close_not_same`.
- Nhận diện superfamily bằng tên là heuristic; sai thì chỉ mất lý do "cùng bộ", không mất cả gợi ý.
- Ngưỡng mục 2/3 là giả thuyết khởi đầu, chưa đo.

## 7. Cần Lead duyệt

1. Bộ vai và hồ sơ ở mục 2 (đủ chưa; có cần Numerals ngay không).
2. Sáu phong cách ở mục 3 (bỏ/thêm), đặc biệt "Hài hoà" có giữ không.
3. Cho phép 04 đặt hàm thuần trong `fontatlas-core` (module `pairing`) và 08 gọi qua command.
4. Cách kiểm chứng ở mục 5 (nhất là chủ app xem 20 gợi ý mù).
