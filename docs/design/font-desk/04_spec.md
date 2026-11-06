# 04 — Spec: Plates

- **Ngày:** 2026-11-06 · **Trạng thái:** đề xuất, chờ chủ app duyệt · **Prototype:** `docs/design/prototype/font-desk-v1.html`
- **Hướng:** lai A + B ở `03_concepts.md`: **Trang + Sổ giọng + Khay**. Thuật ngữ theo `02_naming.md` §4.
- **Ký hiệu ưu tiên:** **MVP** (bản đầu của tính năng) · **P1** (nên có ngay sau) · **Sau**.

---

## 0. Nguyên tắc

1. **Trang là nhân vật chính.** Chữ của người dùng chiếm phần lớn diện tích. Giao diện lùi về lề.
2. **Luôn có gợi ý, người dùng quyết định.** Khay không bao giờ rỗng khi còn font trong thư viện; không có nút "tự chọn hộ"; không điểm, không %, không "khuyên dùng".
3. **Thử trước, đặt sau.** Mọi thay đổi font qua khay đều xem được trên trang trước khi ghi.
4. **Nghĩ theo giọng.** Tối đa 3 giọng mặc định; vai thừa hưởng giọng.
5. **Tiếng Việt trên chữ thật.** Kiểm phủ ký tự bằng chính văn bản của Plate.
6. **Tự lưu, không có nút Lưu.**
7. **Không gác cổng.** Font thiếu tiếng Việt, va chạm hay thiếu dữ liệu vẫn đặt được; giao diện nói rõ hậu quả.

## 1. Kiến trúc thông tin

### 1.1 Vị trí trong sidebar

```
＋ Thêm font
▦ Tất cả            13
☆ Yêu thích          0
◷ Gần đây
⏻ Đang bật           0

▾ Plates          ＋          ← mục mới, nằm ngay dưới nhóm thư viện
   ▤ Phở Lý Quốc Sư           (chỉ Plate có sao, tối đa 7, kéo để sắp)
   ▤ Nhãn nước mắm Cửa Hội
   Tất cả Plate · 12          ← mở Plates home
▾ Dự án
▾ Nguồn
▾ Thẻ
```

- Tiêu đề nhóm "Plates" bấm được: mở **Plates home** (zerostate + danh sách).
- `＋` cạnh tiêu đề (chỉ hiện khi rê chuột, theo kiểu sidebar Claude): Plate mới. Nếu đang chọn một font trong lưới thì neo là font đó.
- Mỗi dòng Plate: icon trang, tên, nút `…` khi rê chuột (Đổi tên · Nhân bản · Bỏ sao · Xoá). Plate đang mở có nền chọn như "Tất cả".
- Không có Plate có sao: nhóm chỉ còn dòng "Tất cả Plate · N"; nếu N = 0, dòng ghi "Plate mới".
- Khi mở Plate, **toàn bộ phần phải** (lưới + panel chi tiết) nhường cho Plate. Sidebar giữ nguyên. Quay về thư viện bằng mục sidebar bất kỳ hoặc `⌘[`.

### 1.2 Điểm vào

| Từ | Thao tác | Kết quả | Ưu tiên |
|---|---|---|---|
| Sidebar | `＋` cạnh "Plates" | Plate mới; neo = font đang chọn, nếu không có thì mở Plates home với ô "Bắt đầu từ font nào?" | MVP |
| Thẻ font (lưới) | Menu ngữ cảnh → **Plate mới từ font này** | Plate mới, neo ở Giọng chính | MVP |
| Thẻ font (lưới) | Menu ngữ cảnh → **Thêm vào Plate ▸** [5 Plate gần nhất…] | Font vào **khay** của Plate đó (nguồn "Đã ghim"), không đổi trang | MVP |
| Thẻ font (lưới) | Kéo thẻ lên một dòng Plate trong sidebar | Như "Thêm vào Plate" | P1 |
| Panel phải (hồ sơ font) | Nút ▤ ở đầu panel | Plate mới từ font này | MVP |
| Tab Ghép cặp (PairTab) | "Mở trong Plate" dưới ứng viên | Plate mới: neo + ứng viên đang xem, đúng vai và chế độ | MVP |
| Similar | Menu kết quả → Thêm vào Plate ▸ | Như thẻ | P1 |
| Phím | `⇧P` trên font đang chọn | Plate mới từ font này | MVP |

`P` giữ nguyên nghĩa "Ghép cặp" (tab trong panel) theo `03_ux_flows.md` §7.3.

### 1.3 Plates home (zerostate)

