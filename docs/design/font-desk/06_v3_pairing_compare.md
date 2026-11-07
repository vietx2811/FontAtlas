# 06 — Pairing Desk vòng 3: hai chế độ Pairing / Compare

- **Ngày:** 2026-11-06 · **Prototype:** `docs/design/prototype/pairing-desk-v3.html` · **Ảnh:** `docs/design/font-desk/shots-v3/`
- **Nền:** phương án **C — Cột vai + Cuộn chọn** (chủ app chọn).
- **Quyết định của chủ app, áp dụng từ vòng này:**
  - Combo nằm trong Case.
  - Option Stepper `‹ ▾ ›` thay **mọi** tab.
  - Cột mốc trong Compare cố định.
  - "Pairing Desk", "Case", "Combo" giữ tiếng Anh.
  - Thuật ngữ font giữ tiếng Anh, giải thích bằng ngôn ngữ người dùng (§6).

---

## 1. Cấu trúc: hai chế độ

| | **Pairing** | **Compare** |
|---|---|---|
| Mục đích | Tìm và thử font cho **từng vai**; ghép giọng thành combo | Đặt nhiều **font** hoặc nhiều **combo** cạnh nhau theo chiều ngang |
| Bố cục màn | Cột vai · Cuộn chọn · Trang · **Dải chi tiết** (dưới trang) | Thanh điều khiển · Bảng so sánh ngang (hoặc Song bản) |
| Pairing Style, Mức ghép (An toàn / Có kiểm soát / Thử nghiệm) | **Có**, đặt ở đầu Cuộn chọn | **Không có** (popover Lọc ghi rõ "không áp dụng ở Compare") |
| Nhãn ●◐◆○ | Giữa các vai/giọng | Chỉ ở hàng "Quan hệ giữa các giọng" của Compare Combo, để tham khảo |
| Đặt font | Bấm dòng / `Enter` / "Đặt vào vai" ở dải chi tiết | "Đặt vào vai" ở đầu cột |

**Chuyển chế độ:**
- Option Stepper `‹ Pairing ▾ ›` là nhóm đầu tiên trên thanh trên.
- Phím `⌘1` / `⌘2`.
- Nút "Đưa vào Compare" (dải chi tiết, dòng ứng viên, `⇧C`).
- Nút "Thử trong Pairing" ở đầu cột Compare.

**Giữ ngữ cảnh khi chuyển:**
- **Pairing → Compare (Font):**
  - Mốc là font đang dùng cho vai đang chọn.
  - Các cột lấy theo thứ tự: ứng viên đang thử → font đã ghim của giọng đó → các cột đã đưa vào trước → bù bằng gợi ý Ghép cho đủ 3.
- **Pairing → Compare (Combo):** từ dropdown Combo, mục "So sánh các combo". Mốc là combo đang dùng.
- **Compare → Pairing:**
  - "Thử trong Pairing" mở Cuộn chọn của vai đang chọn, với font đó đang ở trạng thái **thử** (chưa đặt).
  - "Mở combo này trong Pairing" chuyển sang combo đó.
- **Đặt vào vai từ Compare:** font cũ của vai trở thành một cột, font mới thành mốc. Không mất phương án nào.

## 2. Pairing (nền C)

```
┌ Case ☆ │ ‹ ⊚ Pairing ▾ › │ ‹ Bố cục ▾ › ‹ Combo 1/3 ▾ › │ ⚲ Lọc ↶ ───────────── ✓ Đã lưu ┐
├ VAI · FONT ──┬ CUỘN CHỌN ────────────┬ TRANG ───────────────────────────────────────────┤
│ ② Eyebrow     │ ✦ Đoạn văn · giọng 2   │                                                  │
│   Be Vietnam  │ ✦ ⧉ T ⊞ 📌  (nguồn)     │            [trang bố cục, tự co vừa]             │
│ ① Tiêu đề     │ ‹ ⊛ Mọi kiểu ▾ ›        │                                                  │
│   Fraunces    │ ‹ ⛨ An toàn ▾ ›         ├ CHI TIẾT · Đoạn văn · giọng 2 · ●x ●w ◐serif ─────┤
│▸② Đoạn văn    │ Ở Hà Nội, một… Be Viet. │ ✓ Đang dùng  Be Vietnam Pro   │ Số đo  Đang dùng Đang thử Δ │
│   Be Vietnam  │ Ở Hà Nội, một… Bricol.  │   Ở Hà Nội, một ngôi nhà…     │ x-height  0,53em  0,52em −0,01│
│ ① Trích dẫn   │ …                       │   designer · foundry · v · lic│ contrast  1,2:1   1,4:1  +0,2 │
│ ＋ Giọng nhấn │ lăn/↑↓ thử · ↵ đặt      │ 👁 Đang thử  Bricolage …       │ …                             │
└───────────────┴─────────────────────────┴─────────────── [↵ Đặt vào vai] [⫼ Đưa vào Compare]┘
```

