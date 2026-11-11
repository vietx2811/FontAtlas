# i18n EN — việc còn lại và chuỗi nằm ngoài phạm vi nhánh `feat/i18n-en`

Cập nhật 2026-11-11. Nhánh `feat/i18n-en` (từ `origin/dev/mvp` 6979f4f) đã làm mốc 1–5 phần UI; mục này liệt kê những gì CHƯA làm hoặc thuộc chủ nhánh khác.

## 1. Chuỗi cứng còn lại trong UI (giao cho chủ nhánh khu đó)
Test chặn (`UX/i18n/__tests__/hardcoded.test.ts`) giữ baseline `hardcodedBaseline.json`: chỉ được giảm. Còn:

| File | Dòng còn chữ Việt | Ghi chú |
|---|---|---|
| `components/onboarding/Onboarding.tsx` | 66 | khu 01 (onboarding + splash). Cần ~60 khoá `onb.*` + `en/onboarding.ts`. O1 đã dùng `chooseLocale`. |
| `components/onboarding/Illustrations.tsx` | 14 | alt/aria của hình minh hoạ |
| `components/onboarding/ScanBlocks.tsx` | 1 | |
| `components/settings/Settings.tsx` | 1 | 1 dòng còn lại là chữ mẫu có `i18n-ok` chưa được bộ quét nhận, không phải chuỗi giao diện |
| `store/model.ts` | 2 | dương tính giả của bộ quét (chuỗi có dấu nháy trong regex) |

Không có chữ Việt trong: sidebar, faces, main, grid, detail, zero, boot, scan, App ErrorBoundary, model store. Khu `settings/AutoActivation`, dialog xung đột kích hoạt và `duplicates` đã dùng khoá sẵn (không có chuỗi cứng có dấu), nhưng bộ quét chỉ thấy chuỗi CÓ dấu — chuỗi tiếng Việt không dấu ("Trang sau", "Chung"…) phải duyệt bằng mắt.

## 2. Việc cần chủ nhánh Pairing Desk (05) làm
Các lệnh này vẫn truyền `lang: 'vi'` cứng → câu giải thích do Rust sinh luôn là tiếng Việt. Thay bằng `getLocale()` (xem `api/compare.ts`, `components/detail/tabs/PairTab.tsx`, `components/detail/useFaceDetail.ts` đã đổi):
- `components/desk/PairingDesk.tsx:63` (`pairCandidates … lang: 'vi'`)
- `components/desk/ComboCompare.tsx:44` (`caseCompare(caseId, [], 'vi')`)
- `components/desk/FontCompare.tsx:35`, `components/desk/DetailStrip.tsx:28` (`getFace(f, 'vi')`)
Rust (`fontatlas-index/src/explain.rs`) đã có `Lang::{Vi,En}` và mẫu câu hai ngôn ngữ; `UX/src-tauri/tests` kiểm hai ngôn ngữ.

## 3. Rust trả câu tiếng Việt ra UI
Quét `UX/src-tauri` (bỏ `tests/`, `target/`): 72 file / 433 dòng có chữ Việt trong chuỗi. Phần lớn là log, `assert!` trong test nội bộ, CLI eval, thông báo `#[error(...)]`. Đường ra UI:

| Nguồn | Cách ra UI | Xử lý trong nhánh này |
|---|---|---|
| `CmdError.message` (`src/error.rs`, `src/services/*.rs`, `StoreError`/`PreviewError`/`CoreError`/`ActivateError` qua `#[error]`) | notice/lỗi hiển thị `message` | `api/tauri.ts::localizeCmdMessage`: UI tiếng Anh thay `message` bằng câu theo `code` (`err.*`, 18 mã); UI tiếng Việt giữ câu gốc. Mã lạ → "Something went wrong (code)". Không đổi Rust. |
| Câu giải thích Pairing/Similar (`fontatlas-index/src/explain.rs`, `describe`) | trường `explanation` | Đã song ngữ theo `lang` — chỉ cần UI truyền đúng (mục 2). |
| Tên phân loại (`classify.rs`, `taxonomy`), `fontatlas-core/langdata.rs` | một phần là nhãn hiển thị | Giao diện dùng khoá `class.*`/`term.*` theo mã, không dùng chữ Rust. |
| `access.rs` ("macOS từ chối đọc thư mục", "không mở được thư mục…") | `CmdError`/`ScanEvent::Failed.error.message` | qua `localizeCmdMessage` (`permission_denied`, `timeout`). |

Đề xuất làm tiếp nếu muốn đủ "mã + tham số" ở Rust: thêm `detail` có cấu trúc (`{ "name": …, "max": … }`) cho các lỗi `invalid_arg` hay hiện ra UI, rồi thêm khoá `err.<code>.<field>` có tham số. Hiện hầu hết `invalid_arg` là lỗi lập trình (khoá setting, định dạng tham số), người dùng không gặp.

`DEFAULT_SAMPLE` ("Phở bò Hà Nội", `services/settings.rs`) là chữ mẫu, giữ nguyên.

## 4. Ngôn ngữ mặc định
`uiLang` đã có trong `SettingsDto`/`user.db` (mặc định backend "vi"). Vì backend không phân biệt "chưa chọn" với "chọn vi", giao diện lưu thêm cờ `localStorage['fa.lang']` khi người dùng chủ động chọn. Khởi động: cờ → `uiLang==='en'` → ngôn ngữ hệ thống (`navigator.languages[0]` bắt đầu bằng `vi` → vi, còn lại en). Hệ quả: người đã chọn "Tiếng Việt" trước bản này (nút cũ không có tác dụng) trên máy tiếng Anh sẽ thấy tiếng Anh cho tới khi chọn lại. Muốn chính xác hơn cần Rust trả `uiLang: null` khi chưa đặt (đổi DTO + bindings + test).

## 5. Việc kế tiếp (mốc 6)
- QA native: `QA_LANG=en bash UX/src-tauri/tests/qa/run.sh` chụp ảnh sáng/tối tiếng Anh. Các bước còn so chữ tiếng Việt sẽ báo FAIL ở lượt EN — chỉ dùng để lấy ảnh. Lượt mặc định vẫn là `vi`.
