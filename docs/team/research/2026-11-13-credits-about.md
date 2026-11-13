# D16 — Màn Giới thiệu và ghi công (Credits/About)

## Cách các app khác bố trí
Tra cứu ngắn (không phải khảo sát từng app; không tìm được tư liệu về About của Sketch/Figma/Linear/Raycast/Obsidian nên không khẳng định gì về chúng):
- Quy ước macOS: hộp About có tên, phiên bản, bản quyền; "credits" ghi công người và mã không phải của mình; nên không modal.
- Mẫu chung ở các app có ghi nguồn mở (CodeEdit `AboutWindow`, Winstrix, Proxyman): About gọn, kèm một nút/mục "Acknowledgements" mở danh sách từng thành phần + giấy phép; toàn văn giấy phép đóng gói trong app.
- Quyết định cho Font Atlas: không làm cửa sổ riêng (app đã có lớp phủ Cài đặt dùng chung phím/đóng/tiêu điểm). Thêm nhóm "Giới thiệu và ghi công" trong Cài đặt: câu mở đầu, bốn nhóm (Dữ liệu ngôn ngữ · Nhận diện hình dạng chữ · Lõi ứng dụng · Giao diện), mỗi mục = tên, tác giả, giấy phép, một dòng "dùng để làm gì". Mục Hyperglot đầy đủ nhất vì là nghĩa vụ giấy phép.

## Giấy phép đã đối chiếu
Đọc từ manifest trong `~/.cargo/registry` (crate ghim trong Cargo.toml) và `node_modules/*/package.json`:

| Thành phần | Giấy phép | Ghi chú |
|---|---|---|
| Hyperglot (dữ liệu) | Apache-2.0 | Bắt buộc giữ thông báo + nêu thay đổi: `Resources/NOTICE-hyperglot.md`, `LICENSE-hyperglot.txt` đã đóng gói; màn này hiện `credit.hyperglot` + dòng thay đổi |
| SigLIP 2 base-patch16-224 (trọng số, tải khi bật phân tích hình ảnh) | Apache-2.0 | Đã đối chiếu front matter model card gốc `google/siglip2-base-patch16-224` (`license: apache-2.0`, Lead kiểm) |
| candle 0.11 | MIT OR Apache-2.0 | |
| Tauri 2.9 · @tauri-apps/api | Apache-2.0 OR MIT | |
| read-fonts, skrifa, kurbo | MIT OR Apache-2.0 | |
| wuff | MIT | |
| tiny-skia | BSD-3-Clause | Điều 2 BSD: bản nhị phân phải kèm thông báo bản quyền |
| rusqlite · libsqlite3-sys (SQLite bundled) | MIT · SQLite là public domain | |
| blake3 · png · notify | CC0/Apache-2.0 · MIT/Apache-2.0 · CC0-1.0 | |
| React, react-dom, react-window, zustand, framer-motion, recharts, clsx, tailwind-merge | MIT | |
| lucide-react | ISC | |

Hầu hết giấy phép trên (MIT, BSD, ISC, Apache) yêu cầu giữ thông báo bản quyền/giấy phép khi phân phối, nên màn này liệt kê tất cả; chỉ Hyperglot đang có toàn văn đóng gói.

## Chưa làm (đề xuất cho Lead)
- Đóng gói một `THIRD-PARTY-NOTICES` sinh tự động (ví dụ `cargo about` + `license-checker`) vào `Resources/` để có toàn văn cho mọi phụ thuộc, kể cả phụ thuộc bắc cầu (danh sách ở đây chỉ gồm phụ thuộc trực tiếp).
- Thành phần phụ thuộc bắc cầu và mọi font/dữ liệu khác nếu sau này đóng gói thêm.
