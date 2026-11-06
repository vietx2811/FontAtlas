# 02 · Spec — Search tích hợp filter, Basic/Advanced, chỉ báo ngôn ngữ (vòng 2)

Prototype: `docs/design/prototype/library-filter-v2.html` (v1 giữ nguyên để so sánh) · Ảnh: `shots-v2/` (vòng 1: `shots/`) · Nghiên cứu: `01_research.md`.
Token giữ nguyên (`index.css`): `--sheet #FAFAFA`, `--fg #2A2A2A`, `--accent #D4A574`, `--accent-ink #9A6B3C`, `--on #3FA66B`, `--warn #C2410C`, dark stone; Inter; spacing 4/8/12/16; icon lucide.

---

## 0. Thay đổi vòng 2 (2026-11-06) — theo quyết định chủ app

| # | Quyết định / nhận xét | Thay đổi trong spec | Prototype |
|---|---|---|---|
| D1 | Mọi phân loại **tự nhận diện**, không gán tay | Bỏ `class_override`, chuột phải phân loại. Thêm **§9 Quy tắc nhận diện** + nhãn tin cậy high/medium/low; Mono là cờ độc lập | Ô phân loại có tooltip quy tắc; "Kể cả nhận diện chưa chắc" |
| D2 | Thẻ mặc định Tiếng Việt, đổi trong Cài đặt | Giữ §1 | — |
| D3 | "Gần đủ" = **độ phủ ≥ 90%** bộ `base` | §1, §3, token `lang:vi~` | Mức "≥ 90%" |
| D4 | Bỏ `Ặ vi` | §2 | Không còn nút |
| D5 | Advanced **phủ lên lưới** | §4 | Đúng |
| D6 | Chỉ Hyperglot, **không fallback gflanguages** | §7.1; tên ngôn ngữ theo locale lấy từ CLDR của hệ điều hành (`Intl.DisplayNames`), không nhúng dữ liệu | Tên ngôn ngữ đổi theo locale UI |
| N1 | Advanced thiếu Weight/Width đầy đủ | Nhóm **Weight** (usWeightClass 100–900 + trục `wght`, Stem, Darkness), nhóm **Width** (usWidthClass 1–9 + trục `wdth`, Width o, Avg advance) | Slider có vạch bước + nút `wght`/`wdth` |
| N2 | Advanced chưa dùng hết chỗ | Một lưới **masonry cột hẹp 208px**: Basic và trục chung một dòng chảy → 5 cột @1400, 3 cột @1100; hàng trục 40px, histogram nằm sau track | Đúng |
| N3 | Chip chiếm hết ô tìm | **Một nút tóm tắt** (≤ 60% ô, ô nhập ≥ 40%) + danh sách "Đang lọc" + Backspace theo nhóm + Hoàn tác (§2.1) | Đúng |
| N4 | Thuật ngữ font giữ tiếng Anh, UI qua khoá i18n | **§10**; nhãn trục/nhóm/lớp bằng tiếng Anh, giải thích ở tooltip theo locale | Nút demo "UI: EN" đổi toàn bộ chuỗi |

Sửa lỗi kèm theo: v1 và v2 dùng thuộc tính `data-hist` trên panel → mọi cú bấm trong Advanced v1 bật/tắt histogram thay vì chọn; đã đổi sang `data-histbtn` ở cả hai file.

---

## 1. Chỉ báo ngôn ngữ trên thẻ (thay `−N`)

`Libre Baskerville Regular      TTF  [VI ◔]` — mã ngôn ngữ (BCP 47, viết hoa) + vòng độ phủ trên bộ `base` của Hyperglot.

| Trạng thái | Quy tắc (độ phủ base) | Hình | Màu |
|---|---|---|---|
| Đủ | 100% | vòng đầy | vòng `--on`, chữ `--muted` |
| Gần đủ | ≥ 90% | cung gần kín | cung `--accent-ink`, chữ `--fg` |
| Thiếu | 25–90% | cung = tỷ lệ | cung `--warn` |
| Gần như không | < 25% | vòng rỗng + mã gạch ngang | `--muted` 55% |
| Chưa phân tích | — | không hiện | — |

