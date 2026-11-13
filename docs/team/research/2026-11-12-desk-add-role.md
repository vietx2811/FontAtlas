# Nghiên cứu G3: "+ Role" trong combo (0.4.0)

Ngày: 2026-11-12. Người soạn: 05 Frontend. **[I]** = nhớ từ cách dùng các app, chưa mở lại nguồn.

1. **Figma (style/variant), Notion (thuộc tính), Typewolf-style type scale** [I]: thêm mục bằng nút "+" cuối danh sách, gõ tên tự do, có vài tên gợi sẵn; đổi tên tại chỗ (F2 hoặc bấm đôi); thứ tự kéo hoặc nút lên/xuống.
2. **Hệ thống thiết kế (Material type scale, Apple HIG text styles)** [I]: bộ vai có tên chuẩn (Display, Headline, Title, Body, Caption, Label…); người dùng thêm vai riêng (Tagline, Price…) khi dự án cần.
3. **Quy tắc rút ra**: (a) nút "+ Role" cuối cột vai; (b) gợi ý tên bằng chip bấm một lần, chỉ liệt kê tên chưa có; (c) lỗi tên (trống, trùng không phân biệt hoa thường, quá 40 ký tự, quá 30 vai) báo rõ và giữ ô nhập; (d) xoá vai không xoá font khỏi Case, có Hoàn tác; (e) thao tác chuột có lối bàn phím (nút ↑↓✎× tab được, F2 đổi tên, Enter/Esc).
4. **Quyết định**: tên người dùng gõ trùng một vai gợi sẵn (kể cả tên đã dịch: "Body") thì lưu khoá preset (`body`) để vi/en cùng đọc; còn lại lưu đúng chữ gõ. Gợi ý: 7 vai cũ + Label + Numerals. Vai tuỳ ý mặc định coi là vai nội dung cho tới khi có hồ sơ vai (G5, "người dùng chọn hồ sơ vai gần nhất").
5. **Combo chưa lưu vai nào** (roles = []): giao diện hiện bộ mặc định theo font đã có; lần thêm vai đầu tiên lưu luôn các vai đang hiển thị để thứ tự khớp.
6. **Chưa làm**: kéo-thả đổi thứ tự (dùng nút ↑↓); hồ sơ vai cho vai tuỳ ý (G5).
