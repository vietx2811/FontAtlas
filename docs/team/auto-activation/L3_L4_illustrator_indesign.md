# L3 + L4 — Illustrator (panel CEP) và InDesign/InCopy (startup script) (0.3.0)

Nhánh `feat/auto-activation`. Crate `fontatlas-cep` (L3) và `fontatlas-indesign` (L4); dịch vụ app ở `services/cep.rs`, `services/indesign.rs` (lệnh Tauri cho Settings thuộc L5). Cả hai gọi broker HTTP của L1 (`POST /v1/ensure`, token trong `broker.json`).

## L3 — Illustrator: panel CEP nền

- **Chạy trong tiến trình Illustrator**: không Apple Events, không hộp Allow (spike 06-3). Panel ẩn (`AutoVisible=false`, `Type=Custom`) tự nạp lúc Illustrator khởi động; manifest theo mẫu của panel nền thật (Version 8.0, HostList trong `<Extension>`, `StartOn` đủ 4 sự kiện, `--mixed-context`).
- `panel/jsx/host.jsx`: `faSig()` (chữ ký RẺ của tập tài liệu) và `faScan(reset, budgetMs)`: quét từng **text run** của mỗi text frame (mỗi run có thể một font riêng, bắt được font thứ hai trong cùng frame), giữ font KHÔNG có trong `app.textFonts`. Quét theo lát ≤ 700 ms rồi nhả luồng chính (tiếp tục bằng lần gọi sau, con trỏ giữ trong `$.global`). ExtendScript là ES3 không có `JSON` nên tự dựng chuỗi JSON (`faQ`). Không sửa tài liệu, không hộp thoại.
- `panel/main.js`: dựa vào SỰ KIỆN tài liệu (gộp 400 ms); poll dự phòng thưa 20 s chỉ hỏi `faSig()` và chỉ quét khi chữ ký đổi; mỗi tài liệu chỉ gửi lại khi tập font đổi; `POST /v1/ensure` (`app:"Adobe Illustrator"`); `busy`/`not_downloaded`/`failed` thì hỏi lại sau 5 s (tối đa 6 lần); có font bật được thì gọi `app.redraw()`. Font Atlas chưa chạy → không có `broker.json` → im lặng, lượt poll sau thử lại. Đọc `broker.json` mỗi lần (cổng + token đổi mỗi lần Font Atlas khởi động). Đường dẫn `broker.json` nằm trong `fontatlas.json` do bộ cài ghi vào gói (đã ký).
- **Ký gói tự ký, không cần PlayerDebugMode (giả thuyết, xem "Chưa đo")**: máy này không có ZXPSignCmd (và tải về cần chủ máy đồng ý), nhưng có sẵn một panel nền thật (`Logi Plugin for Illustrator` trong `/Library/Application Support/Adobe/CEP/extensions`) ký bằng chứng chỉ TỰ KÝ, không timestamp, RSA-SHA1 trên C14N, digest manifest SHA-256. Ta tự viết bộ ký bằng Rust theo đúng định dạng đó. Đã đối chiếu: `tests/format.rs::canonical_manifest_reproduces_a_real_signed_panel` dựng lại DigestValue của manifest Logi **trùng từng byte**; `signature_verifies_with_openssl_the_way_adobe_would` verify chữ ký của ta bằng openssl như Adobe sẽ làm; sửa/đổi/thêm/bớt tệp, chứng chỉ khác, chữ ký hỏng đều bị phát hiện.
- **Khoá**: sinh riêng cho từng máy lúc cài đầu tiên (RSA 2048, chứng chỉ tự ký 2024–2060), lưu `<data dir>/cep/signing.pem` (0600) + `signing.cert.der`; dùng lại cho các lần cài sau. Không có khoá bí mật nào trong repo hay trong app. Hệ quả: gói đã cài chỉ hợp lệ với khoá của máy đó; mất khoá thì `status` báo `Stale`, cài lại là xong.
- **Khoá ký được GIỮ** ở `<data dir>/cep/signing.pem` (0600) cho đến khi người dùng xoá (Settings: `auto_forget_key`, chỉ khi panel đã gỡ; `AutoStatusDto.signingKeyPresent`). Không dùng Keychain.
- **Cài / gỡ / trạng thái** (`PanelInstaller`): đích `~/Library/Application Support/Adobe/CEP/extensions/com.fontatlas.autoactivate`. Ghi vào thư mục tạm cùng cha rồi thay nguyên tử (bản cũ sang `.old-<pid>`, bản mới vào, xoá bản cũ; lỗi giữa chừng trả bản cũ về); `install` và `remove` đều dọn `.tmp-*`/`.old-*` sót lại của chính ta; không bao giờ đè/xoá thư mục cùng tên mà manifest không khai `ExtensionBundleId` của ta (`Foreign`). `status`: `NotInstalled | Installed{version} | Stale{reason} | Foreign`; `Stale` khi khác phiên bản app, `broker.json` chuyển chỗ, nội dung panel cũ, tệp bị sửa/thêm, hoặc chữ ký của khoá khác. `install` trả `restart_illustrator: true` (manifest chỉ đọc lúc khởi động).
- Ví dụ chạy tay: `cargo run -p fontatlas-cep --example panel -- <install|remove|status> <state_dir> <broker.json> [extensions_dir]`.

