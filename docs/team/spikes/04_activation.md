# Spike 04 — Font Activation (macOS + Windows)

> Người làm: Font Activation · Ngày: 2026-10-15 · Trạng thái: SPIKE (code bỏ đi)
> Code thử: worktree `trusting-cray-12bc0a`, thư mục `spikes/activation/` (chưa commit, không đụng code sản phẩm).

## 0. Tóm tắt & khuyến nghị

1. **macOS: dùng CoreText đăng ký tại chỗ (không copy file).** Đã chạy thật: đăng ký/gỡ mất 10–70 ms, tiến trình khác thấy font **ngay lập tức**. Gỡ sạch, không để lại dấu vết.
2. **Mặc định = "theo phiên app" kiểu FontBase, nhưng làm bằng scope `session` chứ không phải `process`.** Scope `process` chỉ tiến trình FontAtlas thấy, Photoshop/Figma không thấy. Cách làm: khi app mở thì đăng ký lại từ `activation_state`, khi app thoát thì gỡ. Nếu app crash, font vẫn còn đến khi đăng xuất. Hậu quả này chấp nhận được, và lần mở sau sẽ đối soát lại.
3. **Tuỳ chọn "Giữ kích hoạt khi tắt app" = scope `persistent`, vẫn đăng ký tại chỗ.** RightFont trên chính máy này đang làm đúng như vậy với thư viện nằm trong iCloud Drive (xem §1.4).
4. **Không copy vào `~/Library/Fonts` trong bản đầu.** Nếu cần thì sau này thêm thành hành động riêng "Cài vào hệ thống", có xác nhận.
5. **CoreText không chặn trùng PostScript name** (đã thử thật). FontAtlas phải tự phát hiện xung đột trước khi kích hoạt.
6. **Windows (chưa chạy thật):**
   - Chế độ phiên: `AddFontResourceExW(path, 0)` rồi broadcast `WM_FONTCHANGE`; gỡ khi thoát.
   - Chế độ bền vững: copy vào `%LOCALAPPDATA%\Microsoft\Windows\Fonts`, ghi giá trị HKCU trỏ đường dẫn đầy đủ, rồi `AddFontResourceExW` và broadcast.
   - Cần Windows 10 1809 trở lên. Microsoft Learn không có trang chính thức mô tả nhánh HKCU; nguồn hiện có là cộng đồng (§3).

## 1. macOS — kết quả thử thật

**Môi trường:** macOS 27.2 (build 26B5101f), Apple Silicon, rustc 1.96.0, `objc2-core-text` 0.3.2 + `objc2-core-foundation` 0.3.2.
**Font thử:** Libre Baskerville Regular (OFL), file `fonts/samples/libre_baskerville.ttf` trong worktree. Trước khi thử, font này chưa có trên máy. Được người dùng đồng ý cho thử cả 3 scope.
**API:** `CTFontManagerRegisterFontsForURL` / `CTFontManagerUnregisterFontsForURL` (đồng bộ, trả về `CFError`). Kiểm tra bằng `CTFontManagerCopyAvailablePostScriptNames` từ **tiến trình riêng**. Thêm một kiểm tra bằng `NSFontManager.availableFontFamilies` trong một tiến trình Swift/AppKit riêng, cùng cơ chế mà TextEdit dùng để liệt kê font.

### 1.1 Bảng kết quả

| # | Thử nghiệm | Kết quả |
|---|---|---|
| 1 | `process`: đăng ký → tự thấy → gỡ | OK, 3.4 ms. Sau khi tiến trình thoát, tiến trình khác **không** thấy font |
| 2 | Đăng ký lần 2 cùng file, cùng scope | Lỗi `CTFontManagerErrorDomain` **105** "already been registered in the specified scope". Dùng được để nhận biết trạng thái idempotent |
| 3 | `session`: đăng ký | OK, 69 ms. Tiến trình khác thấy ngay ở +0 s, +1 s, +3 s. AppKit (`NSFontManager`) cũng thấy family "Libre Baskerville" |
| 4 | **Trùng tên**: đăng ký bản copy cùng PostScript name ở đường dẫn khác, cùng scope `session` | **OK, không báo lỗi**. Gỡ bản copy xong, font vẫn AVAILABLE nhờ bản gốc. Chỉ gỡ cả hai thì font mới biến mất |
| 5 | Đăng ký (session) rồi **đổi tên file** | Tên font **vẫn được báo AVAILABLE** dù file ở đường dẫn cũ không còn: registry bị cũ, có thể render lỗi hoặc rơi về font khác |
| 6 | Đăng ký (session), **xoá file**, rồi gỡ theo đường dẫn cũ | Gỡ OK. Gỡ theo đường dẫn vẫn chạy khi file đã mất, **miễn là đúng chuỗi đường dẫn đã canonical** (`/private/tmp/...` khác `/tmp/...`) |
| 7 | `persistent`: đăng ký | OK, 31 ms. Tiến trình khác thấy ngay. `~/Library/Fonts` **không thay đổi** |
| 8 | `persistent`: gỡ nhầm với scope `session` | Lỗi **201** "not registered in the specified scope". Phải lưu scope đã dùng |
| 9 | `persistent`: gỡ đúng scope | OK, 16 ms. Không còn chuỗi nào trỏ tới file thử trong registry |