```
┌──────────────────────────────────────────────────────────────────────┐
│  Plates                                                              │
│  Atlas là tập các plate. Mỗi Plate là một trang chữ bạn dựng quanh   │
│  một font.                                                           │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐    │
│  │  Bắt đầu từ font nào?   [ Tìm trong thư viện…            ⌘K ]│    │
│  │  hoặc kéo một font vào đây                                    │    │
│  │  Gần đây:  [Playfair Display] [Be Vietnam Pro] [Oswald] …    │    │
│  │  Bố cục:   (•) Menu  ( ) Bảng hiệu  ( ) Bao bì  ( ) Brand  ( ) Editorial │
│  └──────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  Có sao                                         Gần đây ▾  ▦ ☰        │
│  ┌────────────┐ ┌────────────┐                                       │
│  │ PHỞ LÝ     │ │ NƯỚC MẮM   │   (thẻ: ảnh thu nhỏ trang thật,       │
│  │ Phở bò…    │ │ Cửa Hội    │    tên, bố cục, 2–3 tên font, sửa lúc)│
│  │ ★ Menu     │ │ ★ Bao bì   │                                       │
│  └────────────┘ └────────────┘                                       │
│  Tất cả (12)  …                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

- Nhóm "Bắt đầu" chỉ hỏi **một câu**. Bố cục mặc định là bố cục dùng lần trước (lần đầu: Menu).
- Danh sách: "Có sao" trước, rồi "Tất cả" sắp theo lần mở gần nhất (đổi được: tên, ngày tạo). Thẻ hiện ảnh thu nhỏ chính trang đó (render lúc lưu), không phải icon.
- **Chưa có Plate nào:** chỉ hiện nhóm "Bắt đầu" và ba neo gợi ý lấy từ font yêu thích hoặc font mở gần nhất của người dùng. Không có ảnh minh hoạ trang trí.
- **Thư viện rỗng:** "Thêm font trước khi dựng Plate" + nút Thêm thư mục (luồng onboarding có sẵn).
- Thùng rác: Plate xoá vào "Đã xoá" ở cuối danh sách, giữ 30 ngày, khôi phục được.

## 2. Màn làm việc

### 2.1 Bố cục màn hình (phần phải, ≥ 1100 px tổng chiều rộng cửa sổ)

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ THANH TRÊN (48)                                                               │
│ Phở Lý Quốc Sư ☆   Menu · Bảng hiệu · Bao bì · Brand sheet · Editorial         │
│                    ⓥ Tiếng Việt  ◑ x-height  ○●○ độ sâu   ⋯   Đã lưu          │
├──────────────────────────────────────────────────────────┬────────────────────┤
│ TRANG (nền phẳng #FFFFFF, không kính)                    │ SỔ GIỌNG (264;     │
│                                                          │ 220 khi < 1280)    │
│  khổ trang theo bố cục, co theo chiều rộng, giữa vùng    │                    │
│  vai đang chọn có viền chấm amber                         │ ① Giọng chính      │
│  vai rê chuột: nhãn vai nhỏ ở lề trái                     │   Playfair Display │
│                                                          │   ⚓ neo · Bold     │
│                                                          │   Tên quán · Mục   │
│                                                          │ ② Giọng đọc        │
│                                                          │   Be Vietnam Pro   │
│                                                          │   ● x-h ◐ chân …   │
│                                                          │   Tên món · Mô tả  │
│                                                          │   · Giá · Ghi chú  │
│                                                          │ ＋ Giọng nhấn       │
│                                                          │ ─────────          │
│                                                          │ Tiếng Việt         │
│                                                          │ ● đủ cho chữ trang │
├──────────────────────────────────────────────────────────┴────────────────────┤
│ KHAY (232; thu còn 40 bằng T) — cho: ② Giọng đọc                              │
│ Ghép · Giống · Cùng họ · Thư viện · Đã ghim (2)     Kiểu ghép: [Tất cả ▾]     │
│                                    Chế độ: An toàn | Có kiểm soát | Thử nghiệm │
│ ┌────────────┐┌────────────┐┌────────────┐┌────────────┐┌──────┐               │
│ │ thẻ ứng viên (chữ của vai đang chọn, nhãn trục, Thử · Đặt · Ghim)      │ ‹ › │
│ └────────────┘└────────────┘└────────────┘└────────────┘└──────┘               │
└───────────────────────────────────────────────────────────────────────────────┘
```

**Kích thước và token.** Giữ token app: nền khung `#FAFAFA`, chữ `#2A2A2A`, nhấn `#D4A574`, mực nâu `#9A6B3C` cho chữ nhấn trên nền sáng (AA), viền `#E7E5E4`, muted `#78716C`; dark stone `#1C1917` / `#23201E`. Font UI Inter (đã đóng gói). Chữ UI ≥ 11 px. Trang luôn nền trắng phẳng, kể cả dark mode (spec B1: specimen trung tính), viền trang 1 px `#E7E5E4`, không bóng đổ đậm.

**Thanh trên.**
- Tên Plate: sửa tại chỗ (bấm). Mặc định "Plate mới · <tên neo>"; khi người dùng sửa chữ "Tên quán" trên trang lần đầu, đề xuất đổi tên Plate theo đó (một dòng gợi ý, bấm mới đổi).
- ☆: đánh sao, phản ánh ngay ở sidebar.
- Bố cục: tab ngang hàng. Đổi bố cục **giữ giọng**, ánh xạ vai sang bố cục mới (§3.3).
- ⓥ "Tiếng Việt": công tắc `project_vi`, mặc định bật (giống PairTab).
- ◑ "Cân x-height": công tắc; bật thì các giọng hiển thị cỡ sao cho x-height bằng nhau ở cùng cấp (spec B5).
- Độ sâu 0/1/2 (`1` `2` `3`), giống S2/S4.
- "Đã lưu" / "Đang lưu…" / "Chưa lưu được · Thử lại" ở góc phải.
- `⋯`: Nhân bản · Xuất PNG/PDF (P1) · Lịch sử · Xoá.

### 2.2 Trang

- Render bằng font thật của người dùng qua `FontFace` (đã có cơ chế cho panel chi tiết). Mỗi vai là một khối có `data-role`.
- **Sửa chữ tại chỗ**: bấm đúp vào khối để sửa; Enter xuống dòng ở khối nhiều dòng, `Esc` thoát. Chữ đã sửa lưu theo Plate, không đổi chữ mẫu toàn cục.
- **Chọn vai**: bấm một khối → khay chuyển sang **giọng của vai đó**; sổ giọng tô dòng vai. Vai đang chọn có viền chấm 1 px amber cách chữ 6 px.
- **Thử**: khi rê chuột (hoặc focus) lên nút "Thử" của một thẻ trong khay, hoặc giữ `Space`, mọi vai thuộc giọng đó đổi sang ứng viên. Có dải 2 px amber ở mép trái các vai đang được thử và dòng trạng thái "Đang thử Lora — nhả để quay lại". Thả ra là về như cũ. Chuyển động: đổi font tức thì, không mờ dần (tránh nhìn nhầm).
- **Độ sâu 0**: chỉ trang. **Độ sâu 1** (mặc định): nhãn trục trong sổ giọng và trên thẻ khay. **Độ sâu 2**: thẻ khay mở rộng thành bảng trục (dumbbell), thêm hàng chồng dấu Việt của hai font; trang không đổi.
- **Khổ trang**: Menu A4 dọc; Bảng hiệu 3:1 ngang; Bao bì nhãn 4:3; Brand sheet A4 ngang; Editorial A4 dọc. Trang co theo vùng, không cuộn ngang; cuộn dọc khi cao hơn vùng.

