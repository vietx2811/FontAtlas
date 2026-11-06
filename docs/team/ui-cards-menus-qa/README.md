# QA native: tiêu đề bằng font, tooltip, đa chọn, menu ngữ cảnh

Bản debug, dữ liệu tạm, `bash UX/src-tauri/tests/qa/run.sh` (23/23 bước OK). Ảnh chụp WKWebView, theme sáng (trừ `tooltip-dark`).
Script chốt animation (`document.getAnimations().finish()`) trước mỗi ảnh vì cửa sổ QA không ở trước nên WKWebView không chạy animation.

- `cards-own-font.png`: tên thẻ vẽ bằng font của thẻ (Bebas, Dancing Script, Playfair, Source Code…)
- `tooltip-badge.png`, `tooltip-settings.png`, `tooltip-dark.png`: tooltip
- `selection-bar.png`, `context-menu.png`: ⌘-click 2 thẻ → "2 đã chọn" + menu nhiều font
- `context-submenu.png`, `new-project-prompt.png`, `project-created.png`: Thêm vào dự án ▸ Dự án mới… → dự án có 2 font (kiểm bằng `list_collections`)
- `context-menu-single.png`: menu 1 font, tiêu đề panel bằng font của thẻ

Chưa có ảnh menu ở theme tối (bước chụp dark không bắt được menu; hành vi đã có test đơn vị).
