# Desk: thanh cuộn ngang ở cửa sổ hẹp

**Nguyên nhân (đo ở 1000px, Desk 752px):** lưới Desk cố định 240 + 300 nên cột trang chỉ còn 212px; bảng số đo của DetailStrip (`white-space: nowrap`, ~320px) tràn và khiến cả `fa-shell-sheet` cuộn ngang.

**Sửa:**
- `.fa-desk` thành container `desk`; khi Desk ≤ 860px, hai cột trái co về `minmax(140,190) / minmax(190,250)` (cột trang 212 → 312px).
- Bảng số đo bọc trong `.fa-dstrip-tablewrap` (`overflow-x: auto`): nếu vẫn thiếu chỗ thì chỉ bảng cuộn, không phải cả trang.
- Lưới thẻ DetailStrip dùng `minmax(0, 1fr)` theo container query thay vì media query của viewport.

**Kiểm:** mock ở 1000×700: `sheet 752/752` (không còn tràn); vitest `components/desk` 72 pass; tsc sạch. Chưa kiểm trên bản native.
