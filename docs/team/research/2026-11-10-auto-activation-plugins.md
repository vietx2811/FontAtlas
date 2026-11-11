# Nghiên cứu: Auto-activation thật sự (plugin) cho FontAtlas, 2026-11-10

Quy ước: [V] = đã xác minh bằng link; [V-local] = Lead đã kiểm tra trên máy; [I] = suy luận (ghi rõ lý do). Ràng buộc: app ad-hoc signed, không Developer ID, không notarize. Không đề xuất flow "kéo document vào font app".

Lưu ý phương pháp: nhiều nguồn là diễn đàn/blog bên thứ ba; trang tài liệu chính thức của Adobe không công bố ngày gỡ CEP. Những chỗ chưa tìm thấy bằng chứng được ghi "chưa xác minh".

---

## 1. Kiểm kê tích hợp auto-activation của các font manager

### 1.1 Từng sản phẩm

| Sản phẩm | Công nghệ theo app | Nguồn |
|---|---|---|
| Typeface 4.5.1 | Illustrator, InCopy, InDesign, QuarkXPress (2026+): kích hoạt "tạm thời" qua Apple Events + AppleScript run-only; mọi app khác (Photoshop, After Effects, Sketch, OmniGraffle, Acorn, Pages/Numbers/Keynote, Pixelmator Pro, TextEdit, Affinity Studio): CoreText font request (không cần quyền). Font Switch (kéo font vào Affinity): Accessibility/Device Control. | [V] https://typefaceapp.com/help/articles/auto-activation ; [V] https://typefaceapp.com/help/articles/font-switch/affinity-canva ; [V-local] entitlement + scripts + AutoActivation.xpc |
| RightFont 10.2.1 | "Tích hợp trực tiếp, không cần plugin" (trừ Figma): Apple Events/osascript + ExtendScript .jsxbin (ai/id/ps/ic); Sketch script; Figma = plugin Community + mở deep link về app. Hỗ trợ Photoshop, Illustrator, QuarkXPress, Sketch, Affinity Designer, "more". Silent Mode. | [V] https://rightfontapp.com/docs/auto-activate-missing-fonts ; [V-local] .jsxbin, RFFontActivator, findMissingFontsInFile |
| Extensis Connect Fonts / Suitcase Fusion (nay thuộc Monotype) | Plug-in C++ native cho Adobe (CS3 trở đi); tài liệu Fusion 21: hỗ trợ Illustrator/InCopy/InDesign/Photoshop CC 2021; bảng support guide chỉ liệt kê đến CC 2022 (Fusion 22.0+). Photoshop Apple Silicon cần Rosetta/ công việc thêm. Dữ liệu 2025/2026: chưa xác minh trên trang Extensis (đã redirect sang help.monotype.com). | [V] https://www.extensis.com/support/suitcase-fusion-support-guide/ ; [V] https://creativepro.com/extensis-announces-plug-ins-for-adobe-indesign-and-illustrator-cs3 |
| Monotype Fonts desktop app (v7.x) | InDesign: plugin native (no-click, "native InDesign technology"), bản legacy dùng web tech; Illustrator: file .aip native (one-click); Acrobat: plugin native; Photoshop: M-series = native, Intel = web tech; InCopy: web tech; Sketch: one-click; Figma: plugin (cần Monotype Desktop App >= 7.3.2, publish bởi Org Admin/Developer, dùng `documentAccess: dynamic-page`); Keynote/Pages/Numbers: chỉ phát hiện. Plugin copy vào thư mục mặc định của OS/app, cần quyền admin cho plugin native; chỉ InDesign 18.0–21.0 (CC 2023–2026) cho plugin mới. Cần internet (tìm font trên cloud Monotype). | [V] https://support.monotype.com/en/articles/9268204-plugins-in-monotype-desktop-app-v7-1-0 ; [V] https://support.monotype.com/en/articles/7860056-auto-activation-plugin-for-figma ; [V] https://support.monotype.com/en/articles/8294762-guide-to-auto-activation-with-font-format-detection-in-adobe-indesign |
| FontBase | Plugin Adobe (Photoshop, InDesign "3-4 giây sau khi mở doc", Illustrator hay lỗi); thư mục Plug-ins phải do app tự dò hoặc user chỉ; bản cũ chạy Rosetta trên M1; chưa thấy Figma. Công nghệ cụ thể (C++/CEP) chưa xác minh. | [V] https://forum.fontba.se/t/auto-activation-for-missing-fonts-not-working/684 ; [V] https://forum.fontba.se/t/auto-activate-and-plugins/162 ; [V] https://forum.fontba.se/t/adobe-apps-no-auto-activation/1602 |
| FontAgent (Insider Software) 10.x | Plugin Adobe cho Mac: CC 2025/2026 cần plugin "Automatic" mới cho InDesign/Illustrator/Photoshop; 2022–2024 InDesign tự động, Illustrator/Photoshop "none required" (dùng cơ chế OS). Windows 2025: Manual. Có plugin Quark. Có per-version maintenance rõ ràng (bảng theo năm CC). | [V] https://www.insidersoftware.com/auto-activating-fonts-in-adobe-applications/ ; (trang hướng dẫn plugin fawin.docs.insidersoftware.com bị 504, chưa đọc được) |
| Adobe Fonts | Built-in InDesign: "Auto-activate Adobe Fonts" trong File Handling prefs; chỉ cho font Adobe. | [V] https://helpx.adobe.com/ca/indesign/desktop/fonts/install-and-activate-fonts.html (qua kết quả tìm kiếm) |
| Font Explorer X | Đã dead (chỉ thấy báo cáo auto-activation chậm trên InDesign 17.2). | [V] community.adobe.com thread FontXplorer X Pro 7.3.0 InDesign 17.2 (kết quả tìm kiếm) |
| Fontstand, Fontcase, Eagle | Không tìm thấy tài liệu auto-activation theo app. Chưa xác minh. | không có nguồn |
| Proxima FontExpert | Plugin InDesign "Missing Fonts Loader" (Windows, InDesign 2021/CC2020). | [V] https://www.ProximaSoftware.com/fontexpert/plugins/missing-fonts-loader |

