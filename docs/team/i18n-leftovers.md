# i18n EN — việc còn lại

Dọn lại 2026-11-13 (D16, 0.5.0). Bản 2026-11-11 liệt kê nhiều việc đã xong ở các nhánh sau; mục này chỉ giữ những gì còn thật, đã đối chiếu với code.

## Đã xong (không còn cần theo dõi)
- **Chuỗi cứng trong UI.** Baseline `UX/i18n/__tests__/hardcodedBaseline.json` chỉ còn `ScanBlocks.tsx` (3) và `blueprint.tsx` (1): đều là chú thích mã và ô chữ mẫu (`'ê', 'ẫ', 'ự'…`), không phải chuỗi giao diện. Onboarding, Illustrations, Settings, store/model đã dùng khoá `onb.*`/`set.*`.
- **Lệnh truyền `lang: 'vi'` cứng.** Pairing Desk, Detail và `api/compare.ts` dùng `getLocale()`; Rust (`explain.rs`) song ngữ theo `Lang`.
- **Ngôn ngữ mặc định.** `SettingsDto.uiLang` là `string | null`; chưa chọn → theo ngôn ngữ hệ thống (`resolveLocale`).
- **Giới thiệu và ghi công** (Hyperglot, SigLIP 2, thư viện khác) có màn riêng trong Cài đặt, vi + en.

## Còn lại
1. **Rust trả câu tiếng Việt ra UI.** Đường ra UI là `CmdError.message`: UI tiếng Anh thay bằng câu theo `code` (`api/tauri.ts::localizeCmdMessage`, 28 khoá `err.*` trong `i18n/en/err.ts`); mã lạ → "Something went wrong (code)". Còn lại đề xuất (chưa làm): thêm `detail` có cấu trúc (`{ "name": …, "max": … }`) cho các lỗi `invalid_arg` hay hiện ra UI rồi thêm khoá `err.<code>.<field>` có tham số. Hiện hầu hết `invalid_arg` là lỗi lập trình nên người dùng ít gặp.
2. **Chuỗi tiếng Việt không dấu.** Bộ quét chỉ thấy chuỗi CÓ dấu; chuỗi không dấu ("Trang sau", "Chung"…) phải duyệt bằng mắt khi thêm màn mới.
3. **QA tiếng Anh.** `QA_LANG=en bash UX/src-tauri/tests/qa/run.sh` chụp ảnh sáng/tối tiếng Anh. Một số bước còn so chữ tiếng Việt sẽ báo FAIL ở lượt EN, chỉ dùng để lấy ảnh; lượt mặc định vẫn là `vi`.

Không đổi: `DEFAULT_SAMPLE` ("Phở bò Hà Nội", `services/settings.rs`) là chữ mẫu.
