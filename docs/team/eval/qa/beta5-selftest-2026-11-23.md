# Tự kiểm beta.5 — khay tác vụ (07 QA/DevOps, 2026-11-23)

Mã: `ffe2cfc0` (origin/dev/mvp = beta.5 + test/perf của 02). Chỉ bản **debug**, `FONTATLAS_DATA_DIR` riêng trên ổ dữ liệu, font thử là fixture của repo (đổi tên bằng `patch_font_names.py`). Không đụng `~/Library/Fonts`, thư mục font/cloud thật, không mở release/DMG. Kịch bản: `beta5-selftest/` (`run09.sh` + `qa09.js`; mục 6 dùng harness `cloud-reload/` với `lib8-nosub.js`).

| # | Mục | Kết quả |
|---|---|---|
| 1 | Bật / tắt / chuyển chỗ | **PASS** |
| 2 | Thêm font bằng kéo thả (50 file) | **PASS** |
| 3 | Tải từ cloud (mô phỏng CloudStorage) | **PASS một phần**: chưa xác nhận được tải thật |
| 4 | Huỷ giữa chừng (quét, font trùng, chép vào kho) | **PASS** |
| 5 | Sidebar thu gọn + giao diện tối, ≥3 việc | **PASS** |
| 6 | Số đo đối chiếu beta.4 | **PASS** |

## 1. Bật / tắt / chuyển chỗ
6 font fixture, scope `session`. Khay hiện "Bật font" (Đang đếm… → 1/6 → 5/6 → Xong), "Tắt font", "Chuyển đường dẫn font". ✓ tự biến mất sau **8,0 s** (8000 / 8032 / 8032 ms). Kiểm trong hệ thống bằng CoreText (`ctlist.swift`) và `system_profiler`: sau bật thấy đủ 6 font; sau tắt 0; sau chuyển chỗ (act2 → act) CoreText đăng ký lại 6 font ở đường dẫn mới, `changed: 6`.

## 2. Kéo thả
- 50 file (`import_paths_into`): tiến độ 0/50 → 50/50 → Xong; tray không lùi (0 bước lùi); 54 sự kiện tiến độ đơn điệu trong từng pha (adding_sources → scanning → adding). Case có đủ 50 face. Khay hiện ngay `0/50` thay vì "Đang đếm…" vì tổng đã biết trước.
- Thả cả thư mục (50 file trong thư mục con): có "Đang đếm…", không lùi.

## 3. Tải từ cloud (mô phỏng)
40 stub `.TÊN.ttf.icloud` dưới `…/Library/CloudStorage/cloud` chuyển dần thành file thật (ghi `.part` rồi đổi tên, 1,2 s/file, 48 s). Khay chỉ hiện "Quét «cloud»". Quét 2 lượt: t=30 s (MAX_WAIT 30 s khi luồng còn ghi, analyzed 25) và t=56 s sau khi yên 8 s (analyzed 15), nên **sau khi yên chỉ 1 lượt**. Dòng "Tải từ cloud «cloud»" có trong khay (kiểu cloudDownload, hiện Xong) khi gọi `source_download_cloud`, nhưng 14/14 lỗi `cloud_download_failed` vì stub không phải file dataless thật của File Provider: **không mô phỏng được tải thật**, cần máy có OneDrive/iCloud thật.
Ghi chú: khi không còn gì để tải mà gọi tải, khay vẫn nháy "Tải từ cloud — Xong" (0 việc).

## 4. Huỷ giữa chừng
- Quét: huỷ ở 256/1.150, không hỏi lại (đúng thiết kế), dừng hẳn, giữ 274 font đã quét; quét lại lên đủ 1.150.
- Xử lý font trùng (4.000 bản trùng, 10 nhóm): có hộp hỏi lại; "Giữ chạy" chạy tiếp; "Huỷ việc này" dừng ở 2.519/3.990, còn 4 nhóm nguyên; **Undo hoàn lại 2.519, về đủ 10 nhóm**.
- Chép vào kho Organized: có hộp hỏi lại; huỷ sau 189 file; trên đĩa 189 `.ttf` đúng bằng nguồn, 0 file `.part/.tmp`, checksum nguồn nguyên vẹn.
- Chưa thử: "sắp xếp lại kho" (resort) khi áp dụng; chỉ chép vào kho.
- Lưới trắng: 1 mẫu 53 ms tại đúng lúc huỷ quét (256 font); các lượt khác 0.

## 5. Sidebar thu gọn + tối (4 quét cùng lúc)
Nút khay nhãn "4 tác vụ nền", huy hiệu 4; sau thu gọn nút vẫn có, popover 288×144 nằm gọn trong cửa sổ; giao diện tối hiển thị đúng (ảnh 5a–5e).

## 6. Số đo đối chiếu (1.512 font + nguồn cloud thử 61 file, nguồn local)
| | beta.4 (0753e378) | beta.5 (ffe2cfc0) |
|---|---|---|
| Sidebar mở lần đầu ("All") | 1,16–1,37 s | **1,00 / 1,17 / 1,15 s** |
| IPC p95 khi quét (list_sources / query_faces) | C 145 / 145 ms | C 59 / 37 ms (lượt 2); lượt 1 có 1 cú 2,3 s không tái hiện |
| D ghi đè cùng hash: query_faces | 3 mỗi lượt (d2a448ca) → 0 (0753e378) | **0** (4 lượt quét, không reload) |
| C 30 file/4 s: lượt quét / query_faces | 9 / 27 | 9 / 33–39 |
| Lưới trắng | 0 | 0 |

Ghi chú kỹ thuật: `scan_subscribe` chỉ có một người nghe; gọi lần hai (như harness cũ) làm giao diện mất sự kiện quét (lưới trống, số đếm đứng). Harness mới suy lượt quét từ `list_tasks`.