## L4 — InDesign / InCopy: startup script

- `script/FontAtlas_Startup.jsx` (`#targetengine "fontatlas"`): nghe `afterOpen`, đọc `doc.fonts` với `Font.status`, font nào khác `INSTALLED` thì `POST /v1/ensure` qua `Socket` BINARY tới `127.0.0.1` (UTF-8 an toàn), đợi tối đa **3 s** (không giữ InDesign lâu; Font Atlas tải file đám mây chạy nốt ở nền, lần mở sau có ngay). `broker.json` được đọc bằng regex có kiểm (cổng là số 1–65535, token đúng 64 ký tự hex), KHÔNG `eval`. Không sửa tài liệu, không hộp thoại, lỗi thì im lặng. Chạy lại script thì gỡ listener cũ (targetengine bền).
- Không cần ký, không Automation, không PlayerDebugMode (đã chạy trên InDesign 2026: spike 06-2 "Kết quả đêm 2026-11-11").
- **Cài vào đâu** (`StartupInstaller`): `~/Library/Preferences/Adobe InDesign|InCopy/Version <N>.0/<locale>/Scripts/startup scripts/FontAtlas_Startup.jsx` cho MỌI phiên bản và locale ĐÃ có thư mục (app đã chạy ít nhất một lần). Không đoán locale cho bản chưa chạy: bản đó không có mục trong `targets()`, UI nên nói "mở InDesign một lần rồi cài lại". Máy này hiện có `Version 20.0` (rỗng), `21.0/en_US`, `22.0` (rỗng, InDesign 2027 chưa chạy xong lần đầu).
- Nhận tệp của mình bằng dòng đầu `// FontAtlas startup script <ver>`; tệp cùng tên không phải của ta (`Foreign`) không bị đè hay xoá, và một nơi lỗi không chặn nơi khác. `status` theo từng nơi: `NotInstalled | Installed | Stale | Foreign` (so băm SHA-256 với script sẽ cài: khác phiên bản, khác đường dẫn `broker.json`, hoặc bị sửa → `Stale`).
- Đường dẫn `broker.json` được ghi cứng vào script lúc cài (đúng thư mục dữ liệu thật của app, không phải `~/Library/Application Support/FontAtlas/` của spike); đổi thư mục dữ liệu → `Stale` → cài lại.
- InDesign đọc startup script lúc khởi động: đang chạy thì phải khởi động lại.

## Kiểm đã làm

