# QA native: tiêu đề bằng font, tooltip, đa chọn, menu ngữ cảnh

Bản debug, dữ liệu tạm, `bash UX/src-tauri/tests/qa/run.sh` (23/23 bước OK). Ảnh chụp WKWebView, theme sáng (trừ `tooltip-dark`).
Script chốt animation (`document.getAnimations().finish()`) trước mỗi ảnh vì cửa sổ QA không ở trước nên WKWebView không chạy animation.

- `cards-own-font.png`: tên thẻ vẽ bằng font của thẻ (Bebas, Dancing Script, Playfair, Source Code…)
- `tooltip-badge.png`, `tooltip-settings.png`, `tooltip-dark.png`: tooltip
- `selection-bar.png`, `context-menu.png`: ⌘-click 2 thẻ → "2 đã chọn" + menu nhiều font
- `context-submenu.png`, `new-project-prompt.png`, `project-created.png`: Thêm vào dự án ▸ Dự án mới… → dự án có 2 font (kiểm bằng `list_collections`)
- `context-menu-single.png`: menu 1 font, tiêu đề panel bằng font của thẻ

Chưa có ảnh menu ở theme tối (bước chụp dark không bắt được menu; hành vi đã có test đơn vị).

## Lưới tràn dưới panel (đã sửa)
`selection-bar.png` của 699b309 cho thấy lưới 4 cột đè lên panel. Đo trong QA native (cửa sổ QA ở nền: `document.hidden=true`, `hasFocus=false`, 0 tick rAF/500 ms):

| mốc | trước (RO) | sau (đo đồng bộ) |
|---|---|---|
| chưa mở panel | scroller 248→1400, 4 cột (đúng) | giống |
| panel mở +300 ms và +1800 ms | grid thật 812 px (DOM đúng) nhưng `aria-colcount`=4 (cần 3), scroller.right 1400 > panel.left 1060 | 3 cột, scroller.right 1060 = panel.left |
| thanh chọn hiện | chỉ sửa được khi RO chạy muộn | 3 cột, vừa khung |

Nguyên nhân: ResizeObserver không được phát khi WebView không dựng khung; `useSize` giữ bề rộng cũ và `List` có `style.width` px cũ. DOM layout đã đúng, chỉ state JS cũ.
