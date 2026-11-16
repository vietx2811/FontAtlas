# 06: W1 — phần kích hoạt font chỉ chạy trên macOS (chuẩn bị Windows)

Ngày 2026-11-13. Đọc code ở dev/mvp 1b64934b (nhánh `fix/signal-exit`). Chưa làm tính năng Windows; chỉ liệt kê. Nhãn: **ĐỌC CODE** / **SUY LUẬN**.

## A. Đã gom sau `cfg(target_os = "macos")` (có đường thay thế hoặc stub)
| Chỗ | macOS làm gì | Phần không phải macOS hiện tại |
|---|---|---|
| `fontatlas-activate/src/mac.rs` + dep `objc2-core-text/-core-foundation` (Cargo.toml `[target.'cfg(target_os="macos")']`) | `MacCoreTextEngine`: `CTFontManagerRegister/UnregisterFontsForURL`, liệt kê PostScript name, `providers()` | `windows.rs`: `WindowsGdiEngine` stub, mọi thao tác trả `Unsupported`; `platform_engine()` chọn theo `cfg` |
| `fontatlas-fontreq` (toàn crate dưới `cfg(macos)`): `CTFontManagerCreateFontRequestRunLoopSource`, `proc_pidpath` | Nghe font request của app thiết kế (Photoshop…) | Không có; `src/services/fontreq.rs` có nhánh stub |
| `fontatlas-activate/src/paths.rs` `SF_DATALESS` (+ `std::os::macos::fs::MetadataExt::st_flags`) | Nhận biết file iCloud "dataless" | Nhánh khác bỏ qua cờ (luôn `Ready`) |
| `src/trash.rs` | `NSFileManager.trashItemAtURL` | stub |
| `src/services/cloud_download.rs` | `startDownloadingUbiquitousItemAtURL` | "không hỗ trợ" |
| `src/glass.rs`, `qa_snapshot` (WKWebView) | hiệu ứng kính, ảnh chụp QA | không có |
| `src/signals.rs` (mới, `cfg(unix)`) | SIGTERM/SIGINT thoát gọn | Windows cần `SetConsoleCtrlHandler`/sự kiện đóng phiên: chưa có |

