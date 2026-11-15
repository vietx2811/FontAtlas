# W1(b) — "Mở cùng app" / App Font Sets trên Windows: id app là gì

Ngày: 2026-11-16 · Tác giả: 02 Core Engine · Trạng thái: **đánh giá, chưa có mã**. Mọi điều về Windows dưới đây là **suy luận từ tài liệu
API đã biết, chưa chạy trên máy Windows nào**. Mỗi điều chưa kiểm được đánh dấu *(chưa kiểm)*.

Bối cảnh: B5 (đã vào dev/mvp) gắn Case với app qua cột `case_apps.bundle_id` (+ `app_name`). Bộ quan sát
`running_bundle_ids()` thăm dò `NSWorkspace.runningApplications` mỗi 2 s; ngoài macOS trả tập rỗng. `installed_apps()` quét
`/Applications`, `/System/Applications`, `~/Applications`. Hàm đồng bộ `sync_app_sets` chỉ so **tập chuỗi id đang chạy** với `case_apps`, nên
phần lõi không đổi: chỉ cần (1) quy ước chuỗi id Windows, (2) hai hàm thu thập, (3) UI không giả định "bundle id".

## 1. Kết luận ngắn

- Id Windows là **chuỗi mờ có tiền tố**, lưu nguyên trong cột `bundle_id` (không đổi schema):
  - `win:pfn:<PackageFamilyName>` cho app đóng gói MSIX/Store (ổn định qua cập nhật).
  - `win:exe:<tên-file-exe-viết-thường>` cho app Win32 cổ điển, ví dụ `win:exe:photoshop.exe`.
- macOS giữ nguyên id trần (`com.adobe.illustrator`), **không** thêm tiền tố → dữ liệu cũ không cần migrate.
- **Không dùng** đường dẫn exe đầy đủ làm id (đổi theo năm: `Adobe Photoshop 2026` → `2027`), cũng không dùng AppUserModelID làm id chính
  (không đọc được từ tiến trình ngoài cho app Win32; xem §3).
- Danh sách "app đã cài": liệt kê thư mục ảo **AppsFolder** (đúng thứ Start Menu hiện), không đọc registry Uninstall làm nguồn chính.
- Phát hiện đang chạy: liệt kê tiến trình (crate `sysinfo`, lấy đường dẫn exe), suy ra id từ đường dẫn.
- Frontend coi id là **chuỗi mờ**; chỉ dùng thêm hai trường (`kind`, `platform`) nếu cần lọc. Chi tiết §6.

## 2. Vì sao không phải bundle id

macOS có một định danh chuẩn, ổn định, do hệ điều hành cấp (`CFBundleIdentifier`), đọc được cả từ tiến trình đang chạy lẫn từ bó `.app` đã cài.
Windows **không có** định danh nào vừa ổn định vừa đọc được ở cả hai phía cho mọi loại app:

| Ứng viên | Đọc từ tiến trình đang chạy | Đọc từ app đã cài | Ổn định qua cập nhật | Phủ mọi app |
|---|---|---|---|---|
| Đường dẫn exe đầy đủ | có (`QueryFullProcessImageNameW`) | có (đích của lối tắt) | **không** (thư mục có năm/phiên bản; app cài theo user) | có |
| Tên file exe | có | có | có (`Photoshop.exe`, `Illustrator.exe`, `InDesign.exe`, `AfterFX.exe` giữ nguyên qua các năm) | có, nhưng có thể trùng tên |
| AppUserModelID (AUMID) | app đóng gói: có (`GetApplicationUserModelId` trên handle tiến trình); app Win32: chỉ khi app tự đặt và phải đi qua cửa sổ (`PKEY_AppUserModel_ID`) | có (AppsFolder) | có | một phần |
| PackageFamilyName | có (từ handle hoặc suy từ đường dẫn, §4) | có | có | chỉ app đóng gói |
| Registry Uninstall (khoá + `DisplayName`) | không | có | có | không (không có đường dẫn exe chắc chắn; nhiều app không ghi `DisplayIcon`/`InstallLocation`) |

Không ứng viên nào đứng một mình; ghép `pfn` (đóng gói) + `exe` (Win32) phủ gần hết và cả hai đều tính được từ **cùng một nguồn ở phía chạy**:
đường dẫn exe của tiến trình.

## 3. Vì sao không chọn AUMID làm id chính

