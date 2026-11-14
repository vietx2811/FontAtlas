# Spike B6 — Plugin Figma và link `fontatlas://`

> Người làm: 08 · Ngày: 2026-11-13 · Trạng thái: SPIKE (code thử bỏ đi, `spikes/figma-plugin/`)
> Nền: dev/mvp `1b64934b`. Không cài gì vào Figma thật, không đụng tài khoản Figma của chủ app, không đăng ký URL scheme trên máy này.

## 0. Tóm tắt

1. **Plugin đọc được font đang dùng trong file**, kể cả node có font thiếu, nhưng **không cho biết thẳng font nào thiếu**. Phải tự đối chiếu `getRangeAllFontNames` với `listAvailableFontsAsync`. Đã viết và chạy thử phần gom font bằng mock (`collect.test.js`), chưa chạy trong Figma thật.
2. **Đường chắc chắn nhất để đưa danh sách font sang app: link `fontatlas://` đã mã hoá, hoặc clipboard.** Nút "mở app" từ plugin có rủi ro cao: chưa có tài liệu nào xác nhận `figma.openExternal` mở được scheme khác `https`. Cần xác nhận bằng một plugin thử trong tài khoản Figma riêng, không phải của chủ app (§4).
3. **Plugin gọi `localhost` về mặt kỹ thuật được nhưng không nên làm** (§1.3): lúc phát hành phải khai `localhost` trong `allowedDomains` kèm `reasoning`, bắt buộc app mở cổng, iframe `null` origin nên cần CORS `*`. Rủi ro review, bảo mật và hỗ trợ lớn hơn lợi ích.
4. **Tauri 2: đăng ký scheme trên macOS làm lúc build** bằng `plugins.deep-link.desktop.schemes`, chỉ chạy với bản đã đóng gói và nằm trong `/Applications`; không thử được ở `tauri dev`. Link đến lúc app đã mở và lúc app chưa mở đều về cùng một sự kiện `RunEvent::Opened` (§2).
5. **Đề xuất MVP:** (a) lệnh dán/mở link "Từ Figma" trong app → hộp thoại xem trước → tạo Case từ font có trong thư viện, liệt kê font thiếu; (b) plugin tối thiểu chỉ đọc font rồi sao chép link; (c) nút mở thẳng chỉ thêm nếu §4 xác nhận được. Cần thêm một lệnh backend khớp tên font (§3.3).

## 1. Figma: plugin làm được gì

Nguồn: tài liệu plugin của Figma (developers.figma.com/docs/plugins), đọc ngày 2026-11-13. Chưa chạy thử trong Figma.

### 1.1 Đọc font trong file

| API | Cho gì | Lưu ý |
|---|---|---|
| `TextNode.getRangeAllFontNames(start, end)` | Mọi `{family, style}` trong đoạn | Cách chính để gom font của node, kể cả node có nhiều font |
| `TextNode.fontName` | Một font, hoặc `figma.mixed` | Không dùng riêng được: node nhiều font trả `mixed` (cả khi chỉ khác variation) |
| `TextNode.hasMissingFont` | `true` nếu node có font không khả dụng cho tài liệu | Mức node, không nói font nào |
| `figma.listAvailableFontsAsync()` | Danh sách font khả dụng cho plugin | Tài liệu không nói rõ có gồm font local không; cần thử |
| `TextNode.getStyledTextSegments(fields)` | Các đoạn kèm trường chọn | Tài liệu không liệt kê trường hợp lệ cho font; cần thử |

**Font nào thiếu:** `hasMissingFont` + font thuộc node nhưng không có trong `listAvailableFontsAsync` ⇒ nghi thiếu. Đây là suy luận, cần kiểm trong Figma thật (§4 mục 2). Font thiếu vẫn hiển thị đúng vì Figma lưu đường vẽ của chữ, nhưng không sửa chữ được; tên font vẫn đọc được, nên Font Atlas khớp được tên đó với thư viện.

Truy cập node: manifest `documentAccess: "dynamic-page"` buộc plugin chỉ đọc trang hiện tại, đủ cho MVP (chọn vùng hoặc trang). Quét cả file cần `loadAllPagesAsync` và chậm với file lớn.

### 1.2 Figma lấy font local thế nào

Figma desktop có sẵn **FigmaAgent**, web cần cài "font installer". Đây là tiến trình nền chạy server HTTP và HTTPS trên `localhost` (127.0.0.1), **chỉ chấp nhận kết nối từ figma.com** và không lộ ra Internet. Trên macOS font cài qua Font Book, trên Windows vào thư mục Fonts. Linux và ChromeOS không hỗ trợ font local.