### 2.3 Sổ giọng (cột phải)

Mỗi giọng là một khối:
```
① Giọng chính                      ⋯
Playfair Display                   (tên family, bấm mở hồ sơ font)
⚓ neo · Bold 700 · 9 kiểu          (dòng meta nhỏ)
● x-height  ◐ chân  ● nhịp          (độ sâu ≥ 1: nhãn so với giọng chính; giọng chính thì không có)
Tên quán · Mục · Eyebrow           (vai thuộc giọng; bấm để chọn vai trên trang)
```
- `⋯` của giọng: Đổi kiểu (weight/style) · Mở hồ sơ · Tìm giống · Đổi neo sang giọng này · Bỏ giọng (vai chuyển về giọng còn lại).
- Kéo một vai từ giọng này sang giọng khác để đổi giọng của vai. Menu vai: "Tách thành giọng mới".
- **＋ Giọng nhấn**: thêm giọng thứ ba (dùng cho số/giá, script trang trí, eyebrow). Khi đã có 3 giọng, nút đổi thành "＋ Giọng" và có dòng nhỏ: "Trên 3 font, trang dễ rời rạc." Không chặn (09 D2).
- **Khối Tiếng Việt** ở đáy: "● Đủ cho chữ trang này" hoặc "◆ Be Vietnam Pro thiếu: Ữ ặ — ở vai Tên món". Bấm → tô chữ thiếu trên trang (gạch chân đỏ đất `#C2410C` + chấm ◆ ở lề, không chỉ dùng màu).
- Nguồn phân tích: dòng meta nhỏ "Số đo" / "Số đo + Hình ảnh" theo font (quyết định 15/10).

### 2.4 Khay (bàn so sánh)

**Ngữ cảnh.** Khay luôn phục vụ **một giọng** (giọng của vai đang chọn). Tiêu đề: "Cho ② Giọng đọc · so với ① Playfair Display".

**Nguồn (tab ngang hàng):**

| Nguồn | Nội dung | Lấy từ | Ưu tiên |
|---|---|---|---|
| **Ghép** | Font ghép được với giọng neo, theo chế độ và kiểu ghép | `pair_candidates` (vai neo ↔ vai giọng này) | MVP |
| **Giống** | Font giống font **đang đặt** ở giọng này (để thay thế: bản đủ tiếng Việt, bản rẻ hơn) | `similar_families` | MVP |
| **Cùng họ** | Các kiểu khác trong family hoặc superfamily của font đang đặt / của neo | `get_family_styles` + superfamily (R2) | MVP |
| **Thư viện** | Tìm/lọc toàn thư viện (ô tìm + chip phân loại); kéo thẻ lên trang hoặc sổ giọng | `query_faces` | MVP |
| **Đã ghim** | Ứng viên người dùng ghim hoặc thêm từ ngoài vào ("Thêm vào Plate") | `plate_tray` | MVP |
| Bản đồ | Không gian 2 trục quanh neo, chọn điểm → vào khay | `axis_stats` + vector | Sau |

Mặc định mở **Ghép** cho giọng ② và ③; mở **Cùng họ** cho giọng ① (giọng chính thường đổi kiểu, ít đổi font).

**Gợi ý luôn có.** Khi Ghép rỗng ở chế độ hiện tại, khay không trống: hiện dòng "Không có ứng viên An toàn cho Be Vietnam Pro" kèm hai nút "Thử Có kiểm soát" và "Xem Giống", **đồng thời** tự điền 4 thẻ từ nguồn Giống bên dưới, ghi rõ nguồn trên từng thẻ.

**Thẻ ứng viên (rộng 220, cao 184 ở độ sâu 1):**
```
┌─────────────────────────────┐
│ Lora                    ⚑ ⋯ │  tên family · ghim · menu
│ Phở bò tái lăn              │  chữ của vai đang chọn, cỡ cố định 22
│ Nước dùng ninh xương 12…    │  dòng thứ hai: vai đọc khác của giọng (nếu có)
│ 65.000đ                     │  số/giá nếu giọng có vai Giá
│ ● x-height  ◐ chân  ● nhịp  │  ≤ 3 nhãn, ký hiệu + chữ
│ Giữ x-height, đổi chân.     │  câu tóm tắt (summary từ BE), 1 dòng
│ [Thử]  [Đặt]       Ghép · AT│  nguồn · chế độ qua cổng
└─────────────────────────────┘
```
- **Thử**: giữ chuột/hover/`Space` → trang đổi tạm (§2.2).
- **Đặt**: ghi font vào giọng. Font cũ tự vào "Đã ghim" (để quay lại được), kèm dấu "đã dùng trước".
- **Ghim** ⚑: giữ thẻ ở "Đã ghim". Ghi chú một dòng tuỳ chọn ("khách chê nặng").
- **Thiếu tiếng Việt cho chữ trang**: thẻ vẫn hiện; nhãn "◆ thiếu Ữ ặ" lên đầu danh sách nhãn; nút Đặt vẫn bấm được.
- **Thứ tự**: giữ nguyên thứ tự từ điển của backend (spec 6.3), không sắp lại theo điểm. Hiện "3 / 12".
- **Duyệt**: `←` `→` đổi thẻ đang focus; `Enter` = Đặt; `Space` giữ = Thử; `G` = Ghim.
- **Độ sâu 2**: thẻ rộng 320, thêm bảng trục: mỗi trục một dòng dumbbell (hai chấm: giọng neo, ứng viên; thang là phân vị trong thư viện người dùng), nhãn, số thật ("0,51 vs 0,53"); hàng chồng dấu `Ữ Ặ Ệ Ỗ ươ` của hai font chồng lên nhau; dòng ràng buộc (body warnings, heading overflow).
- **Giọng nhấn (③)**: thẻ có hai hàng nhãn — so với ① và so với ② (gọi `assess_pair` hai lần).

**Kiểu ghép** (bộ lọc trên nguồn Ghép, chọn một, mặc định "Tất cả"): xem §4.

