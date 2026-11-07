# 01 — Nghiên cứu: không gian ghép chữ theo use case

> **Cập nhật vòng 2 (2026-11-06):** chủ app đã chốt tên **Pairing Desk** (tính năng) và **Case** (đơn vị); không dùng "Plate". Bỏ định hướng F&B, đối tượng là Graphic Designer / Art Director / Layout Artist. Hệ từ, persona và layout mới: xem `05_layout_options_v2.md`. Nội dung dưới đây giữ nguyên làm lịch sử quyết định.


- **Ngày:** 2026-11-06 · **Người viết:** Visual/UX worker (phiên font-desk) · **Nhánh:** `docs/font-desk` (từ `dev/mvp` 484b29d)
- **Tên tạm trong yêu cầu:** "Font Desk". Tên chọn ở `02_naming.md` (**Plate**). File này dùng "Font Desk" khi trích yêu cầu, còn lại dùng "Plate".
- **Nhãn mức chắc chắn:** **[XM]** đã xem nguồn gốc hoặc tài liệu nội bộ đã kiểm · **[TT]** qua bên thứ ba hoặc kết quả tìm kiếm · **[SL]** suy luận của tôi, cần kiểm bằng người dùng thật.

---

## 1. Người dùng

**Chân dung chính.** Art Director hoặc designer người Việt, làm agency hoặc freelance. Mảng mạnh: F&B (quán, chuỗi, bếp gia đình làm thương hiệu), branding cho cửa hàng nhỏ và vừa. Sản phẩm giao: menu, bảng hiệu, bao bì và nhãn, brand sheet, poster sự kiện. Thư viện vài nghìn font, trộn font mở (Google Fonts), font mua, font Việt hoá. [SL, theo hồ sơ chủ app và spec MVP §1.1]

**Bối cảnh làm việc.**
- Khách duyệt bằng mắt, trên một trang hoàn chỉnh ("cho anh xem cái menu"), không duyệt một dòng pangram.
- Chữ luôn có dấu chồng: PHỞ, BÚN CHẢ, HỦ TIẾU, NƯỚC MẮM. Chữ hoa tiêu đề kiểu `ỮA, Ặ, Ệ, Ỗ` là chỗ hay vỡ (research 09 H3). [XM]
- Số và giá xuất hiện dày: `65.000đ`, `120K`, `250g`, `HSD 06/11/2026`. Font đẹp ở chữ nhưng số xấu là lỗi gặp ở menu. [SL]
- Thường làm 2–3 hướng song song để khách chọn. [SL]

## 2. Việc cần hoàn thành (job-to-be-done)

> Khi tôi đã có **một font neo** cho thương hiệu (khách đưa, hoặc tôi vừa tìm ra), tôi muốn **dựng nhanh cả hệ chữ quanh nó trên đúng loại ấn phẩm tôi đang làm**, so vài phương án cạnh nhau và hiểu vì sao từng cặp hợp hay lệch, để **chốt 2–3 font tự tin** và **quay lại tiếp sau** mà không phải dựng lại.

Tách thành 5 việc nhỏ:

| # | Việc | Hiện làm bằng | Đau |
|---|---|---|---|
| J1 | Từ một font, tìm font **giống** (thay thế, bản rẻ hơn, bản đủ tiếng Việt) | Typewolf, Google Fonts, trí nhớ | Không biết font nào trong máy mình giống; không có lời "giống ở đâu" |
| J2 | Từ một font, tìm font **ghép được** cho vai trò khác | Fontjoy, Fontpair, Pinterest | Gợi ý chỉ từ Google Fonts, không có font của mình; không giải thích |
| J3 | **Xem cặp trong bố cục thật** của use case (menu có giá, nhãn có khối lượng) | Mở Figma/Illustrator, dựng lại file mẫu, đổi font tay | Chậm; đổi một font phải đổi ở chục text box |
| J4 | **So nhiều phương án cạnh nhau** trước khi chọn | Nhân bản artboard | Rối, khó nhớ phương án nào dùng font gì |
| J5 | **Giữ lại** phương án, mở lại khi khách phản hồi | Lưu file, chụp màn hình | Mất ngữ cảnh: vì sao đã bỏ font kia |

## 3. Pain point cụ thể