- **Không có số trên thẻ.** Tooltip: `Tiếng Việt · thiếu 68/186 ký tự` + 12 ký tự thiếu đầu tiên + "Bấm để xem chi tiết". Bấm → chi tiết font, tab ngôn ngữ đó.
- Ngôn ngữ hiển thị = ngôn ngữ đang lọc (nhiều → mã đầu + `+N`); không lọc → **ngôn ngữ ưu tiên** trong Cài đặt (mặc định Tiếng Việt).
- a11y: `aria-label` đầy đủ; 4 trạng thái khác hình, không chỉ khác màu.

---

## 2. Search tích hợp

### 2.1 Tóm tắt bộ lọc trong ô tìm (thay dãy chip)
```
[🔍 (Sans +1 · 5 ngôn ngữ · Weight 400–700 (+3) ×)  Thêm từ khoá…        ⚙︎12 ]
```
**Bố cục**: `icon 16 | tóm tắt ≤ 60% − 72px | ô nhập ≥ 40% | ⌘K | nút bộ lọc`. Ô nhập **luôn ≥ 40%** bề rộng ô tìm.

**Nhóm** (thứ tự cố định): Phân loại → Ngôn ngữ → Weight → Width → Style → Số đo (các trục khác) → Thư viện → Giấy phép. Mỗi nhóm = **một đoạn**:

| Nhóm | 1 giá trị | ≥ 2 giá trị |
|---|---|---|
| Phân loại | `Sans` | `Sans +1` |
| Ngôn ngữ | tên theo locale (`Tiếng Việt`, `≥90%` nếu gần đủ) | `5 ngôn ngữ` |
| Weight / Width | tên nấc nếu khớp đúng nấc (`Bold`, `Light–Regular`, `Condensed`) ngược lại số (`Weight 400–700`) | — (một range) |
| Style | `Variable` | `Variable, Italic` |
| Số đo | `x-height 0,48–0,54 em` | `3 số đo` |
| Thư viện | `★ Yêu thích` / tên nguồn | `Brand A +2` |
| Giấy phép | `OFL` | `2 giấy phép` |

**Quy tắc thu gọn**: đặt lần lượt các đoạn; đoạn nào làm tóm tắt vượt giới hạn → dừng, phần còn lại thành `+N` (N = số **nhóm** còn lại). Nếu chỉ một đoạn cũng không vừa → `Bộ lọc +N`. Tính lại khi đổi kích thước cửa sổ.

**Hành vi**:
- Bấm tóm tắt → popover **"Đang lọc"** (420px): mỗi nhóm một hàng `tên nhóm | các giá trị (× từng cái) | Sửa · Bỏ nhóm`; chân: gợi ý phím + `Xoá tất cả`. "Sửa" mở Basic (hoặc Advanced cho Số đo) đúng mục.
- `×` cuối tóm tắt = xoá mọi bộ lọc (giữ chữ đang gõ).
- **Backspace** khi ô nhập trống: lần 1 **chọn nhóm cuối** (đoạn đó tô `--accent`; nếu nhóm nằm trong `+N` thì `+N` được tô và hàng tương ứng trong "Đang lọc" sáng lên); lần 2 **bỏ cả nhóm**. Gõ phím khác → bỏ chọn.
- `⌘⌫` trong ô: xoá tất cả. Mọi thao tác bỏ/xoá hiện toast `Đã bỏ <nhóm> · Hoàn tác ⌘Z` (4 s); `⌘Z` khi ô nhập trống = hoàn tác (ngăn xếp 20 bước).
- Hover tóm tắt → tooltip liệt kê đủ nhóm: giá trị.
- Trong family scope: đoạn đầu là `Family: X` (màu đảo), không gộp vào `+N`.

### 2.2 Cú pháp token (khoá tiếng Anh; alias không dấu)
| Token | Ví dụ | Ánh xạ |
|---|---|---|
| `lang:` (`nn:`) | `lang:vi`, `lang:fr,de`, `lang:vi~` (≥ 90%) | `langs[]`, `langMode` |
| `class:` | `class:serif`, `class:mono` | `classes[]` |
| `weight:` (`wght:`) | `weight:bold`, `weight:600-900` | `ranges.weight` |
| `width:` (`wdth:`) | `width:condensed`, `width:3-5` | `ranges.widthc` |
| `var` · `italic` · `upright` | | variable, italic |
| `fav` · `on` | | favoritesOnly, activation |
| `src:` · `license:` | `src:"Brand A"`, `license:ofl` | sourceIds, license[] |
| `<axis>:` | `x:0.5-0.55`, `contrast:0.6-1`, `slant:0-2` | `ranges[axis]` |
| `-` / `"…"` | `-class:script`, `"PT Sans"` | phủ định (P3) / tên chính xác |