### 2.5 Kéo thả và menu ngữ cảnh trong Plate

| Kéo từ | Thả vào | Kết quả |
|---|---|---|
| Thẻ khay | Một vai trên trang | Đặt font cho **giọng của vai đó** (hỏi một lần nếu muốn chỉ vai đó: "Chỉ vai này (tách giọng mới)") |
| Thẻ khay | Một giọng trong sổ giọng | Đặt font cho giọng |
| Thẻ khay | Vùng "＋ Giọng nhấn" | Tạo giọng mới với font đó |
| Thẻ thư viện (trong khay) | Như trên | Như trên |
| Vai trong sổ giọng | Giọng khác | Đổi giọng của vai |

Menu ngữ cảnh trên vai (trang): Chọn font khác… (mở khay) · Đổi cỡ/độ đậm/hoa-thường · Chuyển sang giọng ▸ · Tách thành giọng mới · Khôi phục chữ mẫu · Ẩn vai.

Mọi thao tác kéo có phương án bàn phím tương đương (menu ngữ cảnh, `⇧F10`).

## 3. Giọng, vai và bố cục

### 3.1 Mô hình hai lớp

- **Giọng** = một face cụ thể (family + kiểu). Có 1–3 giọng (cho phép thêm, cảnh báo nhẹ trên 3).
- **Vai** = một khối chữ trên trang, có `voice`, `size_step`, `weight` (trong family của giọng), `case` (thường/HOA/Hoa đầu), `tracking`, `leading`.
- Cỡ chữ đi theo **thang tỉ lệ** (09 D6): cỡ nền = cỡ đọc của bố cục, tỉ lệ mặc định 1,25 (Menu, Bao bì), 1,333 (Editorial, Brand), 1,5 (Bảng hiệu). Người dùng chỉnh tỉ lệ ở `⋯` → Thang cỡ (P1).
- **Khoảng dòng tiếng Việt** (09 H8, số là [SL]): đọc 1,5; tiêu đề ≥ 1,2; chữ HOA có dấu chồng ≥ 1,25. Khi font có `stacked_marks_overflow` hoặc `vi_mark_top` vượt ascender, tăng leading của vai HOA và báo trong sổ giọng.

### 3.2 Danh mục vai

Căn cứ: thứ tự vai trong brand guideline (eyebrow/label → heading → body → CTA; eyebrow chữ HOA, khoảng chữ rộng, thường dùng font phụ) [TT, 01 §5]; menu ba tầng mục / tên món / mô tả, giá cùng cỡ mô tả [TT]; 09 D1 (mỗi font một vai nhất quán), D7 (tiêu đề biểu cảm, đọc trung tính), G1 (optical size), H3 (chữ HOA có dấu).

| Mã vai | Tên UI | Giọng mặc định | Bậc cỡ | Kiểu mặc định | Kiểm gì |
|---|---|---|---|---|---|
| `wordmark` | Tên quán / Logo | ① | +4…+6 | HOA hoặc Hoa đầu, Bold | Dấu chồng chữ HOA, va dòng |
| `eyebrow` | Eyebrow / Meta | ② (hoặc ③) | −1 | HOA, tracking +0,12 em, Medium | Khoảng chữ rộng, dấu nhỏ ở cỡ nhỏ |
| `headline` | Tiêu đề | ① | +3 | Hoa đầu | Cá tính, nhịp với đọc |
| `deck` | Phụ đề / Sapo | ① hoặc ② | +1 | Italic hoặc Regular | Chuyển tiếp giữa hai giọng |
| `section` | Mục (MÓN NƯỚC) | ① | +1 | HOA, tracking +0,06 | Thứ bậc so với tên món |
| `item` | Tên món | ② | +0,5 | Semibold | Độ đậm đủ khác mô tả (09 D8) |
| `body` | Đoạn đọc / Mô tả | ② | 0 | Regular, leading 1,5 | x-height, độ mở, độ xám (E1) |
| `price` | Giá / Số | ② (tách ③ được) | 0 | Tabular nếu có, Semibold | Hình số, `đ` và `₫`, chữ số thẳng hàng |
| `caption` | Chú thích / Ghi chú | ② | −1 | Regular hoặc Italic | Đọc ở cỡ nhỏ (G4) |
| `cta` | Nút / Lời gọi | ② | 0 | HOA nhỏ hoặc Semibold | Nhận ra như hành động |
| `fine` | Chữ nhỏ pháp lý | ② | −2 | Regular | Dấu ở cỡ rất nhỏ (H4) |
| `quote` | Trích dẫn | ① | +2 | Italic | Italic thật hay giả lập (G2) |
| `numeral_big` | Số lớn (năm, số nhà) | ① | +3 | Bold | Hình số ở cỡ lớn |

### 3.3 Bố cục (mode theo use case)

Mỗi bố cục là một **dữ liệu JSON có phiên bản** (danh sách vai, chữ mẫu, khổ), không hard-code trong component. Nội dung mẫu tự viết, không trích văn bản có bản quyền.

