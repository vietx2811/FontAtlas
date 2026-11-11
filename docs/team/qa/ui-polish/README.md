# UI polish (fix/ui-polish-en) — trước/sau

Trước: `docs/team/qa/i18n-en/en-*.png`. Sau: `after-*.png` ở đây (QA native debug, QA_LANG=en và vi; cả hai lượt 35 bước OK).

| Mục | Kết quả |
|---|---|
| Auto-activation: cột trạng thái cùng cỡ chữ, hàng đều, "!" cạnh nhãn | Xong. Cột trạng thái cùng cỡ chữ thân, "!" cùng dòng với nhãn, độ rộng cột cố định. "Open InCopy once, then come back" vẫn xuống 2 dòng nhưng cùng cỡ chữ. |
| Bớt lặp "Auto-activation" | Nhãn công tắc đổi thành "Turn on" / "Bật" (aria-label đầy đủ "Turn on auto-activation" / "Bật font tự động"). Cụm "Auto-activation" cuối đoạn giới thiệu là từ TermTip nên giữ. Câu vi bị ngắt "Auto-\nactivation" ở cuối dòng (nhỏ). |
| Nút Install mờ chỉ khi thật sự bị khoá, có lý do | Nút khoá (dev build / app lạ / đang bận) có Tooltip nêu lý do; nút không khoá giữ nguyên. Trên ảnh QA các nút Install vẫn mờ vì QA chạy dev build, đúng với thực tế. |
| Nhãn "Include hybrid fonts" / "Confident labels only" | Xong, cùng cỡ chữ với nhãn khác (đã xem ảnh en và vi). |
| "advance variation" | Tên không còn bị ngắt giữa từ; thẻ "measured" và số đếm xuống dòng dưới, hiện số "13" nằm một mình ở bên phải. Chưa lý tưởng, nhưng không còn tên 2 dòng. |
| Radio đã chọn | Xong, chấm lớn + vòng accent theo token (ảnh `after-en-settings-dark-real`). |
| C · Sidebar tối nền trắng | Không phải lỗi app, mà do ảnh QA: `qa-snap` chỉ chụp webview (UX/src-tauri/src/lib.rs), không chụp lớp kính native (vibrancy) nằm sau webview trong suốt, nên vùng sidebar hiện nền trắng. Đặt theme qua radio trong Cài đặt (đường thật, gọi `applyTheme` → `setTheme` của cửa sổ native) cho cùng kết quả ảnh. **Chưa xác nhận bằng ảnh chụp cửa sổ thật**: cần xem trực tiếp trên máy, nếu sidebar thật vẫn sáng thì mở lại. |
