# QA native — giao diện tiếng Anh (milestone 6)

Chạy: `QA_LANG=en bash UX/src-tauri/tests/qa/run.sh` trên bản debug (dữ liệu tạm, không chạm thư viện thật).
Kết quả trên dev/mvp @8e7670c0: toàn bộ bước OK (exit 0). Ảnh `en-*.png` là ảnh chụp trong lượt đó; chủ đề sáng/tối theo từng bước (hậu tố -light/-dark).

Điều chỉnh QA để chạy được cả hai ngôn ngữ:
- `run.sh`: với `QA_LANG=en`, các bộ chọn `aria-label` tiếng Việt và nút "Xem family" được đổi sang chuỗi tiếng Anh tương ứng của app.
- `main_flows.js`: các regex phụ thuộc chữ (đủ|full, chưa chọn|not selected, Xoá tất cả|Clear all, Hoàn tác|Undo, Bật font tự động|Auto-activation) và dấu thập phân (0,528 / 0.528) khớp cả hai ngôn ngữ.
- `run.sh` in kết quả không còn sập khi một bước thiếu `detail`.

## Cần sửa (xem đủ 14 ảnh, chưa sửa trong code)

Mức: **A** = nhìn là thấy lỗi, **B** = chưa đẹp, **C** = cần kiểm trước khi coi là lỗi.

1. **A · Auto-activation (`en-auto-activation-*`)**: cột trạng thái chữ to và không đều. "Dev build (separate data)", "Not installed", "Open InCopy once, then come back" dùng cỡ chữ lớn (~17px), trong khi cột nguồn và phụ đề chỉ ~11px. Chữ dài như "Dev build (separate data)" và "Open InCopy once, then come back" tràn xuống 2 dòng, đẩy hàng cao thấp khác nhau (Photoshop/After Effects cao hơn Illustrator/InDesign). Biểu tượng "!" nằm riêng một dòng dưới chữ trạng thái, không cùng dòng. Cần: một cỡ chữ cho cột trạng thái, độ rộng cột cố định, câu dài rút gọn (ví dụ "Dev build · separate data").
2. **B · Auto-activation**: tiêu đề trang, nhãn công tắc và từ "Auto-activation." ở cuối đoạn giới thiệu lặp lại cùng một cụm ba lần. Đoạn giới thiệu nên dùng tên gọi khác hoặc bỏ cụm cuối.
3. **B · Auto-activation**: "Font-request source (nothing is installed in the app)" xuống 2 dòng ở mọi hàng và chiếm nhiều chỗ; nút "Install" của Illustrator, InDesign, InCopy rất nhạt (có vẻ bị vô hiệu) trong khi hàng không bị vô hiệu.
4. **B · Bộ lọc nâng cao (`en-advanced-sheet-*`, `en-lang-picker-dark`)**: nhãn "Include hybrid fonts" và "Confident labels only" cỡ chữ lớn hơn hẳn các nhãn khác trong cột Class (tiếng Anh nhưng cũng có thể có ở tiếng Việt: cần xem ảnh vi). Nhãn "Include hybrid fonts" lại mờ còn ô chọn thì không, kèm dòng "Choose a class first." nhỏ ngay dưới.
5. **B · Bộ lọc nâng cao**: "advance variation" tự xuống 2 dòng trong cột Proportions, trong khi các tên khác nằm 1 dòng. Có thể đổi thành "advance var." hoặc nới cột.
6. **B · Thẻ font (`en-similar`)**: ở lưới hẹp (khi mở panel chi tiết) tên "Libre Baskerville Regul…" bị cắt, phần huy hiệu TTF/VIE sát tên. Chấp nhận được, nhưng nên có tooltip tên đầy đủ (kiểm xem có chưa).
7. **A · Onboarding (`en-onb-02`, `en-onb-05`)**: còn nguyên tiếng Việt ("Quét xong", "Tiếp tục", "Quay lại", "Kích hoạt font cho app thiết kế", "Để sau", "Bật kích hoạt font", cảnh báo RightFont). Trộn lẫn với "Step 3/6 · back · next" và chú giải tiếng Anh. Việc của 01 (onboarding-polish), chưa nằm trong catalog en.
8. **C · Chủ đề tối (`en-*-dark`)**: sidebar vẫn nền trắng, chữ mục mờ, trong khi vùng chính tối. Có thể do QA chỉ đặt `data-theme=dark` bằng JS mà phần sidebar dùng biến khác; cần mở app thật, đặt Dark trong Cài đặt rồi so sánh. Nếu thật thì là lỗi theme, không liên quan ngôn ngữ.
9. **C · Ảnh Settings**: toast "Cleared all filters · Undo" của bước trước còn nằm trên ảnh (`en-settings`, `en-auto-activation-*`). Là hiện tượng của QA (không đợi toast tắt), không phải lỗi app.
10. **B · Settings → General**: nút radio đã chọn hiển thị là chấm nhỏ "•" không có vòng đầy như radio thường, khó nhận ra đang chọn English hay Tiếng Việt.

Đã ổn trong ảnh tiếng Anh: Library, Anatomy (số dạng 0.528 em), Similar, thanh lọc và popover bộ lọc (Remove group / Clear all), danh sách ngôn ngữ ("Vietnamese Tiếng Việt").