Chữ tự do = tìm tên. Chỉ đổi thành bộ lọc khi chọn gợi ý (Enter/Tab) hoặc gõ token có `:`.

### 2.3 Autocomplete
Nhóm "Lọc nhanh / Bộ lọc / Số đo / Font / Gần đây", mỗi dòng có token minh hoạ và đếm; ↑↓ Enter Tab Esc. Gõ `lang:` → danh sách ngôn ngữ (tên theo locale + autonym); gõ `x:` → min–max thư viện.

---

## 3. Basic mode (popover 540px, neo phải ô tìm)

| Hàng | Điều khiển | Dữ liệu |
|---|---|---|
| Phân loại | ô "Ag": **Sans · Serif · Slab · Script · Decorative · Mono** (chọn nhiều, OR); checkbox "Kể cả nhận diện chưa chắc" | §9 |
| Ngôn ngữ | pill nhanh (ngôn ngữ ưu tiên + 2 gần đây) + ô tìm ngôn ngữ (tên locale/autonym/tên Anh/mã, nhóm theo hệ chữ, có đếm); **Độ phủ: Đủ 100% / ≥ 90%** | §7.1 |
| Weight | 3 nấc minh hoạ `a`: Light (100–300) · Regular (400–500) · Bold (600–900) | `ranges.weight` |
| Width | Condensed (1–4) · Normal (5) · Expanded (6–9) | `ranges.widthc` |
| x-height | Low · Mid · High (tam phân vị thư viện) | `ranges.x_height_em` |
| Contrast | Low · Mid · High (High = tỷ lệ mảnh/đậm thấp) | `ranges.contrast` |
| Style | Variable · Italic · Upright | |
| Thư viện | ★ Yêu thích · nguồn | |
| Giấy phép | OFL · Apache · Khác · Không rõ | |

**Nấc = lối tắt vào cùng range của Advanced** (một nguồn sự thật). Chọn nhiều nấc → range = bao lồi (Light + Bold = 100–900); bỏ nấc ở mép → co range; nấc "sáng" khi range phủ trọn nấc đó. Footer: `N font · Đặt lại · Advanced ⤢`.

---

## 4. Advanced mode (phủ lên lưới)

- Sheet neo dưới TopBar, trái/phải 12px, cao tối đa `100vh − 72px`; lưới phía sau vẫn cập nhật; Esc → về Basic → đóng.
- **Một lưới masonry** `columns: 208px; column-gap: 10px`, mỗi thẻ nhóm `break-inside: avoid`. Thứ tự dòng chảy: Phân loại → Weight → Width → Ngôn ngữ → Proportions → Contrast & Stress → Aperture → Spacing → Serif & Slant → Style → Thư viện → Giấy phép. Kết quả: **5 cột @1400, 3 cột @1100**, đủ 18 trục + Basic **không cuộn** ở 1400×920.

| Nhóm (thuật ngữ EN) | Icon lucide | Hàng |
|---|---|---|
| Weight | `bold` | **Weight class** (usWeightClass 100–900, bước 100, nút `wght`) · Stem · Darkness |
| Width | `move-horizontal` | **Width class** (usWidthClass 1–9, bước 1, nút `wdth`) · Width (o) · Avg advance |
| Proportions | `ruler` | x-height · x / Cap · Ascender |
| Contrast & Stress | `pen-tool` | Contrast · Stress |
| Aperture | `circle-dashed` | Aperture (c) · Aperture (e) · Counter (o) |
| Spacing | `space` (hoặc icon lucide tương đương) | Spacing · Width variance |
| Serif & Slant | `italic` | Serif flare · Slant |

