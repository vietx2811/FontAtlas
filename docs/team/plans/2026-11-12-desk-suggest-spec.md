# Spec G5 + G6: gợi ý và chọn lựa trong Pairing Desk (0.4.0)

Soạn: 01 (Product & Research), 2026-11-12. **Trạng thái: chờ Lead duyệt, chưa ai code.**
Liên quan: kế hoạch `docs/team/plans/2026-11-12-v0.4.0-plan.md` (G2, G3, G4, G5, G6), nghiên cứu `docs/team/research/2026-11-12-desk-suggest.md`, thiết kế cũ `docs/design/font-desk/04_spec.md` §2.4 và `06_v3_pairing_compare.md` §2.
Người dùng: graphic designer, art director, layout artist. Tính năng tên "Pairing Desk", đơn vị "Case". Thuật ngữ font giữ tiếng Anh, luôn có TermTip.

---

## 0. Tóm tắt (đọc phần này là đủ để duyệt)

1. **Cột 2 ("Cuộn chọn") đổi thành "Picker" nhiều nguồn.** Năm nguồn chọn một, cạnh nhau: **Pairs** (hợp với font đang ở vai đối diện, là nguồn hiện có) · **Role fit** (hợp vai đang chọn, G4) · **Similar** (giống font đang đặt ở vai này) · **Pinned** (font đã ghim trong Case) · **Recent** (vừa xem). Một hàng chip **class** lọc nhanh (Serif, Sans, Mono…), một công tắc "Đủ tiếng Việt". Mỗi dòng có một câu "vì sao".
2. **Tên combo lên đầu Desk** thành ô sửa tại chỗ. Bấm vào hoặc gõ thì mở **popover "Gợi ý combo"**: nhận ra ý đồ từ tên ("Editorial", "Tech startup", "Menu nhà hàng"), dựng 3–6 combo **từ thư viện của người dùng**, hiện bằng chữ thật. Nhận **cả combo** (vào combo mới, hoặc điền combo đang rỗng) hoặc **từng vai** (vào combo đang mở). Mọi lần nhận có hoàn tác.
3. **Khoá cái đã chọn** (học từ Fontjoy): vai đã có font được giữ khi gợi ý lại, trừ khi người dùng bỏ khoá.
4. Backend thêm 2 lệnh (mục 7) do 08 làm, 04 cấp mô hình hợp vai và phong cách pair (G4). UI do 05. Phần chưa có dữ liệu thì UI hiện trạng thái rỗng đúng nghĩa, không chặn nhau (mục 6).

---

## 1. Nghiên cứu: rút ra gì

Chi tiết và mức chắc chắn ở file research. Năm điều dẫn tới quyết định:

| Quan sát | Nguồn | Quyết định |
|---|---|---|
| Khoá phần đã ưng, chỉ đổi phần còn lại; có nút "gợi ý khác" | Fontjoy | Khoá vai đã đặt; nút "Gợi ý khác" (seed mới) |
| Gợi ý theo ngữ cảnh dùng tốt hơn danh sách trơn | Typewolf, Google Fonts (cặp chọn tay kèm ví dụ thật) | Ô tên combo nhận ý đồ (Editorial, Menu…) và hiện combo bằng chữ thật |
| "Giống" và "hợp ghép" là hai nguồn khác nhau; bật ngay trên dòng | Adobe Fonts (Recommendations) | Hai nguồn riêng: Similar và Pairs; nút hành động ngay trên dòng |
| Hai font gần giống mà không giống hẳn trông như lỗi | Typeface help | Similar chỉ để *thay thế*, không dùng để ghép; Pairs đã có cổng va chạm |
| Không ai dựng gợi ý từ thư viện riêng của người dùng | cả sáu | Đây là điểm khác của FontAtlas: gợi ý chỉ từ font người dùng *đang có* (không mở mua/tải) |

Chưa kiểm được: nội dung bài Typewolf, cách FontBase làm "smart pairing". Không ảnh hưởng quyết định, ghi để khỏi nhắc như sự thật.

---