1. **Pangram không nói được gì.** Nghiên cứu 06 §1.3 rút ra: specimen đáng tin là văn bản thật trong bố cục thật (Typeface presets, Google Fonts nhiều khối, Fonts In Use ảnh thật). [XM]
2. **Kiểm tiếng Việt phải làm riêng.** Không công cụ ghép cặp nào kiểm dấu chồng; font thiếu một chữ `ữ` sẽ bị hệ thống chen font khác vào giữa từ (09 H1). [XM]
3. **Không ai giải thích theo trục.** FontBase dùng trục để lọc, Monotype có lọc "có tương phản / cùng family", Fontjoy có một thanh trượt; không ai nói "x-height gần nhau, tương phản nét khác rõ" (06 §1.3). [XM]
4. **Gợi ý là hộp đen hoặc bộ sưu tập biên tập**, nằm ngoài thư viện của người dùng. [XM, xem §5]
5. **Thiếu chỗ "nháp so sánh".** Công cụ hoặc thay font thẳng vào bố cục (mất phương án cũ), hoặc chỉ cho xem danh sách (không thấy trong bố cục). Typeface 4.3 giải quyết bằng cách nhân bản cả studio. [TT]
6. **Quá nhiều font.** Butterick: hầu hết tài liệu chịu được hai font, ít tài liệu cần ba (09 D2). Công cụ cho đổi font từng khối dễ dẫn tới 5–6 font trên một menu. [XM]

## 4. Hành vi ghép font thật

Tổng hợp từ 07, 09 và các nguồn web §7. Đây là cách AD thực sự nghĩ, không phải cách thuật toán chạy.

- **Bắt đầu từ một neo.** Thường là font tiêu đề hoặc wordmark (cá tính nhất), rồi tìm font đọc. Ít khi bắt đầu từ font nội dung. Jiang et al. cũng mô hình hoá cặp là **bất đối xứng** tiêu đề → nội dung (07 §1b). [XM]
- **Nghĩ theo giọng, không theo từng khối.** Một menu có 8 khối chữ nhưng chỉ 2–3 font; mỗi font lo một nhóm vai trò bằng đổi cỡ, độ đậm, chữ hoa, khoảng chữ (09 D1, D2). Hệ quả thiết kế: **chọn font theo "giọng", vai trò thừa hưởng giọng** (xem 04 §3). [XM + SL]
- **Có vài "kiểu ghép" quen tay** (đều có trong 09):
  - cùng họ hoặc superfamily (C2): an toàn nhất;
  - có chân + không chân (D4): lối tắt khi phân vân;
  - trưng bày + đọc (D7): tiêu đề nhiều cá tính, nội dung trung tính;
  - giữ một, đổi một (D5): giống cấu trúc, khác bề mặt;
  - một họ nhiều độ đậm (concord thuần);
  - phá cách có chủ đích (Thử nghiệm): biết mình đang phá luật nào.
- **Cùng khung, khác da là bẫy.** Hai sans gần giống (kiểu Montserrat + Poppins) bị coi là lỗi; luật R8 của nhóm bắt đúng loại này. [XM nội bộ]
- **Kiểm bằng nội dung thật của khách.** Tên quán, tên món, giá. Từ dài nhất và chữ hoa có dấu chồng là phép thử. [SL]
- **So 2–3 phương án, không 20.** Fontjoy cho khoá một font và sinh lại phần còn lại; Typeface cho duyệt ← → trong bố cục đứng yên. Cả hai cho thấy so sánh tuần tự trong bố cục cố định là thói quen chính, so song song dùng khi chốt. [TT + XM]

## 5. Phân tích đối thủ và công cụ liên quan

Truy cập ngày 2026-11-06 trừ khi ghi khác. Tóm tắt bằng lời của tôi.