Hệ quả cho Font Atlas:
- Font bật bằng kích hoạt của app (scope session/persistent qua CoreText, spike 04) có thể được Figma thấy sau khi tải lại file hoặc khởi động lại Figma. **Chưa kiểm**; cần thử với tài khoản Figma riêng, và nhớ Figma web khác desktop.
- Plugin không đi qua FigmaAgent. Plugin thấy font nào là do Figma quyết định.

### 1.3 Mạng và CSP của plugin

- `manifest.networkAccess` gồm `allowedDomains` (Figma chặn domain ngoài danh sách), `reasoning`, `devAllowedDomains`. Giá trị `"none"` chặn hết; `"*"` cần `reasoning`.
- Iframe plugin có origin **`null`**, nên chỉ gọi được API trả `Access-Control-Allow-Origin: *`. Vi phạm CSP hiện lỗi trong console.
- Tài liệu chỉ dẫn `http://localhost:3000` cho **`devAllowedDomains`**, kèm câu "chỉ cần cho phát triển thì dùng devAllowedDomains". Nếu cần `localhost` trong bản phát hành thì phải đặt vào `allowedDomains` kèm `reasoning` hiển thị trên trang Community. Tài liệu không nói Figma có từ chối `localhost` khi duyệt hay không.
- Chưa kiểm: chặn mixed content/Private Network Access ở trình duyệt khi plugin chạy trong Figma web.

**Kết luận:** về nguyên tắc gọi được, nhưng app phải chạy server cục bộ có CORS `*`, tức mọi trang web trên máy đều gọi được. Với app xử lý font, đường dẫn và thư viện cá nhân, đó là bề mặt tấn công không đáng. **Không đề xuất.**

### 1.4 Mở app từ plugin

`figma.openExternal(url)` có trong tài liệu ("opens a URL in a new tab"). Không tìm thấy tài liệu hay báo cáo nào xác nhận **scheme tuỳ biến**: một câu hỏi trên diễn đàn Figma (2022, `spotify://`, `window.open` không chạy) không có lời giải. Một số host Electron khác chỉ mở `http(s)`. Coi là **chưa biết, nghiêng về bị chặn**; thiết kế không được phụ thuộc vào nó.

Đường thay thế đã rõ:
1. **Clipboard:** UI plugin sao chép link/JSON (`document.execCommand('copy')` trong iframe; `navigator.clipboard` có thể bị chặn quyền). Trong app có lệnh "Dán từ Figma".
2. **Liên kết trong UI plugin** `<a href="fontatlas://…" target="_blank">`: thử sau §4.
3. **Trang trung gian https**: cần máy chủ của chúng ta; mâu thuẫn với "chạy cục bộ, không gửi gì ra ngoài". Không làm.

## 2. Tauri 2: đăng ký URL scheme trên macOS

Nguồn: tài liệu Tauri 2 (plugin deep-link) và mã nguồn `tauri-bundler`, `plugins-workspace`.

### 2.1 Cấu hình cần thêm
- `Cargo.toml` (app): `tauri-plugin-deep-link = "2"`; `tauri-plugin-single-instance = { version = "2", features = ["deep-link"] }` cho Windows/Linux (macOS đã luôn chỉ một instance).
- `lib.rs`: `.plugin(tauri_plugin_deep_link::init())` (single-instance đăng ký **đầu tiên** trong chuỗi plugin nếu dùng).
- `tauri.conf.json`: `"plugins": { "deep-link": { "desktop": { "schemes": ["fontatlas"] } } }`. Bundler thêm `CFBundleURLTypes`/`CFBundleURLSchemes` vào `Info.plist` khi đóng gói; tên mặc định là `identifier` + scheme (`com.vietx.fontatlas`).
- Capability: `core:event:default` và `deep-link:default` cho cửa sổ chính (`UX/src-tauri/capabilities/default.json`).
- Hiện app **chưa** có plugin nào trong số này (đã kiểm `Cargo.toml`, `lib.rs`, `capabilities/`).