**Đã dọn sạch:** cả 3 scope đã gỡ, `LibreBaskerville-Regular: not available`. Không ghi gì vào `~/Library/Fonts`.

### 1.2 Persistent được lưu ở đâu (quan sát, chỉ đọc)

- Nằm trong SQLite `$(getconf DARWIN_USER_CACHE_DIR)/com.apple.FontRegistry/fontregistry.user`. Bên cạnh có `fontregistry.session` cho scope session.
- Mỗi mục lưu **đường dẫn tuyệt đối** cùng `postscript-name=…`. Đây là lý do không được di chuyển file sau khi đăng ký.
- **Rủi ro:** file này nằm trong thư mục *cache* của user (`/var/folders/.../C/`). Chưa xác minh macOS có bao giờ dọn hoặc dựng lại nó không (ví dụ khi chạy `atsutil databases -remove`, Safe Boot, hoặc nâng cấp OS). Vì vậy app phải đối soát lại mỗi lần khởi động (§4).

### 1.3 Chưa thử (người dùng chọn chỉ kiểm tra tự động)

- **Font Book / TextEdit (GUI), Photoshop 2026, Illustrator 2026, Figma**: có trên máy nhưng chưa mở để kiểm tra. Bằng chứng gián tiếp: tiến trình AppKit riêng thấy font ngay.
- **Đăng xuất / khởi động lại:** chưa làm. Cách thử thủ công:
  1. `cargo run -- register fonts/samples/libre_baskerville.ttf session`, và một font khác với `persistent`.
  2. Đăng xuất rồi đăng nhập lại. Chạy `cargo run -- names <file>`: kỳ vọng font session đã biến mất, font persistent vẫn còn.
  3. Khởi động lại máy, kiểm tra lần nữa.
  4. Gỡ bằng `unregister <file> persistent`.
- **Checklist cho app thiết kế (lần sau, khi người dùng sẵn sàng):** kích hoạt ở scope session. Với mỗi app, ghi hai trường hợp: (a) app **đang mở**, font có xuất hiện trong menu không; (b) phải khởi động lại app thì mới có.
  - Photoshop: Type tool → menu font.
  - Illustrator: Type → Font.
  - Figma desktop: chọn text → ô font. Figma đọc font qua agent font cục bộ; cần ghi lại có phải restart agent không.
  - Tài liệu cộng đồng Adobe nói thường **phải mở lại app** vì Photoshop/Illustrator cache danh sách font. Illustrator 30.3 trên macOS có báo cáo hỏng auto-activation của Typeface.

### 1.4 Bằng chứng từ thị trường (trên máy này)

`fontregistry.user` có hàng chục nghìn chuỗi `…/Mobile Documents/com~apple~CloudDocs/Fonts/Grid.rightfontlibrary/fonts/…`. Như vậy **RightFont kích hoạt bằng cách đăng ký CoreText tại chỗ, scope persistent**, ngay trên thư viện trong iCloud Drive, không copy vào `~/Library/Fonts`. Mình chỉ đếm, không đọc thêm dữ liệu cá nhân.

