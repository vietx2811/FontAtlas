# Spec UX — Thẻ font, Top Bar, Sidebar (mục A, B, C) + bộ lọc taxonomy và tag người dùng

Tác giả: FontAtlas·01 PM (chỉ spec, không code) · Ngày 2026-11-11 · Nhánh `docs/library-view-options` (từ dev/mvp `14f0435`)
Nguồn yêu cầu: `docs/team/plans/2026-11-11-owner-feedback-batch.md` mục A1–A3, B4–B5, C6 (commit 28af552) và lời nhắn của Lead (thêm phần D: lọc theo taxonomy mới + tag phong cách/cảm xúc do người dùng gắn, từ `docs/team/research/2026-11-11-font-classification-taxonomy.md` mục 4, commit 4c9e086).
Người code: worker 05, sau khi xong Font Desk.

Nguyên tắc chủ app áp dụng: **nghiên cứu hành vi người dùng trước** (mục 2), rồi mới đề xuất (mục 3). Thuật ngữ font giữ tiếng Anh kèm giải thích tiếng Việt; mọi term có `TermTip`; chuỗi qua i18n; cảnh báo/chi tiết theo motif `NoteMark` (dấu "!" đỏ ở góc + tooltip); mọi thao tác dùng được bằng bàn phím.

Quy ước nguồn: **[code]** đọc từ mã trên dev/mvp, **[web]** trang công khai (URL ở mục 8), **[TK]** thiết kế của tôi, chưa có nguồn ngoài, **[?]** chưa xác minh được.

---

## 0. Tóm tắt và các quyết định cần Lead / chủ app

Đề xuất chính (đọc 2 phút):

1. **Chấm bật/tắt** nằm đầu hàng tên, là nút thật (toggle, `aria-pressed`), vùng bấm 24 px, hình chấm 8 px. Bật = chấm xanh đặc có quầng; tắt = chấm xám đặc không quầng; "có sẵn trên máy" (bản giống hệt do bên khác cung cấp) = vòng xanh rỗng, không bấm được (giữ hành vi hiện tại). Xoá nút `.fa-pow` và chấm `.fa-dot` góc phải.
2. **Badge ngôn ngữ** bỏ hẳn xanh/hổ phách: chữ mã ngôn ngữ + vòng độ phủ, toàn bộ màu trung tính (đậm/nhạt theo trạng thái). Hình vòng (đầy / gần kín / cung / gạch) vẫn là tín hiệu chính, nên không mất thông tin. Chỉ chấm bật/tắt còn màu xanh trên thẻ.
3. **Một popover "Hiển thị" (View options)** duy nhất, mở bằng nút trên Top Bar hoặc **⌘J** (Finder và Lightroom đều dùng ⌘J cho View Options; ⌘J đang trống trong app), gồm 3 khối: *Gộp & sắp xếp*, *Thông tin trên thẻ*, và liên kết "Bố cục lưới/danh sách" vẫn ở nút riêng bên cạnh.
4. **Phễu trong ô tìm kiếm** thay nút "Advanced": icon phễu ở mép phải ô, chấm đếm khi có bộ lọc, giữ ⌥⌘F và `data-testid="advanced-open"`.
5. **Sắp xếp**: Tên · Ngày thêm vào thư viện · Năm phát hành (ước lượng) · Ngày tạo file · Độ đậm (Weight) · (Khớp, chỉ khi đang tìm). Mỗi trường có nhãn nguồn và độ tin cậy ngay trong popover. **Nhóm theo Family** là tiêu đề nhóm có thể thu gọn (kiểu Finder "Group"), không gộp mỗi family thành một thẻ (kiểu Typeface).
6. **Menu chuột phải cho Sidebar** dùng chung `ui/Menu` (thay `.fa-menu` tự chế đang có), mở được bằng phím Menu / ⇧F10, trả focus đúng mục. Mô hình dữ liệu theo loại mục để Case/Smart cắm vào sau.
7. **Lọc theo taxonomy mới và tag người dùng** (mục D): thêm 2 thẻ vào AdvancedSheet (Đặc điểm / Traits; Phong cách & cảm xúc) và chuyển lọc thẻ vào `filters` để phễu, chip và "Đặt lại" nhất quán.

**Cần quyết định (xếp theo mức chặn):**

| # | Câu hỏi | Đề xuất của tôi | Ai quyết |
|---|---|---|---|
| Q1 | Taxonomy nghiên cứu đề xuất class `Display`; chủ app đã quyết 2026-11-08 "Display là usage tag, không phải class" (`traits[]`). Spec theo quyết định của chủ app: stencil/inline/pixel/shaded là **traits**, Display là **use tag**. | Giữ quyết định chủ app; Lead xác nhận để 02 không làm class Display. | Lead |
| Q2 | "Nhóm theo Family": tiêu đề nhóm (mọi kiểu vẫn hiện) hay một thẻ đại diện mỗi family? | Tiêu đề nhóm, mặc định **tắt** (giữ hành vi hiện tại). | Chủ app |
| Q3 | Badge ngôn ngữ thiếu ký tự: giữ hổ phách cũ hay trung tính hoàn toàn? Theo ghi nhớ "note-mark" (missing glyphs = xám) và yêu cầu A2, tôi chọn trung tính. | Trung tính. | Chủ app |
| Q4 | "Ngày phát hành": file font **không có** trường ngày phát hành. Chỉ có năm trong copyright, `head.created`, `head.modified`. | Gọi là "Năm phát hành (ước lượng)", hiện nguồn + độ tin cậy, font không rõ luôn xếp cuối. | Chủ app |
| Q5 | Sắp xếp/nhóm/thông tin thẻ lưu **toàn cục** hay theo từng view (Finder lưu theo từng thư mục)? | Toàn cục; riêng "Gần đây" và "Đang bật" khoá thứ tự. | Lead |
| Q6 | Backend 02 cần thêm trường (mục 6). Thứ tự làm: ngày + badge nhiều ngôn ngữ trước, traits/tag sau. | Như mục 7. | Lead |

---

## 1. Hiện trạng trong code (dev/mvp `14f0435`)

