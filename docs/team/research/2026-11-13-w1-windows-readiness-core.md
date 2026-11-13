# W1 — Rà phần core chỉ chạy trên macOS (cần cổng `cfg` cho Windows)

Người viết: 02 · 2026-11-13 · Chỉ ĐỌC code và Cargo.toml (grep `cfg`, `objc2`, `HOME`, đường dẫn `Library/`…), CHƯA build Windows nên mọi mục "sẽ gãy" là suy luận từ code. Không làm tính năng Windows nào. Phần activation (06) và job CI Windows (07) chỉ ghi chỗ giao nhau.

## A. Đã có cổng đúng (không cần làm gì trước)
| Chỗ | Ghi chú |
|---|---|
| `objc2*`, `libc`, `tauri` WKWebView/AppKit trong `Cargo.toml` gốc | nằm dưới `[target.'cfg(target_os = "macos")'.dependencies]` |
| `fontatlas-activate`: `mac.rs`, `MacCoreTextEngine`, `platform_engine()` | Windows nhận `WindowsGdiEngine` (stub `Unsupported`) |
| `fontatlas-fontreq` (crate + `services/fontreq.rs`), `glass.rs`, `trash.rs`, `services/auto.rs` (liệt kê tiến trình bằng `proc_listallpids`), `services/cloud_download.rs` (nhánh macOS) | có nhánh `not(macos)` |
| `fontatlas-embed`: candle `accelerate`/`metal` | chỉ macOS; Windows rơi về CPU (chưa build thử) |
| `fontatlas-store::rescan::os_placeholder` | có nhánh Windows (`FILE_ATTRIBUTE_OFFLINE`/`RECALL_ON_DATA_ACCESS`), CHƯA có test trên Windows thật |
| `fontatlas-embed` `sync_directory`, `fontatlas-broker` quyền file 0600 | `cfg(unix)` + nhánh khác |
| `fontatlas-core`, `-preview` (skrifa, tiny-skia, png), `-index`, `-eval`, `-describe` | không thấy phụ thuộc nền tảng trong Cargo.toml (chưa xác nhận bằng build) |