| Bố cục | Vai có mặt | Chữ mẫu (trích) | Phép thử chính |
|---|---|---|---|
| **Menu F&B** (MVP) | eyebrow, wordmark, section, item, body, price, caption, fine | "THỰC ĐƠN · MÙA THU 2026" · "PHỞ LÝ QUỐC SƯ" · "MÓN NƯỚC" · "Phở bò tái lăn — 65.000đ" · "Nước dùng ninh xương 12 tiếng, bánh phở tươi tráng trong ngày." · "Bún chả Hữu Tiệp — 70.000đ" · "Giá đã gồm VAT. Gọi thêm quẩy: 10.000đ" | Ba tầng thứ bậc, giá, chữ HOA có dấu, 2 kiểu đặt giá (căn phải có dấu chấm dẫn / cuối mô tả) |
| **Bảng hiệu / Logo** (MVP) | wordmark, deck (tagline), numeral_big, caption | "BÚN CHẢ HỮU" · "Từ 1979 · Hàng Mành" · "36" · "Mở cửa 6:00 – 22:00 · 0912 345 678" | Chữ HOA cỡ lớn, dấu Ữ, số nhà, xem ở 3 cỡ (gần/vừa/xa: thu nhỏ 100/40/15%) |
| **Bao bì / Nhãn** (MVP) | wordmark, item (tên sản phẩm), eyebrow (vị), body (thành phần), fine (pháp lý), price (khối lượng) | "NƯỚC MẮM CỬA HỘI" · "Cá cơm ủ chượp 18 tháng" · "ĐỘ ĐẠM 40°N" · "Thành phần: cá cơm, muối biển." · "NSX 06/11/2026 · HSD 24 tháng · Khối lượng tịnh 500 ml" | Chữ nhỏ có dấu, số ngày tháng, đơn vị |
| **Brand sheet** (P1) | wordmark, headline, deck, body, eyebrow, numeral_big, caption + bảng thang cỡ, bảng Aa của từng giọng | "Gánh Hàng Rong — hệ chữ" · đoạn mô tả giọng thương hiệu · "Aa Ăă Ââ Đđ Êê Ôô Ơơ Ưư 0123456789 ₫" | Đủ hệ thống, thang cỡ, glyph đặc thù Việt |
| **Editorial / Poster** (P1) | eyebrow, headline, deck, body, quote, caption | "BẾP NHÀ" · "Mùa nước nổi ở Châu Đốc" · "Ba thế hệ giữ nghề mắm cá linh" · đoạn 120 chữ · trích dẫn · chú thích ảnh | Nhịp đoạn dài, italic thật, độ xám trang |

**Đổi bố cục giữ giọng.** Vai có mặt ở cả hai bố cục giữ font và kiểu người dùng đã chỉnh. Vai mới lấy giọng mặc định trong bảng §3.2. Chữ đã sửa giữ theo từng bố cục.

## 4. Kiểu ghép (style pair) và nhãn trục

Nhãn trục giữ nguyên hệ hiện có: **● hoà hợp · ◐ tương phản · ◆ va chạm · ○ chưa chắc · ◑ gợi ý độ đậm** (`PairLabelDto.symbol`). Kiểu ghép là **bộ lọc có tên** đặt trên kết quả `pair_candidates`, không phải điểm.

| Kiểu ghép | Ý nghề (nguồn) | Điều kiện trên dữ liệu hiện có | Chế độ thường gặp | Câu mô tả trong UI |
|---|---|---|---|---|
| **Cùng họ** | Superfamily / cùng nhà thiết kế (09 C2, C3) | R2 superfamily = đúng, hoặc cùng family khác kiểu | An toàn (luôn qua cổng) | "Cùng gốc thiết kế: tỉ lệ và nhịp sẵn khớp." |
| **Có chân + không chân** | Lối tắt khi phân vân (09 D4) | Trục `serif` = ◐ (không tính `C~` của R3) và không có ◆ cấu trúc | An toàn / Có kiểm soát | "Khác chân, giữ khung: khác biệt đọc được từ xa." |
| **Trưng bày + đọc** | Tiêu đề biểu cảm, nội dung trung tính (09 D7) | Neo thuộc display/script hoặc contrast cực; ứng viên qua ràng buộc vai nội dung (spec 6.3, R9) | Có kiểm soát | "Tiêu đề nhiều cá tính, chữ đọc lùi lại." |
| **Giữ một, đổi một** | H&FJ (09 D5) | ≥ 1 trục cấu trúc ● và ≥ 1 trục biểu cảm ◐ rõ, 0 ◆ | Có kiểm soát | "Giữ: {axes_same}. Đổi: {axes_diff}." (mẫu câu 09 D5) |
| **Một giọng nhiều độ đậm** | Concord thuần (09 D3, D8) | Cùng family; gợi ý kiểu theo `weightHint` (R5) | An toàn | "Một family, phân cấp bằng độ đậm." |
| **Phá cách** | Thử nghiệm có ý thức (07 §3.5) | Chế độ Thử nghiệm; hiện mọi ◆ và lý do (R7, R8) | Thử nghiệm | "Phá luật có chủ đích: xem các chỗ va chạm bên dưới." |

- **Chế độ** (An toàn / Có kiểm soát / Thử nghiệm) vẫn là tab ngang hàng trong khay, như PairTab. Kiểu ghép thu hẹp thêm trong chế độ đó. Chọn kiểu ghép không hợp chế độ (ví dụ "Phá cách" khi đang An toàn) thì chế độ tự chuyển và có dòng báo.
- Bộ lọc chạy **phía client** trên tối đa 100 ứng viên ở MVP (dùng `labels`, `gate`, `row.classification`); nếu kết quả < 4 thì gợi ý nới. Đề nghị BE thêm tham số `style` sau (§8).
- Không đặt nhãn "khuyên dùng" cho kiểu nào. Thứ tự kiểu trong menu cố định như bảng.

## 5. Luồng chính

**F1 — Bắt đầu từ một font (từ thư viện).** Chuột phải thẻ Playfair Display → "Plate mới từ font này" → Plate mở, bố cục Menu, ① = Playfair (neo), ② = để trống nhưng **trang không trống**: vai của ② tạm dùng ứng viên đầu tiên ở nguồn Ghép, có nhãn "gợi ý tạm · chưa đặt" trong sổ giọng; khay mở ở giọng ②. Người dùng duyệt ← →, Thử, Đặt. *Lý do:* trang luôn có chữ thật, người dùng không phải quyết trước khi thấy (I3), nhưng font chưa thành của Plate cho tới khi Đặt.

**F2 — Bắt đầu từ zerostate.** Plates home → gõ "playf" trong ô "Bắt đầu từ font nào?" → chọn → như F1. Chọn bố cục ngay dưới ô tìm.

**F3 — Bắt đầu từ một family.** Chọn neo ở mức family: ① dùng kiểu Regular (hoặc kiểu gần 400 nhất); nguồn "Cùng họ" liệt kê mọi kiểu để chọn cho từng vai (Bold cho tên quán…).

