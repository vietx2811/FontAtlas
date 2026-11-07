# 05 — Pairing Desk · vòng 2: phương án layout, Combo, So sánh, icon, bộ lọc

- **Ngày:** 2026-11-06 · **Prototype:** `docs/design/prototype/pairing-desk-v2.html` · **Ảnh:** `docs/design/font-desk/shots-v2/`
- **Thay đổi so với vòng 1** (quyết định của chủ app):
  - Tên tính năng là **Pairing Desk** (trước gọi Pairing Studio). Đơn vị làm việc là **Case**. Không dùng "Plate" nữa.
  - Bỏ định hướng F&B. Đối tượng là Graphic Designer, Art Director, Layout Artist. Nội dung mẫu đa ngành: editorial, branding, poster, UI/web, bao bì nói chung. Vẫn dùng tiếng Việt thật có dấu chồng.
- File 01–04 vẫn đúng về cơ chế: giọng/vai, nguồn gợi ý, nhãn trục, dữ liệu, API. Riêng tên gọi và persona theo file này.

---

## 1. Hệ từ mới

| Khái niệm | UI tiếng Việt | English | Ghi chú |
|---|---|---|---|
| Tính năng, màn toàn phần phải | **Pairing Desk** | Pairing Desk | Giữ tên tiếng Anh cả trong UI tiếng Việt, như "Atlas" |
| Mục sidebar | **Pairing Desk** (nhóm), bên dưới là các Case có sao, rồi dòng "Tất cả Case · N" | | Thay "Plates" / "Pairing Labs" |
| Đơn vị làm việc | **Case** | Case | Tự lưu, đánh sao, thùng rác 30 ngày |
| Một phương án ghép trong Case | **Combo** | Combo | Một Case có 1–n combo; mỗi combo là một bộ giọng đầy đủ. Thay ý "nhân bản Plate" ở vòng 1 |
| Font khởi đầu | Neo | Anchor | |
| Ô font | Giọng (①②③) | Voice | Tối đa 3, cảnh báo nhẹ khi vượt |
| Khối chữ | Vai | Role | |
| Loại ấn phẩm | Bố cục | Layout | Editorial · Brand sheet · Poster · UI/Web · Bao bì |
| Chế độ ghép | Chế độ | Mode | An toàn · Có kiểm soát · Thử nghiệm |
| Kiểu ghép | Kiểu ghép | Pair style | Cùng họ · Chân + không chân · Trưng bày + đọc · Giữ một đổi một · Phá cách |
| Xem tạm / ghi | Thử / Đặt | Try / Set | |
| Chế độ so sánh mở rộng | **So sánh** | Compare | Nút ⤢, phím `C` |

**Zerostate** (Pairing Desk home): "Bắt đầu từ font nào?" có ô tìm, font gần đây và chọn bố cục. Bên dưới là "Case có sao", rồi "Khác", rồi "Đã xoá".

## 2. Persona (thay §1 của `01_research.md`)

| Persona | Việc chính | Cần nhất ở Pairing Desk |
|---|---|---|
| **Graphic Designer** (studio, freelance) | Poster, ấn phẩm sự kiện, social, bao bì | Thử nhanh nhiều font tiêu đề trên bố cục thật; chữ HOA có dấu chồng không vỡ |
| **Art Director** (agency, in-house) | Brand sheet, hệ chữ cho chiến dịch, duyệt phương án | So 2–3 combo cạnh nhau, giải thích được lý do cho khách |
| **Layout Artist** (tạp chí, sách, báo cáo) | Dàn trang dài, nhiều cấp tiêu đề | Độ xám đoạn văn, x-height, italic thật, khoảng dòng tiếng Việt |
| UI/Web designer (phụ) | Landing, app | Cỡ nhỏ, nút, số liệu; font đủ tiếng Việt cho web |

