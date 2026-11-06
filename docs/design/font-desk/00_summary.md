# 00 — Tóm tắt: Pairing Desk

- **Ngày:** 2026-11-06 (vòng 3) · **Nhánh:** `docs/font-desk` · **Trạng thái:** chờ chủ app duyệt hai chế độ Pairing / Compare
- **Tài liệu:**
  - vòng 3 (hiện hành): `06_v3_pairing_compare.md` · prototype `docs/design/prototype/pairing-desk-v3.html` · ảnh `shots-v3/`
  - vòng 2: `05_layout_options_v2.md` · `pairing-desk-v2.html` · `shots-v2/`
  - vòng 1 (lịch sử, cơ chế giọng/vai/nguồn/nhãn/dữ liệu vẫn đúng): `01`–`04` · `font-desk-v1.html` · `shots/`

## Đã chốt (chủ app)
- **Pairing Desk** là tính năng. **Case** là đơn vị làm việc. **Combo** là phương án ghép, nằm trong Case. Cả ba giữ tiếng Anh.
- Người dùng là Graphic Designer, Art Director, Layout Artist (và UI/Web). Nội dung mẫu đa ngành, tiếng Việt thật có dấu chồng.
- Nền layout là **phương án C — Cột vai + Cuộn chọn**.
- **Option Stepper** `‹ ▾ ›` thay mọi tab. **Cột mốc** trong Compare cố định.
- **Thuật ngữ font giữ tiếng Anh** (x-height, contrast, stress, aperture, weight, width, serif, Variable, Italic…), giải thích bằng gloss theo locale. Mọi chuỗi UI qua khoá i18n.

## Vòng 3: hai chế độ
| | Pairing | Compare |
|---|---|---|
| Để làm gì | Tìm và thử font cho từng vai, ghép giọng thành combo | Đặt nhiều font **hoặc** nhiều combo cạnh nhau theo chiều ngang |
| Màn hình | Cột vai · Cuộn chọn · Trang · **Dải chi tiết** dưới trang | Bảng so sánh ngang (Specimen / Số đo / Meta), hoặc Song bản |
| Pairing Style, Mức ghép | Có, ở đầu Cuộn chọn (nguồn Ghép) | Không có |
| Chuyển | `‹ Pairing ▾ ›`, `⌘1` | `⌘2`, `⇧C` (đưa font đang thử vào Compare), "So sánh các combo" |

- **Dải chi tiết** (`I` để thu gọn) — trả lời nhận xét 1:
  - thẻ **Đang dùng** và **Đang thử**: chữ thật của vai + meta (Designer, Foundry, Version, License, Glyphs, Styles, Variable axes, VI x/186);
  - bảng số đo: Đang dùng · Đang thử · **Δ**;
  - nhãn ●◐◆○ ở đầu dải; nút "Đặt vào vai" và "Đưa vào Compare".
- **Compare Font** — trả lời nhận xét 2:
  - mốc cố định là font đang dùng cho vai;
  - mỗi ô số có Δ so với mốc;
  - kéo đổi chỗ cột và hàng (trong cùng nhóm); sắp theo thông số; ghim, bỏ, thêm cột (tìm + gợi ý);
  - Đặt vào vai; Thử trong Pairing.
- **Compare Combo:** trang thu nhỏ, giọng ①②③, nhãn ①↔②, Δ x-height, contrast, VI thấp nhất, số font, license. Có Dùng combo, Mở trong Pairing, thêm combo từ gợi ý.
- **Giữ ngữ cảnh khi chuyển chế độ:**
  - Pairing → Compare mang theo font đang thử và font đã ghim;
  - Compare → Pairing mở Cuộn chọn với font đó ở trạng thái thử;
  - Đặt từ Compare thì font cũ thành một cột, không mất phương án.

## Thay đổi đáng chú ý so với vòng 2
- Panel chi tiết đặt **dưới trang**, không đặt bên phải. Panel bên phải làm trang co còn khoảng 360 px ở 1400; đặt dưới, trang giữ khoảng 660 px.
- "Chế độ" nay là Pairing / Compare. Bộ An toàn / Có kiểm soát / Thử nghiệm đổi tên thành **"Mức ghép"**.
- Tên Pairing Style chuyển sang tiếng Anh: Superfamily · Serif + Sans · Display + Text · Keep one, change one · Rule-breaking.
- `AXIS_META` (nhãn trục tiếng Việt trong code) cần đổi sang nhãn EN + gloss khi triển khai (06 §6).

## Dữ liệu
- Đã có: `MetadataDto` (designer, manufacturer, version, license, variableAxes), `FileInfoDto.numGlyphs`, `VietnameseDto` (186), `AxisReadingDto`.
- Cần thêm:
  - `faces_detail(keys[])` để lấy nhiều face một lần;
  - bảng `case_compare` để nhớ cột, thứ tự, nhóm đóng;
  - file gloss `locales/<lang>/terms.json`;
  - mẫu câu giải thích (Rust) chèn nhãn EN.

## Đã kiểm
- Prototype v3: **0 lỗi console**. Kịch bản tự động chạy hết các luồng chính của Pairing, Compare Font, Compare Combo, chuyển chế độ hai chiều, sáng/tối, 5 bố cục, và Case từ font thiếu tiếng Việt.
- 11 ảnh ở 1400×920 và 1100×800. Lỗi thấy trên ảnh đều đã sửa (06 §10).
- **Giới hạn:**
  - số đo, version, số glyph là dữ liệu giả;
  - kéo thả chỉ giả lập sự kiện;
  - chưa có locale thứ hai; chưa kiểm trình đọc màn hình; chưa test với người dùng thật.

## Câu hỏi cho chủ app
1. Dải chi tiết **dưới trang** (đề xuất) hay panel **bên phải**?
2. Hàng nguồn gợi ý (5 icon) giữ dạng nút bấm một lần, hay đổi sang Option Stepper cho đồng nhất?
3. Cột Compare Font dùng chung cho mọi vai, hay mỗi giọng một danh sách riêng?
4. Tên "Mức ghép", hay một từ tiếng Anh (ví dụ "Strictness")?
5. Ở 1100 px, dải chi tiết ẩn dòng meta. Chấp nhận được không?