| Công cụ | Cách bắt đầu | Gợi ý | Bố cục xem | So song song | Giải thích | Lưu | Bài học cho Plate |
|---|---|---|---|---|---|---|---|
| **Typeface 4.3 — Pairing Studio** (macOS) | Chọn font trong thư viện, mở từ menu Font hoặc ⌘K | Không gợi ý; người dùng tự đưa ứng viên vào | Preset bố cục (Logo, Article, Manifesto, Hierarchy); mỗi khối chữ gắn một font, danh sách font ở sidebar | Nhân bản studio (⌘K), đặt cạnh nhau | Cố ý không phán xét | Theo phiên | Gần nhất với ý chủ app. **Khoảng trống:** không gợi ý, không giải thích, không tiếng Việt [TT changelog 4.3, help "Pairing Fonts"; XM ảnh trong 06] |
| **Fontjoy** | Bấm Generate | Mạng nơ-ron trên Google Fonts | 3 khối dọc: tiêu đề, phụ đề, thân | Không | Một câu chung | Không | **Khoá một font, sinh lại phần còn lại** và **thanh trượt giống ↔ khác** là hai thao tác đáng học [TT fontjoy.com, madegooddesigns] |
| **Fontpair** (web + plugin Figma) | Duyệt bộ sưu tập | Hơn nghìn cặp biên tập tay, Google Fonts | Trang mẫu; sửa được chữ | Không | Không | Trên web | Gợi ý biên tập là "điểm khởi hành" tốt cho người mới, nhưng tách khỏi thư viện người dùng [TT fontpair.co FAQ, Figma page] |
| **Google Fonts — specimen** | Trang một font | Từng có mục "Popular pairings"; trang specimen dựng trên lưới 4 cột để chừa chỗ module ghép cặp | Nhiều khối Heading/Body, chọn ngôn ngữ (có Vietnamese) | Không | Bài Knowledge tách riêng | Danh sách chọn | Chọn ngôn ngữ đổi chữ mẫu: ý hay cho tiếng Việt. Trạng thái hiện tại của mục pairings chưa xác minh [TT design.google, wptavern, hashrocket] |
| **Adobe Fonts — Recommendations** (2021) | Trang một font | Sensei: font tương tự + carousel cặp header/body, có cờ "không thích" | Carousel mẫu | Không | Không | Web project | Phản hồi nhẹ (cờ) giúp học sở thích; hộp đen [TT blog.adobe.com 2021-05-04] |
| **Monotype Fonts — Font Pairing** | Icon pair cạnh font, chọn vai Headline/Body | ML + chuyên gia; lọc "có tương phản / cùng family" | Danh sách cặp | Không | Không | Tim, thêm vào library | Chọn **vai của font neo** trước là đúng; lọc theo "kiểu ghép" là tiền lệ cho style pair [XM help center, qua 06] |
| **FontBase** | Playground: trang tài liệu giả H1/H2/P, chọn Font A/B cho từng khối | Không | Trang tài liệu, chỉnh cỡ, line-height | Không | Không (trục chỉ để lọc) | Không | Trục đo được chỉ dùng lọc → khác biệt của FontAtlas là giải thích gắn trên specimen [XM, qua 06] |
| **Typewolf** | Trang một font | Biên tập: similar, thay thế miễn phí, suggested pairing, site thật dùng font | Ảnh website thật | Không | Lời biên tập | Không | Cặp kèm **bằng chứng dùng thật** có sức thuyết phục; FontAtlas chỉ có dữ liệu của chính người dùng [XM, qua 06] |
| **RightFont 5** | Thư viện | Không thấy tính năng ghép cặp chính thức; FAQ nói về xem trước và so sánh trước khi kích hoạt | Lưới | So sánh nhanh | Không | Library | Không có đối thủ trực tiếp ở mảng ghép trong các font manager macOS trừ Typeface [TT rightfontapp.com/faqs] |
| **Plugin Figma** (Fontpair, Type System Builder, Typescale) | Trong file Figma | Bộ cặp tuyển chọn; preset thang cỡ (Product UI, Editorial, Marketing) | Thang cỡ chữ áp vào file | Không | Không | Text style Figma | **Preset theo loại ấn phẩm + thang cỡ** là cách nghĩ AD đã quen [TT figma.com, fontpair.co/figma, gumroad] |
| **Brand guideline** (hệ thống chữ của các thương hiệu) | — | — | Thứ tự vai trò eyebrow/label → heading → body → CTA; eyebrow chữ hoa, khoảng chữ rộng, thường dùng font phụ | — | — | — | Nguồn cho danh sách vai trò ở 04 §3 [TT brand.athenahealth.com, cps.edu, f5xc theme] |
| **Hướng dẫn menu** (blog thiết kế, nguồn mỏng) | — | — | Ba tầng: tên mục, tên món, mô tả; khoảng 4–6 pt giữa các tầng; giá cùng cỡ mô tả; tranh luận giá căn phải có dấu chấm dẫn hay đặt cuối mô tả | — | — | — | Mode Menu cần **hai kiểu đặt giá** để AD chọn [TT, nguồn SEO — chỉ dùng làm điểm khởi đầu] |

### Kết luận đối thủ
1. **Không ai làm đủ ba thứ cùng lúc:** gợi ý từ thư viện của chính người dùng + giải thích theo trục + xem trong bố cục use case thật có tiếng Việt. Typeface có bố cục, Fontjoy có gợi ý, FontAtlas có trục. Plate ghép ba thứ đó. [XM + TT]
2. **Thao tác đã được thị trường chứng minh:** khoá một font và sinh lại phần còn lại (Fontjoy); duyệt ← → trong bố cục đứng yên (Typeface); nhân bản để so (Typeface); chọn vai của font neo trước (Monotype); preset loại ấn phẩm (plugin Figma).
3. **Chỗ FontAtlas có lợi thế riêng:** kiểm tiếng Việt trên **chính chữ của bố cục** (không chỉ "font đủ 134 ký tự"), số đo thật trong câu giải thích, không gác cổng.

## 6. Insight

