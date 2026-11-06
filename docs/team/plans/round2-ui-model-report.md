# Round 2 — UI tải model (2026-11-06)

Base: `357ec1b` (đã merge từ dev/mvp, gồm downloader backend). Không sửa Rust/bindings.

- Cập nhật đồng thời done/total theo từng phase; reset vòng tiến độ khi total đổi. Khi đủ byte nhưng chưa Ready, hiện “Đang kiểm tra tệp…” và spinner, chưa cho Similar dùng hình ảnh.
- Ánh xạ các mã lỗi theo contract; HTTP 429 hiện giới hạn và Retry-After (giây hoặc thời điểm). HTTP 403 không bị nhận thành quyền ghi đĩa. Message kỹ thuật chỉ ở “Chi tiết” đóng sẵn. Cancelled là thông tin + “Tiếp tục tải”; lỗi khác “Thử lại”.
- Store và feedback dùng chung ở sidebar, Settings, Onboarding, Similar. Tốc độ MB/s theo hệ thập phân. Theo dõi tải vẫn hiện khi tắt phân tích hình ảnh.
- O6 “Tải ngay” giữ màn để thấy tiến độ/lỗi; có “Tiếp tục” để tải nền và tiếp tục onboarding. “Không dùng”/“Để sau” giữ nguyên trước khi tải.

## Kiểm tra

- `npx tsc --noEmit`: qua.
- Full Vitest: **49 file, 474/474 test qua**. Có mapping mã lỗi, 429/403, cancelled, chờ Ready, đổi mẫu số phase, spinner/chi tiết ở sidebar và O6.
- `npm run build`: qua. Cảnh báo Vite có sẵn về import động/tĩnh Tauri core; không làm hỏng build.
- Playwright Chromium, viewport **1000×700**, sáng/tối; đã mở và tự xem đủ 6 ảnh. Kiểm tra Chi tiết TLS mở/đóng đúng, không tràn ngang. Console không có error trên lượt chụp.

Ảnh trong `docs/design/screenshots/round2/`:

- `model-downloading-1000-light.png`, `model-downloading-1000-dark.png`
- `model-checking-1000-light.png`, `model-checking-1000-dark.png`
- `model-tls-1000-light.png`, `model-tls-1000-dark.png`

Dựng lại: `http://localhost:3012/?qa=model-<state>#settings=1&enable=1&model=<state>`, state là `downloading`, `checking`, `tls`; chọn “Phân tích hình ảnh”. Theme qua `document.documentElement.dataset.theme = 'light' | 'dark'`. Preset chỉ chạy dev; dữ liệu browser mock, không chứng minh native IPC hoặc tải mạng thật. Không tải model thật, kích hoạt font, hay đổi CA/mirror/quyền.