| Khu vực | Hiện trạng [code] |
|---|---|
| Thẻ font | `UX/features/library-grid/FaceCell.tsx`: nút nguồn `.fa-pow` (icon power, ẩn tới khi hover, `right:46px`) + chấm `.fa-dot` (7 px, góc phải, `--on`) là hai thứ riêng; sao yêu thích góc phải; hàng `.fa-meta` = tên · định dạng file (`.fa-fmt`) · `LangBadge`. Chấm "available" là vòng xanh rỗng (`data-scope="available"`) và khi có nó thì không có nút nguồn. |
| Badge ngôn ngữ | `LangBadge.tsx`: mã `VI` + vòng 14 px, bốn hình (đầy / cung gần kín / cung theo tỷ lệ / vòng rỗng gạch). Màu: `ok` = `--on` (xanh), `near` = `--accent-ink`, `miss` = `--warn`. Một badge, ngôn ngữ theo `query.badgeLang` (mặc định `vie`, hoặc ngôn ngữ đầu tiên đang lọc). `FaceRow.langBadge` là **một** đối tượng. |
| Top Bar | `TopBar.tsx`: tiêu đề · `SearchField` · nút `Ặ vi` · `AdvancedFilters` (nút chữ "Advanced", `data-testid="advanced-open"`, ⌥⌘F) · `TopBarActions` · nhóm Lưới/Danh sách (`layout`). |
| Ô tìm kiếm | `SearchField.tsx`: icon · input · `FilterSummary` (tóm tắt chip) + `ActiveFiltersPopover` · nút xoá (Esc) hoặc gợi ý ⌘K. Backspace trên ô rỗng gỡ nhóm bộ lọc cuối. |
| AdvancedSheet | `filter/AdvancedSheet.tsx`: thẻ Phân loại (class → subclass, có đếm), Style (Variable, Italic), Tiếng Việt, Ngôn ngữ (`LangPicker`), Thư viện (Yêu thích, Nguồn), Giấy phép, 18 trục chia nhóm. Đếm điều kiện bằng `filterChips()`. |
| Sắp xếp / nhóm | `FaceQuery.sort` = `'family' \| 'weight' \| 'added' \| 'match'` + `descending`. **Không có UI** sắp xếp, **không có** nhóm theo family: `LibraryGrid` ảo hoá bằng `react-window` `List`, hàng đồng chiều cao. `FaceRow.addedAt` có; **không có** năm phát hành hay ngày file. |
| Lưu giao diện | `ui_state` (`sidebarCollapsed`, `openSections`, `layout`, `sampleSize`, `infoExpanded`) + `api.getSetting/setSetting` khoá `ui.*` (regex `^ui\.[a-z0-9_.-]{1,60}$`, ví dụ `ui.recent_views`). |
| Sidebar | `sidebar/Sidebar.tsx`: mục hệ thống (Thêm font, Tất cả, Yêu thích, Gần đây, Đang bật, Trùng lặp, "Đang so sánh"), Dự án (collections), Nguồn, Thẻ. Menu dự án là `.fa-menu` tự chế (nút "…", chỉ Đổi tên / Xoá dự án, **không** có phím mũi tên, không trả focus). Nguồn **không có** menu. Mục Case/Smart **chưa có** trong Sidebar. |
| Menu chung | `ui/Menu.tsx`: portal, mũi tên/Home/End/Enter/Esc, submenu →/←, `reason` cho mục vô hiệu, `returnFocus`, `checked`. `FaceCell` đã mở nó qua chuột phải / phím Menu / ⇧F10 (`faces/faceMenuModel.ts`). |
| API sẵn có | `renameCollection`, `deleteCollection`, `activationActivateCollection` / `activationDeactivateCollection`, `removeSource`, `rescanSources`, `revealLocation`, `listTags/createTag/tagFaces/untagFaces`, `classFacets`. **Không có**: đổi tên / xoá thẻ, xoá lịch sử "Gần đây" (làm ở client bằng `setSetting('ui.recent_views','[]')`). |
| Năm phát hành | Chỉ trong `FontExtras.years {created, modified, copyright}` (fontatlas-core `extras.rs`), chưa vào `FaceRow`, chưa sort được. |

---

## 2. Nghiên cứu hành vi: các app khác làm thế nào

Mục đích: biết người dùng **đã quen** gì, rồi mới chọn. Bảng ghi điều xác minh được; chỗ không xác minh được ghi rõ [?], không điền bằng trí nhớ.

### 2.1 Chấm / nút bật-tắt font

| App | Hành vi | Nguồn |
|---|---|---|
| FontBase | Một **vòng tròn ở bên trái tên font**; bấm để kích hoạt. Font mới thêm vào ở trạng thái chưa kích hoạt; font bật giữ bật khi app mở, tắt khi đóng app, khôi phục lần mở sau. Trang không nêu màu, không nêu bấm lần nữa có tắt không, không nêu bật hàng loạt. | [web] fontba.se/learn/font-activation |
| Typeface | Bật / tắt cho **font đang chọn** bằng phím: ⌘; bật, ⇧⌘; tắt, ⌥⌘; bật tạm thời; ⇧⌘, xoá bật tạm. Thẻ "Activated / Deactivated" là tag hệ thống để lọc. | [web] typefaceapp.com/help/articles/keyboard |
| Mail (macOS) | Trạng thái bộ lọc biểu thị bằng **đổi màu nút**, không thêm nhãn chữ. | [web] support.apple.com/guide/mail/mlhl1f6cf15a/mac |

Kết luận cho A1: vị trí **bên trái tên** là chuẩn của FontBase (app cùng loại, cùng người dùng); bật hàng loạt qua phím tắt theo lựa chọn (Typeface) — FontAtlas đã có ⌘⇧A. Ai cũng dựa vào **màu** cho trạng thái, nên FontAtlas thêm tín hiệu thứ hai không phải màu (quầng) [TK].

### 2.2 Chọn thông tin hiện trên thẻ ("View Options")

| App | Hành vi | Nguồn |
|---|---|---|
| Finder | **View > Show View Options (⌘J)**: popover theo từng chế độ xem. *Sort By* nằm trong đó; *Group* là nút trên thanh công cụ; "Show Item Info" bật thêm dòng phụ dưới biểu tượng (kích thước ảnh, số mục…). Cài đặt **theo từng thư mục**, giữ tới khi đổi. Ở List view: chuột phải tiêu đề cột để bật/tắt từng cột; bấm tiêu đề cột để sắp xếp, bấm lại để đảo chiều. | [web] support.apple.com/guide/mac-help/mchlp1745/mac; MacSales (mô tả "Show Item Info") |
| Lightroom Classic | **View > View Options (⌘J)** tab Grid View; "Show Grid Extras" có 3 trạng thái: Expanded / Compact / ẩn, **phím J xoay vòng** giữa chúng; tách riêng *badges* (⌘⌥⇧H bật/tắt) khỏi *extras*; người dùng chọn nhãn chữ cho ô (vd. tên file + đuôi file). | [web] jkost.com (thứ cấp); helpx.adobe.com (tìm kiếm trả về mô tả, trang gốc trả 403 khi tôi tải) |
| Apple Photos | **View > Metadata > Titles** bật tên dưới ảnh; **View > Sort** theo album, một số album tự động bị khoá thứ tự (mục xám). Không có sắp xếp theo tên file. | [web] Apple Community (diễn đàn, bản Catalina; có thể đã đổi) |
| Typeface | Cùng một nút "view" trên thanh công cụ vừa chọn Grid/Column/List, vừa **Group by Family** (⌥G). Mặc định gộp 1 xem trước mỗi family, nút góc phải thẻ hiện số kiểu; bấm để mở tất cả. | [web] typefaceapp.com/help/tips/group-fonts-by-family; …/keyboard |
| FontBase | Từ bản 2.20.1 có **nút sắp xếp ở bên phải ô tìm kiếm**; danh sách tuỳ chọn khi bấm. Diễn đàn người dùng xin: tên, foundry, ngày thêm, số kiểu, theo tag. Trang không nêu chiều đảo. | [web] forum.fontba.se/t/add-the-ability-to-sort-fonts/27; changelog (Softpedia) qua tìm kiếm |

Kết luận cho A3/B5: (1) người dùng quen **một** chỗ Hiển thị mở bằng ⌘J; (2) Lightroom cho thấy **tách "thông tin" khỏi "huy hiệu"** và có chế độ gọn/đầy đủ xoay vòng; (3) Photos cho thấy mục **bị khoá thì xám, không biến mất**, kèm lý do; (4) nhóm theo Family có hai trường phái (Typeface gộp thẻ; Finder tiêu đề nhóm) — xem Q2; (5) Finder lưu theo thư mục, FontAtlas ít view nên toàn cục (Q5).

### 2.3 Tìm kiếm + bộ lọc

| App | Hành vi | Nguồn |
|---|---|---|
| RightFont | Ô tìm kiếm trên thanh công cụ (⌘F mở Unified Search); **panel bộ lọc** lọc theo Category (serif, sans, script, display…; font chưa phân loại gán bằng menu chuột phải), thuộc tính (width, weight, x-height…), Language (dropdown); nút ↻ đặt lại. Trang không nêu cách hiển thị bộ lọc đang bật. | [web] rightfontapp.com/docs/search-fonts |
| FontBase | Nút sắp xếp **bên phải ô tìm kiếm** (xem trên); tìm bằng cú pháp (`tags:`, `weight:`…). | [web] forum.fontba.se; Softpedia |
| Mail (macOS) | **Nút Filter ở đầu danh sách**, đổi màu khi có bộ lọc; chọn từ menu có dấu tích; tắt bằng chính nút đó. | [web] support.apple.com/guide/mail/mlhl1f6cf15a/mac |
| Figma | Bộ chọn font: mở bằng tên font; "All fonts" mở danh sách bộ lọc; nhớ bộ lọc cuối trong phiên. Lọc theo phân loại vẫn là *yêu cầu tính năng* trên diễn đàn, chưa thấy trong tài liệu. | [web] help.figma.com (qua tìm kiếm); forum.figma.com (yêu cầu) |