### 1.2 Ma trận theo app đích (tổng hợp)

| App | Ai hỗ trợ / bằng gì |
|---|---|
| InDesign | Extensis/Monotype (native C++ SDK), FontAgent (plugin), FontBase (plugin), Typeface (Apple Events+AppleScript, cần Apple Events ngược về Typeface), RightFont (ExtendScript qua osascript [V-local]) |
| InCopy | Typeface (AppleScript), RightFont (jsxbin [V-local]), Monotype (web tech) |
| Illustrator | Monotype (.aip native), Extensis (plugin), Typeface (AppleScript+ExtendScript "do javascript" [V-local]), RightFont, FontBase (hay lỗi) |
| Photoshop | Typeface = CoreText request; Monotype (native M-series), FontBase, FontAgent, RightFont (Apple Events) |
| After Effects | Typeface = CoreText request [V]; Suitcase Fusion 7 liệt kê plugin AE (cũ) [V qua tìm kiếm Extensis] |
| Premiere | Không tìm thấy hãng nào hỗ trợ. Chưa xác minh. |
| QuarkXPress | Typeface (từ 2026+), RightFont, FontAgent (plugin Quark) |
| Figma desktop+web | RightFont (plugin Community + deep link; extension Chrome có báo cáo hỏng sau UI mới 2025/05, chỉ 1 nguồn), Monotype (plugin Figma). Typeface: không thấy. |
| Sketch | RightFont, Monotype (one-click), Typeface (CoreText request), Typeface switch-sketch.sketchplugin [V-local] |
| Affinity (v2 / by Canva) | Typeface (CoreText request + Font Switch qua quyền Accessibility), RightFont (Affinity Designer), FontBase (nhận font mới đang chạy; không có plugin vì Affinity không có scripting công khai) |
| Office, Pages/Keynote | Pages/Keynote/Numbers: CoreText request nhưng Typeface cảnh báo app sandbox (App Store) không gửi request trên macOS 27 [V]; Monotype chỉ phát hiện. Office: chưa tìm thấy dữ liệu. |
| Canva, Framer, DaVinci, C4D, Blender | Không tìm thấy hãng nào. Chưa xác minh. |

### 1.3 Nơi cài plugin và build theo version

- Monotype: copy plugin vào thư mục mặc định của OS/app; plugin native Acrobat/InDesign/Photoshop M-series nằm trong cấu trúc thư mục của app Adobe và cần quyền admin; bật/tắt xong phải restart app. [V] link Monotype 9268204 ở trên.
- FontBase: user có thể phải chỉ thư mục Plug-ins của Illustrator thủ công. [V] https://forum.fontba.se/t/auto-activate-and-plugins/162
- Per-version: Monotype nêu rõ phạm vi InDesign 18.0–21.0; FontAgent liệt kê theo năm CC; C++ SDK của Illustrator phải build lại cho mỗi version. [V] https://mapsoft.com/posts/illustrator-uxp-status.html ; [I] hầu hết plugin C++ đều per-version vì ABI SDK đổi mỗi năm.