Job-to-be-done giữ nguyên: từ một font neo, dựng hệ chữ trên đúng loại ấn phẩm, so vài phương án, hiểu lý do, chốt 2–3 font, quay lại sau. Insight I1–I8 ở `01_research.md` §6 vẫn đúng. Phần nói về F&B (menu, giá) chỉ còn là một ví dụ.

---

## 3. Bốn tác vụ chuẩn và cách đo

Đo **ước lượng** trên cửa sổ 1400×920, sidebar 240 px:
- **Click:** số lần bấm chuột, không tính cuộn và hover.
- **Quãng chuột:** tổng khoảng cách con trỏ đi, làm tròn 50 px, tính từ vị trí khối chữ đang làm việc (giữa trang).
- **Phím:** cách làm chỉ bằng bàn phím, nếu có.

Đây là số đếm trên sơ đồ, chưa đo bằng người dùng thật.

| Mã | Tác vụ |
|---|---|
| T1 | Đổi font cho một vai (chọn một gợi ý) |
| T2 | Thử 3 ứng viên trên trang rồi quay lại |
| T3 | So sánh chi tiết 2 combo (specimen + thông số) |
| T4 | Đổi bố cục hoặc chế độ ghép |

---

## 4. Các phương án

### Mốc — v1 "Trang + Sổ giọng + Khay" (vòng 1)

```
┌ top bar: tên · bố cục · VI · độ sâu ○●○ ─────────────────┐
│ TRANG                                        │ SỔ GIỌNG  │
│                                              │           │
├──────────────────────────────────────────────┴───────────┤
│ KHAY: nguồn · chế độ · thẻ ứng viên [Thử][Đặt] ……         │
└──────────────────────────────────────────────────────────┘
```
Vấn đề: chọn vai ở giữa trang, nhưng gợi ý và nút Đặt nằm ở khay dưới cùng, cách 400–500 px. Độ sâu, chế độ và bố cục nằm ở thanh trên. Mắt và chuột phải đi lại liên tục.

### A — "Tại chỗ" (thanh nổi cạnh khối chữ)

```
┌ top bar: Case ☆ · ‹ Bố cục ▾ › · ‹ Combo 2/3 ▾ › · ⚲ Lọc · ⤢ So sánh · ☾ ┐
│                                                         │ ①②③ │
│        ┌─────────────────────────────────────────┐      │ rail │
│        │ ① ‹ [Fraunces ▾] › │ B− B+ │ Aa │ A− A+ │ 📌 ⤢ │      │ 56px │
│        └─────────────────────────────────────────┘      │      │
│        ╎ Những ngôi nhà ống biết thở ╎  ← vai đang chọn  │      │
│          Năm kiến trúc sư trẻ kể chuyện…                 │      │
│                                                         │      │
│        [Fraunces ▾] mở danh sách ngay dưới thanh:       │      │
│        ┌ 🔍 tìm · ✦ Ghép  ⧉ Giống  T Cùng họ  ⊞ Thư viện ┐│      │
│        │ Fraunces       ● ● ◐   Giữ x-height…           ││      │
│        │ Lora           ● ◐     …   (hover = thử)       ││      │
│        └────────────────────────────────────────────────┘│      │
└─────────────────────────────────────────────────────────┴──────┘
```
- Bấm một khối chữ → **thanh nổi** hiện ngay trên khối, cách 12 px:
  - huy hiệu giọng;
  - `‹ ›` chuyển ứng viên (thử ngay trên trang);
  - tên font ▾ (danh sách gợi ý thả xuống, hover là thử);
  - độ đậm −/+, hoa/thường, cỡ −/+;
  - ghim, so sánh.
- Đang thử: thanh hiện thêm "Thử 2/4", `✓ Đặt` và `✕ Bỏ`; `Enter` / `Esc` làm tương tự.
- Danh sách thả xuống mở **ngay dưới khối chữ** (không che chính khối đang sửa). Đầu danh sách có ô tìm và Stepper kiểu ghép; tab nguồn là icon kèm chữ. Đáy danh sách ghi phím tắt.
- Rail phải chỉ 56 px: ba chấm giọng ①②③, hover thì bung tên font. Mọi thứ khác nằm ở thanh trên, gom theo nhóm.
- **Ưu:** quãng chuột ngắn nhất cho T1, T2; trang chiếm gần hết màn; học như Figma, Canva.
- **Nhược:** thanh nổi che một phần dòng phía trên (tự lật xuống dưới khi khối sát mép); chỉ thấy một ứng viên trên trang tại một lúc.