### 2.2 Hành vi trên macOS
- **Không đăng ký lúc chạy được** (`register`/`unregister`/`is_registered` trả `UnsupportedPlatform`). Scheme gắn vào bundle, chỉ có hiệu lực với bản `.app` đã cài (tài liệu: phải ở `/Applications`).
- `tauri dev` **không** nhận link. Cần thử bằng bản đóng gói; vì quy tắc "không chạy bản release cho smoke" (bản release đụng dữ liệu thật, xem `FONTATLAS_DATA_DIR` chỉ có ở debug), cần một đường riêng: gói bản debug trỏ `FONTATLAS_DATA_DIR` vào thư mục tạm. **Việc này đăng ký scheme vào LaunchServices của máy, nên cần Lead hoặc chủ app đồng ý trước; chưa làm.**
- **App đang chạy**: macOS gửi `RunEvent::Opened { urls }`, plugin phát sự kiện `deep-link://new-url`. Nhận bằng `on_open_url` (Rust) hoặc `onOpenUrl` (JS).
- **App chưa chạy**: macOS mở app rồi gửi cùng sự kiện `Opened`. Plugin lưu URL vào `current`, đọc bằng `get_current()`/`getCurrent()`. Mã nguồn không chứng minh sự kiện luôn đến sau khi frontend sẵn sàng, nên **frontend phải đọc `getCurrent()` lúc khởi động rồi mới đăng ký `onOpenUrl`**, và backend nên giữ hàng đợi link tới khi giao diện báo sẵn sàng. Cần thử thật (§4 mục 3).
- Tài liệu cảnh báo: người dùng có thể giả link bằng tham số dòng lệnh; plugin chỉ khớp scheme đã cấu hình nhưng **phải tự kiểm định dạng URL** (§3.4).

### 2.3 Hai scheme: `fontatlas://` (phát hành) và `fontatlas-dev://` (debug, CI, QA)
Vấn đề: macOS đăng ký scheme của mọi `.app` nó thấy (kể cả khi chỉ mở bản debug ở một thư mục tạm). Nếu bản debug cũng khai `fontatlas://`, link từ Figma có thể mở một bản debug cũ trong thư mục CI thay vì app đã cài.

Quy ước (đã làm):
- `UX/src-tauri/tauri.conf.json` (mặc định, dùng cho `tauri dev`, `cargo tauri build --debug`, CI, QA, smoke) khai **`fontatlas-dev`**.
- `UX/src-tauri/tauri.release.conf.json` là overlay chỉ đổi `plugins.deep-link.desktop.schemes` thành **`fontatlas`**. Bản phát hành build bằng `cargo tauri build --config src-tauri/tauri.release.conf.json` (`npm run tauri:build` đã có cờ này). Quên cờ thì bản phát hành mất scheme (hỏng an toàn, dễ thấy), không bao giờ bản debug giành `fontatlas://`.
- Backend (`services/figma::LINK_SCHEME`) chọn theo `debug_assertions`: debug = `fontatlas-dev`, release = `fontatlas`; mỗi build chỉ nhận scheme của mình và bỏ link của build kia.
- `tools/release/make_dmg.sh` từ chối đóng DMG nếu `Info.plist` của app không khai đúng `fontatlas`.
- Plugin Figma thử có ô "Bản dev" để sinh link `fontatlas-dev://`.

Dọn đăng ký cũ (tay, không tự động xoá gì): chỉ ĐỌC bằng
```bash
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -B2 -A12 "fontatlas"
```
để xem những bundle nào đang giữ `fontatlas` hoặc `fontatlas-dev`, và `Font Atlas.app` nào nằm ở thư mục tạm của CI. Muốn gỡ một bản cụ thể thì chủ máy tự chạy `lsregister -u "<đường dẫn .app>"` cho đúng bản đó (hoặc xoá thư mục bản debug cũ rồi `lsregister -gc`); không có script nào của dự án làm việc này.

## 3. Luồng tối thiểu đề xuất

### 3.1 Hai link
| Link | Ý nghĩa |
|---|---|
| `fontatlas://font?family=Playfair%20Display&style=Bold` | Mở thư viện tới đúng font (Figma → một font) |
| `fontatlas://fonts?v=1&d=<base64url JSON>` | Danh sách font của file; `d` = `{v, src:"figma", file, fonts:[[family, style, missing01], …]}` |

Kích thước: 60 font ≈ 3,3 KB (đo bằng `collect.test.js`), nằm xa giới hạn URL của macOS. Đặt trần cứng 200 font/link; quá thì dùng clipboard.

### 3.2 Luồng người dùng
1. Trong Figma, chọn khung hoặc trang rồi chạy plugin "Font Atlas". Plugin liệt kê font dùng và gắn nhãn thiếu.
2. Hai nút: **Mở trong Font Atlas** (liên kết, nếu §4 mục 1 chạy) và **Sao chép** (luôn chạy).
3. Font Atlas nhận link hoặc "Dán từ Figma". Hộp thoại **xem trước, không tự làm gì**: "12 font trong file Menu v3: 9 có trong thư viện, 3 không có".
4. Nút chính **Tạo Case từ file này** (tên `Figma: Menu v3`, nguồn gốc ghi `origin`). Font thiếu hiển thị bằng NoteMark, kèm hành động "Tìm font tương tự" (đã có `similar`) và "Kích hoạt" cho font có trong thư viện nhưng đang tắt.
5. Link một font mở thẳng chi tiết font; nếu không khớp thì mở ô tìm kiếm với tên đó.