## B. Giả định macOS KHÔNG có cổng (sẽ gãy hoặc sai trên Windows) — theo mức ưu tiên
| # | Chỗ | Vấn đề | Hướng xử lý (sau này) |
|---|---|---|---|
| 1 | `fontatlas-store/src/cloud.rs` (`cloud_provider_of`) | nhận ổ cloud bằng đường dẫn `Library/CloudStorage/…`, `Mobile Documents/…` | tách `cfg`: Windows nhận `OneDrive*`, `Dropbox`, `Google Drive`/`GoogleDrive` dưới `%USERPROFILE%` và ký tự ổ; giữ test mac |
| 2 | Đường dẫn trong DB: `file_locations.path TEXT UNIQUE`, so tiền tố bằng chuỗi (`LIKE`, `substr`, `starts_with`), `remove_source`, pack import | Windows không phân biệt hoa/thường, dấu `\`, tiền tố `\\?\` do `canonicalize` trả về (có 18 chỗ gọi) | một hàm chuẩn hoá đường dẫn dùng chung (bỏ `\\?\`, chuẩn hoá hoa/thường khi so), quyết trước khi có dữ liệu Windows; đụng schema nên làm sớm |
| 3 | `services/dup_resolve.rs` (danh sách thư mục hệ thống `/System`, `/Library`, `/private/var/db`, `/Applications`, `~/Library/Fonts`) và `fontatlas-activate::in_protected_font_dir` | bảo vệ "không đưa font hệ thống vào Thùng rác" dựa vào đường dẫn macOS; trên Windows không bảo vệ gì | danh sách theo nền tảng: `%WINDIR%\Fonts`, `%LOCALAPPDATA%\Microsoft\Windows\Fonts` |
| 4 | `HOME` (`services/system.rs`, `auto.rs`, `dup_resolve.rs`, `fontatlas-cep`, `fontatlas-indesign`) | Windows dùng `USERPROFILE`/`APPDATA`; chỉ `fontatlas-activate::service` đã có dự phòng `USERPROFILE` | một helper `home_dir()`/`app_support_dir()` dùng chung |
| 5 | `fontatlas-cep/install.rs` (`~/Library/Application Support/Adobe/CEP/extensions`), `fontatlas-indesign` (`~/Library/Preferences`) | đường dẫn plug-in Adobe khác trên Windows (`%APPDATA%\Adobe\CEP\extensions`; registry `PlayerDebugMode`) | tính năng riêng, gom cùng 06; trước mắt `cfg(target_os = "macos")` cả hai crate để Windows build được |
| 6 | `fontatlas-cep/identity.rs` | `std::os::unix::fs::OpenOptionsExt` cho khoá ký 0600, không có nhánh khác (broker thì có) | `cfg(unix)` + nhánh Windows (ACL hoặc bỏ qua có ghi chú) |
| 7 | `services/system.rs::detect_font_managers` | dò `/Applications/*.app` | danh sách theo nền tảng (đường cài Windows) hoặc ẩn trên Windows |
| 8 | `services/cloud_watch.rs` | `notify` dùng backend mặc định của Windows (ReadDirectoryChangesW) — đúng; nhưng bộ lọc dùng quy ước tệp ẩn bắt đầu bằng `.`, và so tiền tố gốc bị `\\?\` làm lệch (xem mục 2) | sửa cùng mục 2; thêm lọc tệp tạm của Office/Windows (`~$`, `.tmp` đã có) |
| 9 | `fontatlas-store::rescan::walk_fonts_report` | bỏ mục ẩn theo tên `.xxx`, bỏ stub `.icloud`; Windows ẩn bằng thuộc tính; không theo symlink (junction/reparse point cần xét riêng, OneDrive dùng reparse) | thêm kiểm thuộc tính HIDDEN, quyết cách xử lý reparse point |
| 10 | `fontatlas-store::rescan::walk_fonts_report` (`Metadata::created()`), `release::created_src` | ngày tạo lấy từ birth time qua `std` (đa nền tảng, không gãy build); trên Windows `CreationTime` đổi khi copy và `created_src` chỉ nhận ổ cloud theo đường dẫn macOS (mục 1) | kiểm thử trước khi tin để sắp xếp "mới thêm"; không cần cổng `cfg` |
| 11 | `tauri.conf.json` + `Cargo.toml` | `macOSPrivateApi`, cửa sổ `transparent`, `titleBarStyle: Overlay`, bundle `dmg`/`icns`, `glass.rs` | cấu hình riêng theo nền tảng (`tauri.windows.conf.json`), kính chỉ macOS (đã gate) |
| 12 | Test dùng `std::os::unix::fs::symlink`/`PermissionsExt` (`dup_resolve`, `activate::paths`, `access`) | không biên dịch được trên Windows | gắn `#[cfg(unix)]` cho test đó |
| 13 | Đường dẫn cứng trong test/fixture (`/Users/…`, `/Volumes/…`, `/System/…`) | chỉ là chuỗi trong test, nhưng một số gọi API đĩa thật | rà khi bật job CI Windows |

## C. Khuyến nghị thứ tự (khi có việc Windows)
1. 07 thêm job `cargo check --workspace` trên Windows (không chạy test) để biết lỗi biên dịch thật; mục B-5, B-6, B-12 sẽ lộ ngay.
2. Quyết mục B-2 (chuẩn hoá đường dẫn) TRƯỚC khi có người dùng Windows, vì nó đụng định dạng dữ liệu và gói thư viện (B7).
3. B-1, B-3, B-4, B-7 là thay đổi cục bộ, làm được độc lập.
4. Phần activation (đăng ký font với GDI/`AddFontResourceEx`, broadcast `WM_FONTCHANGE`) là việc của 06.
