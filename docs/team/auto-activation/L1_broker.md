# L1 — Activation broker (0.3.0)

Nhánh `feat/auto-activation`. Crate mới `fontatlas-broker`; nối vào app ở `services/broker.rs`.

## Làm gì

App thiết kế (helper font request, panel CEP, startup script) hỏi "font này có không, bật giúp". Broker khớp với thư viện, bật scope **session**, ghi lý do `auto:<app>`, trả `found`/`missing`.

- Chỉ nghe `127.0.0.1`, cổng động. Mở sau boot (sau reconcile), đóng ở `on_exit`. Engine không hỗ trợ (Windows stub) thì không mở.
- Client đọc `broker.json` trong **data dir** của app (`AppDirs.data`, macOS: `~/Library/Application Support/<bundle id>/`):
  `{"v":1,"port":51234,"token":"<64 hex>","pid":123}`. Quyền 0600, ghi nguyên tử, xoá lúc thoát. Mỗi lần khởi động token đổi.
- Công tắc: setting `auto_activation_enabled` (`"0"` = tắt, thiếu = bật). Tắt → 503.

## Giao thức v1

Mọi yêu cầu cần `Authorization: Bearer <token>` (trừ `OPTIONS`).

`GET /v1/ping` → `{"ok":true,"v":1}`.

`POST /v1/ensure`
```json
{"v":1,"app":"Adobe Photoshop 2026","doc":"poster.psd",
 "fonts":[{"ps":"LibreBaskerville-Regular"},{"family":"Libre Baskerville","style":"Bold"}]}
```
→
```json
{"ok":true,"v":1,"app":"Adobe Photoshop 2026",
 "results":[{"name":"LibreBaskerville-Regular","status":"activated"},
            {"name":"Libre Baskerville / Bold","status":"missing"}],
 "found":["LibreBaskerville-Regular"],"missing":["Libre Baskerville / Bold"]}
```

- Khớp: `ps` đúng từng ký tự trước; không có thì `family` (gập dấu, không phân biệt hoa thường) + `style` (bỏ khoảng trắng/gạch, rỗng = Regular). Không đoán gần đúng.
- `app`: nhận tên hiển thị hoặc bundle id; ánh xạ về `photoshop | aftereffects | illustrator | indesign | incopy | sketch | affinity | figma | other`.
- `status`: `activated` (vừa bật) · `already_active` · `available` (máy đã có đúng file này do bên khác cung cấp) · `missing` (thư viện không có) · `conflict` (máy có font cùng tên nhưng KHÁC nội dung, không tự ghi đè) · `unavailable` (file mất / đã đổi) · `not_downloaded` (file placeholder OneDrive/iCloud chưa tải xong trong hạn chờ; tải vẫn chạy ngầm, hỏi lại sau) · `failed` · `busy` (hỏi lại sau) · `invalid` (yêu cầu rỗng).
- `path` (tuỳ chọn, thêm sau): đường dẫn file font cho kết quả dùng được. Dành cho nguồn font request (L2, xem `L2_fontreq.md`); panel/script có thể bỏ qua.
- `found` = `activated | already_active | available`. Phần còn lại vào `missing`; lý do ở `results`.
- Giới hạn: ≤ 256 font, mỗi chuỗi ≤ 512 ký tự không có ký tự điều khiển, thân ≤ 128 KiB.

Mã lỗi HTTP: 400 (`bad_json`, `bad_request`) · 401 `unauthorized` · 403 `bad_host`/`bad_origin` · 404 · 405 · 413 · 429 (kèm `Retry-After: 1`) · 503 `disabled` · 500 `internal`. Thân lỗi: `{"ok":false,"error":"<mã>","message":"..."}`.

## Bảo vệ

- Token 256 bit từ hệ điều hành, so sánh thời gian cố định.
- Host phải là `127.0.0.1:<port>` hoặc `localhost:<port>` (chống DNS rebinding).
- Origin: chỉ `null` và `file://` (panel CEP gửi `Origin: null`) và danh sách bổ sung do app cấu hình (mặc định rỗng). Trang web (`http(s)://…`, `chrome-extension://…`) bị 403 dù có token. `OPTIONS` preflight chỉ đáp cho origin được phép.
- Giới hạn tần suất: xô 60 yêu cầu, nạp 30/giây, một xô cho cả server.
- Xung đột (`conflict`) không bao giờ tự ghi đè: không có ai để hỏi lúc app thiết kế đang mở file.

## Dữ liệu

- `activation_reasons.reason` nhận thêm `auto:<app>` (user.db **v8** (sau taxonomy-v2 = v7), dựng lại bảng vì SQLite không sửa được CHECK; có backup `user.v7.<ms>.db` trước migrate, giữ dữ liệu và index).
- Bảng mới `auto_activation_log(at, app, doc, requested, outcome, hash)`: giữ 500 dòng mới nhất; không ghi `already_active`/`available`/`invalid`; cùng (app, tên, kết quả) trong 60 s chỉ ghi một lần (Photoshop hỏi lại mỗi lần vẽ chữ).
- Tắt tay một font (Manual) cũng gỡ mọi lý do `auto:*` của nó. Gỡ collection mà font còn `auto:*` thì `KeptActive{manual:false, collectionIds:[]}`: UI hiểu là "đang có app dùng".
- `ReasonDto` thêm `{"kind":"auto","app":"photoshop"}` (bindings đã sinh lại).

## Chưa làm ở L1

- Panel CEP (L3), startup script (L4), trạng thái từng app + lệnh Tauri cho Settings (L5).
- Không có lệnh Tauri nào mới: UI chưa thấy gì ngoài `reasons` mới trong `activation_state`.
- Chưa có bài thử trên CoreText thật qua broker (test dùng `FakeEngine`; hành vi đăng ký session thật đã đo ở spike 06).


## File placeholder (dataless): OneDrive / iCloud / FileProvider

Thư viện thật của chủ app nằm dưới `~/Library/CloudStorage/` (OneDrive), nhiều file chỉ là placeholder chưa tải. Việc tải do `fontatlas_activate::ensure_local` (worker 03, `feat/cloud-fonts`) làm; broker chỉ chọn hạn chờ.

- `ActivationService::one_file` gọi `ensure_local` trước khi đăng ký. Yêu cầu từ app (lý do `auto:*`) dùng đường riêng `ensure_local_auto` với `AUTO_DOWNLOAD_TIMEOUT` = 8 s (kích hoạt thủ công vẫn `CLOUD_DOWNLOAD_TIMEOUT` = 30 s), vì app thiết kế đang đứng đợi trong callback. Đổi được bằng `with_auto_ensure_local`.
- Kết quả broker: tải xong thì bật bình thường; `cloud_timeout` hoặc chỉ có stub `.icloud` → `not_downloaded` (không phải lỗi cứng, tải vẫn có thể tiếp tục, hỏi lại sau); `missing_file` → `unavailable`; `cloud_download_failed` → `failed`.
- Test: `auto_request_uses_its_own_short_download_path` (đường auto, không đụng đường thường, hết hạn thì không đăng ký gì); các test dataless của 03 cho phần còn lại.
- CHƯA đo: Photoshop chờ callback CoreText được bao lâu (8 s là đoán, đo ở L2 với helper), và tải thật từ OneDrive (chưa chạm thư viện thật).
