# 07 — Lối vào Pairing Desk và hành vi Compare/Pairing

**Ngày:** 2026-11-07

**Người dùng:** Graphic Designer, Art Director, Layout Artist, UI/Web designer.

**Liên quan:** `06_v3_pairing_compare.md` (bố cục, mode); `01_research.md` §2–6; `docs/team/research/06–09`.

**Plan triển khai:** `docs/team/plans/2026-11-07-font-desk-plan.md`, nằm ở checkout chính.

Ký hiệu **[INF]** đánh dấu chỗ là suy luận, không có nguồn trực tiếp.

## Quyết định đã chốt (chủ app, 2026-11-07)
1. Font Desk khi chưa có điểm xuất phát dùng **Brief starter**: một màn, bỏ qua được. **Không** làm wizard nhiều bước, vì concept E đã bị loại.
2. Chọn **2–4 font** trong Library sẽ có nút **"Pair these"**: engine tự gán vai rồi xếp hạng các Combo.
3. AI sau này chỉ dùng **Claude Code CLI có sẵn trên máy**. API key và MCP để sau. **Phần AI làm ở một session khác.** Bản này chỉ lưu ý tưởng (Phụ lục).
4. Bổ sung mọi dữ liệu mà công cụ phát hiện tự động được. Phần **Phân loại** phải làm đủ (xem plan, mục D).

---

## 1. Hành vi khi so sánh (Compare)
Mục đích là **loại bớt** trong một shortlist 2–4 font. Họ hiếm khi so 20 font.

| Hành vi | Hệ quả cho app | Nguồn |
|---|---|---|
| Thử bằng nội dung thật (tên brand, headline, đoạn văn) và một chuỗi soi anatomy | Một ô text dùng chung cho mọi cột | blog.typekit.com/2012/05/23/type-study-pairing-typefaces/ |
| So ở đúng cỡ sẽ dùng: "đẹp ở 72px có thể vỡ ở 16px". Thử thêm italic, bold, small caps, chuỗi dài | Mỗi hàng là một cỡ: Display / Heading / Body / Caption | designproject.io/resources/guides/choose-font-pairings/ |
| Xét theo nhiều lớp: tone → cấu trúc chữ → độ dễ đọc ở cỡ body → nội dung thật | Thứ tự hàng trong bảng đi theo đúng các lớp này | betterwebtype.com/is-ai-better-at-picking-and-pairing-fonts-than-you/ |
| Đưa về cùng tỷ lệ (cùng size hoặc cùng x-height) trước khi đánh giá | Có nút bật/tắt "Same size / Match x-height" | madegooddesigns.com (chỉ đọc được snippet) |
| Chỉ khớp x-height là chưa đủ; cap height và ascender cũng ảnh hưởng | Hiện đủ ba chỉ số kèm Δ | lightrun.com, thảo luận của Font Bakery |
| Muốn overlay glyph nhưng công cụ hiện tại rất kém | Chế độ overlay căn chung baseline. **Điểm khác biệt so với đối thủ** | forum.fontlab.com/fontlab-8/ensuring-similar-glyphs-have-a-similar-width/ |
| Hiện phải xuất ảnh từng font rồi ghép tay | Xuất PNG/PDF cả bảng so sánh | wildandfreetools.com/blog/fp-compare-fonts-side-by-side |
| License quyết định font có vào shortlist hay không | Hiện license ngay trong bảng, không chặn | klim.co.nz/licences · subtraction.com/2015/05/26/fontstand/ |
| Tiếng Việt: dấu chồng (ổ ề ậ), chữ hoa có dấu va vào dòng trên | Dòng test tiếng Việt cố định, cảnh báo khi dấu tràn | vietnamesetypography.com/design-challenges/ · github.com/uswds/public-sans/issues/85 |
| Đem nhiều phương án đi trình khách hàng | Xuất PDF | typefaceapp.com/help/articles/pairing-fonts [INF một phần] |

**Thông tin cần thấy khi so sánh:**
- Specimen ở nhiều cỡ.
- x-height, cap height, width, contrast.
- Dải weight, có italic thật không, variable axes.
- Tiếng Việt và độ phủ ngôn ngữ.
- OpenType: figures, small caps.
- License.

Mỗi chỉ số hiện **Δ so với baseline**. Phần OpenType, italic và kerning là [INF]: đây là nhu cầu suy ra, chưa có khảo sát nào liệt kê.

