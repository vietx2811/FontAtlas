# Changelog

Mọi thay đổi đáng chú ý của Font Atlas được ghi ở đây.
Định dạng theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/). Số phiên bản theo [Semantic Versioning](https://semver.org/lang/vi/).
Thuật ngữ typo giữ nguyên tiếng Anh, giống trong app.

## [0.2.0] — 2026-11-09

Bản phát hành đầu tiên. App được dựng lại toàn bộ trên engine phân tích font viết bằng Rust và chạy hoàn toàn trên máy: không tải font hay số đo của bạn lên đâu cả.
Bản này dành cho macOS Apple Silicon.

### Thêm mới

**Thư viện font**
- Thêm nhiều thư mục nguồn, quét lại theo thay đổi, nhận ra file bị đổi chỗ. Font còn nằm trên iCloud (chưa tải về) được đếm riêng.
- Lưới font cuộn mượt với thư viện lớn. Mỗi ô hiện tên font bằng chính font đó; có chế độ danh sách.
- Tìm theo tên (gõ không dấu cũng được, ví dụ "d" tìm ra "đ"); phím ⌘K để tìm nhanh.
- Bộ lọc: Vietnamese, license, nguồn, yêu thích, class. Mỗi bộ lọc đang bật có chip để gỡ.
- Yêu thích, Gần đây, Đang bật, Dự án, Thẻ. Chọn nhiều font cùng lúc, menu chuột phải.
- **Trùng lặp**: gom các file giống hệt nhau, các phiên bản và các định dạng của cùng một font. Mỗi bản có nút mở đúng vị trí trong Finder. App chỉ báo, không bao giờ tự xoá.
- Bật/tắt font cho hệ thống (activation): theo phiên làm việc hoặc giữ lâu dài, báo xung đột khi trùng tên, tự dọn khi thoát app.

**Chi tiết font**
- Specimen dạng waterfall, tab Vietnamese với lưới 134 ký tự và cảnh báo dấu bị tràn.
- Tab Anatomy: số đo đo thật trên outline (x-height, cap height, contrast, stress, aperture, slant, serif, width…), có biểu tượng cho từng trục.
- Đường guide Hxg (cap height, x-height, baseline) khớp đúng nét chữ thật. Nếu số đo của bộ phân tích lệch với hình hiển thị, app ghi chú cho biết.
- **Classification**: class, subclass và độ tin cậy. Có thể tự sửa class, và class đã sửa được dùng cho cả bộ lọc. Display là thẻ công dụng (usage), không phải class.
- **Similar**: tìm font giống theo family, kèm giải thích "giống ở / khác ở". Có thể bật thêm phân tích hình ảnh (model chạy trên máy, tải một lần, tạm dừng hoặc tiếp tục được).
- **Pairing**: gợi ý ghép cặp font theo vai trò, lưu các cặp đã chọn.

**Compare** (so sánh 2–8 font)
- Bảng so sánh specimen, số đo, class và metadata. Font đầu là mốc, mỗi số đo có Δ so với mốc.
- Cột property đứng yên khi cuộn ngang. Pin font để luôn đứng đầu bảng.
- Nút **+** thêm font từ thư viện, nút × bỏ font. Mục **Đang so sánh · N** trên sidebar để quay lại phiên đang so sánh.
- Nhóm property thu gọn hoặc mở bằng −/+; ẩn hàng không cần và thêm lại khi muốn.
- Sửa chữ mẫu ngay trong ô specimen; sửa một cột thì mọi cột đổi theo.
- Ký tự font không có hiện màu xám, kèm dấu "!" đỏ. App đọc thẳng bảng ký tự (cmap) của file font nên không báo nhầm.
- Hàng **Vietnamese** đánh dấu các ký tự có dấu chồng (stacked diacritics, ví dụ ấ, ở, ặ) và báo khi dấu va chạm hay tràn.
- **Overlay** chồng nét các font lên nhau để so, bật/tắt cho từng font (tối đa 4).
- Thanh đo tô từ 0 đến giá trị, cùng thang cho các font đang so. Percentile so với thư viện tham chiếu nằm trong tooltip.
- Chế độ Same size hoặc Match x-height.
- Xuất **PNG** đúng như bảng đang hiện: thứ tự pin, hàng đã ẩn, overlay, đơn vị. Có cả bản sáng và tối.

**Giao diện**
- Onboarding 7 bước có minh hoạ, giải thích license, model và activation trước khi bắt đầu.
- Settings dạng tấm phủ trên thư viện: chung, nguồn, phân tích hình ảnh, activation, license, dữ liệu, phím tắt.
- Mọi thuật ngữ typo đều có tooltip giải thích, kèm hình minh hoạ.
- Cảnh báo và chi tiết phụ hiện bằng dấu "!" đỏ ở góc; rê chuột hoặc dùng bàn phím để xem nội dung.
- Dark mode: vùng chữ mẫu nền đen chữ trắng. Giao diện phối các tông đen, xám đậm và xám nhạt.
- Màn hình trống có hướng dẫn cho từng khu vực. Nguồn cần cấp quyền truy cập thư mục được đánh dấu riêng.

**Cài đặt**
- DMG kèm script **Cài Font Atlas.command**: double-click là cài và chạy được, chưa cần notarize.

### Ghi chú cài đặt
- Bản này chưa được Apple notarize (chỉ ký ad-hoc). Cách cài:
  1. Mở DMG, double-click **Cài Font Atlas.command**. Script chép app vào Applications, gỡ quarantine, ký ad-hoc lại rồi mở app; thư viện và cài đặt cũ được giữ nguyên.
  2. Nếu macOS chặn script ở lần đầu, vào System Settings → Privacy & Security, bấm **Open Anyway**, rồi double-click lại. Trên macOS 14 trở về trước, có thể chuột phải vào script rồi chọn **Open**.
  3. Cài bằng Terminal (thay cho script): kéo app vào Applications, rồi chạy `xattr -dr com.apple.quarantine "/Applications/Font Atlas.app"`.
- Font Atlas là công cụ: app không chặn việc xem hay phân tích font vì lý do license, nhưng hiển thị loại license để bạn tự chịu trách nhiệm khi dùng.

## 0.1.0

Bản prototype nội bộ, không phát hành.