### 4.1 Hàng trục (40px)
```
Weight class [wght]                400–700   ↺
▁▂▅▇▅▂▁▁▂  (histogram 11px, nằm sau track)
●━━━━━━━━●  ┊ ┊ ┊ vạch bước (trục rời rạc)
```
- Tên = thuật ngữ EN, gạch chấm; hover 300 ms → tooltip 240px: minh hoạ SVG + giải thích theo locale + đơn vị.
- Một slider 2 đầu; chưa lọc: không tô đoạn chọn, núm ở hai mép; kéo = bật lọc; kéo giữa đoạn = dời khoảng; **nháy đúp thanh = bỏ lọc**; trục rời rạc (weight/width) **snap theo bước** và có vạch.
- **Nút `wght` / `wdth`** (mặc định bật): font Variable khớp khi **dải trục của nó giao** khoảng chọn (vd Inter 100–900 khớp "Bold"); tắt → chỉ dùng usWeightClass/usWidthClass của instance mặc định. `wdth` (%) quy về class theo bảng OS/2: 50, 62.5, 75, 87.5, 100, 112.5, 125, 150, 200 → 1…9.
- usWeightClass bất thường: < 10 (font cũ ghi 1–9) → ×100; giá trị lẻ (250, 350) giữ nguyên, lọc theo số.
- Số: click → input tại chỗ (44px); Enter/Tab lưu, Esc huỷ, sai → viền `--warn` + tooltip, không đóng. Đơn vị viết tắt: `em`, `× x`, `× H`, `°`.
- Bàn phím: ←/→ = 1 bước (trục rời rạc) hoặc 1% miền; Shift ×10; Home/End.
- Darkness là độ đậm **đo được** (so được giữa các family, không phụ thuộc metadata) — đặt cạnh Weight class để người dùng thấy hai cách nhìn.

### 4.2 Histogram (P3; nút "Mật độ (xem trước)" bật/tắt)
Lớp nhạt = toàn thư viện, lớp đậm = kết quả hiện tại (bỏ chính trục đó); cột trong khoảng chọn tô `--accent`. Trục rời rạc: mỗi bước một cột. Chỗ đã chừa sẵn → bật/tắt không làm nhảy bố cục.

---

## 5. Trạng thái
| Trạng thái | Hiển thị |
|---|---|
| 0 kết quả | "Không có font khớp N điều kiện." + tối đa 3 nút **"Bỏ <nhóm> → n font"** (bỏ theo nhóm, có hoàn tác) + Xoá bộ lọc |
| Đang tải facet/stats | đếm `…`, slider skeleton, không khoá thao tác |
| Lỗi | dòng `--warn` + Thử lại; tóm tắt vẫn gỡ được |
| Phân loại chưa chắc | mặc định ẩn khỏi kết quả lọc class; checkbox "Kể cả nhận diện chưa chắc" |

## 6. Bàn phím
`⌘K` tìm · `⌥⌘F` Advanced · `↓` gợi ý · `⌫⌫` bỏ nhóm cuối · `⌘⌫` xoá hết · `⌘Z` hoàn tác · `Esc` đóng lần lượt (gợi ý → Đang lọc → Advanced → Basic).

---

## 7. Backend cần thêm

### 7.1 Ngôn ngữ (chỉ Hyperglot)
1. `fontatlas-core/langdata`: build-time từ Hyperglot (ghim phiên bản), chỉ `validity ∈ {verified, preliminary}`, `status = primary`, kiểm chéo script; nhúng gzip ~70 KB: `{id (ISO 639-3), bcp47, name_en, autonym, script, base, marks}`.
2. Coverage per face: `face_langs(face_id, lang_idx, have_base, total_base)` chỉ lưu khi độ phủ ≥ 90% (đủ cho cả hai mức) + giữ `vi_missing` cho thẻ. Tính từ cmap lúc quét; đổi `lang_db_version` → tính lại từ cmap cache.
3. Tên hiển thị theo locale: UI gọi `Intl.DisplayNames([locale], {type:'language'}).of(bcp47)` (CLDR của WebKit/ICU hệ thống, không nhúng); không có → autonym Hyperglot → `name_en`.
4. `lang_coverage(face, lang)` → danh sách ký tự thiếu (tổng quát hoá `VietnameseDto.missing`).

### 7.2 Phân loại tự nhận diện (§9)
5. Đọc thêm: **OS/2 `sFamilyClass`**, **`post.isFixedPitch`** (PANOSE đã đọc, đang nằm trong `metadata_json`).
6. Số đo mới trong `fontatlas-describe`: `serif_thickness` (bề dày serif ÷ stem, đo ở chân n/l), `overhang` (tỷ lệ chữ thường có outline vượt hộp advance), `contour_profile` (số contour của o, l, I, O, H), `caps_only` (chữ thường trỏ cùng glyph chữ hoa hoặc x/Cap ≥ 0,95).
7. Cột mới `faces`: `class TEXT`, `class_conf TEXT (high|medium|low)`, `is_mono INT`, `class_signals TEXT (JSON, cho tooltip "vì sao")`, `class_rules_version INT`. Đổi quy tắc → tính lại từ tín hiệu đã lưu, không quét lại file.

