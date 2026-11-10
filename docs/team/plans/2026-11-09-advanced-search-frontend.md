# Luồng 2a · Advanced search (frontend) — plan ngắn

Nhánh `feat/advanced-search` từ `origin/dev/mvp` (ad104db). Spec: `docs/design/library-filter/02_spec.md`, prototype `library-filter-v2.html`.
Hiện có: `FilterPanel.tsx` (popover Lọc: vi, class, subclass, trait, rawAxisRanges, license, source), `FilterChips.tsx`, `SearchField.tsx`, `store/ui.ts` (`filters`), api `axisStats` + `classFacets`. Giữ class override (đã có).

## 1. Component (thư mục mới `components/filter/`, thay dần FilterPanel)
- `FilterSummary` — nút tóm tắt trong ô tìm (đoạn theo nhóm, thu gọn `+N`, ô nhập ≥ 40%), hover tooltip liệt kê nhóm.
- `ActiveFiltersPopover` — "Đang lọc" 420px: mỗi nhóm một hàng (× từng giá trị, Sửa, Bỏ nhóm), Xoá tất cả.
- `AdvancedSheet` — sheet phủ lưới, masonry `columns: 208px`, Esc đóng; chứa `BasicCards` (Phân loại, Style, Thư viện, Giấy phép, Vi/Ngôn ngữ) + `AxisGroup` (Weight, Width, Proportions, Contrast & Stress, Aperture, Spacing, Serif & Slant).
- `AxisRow` — tên (TermTip có hình), slider 2 đầu (`RangeSlider`, snap + vạch cho trục rời rạc, nháy đúp = bỏ lọc, bàn phím), input số tại chỗ, nút `wght`/`wdth`, `Histogram` (lớp nhạt + đậm, nằm sau track).
- `LangPicker` + `LangBadge` (thẻ font) — dựng trên mock; tên ngôn ngữ `Intl.DisplayNames`.
- `useFilterHistory` — ngăn xếp hoàn tác 20 bước + `UndoToast`.

## 2. State
- `filters` (store) là nguồn sự thật duy nhất: nấc Basic và slider Advanced cùng ghi `rawAxisRanges`. Mở rộng `LibraryFilters` thêm `langs`, `langMode`, `usWeight/usWidth` + `varAxisMatch` (mock trước, bám DTO của worker 02 khi chốt).
- Hàm thuần `groupFilters(filters) → Group[]` (thứ tự cố định) dùng chung cho tóm tắt, "Đang lọc", Backspace, toast: dễ test.
- `AdvancedSheet` mở/đóng + `selectedGroup` (Backspace lần 1) ở state cục bộ SearchField. Stats/histogram: `api.axisStats(q trừ chính trục)`, fetch có huỷ, skeleton, không khoá thao tác.
- Cầu nối mock: `api.mock` thêm `axisHistogram`, `langList`, `faceLang`, `usWeight/usWidth` để dựng UI; adapter mỏng, đổi sang command thật khi 02 chốt shape.

## 3. Thứ tự commit (mỗi commit có vitest)
1. `groupFilters` + `useFilterHistory` + i18n khoá.
2. `FilterSummary` + `ActiveFiltersPopover` + Backspace/⌘⌫/⌘Z + toast.
3. `RangeSlider` + `AxisRow` (+ TermTip hình cho đủ 18 trục) .
4. `AdvancedSheet` masonry 18 trục, Weight/Width class, nút `wght`/`wdth`.
5. Histogram (mock → `axisStats` bỏ chính trục).
6. LangPicker + LangBadge (mock), thay chỉ báo `−N`.
7. QA native + ảnh, nối backend 02 khi sẵn sàng.

## 4. QA native (thêm vào `main_flows.js`, chờ có điều kiện, không sleep cố định)
- mở Advanced bằng `⌥⌘F`: thấy đủ 18 hàng trục, không cuộn ở 1400×920, 3 cột ở 1100 (đo `columns`/số cột thực).
- kéo/đặt số Weight 600–900: lưới đổi số lượng, tóm tắt hiện "Bold", nháy đúp bỏ lọc.
- Backspace 2 lần bỏ nhóm cuối + toast + `⌘Z` hoàn tác; ô nhập luôn ≥ 40% bề rộng.
- mọi `[data-term]` ở Advanced có TermTip (dùng `termCheck`), NoteMark cho cảnh báo.
- ảnh native light + dark từ font fixture (không font riêng của chủ app).
Cổng: `local_ci` PASS trước mỗi báo cáo; chỉ commit trên nhánh, không push.

## 5. Rủi ro / cần Lead chốt
- Spec ghi "bỏ class_override": giữ nguyên theo lời Lead.
- usWeightClass/usWidthClass và `wght`/`wdth` cần worker 02 (1b); trước đó trục Weight/Width dùng `weight`/`width_*` đo được.

## 6. Ghi chú sau review (2026-11-10)
- Chỉ báo ngôn ngữ trên thẻ theo spec §1: mã viết hoa + vòng độ phủ, **không có số** (không còn `−N`); tooltip ghi "thiếu x/y ký tự". `badgeLang` = ngôn ngữ đầu tiên đang lọc.
- Thanh trượt: đầu ở mép miền hiển thị (p1–p99) lưu **cận mở** = cực trị thật của `axisStats`; phím gộp 300 ms thành một lần ghi; thay đổi trục liên tiếp ≤ 400 ms là một bước hoàn tác.
- Toast Hoàn tác chỉ khôi phục các khoá của thao tác gỡ nhóm (không pop ngăn xếp). Esc đóng popover trước, rồi mới đóng sheet.
- "Ặ vi" và nhóm Tiếng Việt là cùng một trạng thái (`filters.vi`, có test); gỡ nhóm = tắt "Ặ vi" là chủ ý.
- Backend (báo 02): tooltip `langs.rs` ghi "thiếu 70 (0/70)" khi `have = 0`.
