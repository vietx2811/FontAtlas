# 02 — Đặt tên tính năng và hệ thuật ngữ

- **Ngày:** 2026-11-06 · **Kết luận:** tên chính **Plate**, dự phòng **Case**. Đơn vị làm việc cũng gọi là **Plate**; mục sidebar là **Plates** (thay "Pairing Labs").

---

## 1. Tiêu chí

| Mã | Tiêu chí | Câu hỏi kiểm |
|---|---|---|
| T1 | **Signature** | Nghe một lần có nhớ không? Có câu chuyện riêng của FontAtlas không? |
| T2 | **Khớp "Atlas"** | Có thuộc cùng trường nghĩa bản đồ / sách bản đồ / in ấn không? |
| T3 | **Ngắn** | ≤ 2 âm tiết, vừa nút sidebar 240 px khi có số đếm |
| T4 | **Dễ nói Việt + Anh** | Designer Việt đọc được tự nhiên, dùng được trong câu ("mở Plate Phở Lý") |
| T5 | **Không đụng tính năng phổ biến** | Không trùng với "Collection", "Project", "Library", "Desk" (đã có trong code cũ: `services/desk`, nút "Add to Desk"), "Lab" (độ sâu 2 đang gọi là Phòng thí nghiệm) |
| T6 | **Đúng nghĩa việc làm** | Gợi được "dựng một trang chữ hoàn chỉnh quanh một font, để so và chốt" |

Thang 1–5. Trọng số: T1 ×2, T6 ×2, còn lại ×1 (tối đa 40).

## 2. Ứng viên và điểm

| # | Tên | Nghĩa / câu chuyện | T1 | T2 | T3 | T4 | T5 | T6 | Tổng |
|---|---|---|---|---|---|---|---|---|---|
| 1 | **Plate** | Ba nghĩa cùng lúc: **bản (plate) trong sách bản đồ** — một atlas là tập các plate, mỗi plate là một trang hoàn chỉnh; **bản in** trong nghề in; **cái đĩa** — hợp mảng F&B. Mỗi Plate là một trang chữ hoàn chỉnh của một use case | 5 | 5 | 5 | 4 | 5 | 5 | **39** |
| 2 | **Case** | Khay chữ chì (gốc của chữ "uppercase/lowercase") + "design case" của chủ app + tiếng lóng agency Việt "case khách" | 4 | 3 | 5 | 5 | 3 | 5 | **34** |
| 3 | Sa bàn | Mô hình địa hình để bàn kế hoạch; rất Việt, khớp bản đồ | 4 | 5 | 4 | 2 | 5 | 3 | 30 |
| 4 | Folio | Tờ của sách; số trang trong editorial | 3 | 4 | 5 | 4 | 3 | 3 | 28 |
| 5 | Galley | Khay dàn trang chì; bản in thử; bếp trên tàu (F&B) | 4 | 3 | 4 | 2 | 4 | 4 | 29 |
| 6 | Mise | Mise en place: bày nguyên liệu trước khi nấu — đúng ý "so trước khi đưa vào" | 4 | 1 | 5 | 2 | 5 | 4 | 29 |
| 7 | Legend / Chú giải | Ô chú giải trên bản đồ — hợp nhãn ●◐◆○ | 3 | 5 | 4 | 4 | 4 | 2 | 27 |
| 8 | La bàn / Compass | Tìm hướng; hợp "luôn có gợi ý" | 3 | 4 | 4 | 4 | 3 | 2 | 25 |
| 9 | Hải đồ / Chart | Bản đồ đi biển | 3 | 5 | 4 | 3 | 1 (đụng "biểu đồ") | 2 | 23 |
| 10 | Bàn sắp chữ / Composing | Bàn của thợ sắp chữ | 3 | 3 | 1 | 3 | 4 | 4 | 25 |
| 11 | Proof / Bản thử | Bản in thử để duyệt | 2 | 3 | 5 | 4 | 3 | 4 | 27 |
| 12 | Trắc địa / Survey | Đo đạc thực địa | 3 | 4 | 3 | 2 | 4 | 2 | 23 |
| 13 | Atelier | Xưởng | 2 | 1 | 4 | 3 | 3 | 3 | 21 |
| 14 | Pairing Lab | Mô tả thẳng | 1 | 1 | 2 | 4 | 1 (đụng "Phòng thí nghiệm") | 3 | 16 |
| 15 | Font Desk | Tên tạm | 1 | 1 | 3 | 5 | 1 (đụng "Desk" cũ) | 3 | 18 |
| 16 | Specimen | Trang mẫu chữ | 2 | 3 | 3 | 3 | 1 (đụng tab Specimen) | 3 | 20 |

