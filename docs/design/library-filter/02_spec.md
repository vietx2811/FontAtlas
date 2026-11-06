# 02 · Spec — Search tích hợp filter, Basic/Advanced, chỉ báo ngôn ngữ

Prototype: `docs/design/prototype/library-filter-v1.html` · Ảnh: `shots/` · Nghiên cứu: `01_research.md`.
Token giữ nguyên (`index.css`): nền `--sheet #FAFAFA`, chữ `--fg #2A2A2A`, `--accent #D4A574`, `--accent-ink #9A6B3C`, `--on #3FA66B`, `--warn #C2410C`, dark stone; Inter; spacing 4/8/12/16; icon lucide.

---

## 1. Chỉ báo ngôn ngữ trên thẻ (thay `−N`)

**Vấn đề**: `−68` đọc như điểm trừ; không nói "so với gì", "thiếu gì".

**Thiết kế — "nhãn ngôn ngữ + vòng độ phủ"** (góc phải dòng tên, thay `fa-vi`):

```
Libre Baskerville Regular      TTF  [VI ◔]
```

| Trạng thái | Hình | Màu | Quy tắc |
|---|---|---|---|
| Đủ | `VI` + vòng **đầy** | chữ `--muted`, vòng `--on` | thiếu 0 base (`vi_status = full`) |
| Thiếu một phần | `VI` + vòng **cung** = tỷ lệ có/186 | chữ `--fg`, cung `--warn` 80% | 0 < thiếu, tỷ lệ ≥ 25% |
| Gần như không | `VI` gạch chéo mảnh, vòng rỗng | `--muted` 60% | tỷ lệ < 25% (font chỉ Latin cơ bản) |
| Chưa phân tích | không hiện nhãn | — | đang quét |

- **Không có số trên thẻ.** Số chỉ trong tooltip (hover 150 ms, đúng `Tooltip` hiện có):
  `Tiếng Việt · thiếu 68/186 ký tự` + dòng mẫu 12 ký tự thiếu vẽ bằng font giao diện: `Ơ ơ Ư ư Ả ả Ấ ấ …` + gợi ý `Bấm để xem chi tiết`.
- Bấm nhãn → mở chi tiết font ở tab Tiếng Việt (không chọn thẻ).
- **Ngôn ngữ hiển thị = ngôn ngữ đang lọc**; không lọc ngôn ngữ → **ngôn ngữ ưu tiên** (Cài đặt, mặc định Tiếng Việt). Lọc nhiều ngôn ngữ → nhãn ngôn ngữ đầu + `+2` (tooltip liệt kê từng cái).
- Danh sách (list layout): cùng nhãn, cột cố định 44px.
- a11y: `aria-label="Tiếng Việt: thiếu 68 trên 186 ký tự"`; không chỉ dựa màu (hình vòng khác nhau).
- Bỏ "Ặ vi" ở TopBar: chuyển thành chip ngôn ngữ (xem §2) và lựa chọn nhanh trong Basic.

---

## 2. Search tích hợp

### 2.1 Bố cục TopBar
```
[Tất cả 13]   [🔍 (Tiếng Việt ×)(Serif ×)(Đậm ×) phở… | ⚙︎2 ]   [▦][☰]
```
- Ô tìm (min 420, co giãn tới 640) chứa: icon, **chip điều kiện**, chữ đang gõ, cuối ô là **nút bộ lọc** (lucide `sliders-horizontal`) kèm số điều kiện. `⌘K` focus ô.
- Chip: nền `--accent-soft`, chữ `--accent-ink`, 22px cao, nhãn ngắn ("Serif", "Đậm 600–900", "x 0,50–0,55 em"), `×` gỡ. Bấm thân chip → mở đúng mục trong Basic/Advanced. Quá rộng → gộp `+3` (bấm mở danh sách).
- Chip phủ định: gạch ngang chữ, tiền tố `không` trong aria (`-lang:vi`).
- Backspace ở đầu ô trống → chọn chip cuối; Backspace lần 2 → xoá. ←/→ di giữa chip.