## 2. G5: Picker (cột 2)

### 2.1 Wireframe (Desk, 1280 px)

```
┌ ← Thư viện   [ Editorial        ▾ ✎ ]  Case: Tạp chí Bếp   ‹ Combo 2/3 ›   [Pairing|Fonts|Combos]  Bật cả Case ┐
├ VAI ───────────────┬ PICKER · Body ────────────────────┬ TRANG ──────────────────────────────────────────┤
│ DISPLAY            │ ( Pairs 12 )( Role fit )( Similar )│                                                 │
│  Fraunces          │ ( Pinned 4 )( Recent 9 )           │                                                 │
│ HEADING            │ ──────────────────────────────────│        [trang mẫu: heading + body thật]         │
│  Fraunces Soft     │ Class  [Sans][Serif][Mono][Slab]… │                                                 │
│▸BODY               │ [✓ Đủ tiếng Việt]        12 font  │                                                 │
│  Be Vietnam Pro    │ ──────────────────────────────────│                                                 │
│ CAPTION            │ ▸ Source Serif 4              📌  │                                                 │
│  (trống)           │   Ở Hà Nội, một ngôi nhà nhỏ…     │                                                 │
│ + Role             │   Cùng cấu trúc, đổi chân. ●x ◐serif│                                                │
│                    │   Lora                  [＋ Đặt]  │                                                 │
│                    │   Ở Hà Nội, một ngôi nhà nhỏ…     │                                                 │
│                    │   x-height cao hơn, dễ đọc cỡ nhỏ │                                                 │
│                    │ ↑↓ thử · ↵ đặt · 1–5 nguồn · / lọc│                                                 │
└────────────────────┴───────────────────────────────────┴──────────────────── Dải chi tiết ───────────────┘
```

- Tiêu đề cột: "Picker · {vai}" (vi: "Cuộn chọn · {vai}", giữ tên cũ trong UI tiếng Việt cho khỏi lạ; en: "Picker").
- Hàng nguồn là nhóm chọn một (`role="radiogroup"`), **không phải tab** (theo quyết định vòng 3: nút bật, chuyển một click). Mỗi chip có số lượng khi đã biết.
- Dòng ứng viên viết **chữ thật của vai** bằng chính font đó (như hiện nay), thêm một dòng "vì sao" (≤ 1 dòng, cắt có dấu …, đầy đủ trong tooltip).

### 2.2 Năm nguồn

| Nguồn (vi / en) | Trả lời câu hỏi | Dữ liệu | Có sẵn? |
|---|---|---|---|
| **Ghép / Pairs** | Font nào đi với font đang ở vai đối diện? | `pair_candidates(anchor, …)` (đang dùng ở Desk) | Có |
| **Hợp vai / Role fit** | Font nào *tốt cho vai này* (Body cần x-height, aperture, đọc cỡ nhỏ, đủ VI; Display cho phép tương phản cao)? Không cần font đối diện | Mới: `role_fit` (G4, 04 mô hình, 08 API) | Chưa (mục 7) |
| **Giống / Similar** | Font nào giống font đang đặt ở vai này (để thay: đủ tiếng Việt hơn, nhẹ hơn)? | `similar_families` / `similar` (embedding) | Có |
| **Đã ghim / Pinned** | Những font tôi đã ghim vào Case này | Danh sách font của Case (`CaseDto.faces`) | Có |
| **Vừa xem / Recent** | Font tôi vừa mở xem ở thư viện | `recentViews` (store) → `query_faces({faceKeys})` | Có |

- **Mặc định khi mở vai:** Pairs nếu vai đối diện có font; nếu không, Role fit; nếu Role fit chưa có (mục 7) thì Pinned. Nhớ lựa chọn cuối **theo vai** trong phiên Desk.
- **Similar** cần font đang đặt ở vai. Nếu vai trống: nguồn mờ đi với tooltip "Đặt một font vào vai này trước".
- Similar **không** hiện nhãn ●◐◆ (đó là nhãn của ghép); hiện "giống {n}%" và khác biệt chính ("rộng hơn", "x-height thấp hơn") do BE trả (đã có `summary` kiểu này ở phía Pairs).
- Mọi nguồn **loại** font `missing` (ổ rút, file mất) khỏi gợi ý, trừ Pinned: Pinned vẫn hiện, có NoteMark "Không khả dụng" và nút Đặt bị khoá.
- Font đang nằm ở vai (inUse) hiện đánh dấu "Đang dùng", Enter không làm gì.