| # | Insight | Hệ quả thiết kế |
|---|---|---|
| I1 | AD chọn **giọng** (2–3 font), không chọn font cho từng khối | Mô hình hai lớp: **Giọng** (font) → **Vai** (cỡ, độ đậm, hoa/thường) |
| I2 | Neo đến trước, vai của neo quyết định gợi ý | Bước đầu tiên duy nhất: chọn neo + vai của neo (mặc định Tiêu đề) |
| I3 | Thử trong bố cục thật mới tin, nhưng thay thẳng thì mất phương án cũ | Tách **thử** (xem tạm) khỏi **đặt** (ghi vào Plate); có **bàn so sánh** giữ ứng viên |
| I4 | "Giống" và "ghép được" là hai câu hỏi khác nhau, dùng chung một chỗ chọn | Bàn so sánh có các nguồn ngang hàng: Cùng họ · Giống · Ghép · Thư viện · Đã ghim |
| I5 | Kiểu ghép là ngôn ngữ chung của nghề | "Kiểu ghép" thành bộ lọc trên nguồn Ghép, ánh xạ vào chế độ An toàn / Có kiểm soát / Thử nghiệm và luật R1–R9 |
| I6 | Lỗi tiếng Việt chỉ lộ ra ở chữ cụ thể | Kiểm phủ ký tự trên **văn bản của Plate**; chỉ rõ chữ nào, vai nào |
| I7 | Công việc kéo dài nhiều ngày, nhiều vòng khách | Tự lưu, đánh sao, mở lại đúng trạng thái; giữ lý do bỏ font (ghim + ghi chú) |
| I8 | Một trang thật nhiều chữ, giao diện phải lùi lại | Canvas là nhân vật chính; nhãn trục ở cột lề và bàn so sánh, không đè lên chữ ở độ sâu 0 |

## 7. Nguồn (truy cập 2026-11-06)

**Nội bộ (đã đọc):** spec MVP §6–7 (`docs/superpowers/specs/2026-10-16-fontatlas-mvp-design.md`); research 06, 07, 08, 09, 10 (`docs/team/research/`); luật `docs/team/eval/rules_plan3.md`; `docs/design/03_ux_flows.md` §1, §6.4, §7.3, §8.4; `docs/design/01_ui_hien_tai_va_tinh_nang.md`; `docs/design/04_ui_direction_notes.md`; ảnh `docs/design/screenshots/round2-native/`; code `UX/components/detail/tabs/PairTab.tsx`, `UX/api/index.ts`, `UX/bindings/PairCandidateDto.ts`.

**Web:**
- Typeface — Pairing Fonts (help): https://typefaceapp.com/help/articles/pairing-fonts
- Typeface 4.3 changelog: https://typefaceapp.com/changelog/typeface-4-3-0
- Typeface 4 trên App Store: https://apps.apple.com/app/id1062679359
- Fontjoy: https://fontjoy.com ; hướng dẫn: https://madegooddesigns.com/fontjoy/
- Fontpair FAQ: https://www.fontpair.co/faqs-for-fontpair ; plugin Figma: https://fontpair.co/figma
- Adobe — Choose fonts faster with new recommendations feature (2021-05-04): https://blog.adobe.com/en/publish/2021/05/04/choose-fonts-faster-with-new-recommendations-feature
- Google Design — Reimagining Google Fonts: https://design.google/library/reimagining-google-fonts ; WP Tavern — Google Fonts Gets a Redesign: https://wptavern.com/google-fonts-gets-a-redesign ; Hashrocket TIL — Popular pairings: https://til.hashrocket.com/posts/ozlmklqh24-google-font-popular-pairings
- RightFont FAQ: https://rightfontapp.com/faqs
- Figma — AI typography pairer: https://www.figma.com/solutions/ai-typography-pairer/ ; Type System Builder: https://brandingboard.gumroad.com/l/type-system-builder-figma-plugin
- Hệ chữ trong brand guideline: https://brand.athenahealth.com/guidelines/typography ; https://www.cps.edu/media/cps-branding/typography ; https://f5xc-salesdemos.github.io/docs-theme/typography
- Thứ bậc chữ trên menu (nguồn blog, độ tin thấp): https://menutype-i5.pages.dev/how-to-select-typefaces-for-cafe-food-list-hierarchy-menu-hierarchy-guides ; https://madegooddesigns.com/menu-design/
- Nguồn học thuật về cặp bất đối xứng (qua 07): Jiang et al., *Visual Font Pairing*, arXiv:1811.08015

**Giới hạn:** chưa phỏng vấn người dùng thật; hành vi ở §4 dựa trên tài liệu và kinh nghiệm nghề được ghi trong 09. Các số cỡ chữ trên menu từ blog chỉ là điểm khởi đầu. Không tự chạy thử Typeface 4.3 trong phiên này.