### 2.2 Cú pháp token (khoá Anh + alias Việt, không dấu cũng nhận)
| Token | Alias | Ví dụ | Ánh xạ |
|---|---|---|---|
| `lang:` | `nn:` | `lang:vi`, `lang:fr,de`, `lang:vi~` (thiếu ≤ 5) | `langs[]`, `langMode` |
| `class:` | `loai:` | `class:serif` | `classes[]` |
| `weight:` | `dam:` | `weight:600-900`, `dam:dam` | weight_class |
| `width:` | `rong:` | `width:hep` | width_class |
| `var` | `bien` | `var` | variable |
| `italic` | `nghieng` | `italic`, `-italic` | italic |
| `fav` | `★`, `yeuthich` | | favoritesOnly |
| `on` | `dangbat` | | activation |
| `src:` | `nguon:` | `src:"Brand A"` | sourceIds |
| `license:` | `phep:` | `license:ofl` | license[] |
| `<trục>:` | label tắt | `x:0.5-0.55`, `stem:>0.15`, `slant:<2` | rawAxisRanges |
| `-` đầu token | | `-class:script` | phủ định (pha 3) |
| `"…"` | | `"PT Sans"` | khớp tên chính xác |

Từ tự do không phải token = tìm tên (FTS hiện có). **Không tự đổi từ thành chip khi gõ**; chỉ đổi khi chọn gợi ý (Enter/Tab) hoặc gõ token có dấu `:` rồi Space.

### 2.3 Autocomplete (dropdown dưới ô, rộng = ô)
Nhóm, tối đa 8 dòng, ↑↓ chọn, Enter áp dụng, Tab hoàn tất, Esc đóng:
1. **Bộ lọc khớp** (`ser` → Phân loại: Serif · 312 / Ngôn ngữ: Serbian · 140) — có đếm.
2. **Font** (tên family, chữ vẽ bằng chính font như thẻ) — Enter mở family.
3. **Gợi ý cú pháp** khi gõ `lang:` → danh sách ngôn ngữ có đếm; khi gõ `x:` → hiện min–max thư viện.
Ô trống + focus: "Gần đây" (3 truy vấn) + "Lọc nhanh": Tiếng Việt · Serif · Sans · Variable · Yêu thích.

---

## 3. Basic mode (popover từ nút bộ lọc, 560px, neo phải ô tìm)

Kiểu RightFont/Adobe: lựa chọn trực quan, mỗi lựa chọn có **đếm kết quả** (facet, mờ khi 0). Áp dụng tức thì; chip tương ứng hiện trong ô tìm.

| Hàng | Điều khiển | Dữ liệu |
|---|---|---|
| **Phân loại** | 6 ô minh hoạ "Ag": Sans · Serif · Slab · Mono · Script · Trang trí (+ "Chưa rõ") — chọn nhiều (OR) | `classification` (cần lưu vào `faces`, §7) |
| **Ngôn ngữ** | Pill nhanh: Tiếng Việt · English · Français · + "Thêm ngôn ngữ…" (combobox tìm theo tên Việt/Anh/autonym/mã, có đếm, nhóm theo hệ chữ). Công tắc: **Đủ dấu** / **Dùng được** (thiếu ≤ 5 ký tự auxiliary) | bảng ngôn ngữ (§7) |
| **Độ đậm** | 3 nấc minh hoạ chữ `a`: Mảnh (≤300) · Thường (400–500) · Đậm (≥600) — chọn nhiều | weight_class |
| **Độ rộng** | Hẹp (≤4) · Thường (5) · Rộng (≥6) | width_class |
| **Chiều cao x** | Thấp · Vừa · Cao | tam phân vị `x_height_em` của thư viện |
| **Tương phản** | Thấp · Vừa · Cao | tam phân vị `contrast` |
| **Kiểu** | toggle: Variable · Nghiêng · Đứng | variable, italic |
| **Thư viện** | Yêu thích · Đang bật · Nguồn ▾ · Giấy phép ▾ (OFL, Apache, Khác, Không rõ) | sẵn có |

Footer: `128 font` · `Đặt lại` · **`Nâng cao ⤢`** (⌥⌘F). Nấc 3 bậc là **cách nhập nhanh vào cùng range** của Advanced (chọn "Cao" = đặt range x-height = [p66, max]); mở Advanced thấy đúng range đó.

---

## 4. Advanced mode (bảng rộng)

