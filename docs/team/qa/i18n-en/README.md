# QA native — giao diện tiếng Anh (milestone 6)

Chạy: `QA_LANG=en bash UX/src-tauri/tests/qa/run.sh` trên bản debug (dữ liệu tạm, không chạm thư viện thật).
Kết quả trên dev/mvp @8e7670c0: toàn bộ bước OK (exit 0). Ảnh `en-*.png` là ảnh chụp trong lượt đó; chủ đề sáng/tối theo từng bước (hậu tố -light/-dark).

Điều chỉnh QA để chạy được cả hai ngôn ngữ:
- `run.sh`: với `QA_LANG=en`, các bộ chọn `aria-label` tiếng Việt và nút "Xem family" được đổi sang chuỗi tiếng Anh tương ứng của app.
- `main_flows.js`: các regex phụ thuộc chữ (đủ|full, chưa chọn|not selected, Xoá tất cả|Clear all, Hoàn tác|Undo, Bật font tự động|Auto-activation) và dấu thập phân (0,528 / 0.528) khớp cả hai ngôn ngữ.
- `run.sh` in kết quả không còn sập khi một bước thiếu `detail`.