- **Cột vai:** mỗi dòng gồm vai, giọng, tên font viết bằng chính font đó, nhãn rút gọn so với giọng ①. Khi đang thử, tên font hiện in nghiêng màu nhấn ở **mọi vai cùng giọng**.
- **Cuộn chọn:**
  - Nguồn dạng icon: Ghép · Giống · Cùng họ · Thư viện · Ghim.
  - Nguồn **Ghép** có thêm 2 Option Stepper: **Pairing Style** và **Mức ghép**. Các nguồn khác không có hai stepper này.
  - Dòng ứng viên viết chữ thật của vai. Rê chuột hoặc lăn chuột là **thử**.
  - Nút trên dòng (hiện khi hover hoặc focus): mức ghép, Ghim, **Đưa vào Compare**, Đặt.

### 2.1 Dải chi tiết — trả lời nhận xét 1 của chủ app

Vị trí: dưới trang, cùng cột với trang. **Thu gọn** bằng `I` hoặc nút ⌄, còn một dòng 38 px giữ tiêu đề, nhãn và nút.

| Vùng | Nội dung |
|---|---|
| Đầu dải | Vai · giọng · nhãn ●◐◆○ của font đang thử (hoặc đang dùng) so với giọng ① · mức ghép · `↵ Đặt vào vai` (khi đang thử) · `Đưa vào Compare` |
| Thẻ **Đang dùng** | Tên font · vòng **VI** (x/186) · chữ thật của vai · meta một dòng có icon: Designer · Foundry · Version · License · Glyphs · Styles/Italic · Variable (tag trục min–max) |
| Thẻ **Đang thử** | Như trên, nền nhấn. Glyphs kèm Δ. Khi chưa thử gì: ô gợi ý "Rê chuột lên một ứng viên để xem chênh lệch" |
| Bảng **Số đo** | Hàng: x-height · cap height · contrast · width · weight · stress · aperture · serif. Cột: Đang dùng · Đang thử · **Δ** (dấu + / −, màu theo chiều; ±0 xám). Đơn vị ghi sát số (`em`, `: 1`, `× x`, `°`) |

- **Vì sao đặt dưới trang, không phải panel phải:** ở 1400 px, panel phải 304 px cộng cột vai và cuộn chọn làm trang chỉ còn khoảng 360 px (đã thử, ảnh cũ bị thay). Đặt dưới, trang giữ khoảng 660 px, mắt đi từ dòng ứng viên xuống bảng Δ chỉ một đoạn ngắn.
- **Ở 1100 px:** hai thẻ thu gọn (ẩn dòng meta, chữ 16 px) và nằm cạnh bảng số đo. Meta đầy đủ xem ở Compare hoặc khi mở rộng cửa sổ.

## 3. Compare — trả lời nhận xét 2 của chủ app

**Thanh điều khiển** (đều là Option Stepper):
- `‹ So sánh: Font | Combo ›`;
- `‹ Vai ▾ ›` (chỉ ở Font; quyết định chữ mẫu và giọng nhận "Đặt vào vai");
- `‹ Sắp theo ▾ ›` + nút đảo chiều;
- `‹ Hiển thị: Bảng | Song bản ›`;
- `＋ Thêm cột ▾`: tìm, kèm nhóm Đã ghim · Gợi ý ghép · Giống · Thư viện.

### 3.1 Compare Font
- **Cột:**
  - **mốc** cố định, dính bên trái, nền sọc, icon neo. Đó là font đang dùng cho vai đang chọn;
  - tiếp theo là các ứng viên.
- **Thao tác trên đầu cột** (hiện khi hover/focus, phủ lên góc phải để không cắt tên): Ghim · Đặt vào vai · Thử trong Pairing · Bỏ. Kéo `⠿` hoặc `⌥←/→` để đổi chỗ; `⌫` để bỏ.
- **Hàng**, chia 3 nhóm, mỗi nhóm thu gọn được; kéo đổi chỗ **trong cùng nhóm**:

  | Nhóm | Hàng |
  |---|---|
  | Specimen | Vai đang chọn · Đoạn văn · Dấu chồng (Ữ Ặ Ệ Ỗ ươ) · Bảng chữ |
  | Số đo | x-height · cap height · contrast · width · weight · stress · aperture · serif. Mỗi ô: số + đơn vị + **Δ so với mốc** |
  | Meta | Designer · Foundry · Version · License · Glyphs (Δ) · Styles · Variable (tag trục) · VI (vòng + x/186, Δ) |