## B. KHÔNG gắn `cfg` nhưng giả định macOS (lỗi âm thầm trên Windows)
1. **`paths.rs`: `classify_owner`, `in_protected_font_dir`, `replace_tier`**: hằng đường dẫn `/System`, `/Library/Fonts`, `/Network/Library/Fonts`, `~/Library/Fonts`, `Library/Application Support/{Typeface,Extensis,Adobe/CoreSync}`, `RightFont`, `Mobile Documents/com~apple~CloudDocs`. Windows: `C:\Windows\Fonts`, `%LOCALAPPDATA%\Microsoft\Windows\Fonts`, thư mục dữ liệu của từng trình quản lý font. Cần bảng theo nền tảng và so sánh đường dẫn không phân biệt hoa/thường, dấu `\`, đường dẫn UNC/ổ đĩa.
2. **`services/dup_resolve.rs`**: danh sách thư mục bị cấm `["/System","/Library","/private/var/db","/Applications"]` và `Library/Fonts` của người dùng. Cần tương đương Windows.
3. **`services/system.rs`** (trình quản lý font đã cài: `RightFont.app`, `Typeface.app`…) và **`services/auto.rs`** (quét `/Applications`, tiền tố tên gói `.app`): Windows dùng đường dẫn `.exe`/registry Uninstall; khái niệm `.app` không có.
4. **`services/cep.rs`, `services/indesign.rs`**: thư mục Adobe dưới `~/Library/...` (panel CEP, startup script). Windows: `%APPDATA%\Adobe\...` và `C:\Program Files\Common Files\Adobe\CEP`.
5. **`fontatlas-store/src/cloud.rs`**: danh sách gốc đám mây theo macOS (`CloudDocs`, Dropbox, ...). Windows: OneDrive/Dropbox/`%USERPROFILE%`, và cờ placeholder khác (`FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`).
6. **Chuỗi đường dẫn**: code lưu `String` có `/`; `canonical_path` dựa `std::fs::canonicalize` (Windows trả `\\?\C:\...`). Cần chuẩn hoá một chỗ.
7. **Test** dùng `std::os::unix::fs::symlink` và đường dẫn `/Users/u/...` (nhiều test trong `tests/replace.rs`, `service.rs`): cần `cfg(unix)` hoặc đường dẫn theo nền tảng khi có job build Windows.
8. **`libc`** là dep chung (`Cargo.toml:47`): biên dịch được trên Windows nhưng mọi dùng `libc::` phải nằm sau `cfg(unix)` (`signals.rs` đã làm).

## C. Khác biệt ngữ nghĩa kích hoạt (SUY LUẬN, chưa đo trên Windows)
- Hiện có ba scope CoreText: process / session / persistent; ta dùng session và persistent. Windows GDI: `AddFontResourceEx` (`FR_PRIVATE` = chỉ tiến trình; không private = cả phiên đăng nhập cho tới khi đăng xuất, cần `WM_FONTCHANGE`); cài lâu dài = chép vào `%LOCALAPPDATA%\Microsoft\Windows\Fonts` + registry (khác bản chất: **là cài, không phải đăng ký tại chỗ**). Mô hình "đăng ký tại chỗ, không chép file" của Font Atlas không áp dụng được cho persistent trên Windows. Cần quyết định sản phẩm.
- Không có font request như L2 (CoreText); auto-activation trên Windows chỉ qua plugin/panel của từng app.
- Quan sát app mở/đóng (B5): NSWorkspace → Windows: thăm dò tiến trình hoặc WMI/ETW process start.

## D. Việc nên làm trước khi có job Windows
1. Tách `PlatformPaths` (thư mục hệ thống/được bảo vệ, thư mục font người dùng, thư mục của trình quản lý font) thành một trait/mô-đun có hai bản triển khai; đây là phần lớn mục B.
2. Biên dịch crate `fontatlas-activate` trên Windows ở chế độ chỉ-kiểm-tra (`cargo check --target x86_64-pc-windows-msvc`) để thấy lỗi thật; stub engine đã đủ để biên dịch (chưa kiểm).
3. Gắn `cfg(unix)` cho test dùng symlink.

## E. CEP/InDesign trên Windows

Đánh giá, chưa làm. Dùng lại được: panel CEP, cách ký ZXP (RSA/SHA-1/SHA-256 thuần Rust, không phụ thuộc nền tảng), broker nghe `127.0.0.1` + `broker.json`. Còn thiếu:

1. **Đường cài panel.** `windows_extensions_dir` đã trỏ `%APPDATA%\Adobe\CEP\extensions` (có unit test). Chưa kiểm ở runtime trên Windows.
2. **Cờ `PlayerDebugMode`.** CEP chỉ nạp panel tự ký khi cờ này bật. macOS dùng `defaults`; Windows ghi registry `HKCU\Software\Adobe\CSXS.<n>`, giá trị chuỗi `PlayerDebugMode` = `1`. Đây là thay đổi cấu hình của Adobe nằm ngoài app, nên **phải hỏi người dùng trước** (như macOS), nêu rõ khoá nào sẽ ghi và cách gỡ; cần crate truy cập registry.
3. **Quyền khoá ký.** Khoá của máy nằm ở thư mục dữ liệu app; mode `0600` chỉ áp dụng dưới `cfg(unix)`. Windows cần ACL hạn chế cho người dùng hiện tại, hoặc chấp nhận quyền mặc định của thư mục dữ liệu (yếu hơn) và ghi rõ.
4. **Phát hiện app.** `services/{system,auto,app_sets}.rs` quét `/Applications`, đọc `Info.plist`, dùng NSWorkspace: chỉ macOS. Windows cần quét registry gỡ cài đặt / `App Paths` và thư mục `Program Files\Adobe`, và nhận biết app đang chạy theo tên tiến trình thay cho bundle id.
5. **Đường startup script của InDesign.** `default_prefs_root` trỏ thư mục prefs của macOS; Windows dùng cây `%APPDATA%\Adobe\InDesign\Version <n>\<locale>\Scripts\startup scripts` — cần bảng đường dẫn riêng và kiểm với bản InDesign thật.
6. **Engine font cho Windows (phần lớn nhất).** Hiện chỉ có engine CoreText. Windows cần engine riêng quanh `AddFontResourceEx` (`FR_PRIVATE` = chỉ tiến trình; không private = cả phiên đăng nhập, kèm `WM_FONTCHANGE`). Xem mục C về việc "cài" khác "đăng ký tại chỗ" và cần quyết định sản phẩm trước khi viết.

Thứ tự gợi ý: (1) xác nhận cài panel + đường startup script bằng bản Adobe thật, (2) registry có hỏi người dùng, (3) phát hiện app, (4) ACL khoá, (5) engine.