### 2.3 Lọc nhanh

```
Class  [ Sans ][ Serif ][ Mono ][ Slab ][ Script ][ Handwriting ][ Display ][ Blackletter ]   (chọn nhiều = OR)
[✓ Đủ tiếng Việt]  (mặc định bật nếu Case đặt "Dự án dùng tiếng Việt")      12 font · Xoá lọc
```

- Chip class lấy từ phân loại hiệu lực (taxonomy v2; font có `uncertain` hiện ở chip "Chưa chắc", không ẩn). Chip chỉ hiện class **có trong tập đang xem** kèm số; chip 0 mờ đi, không biến mất (để bố cục không nhảy).
- Chip là thuật ngữ: mỗi chip có TermTip (gloss như ở bộ lọc thư viện: "Serif: có chân" ...). Không dịch tên class.
- Lọc **áp client-side** trên tối đa 100 dòng cho Pairs/Role fit/Similar; Pinned/Recent lọc qua `query_faces({classes, faceKeys})` để đúng khi >100. Đổi nguồn **giữ** bộ lọc.
- Công tắc "Đủ tiếng Việt" = chỉ font đủ bộ dấu của dự án (`viStatus` full); tắt thì font thiếu hiện với NoteMark đỏ "!" (motif sẵn có), không dùng chip chữ.
- Hết kết quả sau lọc: xem mục 6.

### 2.4 Dòng ứng viên

```
▸ Source Serif 4                       Pairs        📌   [＋ Đặt]
  Ở Hà Nội, một ngôi nhà nhỏ nằm sau   (chữ thật của vai, cỡ 15)
  Cùng cấu trúc, đổi chân.   ●x-height  ◐serif                       ← "vì sao" + ≤ 2 nhãn
```

- Nút trên dòng (hiện khi hover **hoặc focus**, luôn có trong cây truy cập): **＋ Đặt** (vào vai đang chọn, G2 của 05 dùng chung nút này), **📌 Ghim/Bỏ ghim**, **⫼ Đưa vào Compare** (có sẵn).
- Hover/rê = *thử* (trang đổi tạm, như hiện nay). Click = chọn dòng. Double-click hoặc Enter = đặt.
- Hiện nguồn của dòng khi một font xuất hiện ở nhiều nguồn: chỉ nguồn đang xem; không gộp.
- Kéo dòng thả vào Sidebar (Collection/Case) là việc của G1, dòng phải `draggable` và mang cùng payload với thẻ lưới.

### 2.5 Phím tắt (Desk, Picker có focus hoặc Desk root)

| Phím | Tác dụng |
|---|---|
| `↑` `↓` | Thử ứng viên trước/sau (đã có) |
| `Enter` | Đặt ứng viên đang thử vào vai (đã có) |
| `1`–`5` | Chọn nguồn: Pairs, Role fit, Similar, Pinned, Recent |
| `/` hoặc `F` | Chuyển focus vào hàng class; `←` `→` di chuyển, `Space` bật/tắt chip, `Esc` quay lại danh sách |
| `V` | Bật/tắt "Đủ tiếng Việt" |
| `P` | Ghim/Bỏ ghim font đang thử (Pinned) |
| `Esc` | Như hiện nay: về thư viện (nếu popover/lọc đang mở thì đóng cái đó trước) |

Phím một chữ không chạy khi focus trong ô nhập. Không đè phím sẵn có: `[` `]` (đổi combo), `I` (dải chi tiết), `⇧C` (Compare), `⌘1/⌘2` (chế độ).

---

## 3. G6: Tên combo lên đầu và gợi ý combo