**F4 — Thêm từ thư viện khi đang ở thư viện.** Đang duyệt lưới, thấy Lora → chuột phải → "Thêm vào Plate ▸ Phở Lý Quốc Sư" → toast "Đã ghim Lora vào khay Phở Lý · Mở". Plate không đổi trang.

**F5 — Thêm từ thư viện khi đang trong Plate.** Khay → tab Thư viện → tìm → kéo thẻ lên vai "Tên món" hoặc lên giọng ②. Hoặc chọn thẻ, `Enter` = Đặt cho giọng đang chọn.

**F6 — So song song rồi mới đặt.** Ghim 3 ứng viên → tab Đã ghim → độ sâu 2 để xem bảng trục cạnh nhau → giữ `Space` trên từng thẻ để thử trên trang → Đặt một cái.

**F7 — Đổi bố cục.** Đang Menu, bấm "Bao bì" → trang đổi, giọng giữ; vai "fine" mới thuộc ②; nếu ② thiếu ký tự cho "Khối lượng tịnh" (ví dụ `ị`), khối Tiếng Việt báo ngay.

**F8 — Tách giọng cho số.** Chuột phải vai Giá → "Tách thành giọng mới" → ③ tạo với font hiện tại của ② → khay mở cho ③ với kiểu ghép "Tất cả" → chọn font có số tabular.

**F9 — Lưu, sao, mở lại.** Mọi thao tác tự lưu (debounce 600 ms, ghi sau mỗi Đặt/Ghim/đổi bố cục ngay lập tức). ☆ đưa Plate lên sidebar. Mở lại ở đúng bố cục, giọng, vai đang chọn, tab khay, độ sâu.

**F10 — Hoàn tác và lịch sử.** `⌘Z` / `⇧⌘Z` trong phiên. `⋯ → Lịch sử` (P1): danh sách mốc tự động (mỗi lần Đặt / đổi bố cục), khôi phục được một mốc thành bản sao.

**F11 — Nhân bản để so phương án** (P1). `⋯ → Nhân bản` → "Phở Lý Quốc Sư · B". Sau: màn "So Plate" đặt 2–3 Plate cạnh nhau cùng bố cục (từ hướng C).

## 6. Trạng thái

| Trạng thái | Hiển thị | Hành động |
|---|---|---|
| Chưa có Plate | Plates home chỉ có nhóm "Bắt đầu" + 3 neo gợi ý từ font yêu thích/gần đây | Chọn neo |
| Thư viện rỗng | "Thêm font trước khi dựng Plate" | Thêm thư mục |
| Đang tải ứng viên | Thẻ khung xám (skeleton) đúng kích thước thẻ, không spinner giữa màn; trang giữ nguyên | — |
| Ghép rỗng ở chế độ này | Một dòng lý do + "Thử Có kiểm soát" / "Xem Giống"; khay vẫn điền 4 thẻ từ Giống | Đổi chế độ / nguồn |
| Neo thiếu tiếng Việt (dự án tiếng Việt) | BE trả rỗng cho Ghép (H1). Sổ giọng: "◆ Playfair thiếu 4 ký tự: Ữ ặ ỗ ự". Khay vẫn có Giống ("font giống mà đủ tiếng Việt" lên đầu nếu BE hỗ trợ lọc `vi_full`) | Xem ký tự thiếu · Tìm bản giống đủ tiếng Việt · Tắt Tiếng Việt |
| Ứng viên thiếu ký tự cho chữ trang | Nhãn ◆ trên thẻ + tô chữ thiếu khi Thử | Vẫn Đặt được |
| Chữ do người dùng sửa có ký tự mới thiếu | Khối Tiếng Việt cập nhật sau 300 ms | Bấm để tô |
| Trục chưa đủ dữ liệu | Nhãn ○ "chưa chắc", nét đứt trong dumbbell; câu "ước lượng" | — |
| Font của Plate không còn (nguồn xoá / ổ ngoài rút) | Giọng ghi "Không khả dụng · ổ VXData chưa gắn"; trang dùng ảnh preview Rust đã cache nếu có, nếu không thì font UI kèm gạch chéo nhẹ trên vai | Gắn ổ / Chọn font khác / Giữ nguyên |
| Lưu lỗi | "Chưa lưu được · Thử lại" ở thanh trên; giữ trạng thái trong bộ nhớ; thử lại tự động sau 5 s | Thử lại |
| Font chỉ có chữ HOA / không phải Latin | Thẻ ghi "Chỉ chữ HOA" / "Không có Latin"; vai đọc báo ◆ | Vẫn Đặt được |
| Hơn 3 giọng | Dòng nhỏ dưới sổ giọng | Không chặn |

## 7. Bàn phím và tiếp cận

| Phím | Tác dụng | Phạm vi |
|---|---|---|
| `⇧P` | Plate mới từ font đang chọn | Thư viện, hồ sơ |
| `⌘⇧N` | Plate mới (hỏi neo) | mọi màn |
| `Tab` / `⇧Tab` | Vòng focus: thanh trên → trang (từng vai) → sổ giọng → khay | Plate |
| `↑` `↓` | Chọn vai trước/sau trên trang | Plate (focus trang) |
| `←` `→` | Ứng viên trước/sau trong khay | Plate |
| `Space` (giữ) | Thử ứng viên đang focus | Plate |
| `Enter` | Đặt ứng viên đang focus | Plate |
| `G` | Ghim / bỏ ghim | Plate |
| `T` | Thu / mở khay | Plate |
| `1` `2` `3` | Độ sâu 0/1/2 | Plate |
| `⌥1`…`⌥5` | Đổi bố cục | Plate |
| `⌘D` | Đánh sao | Plate |
| `⌘Z` / `⇧⌘Z` | Hoàn tác / làm lại | Plate |
| `Esc` | Huỷ thử → bỏ chọn vai → thoát sửa chữ, theo thứ tự | Plate |

