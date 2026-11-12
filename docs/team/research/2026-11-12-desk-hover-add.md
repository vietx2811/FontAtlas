# Nghiên cứu G2: hover tên font trong Desk → nút "+" (0.4.0)

Ngày: 2026-11-12. Người soạn: 05 Frontend. Ghi chú ngắn trước khi code. **[I]** = nhớ từ cách dùng các app, chưa mở lại nguồn trong phiên này; cần chủ app hoặc người review kiểm lại.

1. **Finder / macOS (Quick Look, Add to…)** [I]: hành động phụ hiện khi rê chuột hoặc khi hàng được chọn, ẩn khi rảnh để danh sách không rối; hàng đã chọn luôn hiện nút để bàn phím không mất lối.
2. **RightFont** [I]: rê lên một font trong danh sách thì hiện nút nhỏ ở mép phải (thêm vào Library/Set); kéo thả là lối chính, nút hover là lối ngắn.
3. **Typeface** [I]: hàng font có menu "+" khi hover để cho vào Library; click cả hàng chỉ để xem, không thêm, tránh thêm nhầm.
4. **FontBase** [I]: rê lên thẻ font thì hiện biểu tượng sao/thêm vào Collection; mọi nút hover cũng có lối bàn phím (phím tắt chung).
5. **Quy tắc chung rút ra**: (a) nút chỉ hiện khi hover, khi hàng được chọn, hoặc khi focus bàn phím; (b) nút nằm ở mép phải, không che tên font; (c) có tooltip nói rõ đích ("Thêm vào vai Body"); (d) có phím tắt tương ứng; (e) không thêm khi click hàng (click chỉ chọn/thử).
6. **Quyết định cho FontAtlas**: nút "+" ở mép phải mỗi ứng viên trong Cuộn chọn, đích là **vai đang chọn** (đúng cái Enter và double-click đang làm). Hiện khi hover / hàng chọn / focus. Phím `+` (hoặc `=`) đặt ứng viên đang chọn, cùng Enter. Font đã đang dùng cho vai thì không hiện nút. Sau khi đặt có thông báo "Đã đặt …" như cũ; hoàn tác chưa có trong Desk (ngoài phạm vi G2, báo Lead nếu muốn).