### 7.3 Truy vấn
8. `FaceQueryDto` thêm: `langs[]`, `langMode: "full"|"near"`, `classes[]`, `includeLowConf`, `ranges: {key:[a,b]}` (gộp weight/widthc với trục đo), `varAxisMatch: {wght, wdth}`; giữ `vi`, `rawAxisRanges` để tương thích.
9. Cột `faces.wght_min/max`, `wdth_min/max` (từ `fvar`) để lọc giao khoảng bằng SQL.
10. `facet_counts(q)` (đếm "nếu chọn thêm" cho mọi lựa chọn Basic), `count_faces(q)` nhẹ (kéo slider, gợi ý nới), `axis_histogram(q, key, bins)` (P3; trục rời rạc = theo bước).

---

## 8. Kế hoạch theo pha
| Pha | Nội dung | Phụ thuộc |
|---|---|---|
| **P1** | Nhãn ngôn ngữ (VI, dùng `vi_missing`); tóm tắt bộ lọc + "Đang lọc" + Backspace/Hoàn tác; Basic: Weight/Width/Style/Thư viện/Giấy phép/Tiếng Việt; Advanced masonry 18 trục, Weight/Width class + `wght`/`wdth`; chuỗi UI qua i18n | 7.3-8 (ranges), 9 |
| **P2** | Hyperglot + coverage + `lang:` + tên theo locale; phân loại tự nhận diện + tin cậy; facet counts; nấc x-height/contrast; gợi ý nới | 7.1, 7.2, 10 (facet/count) |
| **P3** | Histogram; phủ định token; Smart filter ở sidebar | 10 (histogram) |

### Tiêu chí nghiệm thu
- Không còn số trên thẻ; 4 trạng thái nhãn phân biệt khi in đen trắng.
- Ô nhập **≥ 40%** ô tìm với mọi số bộ lọc; mọi bộ lọc xem/gỡ được trong ≤ 2 click.
- Advanced 1400×920: thấy đủ 18 trục + Basic không cuộn; 1100: không cuộn ngang.
- Lọc "Bold" với `wght` bật trả về cả font Variable có dải đi qua 600–900.
- Đổi locale → mọi chuỗi UI và tên ngôn ngữ đổi; thuật ngữ font giữ nguyên.
- Phân loại: đạt ngưỡng ở §9.4 trên bộ kiểm thử.

---

## 9. Quy tắc nhận diện phân loại (tự động, không gán tay)

### 9.1 Tín hiệu
| Nguồn | Tín hiệu | Có sẵn? |
|---|---|---|
| **Đo (M)** | `serif_foot` (loe chân ÷ thân), `contrast`, `width_cv`, `slant_deg`, `x_height_cap` | có (16 trục) |
| | `serif_thickness`, `overhang`, `contour_profile`, `caps_only` | **cần thêm** (7.2-6) |
| **Metadata (D)** | PANOSE: `bFamilyType` (2 Latin Text, 3 Hand Written, 4 Decorative, 5 Symbol), `bSerifStyle` (2–10 có serif; 4/5/6 square; 11–13 sans; 14 flared; 15 rounded), `bProportion` = 9 Monospaced | PANOSE có; |
| | OS/2 `sFamilyClass` lớp: 1–4, 7 serif (4 Clarendon), 5 Slab, 8 Sans, 9 Ornamental, 10 Script, 12 Symbolic | **cần đọc** |
| | `post.isFixedPitch` | **cần đọc** |
| **Tên (N)** | token trong family/subfamily/PostScript (không phân biệt hoa thường): Script, Hand, Brush, Calligraph, Signature · Slab, Egyptian, Clarendon · Stencil, Inline, Outline, Shadow, Ornament, Dingbat · Mono, Code, Typewriter. **Không dùng "Display"** (thường là optical size: Playfair Display). | có |

Font giá trị PANOSE/sFamilyClass = 0 ("any/no classification") → bỏ qua tín hiệu đó.