### B — "Song bản" (nhiều bản trang đặt cạnh nhau)

```
┌ top bar: Case ☆ · ‹ Bố cục › · ‹ Combo › · Vai: ‹ Tiêu đề ▾ › · ⚲ · ⤢ ┐
│ ┌ Đang dùng ──────┐ ┌ Lora ────────────┐ ┌ Roboto Slab ─────┐  ‹ 1–3/12 › │
│ │ ① Fraunces       │ │ ●● ◐ · An toàn    │ │ ● ◐ ◐ · An toàn  │             │
│ │ [trang thu nhỏ]  │ │ [trang thu nhỏ]   │ │ [trang thu nhỏ]  │             │
│ │                  │ │                   │ │                  │             │
│ │                  │ │ [✓ Đặt] [📌] [⤢]   │ │ [✓ Đặt] [📌] [⤢]  │             │
│ └──────────────────┘ └───────────────────┘ └──────────────────┘             │
│ ▸ Thông số (mở được): x-height · tương phản · độ rộng · tiếng Việt          │
└──────────────────────────────────────────────────────────────────────────────┘
```
- Trang hiện thành **3 bản** (2 bản ở 1100 px). Bản đầu là combo hiện tại, các bản sau thay giọng của vai đang chọn bằng ứng viên kế tiếp. `‹ ›` lật trang ứng viên theo nhóm 3.
- Bấm một vai trên bất kỳ bản nào thì đổi vai đang chọn cho cả dãy.
- **Ưu:** T2 không cần click (nhìn là thấy); T3 gần như có sẵn; hợp AD trình bày phương án cho khách.
- **Nhược:** trang nhỏ lại (khoảng 1/3), không hợp bố cục dài như Editorial; tốn tài nguyên (3 bộ `FontFace`); sửa chữ trên bản thu nhỏ khó.

### C — "Cột vai + Cuộn chọn" (bàn phím trước)

```
┌ top bar: Case ☆ · ‹ Bố cục › · ‹ Combo › · ⚲ · ⤢ ──────────────────────┐
│ CỘT VAI (sát trang) │ CUỘN CHỌN ↕ (mở khi chọn vai) │ TRANG               │
│ Eyebrow   ② Be Viet │ ▲                             │                      │
│ Tiêu đề   ① Fraunces│  Lora          ● ◐            │  Những ngôi nhà ống  │
│ ▸Phụ đề   ① Fraunces│ ▶Roboto Slab   ● ◐  ← đang thử│  biết thở            │
│ Đoạn văn  ② Be Viet │  Source Serif  ● ●            │                      │
│ Chú thích ② Be Viet │ ▼   ⚲ ✦Ghép ⧉Giống T Cùng họ  │                      │
└─────────────────────┴───────────────────────────────┴──────────────────────┘
```
- Bên trái trang là **cột vai**: mỗi dòng là tên vai + tên font viết bằng chính font đó + nhãn rút gọn. Đây là "cột ghi tên font" trong ý gốc của chủ app, đặt sát trang.
- Chọn một dòng → **cuộn chọn** mở ngay bên phải dòng đó. Đó là danh sách dọc ứng viên, mỗi ứng viên viết bằng chữ của vai đó.
- Lăn chuột hoặc `↑ ↓` để thử từng ứng viên trên trang; bấm hoặc `Enter` để đặt; `←` đóng; `Tab` sang vai kế.
- **Ưu:** T2 bằng lăn chuột, không click; điều khiển hoàn toàn bằng bàn phím; luôn thấy toàn bộ hệ vai → font.
- **Nhược:** chiếm thêm chiều ngang (khoảng 220 + 240 px khi mở cuộn chọn) nên trang hẹp ở 1100 px; người quen thao tác trực tiếp trên trang thấy gián tiếp hơn A.