- Mở bằng `Nâng cao` trong Basic hoặc ⌥⌘F. Là **sheet ngang** phủ trên lưới, rộng = vùng nội dung (trừ sidebar), cao tối đa 86vh (≤ 800px), neo dưới TopBar; lưới phía dưới vẫn cập nhật (đếm ở footer). Esc thu về Basic; Esc lần 2 đóng.
- Bố cục: cột trái 236px = Basic thu gọn; phần còn lại = **3 cột cố định** cân theo số trục: [Tỷ lệ dọc + Lòng & độ mở] · [Nét & màu chữ + Chân & nghiêng] · [Bề ngang & nhịp] (6/6/4 trục) → 1400×920 thấy đủ 16 trục không cuộn.
- < 1240px: Basic thành dòng gập `▸ Cơ bản · Tiếng Việt · Serif` phía trên (mặc định gập), mở ra là lưới 3 cột; 3 cột trục giữ nguyên (1100px không cuộn ngang).
- **5 nhóm, mỗi nhóm có icon + tiêu đề**:

| Nhóm | Icon lucide | Trục |
|---|---|---|
| Tỷ lệ dọc | `ruler` (dọc) | Chiều cao x (em) · x / chữ hoa · Nét lên (em) |
| Nét & màu chữ | `pen-tool` | Độ dày thân nét · Mật độ mực · Tương phản nét · Trục nhấn (°) |
| Lòng & độ mở | `circle-dashed` | Độ mở c · Độ mở e · Lòng chữ o |
| Bề ngang & nhịp | `move-horizontal` | Độ rộng o · Bước chữ TB · Biến thiên bước chữ · Khoảng cách chữ |
| Chân & nghiêng | `italic` | Chân serif · Độ nghiêng (°) |

### 4.1 Một hàng trục (38px)
```
Chiều cao x ⓘ                     0,500 – 0,560 em   ↺
[▁▂▃▅▇█▇▅▃▂▁▁]  ← histogram (pha 3)
●━━━━━━━━━━━━━━●  ← MỘT slider 2 đầu
```
- **Đơn vị trong hàng viết tắt** (`em`, `× x`, `× H`, `°`, tỷ lệ không ghi); tên đầy đủ đơn vị ở tooltip.
- **Tên**: hover 300 ms → tooltip rộng 220px: minh hoạ SVG 160×64 (vd chữ `x` với đường baseline + x-height, chú thích "x-height ÷ em") + 1 câu giải thích + đơn vị. Tooltip không chặn trỏ chuột.
- **Slider 2 đầu**: track 4px, phần chọn `--accent`, 2 núm 14px. Mặc định chưa lọc: hai núm ở hai đầu miền P1–P99, track xám, giá trị mờ; kéo một núm = bật lọc (hàng chuyển trạng thái *đang lọc*: tên đậm + nút ↺). Kéo **giữa đoạn** = dời cả khoảng. Miền mặc định P1–P99; ngoài miền có vạch "…" ở hai mép, **double-click mép** = mở miền min–max (thay nút "Toàn khoảng").
- **Sửa số**: con số là text; **click** → biến thành input tại chỗ (cùng kích thước, chọn sẵn), Enter/Tab lưu, Esc huỷ; sai → viền `--warn` + tooltip lỗi, không đóng input. Không có ô nhập thường trực.
- Bàn phím: Tab tới núm; ←/→ bước = (max−min)/100; Shift = ×10; Home/End; `aria-valuetext="0,500 em"`; nhóm `role="group" aria-labelledby`.
- Áp dụng khi **thả chuột** (như hiện tại), đếm ở footer cập nhật live khi kéo (debounce 120 ms, dùng count-only).
- Trục hằng số (min = max) → hàng mờ + "Mọi font: 0,52 em". Không có số đo → "Chưa có số đo".

### 4.2 Histogram mật độ (pha 3, đã chừa chỗ)
- Dải 16px phía trên slider, 24 bin trên miền đang hiện; cột `--muted` 25%, phần trong khoảng chọn `--accent` 60%.
- Hai lớp: **toàn thư viện** (nhạt) và **kết quả hiện tại** (đậm) → thấy ngay khoảng nào còn font.
- Khi chưa có API: dải ẩn, không chiếm chỗ (hàng không nhảy khi bật → chừa margin cố định 16px).

---

## 5. Trạng thái

| Trạng thái | Hiển thị |
|---|---|
| Rỗng thư viện | giữ empty-state hiện có; nút bộ lọc disabled |
| **Không kết quả** | minh hoạ `ở` hiện có + "Không có font khớp N điều kiện." + **gợi ý nới**: tối đa 3 nút "Bỏ *Đậm* → 24 font" (tính bằng count bỏ từng điều kiện) + `Xoá bộ lọc` |
| Đang tải stats/facets | skeleton thanh slider; đếm hiện `…`; không khoá thao tác |
| Lỗi | dòng `--warn` trong panel: "Không đọc được số đo. [Thử lại]" — chip vẫn gỡ được |
| Trong family scope | chip "Family: X" đứng đầu dãy chip (thay chip hiện tại ở dòng dưới) |