Kết luận cho B4: nút bộ lọc **cạnh ô tìm kiếm** (FontBase) hoặc đổi màu khi có bộ lọc (Mail) đều là mẫu quen; thêm **chấm đếm** là bổ sung của FontAtlas vì đã có nhiều loại bộ lọc cùng lúc [TK]. "Đặt lại" nằm trong panel (RightFont ↻) — FontAtlas đã có ("Đặt lại" trong sheet).

### 2.4 Menu chuột phải cho mục Sidebar

| App | Hành vi | Nguồn |
|---|---|---|
| Finder | Control-click mục sidebar tuỳ chỉnh có **"Remove from Sidebar"**; gỡ mục tiêu chuẩn chỉ tắt lựa chọn trong Settings, **không xoá gì trên đĩa**. Apple dạy cách *kéo ra khỏi sidebar*; trang Apple tôi tải không nhắc menu chuột phải. | [web] cultofmac.com, mactrast.com (thứ cấp); support.apple.com/guide/mac-help/mchl83c9e8b8 |
| Photos | Không tìm được nguồn công khai mô tả menu chuột phải của album trong sidebar. | [?] |
| RightFont, FontBase, Typeface, Connect/Suitcase | Tag/bộ sưu tập: FontBase "right-click on a single selected font or fonts and add tags" (menu trên **font**, chưa nêu cho mục sidebar); RightFont trang Tags trả 404; Extensis chuyển hướng sang Monotype Connect, không tải được nội dung. | [web] fontba.se (release notes qua tìm kiếm); [?] cho phần sidebar |

**Giới hạn của nghiên cứu mục này:** tôi **không có nguồn công khai** cho menu chuột phải sidebar của app quản lý font nào. Menu ở mục C vì vậy dựa trên (a) quy ước macOS (chuột phải = thao tác với mục đó, không đổi view đang xem), (b) mô hình menu font đã có ở FontAtlas (`faceMenuModel.ts`), (c) tính an toàn dữ liệu. Nên có một lượt thử với 2–3 nhà thiết kế (câu phỏng vấn gợi ý ở mục 9) trước khi chốt tên mục.

---

## 3. Spec chi tiết

### A1. Chấm bật/tắt đầu tên font

**Bố cục** (thẻ lưới và danh sách):

```
┌──────────────────────────────┐
│ [specimen preview]        ☆  │   ← góc phải chỉ còn sao
│                              │
│ ● Hamburgefonts Regular  TTF │   ← hàng 1: chấm · tên · định dạng
│   VI ◉  RU ◔                 │   ← hàng 2: thông tin theo "Hiển thị" (A3)
└──────────────────────────────┘
```

Hàng 1 = `[chấm] [tên font, co lại có ellipsis] [định dạng]`. Định dạng (`.fa-fmt`) nằm sau tên như hiện tại; nếu A3 tắt "Định dạng file" thì không có. Hàng 2 chỉ xuất hiện khi có ít nhất một thông tin bật (A3).

**Trạng thái** (một nút, `aria-pressed` cho hai trạng thái bấm được):

| Trạng thái | Dữ liệu | Hình | Bấm |
|---|---|---|---|
| Tắt | không có trong `active`, không `available` | chấm đặc 8 px, `--muted` 55% | bật (qua `useActivationFlow`, có hộp thoại xung đột như hiện nay) |
| Bật · phiên này | `activeScope = session` | chấm đặc `--on` + quầng 3 px | tắt |
| Bật · luôn bật | `activeScope = persistent` | như trên, thêm vòng ngoài 1 px (phân biệt với "phiên") | tắt |
| Có sẵn trên máy | `available` (bản giống hệt do bên khác cung cấp) | **vòng** xanh rỗng 8 px, không quầng | không bấm được (giữ hành vi hiện tại: không có nút nguồn); tooltip nói ai cung cấp |
| Đang chuyển | đang chờ `setActive` | vòng quay 8 px, `aria-busy="true"` | bỏ qua bấm tiếp |
| File mất | `row.missing` | chấm xám gạch chéo, `aria-disabled`, tooltip lý do + `NoteMark` đỏ nếu cần nêu rõ | không |
| (Dành cho mục E) File trên cloud chưa tải | dataless | chấm xám + icon mây 10 px ở hàng 2; bấm sẽ tải rồi bật, trạng thái "Đang chuyển" kèm tiến độ | tải + bật |

Tín hiệu không phải màu: bật có **quầng**, "luôn bật" có **vòng ngoài**, "có sẵn" là **vòng rỗng**, "đang chuyển" **quay**. Ba trạng thái xanh/xám còn lại vẫn đọc được khi mù màu nhờ quầng + `aria-pressed` + tooltip.

**Kích thước:** hình 8 px, vùng bấm 24×24 px (WCAG 2.2 mục 2.5.8 "Target Size Minimum" là 24 CSS px [TK, chuẩn W3C]). Vùng bấm nằm trong phần đệm trái của thẻ, không đẩy tên lệch hẳn; tên căn lề trái với chấm trong chế độ danh sách.

**Tooltip** (qua `Tooltip`, không dùng `title`): `Tắt Active` / `Bật Active` giữ khoá hiện có `act.card.on/off`; chấm "có sẵn" giữ `act.available.dot`. `Active` ở đây là thuật ngữ → thêm `TermTip` `term.active` (giải thích: font đang được đăng ký với hệ điều hành nên app thiết kế dùng được, tắt thì không).

**Bàn phím:** nút nằm trong thứ tự Tab của thẻ ngay trước tên? Hiện cả thẻ là một `gridcell tabIndex=0` và nút con chặn `keydown` nổi bọt. Giữ như vậy: Tab từ thẻ vào nút chấm → Enter / Space bật tắt (không chọn thẻ). Ngoài ra ⌘⇧A (đã có) bật/tắt cho cả lựa chọn. `aria-label` **cố định** = `Active` + tên font (`aria-pressed` mang trạng thái, không đổi nhãn theo trạng thái — APG toggle button).

**Việc xoá:** `.fa-pow` (nút), `.fa-dot` góc phải; CSS `.fa-cell:hover .fa-pow…`, `data-layout="list" .fa-pow`. Giữ `data-testid="cell-active"` trên nút mới và `active-dot` cho ba trạng thái dot cũ để QA không đổi chuỗi test; cập nhật các test chạm hai phần tử đó.

**Hover:** không còn nút ẩn tới khi hover, nên bật/tắt khám phá được bằng mắt ngay (khác hiện nay).

### A2. Badge ngôn ngữ không dùng xanh

Thay đổi trong `LangBadge`/`libraryGrid.css`:

| Trạng thái | Chữ mã | Vòng | Màu |
|---|---|---|---|
| `full` đủ | `VI` | đầy | `--fg` 80% (chữ), vòng đặc cùng màu |
| `near` gần đủ ≥ 90% | `VI` | cung gần kín | `--fg` 80% |
| `partial` / `low` / `below` thiếu | `VI` | cung theo tỷ lệ / vòng rỗng gạch | `--muted` (xám) |
| `unknown` | không hiện | | |

