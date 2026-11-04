# Bàn giao: prototype UI v1 + Plan 6b → Lead

**Ngày:** 2026-11-04 · **Từ:** phiên cloud (tiếp nối Lead khi Lead hết quota) · **Cho:** FontAtlas·00 Lead

## 1. Bối cảnh
- Lead dừng lúc 07:31 UTC vì hết quota. Ghi chú để lại ở `docs/design/04_ui_direction_notes.md` (trên Mac).
- Phiên cloud làm tiếp bước 1 trong ghi chú đó: dựng prototype HTML và duyệt với chủ app. Sau đó viết Plan 6b.

## 2. Kết quả
| Thứ | Ở đâu |
|---|---|
| Prototype đã duyệt (7 vòng sửa) | `docs/design/prototype/fontatlas-ui-v1.html`. Xem trực tiếp: https://claude.ai/artifact/DEeiq6WtWRLYpjM7buhjo2 |
| Plan 6b (bản nháp, có mục 10 đối chiếu `dev/mvp`) | `docs/superpowers/plans/2026-11-04-plan6b-ui.md` |
| PR (draft, base `main`) | https://github.com/vietx2811/FontAtlas/pull/2, nhánh `claude/friendly-edison-514dez` |

**Lưu ý nhánh:** PR đang đặt base là `main`, chưa phải `dev/mvp`. Thay đổi chỉ có tài liệu (3 file mới), nên đưa vào `dev/mvp` bằng cherry-pick hoặc đổi base PR đều không xung đột code.

## 3. Quyết định chủ app đã chốt (ràng buộc cứng cho 6b)
1. Chỉ desktop, không làm cho điện thoại. Cửa sổ tối thiểu 1000×700.
2. Sidebar không bao giờ đè lên nội dung. Khi thu gọn, sidebar là dải icon 56px vẫn nằm trong layout. Không có kiểu bung ra khi rê chuột, không có ngăn kéo.
3. Ô tìm kiếm luôn ở thanh trên cùng, không bao giờ ẩn, mở bằng ⌘K.
4. Sidebar trong suốt theo material sidebar của macOS: không viền, không logo. Các nhóm Dự án, Nguồn, Thẻ mở/thu được.
5. Ô chữ mẫu nổi ở chân lưới. Chưa có nút định dạng.
6. Panel chi tiết:
   - dưới tên font là dòng tóm tắt;
   - kiểu chữ chọn bằng **dropdown**, mỗi lựa chọn hiện bằng chính kiểu đó;
   - thẻ thông tin lấy từ name table, mặc định chỉ hiện Thiết kế và Giấy phép, bấm mới mở thêm Hãng, Phiên bản, Bản quyền, Tệp.
7. Tab panel chỉ dùng icon: Aa · Ặ · Giải phẫu · Tương tự · Ghép cặp.

## 4. Việc Lead cần làm tiếp
1. **Review Plan 6b**, nhất là mục 10. Đáng chú ý:
   - T6 phải dựa trên `features/library-grid` của Plan 5, không dựa trên `VirtualizedFontGrid` cũ.
   - B3 (tiếng Việt) đã xong ở core.
   - Phần thiếu ở backend thu lại còn: `copyright`, toạ độ weight của từng named instance, và API đưa `Metadata` ra UI.
2. **Đưa tài liệu vào `dev/mvp`** (cherry-pick hoặc đổi base PR), rồi đóng hoặc merge PR vietx2811/FontAtlas#2.
3. **Chuyển phần thiếu backend (mục 6 và 10 của plan) cho 02**, chốt cùng Plan 6a.
4. **Việc treo từ trước:** agent review Plan 7 đã chết do hết quota, cần nhắn SendMessage để nó chạy tiếp rồi chuyển kết quả cho 06. Plan 6a của 02 cũng đang chờ duyệt.
5. **Hỏi chủ app 3 câu mở** (mục 9 của plan): Thêm font mở hộp thoại hay menu; Gần đây tính theo lần xem hay lần bật; hàng Profile có cần không.
6. Giao T1 (cửa sổ và vibrancy) và T2 (font cục bộ) trước, vì hai việc này ít đụng tới 02 và 06.