### 3.1 Ô tên combo

```
┌ ← Thư viện   [ Editorial            ▾ ]  ✎   ‹ Combo 2/3 ›  ...
              └ tên combo, cỡ chữ tiêu đề (15–17 px), sửa tại chỗ
```

- Thay thế nhãn tên combo trong `ComboStepper`; stepper `‹ 2/3 ›` và menu combo (nhân đôi, đặt chính, xoá) giữ nguyên bên cạnh.
- Combo chưa có tên hiện placeholder "Đặt tên combo… (vd Editorial)" / "Name this combo… (e.g. Editorial)".
- `Enter` lưu tên; `Esc` bỏ thay đổi. Lưu qua `update_combo`.

### 3.2 Popover "Gợi ý combo"

Mở khi: ô tên nhận focus (click, hoặc phím `N` ở Desk), hoặc `↓` trong ô tên. Đóng bằng `Esc` hoặc bấm ra ngoài. **Tên đã gõ giữ nguyên** dù đóng.

```
[ Editorial_            ]
┌ Gợi ý combo cho "Editorial" ───────────────────────── Phong cách: [Tất cả ▾]  [⟳ Gợi ý khác] ┐
│ Hiểu là: tạp chí, báo, bài dài · Tương phản serif + sans         ( Giữ font đã đặt ✓ )         │
│ ┌───────────────────────────────┐ ┌───────────────────────────────┐ ┌───────────────────┐   │
│ │ Mùa nước nổi (Fraunces)       │ │ Mùa nước nổi (Playfair)       │ │ …                 │   │
│ │ Ở Hà Nội, một ngôi nhà nhỏ…   │ │ Ở Hà Nội, một ngôi nhà nhỏ…   │ │                   │   │
│ │ (Source Serif 4)              │ │ (Inter)                       │ │                   │   │
│ │ Serif + Sans · đủ tiếng Việt  │ │ Display + Text · thiếu ặ ẫ  ! │ │                   │   │
│ │ Heading Fraunces [＋]         │ │                               │ │                   │   │
│ │ Body    Source Serif 4 [＋]   │ │                               │ │                   │   │
│ │ [Nhận cả combo]               │ │ [Nhận cả combo]               │ │                   │   │
│ └───────────────────────────────┘ └───────────────────────────────┘ └───────────────────┘   │
│ ↑↓←→ chọn · ↵ nhận cả combo · Tab vào từng vai · R gợi ý khác · Esc đóng                      │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

**Nhận ra ý đồ từ tên** (client, bảng tĩnh `intents.ts`, vi + en, không gọi mạng, không LLM):

- Khớp không phân biệt hoa thường, bỏ dấu, theo từ khoá và đồng nghĩa ("editorial", "tạp chí", "magazine"; "tech startup", "công nghệ", "app"; "menu nhà hàng", "thực đơn", "restaurant"; "poster", "áp phích"; "brand", "thương hiệu"; "wedding", "thiệp cưới"; "báo cáo", "report"…). Danh sách khởi điểm ở Phụ lục A; 05/08 mở rộng được, mỗi ý đồ chỉ gồm: *bộ vai*, *các phong cách pair ưu tiên* (id của G4), *thẻ tâm trạng* (tuỳ chọn).
- Dòng "Hiểu là: …" luôn hiện, người dùng thấy máy hiểu gì và **đổi được** bằng menu "Phong cách".
- **Không nhận ra** (vd "Dự án 7"): vẫn gợi ý, dòng ghi "Chưa nhận ra ý đồ, gợi ý theo các cặp phổ biến" và bộ vai lấy từ combo đang mở (nếu rỗng: Heading + Body). Tên không bao giờ là điều kiện bắt buộc.
- Gõ dở thì khớp tiền tố sau 300 ms, huỷ yêu cầu cũ khi gõ tiếp.

**Dựng gợi ý** (BE, mục 7): từ **font của người dùng** (mặc định cả thư viện; công tắc "Chỉ font trong Case" cho phạm vi hẹp). Mỗi gợi ý = bộ (vai → font) + phong cách pair + 1–2 câu lý do + trạng thái VI. Sắp theo điểm tổng (hợp vai + hợp ghép) nhưng **không hiện số điểm**; chỉ hiện nhãn ●◐◆○ đã dùng ở Desk. Đa dạng: tối đa 1 gợi ý mỗi family tiêu đề, để 3–6 thẻ không giống nhau.

**Nhận:**

| Hành động | Kết quả | Hoàn tác |
|---|---|---|
| **Nhận cả combo** (Enter / nút) | Combo đang mở **rỗng** → điền. Đã có font → tạo **combo mới** (tên theo ô tên, hậu tố " 2") và chuyển sang đó; combo cũ nguyên vẹn | Toast "Đã tạo {tên} · Hoàn tác (⌘Z)" |
| **＋ ở một vai** | Đặt font đó vào vai tương ứng của combo **đang mở** (thay font cũ nếu có, font cũ hiện trong toast) | Toast "Đã đặt {font} vào {vai} · Hoàn tác" |
| **Gợi ý khác** (R) | Seed mới; vai đã đặt (và đang khoá) giữ nguyên | — |

- **Giữ font đã đặt** (mặc định bật): các vai đã có font coi là *neo khoá*; gợi ý chỉ đổi vai còn lại và vai trống. Tắt thì gợi ý lại toàn bộ.
- Khi gợi ý có vai mà combo chưa có (vd Caption): "Nhận cả combo" thêm vai đó (tạo vai tuỳ ý, G3).
- Thẻ gợi ý thiếu tiếng Việt vẫn hiện, có NoteMark đỏ "!" và tooltip nêu ký tự thiếu; nút Nhận vẫn bấm được (công cụ, không gác cổng). Nếu Case đặt "Dự án dùng tiếng Việt" thì thẻ đủ VI xếp trước.
- Chữ mẫu trong thẻ: tiêu đề mẫu lấy từ ô chữ mẫu của người dùng (nếu đã sửa), không thì câu mẫu tiếng Việt có đủ dấu chồng.

### 3.3 Phím tắt

| Phím | Ở đâu | Tác dụng |
|---|---|---|
| `N` | Desk | Focus ô tên combo, mở popover |
| `↓` | Ô tên | Mở popover / vào thẻ đầu |
| `←` `→` `↑` `↓` | Popover | Chọn thẻ (lưới) |
| `Enter` | Thẻ | Nhận cả combo |
| `Tab` | Thẻ | Vào danh sách vai của thẻ; `Enter` = nhận vai đó; `Shift+Tab` ra |
| `R` | Popover (không ở ô nhập) | Gợi ý khác |
| `L` | Popover | Bật/tắt Giữ font đã đặt |
| `Esc` | Popover | Đóng, giữ tên đã gõ |
| `⌘Z` | Desk | Hoàn tác lần nhận gần nhất (qua toast hoặc phím) |

---

## 4. Truy cập, nhịp, chi tiết giao diện

- Popover là `role="dialog"` không modal, có `aria-label` "Gợi ý combo". Lưới thẻ `role="listbox"` với `aria-activedescendant`; danh sách vai trong thẻ là `role="group"`. Kết quả thay đổi báo bằng vùng `aria-live=polite` ("6 gợi ý").
- Mọi nút chỉ-hiện-khi-hover đều xuất hiện khi focus (kiểm bàn phím G2).
- Ảnh/chữ mẫu dùng `FaceTitle`/`useFontFamily` sẵn có, tải font theo thứ tự hiển thị, tối đa ~8 font cùng lúc; ngoài màn hình thì chưa tải.
- Giảm chuyển động: popover không trượt, danh sách không fade.
- Hẹp (< 1100 px): hàng nguồn thành `<select>`; chip class xuống 2 dòng; popover rộng tối đa `calc(100vw - 32px)`, 1–2 thẻ mỗi hàng.

---

## 5. Chuỗi giao diện (vi + en), khởi điểm

Khoá bắt đầu `pick.` (Picker) và `cs.` (combo suggest). Thuật ngữ giữ tiếng Anh; câu giải thích đi vào khoá `*.gloss`.

| Khoá | vi | en |
|---|---|---|
| pick.title | Cuộn chọn · {role} | Picker · {role} |
| pick.src.pair | Ghép | Pairs |
| pick.src.role | Hợp vai | Role fit |
| pick.src.sim | Giống | Similar |
| pick.src.pin | Đã ghim | Pinned |
| pick.src.recent | Vừa xem | Recent |
| pick.src.pair.gloss | hợp với font đang ở vai đối diện | goes with the font in the opposite role |
| pick.src.role.gloss | tốt cho riêng vai này, không cần font đối diện | suits this role on its own |
| pick.src.sim.gloss | giống font đang đặt ở vai này, để thay thế | looks like the font in this role, to swap in |
| pick.src.pin.gloss | font bạn đã ghim trong Case này | fonts you pinned in this Case |
| pick.src.recent.gloss | font bạn vừa mở xem ở thư viện | fonts you just looked at in the library |
| pick.count | {n} font | {n} fonts |
| pick.class | Class | Class |
| pick.viOnly | Đủ tiếng Việt | Full Vietnamese |
| pick.clear | Xoá lọc | Clear filters |
| pick.pin / pick.unpin | Ghim / Bỏ ghim | Pin / Unpin |
| pick.place | Đặt | Place |
| pick.inUse | Đang dùng | In use |
| pick.needAnchor | Đặt một font vào vai {role} trước | Place a font in {role} first |
| pick.keys | ↑↓ thử · ↵ đặt · 1–5 nguồn · / lọc | ↑↓ try · ↵ place · 1–5 source · / filter |
| cs.name.ph | Đặt tên combo… (vd Editorial) | Name this combo… (e.g. Editorial) |
| cs.title | Gợi ý combo cho "{name}" | Combo ideas for "{name}" |
| cs.title.none | Gợi ý combo | Combo ideas |
| cs.understood | Hiểu là: {intent} | Read as: {intent} |
| cs.unknown | Chưa nhận ra ý đồ, gợi ý theo các cặp phổ biến | Didn't recognise the intent, showing common pairs |
| cs.style | Phong cách | Style |
| cs.style.all | Tất cả | All |
| cs.again | Gợi ý khác | More ideas |
| cs.keep | Giữ font đã đặt | Keep placed fonts |
| cs.takeAll | Nhận cả combo | Take the whole combo |
| cs.takeRole | Nhận vai {role} | Take {role} |
| cs.done.all | Đã tạo {name} | Created {name} |
| cs.done.role | Đã đặt {font} vào {role} | Placed {font} in {role} |
| cs.undo | Hoàn tác | Undo |
| cs.libOnly | Chỉ font trong Case | Case fonts only |
| cs.vi.ok | đủ tiếng Việt | full Vietnamese |
| cs.vi.miss | thiếu {chars} | missing {chars} |

Tên phong cách pair (Superfamily, Serif + Sans, Display + Text, Keep one change one, Rule-breaking…) và nhãn ●◐◆○ do 04 chốt trong G4, UI chỉ hiển thị id → chuỗi.

---

## 6. Trạng thái

| Tình huống | Picker (G5) | Gợi ý combo (G6) |
|---|---|---|
| **Đang tải** | Sau 150 ms hiện 6 dòng khung (skeleton); danh sách cũ mờ 60% trong lúc nạp lại (đổi vai/nguồn) để khỏi giật; huỷ yêu cầu cũ | Sau 150 ms hiện 3 thẻ khung; "Đang dựng gợi ý từ thư viện của bạn…" (aria-live) |
| **Rỗng: nguồn không có gì** | Pinned: "Chưa ghim font nào. Bấm 📌 trên dòng hoặc nhấn P." · Recent: "Chưa xem font nào trong phiên này." (+ nút "Mở thư viện") · Pairs: giữ hai nút cũ "Thử Có kiểm soát", "Xem Giống" và tự điền 4 dòng từ Similar có ghi nguồn · Similar: "Chưa có font giống đủ gần" | "Chưa dựng được gợi ý với bộ lọc này." + nút "Bỏ giữ font đã đặt" / "Tất cả phong cách" |
| **Rỗng: lọc loại hết** | "Không có font nào khớp lọc. {n} font bị ẩn" + "Xoá lọc" | — |
| **Thư viện quá nhỏ** | < 12 family: ghi "Thư viện còn ít font, gợi ý có thể lặp" (NoteMark, không chặn) | < 6 family: tương tự; < 2: "Cần ít nhất 2 family để ghép" + nút thêm thư mục |
| **Dữ liệu chưa sẵn** | Similar khi embedding chưa xong: "Đang phân tích hình ảnh {n}%" + liên kết Cài đặt ▸ Phân tích; nguồn khác dùng được. Role fit khi chưa có điểm cho font: dòng vẫn hiện, nhãn "chưa đo" | Thiếu số đo của nhiều font: hiện gợi ý có NoteMark "một số font chưa đo" |
| **Lỗi** | Lỗi **theo nguồn**, hiện ngay trong danh sách (`role=alert`) + "Thử lại"; nguồn khác vẫn dùng | Lỗi trong popover + "Thử lại"; tên combo không mất |
| **Font mất** | Pinned hiện NoteMark "Không khả dụng"; các nguồn khác loại ra | Loại ra; nếu cả combo phụ thuộc, bỏ thẻ |
| **Không có vai đối diện** | Pairs mờ, tooltip `pick.needAnchor`; mặc định chuyển sang Role fit/Pinned | Không ảnh hưởng |
| **Đổi vai khi đang nạp** | Bỏ kết quả vai cũ (đã có cơ chế `candRole`) | Bỏ kết quả cũ nếu tên/phong cách đổi |

---

## 7. Hợp đồng dữ liệu (đề xuất cho 04 và 08, chưa có trong code)

Mọi thứ cục bộ, không gọi mạng. DTO mới sinh bindings như thường (`bindings-drift` phải sạch).

**7.1 `role_fit(role, opts) -> Vec<RoleFitDto>`** (G5, nguồn Role fit; 04 mô hình, 08 API)

- Vào: `role` (vai, kể cả vai tuỳ ý của G3 → nhóm vai gần nhất), `projectVi`, `k ≤ 100`, `classes?`, `collectionId?`, `lang`.
- Ra mỗi dòng: `face`, `row`, `score` (nội bộ, UI không hiện số), `labels` (≤ 2 nhãn đã dùng), `reason` (1 câu, đã bản địa hoá), `insufficientData`.
- Tiêu chí theo vai (G4): Body cần x-height, aperture, đọc cỡ nhỏ, đủ VI; Display cho phép contrast cao, nét đặc trưng.

**7.2 `suggest_combos(req) -> Vec<ComboSuggestionDto>`** (G6; 08)

- Vào: `intent?` (id từ bảng ý đồ, hoặc null), `styles?` (id phong cách G4), `roles` (bộ vai cần dựng), `locked` (`[{role, face}]`), `scope` (`library` | `case`), `caseId`, `projectVi`, `k` (3–6), `seed`, `lang`.
- Ra mỗi gợi ý: `slots: [{role, face, row}]`, `style` (id), `reasons: [string]`, `viStatus` (`full` | `partial` + danh sách ký tự thiếu), `warnings` (cổng va chạm đã có của Pairs), `seed`.
- Quy tắc: chỉ font có trong thư viện và không `missing`; tối đa 1 gợi ý mỗi family tiêu đề; kết quả **ổn định theo seed** (cùng seed cùng kết quả, để test được); thời gian mục tiêu < 400 ms ở thư viện 5.000 family (08 xác nhận).

**7.3 Không cần lệnh mới:** Pairs (`pair_candidates`), Similar (`similar_families`), Pinned (`CaseDto.faces` + `query_faces`), Recent (`recentViews`), nhận cả combo (`create_combos` + `set_combo_slot`), hoàn tác (hệ undo sẵn của Desk, nếu chưa bọc được thì 05 báo).

**7.4 Lưu lựa chọn:** nguồn cuối theo vai và bộ lọc class lưu trong `CaseDto.view_state` (BE chỉ lưu, đã có). Không lưu tên ý đồ.

**Ai làm gì:** 04 = tiêu chí hợp vai + phong cách (G4) · 08 = hai lệnh trên + test seed ổn định · 05 = Picker UI, ô tên, popover, phím, toast hoàn tác · 01 = bảng ý đồ (Phụ lục A) và rà bản UI thật so với spec này.

---

## 8. Phụ lục A: bảng ý đồ khởi điểm

| Id | Từ khoá vi / en | Bộ vai | Phong cách ưu tiên (id G4, 04 xác nhận) |
|---|---|---|---|
| editorial | editorial, tạp chí, báo, magazine, article | Display, Heading, Body, Caption, Quote | Serif + Sans, Display + Text |
| tech | tech startup, công nghệ, app, saas | Heading, Body, Label | Superfamily, Keep one change one |
| menu | menu, thực đơn, nhà hàng, restaurant | Display, Heading, Body, Price (tuỳ ý) | Display + Text, Serif + Sans |
| poster | poster, áp phích, flyer | Display, Subhead, Body | Display + Text, Rule-breaking |
| brand | brand, thương hiệu, identity | Display, Heading, Body | Superfamily, Serif + Sans |
| wedding | wedding, thiệp, cưới, invitation | Display, Subhead, Body | Display + Text |
| report | báo cáo, report, annual | Heading, Subhead, Body, Caption, Numerals | Superfamily, Keep one change one |
| packaging | bao bì, packaging, nhãn | Display, Heading, Caption | Display + Text, Rule-breaking |

Bảng này là *gợi ý khởi điểm để 05 dựng và test*; người dùng GD/AD nên được hỏi thêm tên ý đồ họ hay đặt (câu hỏi 3 bên dưới).

---

## 9. Câu hỏi cần Lead / chủ app quyết

1. **Số nguồn:** năm nguồn có nhiều quá cho cột 300 px? Phương án rút: gộp Recent vào Pinned thành "Của tôi" (hai nhóm con). *Đề xuất giữ năm; kiểm với bản thật, rút nếu chật.*
2. **Nhận cả combo khi combo đang có font:** đề xuất tạo combo mới (an toàn). Chủ app có muốn thay thế tại chỗ không?
3. **Tên ý đồ:** chủ app hay đặt tên combo thế nào ("Editorial", "Tech startup", "Menu nhà hàng" là ví dụ của chủ app)? Cần 10 tên thật để dựng bảng Phụ lục A đúng nghề.
4. **Vai tuỳ ý (G3) và Role fit:** vai tên tự do ("Giá", "Eyebrow") ánh xạ về nhóm vai gần nhất bằng bảng đồng nghĩa hay hỏi người dùng một lần? Cần 04 và 05 thống nhất.
5. **Phạm vi gợi ý combo:** mặc định cả thư viện hay chỉ font trong Case? Đề xuất cả thư viện (vì Case thường còn ít font), có công tắc thu hẹp.

---

## 10. Tiêu chí nghiệm thu (cho 05/08 và bước rà của 01)

- G5: năm nguồn đổi bằng chuột và `1`–`5`; lọc class và "Đủ tiếng Việt" giữ khi đổi nguồn; mọi trạng thái ở mục 6 có thật (có test hoặc ảnh chụp).
- G6: gõ "Editorial" ra ≥ 3 gợi ý từ thư viện thử; "Nhận cả combo" vào combo mới, hoàn tác được; "＋" một vai đặt đúng vai; khoá font đã đặt đúng; gõ tên lạ vẫn có gợi ý.
- Cả hai: có vi và en đủ khoá; thuật ngữ font có TermTip; chi tiết cảnh báo dùng NoteMark; thao tác được hoàn toàn bằng bàn phím; ảnh chụp sáng/tối ở 1280 và 1100 px.
