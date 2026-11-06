# 00 — Tóm tắt: Plates (tên tạm "Font Desk")

- **Ngày:** 2026-11-06 · **Nhánh:** `docs/font-desk` · **Trạng thái:** đề xuất, chờ chủ app duyệt
- **Tài liệu:** `01_research.md` · `02_naming.md` · `03_concepts.md` · `04_spec.md` · prototype `docs/design/prototype/font-desk-v1.html` · ảnh chụp `shots/`

## Tên
- **Plate** (dự phòng **Case**). Một atlas là tập các plate: mỗi plate là một trang hoàn chỉnh. "Plate" còn là bản in và cái đĩa, nên hợp mảng F&B. Thắng 39/40 điểm trên 6 tiêu chí; "Font Desk" chỉ 18 điểm vì trùng "Add to Desk" trong code cũ.
- **Hệ từ:**
  - Plates: mục sidebar, thay "Pairing Labs".
  - Plate: đơn vị làm việc.
  - Neo: font bắt đầu.
  - Giọng: một font, tối đa 3.
  - Vai: một khối chữ.
  - Bố cục: Menu / Bảng hiệu / Bao bì / Brand sheet / Editorial.
  - Khay: bàn so sánh.
  - Thử / Đặt / Ghim; Sao.

## Hướng chọn: lai "Trang + Sổ giọng + Khay"
- **Trang** chiếm phần lớn diện tích. Đó là ấn phẩm thật với chữ tiếng Việt (menu có giá `65.000đ`, nhãn có khối lượng, chữ hoa có dấu chồng). Sửa chữ ngay trên trang.
- **Sổ giọng** là cột phải, đúng ý chủ app về cột ghi tên font. Cột liệt kê theo **giọng** chứ không theo từng khối chữ. Đổi một giọng thì mọi vai thuộc giọng đó đổi theo, nên trang giữ được 2–3 font.
- **Khay** nằm dưới trang:
  - Nguồn ngang hàng: Ghép · Giống · Cùng họ · Thư viện · Đã ghim.
  - Có chế độ An toàn / Có kiểm soát / Thử nghiệm và bộ lọc "kiểu ghép".
  - Rê chuột hoặc giữ Space để **thử** ngay trên trang. Bấm **Đặt** mới ghi vào Plate. Font cũ tự vào Đã ghim.
- **Luôn có gợi ý.** Khi nguồn Ghép rỗng, khay nói rõ lý do và vẫn điền font giống bên dưới.
- **Không gác cổng.** Font thiếu tiếng Việt hay có va chạm vẫn đặt được. App kiểm tiếng Việt trên **chính chữ của trang** và chỉ ra chữ nào thiếu, ở vai nào.
- **Vì sao chọn:** đạt 91/100 trên ma trận 10 tiêu chí. Hướng thuần của chủ app (A) đạt 66 vì chọn là thay ngay, không so được song song. Bản đồ (D) đạt 48. Wizard (E) đạt 51.

## Phạm vi
| MVP | P1 | Sau |
|---|---|---|
| Sidebar Plates + sao + Plates home; bố cục Menu, Bảng hiệu, Bao bì; sổ giọng 1–3 giọng; khay 5 nguồn, Thử/Đặt/Ghim, kiểu ghép (lọc phía client); kiểm tiếng Việt trên chữ trang; tự lưu, thùng rác 30 ngày, mở lại đúng trạng thái; độ sâu 0/1/2; bàn phím đầy đủ | Brand sheet, Editorial; nhân bản; lịch sử mốc; xuất PNG/PDF; thang cỡ chỉnh được; lưu vào collection | Nguồn "Bản đồ" quanh neo; màn So Plate; gợi ý học từ Plate của người dùng; MCP; chia sẻ file Plate |

**Backend cần thêm:**
- Bảng `plates`, `plate_voices`, `plate_roles`, `plate_tray`, `plate_events` trong `user.db`.
- Các lệnh `plate_*`.
- `text_coverage(face, text)`.
- Tham số `style` và `collectionId` thật cho `pair_candidates`.

Các lệnh `pair_candidates`, `assess_pair`, `similar_families`, `get_family_styles` đã có và dùng lại được (chi tiết ở `04_spec.md` §8).

## Quyết định cần chủ app
1. Duyệt tên **Plate** (hoặc Case), và giữ nguyên tiếng Anh trong giao diện tiếng Việt?
2. Plates thay "Pairing Labs" và thay luôn phương án "Lưu cặp vào collection"?
3. Khi mới tạo Plate, Giọng đọc có tự dùng ứng viên đầu tiên (ghi "gợi ý tạm · chưa đặt") không? Prototype đang làm theo cách này.
4. Bố cục MVP chỉ gồm Menu + Bảng hiệu + Bao bì, hay đưa Brand sheet vào ngay?
5. Giá trên Menu mặc định căn phải có dấu chấm dẫn, hay đặt cuối dòng?
6. Có gỡ "Add to Desk" (code cũ) khi Plates ra không?

## Đã kiểm
- Prototype mở bằng Chrome, **không có lỗi console**. Một kịch bản tự động chạy qua các luồng sau và không lỗi:
  - tạo Plate từ zerostate, thử (hover), đặt, đánh sao (sidebar cập nhật);
  - đổi 5 bố cục, 3 chế độ, kiểu ghép, 5 nguồn;
  - ghim, thêm giọng, độ sâu 2, thu khay, tô chữ thiếu;
  - phím `T` `2` `→`;
  - menu ngữ cảnh "Thêm vào Plate ▸" ở thư viện;
  - xoá rồi khôi phục.
- Ảnh chụp ở 1400×920 và 1100×800 nằm trong `shots/`, chụp bằng Chrome headless vì Paseo browser không chụp được tab nền. Lỗi thấy được trên ảnh đã sửa:
  - khay đẩy sổ giọng ra ngoài ở 1100 px và ở độ sâu 2;
  - dấu chồng bị cắt trên thẻ;
  - chữ bảng hiệu quá to.

## Giới hạn
- Nhãn ●◐◆○ và số đo trong prototype là **dữ liệu giả**, tính bằng luật rút gọn mô phỏng R1–R9. Font Google chỉ là font đứng thay.
- Chưa test với người dùng thật. Kế hoạch test 5 người nằm ở `04_spec.md` §11.
- Chưa kiểm dark mode. Chưa thử kéo thả bằng chuột thật trong ảnh chụp; luồng kéo thả có trong code nhưng chỉ kiểm bằng đọc code.
- Đối thủ tra qua web, không cài thử Typeface 4.3.