- Với app Win32 cổ điển, nếu app không gọi `SetCurrentProcessExplicitAppUserModelID` hoặc lối tắt không gắn `System.AppUserModel.ID`, Windows chỉ
  dùng một AUMID **ngầm** suy từ đường dẫn; ta không tái tạo được nó đáng tin ở phía tiến trình. *(chưa kiểm: Adobe có đặt AUMID tường minh hay không.)*
- Đọc AUMID tường minh của tiến trình khác cần cửa sổ cấp cao nhất của nó và `SHGetPropertyStoreForWindow`; tiến trình nền, không cửa sổ, không có.
- Vẫn nên **lưu thêm** AUMID/đường dẫn làm gợi ý hiển thị (icon, tên), nhưng không dùng để so khớp.

## 4. Thu thập phía "đang chạy"

Mục tiêu: `running_app_ids() -> HashSet<String>`, gọi mỗi 2 s như macOS.

1. `sysinfo` (chưa là dependency của dự án; thêm có điều kiện `[target.'cfg(windows)'.dependencies]`). Làm mới tiến trình chỉ với `exe` (không CPU/RAM)
   để rẻ; dùng `UpdateKind::OnlyIfNotSet` để không hỏi lại exe của pid đã biết. *(chưa kiểm chi phí thực; kỳ vọng vài ms cho vài trăm tiến trình.)*
2. Với mỗi tiến trình có đường dẫn exe `P`:
   - Nếu `P` nằm dưới `...\WindowsApps\<PackageFullName>\...`: thư mục có dạng `<Name>_<Version>_<Arch>_<ResourceId>_<PublisherId>`; lấy
     `PackageFamilyName = <Name>_<PublisherId>` → id `win:pfn:<Name>_<PublisherId>` (so khớp không phân biệt hoa thường). Không cần handle, nên tránh được
     lỗi quyền. *(suy từ định dạng tên gói đã biết; chưa kiểm trên máy thật.)*
   - Ngược lại: id `win:exe:<tên file viết thường>`.
3. Quyền: chỉ cần `PROCESS_QUERY_LIMITED_INFORMATION`, thường mở được cả tiến trình chạy "as administrator" của cùng người dùng; tiến trình bảo vệ
   (protected) và dịch vụ hệ thống có thể không trả đường dẫn → bỏ qua, không phải lỗi. *(chưa kiểm.)*
4. Nhiều tiến trình cùng một app (Adobe, trình duyệt có tiến trình con): gộp theo id, tập hợp tự khử trùng. Tiến trình **nền** của Adobe
   (`CCXProcess.exe`, "Adobe Desktop Service", `AdobeIPCBroker.exe`) có tên exe khác `Photoshop.exe`, nên không làm Case của Photoshop bật nhầm. Điều này là lý do
   không so khớp theo "nhà phát hành".
5. Phiên người dùng khác (chuyển người dùng nhanh, RDP): `sysinfo` thấy tiến trình của mọi người dùng. Nên lọc theo `session id`/chủ sở hữu = người dùng hiện tại
   để font không bật vì app của người khác. *(chưa kiểm khả năng lọc bằng `sysinfo`; có thể cần `ProcessIdToSessionId`.)*

## 5. Thu thập phía "đã cài" (danh sách để người dùng chọn)

`installed_apps() -> Vec<InstalledAppDto>` cần: `id`, `name`, `running`, tuỳ chọn biểu tượng.

Nguồn, theo thứ tự ưu tiên:

1. **AppsFolder** (`shell:AppsFolder`, `FOLDERID_AppsFolder`): liệt kê bằng `IShellItem`/`IEnumShellItems`; mỗi mục có tên hiển thị và `PKEY_AppUserModel_ID`.
   Đây là cái Start Menu hiển thị, gồm cả app đóng gói và Win32 có lối tắt. Với mục đóng gói, AUMID có dạng `<PackageFamilyName>!<AppId>` → ra `win:pfn:`.
   Với mục Win32, cần đường dẫn exe: lấy từ tên phân tích (parsing name; với app không đặt AUMID thường là `{GUID thư mục đã biết}\đường\dẫn\app.exe`, *chưa kiểm*) hoặc
   mở lối tắt `.lnk` tương ứng bằng `IShellLink` để lấy đích → ra `win:exe:`.
2. **Lối tắt Start Menu** (`%ProgramData%\Microsoft\Windows\Start Menu\Programs`, `%APPDATA%\Microsoft\Windows\Start Menu\Programs`, đệ quy): dự phòng nếu (1) lỗi.
   Phân giải `.lnk` → đích `.exe`. Lọc `Uninstall*.lnk`, tài liệu, thư mục "Tools".
