# Nghiên cứu: Auto-activation cho FontAtlas (2026-11-09)

Ký hiệu: **[V]** = đã kiểm chứng từ nguồn có link; **[I]** = suy luận / kiến thức chung, chưa có nguồn xác nhận.

## Tóm tắt (10 dòng)

1. Có **hai cơ chế** thực tế: (a) plugin riêng cho từng app Adobe, (b) CoreText "font request" cho mọi app native khác. Không có cơ chế thứ ba chính thức nào. [V]
2. Typeface 3/4 hiện auto-activate cho Adobe, Affinity Studio, Photoshop, After Effects, Sketch, Pages/Keynote... bằng cách lắng nghe request font của hệ thống; Adobe cần thêm quyền Automation. [V] ([Typeface](https://typefaceapp.com/help/articles/auto-activation))
3. Cơ chế (b) rất có thể là `CTFontManagerCreateFontRequestRunLoopSource`: API **deprecated từ macOS 11**, và trên **macOS 27 beta 5+ không nhận request khi app sandboxed** (FB24764122). Phải dùng helper non-sandboxed. [V] ([Apple Forums](https://developer.apple.com/forums/thread/845853)); việc Typeface dùng đúng API này là [I].
4. Extensis Connect Fonts có plugin auto-activation cho After Effects, Illustrator, InCopy, InDesign, Photoshop CC 2024-2026 và Sketch. [V] ([compat](https://www.extensis.com/support/connect-fonts-compatibility))
5. Illustrator 29.8.x làm hỏng activate/deactivate khi tài liệu đang mở với Connect, FontBase, Nexus; Adobe sửa ở v30. [V] ([Adobe Community](https://community.adobe.com/t5/illustrator-discussions/fonts-management-in-illustrator-29-8-1-doesnt-work-anymore/m-p/15549720))
6. Figma: không có API nào để quản lý font từ bên ngoài. Cách duy nhất là activate bằng CoreText rồi làm FigmaAgent/Figma đọc lại (restart/reload). RightFont có Figma plugin; FontBase nói Figma "most likely can't work". [V]
7. Affinity: không có plugin/SDK nào; manager dựa vào request font hệ thống (FontAgent: bỏ tick "Only for Adobe and Quark"; Typeface: nhóm "Other apps"). [V]
8. Fallback không cần plugin: parse font từ tài liệu (IDML, .sketch, .docx, .psd, .ai, .fig) + kéo-thả / FSEvents. Khả thi, độ chính xác trung bình. [I]
9. Phase 1 (rẻ, giá trị cao): "Activate fonts used by this document" + view Missing fonts. Phase 2: spike CoreText font-request helper (non-sandboxed), sau đó plugin Adobe. Phase 3: Figma, Office.
10. Rủi ro lớn nhất: API font-request deprecated và đang bị Apple siết; cần Developer ID + notarization, phân phối ngoài Mac App Store.

## 1. Bảng theo app

| App | Manager làm thế nào | Công nghệ | Còn chạy 2025-26? | Nguồn |
|---|---|---|---|---|
| InDesign, InCopy | Suitcase/Connect: plugin + Font Panel. Typeface: bật riêng từng app, Font Switch (InDesign). FontBase: plugin | Plugin trong app (tên công nghệ cụ thể: không tìm thấy nguồn; C++ plugin là suy luận [I]); Typeface qua Automation | Connect: CC 2024-2026 [V]. Có lỗi lẻ tẻ phải restart app | [Extensis](https://www.extensis.com/support/connect-fonts-compatibility), [Typeface](https://typefaceapp.com/help/articles/auto-activation) |
| Illustrator | Như trên | Plugin + Automation | Hỏng ở 29.8.x, sửa ở v30; FontBase 2024 báo lỗi riêng | [Adobe Community](https://community.adobe.com/t5/illustrator-discussions/fonts-management-in-illustrator-29-8-1-doesnt-work-anymore/m-p/15549720), [FontBase forum](https://forum.fontba.se/t/illustrator-auto-activation-fonts-dont-activate-correctly/1997) |
| Photoshop | Connect: plugin + panel (panel cần Intel/Rosetta). Typeface: nhóm "Other apps" (request font hệ thống) | Plugin / CoreText request | Có, CC 2024-2026 | Extensis compat, Typeface |
| After Effects | Connect: plugin. Typeface: Other apps | Plugin / CoreText request | Có | như trên |
| Premiere, DaVinci Resolve, Cinema 4D | Không tìm thấy manager nào công bố hỗ trợ | Chưa rõ | Không xác định. Premiere dùng chung engine text Adobe nhưng không có plugin được liệt kê [I] | (không có nguồn) |
| Affinity (Designer/Publisher/Photo v2, Affinity by Canva/Studio) | FontAgent: không cần plugin, chỉ bỏ tick "Only for Adobe and Quark". Typeface: "Affinity Studio" trong Other apps. Extensis: chỉ Application Sets (activate theo app, không theo tài liệu) | Request font của hệ thống / app set | Có. Affinity nhận font mới activate khi đang chạy | [FontAgent](https://insidersoftware.freshdesk.com/support/solutions/articles/25000023081-how-can-i-get-auto-activation-to-work-in-the-affinity-suite-of-applications-), [Extensis App Sets](https://help.extensis.com/hc/en-us/articles/22902282639387-Application-Sets-in-Connect-Fonts), [Typeface](https://typefaceapp.com/help/articles/auto-activation) |
| Sketch | Connect: plugin Sketch. RightFont, Typeface: có | Plugin Sketch / Automation | Có | [Extensis blog](https://www.extensis.com/blog/activate-your-fonts-in-any-application), [RightFont](https://rightfontapp.com/docs/auto-activate-missing-fonts) |
| QuarkXPress | Suitcase/FontAgent: plugin; Typeface: QuarkXPress 2026+ | Plugin / Automation | Có | [Typeface](https://typefaceapp.com/help/articles/auto-activation) |
| Figma (desktop, web) | RightFont: Figma plugin + Chrome extension. Typeface/Connect: không có nguồn. FontBase: không hỗ trợ | Plugin Figma → mở lại RightFont; xem mục 2 | RightFont Chrome extension báo hỏng với UI mới (05/2025) [V, nguồn thứ ba]. Figma desktop 126.4.x có lỗi FigmaAgent [V] | [RightFont](https://rightfontapp.com/docs/auto-activate-missing-fonts), [FontBase forum](https://forum.fontba.se/t/compatibility-with-figma/766) |
| Pages, Keynote, Numbers | Typeface liệt kê "có giới hạn" | CoreText request | **Không** với app sandbox/App Store trên macOS 27: các app này không gửi request | [Typeface](https://typefaceapp.com/help/articles/auto-activation) |
| Canva | Không áp dụng (web, font riêng) [I] | n/a | n/a | n/a |
| Microsoft Office | Không tìm thấy nguồn; Word là ví dụ cho App Sets của Extensis (theo app) | App Sets | Theo app, không theo tài liệu | [Extensis App Sets](https://help.extensis.com/hc/en-us/articles/22902282639387-Application-Sets-in-Connect-Fonts) |
| Monotype FontExplorer X | Đã ngừng bán 2022, ngừng hỗ trợ; mất kết nối Adobe plugin 2022 | Plugin Adobe cũ | Không | [Adobe Community](https://community.adobe.com/t5/adobe-fonts-discussions/fontexplorer-x-pro-no-longer-able-to-manage-adobe-fonts/m-p/12786488/highlight/true) |
| Fontcase | Auto-activation 2010 (CS3/CS4); không có nguồn gần đây | Không rõ | Không xác định | [TNW](https://thenextweb.com/news/fontcase-font-autoactivation) |
| Adobe Fonts | Tự activate font Adobe khi mở file InDesign, chỉ font Adobe | Tích hợp sẵn | Có | [Adobe Help](https://helpx.adobe.com/ph_fil/indesign/desktop/fonts/install-and-activate-fonts.html) |

Ghi chú: không tìm được nguồn công khai nói plugin Adobe của Extensis/FontBase dùng C++ SDK, UXP hay CEP. Không có bằng chứng Adobe bỏ C++ plugin. Trên diễn đàn Adobe, UXP chưa có API liệt kê/khôi phục font thiếu và chưa có hybrid plugin ở InDesign. [V] ([forum](https://forums.creativeclouddeveloper.com/t/priority-request-uxp-hybrid-plugin-support-in-indesign/8140.md), [thread font](https://community.adobe.com/t5/indesign-discussions/loading-and-unloading-custom-fonts-with-indesign-plugin/m-p/14948780)). Phần ánh xạ này nên được xác nhận bằng cách giải nén plugin của Connect/FontBase.

Cơ chế Typeface cho Adobe: **Automation (Apple Events)**, hỏi quyền một lần chung và một lần mỗi app; font Adobe được bật tạm cho mọi app và tắt khi Typeface đóng. Với app khác, font chỉ dành cho app yêu cầu. [V]

## 2. Figma

- Desktop app đi kèm font helper; trình duyệt cần cài FigmaAgent chạy HTTP/HTTPS trên localhost, chỉ nhận kết nối từ figma.com. [V] ([Figma Help](https://figma-signup.helpjuice.com/fonts/missing-font-alert-in-figma-design))
- Bản Rust tái tạo (không chính thức) cho thấy: `GET /figma/font-files` liệt kê font, `GET /figma/font-file?file=` stream file, cổng mặc định 44950/44951 trong bản Linux; liệt kê qua `CTFontManagerCopyAvailableFontURLs` trên macOS. Đây là bản tái tạo, không phải spec của Figma. [V cho bản tái tạo, I cho bản gốc] ([docs.rs](https://docs.rs/crate/figma-agent/0.2.5)). Cổng 18412 hay được dẫn cho bản chính thức, chưa xác nhận.
- Font mới chỉ hiện sau khi reload tab hoặc restart app; cách khắc phục cộng đồng: force quit FigmaAgent trong Activity Monitor rồi mở lại Figma. [V] ([Figma Forum](https://forum.figma.com/report-a-problem-6/my-local-fonts-are-not-working-anymore-suddenly-54488))
- Không có API công khai để "kích hoạt khi file thiếu font". RightFont làm bằng plugin Figma: chạy plugin, "Activate All", rồi mở RightFont; có Silent Mode. [V] ([RightFont](https://rightfontapp.com/docs/auto-activate-missing-fonts)). Plugin Figma đọc được font trong file (suy luận từ Plugin API [I]) rồi gửi tên font cho app local.
- Typeface: không tìm thấy nguồn về Figma. Giả thuyết "activate rồi refresh agent" khớp với hành vi RightFont nhưng chưa xác nhận cho Typeface.
- **Cho FontAtlas**: dùng localhost server nhỏ hoặc custom URL scheme mà một Figma plugin gọi (giống RightFont) [I].

## 3. Affinity

Không tìm thấy SDK hay plugin cho font. Các manager dựa vào cơ chế request font của hệ thống (FontAgent, Typeface), hoặc Application Sets (Extensis). Affinity hiển thị "Missing Fonts" và nhận font activate khi đang chạy. [V] ([Affinity help](https://affinity.help/photo/en-US.lproj/pages/Appendix/installingFonts.html)). Typeface có "Font Switch" cho Affinity by Canva ([trang](https://typefaceapp.com/help/articles/font-switch/affinity-canva)), chưa đọc chi tiết.

## 4. Cơ chế chung

**Font request của hệ thống.**
- `CTFontManagerSetAutoActivationSetting` chỉ bật cho app gọi. Một thread Apple ghi nhận auto-activation không hoạt động với app sandbox, không có phản hồi chính thức của Apple. [V] ([thread 691900](https://developer.apple.com/forums/thread/691900))
- `CTFontManagerCreateFontRequestRunLoopSource(order, callback)`: callback nhận dict thuộc tính font và PID app yêu cầu; deprecated từ macOS 11, ghi "sẽ bị gỡ"; trên macOS 27 beta 5+ không nhận request khi sandboxed; bỏ `app-sandbox` thì chạy lại. Giải pháp Apple (Quinn) chấp nhận: XPC service/launch agent không sandbox đi kèm app. [V] ([thread 845853](https://developer.apple.com/forums/thread/845853)). Typeface xác nhận trên macOS 27, auto activation cho "other apps" chỉ chạy với app không sandbox, và bản Mac App Store cần helper. [V]
- Không tìm thấy API công khai nào cho app bên thứ ba trở thành "font provider" trên macOS. Font provider extension của iOS (`CTFontManagerRegisterFontURLs` với scope user chỉ là đăng ký font cho app/user, không phải callback khi font thiếu) [I]. `CTFontDescriptorMatchFontDescriptorsWithProgressHandler` phục vụ font tải về của Apple, không có hook cho bên thứ ba [I]. `CTFontManagerRequestFonts` là API iOS, không tìm thấy nguồn xác nhận trên macOS.

**Parse tài liệu** [I, chưa kiểm chứng; nên spike]:
- IDML: ZIP/XML, `Resources/Fonts.xml` liệt kê font, dễ. `.indd`: nhị phân, chỉ quét chuỗi heuristic, độ chính xác thấp.
- `.sketch`: ZIP + JSON (`fontDescriptor.attributes.name`), dễ. `.docx`: `word/fontTable.xml` và `rFonts`, dễ.
- `.ai`: PDF-compatible, đọc `/FontName`/`FontDescriptor`, trung bình (chỉ đủ nếu file có PDF compat).
- `.psd`: descriptor Type Layer (`TySh`/engine data), trung bình. `.key`: IWA (Snappy + protobuf), khó.
- `.fig`: định dạng đóng (Kiwi), khó. Nên lấy qua Figma plugin. `.afdesign`: định dạng đóng, chưa có nguồn.

**FSEvents / Recent documents.** Quan sát file sidecar hoặc `~/Library/Application Support/com.apple.sharedfilelist` nhận biết file vừa mở; kết hợp parse ở trên. Cần Full Disk Access tùy đường dẫn [I].

**Pháp lý.** Activate cục bộ từ thư viện của chính người dùng là không phát sinh phân phối. Font hosted/cloud (Adobe Fonts, Monotype) phải theo EULA riêng; không copy font giữa máy. Giữ nguyên chính sách "tool, not gatekeeper": chỉ hiện thông báo trách nhiệm.

## 5. Khuyến nghị theo giai đoạn

| Phase | Việc | Giá trị / công sức | Ký/entitlement |
|---|---|---|---|
| 1 | "Activate fonts used by this document" (kéo-thả/file picker) cho IDML, .sketch, .docx, .ai, .psd; view **Missing fonts** (so tên font với thư viện, gợi ý kích hoạt Case). Giữ session scope + reason counting hiện có | Cao / thấp | Không cần thêm |
| 2a | **Spike** helper non-sandboxed dùng `CTFontManagerCreateFontRequestRunLoopSource`, bao phủ Affinity, Photoshop, After Effects, Sketch, Office thuộc diện non-sandbox cùng lúc. Đo hành vi thực trên macOS 27 | Rất cao / trung bình, rủi ro API bị gỡ | Developer ID, notarization, hardened runtime; helper là launch agent riêng; không lên Mac App Store |
| 2b | Plugin Adobe (InDesign/Illustrator/Photoshop/After Effects) hoặc điều khiển bằng Apple Events như Typeface. Chọn: Automation trước (rẻ, Typeface chứng minh được), plugin C++ chỉ nếu cần đọc danh sách font của tài liệu; UXP chưa đủ API [V] | Cao / cao | `NSAppleEventsUsageDescription` + `com.apple.security.automation.apple-events` (hardened runtime), người dùng cấp quyền Automation từng app |
| 3 | Figma plugin + server localhost; Sketch plugin; Pages/Keynote chỉ nếu Apple cho phép; Resolve/C4D xem sau | Trung bình / trung bình | Server localhost cần xử lý CORS Private Network Access [V cho Chrome 94+, docs.rs] |

Lưu ý quan trọng:
- Các manager lớn đều đang vướng lỗi Adobe theo từng bản (Illustrator 29.8). Cần ma trận test theo phiên bản Adobe.
- Mọi điểm đánh dấu [I] nên được kiểm chứng bằng spike nhỏ trước khi cam kết (đặc biệt Phase 2a).
