# B7 Organized Library: nguyên mẫu "gói thư viện" (backend)

Người viết: 02 · 2026-11-13 · Nguyên mẫu trong `crates/fontatlas-store/src/pack.rs`, test `tests/pack.rs` (thư mục tạm, fixture OFL). Chờ spec v1 của 01; phần dưới là những gì đã chốt được ở tầng dữ liệu và những gì còn mở.

## Định dạng gói (v1)
Một thư mục: `manifest.json` + `library.db` + `user.db`.
- Hai DB là ảnh chụp bằng `VACUUM INTO` (nhất quán, không cần dừng app, không đụng DB đang dùng). **Cài đặt** nằm trong `user.db` (bảng `settings`), **embedding** trong `library.db`, nên không có file riêng.
- `manifest.json`: `format`, `app_version`, `library_schema`, `user_schema`, `created_at`, `counts` (files/faces/embeddings/sources), `source_roots` (gốc nguồn lúc xuất), `files[]` với kích thước + BLAKE3. Manifest ghi SAU CÙNG (qua file tạm rồi đổi tên): có manifest = gói trọn vẹn.
- File font KHÔNG nằm trong gói (hàng chục GB); chúng ở Organized Library hoặc thư mục nguồn.

## Quy tắc an toàn
1. Nhập chỉ vào thư mục dữ liệu CHƯA có `library.db`/`user.db`; không bao giờ ghi đè (`Conflict`).
2. Từ chối gói có `format`/schema mới hơn app hiểu (`NewerSchema`); gói cũ hơn thì `Store::open` migrate như thường.
3. Từ chối gói hỏng/thiếu (kích thước + BLAKE3 từng file) TRƯỚC khi ghi gì.
4. Nhập qua thư mục dàn tạm, đổi tên `user.db` trước, `library.db` sau: crash giữa chừng chỉ để user.db, library tự dựng rỗng và quét lại, không mất dữ liệu người dùng.
5. Sau khi cài lại máy đường dẫn đổi: `import_pack(pack, data_dir, remap)` đổi tiền tố cũ→mới cho `file_locations.path` và `sources.path`. Sau nhập nên quét lại các nguồn (đã kiểm: 2/2 `unchanged`, không phân tích lại).

## Chưa làm / cần quyết (hỏi 01, 05, 06)
- **Font đang bật lâu dài** trỏ đường dẫn cũ: phải cập nhật `file_targets` TRƯỚC rồi 06 gọi `relocate(hashes)` một lần theo lô (đăng ký mới trước, gỡ cũ sau — 06 đã đo khoảng trống). Mặc định chép giữ gốc, không di chuyển/xoá gốc trước.
- Sắp xếp `Class/Tên font/` và đổi class sau khi đã xếp: cần cột "vị trí chuẩn" khác đường dẫn thật; spec 01 quyết có tự di chuyển lại hay không.
- Font cloud chưa tải (dataless): chép cần tải trước; chưa tải thì giữ vị trí cũ, đánh dấu "chưa chép".
- Trùng lặp: chép theo hash (một bản), ghi vị trí còn lại; liên quan Duplicates đã resolve.
- Dung lượng ổ: kiểm trước khi chép (tổng size file + 10%), báo thiếu; chưa viết.
- Xuất/nhập qua lệnh IPC + UI (05) và onboarding: chưa có; chỉ có tầng store.
- Nén/đóng thành một file `.fontatlaspack`: chưa; thư mục đủ cho nguyên mẫu.