- Bỏ `--on` (xanh), `--accent-ink` (nâu) và `--warn` (hổ phách) khỏi `.fa-lang-badge`; **chỉ chấm bật/tắt còn màu xanh trên thẻ** (yêu cầu A2).
- Bốn hình giữ nguyên (đã được thiết kế để khác HÌNH, không chỉ khác màu), nên không mất tín hiệu. Tooltip giữ khoá `badge.*`.
- Lý do chọn trung tính thay vì đỏ: theo motif `NoteMark` đã chốt, cảnh báo thật mới có dấu "!" đỏ; thiếu ký tự một ngôn ngữ không phải lỗi của file, nên glyph thiếu = xám (Q3).
- Nếu **ngôn ngữ đang lọc** là ngôn ngữ của badge và font **thiếu** ký tự (tức là font không đáp ứng bộ lọc đang chọn — chỉ xảy ra khi `langMode='any'` hoặc lọc `near`), hiện thêm `NoteMark` đỏ nhỏ với tooltip "Thiếu {miss}/{total} ký tự {lang}". Đây là ngoại lệ có chủ đích: thông tin này *ảnh hưởng quyết định* của người đang lọc.
- Nhiều ngôn ngữ: xem A3 (nhóm "Ngôn ngữ"); mỗi badge cùng quy tắc màu.

### A3. Panel "Hiển thị" (View options)

**Điểm vào:** nút icon "sliders/hiển thị" trên Top Bar, đặt **trước** nhóm Lưới/Danh sách; `aria-haspopup="dialog"`, `aria-expanded`; phím **⌘J** (chuẩn Finder/Lightroom, đang trống) bật/tắt; tooltip có `shortcut="⌘J"`. Popover neo vào nút (không modal), Esc đóng và trả focus về nút, bấm ngoài đóng. Rộng 320 px, cuộn nếu cao.

**Bố cục popover:**

```
Hiển thị                                     [Đặt lại]
────────────────────────────────────────────────────
Gộp & sắp xếp
  Gộp theo Family (?)                         [ ○ ]
  Sắp xếp theo                  ( Tên            ▾ )
                                [ A → Z ][ Z → A ]
  (nhãn nguồn + độ tin cậy của trường đang chọn)

Thông tin trên thẻ
  ☑ Định dạng file (TTF, OTF…) (?)
  ☑ Ngôn ngữ (?)            [VI ×] [RU ×] [+ Thêm…]
  ☐ Phân loại (Class · Subclass) (?)
  ☐ Số kiểu trong family (?)
  ☐ Variable (?)
  ☐ Giấy phép (?)
  ☐ Nguồn / ổ lưu
  ☐ Năm phát hành (ước lượng) (?)
  ☐ Thẻ của tôi (?)
```

(?) = `TermTip` hoặc `Tooltip` có giải thích; mọi từ chuyên ngành (Family, Variable, Class/Subclass, Format, Hyperglot coverage…) có.

**Danh sách thông tin và mặc định** (chuỗi chính qua i18n):

| Mã (`field`) | Hiển thị trên thẻ | Mặc định | Dữ liệu cần |
|---|---|---|---|
| `format` | `TTF` / `OTF` / `TTC`… | bật (như nay) | `path` có sẵn |
| `lang` | một badge cho mỗi ngôn ngữ đã chọn (tối đa 3 + "+N") | bật, `['vi']` | **cần backend** `langBadges[]` (mục 6) |
| `class` | `Sans · Geometric` (chữ nhỏ; kèm `NoteMark` "!" nếu `confidence` dưới ngưỡng, theo motif) | tắt | có ở `classFacets`/detail; cần đưa vào `FaceRow` |
| `styles` | `12 styles` (số kiểu trong family) | tắt | cần đếm theo family |
| `variable` | `VAR` + số trục (`wght`, `wdth`…) | tắt | `isVariable` có; số trục cần thêm |
| `license` | `OFL` / `Apache` / `Khác` / `Chưa rõ` | tắt | `licenseClass` có |
| `source` | tên nguồn; icon mây nếu nguồn là ổ cloud | tắt | `sourceId`→tên có ở store; cần `sourceId` trong `FaceRow` |
| `year` | `2019 ≈` (nhãn "≈ ước lượng", tooltip nguồn) | tắt | mục 6 |
| `tags` | tối đa 3 tag + "+N" | tắt | tag theo face |

Ghi chú:
- **Chấm bật/tắt, tên, sao** không nằm trong danh sách vì là điều khiển, luôn hiện.
- Danh sách theo hiện trạng yêu cầu của chủ app ("tag ngôn ngữ, phân loại, số kiểu, biến thiên, giấy phép, nguồn/ổ…") cộng `format` (đang hiện) và hai mục ứng viên `year`, `tags`.
- **Hàng 2 trên thẻ:** các mục đã bật xếp ngang, cách nhau khoảng nhỏ, một dòng, thừa thì "+N" (tooltip liệt kê phần ẩn). Khi hàng 2 trống, thẻ thấp hơn (không chừa chỗ trống). Chiều cao thẻ trong lưới ảo hoá thay đổi theo số hàng, nên `boxHeight` phải tính lại khi bật/tắt (một hằng số cộng thêm, không đo từng thẻ).
- **Danh sách (list layout):** cùng trường, nhưng nằm ngang cột cố định thay vì hàng 2; nếu hẹp, ưu tiên theo thứ tự trong bảng trên.
- **Ngôn ngữ:** chọn bằng `LangPicker` có sẵn (tái dùng, dạng popover con); mặc định `vi`; **ngôn ngữ đang lọc** (`filters.langs`) luôn được thêm tự động vào đầu danh sách trong lúc lọc (hành vi `badgeLang` hiện có, mở rộng thành nhiều). Giới hạn 3 badge/thẻ để thẻ không vỡ; vượt thì "+N".
- **"Đặt lại"** đưa **cả hai khối** (Gộp & sắp xếp, Thông tin trên thẻ) về mặc định; mỗi ngôn ngữ trong danh sách có "×" riêng. Không có hộp xác nhận (bật lại là rẻ).
- **Lưu:** khoá `ui.card_fields` (JSON `{"v":1,"fields":["format","lang"],"langs":["vi"]}`) và `ui.sort` (`{"v":1,"by":"family","desc":false,"groupFamily":false}`) bằng `setSetting`, debounce như `ui.recent_views`; đọc ở `hydrate`. Dữ liệu hỏng / khoá lạ → bỏ qua và dùng mặc định (không báo lỗi).
- **Lightroom-style xoay vòng** (Gọn / Chuẩn / Chi tiết bằng một phím): **hoãn**, ghi vào backlog; hiện chỉ cần hộp kiểm.

**Trạng thái và lỗi:** nếu một trường chưa có dữ liệu (chưa phân tích), mục vẫn bật được nhưng thẻ chưa phân tích hiện `—` xám; không ẩn cả hàng (tránh thẻ nhảy chiều cao khi phân tích xong).

### B4. Phễu trong ô tìm kiếm

```
[🔍 | Tìm font ...  chip tóm tắt…   ⌘K |  ▽³ ]
                                          └ phễu + chấm đếm
```

- **Vị trí:** mép phải **trong** ô tìm kiếm, sau vạch ngăn mảnh. Từ phải sang trái: phễu · (nút xoá khi có chữ, hoặc gợi ý ⌘K) · `FilterSummary` · input. Nút xoá ✕ và phễu là **hai** nút riêng.
- **Trạng thái phễu:**

| Trạng thái | Hình |
|---|---|
| Không có bộ lọc | icon phễu viền, `--muted` |
| Có N bộ lọc | phễu đặc `--accent-ink`, chấm đếm N ở góc trên phải (`N` từ `filterChips().length`, cùng cách đếm với "Đặt lại" trong sheet; ≥ 10 hiện `9+`) |
| Sheet đang mở | `aria-expanded="true"`, nền nhấn |

  Cách báo bằng đổi màu + đếm tương đương nút Filter của Mail (đổi màu khi có bộ lọc) [web]; chấm đếm là thêm [TK].