3. **`App Paths`** (`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\<exe>`): tra ngược "tên exe → đường dẫn" để xác nhận app tồn tại; không đủ để liệt kê.
4. **Registry Uninstall** (`HKLM\...\Uninstall`, `HKLM\SOFTWARE\WOW6432Node\...\Uninstall`, `HKCU\...\Uninstall`): chỉ dùng để lấy `DisplayName`/`DisplayIcon`
   bổ sung. Không dùng làm danh sách: nhiều mục là runtime, bản vá, driver, không có exe.

Quyết định: dùng `windows` crate (feature `Win32_UI_Shell`, `Win32_System_Com`, `Win32_Storage_FileSystem`) cho (1)(2). Không gọi PowerShell `Get-StartApps` (chậm cỡ 1 s,
phụ thuộc chính sách thực thi, khó test).

Bắt buộc dành cho cả `running` và `installed`: so khớp id **không phân biệt hoa thường** (chuẩn hoá chữ thường ngay khi tạo id).

## 6. Đề xuất giao diện cho 05 (lọc trong bộ chọn app)

Tôi hiểu "lọc trình duyệt" là bộ chọn/duyệt app trong hộp thoại "Mở cùng app…". Nếu ý khác (ví dụ lọc **ẩn trình duyệt web** khỏi danh sách), xin nói lại.

- Giữ nguyên tên trường hợp đồng hiện có (`bundleId`, `name`); thêm hai trường **đọc-chỉ** vào `InstalledAppDto`:
  - `kind`: `"bundle" | "package" | "exe"` — cách id được tạo (macOS: `bundle`).
  - không thêm `platform`: có thể suy ra từ tiền tố (`win:`), và UI chỉ hiện app của máy hiện tại.
- UI **không** tách chuỗi id, không hiển thị id cho người dùng (hiện `name` + biểu tượng). Id chỉ để gửi lại cho `case_set_apps`.
- Lọc nên làm theo `name` (không phân biệt hoa thường/dấu) và theo cờ `running`; không lọc theo nhà phát hành (Windows không có trường ổn định).
- Khi mở một Case có app gắn từ máy khác hệ điều hành (gói chuyển từ Mac sang Windows): id `com.…` không khớp app nào. UI hiện dòng "App này không có trên máy này"
  (tên lấy từ `app_name` đã lưu) và cho gỡ, không báo lỗi.
- Tùy chọn (chưa cần): bảng ánh xạ cho 6 app đã biết trong `services/auto.rs` (`photoshop`, `illustrator`, `indesign`, `aftereffects`, `incopy`, `affinity`):
  mỗi khoá logic có cả id macOS lẫn Windows, để một Case gắn "Illustrator" chạy được trên cả hai máy. Cần quyết định sản phẩm trước.

## 7. Rủi ro và việc cần kiểm trên máy Windows thật

1. Tên exe trùng giữa hai app khác nhau (`launcher.exe`, `update.exe`). Giảm bằng việc người dùng chỉ chọn từ danh sách đã cài (đã lọc); nếu thực tế trùng, mở rộng id
   thành `win:exe:<tên>?pub=<nhà phát hành>` (đọc `CompanyName` từ version resource, cache theo đường dẫn) mà không cần đổi schema.
2. Adobe trên Windows: tên exe thực tế của từng app (đã biết: `Photoshop.exe`, `InDesign.exe`, `Illustrator.exe` nằm trong `Support Files\Contents\Windows`, `AfterFX.exe`) — *chưa kiểm
   trên máy thật*; cần một người có Windows xác nhận đúng 5 tên.
3. Chi phí thăm dò 2 s trên máy nhiều tiến trình.
4. App chạy quyền cao, protected process, và tiến trình người dùng khác (§4).
5. Windows on ARM: app x64 chạy giả lập, đường dẫn vẫn là đường dẫn exe thường; không khác biệt về id.

## 8. Phần việc Rust nếu được duyệt (không làm bây giờ)

- `running_app_ids()` thay `running_bundle_ids()` (đổi tên; macOS giữ nguyên hành vi), nhánh `cfg(windows)` theo §4.
- `installed_apps()` nhánh `cfg(windows)` theo §5; thêm `kind` vào DTO + bindings + mock (05 sở hữu `api/`).
- Test thuần (không cần Windows): hàm `pfn_from_path(&str)`, `exe_id(&str)` nhận chuỗi đường dẫn kiểu Windows và chạy được trên macOS CI.
- Phụ thuộc: cùng nhánh Windows của 06 (engine GDI) và quyết định đường dẫn trong ADR path-normalization (đang treo).
