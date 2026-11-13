# G7b — Bản sao đổi tên cho font VF trùng tên với static (thiết kế, chưa code)

Người viết: 02 · 2026-11-13 · Trả lời câu hỏi (a)(b)(c) của 06. Quyết định chủ app (qua Lead): chỉ tạo bản sao KHI VF trùng tên với bản static; file gốc không đụng; bản sao vào Organized Library nếu có.

## Trả lời
- **(a) Mục đích:** có, dùng ĐỒNG THỜI VF và static của cùng họ mà không bị CoreText trả nhầm VF khi tra theo PostScript name (đã đo ở G7). Không phải để tài liệu cũ tự thấy bản sao (đúng như 06 nêu).
- **(b) Đổi gì:** đổi cả PostScript name VÀ họ, cùng hậu tố hiển thị " VF" đã dùng ở `faceLabel`: nameID 1 + 16 (họ), 4 (full name), 6 (PostScript), 3 (unique ID), và các PostScript name của named instance trong `fvar` (nameID ≥ 256 mà instance tham chiếu). Chỉ đổi PS name thì Font Book/app vẫn gộp hai họ cùng tên. Giữ nguyên nameID 13/14 (giấy phép) và thêm nameID 10: "Bản sao đổi tên do Font Atlas tạo từ <PS gốc>".
- **(c) Không có Organized Library:** vẫn tạo được, vào thư mục riêng của app `<data>/derived/<hash gốc>-VF.<ext>`; KHÔNG bao giờ đặt cạnh file gốc và KHÔNG đặt trong thư mục nguồn (nếu không B8 sẽ quét ra như font thường mất liên kết với gốc). Có Organized Library thì vào `…/Derived/` của nó.

## Điểm vào (đề xuất)
Lựa chọn **"Giữ cả hai"** trong hộp xác nhận xung đột VF/static của 06 (hỏi một lần + ô "nhớ") và nút trong tooltip dấu "!" của `ClashNote`. Không bao giờ tự tạo ngầm.

## Dữ liệu
- Bảng mới ở **user.db** `derived_fonts(hash PK, derived_from_hash, face_index, suffix, created_at)` — ở user.db để sống sót qua `rebuild_library` và nhập gói (B7). Không thêm cột vào `files`.
- Cần một nơi cho `file_locations.source_id`: đề xuất cho phép `sources.kind = 'derived'` (migration user.db) — nguồn hệ thống ẩn, không quét, không theo dõi (B8), không hiện ở danh sách nguồn; Organized Library thì chính nó là nguồn.
- Broker (`ensure`) và Duplicates loại hash có trong `derived_fonts` khỏi tập ứng viên trừ khi tên yêu cầu đúng bằng tên mới; không tính vào số font của người dùng; xoá font gốc thì hỏi có xoá bản sao.
- Idempotent: tên file theo hash gốc + hậu tố; đã có thì dùng lại.

## Kỹ thuật
- Viết lại bảng `name` + checksum/`head.checkSumAdjustment` tự làm (~150 dòng, không thêm crate `write-fonts`, chưa có trong Cargo.lock); kiểm lại bằng `read-fonts`/`skrifa` (tên mới, các bảng khác giữ nguyên byte, glyph/axes không đổi) và bằng `name_clashes()` (hết trùng).
- Từ chối: `fsType` hạn chế (bit "Restricted License"), TTC/WOFF/WOFF2 (v1; VF phổ biến là TTF/OTF), font không trùng thật.
- Giấy phép: OFL Reserved Font Name cấm dùng tên gốc cho bản sửa — đổi tên chính là cách tuân thủ; vẫn ghi chú ở UI theo "tool not gatekeeper", không chặn.

## Đã chốt (Lead 2026-11-13) + hợp đồng với 06
1. Điểm vào: lựa chọn "Giữ cả hai" (mã `both`) của hộp xung đột G7 + nút ở ClashNote; không bao giờ tạo ngầm. Chỉ làm sau khi chủ app trả lời các câu hỏi B7.
2. Migration `sources.kind = 'derived'` thêm vào CUỐI `USER_MIGRATIONS` (B5 cũng thêm migration; Lead xếp thứ tự khi merge).
3. Hậu tố cố định, người dùng không đặt. Quy tắc tên (02 chốt): họ `X` → `X VF`; full name `X VF <Kiểu>`; **PostScript name chèn `VF` vào phần họ**: `Oswald-Regular` → `OswaldVF-Regular` (một dấu gạch, đúng quy ước PS; không có dấu cách; không trùng PS của bản gốc lẫn của static); PS không có gạch thì thêm `VF` ở cuối; áp dụng giống cho PS name của named instance trong `fvar`; unique ID = gốc + `;FontAtlas-VF`.
4. **fsType "Restricted License" KHÔNG chặn** (tool, not gatekeeper): vẫn tạo bản sao, trả cờ `license_restricted` để UI hiện NoteMark "giấy phép của font hạn chế, bạn chịu trách nhiệm". Chỉ từ chối vì lý do kỹ thuật: TTC/WOFF/WOFF2 (v1), font không phải VF, font không trùng tên thật, font ngoài thư viện (không có hash/không đọc được — 06: "Giữ cả hai" lùi về hành vi G7 hiện tại).

### Hàm cho 06 (store/app, đồng bộ)
`derive_renamed_copy(hash) -> Result<DerivedCopy { hash, path, created, license_restricted }, DeriveError>`
- Idempotent: cùng hash gốc + cùng hậu tố trả lại bản đã có (`created = false`), không tạo trùng.
- Ghi file xong mới trả: viết vào file tạm cùng thư mục, `sync_all`, đổi tên nguyên tử; rồi nạp vào thư viện (nguồn `derived` hoặc Organized Library) và ghi `derived_fonts`.
- `DeriveError`: `NotInLibrary`, `NotVariable`, `NoClash`, `UnsupportedContainer(ttc|woff|woff2)`, `NotDownloaded` (06 gọi `ensure_local` trước; nếu vẫn dataless thì trả lỗi này), `DiskFull`, `Io(kind)`.
- `is_derived(hash) -> bool` (và `derived_from(hash) -> Option<hash>`) để 06 lọc ứng viên ở `ensure`/`pick` (loại trừ khi tên yêu cầu đúng tên mới).
