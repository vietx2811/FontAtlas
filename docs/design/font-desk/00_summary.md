# 00 — Tóm tắt: Pairing Desk

- **Ngày:** 2026-11-06 (vòng 2) · **Nhánh:** `docs/font-desk` · **Trạng thái:** chờ chủ app chọn phương án layout
- **Tài liệu:**
  - vòng 2: `05_layout_options_v2.md` · prototype `docs/design/prototype/pairing-desk-v2.html` · ảnh `shots-v2/`
  - vòng 1 (lịch sử, cơ chế vẫn đúng): `01`–`04` · `font-desk-v1.html` · `shots/`

## Tên và hệ từ (chủ app đã chốt)
- **Pairing Desk** là tính năng, mở toàn phần phải. Mục sidebar cùng tên liệt kê các Case có sao và dòng "Tất cả Case".
- **Case** là đơn vị làm việc: tự lưu, sao, thùng rác 30 ngày. Không dùng "Plate" nữa.
- **Combo** là một phương án ghép trong Case. Một Case có nhiều combo.
- Giữ các từ: Neo · Giọng (①②③, tối đa 3) · Vai · Bố cục · Chế độ · Kiểu ghép · Thử / Đặt · So sánh.

## Người dùng
- Graphic Designer, Art Director, Layout Artist (và UI/Web designer). Bỏ định hướng F&B.
- Bố cục mẫu đa ngành: **Editorial · Brand sheet · Poster · UI/Web · Bao bì**. Chữ mẫu là tiếng Việt thật có dấu chồng (Ữ Ặ Ệ Ỗ, ươ).

## Phương án layout vòng 2 (chủ app chọn; prototype có công tắc A / B / C)
| | Ý chính | T1–T3: click · quãng chuột (ước lượng) | Mạnh ở |
|---|---|---|---|
| v1 (mốc) | Trang + Sổ giọng + Khay dưới | ~5 · ~2.800 px | — |
| **A Tại chỗ** (đề xuất) | Thanh nổi ngay trên khối chữ: ‹ font ▾ › · đậm · hoa/thường · cỡ · so sánh. Danh sách thả xuống ngay dưới khối, hover là thử | ~7 · ~750 px | Đổi font một vai; trang lớn nhất (~85%); hợp bố cục dài |
| B Song bản | 2–3 bản trang đặt cạnh nhau, mỗi bản là một ứng viên; dải thông số mở được | ~3 · ~750 px | Thử nhiều ứng viên, trình bày cho khách |
| C Cột vai + Cuộn chọn | Cột vai → font sát trang; cuộn chọn mở cạnh dòng, lăn chuột là thử | ~4 · ~800 px | Bàn phím, thấy toàn hệ vai → font |

**Đề xuất:** A làm mặc định, kèm chế độ **So sánh**. Trong So sánh có nút "Song bản" (lấy từ B). Danh sách của A dùng `↑ ↓` và lăn chuột (lấy từ C).

## Thành phần mới (có trong prototype)
- **Option Stepper `‹ nhãn ▾ ›`** thay mọi dạng chấm và tab dài. Dùng cho Bố cục, Combo, Chế độ, Kiểu ghép, Vai, Sắp xếp.
  - Mũi tên chuyển ngay; bấm nhãn mở dropdown có icon, mô tả và "+ Thêm".
  - Lăn chuột hoặc `← →` khi focus.
- **Combo indicator** `‹ ⧉ Combo 2/3 ▾ ›`, phím `[` `]`. Dropdown gồm:
  - các combo kèm nhãn trục rút gọn; hover là thử trên trang;
  - 2 combo gợi ý chưa lưu (bấm là thêm);
  - Lưu combo hiện tại (`⌘⇧S`), Đặt làm combo chính, So sánh các combo.
- **So sánh mở rộng** (nút `columns-3`, phím `C`):
  - Cột mốc cố định. Các cột font đặt cạnh nhau.
  - Hàng specimen (tiêu đề, đoạn văn, dấu chồng) và hàng thông số (số đo + đơn vị + ●◐◆○, cổng).
  - Kéo đổi chỗ cột và hàng; sắp theo thông số ↑↓; ghim cột; bỏ cột; thêm cột; Đặt; xem Song bản.
- **Icon lucide** cho mọi nút, tag, chip; bảng ánh xạ đủ ở 05 §7.
  - Luôn hiện: thao tác chính và trạng thái.
  - Hiện khi hover/focus: thao tác phụ trên dòng/cột.
- **Tổ chức tuỳ chọn** theo ba tầng:
  - thanh trên gom thành 4 cụm;
  - popover Lọc (`F`) có 4 nhóm và mục Nâng cao đóng sẵn;
  - chip lọc đang bật, gỡ được.

## Đã kiểm
- Prototype v2: **0 lỗi console**. Kịch bản tự động chạy hết mọi luồng của A, B, C, Combo, Lọc, So sánh (kéo thả, sắp, ghim), sáng/tối và Case từ font thiếu tiếng Việt.
- 15 ảnh ở 1400×920 và 1100×800. Lỗi thấy trên ảnh đều đã sửa (chi tiết ở 05 §10).
- **Giới hạn:**
  - số đo và nhãn là dữ liệu giả; font Google là font đứng thay;
  - kéo thả chỉ giả lập sự kiện, chưa kéo bằng chuột thật;
  - chưa test với người dùng; quãng chuột là số ước lượng trên sơ đồ.

## Quyết định cần chủ app
1. Phương án mặc định: **A** (đề xuất), B hay C? Giữ B làm "Song bản" trong So sánh?
2. Combo nằm trong Case (đề xuất), hay mỗi combo là một Case riêng?
3. Option Stepper thay mọi tab, hay giữ tab ngang cho Bố cục?
4. Cột mốc trong So sánh cố định (đề xuất), hay kéo được như cột khác?
5. Giữ "Pairing Desk", "Case", "Combo" bằng tiếng Anh trong UI tiếng Việt?