- `fontatlas-cep`: `tests/format.rs` (3), `tests/install.rs` (5: chu trình cài/trạng thái/gỡ, khoá 0600 và dùng lại, Stale các kiểu, không đụng thư mục lạ, thư mục tạm sót lại); clippy sạch.
- `fontatlas-indesign`: `tests/install.rs` (6: tìm đúng phiên bản có locale, cài/trạng thái/gỡ, thoát ký tự đường dẫn, Stale, tệp lạ, không có Adobe); clippy sạch.

## Chưa đo (cần Adobe thật chạy được; ghi trung thực)

- **Illustrator nạp panel tự ký CỦA TA khi KHÔNG bật PlayerDebugMode?** Chưa thử. Cơ sở: key `PlayerDebugMode` không tồn tại trên máy này (`defaults read com.adobe.CSXS.12` chỉ có `LogLevel`) mà panel Logi (tự ký) nạp bình thường; định dạng chữ ký của ta trùng định dạng đó. Lần thử bị chặn: cài panel vào thư mục extensions của Adobe là "cài thường trú" và chủ máy chưa cho phép. Nếu hoá ra bắt buộc PlayerDebugMode: dừng, báo Lead trước khi đụng `defaults`.
- Illustrator hiện trên máy là **2027 (31.0)**, không còn 2026 (30.7) như spike; có `CEPHtmlEngine` nhưng chưa xác nhận CEP 2027 nạp panel nền.
- `faFonts()` với font thiếu ở Illustrator 2027: spike (30.7) thấy `textFont.name` giữ tên gốc; chưa kiểm ở 2027 và chưa kiểm rằng font thiếu KHÔNG nằm trong `app.textFonts`.
- Sau khi Font Atlas bật font, tài liệu đang mở có hiển thị lại đúng không (`app.redraw()` có đủ không, hay phải mở lại tài liệu)? Hộp "Missing Fonts" của Adobe và hộp của RightFont (đang chạy ở máy chủ) hiện ngay lúc mở, trước khi panel kịp bật font: chưa biết Adobe có tự bỏ qua sau khi ta bật.
- InDesign: `afterOpen` có chạy trước hộp Missing Fonts không; `Font.status`/tên gốc; Socket tới broker; InDesign có xoá "missing" sau khi bật session không. InDesign trên máy này kẹt ở màn hình trắng lúc khởi động (cả 2026 lẫn 2027 chưa vào được), nên chưa đo được gì ngoài việc script nạp được.

## L3 — log chẩn đoán của panel (thêm 2026-11-23)

Lần thử thật trên Illustrator 2027: panel nạp được (không cần PlayerDebugMode) nhưng KHÔNG gửi `ensure` cho tài liệu có font thiếu, và không có dấu vết nào. Nên panel nay ghi log ra **`cep-panel.log` cạnh `broker.json`** (thư mục dữ liệu của app), tối đa 12 000 ký tự (giữ phần cuối), chỉ gồm mã lỗi, số đếm và TÊN FONT, không bao giờ tên/đường dẫn tài liệu. Đọc nó sau lần thử để biết panel dừng ở đâu:

- `panel nền đã nạp` → panel chạy. Không có dòng này = panel không nạp.
- `faSig lỗi: 'EvalScript error.'` rồi `nạp host.jsx bằng tay` → ScriptPath không chạy cho panel nền; panel tự `$.evalFile` host.jsx một lần.
- `quét xong: tài liệu=… frame=… run=… runKhôngFont=… lỗi=… fontCài=… thấy=[tên(có|thiếu),…] lỗiMẫu=…` — dòng quan trọng nhất:
  - `thấy=[ArialMT(có)]` dù tài liệu dùng font khác ⇒ `textFont.name` KHÔNG giữ tên gốc của font thiếu (Illustrator thay thế), phải tìm cách khác để lấy tên font thiếu.
  - `lỗiMẫu=textFont:…` ⇒ đọc `textFont` của font thiếu ném lỗi (trước đây bị nuốt im lặng).
  - `runKhôngFont>0` ⇒ `textFont` trả null.
  - `frame=0`/`run=0` dù có chữ ⇒ tài liệu import không lộ text frame qua `textFrames`.