### Ma trận ước lượng

| Tác vụ | v1 (mốc) | **A Tại chỗ** | B Song bản | C Cột vai + Cuộn chọn |
|---|---|---|---|---|
| T1 đổi font một vai | 2 click · ~750 px | **2 click · ~100 px** (vai → ›, Đặt) · phím: vai, `→`, `Enter` | 2 click · ~450 px | 2 click · ~150 px · phím: `↑↓` `→` `↓` `Enter` |
| T2 thử 3 ứng viên | 0 click · ~1.150 px (hover 3 thẻ ở khay) | 3 click · ~0 px (› ×3) · phím `→` ×3 | **0 click · 0 px** (cả 3 bản cùng hiện) | **0 click · ~0 px** (lăn 3 nấc) |
| T3 so sánh 2 combo chi tiết | 2–3 click · ~900 px (độ sâu 2, cuộn khay) | 2 click · ~650 px (⤢ trên thanh trên, + cột) | **1 click · ~300 px** (▸ Thông số) | 2 click · ~650 px |
| T4 đổi bố cục / chế độ | 1 click · ~500 px | 1 click · ~500 px · phím `⌥1–5`, `M` | 1 click · ~500 px | 1 click · ~500 px |
| **Tổng T1–T3** | ~5 click · ~2.800 px | ~7 click · ~750 px | ~3 click · ~750 px | ~4 click · ~800 px |
| Trang hiển thị | ~60% vùng | **~85%** | ~30% mỗi bản | ~50% |
| Hợp bố cục dài (Editorial) | tốt | **tốt** | kém | vừa |

**Đề xuất:** **A làm mặc định, kèm chế độ So sánh (§6)**. Lấy thêm hai thứ từ các phương án khác:
- từ C: phím `↑↓` trong danh sách thả xuống của A và lăn chuột trên `‹ ›`;
- từ B: nút "Song bản" trong So sánh, để xem 3 trang cạnh nhau khi cần trình bày.

A giảm khoảng 3,7 lần quãng chuột so với v1 mà vẫn giữ trang lớn nhất. B mạnh ở T2, T3 nhưng làm trang quá nhỏ cho việc dàn trang. **Chủ app chọn:** prototype có công tắc A / B / C trên thanh trên.

---

## 5. Option Stepper và Combo indicator

### 5.1 Option Stepper (thay mọi dạng chấm ○●○ và tab dài)

Thay cho chấm và tab, mọi tập lựa chọn rời rạc dùng **một** control:

```
[ ‹ ]  [ ⧉ Combo 2/3 ▾ ]  [ › ]
```
- `‹` `›`: chuyển sang lựa chọn trước/sau ngay, không mở gì. Ở đầu hoặc cuối danh sách thì nút mờ đi (không vòng lại).
- Bấm nhãn: mở **dropdown** liệt kê mọi lựa chọn, có icon và mô tả một dòng. Mục cuối (nếu tập đó cho thêm) là **"+ Thêm…"**.
- Lăn chuột trên control = `‹ ›`.
- **Bàn phím:** focus control rồi dùng `←` `→`; `Enter` / `Space` / `↓` mở dropdown; `Esc` đóng. Phím toàn cục `[` `]` cho Combo.
- **Dùng cho:**
  - Combo (`[` `]`);
  - Bố cục (`⌥1–5`);
  - Chế độ An toàn / Có kiểm soát / Thử nghiệm (`M` xoay vòng);
  - Kiểu ghép;
  - Vai đang chọn (ở B; ở A, C dùng `↑ ↓`).
- Nhãn luôn có icon. Rộng tối thiểu bằng nhãn dài nhất của tập, để `‹ ›` không nhảy vị trí khi đổi.

