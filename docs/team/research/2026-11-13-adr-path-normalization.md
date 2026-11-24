# ADR (đề xuất): chuẩn hoá đường dẫn lưu trong thư viện — chuẩn bị Windows

Người viết: 02 · 2026-11-13 · Trạng thái: ĐỀ XUẤT, chưa code; ưu tiên thấp (W1). Quyết sớm vì đụng schema và gói thư viện (B7).

## Bối cảnh
`file_locations.path TEXT NOT NULL UNIQUE` và `u.sources.path` lưu đường dẫn tuyệt đối đúng như hệ điều hành trả về. Code so sánh bằng chuỗi (`=`, `LIKE`, `substr`, `starts_with`): `remove_source`, quét (`known` map), watcher, gói B7 (đổi tiền tố), Duplicates. Trên macOS ổn (APFS thường không phân biệt hoa/thường nhưng ta lưu đúng dạng người dùng thêm). Trên Windows có ba vấn đề: dấu `\` so với `/`; `canonicalize` trả `\\?\C:\…` (tiền tố verbatim) làm lệch so sánh; và NTFS không phân biệt hoa/thường nên `C:\Fonts\A.ttf` = `c:\fonts\a.ttf` nhưng UNIQUE coi là hai dòng.

## Phương án
A. Giữ nguyên đường dẫn gốc của hệ điều hành, so sánh bằng hàm nhận biết nền tảng ở mọi chỗ (nhiều chỗ so sánh trong SQL không gọi được hàm Rust).
B. **Lưu một dạng chuẩn duy nhất trong DB** (khuyến nghị): UTF-8, dấu `/`, bỏ `\\?\`, ổ đĩa viết hoa (`C:/Fonts/A.ttf`), UNC là `//server/share/…`; giữ nguyên hoa/thường như người dùng thấy để hiển thị. Thêm cột `path_key` = dạng chuẩn đã hạ chữ thường (chỉ Windows; macOS dùng NFC + giữ hoa/thường) làm khoá UNIQUE và khoá so sánh; `path` vẫn là dạng hiển thị/mở file (chuyển ngược `/` → `\` khi gọi API Windows).
C. Chỉ chuẩn hoá khi nhập/ghi, không đổi schema (không giải quyết UNIQUE hoa/thường).

## Khuyến nghị: B, làm trước khi có bất kỳ dữ liệu Windows nào
- Hàm duy nhất `canonical_db_path(&Path) -> String` + `db_path_key(&str) -> String` trong `fontatlas-store`; mọi INSERT/so sánh đi qua đó. macOS: không đổi giá trị đang lưu (dạng chuẩn = dạng hiện tại), nên migration chỉ thêm cột `path_key` và điền = `path` (NFC), không viết lại đường dẫn nào.
- Gói B7: `source_roots` và `remap` làm việc trên dạng chuẩn `/`, nhờ đó gói xuất từ macOS không dùng được trên Windows chỉ vì tiền tố, nhưng đổi tiền tố là đủ để dùng.
- `LIKE` prefix: dùng `path_key >= ?1 AND path_key < ?1 || x'FFFF'` thay vì `LIKE` (không phụ thuộc ký tự đặc biệt `%`/`_` trong tên thư mục).
- Quyết định mở: macOS APFS có thể được tạo phân biệt hoa/thường; giữ so sánh phân biệt hoa/thường trên macOS (hành vi hiện tại), chỉ Windows hạ chữ thường.
- Chi phí: một migration library (thêm cột + chỉ mục UNIQUE trên `path_key`), sửa ~10 chỗ SQL dùng `path`; cần test với tên chứa `%`, `_`, ký tự không phải ASCII, và đường dẫn dài >260 ký tự (Windows `\\?\` bắt buộc để gọi API, nhưng KHÔNG lưu).