---

## 2. Extensibility của Adobe (2025–2026)

| Nền tảng | InDesign | Illustrator | Photoshop | After Effects | InCopy |
|---|---|---|---|---|---|
| UXP | Có (plugin + script, InDesign 18.5+ cho network) [V] https://developer.adobe.com/indesign/uxp/resources/recipes/network/ | Chưa công khai cho bên thứ ba (đến 5/2026; còn NDA prerelease; thread xin early access còn mở 10/2026) [V] https://mapsoft.com/posts/illustrator-uxp-status.html ; https://forums.creativeclouddeveloper.com/t/illustrator-early-access-request/12222.md?page=2 | Có (từ 2021) | Đang thêm: private beta rồi public beta (Media Encoder đã public beta 9/2026) [V qua kết quả tìm kiếm: https://forums.creativeclouddeveloper.com/t/uxp-now-available-in-adobe-media-encoder-beta/12170.md] | Chưa xác minh; có thread "What to do with CEP plugins for InCopy?" https://forums.creativeclouddeveloper.com/t/what-to-do-with-cep-plugins-for-incopy/12195.md |
| CEP | Chưa deprecated; panel cũ vẫn load [V qua mapsoft.com/posts/indesign-cep-extensions.html, nguồn vendor] | Chưa deprecated; vẫn là lựa chọn panel công khai duy nhất [V] mapsoft illustrator-uxp-status | Adobe nói sẽ deprecated rồi gỡ, không có ngày [V] https://developer.adobe.com/photoshop/uxp/guides/uxp_for_you/uxp_for_cep_devs/ | Vẫn dùng CEP trong AE 2026 [V qua https://samabr85.github.io/blog/cep-after-effects/] | n/a |
| C++ SDK | Có, per-version | Có, build lại mỗi version | Có | Có | Có |
| ExtendScript | Có (đang ở trạng thái legacy, còn chạy) | Có, gần như toàn bộ object model | Có | Có | Có |

Ngày gỡ CEP: một thread cộng đồng 8/1/2026 hỏi về "khung 2026" nhưng không có trả lời của Adobe [V] https://community.adobe.com/questions-606/cep-uxp-roadmap-should-developers-stop-building-cep-plugins-and-what-happens-to-existing-ones-1614807. Kết luận: chưa có ngày chính thức được xác minh; [I] rủi ro dài hạn có thật nhưng chưa phải ngay.

### Trả lời các câu hỏi cụ thể

(a) Sự kiện mở document: InDesign UXP/scripting có `app.eventListeners.add("afterOpen", ...)`; afterOpen có thể quan sát ở cả application và document; listener chỉ sống trong một session, muốn có ở mọi session phải đặt script vào thư mục startup scripts [V] https://developer.adobe.com/indesign/uxp/resources/recipes/indesign-events/. Photoshop UXP: `action.addNotificationListener(['open'], handler)` [V] https://developer.adobe.com/photoshop/uxp/2022/ps-reference/media/photoshopaction; có thread "Autostart plugin on Photoshop startup to listen to open/close" (nội dung chưa đọc) https://forums.creativeclouddeveloper.com/t/autostart-plugin-on-photoshop-startup-to-listen-to-addnotificationlistener-open-close/3940.md. Illustrator: ExtendScript startup scripts chạy lúc app khởi động [V] https://ai-scripting.docsforadobe.dev/introduction/executingScripts/ nhưng không tìm thấy sự kiện "document opened" cho Illustrator (đó là lý do hãng dùng native .aip/C++ notifier hoặc polling Apple Events). Chưa xác minh gì thêm cho Illustrator.