## 2. Hành vi khi ghép cặp (Pairing)
- **Ít font là mặc định.** "Hai font là đủ, ba font là hiếm." Một family duy nhất thường cũng đủ dùng. Mỗi font giữ một vai cố định. Quy tắc "serif + sans" là một huyền thoại. (practicaltypography.com/mixing-fonts.html)
- **Bốn chiến lược ghép:**
  - superfamily;
  - chung chỉ số (x-height, aperture) nhưng không giống tới mức trông như đặt nhầm;
  - tương phản rõ ở một vài trục;
  - cùng thời kỳ hoặc cùng công cụ tạo chữ.

  (Typekit, như trên)
- **Superfamily là một hệ thống.** Có thể đổi font trong cùng superfamily mà không làm vỡ layout. (fonts.google.com/knowledge, đọc qua tóm tắt)
- **Cùng designer hoặc foundry là lối tắt đáng tin**, ví dụ Atlas + Lyon. (Butterick)
- **Nguyên tắc "contrast, not conflict"**: cặp font phải vừa có điểm giống thấy được, vừa có điểm khác thấy được. (typetype.org/font-pairs/)
- **Thứ tự quen thuộc**: chọn font body → gán vai → thêm tối đa một font heading → thử trong layout thật. (designproject.io)
- Theo nghiên cứu nội bộ (`01_research.md`), AD thường **bắt đầu từ một font neo** (headline hoặc wordmark), nghĩ theo "giọng", và so 2–3 phương án.
- **Tìm cảm hứng từ cách font đã được dùng thật**: Fonts In Use, Typewolf. (blog.hubspot.com/marketing/how-to-pick-the-perfect-font-pairings · typewolf.com/blog/google-fonts-combinations)

**Thông tin cần thấy khi ghép cặp:**
- Combo đặt trong layout thật.
- Lý do theo từng trục: ◐ Harmony, ● Contrast, ◆ Clash.
- Kiểm tra parity: tiếng Việt, italic, OpenType.
- License của từng font.
- Monotype cảnh báo khi chọn style quá đậm làm body. (support.monotype.com/en/articles/9189890-font-pairing)

## 3. Đối thủ và khoảng trống

| Công cụ | Bắt đầu từ | Ghi chú |
|---|---|---|
| Typeface (Mac) | Các font đã chọn (⌘K) + font neo | Có template, ←/→ để đổi ứng viên, overlay A/B, xuất PDF. Một số tính năng cần bản Pro |
| Monotype pairing | Một font + vai của nó | Bộ lọc "some contrast / same family", có like/dislike. Chỉ ghép serif/sans |
| Fontjoy | Ngẫu nhiên hoặc khóa một font | Không hiểu ngữ cảnh; khoảng 1/6 gợi ý hỏng |
| Adobe Fonts | Trang family, hoặc tìm bằng ảnh | Chỉ gợi ý font trong kho Adobe; bị chê "kết quả vô dụng" |
| Google Fonts | Trang specimen | "Popular pairings" không giải thích lý do [INF] |
| Wordmark.it | Text người dùng gõ | Hiện text đó bằng mọi font đã cài; chỉ chạy trên Chrome/Edge |
| FontBase / RightFont | Family / Library | Preview chậm; không có chế độ compare |
| FontAgent | Chọn nhiều font | Có Compare view |

Nguồn: typefaceapp.com/help/articles/pairing-fonts; support.monotype.com/en/articles/9189890-font-pairing; betterwebtype.com; blog.adobe.com/en/publish/2021/05/04/choose-fonts-faster-with-new-recommendations-feature; community.adobe.com (visual search useless results); wordmark.it/help/how-does-it-work/; forum.fontba.se/t/body-text-preview-in-family-view/384; insidersoftware.com/fontagent-compare-view.

**Khoảng trống [INF]:**
1. Không công cụ nào kết hợp ý nghĩa (brief, vai) với chỉ số hình học.
2. Không đưa các font về cùng kích thước thị giác, không hiện Δ.
3. Thiếu kiểm tra ngôn ngữ (tiếng Việt) và license.
4. Thường chỉ gợi ý font trong kho riêng của họ. **FontAtlas làm việc trên font người dùng thực sự có trong máy.**
5. Hầu như không công cụ nào xuất file để trình khách hàng.

---

## 4. Hướng A: bắt đầu từ Library (lối vào chính)
Lối vào nằm ở `SelectionBar` (trên TopBar). Nút thay đổi theo số font đang chọn:

| Đang chọn | Nút | Hành vi |
|---|---|---|
| 1 | **Pair** · Find similar | Mở Case mới, font này là neo ở vai Display |
| 2–4 | **Compare** · **Pair these** | **Compare**: font đang focus là baseline (cột cố định), các font còn lại là cột. **Pair these**: thử mọi cách gán vai, trả 2–3 Combo kèm lý do |
| >4 | Compare (cảnh báo nhẹ) | Cuộn ngang, gợi ý thu hẹp, không chặn |
| Đang mở Case | + **Add to Case** | Thêm font vào picker hoặc làm cột Compare |

- Phím tắt `⇧P` / `⇧C`.
- Chuyển giữa Compare và Pairing vẫn giữ context (đúng như 06 v3).
- Chọn nhiều style trong cùng một family vẫn tính là nhiều font, vì người dùng có thể đang so weight.

## 5. Hướng B: Font Desk khi chưa có điểm xuất phát → Brief starter
- Chỉ hiện khi Case còn trống. Bỏ qua được bất kỳ lúc nào.
- Gồm một màn, khoảng 30 giây:
  - **Dùng cho**: Editorial · Branding · Poster · UI/Web · Packaging
  - **Giọng**: 2–3 thanh trượt dạng cặp đối lập (Cổ điển ↔ Đương đại, Ấm ↔ Lạnh, Trầm ↔ Nổi bật)
  - **Ràng buộc**: Cần tiếng Việt (bật mặc định) · Chỉ font đã cài · Có italic · Variable · License thương mại
- Ánh xạ chip sang filter dùng `fontatlas-describe` cùng các class mới, sau đó engine đề xuất **3 Combo khởi đầu** lấy từ Library của người dùng và chuyển sang luồng Pairing.
- Chừa sẵn một chỗ "Kết nối AI" cho session sau.

---

## Phụ lục: Ý tưởng AI, làm ở session khác

**Cách kết nối** (tham khảo app vixio/KIRA, `apps/desktop/src-tauri/src/lib.rs`):
- tìm `claude` trên PATH;
- `claude auth status` để kiểm tra đăng nhập;
- nếu chưa đăng nhập, mở Terminal chạy `claude auth login`;
- liệt kê model bằng handshake `initialize` qua stream-json (không tốn token);
- gọi `claude -p … --output-format json --tools "" --no-session-persistence`.

Paseo chỉ dùng để tham khảo.

**Nguyên tắc** (theo `research/08_ai_mcp.md`):
- AI **không xếp hạng font**.
- Chỉ gửi text (tên + chỉ số), không gửi file font hay ảnh render.
- Coi tên và metadata font là dữ liệu không tin cậy.

Mọi kết quả AI tạo ra phải thuộc một trong ba loại:
- **(a)** filter mà người dùng sửa được;
- **(b)** text có gắn nhãn AI;
- **(c)** gợi ý mà người dùng phải xác nhận.

Khi không có CLI, app dùng bản không AI.

| # | Ý tưởng | Loại kết quả | Ưu tiên |
|---|---|---|---|
| 1 | Dịch brief thành filter (vai, Pairing Style, độ chặt, trục, ràng buộc) | (a) | P1 |
| 2 | Tinh chỉnh bằng hội thoại ("ấm hơn") → Δ trên trục → engine tìm lại | (a) | P1 |
| 3 | Nội dung mẫu tiếng Việt theo ngữ cảnh, có câu thử dấu chồng | (b) | P1 |
| 4 | Phê bình Combo kiểu AD và gợi ý trục nên đổi | (b)+(a) | P1 |
| 5 | Tóm tắt bảng Compare trong 3 dòng | (b) | P2 |
| 6 | Văn bản rationale để trình khách hàng | (b) | P2 |
| 7 | Làm giàu phân loại, thời kỳ, font "anh em", đối chiếu với chỉ số đo được | (c) | P2 |
| 8 | Diễn giải license sang ngôn ngữ thường (không phải tư vấn pháp lý) | (b) | P2 |
| 9 | Đề xuất type scale (cỡ, line-height có tính dấu chồng, tracking) | (b) | P2 |
| 10 | Gợi ý sắp xếp Library, gom bản trùng | (c) | P3 |
| 11 | Ảnh moodboard của người dùng → filter (song song tìm bằng SigLIP local) | (a) | P3 |
| 12 | Chiều ngược lại qua MCP sidecar (`fontatlas-mcp`) | — | P3 |