### 5.2 Combo indicator `‹ ⧉ Combo 2/3 ▾ ›`

**Dropdown:**
```
┌───────────────────────────────────────────────────┐
│ ✓ 1  Fraunces + Be Vietnam Pro    ◐ chân  ● x-h   │  ★
│   2  Fraunces + Lora              ● chân  ◐ nét   │
│   3  Playfair + Inter       Giữ một, đổi một       │
│ ─────────────────────────────────────────────────  │
│ Gợi ý (chưa lưu)                                   │
│   ✦  Fraunces + Source Sans 3     ● ● ◐           │
│   ✦  Fraunces + Roboto Slab       ● ◐             │
│ ─────────────────────────────────────────────────  │
│ ＋ Lưu combo hiện tại thành combo mới    ⌘⇧S       │
│ ⤢  So sánh các combo                     C         │
└───────────────────────────────────────────────────┘
```
- Mỗi dòng gồm: số thứ tự, tên tự sinh "Giọng ① + Giọng ②" (sửa được), 2–3 nhãn trục rút gọn của cặp, sao nếu là combo chính.
- Hover một dòng = **thử** combo đó trên trang; bấm = chuyển sang combo đó.
- Dòng "Gợi ý" lấy 2 combo đầu từ `pair_candidates`. Bấm vào là thêm thành combo mới, chưa ghi đè combo đang dùng.
- **Trạng thái:**

  | Trạng thái | Hiển thị |
  |---|---|
  | Một combo | `‹ ›` mờ; nhãn "Combo 1/1"; dropdown vẫn có Gợi ý và "+ Lưu" |
  | Combo vừa sửa | Chấm nhỏ cạnh số ("Combo 2/3 •") trong 2 giây sau khi tự lưu |
  | Đang thử combo khác (hover dropdown) | Nhãn in nghiêng "Thử: 3/3", trang đổi tạm |
  | Combo có font không khả dụng | Icon ⚠ ở dòng đó |
- **Icon:** `layers` cho combo, `sparkles` cho gợi ý, `plus` cho thêm, `columns-3` cho so sánh, `star` cho combo chính của Case.

## 6. Chế độ So sánh mở rộng

Mở bằng nút ⤢ (`columns-3`) trên thanh trên, trên thanh nổi (A), trên đầu mỗi bản (B), hoặc phím `C`. Chế độ này thay toàn vùng trang (sidebar giữ nguyên). Đóng bằng `Esc` hoặc nút ⤡.

```
┌ So sánh · Giọng: ‹ ② Đọc ▾ › · Sắp: ‹ x-height ▾ › ↓ · + Thêm cột · Song bản · ⤡ ┐
│            ┃ ⠿ 📌 Be Vietnam Pro ✕ ┃ ⠿ Lora ✕       ┃ ⠿ Roboto Slab ✕ ┃ + │
│ ⠿ Tiêu đề  ┃ Những ngôi nhà ống    ┃ Những ngôi nhà ┃ Những ngôi nhà  ┃   │
│ ⠿ Đoạn văn ┃ Năm kiến trúc sư …    ┃ …              ┃ …               ┃   │
│ ⠿ Dấu chồng┃ Ữ Ặ Ệ Ỗ ươ            ┃ …              ┃ …               ┃   │
│ ⠿ x-height ┃ 0,53 em  ●            ┃ 0,50 em  ●     ┃ 0,54 em  ●      ┃   │
│ ⠿ Tương phản┃ 1,2 : 1  ◐           ┃ 2,2 : 1  ◐     ┃ 1,3 : 1  ◐      ┃   │
│ ⠿ Độ rộng  ┃ 1,02     ◆            ┃ 0,98     ●     ┃ 1,00     ●      ┃   │
│ ⠿ Tiếng Việt┃ 134/134 ●            ┃ 134/134 ●      ┃ 134/134 ●       ┃   │
│ ⠿ Cổng     ┃ Thử nghiệm            ┃ An toàn        ┃ An toàn         ┃   │
└──────────────────────────────────────────────────────────────────────────┘
```
- **Cột** là font (hoặc combo) đang so. Cột đầu luôn là **mốc** (giọng neo, nền chấm nhạt). Mọi nhãn ●◐◆○ tính so với mốc.
- **Hàng** gồm hai nhóm, kéo `⠿` để đổi thứ tự trong nhóm:
  - **specimen:** chữ thật của các vai trong bố cục hiện tại + hàng dấu chồng;
  - **thông số:** số đo thật + đơn vị + nhãn.