## 3. Chọn

### Tên chính: **Plate**
- **Vì sao.** Đây là tên duy nhất vừa thuộc "Atlas" (atlas = tập các plate), vừa thuộc nghề in, vừa có duyên với F&B. Nghĩa cốt lõi khớp đúng việc: *một trang chữ hoàn chỉnh của một use case*, không phải một danh sách font.
- **Câu chuyện một dòng** (dùng cho onboarding và About): "Atlas là tập các plate. Mỗi Plate là một trang chữ bạn dựng quanh một font."
- **Rủi ro.** Người dùng tiếng Việt có thể chưa quen nghĩa "plate" trong sách bản đồ. Giảm bằng: icon là một trang có lề (không dùng icon cái đĩa), và câu chuyện một dòng ở zerostate. "Plate" giữ nguyên là danh từ riêng trong giao diện tiếng Việt (giống "Atlas", "Font").
- **Kiểm trước khi chốt:** đọc to 5 câu thật với chủ app ("Mở Plate Phở Lý", "Đánh sao Plate này", "Plate mới từ Playfair Display"); tìm nhanh xem có app font nào dùng "Plate" cho tính năng tương tự (tôi chưa thấy trong lượt tra cứu ngày 2026-11-06).

### Dự phòng: **Case**
- Dùng nếu chủ app thấy "Plate" xa lạ với khách Việt. "Case" khớp ý gốc của chủ app và tiếng lóng agency. Nhược điểm: ít chất Atlas, và "case" dễ chung chung trong câu.

## 4. Hệ thuật ngữ (dùng thống nhất trong UI, tài liệu, code)

| Khái niệm | Tiếng Việt (UI) | English | Định nghĩa | Ghi chú |
|---|---|---|---|---|
| Tính năng / mục sidebar | **Plates** | Plates | Không gian làm việc mở toàn phần phải; mục sidebar liệt kê Plate có sao | Thay "Pairing Labs" |
| Đơn vị làm việc | **Plate** | Plate | Một trang chữ: neo, các giọng, bố cục, chữ đã sửa, khay ứng viên. Tự lưu | Thay Case/Session/Project/Working space |
| Font khởi đầu | **Neo** | Anchor | Font hoặc family người dùng bắt đầu | Luôn thuộc một giọng |
| Ô font | **Giọng** | Voice | Một font đảm nhận một nhóm vai. Tối đa 3: Giọng chính, Giọng đọc, Giọng nhấn | Theo 09 D2 |
| Khối chữ | **Vai** | Role | Tên quán, tiêu đề, tên món, giá… Mỗi vai thuộc một giọng, có cỡ/độ đậm/hoa-thường riêng | |
| Loại ấn phẩm | **Bố cục** | Layout | Menu · Bảng hiệu · Bao bì · Brand sheet · Editorial | Thay "mode" để không trùng "chế độ" An toàn/Có kiểm soát/Thử nghiệm |
| Chế độ ghép | **Chế độ** | Mode | An toàn · Có kiểm soát · Thử nghiệm | Giữ nguyên như PairTab hiện tại |
| Kiểu ghép | **Kiểu ghép** | Pair style | Cùng họ · Có chân + không chân · Trưng bày + đọc · Giữ một đổi một · Phá cách | Bộ lọc trên nguồn Ghép |
| Bàn so sánh | **Khay** | Tray | Nơi đặt ứng viên cạnh nhau trước khi đưa vào Plate | Gợi "khay chữ" của nghề in |
| Thử tạm | **Thử** | Try | Xem ứng viên trên Plate khi giữ chuột/hover; không ghi | |
| Ghi vào Plate | **Đặt** | Set | Ứng viên thành font của giọng | |
| Ghim | **Ghim** | Pin | Giữ ứng viên trên khay qua các lần đổi nguồn | |
| Đánh dấu | **Sao** | Star | Đưa Plate lên sidebar | Dùng "sao" cho Plate, "tim/yêu thích" cho font — tránh lẫn hai khái niệm |

**Quy tắc chữ:** câu ngắn, động từ đầu ("Đặt", "Thử", "Ghim"), không dùng "elegant", "modern", "generic" hay từ chấm điểm ("tốt nhất", "khuyên dùng", "%").
