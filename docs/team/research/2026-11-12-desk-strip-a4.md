# Nghiên cứu A4: dải chi tiết Pairing Desk (0.4.0)

Ngày: 2026-11-12. Người soạn: 05 Frontend. Đã có spec (docs/design/font-desk/06_v3_pairing_compare.md §2.1 và prototype pairing-desk-v3.html); ghi chú này chỉ chốt phần còn thiếu. **[I]** = nhớ từ cách dùng các app, chưa mở lại nguồn.

1. **Việc còn thiếu so với spec 06**: thẻ Đang dùng/Đang thử chưa có vòng VI x/186, chưa có meta (Foundry, Version, Glyphs, tag Variable với trục min–max), bảng Số đo chưa có hàng Glyphs/Δ.
2. **Font Book / FontBase / Typeface** [I]: panel thông tin hiện số glyph và nhãn Variable; Typeface và RightFont liệt kê trục Variable với dải giá trị (vd wght 100–900). Không app nào đưa độ phủ tiếng Việt thành con số; đây là điểm khác của FontAtlas nên cần nổi bật.
3. **Vòng độ phủ**: kiểu vòng tiến độ nhỏ (14 px) kèm số `x/186`. Không dựa riêng vào màu: luôn có số và tooltip ("Thiếu N ký tự VI"); xanh khi đủ, hổ phách khi >60%, đỏ khi thấp hơn (theo prototype).
4. **Tag Variable**: chip "Variable" (thuật ngữ giữ tiếng Anh, có TermTip) kèm tối đa 2 trục `wght 100–900 · wdth 75–125`, phần còn lại "+N trục nữa" và đủ danh sách trong tooltip. Font static không có chip.
5. **Glyphs**: số có phân cách nghìn theo locale + thuật ngữ Glyphs (TermTip). Ở thẻ Đang thử kèm Δ số tuyệt đối so với Đang dùng; hàng Glyphs trong bảng Số đo có Δ phần trăm (cùng quy ước ±0 / dấu +/− với các số đo khác). Ít glyph hơn không phải "xấu hơn" nên Δ chỉ báo chiều, không tô đỏ/xanh theo giá trị tốt/xấu ở thẻ (chỉ lên/xuống).
6. **Dữ liệu có sẵn, không cần backend mới**: `file.numGlyphs`, `vietnamese.total/missing`, `metadata.variableAxes`, `metadata.manufacturer`, `metadata.version` trong `FaceDetailDto`; `buildCompare` mang thêm 4 trường (đều tuỳ chọn, thiếu thì ẩn).
7. **Chưa làm** (ngoài A4 theo lời Lead): chữ thật của vai trong thẻ và Δ Glyphs ở màn Compare Font/Combo.
