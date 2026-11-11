# L2 — Nguồn font request (Photoshop, After Effects, …) (0.3.0)

Nhánh `feat/auto-activation`. Crate mới `fontatlas-fontreq`; nối vào app ở `services/fontreq.rs`. Nền tảng: spike 06-1 (`docs/team/spikes/06_helper_photoshop_affinity.md`).

## Làm gì

Photoshop 2026 (và app khác theo cơ chế CoreText) hỏi font còn thiếu qua `CTFontManagerCreateFontRequestRunLoopSource`: nguồn nào đã đăng ký được gọi với thuộc tính font + pid app hỏi, trả mảng `CFURL` của file font, hệ thống cấp quyền đọc cho app hỏi. Font Atlas đăng ký một nguồn như vậy; khi nhận request:

1. Tên nội bộ/hệ thống (`AdobeSansMM`, `AdobeSerifMM`, `AdobeInvis*`, `AdobeBlank*`, `LastResort`, tên bắt đầu bằng `.`, rỗng, có ký tự điều khiển) → trả mảng rỗng ngay, không chạm thư viện.
2. pid → tên app (`proc_pidpath` → tên gói `.app` ngoài cùng, vd "Adobe Photoshop 2026") → broker ánh xạ `photoshop | aftereffects | …`.
3. Hỏi thư viện bằng đúng đường của broker (`ensure_in_library`, không qua HTTP): khớp PostScript name (family + style nếu app gửi), bật **session**, lý do `auto:<app>`, ghi nhật ký.
4. Trả `CFURL` của file (`results[0].path`). Không có → mảng rỗng.

## Quyết định thiết kế

- **Chạy trong MỘT LUỒNG của app, không phải binary riêng.** App không sandbox; cơ chế không phụ thuộc process nào đăng ký nguồn (đã kiểm: process khác hỏi, nguồn trong process test trả lời, `crates/fontatlas-fontreq/tests/coretext_e2e.rs`). Tránh đóng gói/ký/notarize thêm binary và giám sát tiến trình; gọi thẳng backend nên không cần token. Callback bọc `catch_unwind`: lỗi của ta thành "không có font", không sập app. Logic (`Responder`) tách khỏi CoreText nên chuyển sang binary riêng được nếu cần.
- **Điều kiện Lead đặt khi chấp thuận (2026-11-11)**: (1) luồng riêng với CFRunLoop riêng, không dùng run loop chính của Tauri; (2) callback không giữ app hỏi quá hạn: tải đám mây chạy với `AUTO_DOWNLOAD_TIMEOUT` 8 s và `Responder` có hạn cứng `ANSWER_DEADLINE` 10 s (chạy `ensure` ở luồng phụ, quá hạn/panic → trả rỗng, việc bật vẫn chạy nốt ở nền); (3) `catch_unwind` + log, tắt sạch khi thoát: `on_exit` đóng nguồn TRƯỚC (huỷ source, chờ luồng), rồi broker, rồi `activation::on_exit` gỡ font session; (4) logic thuần tách khỏi CoreText (`Responder`); (5) broker HTTP vẫn giữ cho panel Illustrator và script InDesign.
- **Công tắc** = setting `auto_activation_enabled` (cùng broker), đọc MỖI request: tắt/bật có hiệu lực ngay, không đóng/mở nguồn. Tắt → mảng rỗng (không hỏi thư viện, không ghi gì).
- **Nhớ "không có" 10 s** (`NEGATIVE_TTL`) theo (ps, family, style): Photoshop hỏi lại mỗi lần dựng chữ. Chỉ nhớ khi thư viện nói `missing`; `busy`, `not_downloaded`, `failed`, `unavailable`, `conflict` được hỏi lại lần sau. Hết hạn thì hỏi lại để font mới thêm vào thư viện được thấy.
- **Không phục vụ chính mình** (pid = pid của app).
- **Chỉ mở khi dữ liệu là thật**: nếu `FONTATLAS_DATA_DIR` được đặt (QA, test, thử máy) thì KHÔNG mở nguồn, vì nguồn là của cả phiên đăng nhập và một thư viện tạm không được trả lời thay thư viện thật. Ép mở bằng `FONTATLAS_FONTREQ=1`.
- Trả đường dẫn: `FontResult.path` mới ở giao thức broker v1 (thêm, tương thích; chỉ có khi trạng thái dùng được: `activated | already_active | available`; với `available` là file của bên khác đang cung cấp).
- Ghi log (`log::debug`; `warn` nếu một request trả lời > 3 s): pid, tên, có/không, thời gian. Dùng để đo Photoshop chờ bao lâu.

## Kiểm

- `tests/responder.rs`: lọc tên, ánh xạ pid→app, nhớ/không nhớ theo trạng thái, công tắc, lỗi, request rỗng, không phục vụ chính mình, family/style, tên app từ đường dẫn.
- `tests/coretext_e2e.rs` (macOS, CoreText thật): nguồn trong tiến trình test, `examples/requester` (tiến trình khác) hỏi `LibreBaskerville-Regular` → nhận đúng font (`got=`, có URL); hỏi tên lạ → không nhận; thời gian trả lời < 2 s; mở/đóng/mở lại nguồn được. Chỉ trả URL fixture, không đăng ký font nào vào hệ thống.
- `crates/fontatlas-broker/tests/ensure.rs`: `path` có cho kết quả dùng được (cả `already_active`), không có cho `missing`; không lộ khi không dùng được.

## Quan sát lạ (cần biết)

- Một lần trong ~6 lần chạy `coretext_e2e`, tiến trình hỏi font đứng 10 phút ở `TAutoActivationManager::CopyFontsFromFontProvider` (XPC đồng bộ tới dịch vụ font của hệ thống) trong khi luồng nguồn của ta rỗi trong runloop: hệ thống KHÔNG gọi callback của ta. Chạy lại 4 lần đều ổn, chưa tìm ra nguyên nhân (nghi hệ thống đang chờ một nguồn font request khác, hoặc cache của fontd). Ý nghĩa: nếu có thật ngoài đời thì app thiết kế bị treo do hệ thống chứ không do ta; ta không kiểm soát được. Test đã đặt hạn 30 s cho tiến trình hỏi để không treo CI. Nên để ý khi đo với Photoshop thật.

## Chưa đo / chưa làm

- **Photoshop chờ callback bao lâu** trước khi bỏ cuộc: chưa đo (cần Photoshop thật + tài liệu thiếu font; không dùng osascript lúc này). `AUTO_DOWNLOAD_TIMEOUT` = 8 s vẫn là phỏng đoán. Cách đo: chạy app với `FONTATLAS_FONTREQ=1`, một font placeholder (dataless) trong thư viện, mở tài liệu Photoshop dùng font đó, đọc log `font request ... (took)` và quan sát Photoshop có hiển thị đúng không khi 2, 5, 8, 15 s.
- Chưa chạy lại với Photoshop thật qua nguồn trong app (spike 06-1 đã chứng minh cơ chế bằng helper riêng).
- After Effects, Affinity: chưa đo (xem spike). Nhiều nguồn cùng trả (FontBase/Typeface đang chạy): chưa thử.
- Trạng thái "nguồn đang chạy / lỗi mở" cho Settings sẽ thuộc L5 (`AppState.fontreq.is_some()`).