Hệ quả cho FontAtlas:
- Font của người dùng **có thể đang được RightFont kích hoạt**. Khi kích hoạt bằng FontAtlas sẽ tạo bản trùng (xem thử nghiệm #4).
- Thư viện nằm trên iCloud Drive có thể bị *evict* (file chỉ còn placeholder). Cần kiểm tra file đã được tải xuống máy trước khi đăng ký.

## 2. So sánh 3 chiến lược trên macOS

| Tiêu chí | A. CoreText tại chỗ (`persistent`) | B. Copy vào `~/Library/Fonts` | C. Theo phiên app (`session`, gỡ khi thoát) |
|---|---|---|---|
| Độ tin cậy với app khác | Cao; thấy ngay (đã thử) | Cao nhất; app/legacy quét thư mục đều thấy | Cao như A khi app đang chạy |
| Tồn tại qua restart | Có (theo tài liệu Apple; chưa thử restart) | Có | Không (đúng mục đích) |
| File bị di chuyển / ổ ngoài rút ra | Registry cũ, font "có mà không dùng được" (#5) | Không ảnh hưởng | Như A nhưng chỉ đến khi app thoát |
| Trùng tên | CoreText chấp nhận cả hai, không báo (#4), app phải tự chặn | Trùng *tên file* bị ghi đè; trùng PS name thì macOS có thể vô hiệu một bản | Như A |
| Dung lượng | 0 | Nhân đôi | 0 |
| Dọn dẹp khi gỡ app | **Rác**: entry trỏ tới file vẫn tồn tại. Cần nút "Tắt hết", và trình gỡ cài đặt không làm được việc này | **Rác nặng**: file còn lại, người dùng tưởng là font hệ thống | **Sạch**: tự hết khi đăng xuất |
| Sandbox (Mac App Store) | Cần quyền đọc file mỗi lần khởi động (security-scoped bookmark); chưa thử | Không ghi được `~/Library/Fonts` nếu không có temporary-exception entitlement | Như A |
| Notarize + hardened runtime | Không cần entitlement đặc biệt cho CTFontManager (chưa thử bản notarize thật) | Không cần | Không cần |

**Khuyến nghị:** C làm mặc định, A làm tuỳ chọn cho từng font hoặc collection. Không làm B ở bản đầu. Phát hành ngoài App Store, không sandbox: Tauri + notarize là đủ, không xin entitlement.

## 3. Windows — nghiên cứu (KHÔNG chạy thật: máy là macOS, chưa cài target Windows, không ghi registry)

### 3.1 Quy trình đề xuất

**Chế độ phiên** (tương đương session):
1. `AddFontResourceExW(path, 0, None)`. **Không** dùng `FR_PRIVATE`, vì cờ này chỉ cho tiến trình gọi thấy và tự gỡ khi tiến trình thoát.
2. `SendMessageTimeoutW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0, SMTO_ABORTIFHUNG, 2000)`. Dùng bản có timeout để không bị treo vì một cửa sổ không phản hồi.
3. Khi thoát: `RemoveFontResourceExW(path, 0)` rồi broadcast lại. Font cũng mất sau khi khởi động lại máy (theo Microsoft Learn).
4. Không cần admin. Lưu ý: khi font đang được nạp, file bị **khoá**, không đổi tên hay xoá được.

**Chế độ bền vững per-user, không admin** (Windows 10 1809+; nguồn cộng đồng, chưa có trang Microsoft Learn chính thức):
1. Copy file vào `%LOCALAPPDATA%\Microsoft\Windows\Fonts\`, lấy đường dẫn qua `SHGetKnownFolderPath(FOLDERID_LocalAppData)`. Khi trùng tên file thì thêm hậu tố `_1`, `_2`…
2. Ghi giá trị REG_SZ vào `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts`:
   - **Tên**: `"<Family> <Style> (TrueType)"` (Regular thì bỏ Style).
   - **Dữ liệu**: **đường dẫn đầy đủ**. Nhánh HKLM chỉ cần tên file.
3. `AddFontResourceExW` + broadcast `WM_FONTCHANGE` để font có ngay mà không phải đăng xuất.
4. Gỡ: `RemoveFontResourceExW`, xoá giá trị HKCU, broadcast, xoá file. Nếu file vẫn bị khoá thì đưa vào hàng đợi dọn và thử lại ở lần mở app sau.

**Câu hỏi mở, cần thử trên máy Windows hoặc VM:**
- Giá trị HKCU trỏ thẳng vào file trong thư viện (không copy) có được nạp lại khi đăng nhập không? Nếu được thì Windows cũng làm được "tại chỗ" như macOS.
- Photoshop/Illustrator trên Windows có phản ứng với `WM_FONTCHANGE` khi đang mở không?
- Group Policy có thể chặn font per-user (theo borncity). Cần kiểm tra trên máy công ty.

### 3.2 Crate `windows` 0.62.2 — feature (đã xác minh bằng mã nguồn crate)

| API | Feature |
|---|---|
| `AddFontResourceExW`, `RemoveFontResourceExW`, `FR_PRIVATE` | `Win32_Graphics_Gdi` |
| `SendMessageTimeoutW`, `HWND_BROADCAST`, `WM_FONTCHANGE` | `Win32_UI_WindowsAndMessaging` |
| `RegSetKeyValueW`, `RegDeleteKeyValueW` | `Win32_System_Registry` |
| `SHGetKnownFolderPath`, `FOLDERID_LocalAppData` | `Win32_UI_Shell` (+ `Win32_System_Com` cho `CoTaskMemFree`) |
| Kiểu chung (`HWND`, `PCWSTR`…) | `Win32_Foundation` |

Dùng thêm `Win32_Graphics_DirectWrite` nếu muốn kiểm tra font đã hiển thị cho hệ thống chưa (đếm trong system font collection). Phần này chưa thử.

## 4. Đề xuất `ActivationEngine` + bảng `activation_state`

Đơn vị kích hoạt là **file** vì cả CoreText lẫn GDI đều đăng ký theo file, và TTC thì đăng ký cả file. UI gộp theo family hoặc collection.

```rust
pub enum Scope { Session, Persistent }

pub struct Request<'a> { pub file_hash: &'a str, pub path: &'a Path, pub scope: Scope }

pub enum Outcome {
    Active { registered_path: PathBuf, os_ref: OsRef },        // OsRef: Win = registry value name + copy path
    AlreadyActive,                                              // mac lỗi 105 → coi là thành công
    Conflict { ps_name: String, existing: Option<PathBuf> },    // tên đã có trên hệ thống / bởi app khác
    MissingFile, NotDownloaded /* iCloud placeholder */, Failed { code: String, msg: String },
}

pub trait ActivationEngine: Send + Sync {
    fn capabilities(&self) -> Caps;                  // scope hỗ trợ, có cần copy không, cần notify không
    fn preflight(&self, r: &Request) -> Outcome;      // đọc PS name, so với tên đang có, kiểm file tồn tại/đã tải
    fn activate(&self, reqs: &[Request]) -> Vec<Outcome>;              // theo lô; Windows broadcast 1 lần/lô
    fn deactivate(&self, rows: &[ActivationRow]) -> Vec<Outcome>;      // dùng registered_path + scope ĐÃ LƯU
    fn is_visible(&self, ps_name: &str) -> bool;
    /// Gọi khi app khởi động: so desired vs thực tế, kích hoạt lại session, gỡ entry mồ côi.
    fn reconcile(&self, rows: &[ActivationRow]) -> Vec<(String, Outcome)>;
    /// Gọi khi app thoát: gỡ mọi dòng scope=Session.
    fn shutdown(&self, rows: &[ActivationRow]);
}
// impl: MacCoreTextEngine, WinGdiEngine; tương lai: LinuxFontconfigEngine.
```

```sql
CREATE TABLE activation_state (            -- bảng NGƯỜI DÙNG: rescan không ghi đè
  file_hash       TEXT PRIMARY KEY REFERENCES files(hash),
  desired         TEXT NOT NULL CHECK (desired IN ('active','inactive')),
  scope           TEXT NOT NULL CHECK (scope IN ('session','persistent')),
  platform        TEXT NOT NULL CHECK (platform IN ('macos','windows')),
  registered_path TEXT,      -- chuỗi đường dẫn CANONICAL đã đưa cho OS; bắt buộc để gỡ (mac #6, #8)
  win_copy_path   TEXT,      -- Windows persistent: bản copy trong LOCALAPPDATA
  win_reg_value   TEXT,      -- Windows persistent: tên giá trị HKCU
  actual          TEXT NOT NULL DEFAULT 'unknown'
                  CHECK (actual IN ('active','inactive','conflict','missing_file','error','unknown')),
  conflict_ps     TEXT,      -- PS name trùng
  last_error      TEXT,      -- mã có cấu trúc, vd 'ct.105', 'ct.201', 'win.addfont0'
  activated_at    INTEGER,
  verified_at     INTEGER,
  engine_version  INTEGER NOT NULL
);
CREATE INDEX activation_state_scope ON activation_state(scope, actual);
```

**Quy tắc:**
- Khi gỡ, luôn dùng `registered_path` và `scope` đã lưu, không dùng đường dẫn hiện tại.
- Khi file đổi chỗ (rescan thấy cùng hash ở `file_locations` mới): gỡ đường dẫn cũ rồi đăng ký đường dẫn mới.
- Có nút "Tắt mọi font FontAtlas đã kích hoạt" để dọn trước khi gỡ app.

## 5. Rủi ro

| Rủi ro | Mức | Giảm thiểu |
|---|---|---|
| Trùng PS name với font hệ thống, font Adobe Fonts, hoặc RightFont/FontBase | Cao | `preflight` + trạng thái `conflict`; mặc định không kích hoạt đè |
| File di chuyển hoặc ổ ngoài/iCloud bị evict sau khi kích hoạt persistent → font "ma" | Cao | Theo dõi FS + `reconcile` khi khởi động; kiểm tra iCloud đã tải xuống |
| Adobe/Figma không làm mới menu font khi đang mở | Trung bình | Thông báo "Có thể cần mở lại app"; chưa đo được (§1.3) |
| Registry persistent nằm trong thư mục cache của macOS | Trung bình | `reconcile` lần mở app sau (nếu bị dọn); chưa xác minh macOS có dọn hay không |
| Gỡ app để lại entry persistent (mac) hoặc file + HKCU (win) | Trung bình | Mặc định dùng session; có nút dọn; trình gỡ Windows có thể chạy lệnh dọn |
| Windows: file bị khoá khi đang nạp; nhánh HKCU không có tài liệu Microsoft chính thức; GPO chặn | Trung bình | Hàng đợi dọn; thử trên VM trước khi chốt |
| Chưa thử: đăng xuất/restart, hardened runtime/notarize, app thiết kế thật | — | Spike tiếp theo (§1.3) |

## 6. Nguồn

- Apple — CTFontManagerScope (persistent, session): https://developer.apple.com/documentation/coretext/ctfontmanagerscope · https://developer.apple.com/documentation/coretext/ctfontmanagerscope/persistent · https://developer.apple.com/documentation/coretext/ctfontmanagerscope/session
- Apple — CTFontManagerRegisterFontURLs (không di chuyển file sau khi đăng ký): https://developer.apple.com/documentation/coretext/ctfontmanagerregisterfonturls(_:_:_:_:)
- Header `CTFontManager.h` trong crate `objc2-core-text` 0.3.2 (mô tả scope, mã lỗi): https://docs.rs/objc2-core-text/0.3.2
- Microsoft Learn — AddFontResourceExW: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-addfontresourceexw
- Microsoft Learn — Font Installation and Deletion (WM_FONTCHANGE, file bị khoá): https://learn.microsoft.com/en-us/windows/win32/gdi/font-installation-and-deletion
- PSWinGlue Install-Font.ps1 (per-user: LOCALAPPDATA + HKCU, giá trị = đường dẫn đầy đủ, 1809+): https://www.powershellgallery.com/packages/PSWinGlue/0.6.11/Content/Scripts%5CInstall-Font.ps1
- Scoop manifest per-user font: https://github.com/jonz94/scoop-personal/blob/main/bucket/super-ttc-iosevka-aile.json
- Rainmeter issue (AddFontResourceEx + WM_FONTCHANGE không cần admin): https://code.google.com/archive/p/rainmeter/issues/91
- borncity — font per-user, chặn bằng GPO: https://borncity.com/blog/2020/05/15/windows-server-2019-fonts-werden-pro-user-installiert/
- FontBase — Font Activation: https://fontba.se/learn/font-activation · forum: https://forum.fontba.se/t/fonts-deactivating-when-app-is-closed/800
- Adobe Community — kích hoạt font khi không khởi động lại: https://community.adobe.com/t5/illustrator-discussions/activating-fonts-without-restarting/m-p/9182739 · Illustrator 30.3 lỗi auto-activation: https://community.adobe.com/questions-652/illustrator-30-3-typeface-font-manager-auto-activation-not-working-1555906
- Typeface — sự cố kích hoạt: https://typefaceapp.com/help/troubleshooting/activation-issues
- Crate `windows` 0.62.2 (xác minh feature từ mã nguồn): https://docs.rs/windows/0.62.2
