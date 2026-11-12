# Nghiên cứu G1: kéo font vào Sidebar (0.4.0)

Ngày: 2026-11-12. Người soạn: 05 Frontend. **[I]** = nhớ từ cách dùng các app, chưa mở lại nguồn trong phiên này; chủ app hoặc reviewer kiểm lại.

1. **Finder** [I]: kéo nhiều mục thì con trỏ mang bản mờ kèm **huy hiệu số**; đích (thư mục ở thanh bên) **sáng lên** khi rê qua; thả xong có Hoàn tác (⌘Z). Kéo mục chưa chọn thì chỉ mang mục đó; kéo mục đã nằm trong vùng chọn thì mang cả vùng chọn.
2. **RightFont** [I]: kéo font từ danh sách vào Set/Library ở thanh bên là thao tác chính; đích sáng khi rê; có menu chuột phải "Add to Set" làm lối thay thế.
3. **Typeface** [I]: kéo font vào Library/Collection trên thanh bên; cũng có "Add to Library" ở menu. Đích không hợp lệ không sáng.
4. **FontBase** [I]: kéo thẻ font vào Collection ở thanh bên; số trong ngoặc của mục tăng ngay sau khi thả.
5. **Font Book** [I]: kéo font vào Collection ở thanh bên; không có hoàn tác riêng.
6. **Rút ra**: (a) vùng thả sáng lên và con trỏ báo số; (b) kéo thẻ trong vùng chọn = cả vùng chọn; (c) thả xong số trên mục đổi ngay; (d) luôn có lối bàn phím/menu tương đương; (e) đích không hợp lệ phải nhìn ra được (mờ, không sáng).
7. **Quyết định**: kéo bằng pointer events (không HTML5 DnD) để chạy ổn trong WebView Tauri (nơi kéo-thả file hệ điều hành chiếm sự kiện) và vẽ được con trỏ báo số. Đích hiện có: **Dự án (Collection)** ở thanh bên; dự án đang xem thì mờ ("font đã ở đây"). Thả chỉ thêm font chưa có; thông báo có **Hoàn tác** chỉ gỡ phần vừa thêm. Esc huỷ lượt kéo. Lối bàn phím: **⌥⌘P** ("Thêm vào…") mở danh sách dự án cho font đang chọn (cùng menu với nút ở thanh chọn và menu chuột phải).
8. **Chưa có trong 0.4.0 hiện tại**: thanh bên **chưa có mục Case hay Smart** để làm đích thả (Case chỉ mở từ Pairing Desk; chưa có Smart collection trong code). Khi các mục này có trong thanh bên, chỉ cần gắn `drop` cho NavItem tương ứng và thêm nhánh vào `dropFaces.ts`. Kéo **từ Pairing Desk** chưa làm: danh sách ứng viên có nút `+` (G2) thay thế; cần quyết định có muốn kéo từ Desk ra thanh bên không (Desk che thanh bên khi mở).