- `chưa có broker.json` / `broker.json: …` ⇒ Font Atlas chưa chạy hoặc tệp hỏng. (Lỗi đã sửa: trước đây panel đặt chữ ký là "đã xử lý" nên poll không bao giờ quét lại sau khi Font Atlas khởi động.)
- `gửi ensure: font thiếu=…` rồi `ensure xong: bật=… chưa có=…` hoặc `ensure lỗi http=…` ⇒ đường tới broker.

Test không cần Illustrator: `node --test crates/fontatlas-cep/jstests/panel.test.mjs` (cũng chạy trong `cargo test -p fontatlas-cep`): host.jsx và main.js chạy trong vm của Node với `app` giả (font thiếu, textFont ném lỗi, font bị thay thế như tài liệu import, textFrames lỗi), đồng hồ giả cho vòng poll 20 s và sự kiện, broker giả.

### Kết quả lần thử thật 2026-11-23 (Illustrator 2027, tài liệu SVG import)

- Panel tự ký nạp được KHÔNG cần PlayerDebugMode; `host.jsx` nạp bình thường (không `EvalScript error`).
- **Nguyên nhân panel không gửi `ensure`: `textRange.textRuns` trả 0 run trong Illustrator 2027 dù text frame có chữ** (log: `frame=1 run=0`, không lỗi). Code cũ chỉ đọc font qua textRuns nên không bao giờ thấy font nào (cả font có lẫn font thiếu).
- Sửa: nếu textRuns rỗng/`length` không phải số thì đọc từng paragraph, rồi cả `textRange`; log ghi `qua[run/đoạn/frame]` và `dò{runs=0 paras=1 via=paras}`. Lần thử sau sửa: `run=1 qua[run/đoạn/frame]=0/1/0 thấy=[DMSerifDisplay-Regular(có)]`.
- Chưa kiểm được trọn vòng "font thiếu → panel gửi → font bật" trong Illustrator: app.textFonts có vẻ giữ tên font vài chục giây sau khi font bị tắt (nên font vừa tắt vẫn "có"), và tài liệu import SVG thay font thiếu bằng font khác. Cần tài liệu .ai gốc được lưu với font rồi mở khi font đang tắt.

### Sau review (2026-11-23): các điểm đã sửa trong panel

- **Broker đổi (Font Atlas khởi động lại):** panel nhớ `port:token` lần đọc trước; đổi thì xoá `last/retry/lastSig` rồi quét lại mọi tài liệu (log: `broker đổi …`). Cần vì lượt startup của app xoá auto-on phiên cũ, nên tài liệu đang mở lại thiếu font.
- **Broker vắng:** không quét (không có chỗ gửi) và không ghi log lặp; broker.json xuất hiện/đổi thì quét ngay.
- **Log "quét xong"** chỉ ghi khi kết quả khác lần trước.
- **Đoạn/vùng chữ trộn nhiều font:** **luôn** lấy mẫu từng ký tự (kể cả khi `textFont` của đoạn/vùng có giá trị: Illustrator 2027 thật trả font ĐẦU TIÊN cho đoạn trộn, đo ngày 2026-11-24 với đoạn Chonburi/DM Serif/Arial) (tối đa ~200 lần đọc mỗi mục, luôn gồm ký tự cuối); log `qua[run/đoạn/frame/ký tự]`.
- **Nạp tay host.jsx:** bộ đếm tối đa 3 lần cho một chuỗi lỗi `EvalScript error.`; `faSig` trả `err` tạm (hộp modal…) không tính.