- **Hành vi:** bấm phễu hoặc ⌥⌘F mở `AdvancedSheet` như hiện nay (không đổi sheet). Bấm lần nữa đóng. Sheet đóng thì focus về phễu (đã có `opener`).
- **`aria-label`** = `Bộ lọc nâng cao` + (nếu có) `, {N} đang bật`; tooltip kèm `⌥⌘F`. `data-testid="advanced-open"` chuyển sang phễu để QA native không đổi.
- **Nhập liệu:** bấm vào phần trống của ô vẫn focus input (như nay); bấm phễu **không** focus input. Trong `<label>`: dùng `<div class="fa-search">` thay `<label>` hoặc `stopPropagation` ở phễu để không bị label kích hoạt input; dev chọn cách nào qua test focus.
- **Hẹp:** dưới ~420 px ô tìm kiếm ẩn gợi ý ⌘K trước, rồi `FilterSummary` thu về chỉ số đếm; phễu luôn hiện.
- **Bỏ:** nút chữ "Advanced" (`adv.open` chỉ còn dùng cho `aria-label`/tooltip).
- Nút `Ặ vi` giữ nguyên (không thuộc yêu cầu). Lưu ý: bật `Ặ vi` đặt `filters.vi='full'`, nên **tăng đếm phễu thành 1** — nhất quán với hành vi sheet hiện tại.

### B5. Nhóm theo Family và sắp xếp

**Điểm vào:** khối "Gộp & sắp xếp" của popover Hiển thị (A3). Ngoài ra phím **⌥G** bật/tắt gộp theo Family (cùng phím Typeface [web]) khi không gõ chữ.

**Sắp xếp theo** (chọn một; dùng `<select>` tuỳ biến hoặc radiogroup của `Menu` có `checked`):

| Trường | `sort` | Định nghĩa | Nguồn dữ liệu | Độ tin cậy | Chiều mặc định |
|---|---|---|---|---|---|
| Tên | `family` | Family rồi Subfamily; trong cùng family xếp Weight tăng, đứng trước nghiêng | tên font | cao | A→Z |
| Ngày thêm vào thư viện | `added` | Lúc FontAtlas lần đầu thấy face (`FaceRow.addedAt`) | `faces.added_at` | cao, **nhưng** lần quét đầu cả thư viện cùng giờ, nên chỉ hữu ích cho font thêm sau | Mới→Cũ |
| Năm phát hành (ước lượng) | `released` (mới) | Năm trong dòng copyright; nếu không có thì năm `head.created`; nếu không có nữa thì `head.modified` | `FontExtras.years` | **trung bình-thấp**: copyright thường là năm xuất bản đầu; `head.created` nhiều nhà làm font đặt theo lúc build/tool; có font để 0/1904 | Mới→Cũ |
| Ngày tạo file | `file_created` (mới) | Thời điểm file xuất hiện trên đĩa (birth time macOS / creation time Windows) | hệ điều hành lúc quét | **thấp** làm "ngày font ra đời": sao chép/tải lại/đồng bộ cloud đặt lại ngày; file cloud chưa tải có thể trả ngày khác [?] | Mới→Cũ |
| Độ đậm (Weight) | `weight` | `usWeightClass` | OS/2 | cao | Nhẹ→Nặng |
| Khớp | `match` | Độ liên quan khi có chữ tìm; **chỉ hiện khi `query` không rỗng** | backend | | Khớp nhất trước |

- **Nhãn nguồn ngay trong popover:** dưới trường đang chọn, một dòng chữ nhỏ + `NoteMark` "i" (không đỏ nếu chỉ là thông tin): ví dụ "Năm phát hành ≈ ước lượng từ dòng copyright (hoặc `head.created`). Font không có thông tin xếp cuối." Đây là yêu cầu "ghi rõ nguồn và độ tin cậy" của Lead.
- **Font không có giá trị** (năm không rõ, ngày file không đọc được): luôn **xếp cuối** bất kể chiều, và hết hàng đó là cùng bậc sắp theo Tên. Không bao giờ trộn vào giữa.
- **Chiều:** hai nút đối ngược cạnh nhau; nhãn đổi theo trường (`A → Z / Z → A`, `Mới → Cũ / Cũ → Mới`, `Nhẹ → Nặng / Nặng → Nhẹ`). Đổi *trường* thì chiều đặt về mặc định của trường đó (lựa chọn thiết kế của tôi, Finder không cho nguồn rõ); đổi *chiều* thì giữ trường.
- **Khoá theo view:** "Gần đây" và "Đang bật" (đang dùng `sort='match'` để giữ thứ tự đưa vào) → hàng Sắp xếp **xám, không bấm được**, `reason` = "Thứ tự theo lần mở gần nhất" / "Thứ tự theo lần bật" (kiểu Photos: mục khoá hiện xám) [web]. "Nhóm theo Family" vẫn dùng được trong hai view đó (gom theo thứ tự family đầu tiên xuất hiện).
- **Cột trong chế độ danh sách:** Lightroom/Finder List cho bấm tiêu đề cột để sắp xếp; chế độ danh sách của FontAtlas hiện không có hàng tiêu đề. Ngoài phạm vi; ghi backlog.

**Nhóm theo Family** (tiêu đề nhóm, mặc định tắt):

```
▾ Fraunces                                    9 kiểu
   [thẻ][thẻ][thẻ][thẻ]
   [thẻ][thẻ][thẻ][thẻ]
   [thẻ]
▸ Inter                                      18 kiểu   (đang thu gọn)
▾ Newsreader                                  8 kiểu
```

- Tiêu đề nhóm: nút thu gọn (`aria-expanded`), tên family vẽ bằng font đại diện (như tên thẻ hiện nay vẽ bằng font của nó, dùng lại `useLoadedFamily`), số kiểu, và số lọc khớp nếu đang lọc (`4/9 kiểu`, cùng quy ước chip Family hiện có). Chiều cao tiêu đề cố định (ví dụ 32 px).
- Mỗi nhóm bắt đầu hàng mới, hàng cuối nhóm không lấp bằng thẻ nhóm sau.
- Trạng thái thu gọn: **trong phiên**, không lưu; đổi view / truy vấn → mở hết. Phím: Enter / Space trên tiêu đề thu gọn/mở; ←/→ đóng/mở khi tiêu đề có focus; ↑/↓ chuyển giữa tiêu đề và thẻ.
- **Chọn nhiều:** chọn dải (⇧) và chọn tất cả đi theo thứ tự hiển thị; nhóm thu gọn bị bỏ qua. Bấm vào tiêu đề **không** chọn font. (Chọn cả family: dùng "Xem family" hiện có.)
- **Khi sắp xếp ≠ Tên:** nhóm sắp theo khoá của family = giá trị cực trị theo chiều đang chọn (mới nhất khi Mới→Cũ, cũ nhất khi Cũ→Mới, Weight: nhẹ nhất / nặng nhất); trong nhóm vẫn xếp Weight tăng. Vẫn không bao giờ xé family ra hai nơi.
- **Kỹ thuật cho 05 và 02** (vì lưới ảo hoá bằng `react-window` theo hàng cố định): cần một lệnh trả **chỉ mục nhóm** của truy vấn hiện tại: `[ {family, start, count} ]` (vài nghìn dòng, không tải face), để client dựng bố cục hàng (tiêu đề + các hàng thẻ) và `rowHeight` thành hàm theo chỉ số; `pagedFaces` giữ nguyên phân trang theo vị trí face. Không lọc/sắp xếp ở client (nguyên tắc store/ui).

### C6. Menu chuột phải cho mục Sidebar