(b) Danh sách font thiếu per document:
- InDesign DOM: `Font.status` có `INSTALLED`, `NOT_AVAILABLE`, `FAUXED`, `SUBSTITUTED` [V] https://developer.adobe.com/indesign/dom/api/f/FontStatus ; có script mẫu liệt kê missing fonts [V] https://community.adobe.com/questions-671/jsx-solution-to-list-missing-document-fonts-880025. Cẩn trọng: một người dùng thấy font bị coi missing thực ra là SUBSTITUTED; font trong thư mục "Document fonts" có thể không hiện trong app.fonts. Chưa xác minh `document.fonts` đã đủ dữ liệu ngay lúc afterOpen bắn.
- Photoshop UXP: yêu cầu API "kiểm tra missing fonts" đã bị đưa lên roadmap từ 2019 cho XD; không thấy API công khai tương đương cho Photoshop [V] https://forums.creativeclouddeveloper.com/t/feature-request-add-api-to-check-if-fonts-are-missing/195.md (thread này thực ra về XD). [I] có thể suy từ batchPlay/ descriptor text layer, chưa xác minh.
- Illustrator: có thread hỏi cách lấy missing fonts bằng C++ SDK https://community.adobe.com/t5/illustrator-discussions/how-to-retrieve-missing-fonts-in-active-document-in-illustrator-sdk-using-c/m-p/13639856 (nội dung chưa đọc).

(c) Plugin nói chuyện với localhost/WebSocket: UXP có `WebSocket` global, phải khai báo `requiredPermissions.network.domains` với scheme `wss://`; wildcard hỗ trợ; Photoshop chỉ cho http trên Win32, InDesign thì không bị hạn chế này; yêu cầu InDesign 18.5+, manifest v5+ [V] https://developer.adobe.com/indesign/uxp/resources/recipes/network/. Tài liệu không nói gì về localhost; báo cáo diễn đàn mâu thuẫn (có người chỉ chạy được với `"all"`, người khác chạy với `ws://localhost/`) [V qua https://forums.creativeclouddeveloper.com/t/allowing-network-access-to-a-specific-port/11509 và https://github.com/AdobeDocs/uxp-photoshop/issues/321]. UXP không thể host/nhận kết nối đến [V qua cùng kết quả tìm kiếm]; nghĩa là plugin phải là client, FontAtlas phải mở WebSocket server. Cần thử nghiệm trước (spike).