### 3.3 Việc backend cần
- Lệnh mới `resolve_fonts(items: [{family, style}]) -> [{item, face?: FaceKeyDto, candidates: n}]`. Khớp theo family + style đã chuẩn hoá (không phân biệt hoa thường, bỏ khoảng trắng thừa) với `FaceProfile`; nhiều face cùng tên (nhiều bản, nhiều nguồn) thì trả ứng viên đã xếp hạng, để UI cho chọn. Tên font Figma có thể khác tên đầy đủ trong file font; thử cả family theo ngôn ngữ tiếng Anh và PostScript name.
- Tạo Case dùng lệnh sẵn có `create_case` + `add_to_case(origin)`. Cần thêm giá trị nguồn gốc `figma` cho `CaseOriginDto` (chỉ chuỗi).
- Chuỗi lỗi/thông báo mới theo mã (`err.<mã>`), có vi + en.

### 3.4 An toàn và riêng tư
- Chỉ nhận scheme `fontatlas`, host trong danh sách (`font`, `fonts`), giải mã chặt (độ dài tối đa, số font tối đa, UTF-8 hợp lệ, bỏ tham số lạ).
- **Không bao giờ thực hiện hành động chỉ vì có link**: luôn qua hộp thoại xem trước. Link từ trình duyệt hay tin nhắn có thể do bất kỳ ai tạo.
- Link không chứa đường dẫn, không kích hoạt font, không xoá gì.
- Tên file Figma là dữ liệu riêng tư của người dùng; chỉ nằm trong link cục bộ, không ghi log, không gửi mạng. Cho phép bỏ tên file.
- Plugin dùng `networkAccess: none`: dễ qua duyệt và là lời hứa "không gửi gì ra ngoài" kiểm chứng được.

## 4. Cần kiểm thật (chưa làm) và rủi ro

Cần một tài khoản Figma thử riêng (không phải của chủ app) và một bản `.app` debug đóng gói với dữ liệu tạm.

| # | Kiểm | Vì sao |
|---|---|---|
| 1 | `figma.openExternal('fontatlas://…')` và `<a target=_blank>` có mở được app không (Figma desktop và web) | Quyết định có nút "Mở app" hay chỉ clipboard |
| 2 | `hasMissingFont` + `listAvailableFontsAsync` có phân biệt được font thiếu không; `listAvailableFontsAsync` có chứa font local không | Quyết định độ chính xác của "font thiếu" |
| 3 | macOS: mở link khi app chưa chạy; thứ tự `Opened` so với frontend sẵn sàng | Tránh mất link lần đầu |
| 4 | Font bật bằng kích hoạt của app có xuất hiện trong Figma desktop/web không, cần làm mới gì | Làm cho "Kích hoạt" từ hộp thoại có ý nghĩa với Figma |

Rủi ro:
- **Phát hành plugin trên Figma Community** cần duyệt và dịch vụ của Figma; có thể phải cập nhật khi API đổi. Với MVP có thể chỉ dùng plugin phát triển cục bộ (nhập manifest), không cần Community.
- Tên font Figma ≠ tên thư viện (Google Fonts đặt tên khác, font Adobe, font người dùng đổi tên): sẽ có tỉ lệ không khớp; phải hiển thị thật thà ("không khớp chắc") thay vì đoán.
- Windows: scheme gắn qua registry (NSIS/MSI cấu hình sẵn) và link tới lần chạy mới qua tham số dòng lệnh; cần single-instance. Chưa thử.
- Mở rộng sau MVP: nhận link từ Sketch, Adobe (cần plugin riêng mỗi app), lúc đó giao thức `fonts?v=1` giữ nguyên, thêm `src`.

## 5. Mã thử
`spikes/figma-plugin/`: `manifest.json` (placeholder, `networkAccess: none`, không phải bản cài), `collect.js` (gom font, mã hoá link), `collect.test.js` (mock, `node collect.test.js` chạy xanh). Bỏ đi sau spike.

## 6. Việc tiếp theo (xin Lead quyết)
1. Cho phép tạo tài khoản Figma thử và bản debug đóng gói để làm §4 (đây là bước ngoài quy tắc hiện tại: đăng ký scheme vào LaunchServices và dùng dịch vụ ngoài; cần chủ app đồng ý).
2. Nếu đồng ý hướng đi: 01 viết spec UX cho hộp thoại xem trước; 08 làm `resolve_fonts` + xử lý link (deep-link plugin + hàng đợi khởi động).