**Cách mở:** chuột phải trên mục; phím Menu hoặc ⇧F10 khi mục có focus (giống `FaceCell`); nút "…" hiện có của Dự án vẫn giữ (để khám phá được) nhưng mở **cùng** `Menu` neo theo `rect`. Bỏ `.fa-menu` tự chế (không có phím mũi tên, không trả focus).
**Hành vi chung:**
- Chuột phải **không đổi view đang xem** (quy ước Finder: menu tác động lên mục chạm tới) [TK/quy ước macOS; xem giới hạn ở 2.4]. Mục chạm tới được tô nét viền (`data-context="true"`) trong lúc menu mở, để biết menu thuộc mục nào.
- `Menu` nhận `label` = tên mục (đọc bởi trình đọc màn hình), `returnFocus` = nút của mục; Esc đóng và trả focus.
- Mục vô hiệu hiện xám + `reason` (đã có sẵn trong `Menu`), không ẩn đi.
- Hành động không hoàn tác được → hộp xác nhận bằng `Modal` hiện có, nút huỷ là mặc định; hành động hoàn tác được → thông báo (`notices`) có "Hoàn tác" nếu có API.

**Mục theo loại** (hàm thuần `buildSidebarMenu(kind, item, ctx)` trả `MenuEntry[]`, kiểm thử được như `faceMenuModel.ts`):

| Loại mục | Mục menu (thứ tự) | Ghi chú |
|---|---|---|
| **Nguồn** (`src:`) | Mở · **Hiện trong Finder** (`revealLocation(path)`; vô hiệu + lý do nếu nguồn chưa kết nối) · **Quét lại** (`rescanSources([id])`; vô hiệu khi đang quét) · — · **Gỡ nguồn…** (danger) | Gỡ nguồn: hộp xác nhận nêu rõ "Không xoá file font; chỉ gỡ khỏi thư viện FontAtlas" và nêu số font bị ảnh hưởng, kèm điều gì xảy ra với dự án/thẻ — **cần 02 xác nhận** `removeSource` làm gì với dự án/thẻ/yêu thích trước khi viết câu này. Mục "Luôn giữ trên máy" (ổ cloud, mục E) để dành, thêm sau. |
| **Dự án** (`col:`) | Mở · **Đổi tên** (F2) · **Bật tất cả font** / **Tắt tất cả font** (`activationActivate/DeactivateCollection`; xác nhận nếu > 200 font, nêu số lượng) · Đổi màu ▸ (submenu bảng màu) · — · **Xoá dự án…** (danger, xác nhận) | Đổi tên: ô nhập nội tuyến có sẵn; xong thì focus về mục. Xoá dự án không xoá font (ghi trong xác nhận). Xoá xong focus sang mục kế hoặc "Tất cả". Hiện nay xoá **không** xác nhận → đổi thành có. |
| **Thẻ** (`tag`) | Lọc theo thẻ này · **Đổi tên…** · **Xoá thẻ…** (danger) | **Cần API mới** (`renameTag`, `deleteTag`); nếu chưa có thì chỉ làm "Lọc theo thẻ này" ở đợt này và ghi rõ. |
| **Gần đây** | Xoá lịch sử "Gần đây"… | Client: `setSetting('ui.recent_views','[]')` và xoá store; xác nhận. |
| **Đang bật** | **Tắt tất cả** | Có đếm trong nhãn ("Tắt tất cả (214)"); xác nhận từ 50 font. |
| **Tất cả / Yêu thích / Trùng lặp** | (không menu) | Chuột phải trên mục không có menu → không gì xảy ra, **không** hiện menu hệ thống. |
| **Đang so sánh** | Tiếp tục · **Đóng phiên so sánh** | `closeCompare` + xoá `compareSession`. |
| **Case / Smart** (chưa có trong Sidebar) | Mở · Đổi tên · Nhân đôi · Xoá… | Khi các mục này lên Sidebar, cắm vào `buildSidebarMenu`; không làm đợt này. |
| **Vùng trống của Sidebar** | Thêm font… · Dự án mới… | Tuỳ chọn, nhỏ. |

**Phím tắt:** F2 đổi tên dự án khi mục có focus (hiện trong `hint` của menu). Delete **không** xoá trực tiếp (quá nguy hiểm) — mở hộp xác nhận.

---

## 4. Mục D (Lead bổ sung): lọc theo taxonomy mới và tag phong cách/cảm xúc

**Phạm vi UI.** Backend phân loại là việc của 02 (D1); spec này chỉ mô tả **cách lọc** để 05 làm khi dữ liệu có.

### D1. Bộ lọc phân loại (mở rộng thẻ "Phân loại")

Taxonomy §4: class chính `Serif, Slab, Sans, Script, Handwriting, Mono, Blackletter, Symbol, Uncertain` (không có class `Display`; xem Q1), mỗi class ≤ ~5 subclass; `primary` duy nhất để lọc/nhóm, `secondary` chỉ để hiển thị và tìm (taxonomy §5.2).

- Giữ cấu trúc hiện tại (class có đếm; chọn class mới hiện subclass có đếm). Thêm hai công tắc nhỏ ở cuối thẻ:
  1. **"Gồm cả font lai (hybrid)"** — khớp thêm font có `secondary` thuộc class đã chọn; mặc định **tắt**. Tooltip + `TermTip` "hybrid".
  2. **"Chỉ nhãn chắc chắn"** — loại các font có `confidence` dưới ngưỡng (hiện tại `CONF_MIN`); mặc định **tắt** (tức là cả nhãn "có thể" vẫn hiện, kèm `NoteMark`).
- Nhãn tiếng Anh + giải thích tiếng Việt, mọi class/subclass có `TermTip` (đã làm qua `classTermKey` / `subclassTermKey`; **thêm khoá** cho subclass mới: `venetian_oldstyle`, `garalde`, `transitional`, `didone`, `glyphic`, `clarendon`, `egyptian`, `typewriter`, `grotesque`, `neo_grotesque`, `geometric`, `humanist`, `flared`, `rounded`, `formal_copperplate`, `brush`, `monoline_script`, `upright_script`, `casual_print`, `marker`…). Danh sách thật do 02 chốt; terms đi cùng dữ liệu, nếu thiếu `TermTip` thì mục không được hiện (kiểm tra bằng test như `terms` hiện có).
- **Hiển thị trên thẻ** (A3 `class`): `Sans · Geometric`; có `secondary` → `Sans · Flared` + dấu "!" nhỏ (theo taxonomy §5.1) với tooltip "Có thể là Serif/glyphic".

### D2. Thẻ mới "Đặc điểm (Traits)"

Các đặc điểm độc lập với class (taxonomy §4.10, **bao gồm** các cấu trúc Display đã chuyển thành trait theo quyết định chủ app): `rounded`, `stencil`, `inline_outline`, `pixel`, `shaded`, `fat_face`, `reverse_contrast`, `high_contrast`, `monoline`, `condensed`, `extended`, và tag kỹ thuật `variable`, `color`, `has_italic`.

- Hộp kiểm đa chọn có đếm (từ facet backend); chọn nhiều trait = **AND**? Đề xuất **OR trong nhóm** như class hiện nay, với công tắc "Phải có tất cả" nhỏ. [TK — cần xác nhận ngữ nghĩa với `FaceQuery.traits`.]
- `variable` và `italic` đang nằm ở thẻ "Style": giữ ở đó, **không lặp** trong Traits.
- Mỗi trait có `TermTip` kèm hình minh hoạ chữ mẫu khi làm được (quy ước "typo term luôn có tooltip, có hình minh hoạ nếu có thể").

### D3. Thẻ mới "Phong cách & cảm xúc" (tag người dùng gắn)

Cơ sở: taxonomy §4.11 (mood/use là *tag chủ quan*, "M" — không đo được, lấy từ người dùng) và nguyên tắc "công cụ, không gác cổng": app không tự gán, người dùng gắn, và nhãn do người dùng không bị auto-classify ghi đè (`source=user`).