## 6. Bàn phím
`⌘K` focus tìm · `⌥⌘F` mở/đóng Advanced · `↓` trong ô tìm mở gợi ý · `Esc`: đóng gợi ý → đóng panel → bỏ focus (không xoá chip) · `⌘⌫` trong ô: xoá hết chip+chữ · Space trên ô phân loại/nấc = bật tắt.

## 7. Backend cần thêm

1. **Bảng ngôn ngữ nhúng** (`fontatlas-core/langdata`): build-time từ Hyperglot (§01), `include_bytes!` gzip ~70 KB: `{id, name_en, name_vi?, autonym, script, validity, speakers, base, aux, marks}`.
2. **Coverage per face** (bảng mới `face_langs(face_id, lang_idx, missing_base, missing_aux)` chỉ lưu ngôn ngữ có `missing_base ≤ 5`, hoặc bitset `langs_full BLOB`/`langs_usable BLOB` 98 byte/face). Tính lúc quét từ cmap; `lang_db_version` đổi → tính lại từ cmap cache.
3. **Lưu `classification`** vào `faces` (hiện chỉ tính khi mở chi tiết) + cột `class_override` cho phân loại tay (chuột phải như RightFont). Thêm nhãn Slab/Script/Trang trí cần heuristic mới hoặc phân loại tay — **cần quyết định** (§00).
4. **`FaceQueryDto`** thêm: `langs: string[]`, `langMode: "full"|"usable"`, `classes: string[]`, `weightRanges`, `widthRanges`, `negate` (pha 3). Giữ `vi` để tương thích (= `langs:["vie"]`).
5. **`facet_counts(q)`** → đếm cho từng lựa chọn Basic (class, lang top 30, nấc weight/width/x/contrast, variable, italic, license, source) theo truy vấn hiện tại bỏ chính chiều đó (đếm "nếu chọn thêm").
6. **`count_faces(q)`** nhẹ (live khi kéo slider, gợi ý nới khi 0 kết quả).
7. **`axis_histogram(q, axis, bins=24)`** (pha 3), có thể gộp vào `axis_stats` với cờ `bins`.
8. **`lang_coverage(face, lang)`** cho tooltip/tab: danh sách ký tự thiếu (đã có cho VI qua `VietnameseDto.missing`, tổng quát hoá).

## 8. Kế hoạch theo pha

| Pha | Nội dung | Phụ thuộc |
|---|---|---|
| **P1 — Rõ nghĩa ngay** | Nhãn `VI` + vòng độ phủ trên thẻ (dùng `viMissing`, tổng 186); Search có chip (vi, variable, italic, axis range sẵn có); Basic v1: Kiểu, Tiếng Việt, Độ đậm/rộng (weight/width class), Yêu thích, Nguồn, Giấy phép; Advanced: 5 nhóm, slider 2 đầu, sửa số inline, tooltip minh hoạ | chỉ cần thêm weight/width range vào query |
| **P2 — Đa ngôn ngữ + phân loại** | Bảng Hyperglot, coverage per face, `lang:` token + combobox, nhãn theo ngôn ngữ lọc; lưu classification + phân loại tay; facet counts; nấc x-height/contrast; gợi ý nới khi 0 kết quả; token autocomplete đầy đủ | backend 1–6 |
| **P3 — Mật độ & nâng cao** | Histogram trong slider; phủ định token; Smart filter (lưu bộ lọc thành mục sidebar); preview theo `sample_text` ngôn ngữ | backend 7 |

### Tiêu chí nghiệm thu (P1)
- Không còn số âm trên thẻ; 3 trạng thái nhãn phân biệt được khi in đen trắng.
- Từ trạng thái trống, đặt "x-height 0,50–0,55 em" ≤ 3 thao tác (mở Advanced, kéo 2 núm) hoặc gõ `x:0.5-0.55` + Enter.
- Mọi điều kiện đang bật đều thấy được dưới dạng chip và gỡ bằng 1 click.
- Advanced ở 1100px không cuộn ngang; ở 1400×920 thấy cả 16 trục không cuộn dọc (panel ≤ 86vh).
- 0 kết quả luôn có ít nhất 1 lối ra (gợi ý nới hoặc Xoá bộ lọc).