- **Thao tác:**

  | Việc | Cách |
  |---|---|
  | Đổi vị trí cột | Kéo `⠿` ở đầu cột; hoặc focus đầu cột rồi `⌥←` `⌥→` |
  | Sắp xếp | Stepper "Sắp theo" (x-height, tương phản, độ rộng, số ◆, tên) + nút chiều ↑↓; hoặc bấm tên một hàng thông số. Cột ghim không bị sắp lại |
  | Ghim cột | `📌` (`pin`): cột dính bên trái sau mốc, không bị sắp lại |
  | Bỏ cột | `✕` hiện khi hover hoặc focus; `⌫` khi đầu cột đang focus |
  | Thêm cột | `+` cuối hàng đầu: thêm ứng viên kế tiếp từ nguồn Ghép; giữ `⌥` khi bấm để mở danh sách chọn |
  | Đặt | Bấm đúp đầu cột, hoặc nút `✓` hiện khi hover → đặt font đó cho giọng đang so |
  | Song bản | Nút chuyển sang hiển thị trang thu nhỏ theo cột (từ phương án B) |
- **Ít chữ:** đầu hàng chỉ có icon + tên ngắn (x-height, Nét, Rộng, VI). Đơn vị đặt cạnh số. Câu giải thích hiện khi hover vào ô.
- Số đo trong prototype là số giả; app thật lấy `AxisReadingDto` (giá trị gốc + quality).

## 7. Icon

**Bộ icon:** lucide (`lucide-react` 0.561 đã có trong app). Nét 1,5 px, cỡ 16 px trên thanh và 14 px trong chip, màu `currentColor`. Nhãn trục ●◐◆○ là ký hiệu chữ, không thay bằng icon: đó là hệ nhãn đã chốt và không phụ thuộc màu.

| Hành động / khái niệm | Icon lucide | Luôn hiện / hover |
|---|---|---|
| Pairing Desk (sidebar) | `blend` | luôn |
| Case | `file-text` | luôn |
| Case mới | `plus` | hover nhóm |
| Sao Case / combo chính | `star` | luôn khi đã sao; hover khi chưa |
| Combo | `layers` | luôn |
| Gợi ý (nguồn Ghép, combo gợi ý) | `sparkles` | luôn |
| Giống | `copy` | luôn |
| Cùng họ | `type` | luôn |
| Thư viện | `library` | luôn |
| Đã ghim / ghim | `pin` / `pin-off` | trong danh sách: hover; cột ghim: luôn |
| Trước / sau | `chevron-left` / `chevron-right` | luôn |
| Mở dropdown | `chevron-down` | luôn |
| Thử | `eye` | hover dòng |
| Đặt | `check` | luôn khi đang thử; hover trong danh sách |
| Bỏ thử / đóng | `x` | khi đang thử |
| Hoàn tác | `undo-2` | luôn |
| Bố cục | `layout-template` | luôn |
| Editorial · Brand sheet · Poster · UI/Web · Bao bì | `book-open-text` · `swatch-book` · `presentation` · `panels-top-left` · `package` | luôn |
| Kiểu ghép: Mọi · Cùng họ · Chân + không chân · Trưng bày + đọc · Giữ một đổi một · Phá cách | `blend` · `type` · `case-sensitive` · `presentation` · `anchor` · `flask-conical` | luôn |
| Chỉ font có italic thật · ≥ 4 độ đậm · Nhãn trục | `italic` · `bold` · `tags` | luôn (trong Lọc) |
| Chế độ An toàn · Có kiểm soát · Thử nghiệm | `shield-check` · `gauge` · `flask-conical` | luôn |
| Kiểu ghép | `blend` | luôn |
| Lọc | `sliders-horizontal` | luôn; có số lọc đang bật |
| Tìm | `search` | luôn |
| So sánh / thu lại | `columns-3` / `minimize-2` | luôn |
| Sắp theo / chiều | `arrow-down-wide-narrow` / `arrow-up-down` | luôn trong So sánh |
| Kéo đổi chỗ | `grip-vertical` | hover hàng/cột |
| Xoá cột | `x` | hover |
| Độ đậm − / + | `minus` / `plus` cạnh `bold` | luôn trên thanh nổi |
| Hoa / thường | `case-sensitive` | luôn trên thanh nổi |
| Cỡ − / + | `a-arrow-down` / `a-arrow-up` | luôn trên thanh nổi |
| Cân x-height | `scaling` | luôn (công tắc) |
| Tiếng Việt | `languages` | luôn (công tắc) |
| Cảnh báo (thiếu ký tự, font không khả dụng) | `triangle-alert` | luôn khi có |
| Sửa chữ | `text-cursor-input` | hover khối |
| Neo | `anchor` | luôn |
| Phím tắt | `keyboard` | trong tooltip |
| Sáng / tối | `sun` / `moon` | luôn |
| Phương án layout (chỉ prototype) | `layout-grid` | luôn |