- **Mô hình:** dùng chính **Thẻ** hiện có (`TagDto`, `tagFaces/untagFaces`), thêm tuỳ chọn `group: 'mood' | 'use' | 'custom'`. App có sẵn **danh sách gợi ý**, tạo thẻ thật khi người dùng dùng lần đầu (không tạo hàng loạt):
  - *Mood (~12, taxonomy §4.11):* Business, Calm, **Refined** (thay *Elegant* — từ này chủ app cấm trong lời giải thích của app, theo hồ sơ trong từ điển mô tả), Playful, Friendly/Cute, Loud/Bold, Vintage/Retro, Futuristic/Tech, Rugged, Artistic, Formal, Handmade.
  - *Use:* Text, Display, UI, Editorial, Logo/Branding, Packaging, Signage (Display ở đây là **use tag**, đúng quyết định chủ app).
  - Mỗi gợi ý có nhãn tiếng Việt + giải thích ngắn (`TermTip`), vd. *Loud/Bold — ồn, đậm, gây chú ý; hợp poster, ít hợp đoạn văn dài*.
- **Gắn tag:** menu chuột phải font đã có "Gắn thẻ" + "Thẻ mới" (`faceMenuModel`); thêm phím **⌘T** "Gắn thẻ cho lựa chọn" mở popover chọn nhiều thẻ có ô tìm (theo mẫu ⌘T của Typeface [web]); áp dụng cả khi chọn nhiều font. Có thể bật hiển thị `tags` trên thẻ (A3).
- **Lọc:** thẻ "Phong cách & cảm xúc" trong AdvancedSheet liệt kê thẻ nhóm mood/use (và thẻ tuỳ ý bên dưới nhãn "Thẻ khác") với đếm `faceCount`. Ngữ nghĩa: **OR trong nhóm, AND giữa nhóm** (cùng quy ước class), kèm công tắc "Phải có tất cả thẻ" cho nhu cầu ngược lại. Chip trên phễu: `Thẻ: Playful`.
- **Chuyển lọc thẻ vào `filters`:** hiện `ui.tag` là một id riêng, chỉ một thẻ, tách khỏi `LibraryFilters`, nên **không** được tính vào phễu/chip/"Đặt lại" và bị `setView` xoá. Đề xuất `filters.tagIds: number[]` (đã có `tagIds` ở `FaceQuery`); mục "Thẻ" trong Sidebar đặt `tagIds=[id]`. Hậu quả cần test: đổi view không xoá `tagIds` đã chọn trong sheet (nhưng bấm mục "Thẻ" ở Sidebar thay thế `tagIds`).
- **Đảm bảo không gác cổng:** không có thẻ "bắt buộc"; thẻ mood/use không ảnh hưởng quyền bật font.

---

## 5. Đa ngôn ngữ, TermTip, a11y

### 5.1 Khoá i18n mới (tiếng Việt chính; tiếng Anh theo quy ước `i18n/index.ts`)

Nhóm khoá (tên đề nghị, 05 chốt khi code):

`view.title` Hiển thị · `view.open` Mở Hiển thị · `view.group` Gộp & sắp xếp · `view.groupFamily` Gộp theo Family · `view.sortBy` Sắp xếp theo · `view.sort.family|added|released|fileCreated|weight|match` · `view.sort.asc.*|desc.*` (nhãn đổi theo trường) · `view.sort.locked.recent|active` · `view.sort.source.*` (dòng nguồn + độ tin cậy mỗi trường) · `view.sort.unknownLast` · `view.info` Thông tin trên thẻ · `view.field.format|lang|class|styles|variable|license|source|year|tags` (+ `.gloss`) · `view.langAdd` · `view.reset` · `card.dot.on|off|available|pending|missing` · `search.filter` Bộ lọc nâng cao (+ `.count`) · `sidebar.menu.reveal|rescan|removeSource|rename|activateAll|deactivateAll|recolor|deleteCollection|clearRecent|deactivateAllActive|closeCompare|filterByTag` (+ `.confirm.*`) · `filter.hybrid` · `filter.confidentOnly` · `adv.traits` · `adv.mood` · `adv.tagsOther` · `tag.suggest.*` (+ `.gloss`).

### 5.2 TermTip bắt buộc (mỗi cái: tên EN + giải thích VN, có hình nếu làm được)

`active` · `family` · `subfamily` · `variable` · `format` (TTF/OTF/TTC/WOFF/WOFF2) · `class` · `subclass` (mỗi giá trị mới) · `hybrid` · `trait` (mỗi giá trị) · `hyperglot coverage` / badge ngôn ngữ (4 hình) · `licence` (OFL, Apache, Khác, Chưa rõ) · `release year` (giải thích vì sao là ước lượng) · `head.created` (nếu hiện trong tooltip nguồn) · `fontRevision`/`version` nếu hiện · các thẻ mood/use.

### 5.3 Bàn phím và trình đọc màn hình

| Thành phần | Tab | Phím | ARIA |
|---|---|---|---|
| Chấm bật/tắt | trong thẻ (sau thẻ) | Enter/Space bật tắt; không nổi bọt tới thẻ | `button` `aria-pressed`; nhãn cố định; `aria-busy` khi đang chuyển |
| Nút Hiển thị | trên Top Bar | ⌘J; Esc đóng, focus về nút | `aria-haspopup="dialog"`, `aria-expanded` |
| Popover Hiển thị | focus vào tiêu đề popover khi mở | Tab đi tuần tự; Esc đóng một lớp | `role="dialog"` có nhãn; hộp kiểm là `checkbox`; sắp xếp là `radiogroup`; mục khoá `aria-disabled` + mô tả lý do (`aria-describedby`) |
| Phễu | sau input, trước ✕ | Enter / Space / ⌥⌘F | `aria-expanded`, `aria-label` kèm số lọc |
| Tiêu đề nhóm family | trong lưới | Enter/Space; ←/→ | `aria-expanded`; tên + "9 kiểu" |
| Menu Sidebar | mở bằng phím Menu / ⇧F10 | mũi tên, Enter, Esc, submenu →/← | `role="menu"` `aria-label` = tên mục; focus trả mục |

Mọi thông tin chỉ có ở tooltip phải có đường tới bằng bàn phím (focus vào phần tử → tooltip, như `NoteMark tabIndex=0`). Không dùng **màu** làm tín hiệu duy nhất (đã nêu ở A1, A2).

---

## 6. Yêu cầu backend (gửi 02, qua Lead)

| # | Cần | Dùng cho | Ghi chú |
|---|---|---|---|
| R1 | `FaceRow.langBadges: LangBadgeDto[]` theo `badgeLangs: string[]` trong `FaceQuery` (thay một `badgeLang`) | A3 nhiều ngôn ngữ | Giữ `langBadge` cũ ở bước chuyển tiếp; chú ý chi phí Hyperglot cho mỗi ngôn ngữ thêm |
| R2 | `FaceRow.releaseYear: number \| null`, `releaseYearSource: 'copyright' \| 'head_created' \| 'head_modified' \| null`; `fileCreatedAt: number \| null`; `sort` thêm `'released' \| 'file_created'` | B5 | `ROW_SCHEMA` trong `types.ts` buộc đủ khoá: cập nhật hợp đồng cùng lúc. Bản ghi cũ chưa phân tích → `null` tới lần phân tích lại; cần ghi rõ phiên bản analyzer nào có trường này. Giá trị `head.created` rõ ràng sai (0, 1904, tương lai) coi là `null`. |
| R3 | Lệnh `family_groups(query) → [{family, start, count}]` khớp đúng thứ tự `query_faces` khi `groupByFamily=true`; khoá nhóm theo mục B5 | B5 | Thứ tự trong nhóm: Weight tăng, đứng trước nghiêng |
| R4 | `FaceRow.sourceId`, `FaceRow.styleCount` (số kiểu trong family), `FaceRow.axisCount`, `FaceRow.classId/subclass/confidence/secondary` | A3 | Chỉ khi chủ app bật các trường tương ứng; có thể nạp tách theo trang để không phình hợp đồng |
| R5 | `trait_facets(query)`; ngữ nghĩa `traits` (OR) + cờ `traitsAll`; `classFacets` có `secondary`/`confidence` | D1, D2 | Phụ thuộc D1 của 02 |
| R6 | `rename_tag`, `delete_tag`, `tag.group` | C6, D3 | Nếu chưa kịp: bỏ Đổi tên/Xoá thẻ khỏi menu đợt này |
| R7 | Xác nhận `removeSource` làm gì với dự án/thẻ/yêu thích | C6 | Quyết định câu chữ xác nhận |