- Phím một chữ cái tắt khi đang sửa chữ trên trang.
- Trang có cấu trúc đọc được: mỗi vai là một `group` với nhãn "Tên món — Be Vietnam Pro". Thử/Đặt thông báo qua `aria-live="polite"`: "Đang thử Lora cho Giọng đọc".
- Nhãn trục luôn có ký hiệu + chữ; tương phản chữ UI ≥ 4,5:1; focus ring 2 px `#9A6B3C`.
- `prefers-reduced-motion`: bỏ trượt khay, giữ đổi tức thì.

## 8. Dữ liệu và backend

### 8.1 Bảng mới trong `user.db` (nâng schema, có migration)

```sql
CREATE TABLE plates (
  id            INTEGER PRIMARY KEY,
  name          TEXT NOT NULL,
  layout        TEXT NOT NULL,            -- 'menu' | 'sign' | 'pack' | 'brand' | 'editorial'
  layout_ver    INTEGER NOT NULL,         -- phiên bản JSON bố cục
  starred       INTEGER NOT NULL DEFAULT 0,
  star_order    INTEGER,                  -- thứ tự trong sidebar
  project_vi    INTEGER NOT NULL DEFAULT 1,
  pair_mode     TEXT NOT NULL DEFAULT 'safe',
  view_state    TEXT NOT NULL DEFAULT '{}', -- JSON: vai đang chọn, tab khay, độ sâu, khay thu gọn, cân x-height
  thumb_png     BLOB,                     -- ảnh thu nhỏ cho Plates home
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  opened_at     INTEGER NOT NULL,
  deleted_at    INTEGER                   -- thùng rác 30 ngày
);

CREATE TABLE plate_voices (
  plate_id      INTEGER NOT NULL REFERENCES plates(id) ON DELETE CASCADE,
  slot          INTEGER NOT NULL,         -- 1 = chính, 2 = đọc, 3+ = nhấn
  label         TEXT,                     -- tên người dùng đặt, null = mặc định
  hash          BLOB,                     -- khoá face (hash + face_index); null = gợi ý tạm chưa đặt
  face_index    INTEGER,
  family_name   TEXT NOT NULL,            -- giữ tên để hiện khi font không còn
  is_anchor     INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (plate_id, slot)
);

CREATE TABLE plate_roles (
  plate_id      INTEGER NOT NULL REFERENCES plates(id) ON DELETE CASCADE,
  layout        TEXT NOT NULL,
  role          TEXT NOT NULL,            -- 'wordmark' | 'item' | …
  slot          INTEGER NOT NULL,         -- giọng
  style         TEXT NOT NULL DEFAULT '{}', -- JSON: weight, italic, case, size_step, tracking, leading, hidden
  text          TEXT,                     -- chữ đã sửa; null = chữ mẫu của bố cục
  PRIMARY KEY (plate_id, layout, role)
);

CREATE TABLE plate_tray (
  plate_id      INTEGER NOT NULL REFERENCES plates(id) ON DELETE CASCADE,
  slot          INTEGER NOT NULL,
  hash          BLOB NOT NULL,
  face_index    INTEGER NOT NULL,
  origin        TEXT NOT NULL,            -- 'pinned' | 'added_from_library' | 'previous'
  note          TEXT,
  added_at      INTEGER NOT NULL,
  PRIMARY KEY (plate_id, slot, hash, face_index)
);

CREATE TABLE plate_events (               -- lịch sử nhẹ (P1)
  id            INTEGER PRIMARY KEY,
  plate_id      INTEGER NOT NULL REFERENCES plates(id) ON DELETE CASCADE,
  at            INTEGER NOT NULL,
  kind          TEXT NOT NULL,            -- 'set_voice' | 'layout' | 'split' | 'restore'
  payload       TEXT NOT NULL             -- JSON đủ để khôi phục
);
```

- Không lưu điểm hay kết quả ghép; nhãn tính lại khi mở (có `scorer_version` mới thì nhãn có thể đổi — ghi chú nhỏ "nhãn đã cập nhật theo bộ luật mới").
- Thay cho phương án tạm ở `03_ux_flows.md` §6.4 (lưu cặp vào collection): Plate thay "Lưu cặp". Nút "Lưu vào collection" trong Plate thêm mọi face của các giọng vào collection chọn (P1).

### 8.2 Lệnh backend

| Lệnh | Trạng thái | Ghi chú |
|---|---|---|
| `pair_candidates(face, opts)` | **Có** | Dùng cho nguồn Ghép; cần `opts.k` đến 100 |
| `assess_pair(a, b, opts)` | **Có** | Dùng khi người dùng tự chọn font (Thư viện, Đã ghim) và cho giọng ③ so với ① và ② |
| `similar_families(face, opts, q)` | **Có** | Nguồn Giống |
| `get_family_styles(family)` | **Có** | Nguồn Cùng họ, đổi kiểu giọng |
| `query_faces(q)` | **Có** (thư viện) | Nguồn Thư viện |
| `get_face(face)` | **Có** | Meta giọng, `viMissing` |
| `plates_list({deleted?})`, `plate_get(id)` | **Thiếu** | |
| `plate_create({anchor, layout})` → Plate | **Thiếu** | Trả cả gợi ý tạm cho ② (ứng viên đầu của `pair_candidates`) để mở nhanh |
| `plate_update(id, patch)` | **Thiếu** | Patch giọng, vai, view_state, tên, sao; idempotent, có `updated_at` để tránh ghi đè cũ |
| `plate_set_voice(id, slot, face)` | **Thiếu** | Ghi `plate_events` và đẩy font cũ vào `plate_tray` (`origin='previous'`) |
| `plate_tray_add/remove(id, slot, face, origin, note?)` | **Thiếu** | Dùng cả cho "Thêm vào Plate ▸" từ thư viện |
| `plate_star(id, on, order?)`, `plate_reorder_stars([id])` | **Thiếu** | |
| `plate_duplicate(id)`, `plate_delete(id)`, `plate_restore(id)` | **Thiếu** | Thùng rác 30 ngày, dọn khi khởi động |
| `plate_thumb_set(id, png)` | **Thiếu** | Frontend render ảnh thu nhỏ trang (canvas) rồi gửi |
| `text_coverage(face, text)` → `{missing: [char]}` | **Thiếu** | Kiểm tiếng Việt trên chữ thật của Plate; cần nhanh (< 5 ms/face, dùng cmap đã có) |
| `pair_candidates` thêm `style` và `collectionId` | **Thiếu** | `style` để lọc kiểu ghép ở BE khi > 100 ứng viên; `collectionId` đã có trong DTO nhưng `03_ux_flows` §8.4 ghi chưa chạy |
| `similar_families` thêm lọc `vi_full` | **Kiểm lại** | Để "tìm bản giống đủ tiếng Việt" lên đầu |
| `layouts()` → JSON bố cục có phiên bản | **Thiếu** (có thể chỉ là file tĩnh ở frontend trong MVP) | |