**Quy tắc hiện icon.**
1. Nút chỉ có icon phải có `aria-label` và tooltip "Tên · phím tắt".
2. Thao tác chính của vùng (Đặt, chuyển ‹ ›, mở dropdown) **luôn hiện**, gồm icon và chữ khi có chỗ.
3. Thao tác phụ trên từng dòng hoặc từng cột (ghim, xoá, kéo, thử) **hiện khi hover hoặc focus**. Trạng thái đã bật (đã ghim, đã sao) luôn hiện.
4. Chip và tag luôn có icon bên trái, cỡ 14 px, chữ ≥ 11 px.
5. Không dùng icon trang trí; mỗi icon gắn với một hành động hoặc một trạng thái.

## 8. Tổ chức tuỳ chọn và bộ lọc

**Ba tầng, trong mọi phương án:**
1. **Luôn hiện** trên thanh trên, gom thành các cụm Stepper:
   - nhóm Case: tên, sao;
   - nhóm Trang: Bố cục, Combo;
   - nhóm Gợi ý: Chế độ, nút Lọc;
   - nhóm Xem: So sánh, sáng/tối.
2. **Popover Lọc** (`sliders-horizontal`, phím `F`). Bốn nhóm, mỗi nhóm một dòng:
   - Nguồn: chip Ghép / Giống / Cùng họ / Thư viện / Ghim;
   - Kiểu ghép: Stepper;
   - Ràng buộc: Tiếng Việt, Cân x-height, Chỉ font có italic thật, Có ≥ 4 độ đậm;
   - **Nâng cao** (đóng mặc định): thang cỡ, khoảng dòng, trọng số nhóm trục.
3. **Chip lọc đang bật** hiện ngay dưới thanh trên (A, C) hoặc trên dãy bản (B), có `x` để gỡ. Khi không lọc gì thì dòng này không có.

| Phương án | Nơi chọn nguồn gợi ý | Nơi chọn kiểu ghép / chế độ |
|---|---|---|
| A | Tab icon trong dropdown font của thanh nổi | Thanh trên (Stepper) + popover Lọc |
| B | Thanh trên dãy bản | Thanh trên |
| C | Đầu cuộn chọn (tab icon) | Thanh trên + đầu cuộn chọn |

## 9. Thay đổi so với spec 04 kéo theo