---

## 7. Cách cắt việc cho 05 (đề xuất, mỗi lát độc lập, merge được riêng)

| Lát | Nội dung | Phụ thuộc backend | Kiểm |
|---|---|---|---|
| L1 | A1 chấm bật/tắt + A2 badge trung tính | không | Vitest component; QA native: bật/tắt, mù màu (giả lập), tab + Space |
| L2 | B4 phễu trong ô tìm kiếm | không | Vitest; QA native: đếm đúng, ⌥⌘F, focus trả về |
| L3 | C6 menu Sidebar bằng `ui/Menu` (Nguồn, Dự án, Gần đây, Đang bật, Đang so sánh) | R7 (câu xác nhận) | Vitest mô hình menu thuần; QA native: chuột phải, phím Menu, trả focus, xác nhận xoá |
| L4 | A3 popover Hiển thị — trường có sẵn (`format`, `license`, `variable`) + lưu `ui.card_fields` | không | Vitest; QA: chiều cao thẻ, ⌘J |
| L5 | B5 sắp xếp theo Tên / Ngày thêm / Weight (đã có ở backend) + khoá theo view | không | Vitest; QA native |
| L6 | B5 năm phát hành + ngày file | R2 | |
| L7 | A3 `lang` nhiều ngôn ngữ | R1 | |
| L8 | B5 nhóm theo Family | R3 | cần kiểm hiệu năng cuộn 2.300+ face |
| L9 | D1–D3 | R4–R6, D1 của 02 | |

Gợi ý thứ tự: L1 → L2 → L3 → L4 → L5, rồi các lát cần backend khi 02 sẵn sàng. Mỗi lát: `cargo`/`tsc`/Vitest + QA native theo quy trình của Lead (không chạy bản release; QA native cần màn hình thức, `caffeinate -dimsu`).

**Kiểm tra QA native tối thiểu cho từng lát:** ảnh chụp sáng + tối; thư viện thật 2.342 face cuộn mượt (L4, L8); bàn phím không dùng chuột; kiểm chấm bật/tắt với `available`, `missing`, đang bật qua phiên; kiểm popover với `prefers-reduced-motion`.

---

## 8. Nguồn (truy cập 2026-11-11)

- Apple, *Change how files are displayed/sorted in Finder* — https://support.apple.com/guide/mac-help/mchlp1745/mac (View Options, Sort By, Group, cột List, theo thư mục).
- Apple, *Customize the Finder sidebar* — https://support.apple.com/guide/mac-help/customize-the-finder-sidebar-on-mac-mchl83c9e8b8/mac (không nhắc menu chuột phải). Thứ cấp: https://www.cultofmac.com/?p=131630, https://www.mactrast.com/?p=20254 ("Remove from Sidebar").
- Apple, *Filter emails in Mail* — https://support.apple.com/guide/mail/mlhl1f6cf15a/mac (nút Filter đổi màu).
- Adobe, *Set Library view options* — https://helpx.adobe.com/lightroom-classic/desktop/viewing-photos/setting-library-view-options.html (trang gốc trả HTTP 403 khi tải; nội dung lấy từ kết quả tìm kiếm). Thứ cấp: https://jkost.com/blog/2024/06/working-in-grid-and-loupe-view-in-lightroom-classic.html (Show Grid Extras, J, ⌘⌥⇧H).
- Apple Community (Photos, View > Metadata > Titles; Sort) — https://discussions.apple.com/docs/DOC-250002031, https://discussions.apple.com/thread/255066285 (diễn đàn người dùng, bản Catalina; có thể đã đổi).
- FontBase — https://fontba.se/learn/font-activation (vòng tròn bên trái tên); https://forum.fontba.se/t/add-the-ability-to-sort-fonts/27 (nút sắp xếp bên phải ô tìm kiếm, v2.20.1); changelog Softpedia https://www.softpedia.com/progChangelog/FontBase-Changelog-250915.html (qua tìm kiếm).
- Typeface — https://typefaceapp.com/help/tips/group-fonts-by-family (⌥G, mặc định gộp, nút số kiểu), https://typefaceapp.com/help/articles/keyboard (phím tắt bật/tắt, tag, ⌘T).
- RightFont — https://rightfontapp.com/docs/search-fonts (Unified Search ⌘F, panel bộ lọc, ↻ đặt lại). `…/organize-fonts/create-tags-and-smart-tags` trả 404; `…/search-fonts/filter-fonts` trả 404.
- Figma — https://help.figma.com/hc/en-us/articles/360041308034 (qua tìm kiếm); yêu cầu tính năng https://forum.figma.com/suggest-a-feature-11/filter-fonts-by-classification-18610.
- W3C WCAG 2.2, SC 2.5.8 Target Size (Minimum), 24 CSS px — https://www.w3.org/TR/WCAG22/#target-size-minimum (kiến thức nền, không tải lại trong phiên này).

**Không xác minh được** (ghi để không ai tin nhầm): menu chuột phải sidebar của Photos, RightFont, FontBase, Typeface, Connect/Suitcase; cách Suitcase/Connect hiển thị chấm/nút bật tắt; nội dung chi tiết trang Adobe helpx.

---

## 9. Câu hỏi nên đem hỏi 2–3 nhà thiết kế trước khi chốt (nếu chủ app đồng ý)

1. Khi duyệt font, bạn muốn thấy gì trên mỗi thẻ ngoài tên và mẫu chữ? Xếp theo mức hay dùng (cho ra danh sách mặc định A3).
2. Bạn sắp xếp thư viện theo tiêu chí nào để tìm "font mới tôi vừa thêm" và "font cổ điển / đời mới"? "Năm phát hành ước lượng" có đủ dùng không, hay bạn tự gắn thẻ thời kỳ?
3. Bạn kỳ vọng chuột phải vào một nguồn hoặc dự án trong thanh bên có những gì? (Để kiểm tra mục C6 vì không có nguồn công khai.)
4. Nhóm theo family: bạn muốn thấy **mọi kiểu** dưới tiêu đề, hay **một thẻ đại diện** mỗi family?
5. Bạn dùng từ nào để gắn cảm xúc / phong cách (tiếng Việt hay Anh)? Có tag nào không có trong 12 gợi ý?

---

## 10. Ngoài phạm vi (ghi lại để không bị quên)

Cột có thể bật/tắt và tiêu đề cột sắp xếp được trong chế độ danh sách; xoay vòng Gọn/Chuẩn/Chi tiết bằng một phím (kiểu Lightroom J); sắp xếp xuống từng cấp (ví dụ nhóm → theo nhóm); lưu sắp xếp/nhóm theo từng view (Finder kiểu); tag thông minh (RightFont Smart Tags); mục E (ổ cloud) — chỉ chừa chỗ cho trạng thái "chưa tải" ở chấm bật/tắt và "Luôn giữ trên máy" trong menu Nguồn.