- **Sắp:** bấm tên hàng có thể sắp (các hàng số đo, Glyphs, VI) hoặc dùng stepper. Cột ghim đứng sau mốc và không bị sắp lại.
- **Song bản:** mỗi cột là một trang thu nhỏ của bố cục hiện tại, trong đó giọng đang so được thay bằng font của cột.

### 3.2 Compare Combo
- **Cột:** mốc là combo đang dùng (★ nếu là combo chính), sau đó là các combo khác.
- **Thao tác trên đầu cột:** Ghim · Dùng combo này · Mở trong Pairing. Nút `＋` cuối hàng thêm combo từ gợi ý.
- **Hàng:**

  | Nhóm | Hàng |
  |---|---|
  | Specimen | Trang thu nhỏ |
  | Giọng | ① · ② · ③: tên font và các vai thuộc giọng |
  | Quan hệ giữa các giọng | Nhãn ①↔② · Δ x-height ①② · contrast ① · ② · VI thấp nhất · Số font · License |
- **Sắp theo:** Δ x-height, số chỗ ◆, VI thấp nhất, tên.

## 4. Option Stepper ở vòng 3 (thay mọi tab)

| Stepper | Ở đâu | Phím |
|---|---|---|
| Chế độ: Pairing / Compare | Thanh trên, nhóm đầu | `⌘1` `⌘2` |
| Bố cục (5) | Thanh trên | `⌥1–5` |
| Combo (n) | Thanh trên | `[` `]`, `⌘⇧S` lưu combo |
| Pairing Style (6) | Đầu Cuộn chọn, chỉ nguồn Ghép | — |
| Mức ghép (3) | Đầu Cuộn chọn, chỉ nguồn Ghép | `M` |
| So sánh: Font / Combo | Compare | — |
| Vai | Compare (Font) | `↑↓` ở Pairing |
| Sắp theo | Compare | bấm tên hàng |
| Hiển thị: Bảng / Song bản | Compare | — |

Nguồn gợi ý trong Cuộn chọn vẫn là hàng icon 5 nút. Đó là một nhóm nút bật, không phải tab: mỗi nút là một nguồn, chuyển bằng một click mà không mở menu. **Cần chủ app xác nhận** có muốn đổi cả nhóm này sang Stepper không (§9).

## 5. Bàn phím (Pairing)

| Phím | Tác dụng |
|---|---|
| `↑` `↓` (Cuộn chọn đóng) | Đổi vai |
| `→` / `Enter` | Mở Cuộn chọn |
| `↑` `↓` / lăn chuột (Cuộn chọn mở) | Thử ứng viên trước/sau |
| `⇧↑` `⇧↓` (Cuộn chọn mở) | Đổi vai, giữ Cuộn chọn mở |
| `Enter` | Đặt font đang thử |
| `Esc` | Bỏ thử → đóng popover → đóng Cuộn chọn |
| `←` | Đóng Cuộn chọn |
| `G` | Ghim font đang thử |
| `⇧C` | Đưa font đang thử vào Compare |
| `I` | Thu / mở dải chi tiết |
| `F` | Lọc |
| `⌘Z` | Hoàn tác |

## 6. Thuật ngữ và i18n

**Nguyên tắc:**
1. **Thuật ngữ font giữ tiếng Anh ở mọi locale:** x-height, cap height, contrast, stress, aperture, weight, width, serif, terminal, counter, Variable, Italic, Glyphs, Designer, Foundry, License, Version, Styles, VI.
2. **Giải thích (gloss) theo locale.** Hiện ở tooltip khi rê vào thuật ngữ (gạch chấm dưới chữ) và ở phụ chú khi cần. Không dịch nhãn.
3. **Mọi chuỗi UI qua khoá i18n.** Không viết chuỗi cứng trong component.
4. **Số** định dạng bằng `Intl.NumberFormat(locale)` (vi: `0,53`; en: `0.53`). **Đơn vị** là ký hiệu không dịch (`em`, `: 1`, `× x`, `°`).

**Khoá:**