- **Độ sâu 0/1/2 bỏ khỏi thanh trên.** Độ sâu 2 (dumbbell, chồng dấu) chuyển vào **So sánh**. Độ sâu 0/1 thành công tắc "Nhãn trục ●◐◆○" trong Lọc. Phím `1 2 3` để trống cho sau.
- **Khay (tray) của v1 không còn là vùng cố định.** Nguồn gợi ý nằm trong danh sách thả xuống (A), cuộn chọn (C) hoặc thanh trên dãy bản (B).
- **Combo** thay "Nhân bản Plate". Dữ liệu: bảng `case_combos (case_id, idx, name, voices JSON, role_voice JSON, is_main)` thay cho `plate_voices`. Mọi bảng `plate_*` ở `04_spec.md` §8.1 đổi tên thành `case_*`.
- **Phím mới:** `[` `]` chuyển combo · `M` xoay chế độ · `C` So sánh · `F` Lọc · `←` `→` thử ứng viên (A) · `↑` `↓` đổi vai · `⌘⇧S` lưu combo · `⌥←` `⌥→` đổi chỗ cột trong So sánh.

## 10. Prototype — đã kiểm

- File: `docs/design/prototype/pairing-desk-v2.html`. Mở bằng trình duyệt; có tham số URL để mở thẳng trạng thái (ghi ở đầu file).
- **Console:** 0 lỗi.
- **Kịch bản tự động** chạy hết, không lỗi:
  - A: thử ‹ › ×2, đặt; danh sách font (hover thử, đổi nguồn, đặt); phím `↓↓ Enter`, `→`, `Esc`.
  - Combo: ‹ ›, dropdown, hover thử combo, thêm từ gợi ý, `[` `]`.
  - Bố cục ×5; chế độ `M`; Lọc (kiểu ghép, công tắc, chip gỡ được).
  - Chỉnh độ đậm/cỡ/hoa rồi hoàn tác.
  - So sánh: thêm cột, sắp theo tương phản ↑↓, ghim, kéo cột, kéo hàng, `⌥→`, bỏ cột, Song bản, `Esc`.
  - B: lật nhóm, đặt, đổi vai.
  - C: chọn vai, lăn chuột thử, `↓ Enter`.
  - Sáng/tối; sao; home; Case mới từ font thiếu tiếng Việt (nguồn Ghép rỗng và gợi ý Giống thay thế); menu ngữ cảnh "Thêm vào Case ▸".
- **Ảnh** (`shots-v2/`, Chrome headless, 15 ảnh ở 1400×920 và 1100×800), gồm:
  - A editorial / poster / UI-dark, danh sách font, combo, Lọc;
  - B song bản + thông số;
  - C cột vai + cuộn chọn;
  - So sánh sáng / tối / song bản;
  - home.
- **Lỗi thấy trên ảnh và đã sửa:**
  - danh sách font che chính khối đang sửa;
  - trang bị cắt khi vùng hẹp (nay tự co vừa);
  - nút trên dòng ứng viên chiếm chỗ tên font;
  - stepper chỉ có icon vẫn rộng ở 1100 px;
  - kéo cột sang phải sang ô kế bên không đổi chỗ.
- **Chưa kiểm:**
  - kéo thả bằng chuột thật (chỉ giả lập sự kiện DnD);
  - trình đọc màn hình;
  - ứng dụng native.

## 11. Câu hỏi cho chủ app

1. Chọn phương án nào làm mặc định: **A** (đề xuất), B hay C? Có muốn giữ B như nút "Song bản" bên trong So sánh không?
2. Combo nằm **trong** Case (đề xuất), hay mỗi combo là một Case riêng?
3. Option Stepper có thay **mọi** nhóm tab (bố cục, chế độ, kiểu ghép) không, hay giữ tab ngang cho bố cục vì chỉ có 5 mục?
4. Trong So sánh, cột đầu luôn là mốc cố định không kéo được (đề xuất), hay cho kéo như các cột khác?
5. Tên trong UI tiếng Việt: giữ "Pairing Desk", "Case", "Combo" bằng tiếng Anh?