(d) Refresh font list/recompose sau khi activate: không có API tài liệu. Báo cáo cộng đồng cho thấy vấn đề phía Adobe: "Adobe apps cập nhật menu font theo chu kỳ riêng, có khi vài phút" [V] FontAgent doc qua kết quả tìm kiếm (https://fawin.docs.insidersoftware.com/?p=107); [I] đó là lý do plugin native của Extensis/Monotype tồn tại: chúng nói trực tiếp với font engine của app.

### InDesign: dialog Missing Fonts vs afterOpen
- Nhà phát triển C++ đã thử kích hoạt font trong signal responder trước khi mở, nhưng InDesign vẫn hiện dialog Missing Fonts; không có trả lời [V] https://community.adobe.com/t5/indesign-discussions/fonts-are-not-updating-in-indesign-instantly/m-p/4423143. Plugin thương mại thừa nhận có lúc InDesign vẫn đánh dấu missing dù plugin đã kích hoạt trong lúc mở [V] https://www.ProximaSoftware.com/fontexpert/plugins/missing-fonts-loader.
- Với script: `afterOpen` có thể gặp lỗi "modal dialog or alert is active" (`app.modalState` true) khi tạo doc mới [V] https://community.adobe.com/t5/indesign-discussions/afteropen-event-listener-in-indesign-throws-up-error-of-cannot-handle-request-because-a-modal-dialog/m-p/15229043. Cách giảm: dùng `beforeOpen` và đặt `userInteractionLevel = NEVER_INTERACT` [V qua https://community.adobe.com/questions-671/suppress-missing-font-alert-901868, đã thấy trong tìm kiếm].
- Thread 2010 (ExtendScript) nói before/afterOpen "gần như vô dụng vì bắn trước khi cửa sổ layout mở" (nguồn cũ) [V qua https://community.adobe.com/t5/indesign-discussions/exploring-onopen-eventhandling/m-p/6058528].
- [I] Cách chắc chắn nhất để tránh dialog: kích hoạt font TRƯỚC khi InDesign parse file, chỉ khả thi bằng plugin native hoặc bằng cách đặt font vào thư mục mà InDesign tự load. Bài toán "trước dialog" chưa có lời giải tài liệu cho UXP/ExtendScript.
- Adobe nói InDesign/InCopy kiểm tra font khi mở file hoặc import [V] https://helpx.adobe.com/sk/indesign/desktop/troubleshoot/font-and-plugin-issues/missing-font-error.html.

### Phân phối và ký
- UXP: cài `.ccx` bằng double-click -> Creative Cloud Desktop, có cảnh báo "package ngoài Marketplace", hoặc dùng UPIA (command line, có thể cần admin) [V] https://developer.adobe.com/uxp/guides/how-to/distribution/install/. Trang không nói về chữ ký/chứng chỉ. Không có đường load plugin chưa đóng gói ngoài UDT/developer mode [V qua https://forums.creativeclouddeveloper.com/t/automatically-loading-uxp-plugins-without-developer-tool/3953 và https://forums.creativeclouddeveloper.com/t/installing-plugin-without-udt-without-creative-cloud/12165.md]. Có hướng dẫn cài bằng command line: https://blog.developer.adobe.com/how-to-install-uxp-plugins-using-command-line-tools-a6a62ca4cb1a.
- Quan trọng: hệ thống ký của Adobe cho .ccx không liên quan Apple Developer ID. [I] FontAtlas có thể gói .ccx và giao qua UPIA mà không cần Developer ID Apple; cần thử.
- C++ SDK: build per-version, dùng SDK của Adobe; chưa tìm thấy trang tải SDK công khai trong lần tìm này. [I] Rủi ro bảo trì lớn nhất; file `.aip`/plug-in nằm trong thư mục Plug-ins của app (Monotype yêu cầu admin).

---

## 3. Đường Apple Events: các vấn đề

- TCC Automation theo từng target app; từ chối -> lỗi -1743 `errAEEventNotPermitted` [V qua https://mjtsai.com/blog/2018/08/31/aedeterminepermissiontoautomatetarget-added-but-aepocalyse-still-looms]. API `AEDeterminePermissionToAutomateTarget` có `askUserIfNeeded`, nhưng nhiều nguồn nói không ổn định (treo, không dùng để biết trạng thái im lặng) [V cùng nguồn + https://macscripter.net/t/macos-10-15-beta-apple-event-authorization-dialogs-never-show-up/71701]. Cần `NSAppleEventsUsageDescription` và entitlement `com.apple.security.automation.apple-events` nếu bật hardened runtime [V qua https://developer.apple.com/forums/thread/751802]. [V-local] Typeface có temporary-exception apple-events theo bundle id vì sandbox.
- Grant gắn với danh tính chữ ký: nhiều báo cáo cho thấy chữ ký ad-hoc cho danh tính không ổn định nên grant có thể mất, nhiều báo cáo về việc mỗi bản cập nhật tạo dòng mới/hỏi lại [V mức yếu, nguồn là issue tracker và doc cộng đồng: https://claudeissues.com/issue/54691-macos-tcc-automation-grants-not-preserved-across-cc-versions-settings-panel-accu ; https://docs.claw.so/engine/platforms/mac/permissions/]. Không tìm thấy nguồn Apple xác nhận cơ chế designated requirement cho ad-hoc. [I] Ad-hoc có DR dạng `cdhash H"..."` đổi mỗi build => grant mất khi update (hỏi lại hoặc im lặng từ chối). Mitigation [I]: ký bằng một identity tự tạo (self-signed code-signing cert, giữ cố định ở máy build) để DR dựa trên chứng chỉ + bundle id; self-signed không qua Gatekeeper nhưng TCC chỉ so DR nên có thể ổn định cho cùng cert; NHƯNG mỗi máy người dùng không có cert đó trong keychain thì DR vẫn tính được từ chữ ký (anchor là cert leaf hash), nên khả năng ổn định cao [I], cần spike kiểm chứng. `tccutil reset AppleEvents <bundle-id>` dùng để reset; một thread Jamf nói reset đơn lẻ không dừng được prompt [V mức yếu].
- Modal dialog/ bận: InDesign trả lỗi "cannot handle the request because a modal dialog or alert is active" khi có modal [V link thread afterOpen ở trên]; [I] Apple Events tới app bận sẽ timeout (mặc định ~2 phút) hoặc xếp hàng, nên cần timeout ngắn + retry.
- Không có push notification từ app -> polling [I] (đường duy nhất cho Illustrator nếu không có plugin); tần suất 1-3 giây khi app đang chạy là đủ, cost nhỏ nhưng mỗi lần chạy `osascript`/ExtendScript có thể làm app "nhấp nháy" thực thi script. Typeface dùng AppleScript run-only với các handler `detect`, `detectAll`, `ping`, `notify` [V-local] -> [I] có cơ chế "notify" ngược về Typeface cho InDesign/InCopy (Typeface nói cần thêm quyền để InCopy/InDesign "giao tiếp ngược lại" [V] typefaceapp.com auto-activation), tức đăng ký listener trong InDesign rồi gọi ngược Apple Events.
- Bug Illustrator 29.8.x: font kích hoạt bởi manager không hiện trong phiên đang mở nếu không restart; nhiều người báo đã sửa ở v30 (2026) [V] https://community.adobe.com/questions-652/fonts-management-in-illustrator-29-8-1-doesnt-work-anymore-817367. Cũng có thread riêng FontBase https://forum.fontba.se/t/activating-fonts-do-not-appear-in-illustrator/1733.
- Adobe có nhận font session CoreText không restart? Không có tài liệu Apple/Adobe. Báo cáo cho thấy: Adobe app cập nhật menu theo chu kỳ riêng, có khi vài phút [V qua FontAgent doc]; plugin native của Extensis/Monotype tồn tại để giải quyết điều này. Một Adobe staff nói Illustrator bình thường nên tự refresh font hệ thống mới thêm [V qua tìm kiếm community.adobe.com/questions-652/activating-fonts-without-restarting-819687]. [I] Typeface dùng AppleScript cho 4 app Adobe/Quark chính vì CoreText request không hoạt động tốt với chúng (một poster Adobe cũng nói Core Text auto-activation không chạy đúng với một số app Adobe/Quark [V qua kết quả tìm kiếm]).
- Sandbox: FontAtlas nên không sandbox (không phải App Store). Typeface sandbox nên phải dùng temporary-exception [V-local]. App không sandbox thì chỉ cần entitlement + usage description.

---

## 4. Figma

- Figma desktop có font helper tích hợp; trình duyệt cần FigmaAgent cài riêng, chạy HTTP/HTTPS server trên localhost, chỉ nhận kết nối từ figma.com. Figma dùng font cài trong Font Book, chỉ .ttf/.otf, và sau khi cài font trong trình duyệt phải reload file [V] https://help.figma.com/hc/en-us/articles/360039956994. Không có tài liệu nói desktop có tự phát hiện font mới hay không.
- Plugin API: `text.hasMissingFont`, `figma.listAvailableFontsAsync()`; nhân viên Figma nói plugin chỉ dùng font cài trong OS, font shared của org, và Google Fonts [V qua https://developers.figma.com/docs/plugins/working-with-text và forum.figma.com thread "loadfontasync loading a missing font"]. Plugin trên desktop thấy `~/Library/Fonts` (báo cáo user, không chính thức) [V mức yếu: https://forum.figma.com/share-your-feedback-26/mcp-loadfontasync-fails-for-locally-installed-fonts-52313].
- Network: manifest `networkAccess` có `allowedDomains`, `devAllowedDomains` (ví dụ `http://localhost:3000`), `reasoning` [V] https://developers.figma.com/docs/plugins/manifest ; https://developers.figma.com/docs/plugins/making-network-requests. `devAllowedDomains` chỉ áp dụng khi chạy dev plugin; plugin đã publish phải dùng `allowedDomains`. [I] Cho localhost cần tự kiểm tra việc có chấp nhận `http://localhost:PORT` trong `allowedDomains` khi publish và CORS/mixed-content từ iframe UI; có forum cho thấy CORS vẫn lỗi dù khai báo [V mức yếu: https://forum.figma.com/report-a-problem-6/cors-error-in-figma-plugin-despite-configuring-alloweddomains-and-devalloweddomains-36708].
- Phân phối: Monotype cho thấy plugin Figma publish bị ràng buộc (Org admin / dev) [V]; plugin riêng của FontAtlas có thể publish lên Community hoặc dùng dev plugin (import manifest) không cần review; [I] dev plugin (import from manifest) hoạt động cho từng người dùng desktop, cho phép localhost qua devAllowedDomains, không cần publish.
- Cách RightFont làm: plugin lấy missing fonts -> mở một trang web -> trang gọi deep link khởi động RightFont -> hộp thoại Activate All -> reload tab Figma nếu font không nạp [V] RightFont docs. [I] Dùng custom URL scheme thay vì localhost để tránh các giới hạn network của plugin.
- FigmaAgent: [V] chỉ nhận figma.com. Chưa xác minh hành vi refresh danh sách font.

---

## 5. Sketch, Affinity, CoreText font request

- Sketch: manifest có `handlers.actions` với `"OpenDocument": "onOpenDocument"`, action chạy khi document mở [V] https://developer.sketch.com/plugins/actions ; https://developer.sketch.com/reference/action/OpenDocument. Plugin Fontily nói "Sketch API không cho biết font có thiếu hay không" nên phát hiện thiếu có thể sai [V] https://github.com/ukn530/FontFinder và kết quả liên quan (Fontily: https://awesome.ecosyste.ms/projects/github.com%2Fpartyka1%2FFontily). [I] Tuy nhiên Sketch (không sandbox? chưa xác minh) vốn đã gửi CoreText font request nên Typeface liệt kê Sketch là "requesting app" [V]; ưu tiên CoreText cho Sketch, plugin chỉ là phương án dự phòng.
- Affinity: không tìm thấy API plugin/scripting công khai; một comment diễn đàn nói có thể có hệ thống scripting vào 2026 (không xác minh) [V mức yếu: https://forum.fontba.se/t/affinity-designer-plugin-compatibility/29]. Có panel Scripting được nhắc ở một danh sách bên thứ ba (không xác minh). Affinity tự thay font thiếu bằng font tương đương khi mở [V] https://affinity.help/publisher2ipad/en-US.lproj/pages/Text/fontManager.html (trang iPad). Typeface liệt kê "Affinity Studio" trong nhóm app gửi font request [V] https://typefaceapp.com/help/articles/auto-activation. FontBase users báo Affinity nhận font mới trong khi chạy [V qua kết quả tìm kiếm].
- CoreText font request: `CTFontManagerCreateFontRequestRunLoopSource` đã bị deprecated, Apple ghi "sẽ bị gỡ trong bản tương lai" [V] https://developer.apple.com/forums/thread/845853?answerId=905422022. Từ macOS 27 beta 5, callback không nhận request nếu app sandbox (cả bên gửi lẫn bên nhận cần không sandbox, theo tác giả thread; Apple DTS chưa kết luận) [V cùng link]. Typeface xác nhận: trên macOS 27 chỉ app không sandbox gửi request; app sandbox App Store như Pages/Numbers/Keynote/Pixelmator/TextEdit sẽ không gửi [V] typefaceapp.com auto-activation. Giải pháp Apple endorse: app sandbox + helper không sandbox qua XPC; FontAtlas không sandbox nên khỏi lo, [I] nhưng phải theo dõi vì Apple có thể gỡ API trong macOS 28.
- App nào thật sự gửi request (theo Typeface): Affinity Studio, Photoshop, After Effects, Sketch, OmniGraffle, Acorn, Pages/Numbers/Keynote, Pixelmator Pro, TextEdit [V]. Office: chưa xác minh.

---

## 6. Khuyến nghị kiến trúc

Nguyên tắc: hai tầng. Tầng A (không cần plugin, không cần quyền): CoreText font request listener (một XPC/helper không sandbox, đã có kinh nghiệm từ AutoActivation.xpc của Typeface [V-local]). Tầng B (chỉ cho các app không dùng được tầng A): Apple Events ở mức tối thiểu + một plugin UXP nhỏ cho InDesign/Photoshop/Figma, và dự phòng.

| App | Phương án đề xuất | Giá trị | Công sức | Rủi ro |
|---|---|---|---|---|
| Photoshop, After Effects, Sketch, Affinity, OmniGraffle, Acorn, Pixelmator | CoreText font request listener (Tầng A). Không cần quyền, không per-version. | Cao | Thấp | Apple deprecate API; macOS 27 + sandbox (app đích sandbox sẽ không gửi) [V]. Bù bằng ghi nhận trong UI "app X không hỗ trợ". |
| InDesign (+InCopy) | Ưu tiên 1: ExtendScript startup script (hoặc UXP script/plugin) đăng ký `afterOpen` + `beforeOpen`, đọc `Font.status`, gọi ngược FontAtlas (UXP WebSocket client tới localhost, hoặc `osascript` ngược). Ưu tiên 2: polling Apple Events (cách Typeface/RightFont). Ưu tiên 3 (không khuyến nghị): C++ SDK. | Rất cao (InDesign là app quan trọng nhất của GD/layout artist) | Trung bình | Dialog Missing Fonts có thể vẫn hiện trước khi activate [V]; UXP localhost chưa xác minh; listener có thể crash khi reload plugin [V qua https://forums.creativeclouddeveloper.com/t/event-listeners-crash-indesign-when-rebuilding-a-uxp-plugin/7535]; grant TCC với ad-hoc. Mitigation: cho phép người dùng thấy "Activated N fonts, bấm Relink/ mở lại" khi dialog đã hiện. |
| Illustrator | Polling Apple Events + ExtendScript `do javascript` (như Typeface [V-local]); bắt buộc Illustrator >= v30 để tránh bug 29.8 [V]. CEP/UXP chưa khả thi (UXP chưa công khai [V]). | Cao | Trung bình | TCC; modal; bug phiên bản cũ; không có event => polling. |
| QuarkXPress | Apple Events như Typeface [V]/RightFont [V]. Giá trị thấp hơn ở VN; làm sau. | Thấp–TB | Thấp | Quyền, per-version nhỏ. |
| Figma (desktop + web) | Plugin Figma (dev plugin import manifest hoặc publish Community) đọc `hasMissingFont`, gọi FontAtlas bằng custom URL scheme (như RightFont [V]) hoặc localhost qua `devAllowedDomains`; sau đó nhắc reload file. | Cao | Trung bình | Localhost khi publish chưa xác minh; Figma có thể đổi UI/permission (plugin RightFont extension hỏng sau UI mới [V mức yếu]). |
| Photoshop nâng cao | Có thể dùng CoreText + (tùy chọn) UXP `action.addNotificationListener(['open'])` nhưng thiếu API missing fonts công khai [chưa xác minh]; không cần. | Thấp | Cao | Cao |
| After Effects, Premiere | After Effects: CoreText [V]. Premiere: chưa có ai hỗ trợ, bỏ qua. | Thấp | Thấp | |
| Pages/Keynote/Office/Canva/Framer/DaVinci/C4D/Blender | Pages/Keynote: không làm được trên macOS 27 do sandbox [V]. Canva/Framer là web: ngoài phạm vi; DaVinci/C4D/Blender: chưa có dữ liệu, bỏ qua. | Thấp | n/a | n/a |

Hạ tầng chung:
1. Một "Activation Broker" trong app FontAtlas: map tên font -> file trong thư viện; API nội bộ (Unix domain socket hoặc localhost WebSocket có token) để plugin gọi `activate(fonts[])`; kích hoạt phiên (session) bằng CoreText, kèm cơ chế "poke" (Apple Events/UXP) để app làm mới danh sách font.
2. Ký: dùng một self-signed code-signing identity cố định cho mọi bản build (giữ DR ổn định) thay vì ad-hoc [I]; phải spike: (a) cập nhật bản mới có giữ grant Automation không, (b) trên máy sạch.
3. Cài plugin: UXP `.ccx` cài qua UPIA/double-click [V]; ExtendScript startup scripts copy vào thư mục startup của từng app [V cho Illustrator, tương tự InDesign]; Figma plugin dev import. Cần UI "Cài tích hợp" liệt kê từng app và trạng thái (đã cài, quyền, phiên bản).
4. Spike phải làm trước khi cam kết (đều chưa xác minh): (i) UXP trong InDesign kết nối `ws://localhost:PORT` với manifest permissions; (ii) thời điểm `afterOpen` so với dialog Missing Fonts trên InDesign 2026 và dữ liệu `Font.status`; (iii) InDesign/Illustrator có nhận font session CoreText hay cần restart/poke; (iv) TCC grant sau khi cập nhật với self-signed identity; (v) Figma dev plugin gọi localhost.

Điều cần tránh: C++ plugin (per-version, SDK, quyền admin, Rosetta) cho đến khi các spike trên thất bại.

---

## Nguồn mạnh nhất
- https://typefaceapp.com/help/articles/auto-activation (ma trận app, sandbox macOS 27)
- https://developer.apple.com/forums/thread/845853 (CoreText font request + sandbox, deprecated)
- https://support.monotype.com/en/articles/9268204-plugins-in-monotype-desktop-app-v7-1-0 (công nghệ plugin theo app)
- https://www.insidersoftware.com/auto-activating-fonts-in-adobe-applications/ (per-version plugin FontAgent)
- https://rightfontapp.com/docs/auto-activate-missing-fonts (cách RightFont, Figma flow)
- https://developer.adobe.com/indesign/uxp/resources/recipes/indesign-events/ và /network/ (events, network)
- https://mapsoft.com/posts/illustrator-uxp-status.html (UXP Illustrator, C++ per-version)
- https://help.figma.com/hc/en-us/articles/360039956994 (FigmaAgent localhost) và https://developers.figma.com/docs/plugins/manifest
- https://developer.sketch.com/plugins/actions (OpenDocument)
- https://community.adobe.com/questions-652/fonts-management-in-illustrator-29-8-1-doesnt-work-anymore-817367 (bug 29.8 -> v30)