- **Thử thật 2026-11-24 (Illustrator 2027, .ai tạo bằng osascript):** đoạn 3 font → panel thấy đủ `Chonburi, DMSerifDisplay, ArialMT` (+ MyriadPro của ký tự cách). Chưa kích được nhánh `ensure` thật: `.ai` mang font nhúng nên Illustrator tự thấy font sau khi mở. Lưu ý: kill Font Atlas bằng SIGTERM để lại font session đã đăng ký trong phiên đăng nhập.
- **Lấy mẫu font đoạn/frame (sau review b9acf28a):** `textRuns` của đoạn/frame/ký tự ném lỗi trong Illustrator 2027 (đo 2026-11-24) nên không dò được ranh giới run. Cách làm: bỏ ký tự trắng (đọc `contents` một lần; font ký tự cách như MyriadPro không phải font người dùng thấy), đoạn ≤200 ký tự đọc dày mọi ký tự không trắng; đoạn dài đọc thưa tối đa 16 mẫu rải đều (gồm ký tự cuối), chỉ khi thấy >1 font mới đọc dày (bước `ceil(n/200)`). Font "của vùng" (= font ký tự đầu) chỉ tính khi ký tự đầu không phải khoảng trắng. Đoạn dài đồng nhất tốn ~17 lần đọc thay vì ~200; `viaChars` chỉ tăng khi lấy mẫu tìm thêm font. **Giới hạn:** đoạn >16 ký tự mà font khác chỉ là một run rất ngắn nằm giữa, không trúng mẫu thưa và font đầu/cuối giống nhau → có thể lọt; cũng như run ngắn lọt bước nhảy ở lượt dày. Chấp nhận (font thiếu sẽ hiện ở lần quét khi người dùng sửa, và font ở đầu/cuối luôn được thấy). Thử thật: đoạn 601 ký tự có 1 ký tự DM Serif ở cuối → thấy; đoạn 14 ký tự 3 font → thấy đủ, không còn MyriadPro.

## Panel gửi MỌI font, không chỉ font Illustrator báo thiếu (2026-11-24)

**Lỗi sản phẩm:** `app.textFonts` của Illustrator giữ font Font Atlas ĐÃ TỪNG bật rồi tắt (đo: vẫn còn sau khi gỡ đăng ký và khởi động lại Illustrator; CoreText không còn resolve). Panel chỉ gửi font không có trong `app.textFonts` nên không bao giờ gọi `ensure` cho đúng những font hay dùng nhất.

**Cách sửa:** `faScan` trả thêm `docs[].all` = mọi font khác nhau của tài liệu (tối đa 512) kèm `miss` (Illustrator báo thiếu hay không); `docs[].fonts` (chỉ font thiếu) giữ nguyên cho log. Panel gửi toàn bộ `all` tới broker (chia lô 256 = `MAX_FONTS`), broker so với thư viện + trạng thái bật thật (`activate_auto`: `activated` / `already_active` / `available` / `missing`…) và chỉ bật cái đang tắt. `FontReq.miss` là trường tuỳ chọn của giao thức v1 (client cũ không gửi vẫn chạy, không đổi `PROTOCOL_VERSION`): chỉ dùng để KHÔNG ghi nhật ký `missing` cho font hệ thống không có trong thư viện mà app đã có (`miss=false`); không bao giờ chặn việc bật. `faRedraw()` chỉ gọi khi có font `activated`. Lỗi http (Font Atlas bận/khởi động) giờ hỏi lại sau 5 s (tối đa 6 lần) thay vì bỏ im.

**Chưa đo:** thuộc tính "missing/substituted" của `textFont` (cần màn hình mở khoá); InDesign (script riêng, chưa được duyệt) có thể có cùng lỗi, cần xem sau.

**Font người dùng đã bật:** broker KHÔNG đụng tới font đang bật và đăng ký thật vì lý do của người dùng (thủ công/collection/case): trả `already_active`, không thêm lý do `auto:<app>`, không đổi scope/trạng thái, không ghi nhật ký. Chỉ thêm lý do auto khi chính lượt `ensure` đó bật font, hoặc khi font chỉ đang giữ bởi lý do auto của app thiết kế khác.