| Loại | Mẫu khoá | Ví dụ |
|---|---|---|
| Chuỗi UI | `pd.<vùng>.<tên>` | `pd.mode.pairing` = "Pairing" · `pd.mode.compare.d` = "Đặt nhiều font hoặc combo cạnh nhau theo chiều ngang" · `det.trying` = "Đang thử" · `cmp.base` = "mốc" |
| Thuật ngữ | `term.<axisKey>.label` (cố định, **không** có trong file dịch) · `term.<axisKey>.gloss` (dịch) | `term.x_height_em.label` = "x-height" · `term.x_height_em.gloss` (vi) = "Chiều cao chữ thường x so với cỡ chữ (em). Cao hơn → chữ trông to, dễ đọc ở cỡ nhỏ." |
| Đơn vị | `unit.<id>` (ký hiệu, không dịch) + `unit.<id>.name` (tên đầy đủ, dịch, dùng trong tooltip) | `unit.em` = "em" · `unit.em.name` (vi) = "em — bằng cỡ chữ" |
| Nhãn trục | `pair.sym.<H\|C\|K\|U>` (ký hiệu cố định ●◐◆○) + `pair.sym.<…>.name` (dịch) | `pair.sym.C.name` (vi) = "tương phản" |
| Câu giải thích cặp | `pair.sentence.<axis>.<relation>` — có slot `{a}` `{b}`, dịch theo locale, tên trục chèn bằng `term.*.label` | "{term} gần nhau ({a} và {b}) — đọc liền mạch" |

- `axisKey` dùng đúng khoá canonical của `AxisReadingDto.key` (`x_height_em`, `cap_height_em`, `contrast`, `width`, `stem`, `stress_deg`, `aperture`, `serif_foot`, `terminal`, `counter`…).
- **Ảnh hưởng tới code hiện có:** `UX/components/detail/tabs/measurements.ts` đang có `AXIS_META` với nhãn tiếng Việt ("Chiều cao x", "Tương phản nét"…). Khi làm, `AXIS_META[key].label` đổi thành nhãn EN cố định, và thêm `gloss` lấy từ file dịch. Câu giải thích từ Rust (`sentences`, `summary` của `PairCandidateDto`) cũng phải theo quy ước này, tức là backend chèn nhãn EN của thuật ngữ.
- **Prototype:** có `STR` (chuỗi UI tiếng Việt) + `t(key)` và `TERMS` (nhãn EN + gloss vi) để minh hoạ. Chưa dịch hết mọi chuỗi và chưa có locale thứ hai.

## 7. Thay đổi so với 04 / 05

| Mục | Trước | Vòng 3 |
|---|---|---|
| Layout nền | 05 đề xuất A (thanh nổi tại chỗ) | **C** (cột vai + cuộn chọn); A và B không làm |
| Chế độ | So sánh là một lớp phủ của Pairing | **Hai chế độ ngang hàng** Pairing / Compare, có Stepper và phím riêng |
| Pairing Style, Mức ghép | Thanh trên + popover Lọc, áp dụng mọi nơi | **Chỉ ở Pairing**, nằm ở đầu Cuộn chọn (nguồn Ghép). Lọc ở Compare không có hai mục này |
| Mức ghép (An toàn / Có kiểm soát / Thử nghiệm) | Gọi là "Chế độ" | Đổi tên thành **"Mức ghép"**, vì "Chế độ" nay là Pairing / Compare |
| Chi tiết font | Không có (05), hoặc thẻ trong khay (04) | **Dải chi tiết** dưới trang: Đang dùng / Đang thử / Δ / meta |
| Compare | Chỉ font, mốc là giọng neo | **Font và Combo**. Mốc Font là font đang dùng cho vai đang chọn (không còn là giọng neo); có hàng meta và Δ |
| Nhãn trục trong UI | Tiếng Việt ("độ rộng", "tương phản nét") | Thuật ngữ EN (width, contrast) + gloss |
| Tên Pairing Style | Tiếng Việt (Cùng họ, Chân + không chân…) | Tên EN theo nguyên tắc thuật ngữ: Superfamily · Serif + Sans · Display + Text · Keep one, change one · Rule-breaking. Mô tả một dòng là gloss tiếng Việt |
| Cột Compare | Một danh sách | Vẫn một danh sách cho mọi vai (cột giữ khi đổi vai, mốc đổi theo vai). Xem câu hỏi §9 |

## 8. Dữ liệu và API cần thêm