### 8.3 Frontend
- Màn mới `features/plates/` (route trong vùng phải), store riêng `usePlateStore` (zustand như `store/ui`), không dùng lại `services/desk` cũ (khác khái niệm; đề nghị gỡ "Add to Desk" khi Plate ra để tránh trùng).
- Huỷ request cũ khi đổi giọng/chế độ nhanh (lỗi audit 01: kết quả cũ ghi đè mới).
- Bộ nạp `FontFace` dùng chung ngân sách 128 MB; Plate tải tối đa 3 giọng + 8 thẻ khay đang hiện.

## 9. Phạm vi

| MVP | P1 | Sau |
|---|---|---|
| Sidebar "Plates" + sao + Plates home | Bố cục Brand sheet, Editorial | Nguồn Bản đồ trong khay |
| Bố cục Menu, Bảng hiệu, Bao bì | Nhân bản; lịch sử mốc | Màn "So Plate" (2–3 Plate cạnh nhau) |
| Sổ giọng 1–3 giọng, vai, đổi kiểu | Xuất PNG/PDF có chân trang ghi font + cỡ | Gợi ý học từ Plate của người dùng (DS-kNN, 07 §3.6) |
| Khay: Ghép, Giống, Cùng họ, Thư viện, Đã ghim; Thử/Đặt/Ghim | Thang cỡ chỉnh được | Mở Plate qua MCP (08) |
| Kiểu ghép (lọc client) + chế độ | Lưu vào collection | Chia sẻ Plate (file `.plate`) |
| Kiểm tiếng Việt trên chữ trang | Kéo thẻ lên dòng Plate ở sidebar | |
| Tự lưu, thùng rác, mở lại đúng trạng thái | Xem bảng hiệu ở 3 khoảng cách | |
| Độ sâu 0/1/2 (độ sâu 2: dumbbell + chồng dấu) | | |
| Bàn phím đầy đủ, a11y cơ bản | | |

## 10. Chỉ số thành công

Đo bằng log cục bộ (không gửi đi), chủ app đọc khi test nội bộ; và bằng 5 buổi test có kịch bản (§11).

| Chỉ số | Mục tiêu ban đầu | Vì sao |
|---|---|---|
| Thời gian từ "Plate mới" đến lần **Đặt** đầu tiên | trung vị ≤ 60 s | Gợi ý đủ tốt để quyết nhanh |
| Tỉ lệ Plate có ≥ 1 lần **Thử** trước **Đặt** | ≥ 70% | Người dùng thật sự so trước khi đặt (I3) |
| Số font của Plate được mở lại ≥ 2 lần | 2–3 | Không phình quá 3 giọng (I1) |
| Plate được mở lại ở phiên khác | ≥ 40% Plate có sao | Tự lưu và sao có giá trị (I7) |
| Đặt một font có ◆ thiếu tiếng Việt mà không bấm xem chi tiết | giảm theo thời gian | Cảnh báo có hiệu lực, không chặn |
| Câu nói của người test "tôi hiểu vì sao" (test có kịch bản) | ≥ 4/5 người | Giải thích theo trục đọc được |

## 11. Kế hoạch test (design thinking — Test)

- **Người tham gia:** 5 AD/designer Việt (3 F&B/branding, 1 bao bì, 1 editorial), dùng thư viện thật của họ (bản debug với `FONTATLAS_DATA_DIR` tạm — theo quy tắc không chạy bản release trên dữ liệu thật).
- **Kịch bản:** (1) Khách đưa font tên quán, dựng menu 1 trang tới mức gửi duyệt được. (2) Đổi sang nhãn bao bì cùng thương hiệu. (3) Tìm phương án thay cho font đọc vì khách chê "nặng". (4) Hôm sau mở lại và giải thích cho khách vì sao chọn.
- **Quan sát:** chỗ dừng lâu; có dùng Thử không; có hiểu ●◐◆○ không; có thêm quá 3 giọng không; có phát hiện chữ thiếu dấu không.
- **Prototype dùng để test sớm:** `font-desk-v1.html` cho phần luồng và bố cục (font Google làm stand-in, dữ liệu nhãn giả).

## 12. Câu hỏi mở cho chủ app

1. **Tên:** duyệt "Plate" (dự phòng "Case")? Giữ "Plate" nguyên tiếng Anh trong UI tiếng Việt?
2. **Thay "Pairing Labs" bằng "Plates"** ở sidebar, và Plate thay luôn phương án "Lưu cặp vào collection" của `03_ux_flows.md` §6.4?
3. **Gợi ý tạm khi mới tạo** (F1): trang dùng ngay ứng viên đầu cho giọng ② (ghi rõ "chưa đặt"), hay để vai ② bằng font UI trung tính cho đến khi người dùng Đặt?
4. **Bố cục MVP:** Menu + Bảng hiệu + Bao bì là đủ? Hay cần Brand sheet ngay vì là sản phẩm giao cuối của branding?
5. **Kiểu đặt giá mặc định** trên Menu: căn phải có dấu chấm dẫn hay cuối mô tả?
6. **Giọng ③** mặc định ẩn (chỉ thêm khi cần) hay luôn có ô trống "＋ Giọng nhấn"?
7. Plate cũ có nên **khoá font** khi bộ luật ghép (scorer) đổi, hay luôn tính lại nhãn với luật mới?
8. Có gỡ "Add to Desk" (code cũ `services/desk`) khi Plate ra không?