### 9.2 Thứ tự quyết định
1. **Mono (cờ, đi kèm lớp chính)**: `width_cv < 0,005` **hoặc** `isFixedPitch` **hoặc** PANOSE proportion 9.
2. **Decorative**: `contour_profile` bất thường (o ≠ 2 contour, l/I ≠ 1 → outline/inline/stencil) **hoặc** PANOSE family 4/5 **hoặc** sFamilyClass 9/12 **hoặc** tên nhóm Decorative. (`caps_only` một mình **không** đủ — Bebas Neue vẫn là Sans, chỉ thêm cờ `caps`.)
3. **Script**: PANOSE family 3 **hoặc** sFamilyClass 10 **hoặc** `overhang ≥ 0,35` (+ `slant_deg > 5` tăng tin cậy) **hoặc** tên nhóm Script.
4. **Serif / Sans** theo `serif_foot`: > 1,6 → Serif; < 1,2 → Sans; 1,2–1,6 → dùng PANOSE serif style / sFamilyClass; không có → `low`.
   - PANOSE 14 Flared (Optima) → Sans.
5. **Slab** (tinh chỉnh trong Serif): `serif_thickness ≥ 0,6` **và** `contrast ≥ 0,7`; hoặc sFamilyClass 5 (4 Clarendon → Slab medium); hoặc PANOSE serif 4/5/6; hoặc tên nhóm Slab.

### 9.3 Tin cậy
| Mức | Điều kiện | Hiển thị |
|---|---|---|
| **high** | số đo (M) cùng ít nhất một D hoặc N, **hoặc** số đo cách ngưỡng xa (serif_foot > 1,9 / < 1,1; overhang > 0,5) | lọc bình thường |
| **medium** | chỉ một nguồn (chỉ M sát ngưỡng, hoặc chỉ D/N khi đo thất bại) | lọc bình thường |
| **low** | M mâu thuẫn D/N (vd đo Sans nhưng sFamilyClass Serif) hoặc chỉ tín hiệu yếu | ẩn khỏi lọc trừ khi bật "Kể cả nhận diện chưa chắc" |
| — | không có tín hiệu (symbol, font không có Latin) | "Chưa phân loại", không vào ô nào |

`class_signals` lưu các tín hiệu đã dùng → tooltip trong chi tiết font: "Slab · high — serif_foot 2,2 · contrast 0,85 · sFamilyClass 5".

### 9.4 Kiểm chứng (trước khi bật P2)
- Bộ tham chiếu: danh mục Google Fonts (`METADATA.pb` → `category`: SANS_SERIF, SERIF, MONOSPACE, HANDWRITING, DISPLAY), repo đã có kịch bản `test_google_fonts_classification.py` để tận dụng.
- Ngưỡng đề xuất: Sans/Serif/Mono **≥ 95%** khớp ở mức high; Script recall ≥ 85% so với HANDWRITING; Slab đối chiếu danh sách tay ~50 family slab; DISPLAY của Google trộn nhiều loại → chỉ dùng để đo tỷ lệ Decorative báo nhầm.
- Báo cáo ma trận nhầm lẫn + phân bố tin cậy; chỉnh ngưỡng trong `class_rules_version`.

---

## 10. Thuật ngữ & i18n

- **Nguyên tắc**: thuật ngữ font giữ **tiếng Anh** ở mọi locale — x-height, Cap, Ascender, Stem, Darkness, Contrast, Stress, Aperture, Counter, Spacing, Serif, Slant, Weight, Width, Variable, Italic, Upright, Sans, Serif, Slab, Script, Decorative, Mono, tên nấc (Light/Regular/Bold, Condensed/Normal/Expanded, Low/Mid/High), tag trục (`wght`, `wdth`), tên bảng (usWeightClass, PANOSE…). Giải thích bằng ngôn ngữ người dùng trong tooltip/phụ chú.
- **Chuỗi giao diện** (Bộ lọc, Đặt lại, Ngôn ngữ, Thư viện, Giấy phép, Số đo, Đang lọc, thông báo, toast…) đi qua **khoá i18n** (`filter.reset`, `summary.langs` với tham số `{n}`…). Prototype có bảng `STR.vi / STR.en` làm mẫu đặt khoá.
- **Mỗi thuật ngữ có một khoá mô tả** (`term.x_height.desc`) → tooltip theo locale; nhãn thuật ngữ là hằng, không dịch.
- **Tên ngôn ngữ** theo locale qua `Intl.DisplayNames` (CLDR hệ thống); fallback autonym Hyperglot → tên Anh. Danh sách ngôn ngữ hiện "tên locale + autonym" (vd `Tiếng Pháp · Français`, `Hmong · Hmoob`).
- **Số**: `Intl.NumberFormat(locale)` (vi: `0,48`; en: `0.48`); token luôn nhận cả `.` và `,`.
- Tên nguồn hệ thống ("System") là chuỗi UI; tên nguồn người dùng đặt giữ nguyên.