| Cần | Trạng thái | Ghi chú |
|---|---|---|
| Meta cho dải chi tiết và Compare: designer, manufacturer, version, license, variableAxes, scripts | **Có** trong `MetadataDto` (`get_face`) | Dùng thẳng |
| Số glyph | **Có**: `FileInfoDto.numGlyphs` | |
| Độ phủ VI x/186 + ký tự thiếu | **Có**: `VietnameseDto.total`, `missing` | Thống nhất tổng 186 với đề xuất library-filter |
| Số đo trục + đơn vị + quality | **Có**: `AxisReadingDto` (`rawValue`, `unit`, `quality`) | Δ tính ở frontend từ `rawValue`. Trục `estimated` hiển thị nét đứt, `failed` hiện "—" |
| Số kiểu trong family, có Italic | **Có**: `get_family_styles` | |
| **Lấy chi tiết nhiều face một lần** | **Thiếu**: `faces_detail(keys[]) → FaceDetailDto[]` | Compare mở 4–6 cột cùng lúc; gọi `get_face` từng cái gây 6 round-trip và nhấp nháy |
| **Δ trục giữa hai face** (tuỳ chọn) | Có thể làm ở frontend | Backend chỉ cần trả `rawValue` đồng đơn vị (đã có) |
| **Bảng Compare lưu theo Case** | **Thiếu**: `case_compare (case_id, kind, cols JSON, pinned JSON, sort, dir, view, rows JSON, closed JSON)` | Mở lại đúng cột, thứ tự, nhóm đóng |
| `case_combos` | Như 05 §9 | Compare Combo đọc từ đây |
| Gloss thuật ngữ | **Thiếu**: file dịch `locales/<lang>/terms.json` (frontend) | Không cần backend |
| Câu giải thích dùng nhãn EN | **Đổi**: mẫu câu ở `explain` (Rust) chèn `term.label` thay cho tên trục tiếng Việt | Đồng bộ với §6 |

## 9. Câu hỏi còn mở

1. **Dải chi tiết dưới trang** (đề xuất, vì giữ trang lớn) hay panel bên phải như chủ app nói lúc đầu (trang nhỏ hơn khoảng 45%)?
2. **Hàng nguồn gợi ý** (Ghép · Giống · Cùng họ · Thư viện · Ghim) giữ dạng nhóm icon bấm một lần, hay đổi sang Option Stepper cho đồng nhất?
3. **Cột Compare Font** giữ chung cho mọi vai (hiện tại), hay mỗi giọng có danh sách cột riêng?
4. **"Mức ghép"** có được không (thay chữ "Chế độ" cũ cho An toàn / Có kiểm soát / Thử nghiệm)? Hay đặt tên tiếng Anh theo nguyên tắc thuật ngữ (ví dụ "Strictness")?
5. Ở 1100 px, dải chi tiết ẩn dòng meta. Chấp nhận được không, hay cần một nút mở meta đầy đủ ngay tại dải?

## 10. Prototype — đã kiểm

- **Console:** 0 lỗi.
- **Kịch bản tự động** chạy hết, không lỗi:
  - Pairing: mở Case → chọn vai → hover thử → `↓` → `⇧C` → `Enter` đặt → `⇧↓` đổi vai; thu/mở dải chi tiết; Pairing Style ‹ ›; Lọc theo chế độ.
  - `⌘2` sang Compare Font: Lọc không có Pairing Style; sắp theo contrast; thêm cột từ menu; kéo cột; kéo hàng trong nhóm; đóng/mở nhóm; ghim; bỏ; Đặt vào vai (mốc đổi); Song bản; Thử trong Pairing (về Pairing với font đang thử).
  - Compare Combo (từ dropdown Combo): sắp Δ x-height; ghim; thêm từ gợi ý; kéo cột; Dùng combo; Song bản.
  - `⌘1` quay lại; Stepper chế độ ‹ › và dropdown; sáng/tối; 5 bố cục; Case từ font thiếu tiếng Việt (nguồn Ghép rỗng → gợi ý Giống).
- **Ảnh** (`shots-v3/`, Chrome headless, 11 ảnh ở 1400×920 và 1100×800):
  - Pairing có thử / không thử / tối / 1100 mở và đóng dải chi tiết;
  - Compare Font sáng, tối 1100, Song bản;
  - Compare Combo bảng và Song bản;
  - home.
- **Lỗi thấy trên ảnh và đã sửa:**
  - panel chi tiết bên phải làm trang co còn khoảng 360 px → chuyển thành dải dưới trang;
  - nút ẩn ở đầu cột Compare vẫn chiếm chỗ làm cắt tên font → nút phủ lên góc phải;
  - dải chi tiết ở 1100 px quá cao → thu gọn thẻ;
  - icon trùng cạnh vòng VI.
- **Chưa kiểm:**
  - kéo thả bằng chuột thật (chỉ giả lập sự kiện DnD);
  - trình đọc màn hình;
  - locale thứ hai.
- **Dữ liệu:** số đo, version, số glyph là dữ liệu giả. Designer, foundry, trục variable lấy theo font Google thật ở mức minh hoạ, chưa đối chiếu từng file.
