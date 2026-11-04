# FontAtlas — Spec UX luồng người dùng (đầu vào Plan 6b)

- **Ngày:** 2026-11-04
- **Người viết:** FontAtlas · Frontend (theo yêu cầu của Lead)
- **Trạng thái:** bản nháp để Lead và chủ app duyệt. Đây **không** phải plan code và **không** phải thiết kế hình ảnh. Hình ảnh sẽ làm sau bằng Google AI Studio (`docs/design/02_ai_studio_prompt.md`).
- **Nguồn:**
  - `docs/design/01_ui_hien_tai_va_tinh_nang.md` (danh sách tính năng phần B, trạng thái B8)
  - `docs/team/research/06_pairing_ui_specimen.md` (một canvas, ba độ sâu; W1–W4)
  - spec MVP mục 5.1, 6, 7, 8, 10 và cập nhật ngày 04/11 ở mục 1.3 (kích hoạt font vào bản đầu)
  - `docs/team/plans/plan6-notes.md`, `docs/team/spikes/04_activation.md`
  - API thật của các crate trên `dev/mvp` @ 187eb67
- **Khi mâu thuẫn:** theo spec MVP. Mọi chỗ spec này đề xuất thêm mà backend chưa có đều được đánh dấu **[Thiếu BE]** và gom lại ở mục 8.

## 0. Quy ước

- **Mã:** S = màn, O = bước onboarding, D = hộp thoại/overlay, E = trạng thái model/embedding.

- **Ưu tiên:** P0 = bản dùng được đầu tiên; P1 = nên có; Sau = chỉ chừa chỗ.
- **Nút:** `[Tên nút]`. Nút chính của mỗi bước in đậm. Chuỗi trong ngoặc kép là nội dung hiển thị (tiếng Việt trước, tiếng Anh có bản dịch tương ứng trong file ngôn ngữ).
- **Phím tắt:** viết theo macOS (`⌘`). Trên Windows, `⌘` thành `Ctrl`, `⌥` thành `Alt`.
- **Không chặn người dùng:** mọi tác vụ dài (quét, tải model, phân tích hình ảnh, kích hoạt hàng loạt) chạy nền, hiện ở **thanh trạng thái** và **Trung tâm hoạt động**. Không có hộp thoại modal nào đợi tác vụ nền xong.
- **Không xếp hạng đúng/sai:** không điểm tổng, không phần trăm, không sao. Nhãn trục dùng ký hiệu cộng chữ: ● hoà hợp, ◐ tương phản, ◆ va chạm, ○ chưa chắc.
- **Từ ngữ:** không dùng "elegant"/"thanh lịch" trong nội dung giao diện. Chữ UI không nhỏ hơn 11 px.

---

## 1. Bản đồ màn hình và điều hướng

### 1.1 Khung ứng dụng

```
┌──────────────┬───────────────────────────────────────────────┬────────────────────┐
│ SIDEBAR      │ TOOLBAR: ← → · tìm kiếm (⌘K) · chữ mẫu ▾ ·    │ PANEL PHẢI         │
│              │ cỡ thẻ · lưới/danh sách · bộ lọc · độ sâu ●○○ │ (kéo rộng/hẹp,     │
│ Thư viện     ├───────────────────────────────────────────────┤  đóng bằng Esc)    │
│  Tất cả  N   │                                               │                    │
│  Yêu thích   │              VÙNG NỘI DUNG                    │ Hồ sơ font rút gọn │
│  Đang kích   │   (Thư viện / Hồ sơ / Similar / Pairing /     │ hoặc panel Phòng   │
│  hoạt    N   │    Collection / Font có vấn đề)               │ thí nghiệm         │
│ Collections  │                                               │                    │
│  + Mới       │                                               │                    │
│ Tag          │                                               │                    │
│ Nguồn        │                                               │                    │
│  ● Fonts     │                                               │                    │
│  ◌ Ổ ngoài   │                                               │                    │
│ Font có vấn  │                                               │                    │
│ đề        3  │                                               │                    │
│ ⚙ Cài đặt    │                                               │                    │
├──────────────┴───────────────────────────────────────────────┴────────────────────┤
│ THANH TRẠNG THÁI: Đang quét 1.240/3.800 · Phân tích hình ảnh 412/3.800 · ~2 phút ·│
│ ⏸ · 42 font đang kích hoạt · ⚠ 1                            [Trung tâm hoạt động] │
└───────────────────────────────────────────────────────────────────────────────────┘
```

- **Sidebar:** điều hướng cấp 1 và bộ lọc phạm vi. Mục "Đang kích hoạt" chỉ hiện khi đã bật kích hoạt font.
- **Toolbar:** đổi theo màn. Ô tìm kiếm và chữ mẫu toàn cục luôn có ở Thư viện, Collection và Similar.
- **Panel phải:** dùng chung cho hồ sơ rút gọn (khi chọn một thẻ) và panel Phòng thí nghiệm (độ sâu 2). Panel không bao giờ che vùng specimen; vùng nội dung co lại.
- **Thanh trạng thái:** luôn hiện, cao một dòng, chỉ chứa việc đang chạy và cảnh báo. Không có việc gì chạy thì chỉ hiện số font. Bấm vào mở **Trung tâm hoạt động**.

### 1.2 Danh sách màn

| Mã | Màn | Vào từ | Ra tới | P |
|---|---|---|---|---|
| S0 | Onboarding (7 bước) | Lần đầu mở app; Cài đặt → "Chạy lại hướng dẫn" | S1 | P0 |
| S1 | Thư viện (lưới / danh sách) | Sidebar "Tất cả", "Yêu thích", "Đang kích hoạt", Nguồn, Tag | S2, S3, S4, S5 | P0 |
| S2 | Hồ sơ giải phẫu một font (specimen W4) | Mở thẻ (Enter / nhấp đúp); "Mở toàn màn" từ panel phải | S3, S4, S5 | P0 |
| S3 | Similar | Nút "Tương tự" trên thẻ / hồ sơ (`S`) | S2, S4 | P0 |
| S4 | Pairing Studio (W1–W3) | Nút "Ghép cặp" trên thẻ / hồ sơ / Similar (`P`) | S5 | P0 |
| S5 | Collection | Sidebar → một collection | S1-style lưới, S4 | P0 |
| S6 | Font có vấn đề | Sidebar; cảnh báo ⚠ ở thanh trạng thái | S7 (Nguồn) | P0 |
| S7 | Cài đặt | `⌘,`; sidebar ⚙ | — | P0 |
| S8 | Tìm bằng mô tả (popover dưới ô tìm) | Gõ câu mô tả vào ô tìm, hoặc bấm "Tìm bằng mô tả" | S1 với chip lọc | P0 |
| S9 | Bảng lệnh | `⌘⇧P` | mọi màn | P1 |
| D1 | Xem nhanh (overlay) | `Space` trên thẻ | đóng bằng `Space`/`Esc` | P0 |
| D2 | Trung tâm hoạt động (popover) | Bấm thanh trạng thái | S6, S7 | P0 |
| D3 | Hộp thoại xung đột kích hoạt | Khi kích hoạt gặp trùng tên | — | P0 |
| D4 | Hộp thoại nguồn lồng nhau | Khi thêm thư mục | — | P0 |

### 1.3 Điều hướng

```
                    ┌──────── S0 Onboarding (lần đầu) ────────┐
                    ▼                                         │
S7 Cài đặt ◀── ⌘, ── S1 Thư viện ── Enter ──▶ S2 Hồ sơ ── S ──▶ S3 Similar
                    │  │  ▲                    │  ▲             │
                    │  │  └──── ← (lịch sử) ───┘  └── chọn KQ ──┘
                    │  └── P ──────────────▶ S4 Pairing ◀── P ──┘
                    │                          │
                    └── sidebar ──▶ S5 Collection ◀── "Lưu vào collection"
```

- **Lịch sử:** `⌘[` / `⌘]` và nút ← → trên toolbar đi lùi/tiến giữa các màn và trạng thái (font đang xem, ứng viên đang chọn). Mở lại app thì về màn cuối cùng, nếu font đó còn.
- **Ngữ cảnh mang theo:** chữ mẫu, bộ lọc và vai trò được giữ khi chuyển giữa S1 → S2 → S3 → S4. Ví dụ từ S3 bấm "Ghép cặp" trên một kết quả thì S4 mở với font gốc ở vai trò đã chọn lần trước và kết quả đó làm ứng viên đầu.
- **Chọn nhiều:** trong S1/S5, `⌘`-click và `⇧`-click chọn nhiều thẻ. Thanh hành động nổi hiện: "Thêm vào collection", "Gắn tag", "Yêu thích", "Kích hoạt", "Bỏ chọn".

---

## 2. Onboarding lần đầu (S0)

### 2.1 Nguyên tắc
- **Dạng:** một cửa sổ hướng dẫn chiếm toàn vùng nội dung (không phải modal chồng). Thanh tiến độ 7 chấm ở trên; nút `[Quay lại]` ở trái, nút chính ở phải.
- **Bỏ qua được mọi bước**, trừ bước Giấy phép (O5) chỉ có `[Đã hiểu]`. Mỗi bước có `[Để sau]`. Bỏ qua thì giá trị mặc định ở cột "Nếu bỏ qua" được áp dụng, và việc đó được nhắc lại đúng chỗ (ví dụ Similar nhắc bật phân tích hình ảnh).
- **Quay lại** bất kỳ lúc nào. Thay đổi ở bước trước không huỷ việc đang chạy nền (ví dụ quay về O3 thêm thư mục thì O4 tự cập nhật số liệu).
- **Thoát giữa chừng** (đóng cửa sổ, tắt app): lần mở sau tiếp tục ở bước đang dở. Lưu `onboarding_step` trong `u.settings` **[Thiếu BE: chỉ cần lệnh settings chung]**.
- **Chạy lại:** Cài đặt → Trợ giúp → "Chạy lại hướng dẫn". Khi chạy lại, các bước đã có dữ liệu hiện trạng thái hiện tại (ví dụ O3 liệt kê các nguồn đã có).

### 2.2 Các bước

| Bước | Hiện khi | Nội dung | Nút | Nếu bỏ qua |
|---|---|---|---|---|
| O1 Chào mừng | luôn | Tên app, một câu giới thiệu ("Phân tích giải phẫu font, tìm font tương tự và ghép cặp có giải thích theo từng trục."). Chọn **ngôn ngữ** (Tiếng Việt / English) và **theme** (Theo hệ thống / Sáng / Tối), áp dụng ngay. | **`[Bắt đầu]`** · `[Bỏ qua hướng dẫn]` | Bỏ qua hướng dẫn → đi thẳng S1 rỗng với trạng thái "Thư viện rỗng" (mục 7). Giấy phép sẽ hiện lúc quét đầu tiên. |
| O2 Dữ liệu cũ | phát hiện `fontatlas.db` của bản cũ và `u.settings.legacy_import` chưa có | "Tìm thấy dữ liệu từ phiên bản trước." Xem trước số lượng: N nguồn, N yêu thích, N collection, N tag (đọc thử, chưa ghi). | **`[Chuyển dữ liệu]`** · `[Bỏ qua]` | Không nhập. Cài đặt → Dữ liệu có nút "Nhập dữ liệu cũ" để làm sau. |
| O3 Thư mục font | luôn | Danh sách nguồn (trống ở lần đầu). `[+ Thêm thư mục]` mở hộp chọn thư mục (chọn được nhiều). Gợi ý nhanh: "Font của tôi" (`~/Library/Fonts`), "Font hệ thống" (`/Library/Fonts`), và thư viện RightFont/FontBase nếu phát hiện. Mỗi dòng: đường dẫn, loại (trong máy / ổ ngoài / iCloud), `[Xoá]`. | **`[Tiếp tục]`** (cần ít nhất 1 nguồn) · `[Để sau]` | Thư viện rỗng; S1 hiện nút "Thêm thư mục font". |
| O4 Quét | có nguồn | Quét bắt đầu ngay khi thêm nguồn (không đợi tới bước này). Hiện: "Đã tìm 1.240 file · đã phân tích 860 font · 3 file có vấn đề". Bên dưới là **lưới xem trước nhỏ, font hiện dần** khi phân tích xong. | **`[Tiếp tục]`** (bấm được khi quét đang chạy) | Quét vẫn chạy nền; thanh trạng thái tiếp tục báo. |
| O5 Giấy phép | luôn (một lần) | "FontAtlas phân tích mọi font bạn đưa vào thư viện, kể cả font thương mại. Bạn tự chịu trách nhiệm tuân thủ giấy phép của từng font. Dữ liệu phân tích và hình ảnh không bao giờ rời khỏi máy." Đường dẫn "Tìm hiểu thêm". | **`[Đã hiểu]`** · `[Quay lại]` | Không bỏ qua được. Lưu `license_notice_ack`. Nếu người dùng thoát onboarding trước bước này, thông báo hiện lại lúc quét đầu tiên hoặc lúc mở Similar lần đầu (spec 5.1). |
| O6 Phân tích hình ảnh | luôn | Xem mục 2.4. | **`[Tải ngay]`** · `[Để sau]` · `[Không dùng]` | `[Để sau]`: embedding bật nhưng chưa có model → app chạy chế độ "Số đo". `[Không dùng]`: `embed_enabled = false`. |
| O7 Kích hoạt font | macOS (Windows: hiện "Sắp có trên Windows" rồi qua bước) | Xem mục 2.5. | **`[Bật kích hoạt font]`** · `[Để sau]` | Kích hoạt tắt; thẻ không hiện công tắc kích hoạt. |
| Kết thúc | — | "Xong. Thư viện của bạn đang được phân tích." Ba gợi ý: mở một font, thử ghép cặp, đổi chữ mẫu. | **`[Vào thư viện]`** | — |

### 2.3 Thêm thư mục: nguồn lồng nhau (D4) và ổ ngoài
Backend chặn nguồn lồng nhau: `add_source` trả `nested_source { existing_id }` khi thư mục mới nằm trong, hoặc chứa, một nguồn đã có (Plan 2, A7).

```
Thư mục này nằm trong một nguồn đã có

  Mới:     /Volumes/Ổ ngoài/Fonts/Khách hàng A
  Đã có:   /Volumes/Ổ ngoài/Fonts          (3.120 font)

Font trong "Khách hàng A" đã có trong thư viện qua nguồn "Fonts".
Muốn nhóm riêng font của khách hàng A, hãy tạo một collection.

        [Huỷ]   [Tạo collection "Khách hàng A"]
```

- **Thư mục mới chứa một nguồn đã có** (ngược lại): "Thư mục này chứa nguồn 'Fonts' đã có." `[Huỷ]` · `[Thay nguồn cũ bằng thư mục này]`.
  - Thay thì gỡ nguồn con rồi thêm nguồn cha.
  - Yêu thích, collection và tag được giữ vì khoá theo hash.
  - Font chỉ quét lại phần mới.
- **Trùng đúng đường dẫn đã có:** không hỏi; đánh dấu dòng đó "Đã có" trong danh sách.
- **Ổ ngoài hoặc iCloud:** ghi loại nguồn. Nhắc một dòng: "Font sẽ không khả dụng khi rút ổ; dữ liệu của bạn không mất."
- **Không có quyền đọc** (macOS chặn): "Không đọc được thư mục" + `[Mở Cài đặt hệ thống]`.

### 2.4 O6 — Bật phân tích hình ảnh

```
Phân tích hình ảnh (tuỳ chọn)

So sánh hình dáng chữ bằng một model nhận dạng ảnh, cộng thêm vào số đo
giải phẫu. Kết quả Similar sát với cảm nhận bằng mắt hơn, nhất là với font
display và script.

• Tải một lần ~371 MB từ Hugging Face (repo chính thức của Google:
  SigLIP 2, giấy phép Apache-2.0). Có kiểm tra checksum.
• Thư viện hiện tại: 3.800 font → phân tích mất khoảng 2 phút
  trên máy này, chạy nền, không chặn việc khác.
• Ổ đĩa còn trống: 212 GB.
• Mọi xử lý chạy trên máy. Không gửi font hay ảnh đi đâu.

Không bật thì Similar và Pairing vẫn chạy bằng số đo.

        [Không dùng]   [Để sau]   [ Tải ngay ]
```

- **Thời gian ước tính** = số face chờ × tốc độ.
  - Tốc độ ban đầu theo đo thật ở Plan 4b: 28 ms/face với Metal, 30 ms/face với CPU trên M4 Pro. Ví dụ 3.800 face ≈ 1 phút 50 giây.
  - Sau lô đầu tiên, dùng trung bình trượt của tốc độ thật.
  - Hiện dạng khoảng ("khoảng 2 phút", "khoảng 15 phút"), làm tròn lên. Trên Windows (candle CPU thuần Rust, chưa đo) ghi "chưa có số đo trên máy Windows, sẽ ước tính sau vài giây".
  - Nếu quét chưa xong: "khoảng 2 phút cho 3.800 font đã tìm thấy; con số sẽ tăng khi quét tiếp".
- **Thời gian tải** ước theo tốc độ mạng đo trong 3 giây đầu; trước đó chỉ hiện dung lượng.
- **Kiểm dung lượng trống** trước khi tải: cần ≥ 371 MB + 10%. Không đủ thì nút `[Tải ngay]` tắt, kèm "Cần thêm X MB trống" **[Thiếu BE: lệnh kiểm dung lượng]**.
- **Bấm `[Tải ngay]`:** onboarding đi tiếp sang O7 ngay. Việc tải hiện ở thanh trạng thái; không có màn chờ.

### 2.5 O7 — Bật kích hoạt font

```
Kích hoạt font cho app thiết kế

Bật một font trong FontAtlas để dùng ngay trong Photoshop, Illustrator,
Figma… mà không cần cài vào hệ thống. File font giữ nguyên chỗ cũ.

Khi tắt FontAtlas:
  (•) Tắt luôn các font đã kích hoạt   ← mặc định
      Font chỉ hoạt động khi FontAtlas đang mở. Mở lại app thì font được
      kích hoạt lại tự động.
  ( ) Giữ font hoạt động cả khi tắt FontAtlas
      Font vẫn dùng được sau khi tắt app, cho tới khi bạn tắt nó trong
      FontAtlas.

⚠ Phát hiện RightFont đang kích hoạt khoảng 2.300 font.
  Kích hoạt cùng một font ở hai app sẽ tạo bản trùng tên; app thiết kế có
  thể chọn nhầm bản.
  [✓] Không kích hoạt font đã có trên hệ thống với cùng tên  ← mặc định

        [Để sau]   [ Bật kích hoạt font ]
```

- **Phát hiện app khác:** dò RightFont và FontBase (bản cài, và số đăng ký CoreText trỏ tới thư viện của chúng — spike 04 §1.4).
  - Chỉ đếm, không đọc dữ liệu khác của các app đó.
  - Không phát hiện được thì ẩn khối ⚠ **[Thiếu BE: `detect_font_managers`]**.
- **Thuật ngữ:** "Tắt luôn khi tắt app" ứng với scope `session`; "Giữ cả khi tắt app" ứng với scope `persistent`. Ở UI không dùng chữ "session/persistent".
- **Có thể đổi sau** trong Cài đặt → Kích hoạt font. Đổi chế độ không làm thay đổi các font đang kích hoạt. App hỏi: "Áp dụng cho 42 font đang kích hoạt không?" `[Áp dụng]` `[Chỉ font kích hoạt từ nay]`.

---

## 3. Trải nghiệm model và embedding

### 3.1 Trạng thái

Gộp từ `ModelStatus` (Missing / Partial / Ready / Corrupt), `EmbedMode` (`MeasuresOnly(reason)` / `Images`), tiến trình `ensure()` và `Progress` của `EmbedWorker`.

| # | Trạng thái | Nguồn backend | Thanh trạng thái | Cài đặt → Phân tích hình ảnh | Trong Similar / Hồ sơ | Hành động |
|---|---|---|---|---|---|---|
| E0 | Đã tắt | `embed_enabled = false` → `MeasuresOnly(Disabled)` | (không hiện) | Công tắc tắt. "Similar và Pairing dùng số đo." | Huy hiệu `Số đo`. Dải gợi ý (mục 3.3) chỉ hiện nếu người dùng chưa từng chọn "Không dùng" | `[Bật]` |
| E1 | Chưa tải model | `Missing` → `MeasuresOnly(ModelMissing)` | "Chưa tải model hình ảnh · Tải" (chỉ hiện 1 lần mỗi phiên, đóng được) | "Chưa tải (~371 MB)" `[Tải model]` | Dải gợi ý | `[Tải model]` |
| E2 | Đang tải | `ensure()` đang chạy, callback `progress(done, total)` | "Đang tải model 38% · 140/371 MB · ~1 phút" `[■ Dừng]` | Thanh tiến độ, tốc độ MB/s, `[Dừng]` | "Đang tải model (38%). Kết quả hiện chỉ dựa trên số đo." | `[Dừng]` |
| E3 | Đã dừng tải | `cancel` → `Cancelled`; file `.part` bị xoá | "Đã dừng tải model · Tải lại" | "Đã dừng. Lần tải sau bắt đầu lại từ đầu." `[Tải lại]` | Dải gợi ý | `[Tải lại]` |
| E4 | Lỗi mạng | `ensure()` trả `io` | "⚠ Tải model lỗi mạng · Thử lại" | Mô tả lỗi ngắn + `[Thử lại]`. Tự thử lại sau 30 s, 2 phút, 10 phút (tối đa 3 lần) khi mạng có lại | Dải gợi ý kèm "(lỗi mạng)" | `[Thử lại]` |
| E5 | Lỗi checksum | `checksum` | "⚠ Model tải về bị hỏng · Tải lại" | "File tải về không khớp checksum, đã xoá. Có thể do mạng hoặc proxy." `[Tải lại]` | như E4 | `[Tải lại]` |
| E6 | Model hỏng trên đĩa | `Corrupt` / `MeasuresOnly(ModelCorrupt)` khi khởi động | "⚠ Model hình ảnh bị hỏng" | `[Tải lại]` `[Xoá model]` | như E4 | `[Tải lại]` |
| E7 | Không nạp được model | `MeasuresOnly(LoadFailed)` (Metal lỗi thì đã tự rơi về CPU; tới đây là CPU cũng lỗi) | "⚠ Không chạy được phân tích hình ảnh" | Mã lỗi, `[Thử lại]`, `[Gửi báo lỗi]` (P1) | như E4 | `[Thử lại]` |
| E8 | Đang phân tích | `Progress::Started{pending}` rồi `Batch{done, failed, skipped}` | "Phân tích hình ảnh 412/3.800 · ~2 phút" `[⏸]` | Thanh tiến độ, "Đang dùng: Metal (GPU)" hoặc "CPU", `[Tạm dừng]` `[Dừng]` | Huy hiệu font chưa có ảnh: `Số đo`; đã có: `Số đo + Hình ảnh`. Dòng nhỏ trong Similar: "Đã phân tích hình ảnh 412/3.800 font; kết quả sẽ đầy đủ hơn khi xong." | `[Tạm dừng]` `[Dừng]` |
| E9 | Tạm dừng phân tích | `Paused` | "Phân tích hình ảnh tạm dừng 412/3.800" `[▶]` | `[Tiếp tục]` | như E8 | `[Tiếp tục]` |
| E10 | Xong | `Finished{total_done}`; không còn hàng đợi | Thông báo 5 s: "Đã phân tích hình ảnh 3.800 font", rồi ẩn | "Sẵn sàng · 3.800 font có ảnh · model SigLIP 2 · 371 MB" `[Xoá model]` | Huy hiệu `Số đo + Hình ảnh` | — |
| E11 | Một số font không phân tích được | `failed`/`skipped` > 0 (ví dụ `no_glyphs`: font không đủ ký tự cho chuỗi mẫu) | (không làm phiền) | "12 font không phân tích được hình ảnh" → mở S6 lọc theo loại "Hình ảnh" | Font đó vẫn huy hiệu `Số đo` kèm tooltip lý do | `[Xem]` |

**Quy tắc chung:**
- **Thứ tự:** tải model xong thì bắt đầu phân tích ngay (E2 → E8), không hỏi lại.
- **Có font mới khi quét lại:** font mới vào hàng đợi. Thanh trạng thái hiện E8 với số mới.
- **Khởi động lại app giữa chừng:**
  - Đang phân tích: tiếp tục tự động (backend `reset_running_embeddings` + `requeue_missing_embeddings`).
  - Đang tải: **không** tự tải lại; hiện E3 với `[Tải lại]`, vì tải lại tốn 371 MB.
- **Tạm dừng tải:** backend hiện chưa tiếp tục được (file `.part` bị xoá khi huỷ), nên UI gọi nút là `[Dừng]`, không gọi là "Tạm dừng". **[Thiếu BE — P1]:** tải tiếp bằng HTTP Range để có `[Tạm dừng]`/`[Tiếp tục]` thật.
- **Tắt khi đang chạy:** tắt công tắc phân tích hình ảnh lúc đang tải hoặc đang phân tích → huỷ ngay (plan6-notes: app phải gọi `cancel()`), giữ các embedding đã có. Hỏi một câu: "Giữ kết quả đã phân tích (412 font)?" `[Giữ]` `[Xoá]`.
- **Pin/nhiệt:** Sau. Chừa tuỳ chọn "Chỉ phân tích khi cắm sạc".

### 3.2 Thanh trạng thái và Trung tâm hoạt động
- **Thanh trạng thái** hiện tối đa 2 việc theo thứ tự: quét, tải model, phân tích hình ảnh, kích hoạt hàng loạt. Việc thứ 3 trở đi gộp thành "+1".
- **Trung tâm hoạt động (D2)** liệt kê từng việc với tiến độ, nút điều khiển và lỗi gần nhất. Có `[Mở Cài đặt]` cho mục tương ứng.
- **Đọc màn hình:** cập nhật tiến độ theo `aria-live="polite"`, tối đa 1 lần mỗi 10 s. Lỗi dùng `aria-live="assertive"`.

### 3.3 Dải gợi ý trong Similar (và tab "Tương tự" của Hồ sơ)

```
ⓘ Kết quả chỉ dựa trên số đo giải phẫu.  Bật phân tích hình ảnh để so cả hình
  dáng chữ (~371 MB, khoảng 2 phút cho thư viện của bạn).   [Bật]  [Không hiện lại]
```

- **Theo trạng thái:**
  - E0/E1/E3: bấm `[Bật]` thì bật và tải ngay (sang E2).
  - E4–E7: dải đổi thành mô tả lỗi kèm `[Thử lại]`.
  - E8/E9: dải đổi thành "Đang phân tích hình ảnh 412/3.800 — kết quả sẽ đầy đủ hơn khi xong".
- **Công tắc "Dùng hình ảnh"** trong bảng trọng số của Similar bị mờ khi chưa có model, kèm tooltip giải thích.
- **Không tự chạy lại Similar** khi phân tích xong: hiện "Có kết quả mới dùng hình ảnh · Làm mới".

---

## 4. Kích hoạt font (Plan 7, macOS trước)

### 4.1 Trạng thái một file
Đơn vị kích hoạt là **file**, theo spike 04. Một file TTC chứa nhiều face thì kích hoạt cả file.

| Trạng thái | Nguồn (`activation_state.actual`) | Ký hiệu trên thẻ | Mô tả |
|---|---|---|---|
| Chưa kích hoạt | `inactive` | (không có; công tắc hiện khi rê chuột) | — |
| Đang kích hoạt | (tạm, trong lúc gọi) | vòng xoay nhỏ | — |
| Đang hoạt động — theo phiên | `active` + scope `session` | ⦿ | "Hoạt động khi FontAtlas mở" |
| Đang hoạt động — giữ khi tắt app | `active` + `persistent` | ⦿ có ghim | "Hoạt động cả khi tắt FontAtlas" |
| Trùng tên | `conflict` | ⦿ viền gạch + ⚠ | "Tên PostScript trùng với font khác đang có trên máy" |
| Thiếu file | `missing_file` | ⦿ rỗng + ⚠ | "Không tìm thấy file (ổ ngoài đã rút hoặc file đã chuyển)" |
| iCloud chưa tải | `NotDownloaded` | ⦿ rỗng + ☁ | "File đang ở iCloud, chưa tải về máy" |
| Lỗi | `error` (`last_error`) | ⚠ | Mã lỗi ngắn, `[Thử lại]` |

Ký hiệu luôn có chữ trong tooltip và nhãn a11y, không chỉ dựa vào màu.

### 4.2 Các điểm kích hoạt
- **Trên thẻ (S1, S5):**
  - rê chuột hiện công tắc ⦿ ở góc thẻ; bấm để bật/tắt;
  - phím `A` (trên thẻ đang focus) hoặc thanh hành động khi chọn nhiều;
  - menu chuột phải: "Kích hoạt", "Kích hoạt và giữ khi tắt app", "Tắt".
- **Trong Hồ sơ (S2):** nút `[Kích hoạt]` cạnh tên font, kèm menu ▾ chọn chế độ. Với TTC hoặc family, nút thành "Kích hoạt file (4 style)" và liệt kê các face sẽ kích hoạt cùng.
- **Theo collection (S5, "kích hoạt cả dự án"):**
  - nút `[Kích hoạt cả collection]` trên đầu collection; menu ▾ chọn chế độ;
  - sidebar có công tắc nhỏ cạnh tên collection khi rê chuột;
  - trạng thái collection hiện "18/20 đang hoạt động · 2 trùng tên".
  - Collection có font ở ổ ngoài đang rút: "3 font không có file — sẽ kích hoạt khi ổ được cắm lại" **[Thiếu BE: kích hoạt chờ ổ]**.
- **Tắt hàng loạt:** mục sidebar "Đang kích hoạt" có `[Tắt tất cả]`. Cài đặt có "Tắt mọi font FontAtlas đã kích hoạt" (dùng trước khi gỡ app).
- **Kích hoạt hàng loạt** chạy nền và hiện ở thanh trạng thái ("Đang kích hoạt 120/300"). Huỷ được.

### 4.3 Xung đột trùng tên (D3)
Trước khi kích hoạt, backend chạy `preflight`: đọc tên PostScript và so với tên đang có trên hệ thống. CoreText không tự chặn trùng (spike 04 #4).

```
3 font trùng tên với font đang có trên máy

  Helvetica Neue Bold     đang có: hệ thống macOS
  Playfair Display Bold   đang có: RightFont (iCloud Drive/Fonts/…)
  Be Vietnam Pro Regular  đang có: một file khác trong thư viện
                          (/Volumes/Ổ ngoài/Fonts/BeVietnamPro-Regular.ttf)

App thiết kế có thể dùng nhầm bản khi hai bản cùng tên.

  ( ) Bỏ qua 3 font này, kích hoạt 117 font còn lại   ← mặc định
  ( ) Vẫn kích hoạt cả 120 font
                                      [Huỷ]  [ Tiếp tục ]
```

- **Trùng với font hệ thống:** không cho chọn "Vẫn kích hoạt" cho riêng những dòng đó; ghi lý do "font hệ thống luôn được ưu tiên".
- **Trùng với một file khác trong thư viện:** gợi ý "Kích hoạt bản kia thay thế" và "Mở so sánh hai bản" (mở S2 của từng bản, cùng hash thì là một font và không hiện ở đây).
- **Ghi nhớ lựa chọn:** "Không hỏi lại với font hệ thống" (mặc định bật). Đổi lại trong Cài đặt.

### 4.4 Sau khi khởi động lại
- **Lúc mở app**, `reconcile` chạy nền:
  - font theo phiên được kích hoạt lại;
  - đăng ký mồ côi bị gỡ;
  - font giữ khi tắt app được kiểm còn file hay không.
- **Kết quả** hiện một thông báo gọn, không modal: "Đã kích hoạt lại 42 font. 2 font không tìm thấy file." `[Xem]` mở danh sách trạng thái "Thiếu file".
- **Font thiếu file vì ổ ngoài rút:** giữ trạng thái mong muốn là "đang hoạt động". Khi ổ cắm lại và quét lại, font tự kích hoạt lại. Không bao giờ coi font là đã mất.
- **App bị crash:** font theo phiên còn tới khi đăng xuất (hệ điều hành giữ); lần mở sau `reconcile` dọn. Không cần báo người dùng.
- **Khi thoát app** có font theo phiên đang hoạt động: không hỏi. Tooltip của nút thoát và dòng trong Cài đặt nhắc "Tắt FontAtlas sẽ tắt 42 font đang kích hoạt theo phiên".

### 4.5 Cài đặt → Kích hoạt font
Xem mục 5, nhóm "Kích hoạt font".

---

## 5. Cài đặt (S7, `⌘,`)

Bố cục hai cột: danh mục bên trái, nội dung bên phải. Thay đổi có hiệu lực ngay (không có nút Lưu), trừ khi ghi khác.

| Nhóm | Mục | Kiểu | Mặc định | Ghi chú |
|---|---|---|---|---|
| **Chung** | Ngôn ngữ giao diện | Tiếng Việt / English | theo hệ thống | áp dụng ngay |
| | Giao diện | Theo hệ thống / Sáng / Tối | theo hệ thống | vùng specimen luôn nền phẳng trung tính |
| | Mật độ | Thoáng / Gọn | Thoáng | |
| | Chữ mẫu mặc định | preset ▾ hoặc tự gõ | "Hamburgefonts Ăn phở" | preset: Tiêu đề F&B, Chữ hoa chồng dấu, Thực đơn có giá, Đoạn văn, Số và ký hiệu (research 06 §2.2) |
| | Cỡ thẻ mặc định | 32 / 48 / 64 / 96 | 48 | khớp `PX_BUCKETS` |
| | Mở lại màn cuối khi khởi động | bật/tắt | bật | |
| **Thư viện và nguồn** | Danh sách nguồn | bảng: tên, đường dẫn, loại, số font, trạng thái (Sẵn sàng / Không khả dụng / Đang quét), lần quét cuối | — | `[+ Thêm]` `[Quét lại]` `[Đổi tên]` `[Xoá nguồn]`. Xoá chỉ gỡ khỏi thư viện, không xoá file; giữ yêu thích/collection (khoá theo hash) |
| | Quét lại khi mở app | bật/tắt | bật | file watcher là Sau |
| | Hiện font có file bị thiếu | bật/tắt | tắt | `includeMissing` |
| **Phân tích hình ảnh** | Bật phân tích hình ảnh | công tắc | bật (spec 5.1) | tắt khi đang chạy → huỷ, hỏi giữ kết quả |
| | Trạng thái model | dòng trạng thái E0–E11 | — | `[Tải model]` `[Dừng]` `[Tải lại]` `[Xoá model]` |
| | Tiến độ phân tích | thanh tiến độ, thiết bị (Metal/CPU) | — | `[Tạm dừng]` `[Tiếp tục]` |
| | Trọng số hình ảnh trong Similar | thanh trượt 0–0,8 | 0,8 (`EMBED_WEIGHT`, Plan 4) | P1; nâng cao |
| | Phân tích lại toàn bộ | nút | — | P1; dùng khi đổi model |
| **Kích hoạt font** (macOS) | Bật kích hoạt font | công tắc | theo O7 | tắt → hỏi có tắt các font đang hoạt động không |
| | Khi tắt FontAtlas | Tắt luôn font đã kích hoạt / Giữ font hoạt động | Tắt luôn | đổi thì hỏi áp dụng cho font đang hoạt động |
| | Font trùng tên | Bỏ qua / Luôn hỏi | Luôn hỏi | |
| | Không kích hoạt đè font hệ thống | bật/tắt | bật | |
| | App quản lý font khác | dòng trạng thái: "RightFont: ~2.300 font đang kích hoạt" | — | chỉ thông tin |
| | Tắt mọi font FontAtlas đã kích hoạt | nút nguy hiểm, xác nhận | — | dùng trước khi gỡ app |
| **Giấy phép** | Dòng cố định | văn bản | — | "Bạn tự chịu trách nhiệm tuân thủ giấy phép font của mình. FontAtlas phân tích mọi font trong thư viện và không gửi font hay kết quả đi đâu." (spec 5.1) |
| | Giấy phép model | đường dẫn | — | mở `model-NOTICE.txt`, `model-LICENSE-Apache-2.0.txt` |
| **Dữ liệu** | Vị trí dữ liệu | đường dẫn, `[Mở trong Finder]` | thư mục dữ liệu app | |
| | Sao lưu dữ liệu người dùng | `[Sao lưu ngay]`, danh sách bản sao lưu | — | `user.db` (yêu thích, collection, tag, kích hoạt) **[Thiếu BE: lệnh sao lưu theo yêu cầu]** |
| | Nhập dữ liệu từ bản cũ | nút | — | `import_legacy` |
| | Bộ nhớ đệm preview | dung lượng, `[Xoá bộ nhớ đệm]` | — | `previews.db` |
| | Dựng lại thư viện | nút nguy hiểm | — | xoá `library.db` và quét lại; không đụng `user.db` |
| **Phím tắt** | Bảng phím tắt (mục 7.3) | chỉ xem | — | P1: đổi phím |
| **Trợ giúp** | Chạy lại hướng dẫn | nút | — | S0 |
| | Phiên bản app, `analyzer_version`, `model_version` | văn bản | — | để báo lỗi |
| | Gửi báo lỗi | nút | — | P1 |

---

## 6. Luồng chính

### 6.1 Thư viện (S1)
1. **Lưới** react-window, preview PNG mask (Plan 5), cuộn mượt với 5.000+ font.
   - Mỗi thẻ: preview chữ mẫu; tên family + style; huy hiệu tiếng Việt ("Đủ tiếng Việt" / "Thiếu N ký tự"); ♡ yêu thích; nhãn `VF`; ⦿ kích hoạt khi rê chuột.
   - Thẻ của font chỉ có chữ hoa, hoặc không phải Latin, tự dùng chữ dự phòng và ghi "Chữ mẫu thay thế" nhỏ dưới preview.
2. **Chữ mẫu toàn cục** trên toolbar: ô gõ + preset ▾. Đổi thì các thẻ đang hiện vẽ lại; ảnh cũ giữ tới khi có ảnh mới (Plan 5).
3. **Tìm kiếm** `⌘K`: gõ không dấu vẫn ra có dấu, khớp đầu từ xếp trước. Gõ một câu mô tả thì mở S8 (mục 6.6).
4. **Bộ lọc** (nút "Bộ lọc" → popover).
   - Có sẵn ở backend: phân loại; chỉ font đủ tiếng Việt / có hỗ trợ tiếng Việt; VF; nghiêng; yêu thích; nguồn; collection; tag; giấy phép (chỉ thông tin).
   - Theo khoảng hai đầu trên trục (x-height, tương phản, độ rộng, độ đậm, aperture) là P0 theo B2 nhưng **[Thiếu BE: `FaceQuery` chưa có lọc theo trục]**.
   - Bộ lọc đang bật hiện thành chip xoá được dưới toolbar.
5. **Sắp xếp:** tên / độ đậm / ngày thêm, tăng/giảm.
6. **Chọn một thẻ:** panel phải hiện hồ sơ rút gọn (tên, câu tóm tắt, 3 chip trục, huy hiệu tiếng Việt và nguồn phân tích, các nút: Tương tự · Ghép cặp · Thêm vào collection · Kích hoạt). Enter hoặc nhấp đúp mở S2 toàn màn.
7. **`Space`** mở Xem nhanh (D1): specimen lớn của font đó, gõ thử chữ, `←/→` sang thẻ bên cạnh.

### 6.2 Hồ sơ giải phẫu (S2, W4)
- **Đầu trang:**
  - tên + style ▾ (đổi face trong family);
  - huy hiệu `Số đo` / `Số đo + Hình ảnh`, giấy phép (thông tin);
  - nút: Kích hoạt · Yêu thích · Thêm vào collection · Tương tự · Ghép cặp;
  - công tắc độ sâu 0/1/2.
- **Câu tóm tắt** 1–2 câu, số in đậm (`explain_*` mẫu câu song ngữ, không LLM).
- **Specimen:** chữ hoa chồng dấu cỡ lớn có đường cap + vùng dấu; tiêu đề F&B; đoạn văn tiếng Việt thật; waterfall 12 → 96. VF có thanh trượt trục.
- **Tab dưới:**
  - **Waterfall**.
  - **Glyph tiếng Việt:** 134 ô; ô thiếu có viền gạch + chữ "thiếu", không chỉ màu; bảng chữ hoa chồng dấu tô vùng nhô quá cap/ascender; cảnh báo `stacked_marks_overflow`.
  - **Trục giải phẫu:** dải phân vị; trục "ước lượng" nét đứt; trục "không đo được" ghi lý do (`no_latin_lowercase`, `missing_glyph`…).
  - **Thông tin:** designer, foundry, phiên bản, OpenType features, đường dẫn file. P1.
  - **Tương tự:** 3 font gần nhất kèm "Giống ở … / Khác ở …"; "Xem tất cả" mở S3.
- **Độ sâu 1:** hover chip trục thì overlay đường baseline/x-height/cap/ascender/descender trên specimen.

### 6.3 Similar (S3)
1. **Font gốc** ở đầu màn (đổi được bằng ô chọn). Kết quả dạng lưới hoặc danh sách.
2. **Mỗi kết quả:** preview cùng chữ mẫu; dòng "Giống ở: x-height, độ rộng · Khác ở: tương phản" (`explain_similar`); huy hiệu nguồn phân tích. Không có số điểm.
3. **Bảng trọng số** (thanh trượt theo nhóm: cấu trúc, tỉ lệ, độ đậm, chi tiết) + công tắc "Dùng hình ảnh" (mục 3.3). Đổi trọng số thì kết quả cập nhật trong < 50 ms (S6), không cần nút chạy.
4. **Lọc nhanh:** đủ tiếng Việt, độ đậm, nguồn.
5. **Trên một kết quả:** mở hồ sơ · "Dùng làm font gốc" · Ghép cặp với font gốc · Thêm vào collection · Kích hoạt.
6. **Font gốc không so được** (`NotComparable`: quá ít trục đo được, ví dụ font ký hiệu): "Font này có quá ít số đo để tìm font tương tự." Nếu có ảnh thì gợi ý "Thử chỉ dùng hình ảnh" **[Thiếu BE: chế độ chỉ ảnh ở `similar`]**.

### 6.4 Pairing Studio (S4, W1–W3)
1. **Thanh trên:**
   - **vai trò:** "Tiêu đề: [font A ▾]" và "Nội dung: [ứng viên]"; nút ⇄ đổi vai;
   - **chế độ:** tab ngang hàng `[An toàn | Có kiểm soát | Thử nghiệm]`, không có huy hiệu "khuyên dùng";
   - **preset bố cục:** Menu · Bảng hiệu/Logo · Poster sự kiện · Nhãn bao bì · Brand sheet;
   - **"Dự án dùng tiếng Việt":** công tắc, mặc định bật (`project_vi`);
   - **độ sâu:** công tắc 0/1/2.
2. **Canvas:** bố cục preset với chữ tiếng Việt thật, sửa được tại chỗ. Bố cục đứng yên; `◀ ▶` (hoặc `←/→`) đổi ứng viên ("3 / 12").
3. **Độ sâu:**
   - **0:** chỉ có câu tóm tắt.
   - **1:** 3–5 chip trục ở lề phải; hover chip thì overlay trên specimen.
   - **2:** panel Phòng thí nghiệm gồm dumbbell theo trục (thang: phân vị trong thư viện của bạn); glyph chồng a e g R Ư; bảng chồng dấu Việt hai font; ràng buộc thực tế; nguồn phân tích.
4. **"Chuẩn hoá theo x-height":** công tắc; kèm gợi ý cỡ chữ để cân.
5. **Trạng thái ứng viên:**
   - không còn ứng viên ở chế độ này → "Không có ứng viên ở chế độ An toàn. Thử 'Có kiểm soát' hoặc tắt 'Dự án dùng tiếng Việt'";
   - font tiêu đề thiếu tiếng Việt khi `project_vi` bật → backend trả rỗng; UI ghi rõ lý do và nút "Xem ký tự thiếu".
6. **Lưu:** `[Lưu cặp]` → chọn collection (hoặc tạo mới). Lưu gồm cả hai font, vai trò, chế độ, preset, chữ đã sửa **[Thiếu BE: chưa có bảng lưu cặp; hiện collection chỉ chứa face]**. Phương án tạm nếu chưa có: thêm hai face vào collection và ghi chú "Cặp: A (tiêu đề) + B (nội dung)".
7. **Xuất (P1):** PDF trang hiện tại có chân trang ghi font + cỡ.

### 6.5 Collection (S5)
- **Lưới** như S1, giới hạn trong collection.
- **Đầu trang:** tên (đổi tại chỗ), màu, số font; `[Kích hoạt cả collection]`; `[Ghép cặp trong collection]` (ứng viên Pairing chỉ lấy từ collection) **[Thiếu BE: lọc ứng viên theo collection ở `pair_candidates`]**.
- **Mục "Cặp đã lưu"** (nếu có BE): mỗi cặp là một thẻ specimen nhỏ, mở lại đúng trạng thái trong S4.
- **Gỡ khỏi collection:** `⌫` trên thẻ đang chọn. Xoá collection không xoá font.

### 6.6 Tìm bằng mô tả (S8, spec 8)
1. **Nhập:** gõ câu vào ô tìm, ví dụ "chữ ấm, cổ điển cho quán cà phê". Câu có từ khoá trong từ điển thì popover hiện các **chip bộ lọc suy ra**: "ấm → aperture mở · đầu nét tròn · tương phản thấp–vừa", "cổ điển → serif · trục nhấn nghiêng".
2. **Chỉnh và áp dụng:** người dùng bật/tắt từng chip và chỉnh khoảng; `Enter` áp dụng vào S1 (thành chip lọc thường).
3. **Xung đột từ khoá** (ví dụ "ấm" và "kỹ thuật" kéo ngược trục aperture): hiện "◆ 'ấm' và 'kỹ thuật' trái nhau ở aperture. Chọn một:" với hai lựa chọn. App không tự chọn.
4. **Ghi chú:** phần chủ quan ghi rõ "theo từ điển của FontAtlas (chủ quan)". Từ không có trong từ điển thì gạch dưới mờ, "chưa hiểu từ này".
5. **Backend:** cần từ điển + lệnh phân tích câu, và lọc theo trục **[Thiếu BE]**.

---

## 7. Trạng thái mỗi màn, phím tắt, a11y

### 7.1 Trạng thái theo màn

| Màn | Rỗng | Đang tải | Lỗi | Ổ ngoài rút |
|---|---|---|---|---|
| S1 Thư viện | Chưa có nguồn: minh hoạ tối giản + "Thêm thư mục font" `[+ Thêm thư mục]`. Có nguồn nhưng lọc ra 0: "Không có font nào khớp" + `[Xoá bộ lọc]` | Lần đầu: khung xám thẻ (không spinner giữa màn). Đang quét: font hiện dần, thanh trạng thái đếm | Lỗi tải trang: banner "Lỗi tải thư viện · Thử lại", giữ lưới (Plan 5). Lỗi `query_faces` toàn phần: thông báo giữa màn + Thử lại | Nguồn hiện ◌ "Không khả dụng"; font của nguồn đó **ẩn mặc định** cùng dòng "312 font trên 'Ổ ngoài' đang không khả dụng · Hiện". Không coi là mất (`source_unavailable`) |
| S2 Hồ sơ | — | Khung xám vùng specimen; câu tóm tắt và chip hiện sau | File không đọc được: "Không mở được file font này" + mã lỗi + `[Quét lại]` | Specimen dùng preview Rust đã cache nếu có; nút Kích hoạt và FontFace tắt; dòng "File đang ở ổ ngoài chưa cắm" |
| S3 Similar | Không có kết quả qua bộ lọc: "Không có font tương tự với bộ lọc hiện tại" + `[Bỏ bộ lọc]`. Thư viện < 2 font: "Cần ít nhất 2 font" | Khung xám kết quả (tính < 50 ms nên hiếm thấy) | `NotComparable`: mục 6.3. Lỗi khác: banner + Thử lại | Kết quả của nguồn rút vẫn hiện (dữ liệu đã có), ký hiệu ◌ trên thẻ |
| S4 Pairing | Không có ứng viên: mục 6.4 | Khung xám canvas lần đầu; đổi ứng viên giữ bố cục, font mới hiện khi nạp xong (FontFace) | Font không nạp được bằng FontFace (ví dụ face ≥ 1 trong TTC): dùng preview Rust, ghi "Bản xem trước (không sửa chữ được)" | Như S2; ứng viên ở ổ rút vẫn hiện bằng preview |
| S5 Collection | "Collection trống. Kéo font vào đây hoặc dùng 'Thêm vào collection'" | như S1 | như S1 | Đầu trang: "3 font không có file" |
| S6 Font có vấn đề | "Không có font nào có vấn đề" ✓ | — | — | File trên ổ rút không bị liệt vào đây |
| S7 Cài đặt | — | Dòng trạng thái từng mục | Lỗi từng mục tại chỗ | Nguồn ◌ với `[Quét lại khi cắm]` |
| S0 Onboarding | — | O4: font hiện dần | Thêm thư mục lỗi quyền: "Không đọc được thư mục — cấp quyền trong Cài đặt hệ thống" + nút mở | Thêm ổ ngoài: ghi loại "Ổ ngoài", nhắc "font sẽ không khả dụng khi rút ổ" |

**Trạng thái theo font** (thẻ, hồ sơ, Pairing — B8):

| Trạng thái | Trên thẻ | Trong hồ sơ / Pairing |
|---|---|---|
| Thiếu tiếng Việt | "Thiếu 12 ký tự" | Tab Glyph Việt đánh dấu ô thiếu; Pairing với `project_vi` bật loại font này và nói lý do |
| Chỉ có chữ hoa | Chữ mẫu viết hoa thay thế | Trục cần chữ thường ghi "không đo được — font chỉ có chữ hoa" |
| Không phải Latin | Chữ mẫu lấy từ bảng ký tự của font, ghi "Chữ mẫu thay thế" | Chip tiếng Việt: "không hỗ trợ"; Pairing chỉ ở chế độ Thử nghiệm |
| Không có outline (emoji, bitmap) | "Không có preview" | Hồ sơ chỉ có metadata |
| Trục không đo được | — | Dải trục ghi lý do; dumbbell bỏ trục đó, ghi "bỏ qua: không đo được" |
| Đang chờ phân tích hình ảnh | — | Huy hiệu `Số đo` + "đang chờ phân tích hình ảnh" |

### 7.2 Thông báo và lỗi
- **Thông báo ngắn** (toast) ở góc dưới phải, tự ẩn sau 5 s. Không chồng quá 3. Mọi toast đều có bản ghi trong Trung tâm hoạt động.
- **Lỗi có mã:** hiện mã ngắn (ví dụ `truncated`, `checksum`, `ct.105`) để báo lỗi, kèm câu tiếng Việt dễ hiểu.
- **Hành động nguy hiểm** cần xác nhận, nêu đúng hậu quả: xoá nguồn, dựng lại thư viện, tắt mọi font kích hoạt, xoá model.

### 7.3 Phím tắt

| Phím | Tác dụng | Phạm vi |
|---|---|---|
| `⌘K` | Tìm (focus ô tìm; gõ mô tả mở S8) | mọi màn |
| `⌘⇧P` | Bảng lệnh (P1) | mọi màn |
| `⌘,` | Cài đặt | mọi màn |
| `⌘O` | Thêm thư mục font | mọi màn |
| `⌘R` | Quét lại nguồn đang chọn (hoặc tất cả) | S1, S7 |
| `⌘[` / `⌘]` | Lùi / tiến lịch sử | mọi màn |
| `↑ ↓ ← →` | Di chuyển giữa thẻ | S1, S3, S5 |
| `Enter` | Mở hồ sơ | thẻ đang focus |
| `Space` | Xem nhanh / đóng | thẻ đang focus |
| `S` | Tương tự | thẻ / hồ sơ |
| `P` | Ghép cặp | thẻ / hồ sơ / kết quả Similar |
| `F` | Yêu thích | thẻ / hồ sơ |
| `A` | Kích hoạt / tắt | thẻ / hồ sơ (khi bật kích hoạt) |
| `C` | Thêm vào collection | thẻ / hồ sơ / Pairing |
| `⌫` | Gỡ khỏi collection | S5 |
| `1` / `2` / `3` | Độ sâu 0 / 1 / 2 | S2, S4 |
| `← / →` | Ứng viên trước / sau | S4 |
| `⇧⌘X` hoặc `⇄` | Đổi vai trò | S4 |
| `⌘+` / `⌘−` | Cỡ thẻ (32/48/64/96) | S1, S3, S5 |
| `Esc` | Đóng popover → panel → Xem nhanh, theo thứ tự | mọi màn |

Phím một chữ cái không hoạt động khi đang gõ trong ô nhập.

### 7.4 a11y
- **Bàn phím:**
  - mọi thao tác làm được bằng bàn phím;
  - lưới dùng roving tabindex (`role="grid"`, `gridcell`, `aria-rowcount`/`aria-colcount`, đã có ở Plan 5);
  - focus ring rõ, tương phản ≥ 3:1.
- **Hộp thoại:**
  - có focus trap, `Esc` đóng, trả focus về chỗ cũ;
  - không chồng nhiều modal; onboarding không phải modal.
- **Tương phản:** chữ UI đạt AA. Nhãn trục và trạng thái luôn có **ký hiệu + chữ**, không chỉ màu (●◐◆○, ⦿, ◌, ⚠).
- **Đọc màn hình:**
  - preview là ảnh trang trí (`aria-hidden`), nên mỗi thẻ có nhãn đọc: "Be Vietnam Pro Regular, đủ tiếng Việt, đang kích hoạt";
  - tiến độ nền dùng `aria-live="polite"`, tối đa 1 lần mỗi 10 s;
  - lỗi dùng `aria-live="assertive"`.
- **Chuyển động:** `prefers-reduced-motion` tắt trượt panel và hiện dần overlay. Chữ UI ≥ 11 px.

---

## 8. Ánh xạ màn → lệnh backend

**Quy ước cột:**
- **Có:** crate đã có API, chỉ cần lệnh Tauri mỏng (Plan 6a).
- **Thiếu BE:** cần thêm logic backend.
- **Plan 7:** thuộc kích hoạt font.
- Tên lệnh viết `snake_case`, đối số camelCase. Sự kiện tiến độ dùng `tauri::ipc::Channel`. Bốn lệnh đầu đã có hợp đồng TS ở `UX/features/library-grid/api.ts` (`TauriCommandMap`).

### 8.1 Thư viện, nguồn, quét

| Lệnh / sự kiện | Màn | API crate | Trạng thái |
|---|---|---|---|
| `query_faces({q}) → FacePage` | S1, S5, S8 | `Store::query_faces` | Có (+ hợp đồng TS) |
| `query_faces` với khoảng theo trục (`axes: {xHeight: [a,b], …}`), phân loại | S1 bộ lọc, S8 | — | **Thiếu BE**: `FaceQuery` chưa có trục; cần cột phân vị đã chuẩn hoá |
| `get_face({hashHex, faceIndex}) → FaceDetail` (metadata, tiếng Việt, anatomy, `has_image`) | S2, panel phải | `Store` + `Index::profile` + core `FaceReport` | **Thiếu BE**: chưa có lệnh đọc một face đầy đủ |
| `list_sources() → Source[]` (kèm trạng thái khả dụng, số font) | sidebar, S7, O3 | `Store::sources` | Có; số font và trạng thái khả dụng cần ghép |
| `add_source({kind, name, path}) → id \| nested_source{existingId}` | O3, S7 | `Store::add_source` | Có |
| `remove_source({id})` | S7 | `Store::remove_source` + `gc` | Có |
| `rescan_source({id}) → RescanStats` + Channel `scan_progress{phase, done, total}` | O4, S1, thanh trạng thái | `Store::rescan_source` | Có lệnh; **Thiếu BE**: callback tiến độ trong rescan (hiện chỉ trả thống kê cuối) |
| Sự kiện `faces_added{keys}` (font hiện dần) | O4, S1 | `ingest` theo lô | **Thiếu BE**: phát sự kiện theo lô |
| `problem_files() → ProblemFile[]`, `retry_failed()` | S6 | `Store::problem_files`, `retry_failed` | Có |
| `detect_legacy() → {found, counts}` | O2 | đọc thử DB cũ | **Thiếu BE**: xem trước số lượng (hiện chỉ có `import_legacy`) |
| `import_legacy() → LegacyImport`, `resolve_legacy()` | O2, S7 | `Store::import_legacy`, `resolve_legacy` | Có |
| `set_favorite({key, on})` | thẻ, S2 | `Store::set_favorite` | Có |
| `create_collection`, `rename_collection`, `delete_collection`, `add_to_collection`, `remove_from_collection`, `collections()` | S5, sidebar | `Store::*collection*` | Có |
| `tag`, `tag_faces`, `untag_faces`, `tags()`, `tags_of` | S1, S2 | `Store::tag*` | Có |
| `get_setting({key})`, `set_setting({key, value})` | S0, S7 | `Store::setting`, `set_setting` | Có (chung) |
| `backup_user_db()` | S7 | sao lưu đã có trong migration | **Thiếu BE**: lệnh theo yêu cầu |
| `rebuild_library()` | S7 | dựng lại `library.db` khi mở | **Thiếu BE**: lệnh chủ động |
| `disk_free({path})` | O6, S7 | — | **Thiếu BE** (nhỏ) |

### 8.2 Preview và font

| Lệnh / scheme | Màn | API crate | Trạng thái |
|---|---|---|---|
| scheme `fontatlas-preview://` | S1, S3–S5 | `handle_preview_uri` | Có (Plan 5); Plan 6a đăng ký scheme + CSP |
| scheme `fontatlas-font://` | S2, S4, D1 | `handle_font_uri` | Có (Plan 5) |
| `preview_info()`, `preview_bump_generation()`, `preview_prewarm({items})` | S1 | `PreviewService` | Có (+ hợp đồng TS) |
| `clear_preview_cache()` + dung lượng | S7 | `PreviewCache` | **Thiếu BE** (nhỏ): lệnh xoá và đọc tổng |

### 8.3 Phân tích hình ảnh

| Lệnh / sự kiện | Màn | API crate | Trạng thái |
|---|---|---|---|
| `embed_status() → {enabled, model: Missing\|Partial\|Ready\|Corrupt, mode: EmbedMode, device, done, pending, failed}` | thanh trạng thái, S7, S3, S2 | `ModelStore::status`, `EmbedMode`, `count_embeddings`, `CandleEncoder::device_id` | Có từng phần; **Thiếu BE**: lệnh gộp + đếm hàng đợi |
| `embed_set_enabled({on})` (tắt → `cancel()`) | O6, S7, S3 | `Store` setting `embed_enabled`, `EmbedWorker::cancel` | Có (theo plan6-notes) |
| `model_download()` + Channel `model_progress{done, total, bytesPerSec}`; `model_cancel()` | O6, S7, S3 | `ModelStore::ensure(progress, cancel)` | Có; tốc độ tính ở lệnh |
| `model_remove()` | S7 | `ModelStore::remove` | Có |
| Tải tiếp sau khi dừng (HTTP Range) | S7 | — | **Thiếu BE — P1**: hiện huỷ thì xoá `.part` |
| `embed_pause()`, `embed_resume()`, `embed_cancel()` + Channel `embed_progress` (`Started`/`Batch`/`Paused`/`Resumed`/`Finished`/`Error`) | thanh trạng thái, S7 | `EmbedWorker` | Có |
| `embed_eta() → {msPerFace}` | O6, thanh trạng thái | đo trong worker | **Thiếu BE** (nhỏ): trả tốc độ trung bình; trước lô đầu dùng 28/30 ms theo thiết bị |
| `embed_failures() → [{key, code}]` | S6 lọc "Hình ảnh" | `pipeline_state` embed | **Thiếu BE**: lệnh đọc lỗi embedding theo face |

### 8.4 Similar, Pairing, giải thích, mô tả

| Lệnh | Màn | API crate | Trạng thái |
|---|---|---|---|
| `similar({key, weights, k, filter}) → [{key, sameAt, differAt, source}]` | S3, S2 tab Tương tự | `Index::similar` + `explain_similar` | Có |
| `similar` chỉ dùng ảnh khi font gốc `NotComparable` | S3 | — | **Thiếu BE** (P1) |
| `pair_candidates({key, qRole, candRole, mode, projectVi, k, collectionId?})` | S4 | `Index::pair_candidates` + chế độ (`Safe`/`Controlled`/`Experimental`) | Có; **Thiếu BE**: giới hạn ứng viên theo collection |
| `assess_pair({a, ra, b, rb, projectVi}) → PairAssessment + câu giải thích` | S4 khi người dùng tự chọn ứng viên | `Index::assess_pair` + `explain_pair` | Có |
| `summary({key}) → câu tóm tắt` | S2, panel phải | `explain` mẫu câu | **Thiếu BE**: mẫu câu tóm tắt một font (hiện có cho cặp và similar) |
| `vi_glyph_table({key})` (134 ô, thiếu, `stacked_marks_overflow`, vùng dấu) | S2, S4 độ sâu 2 | core `VietnameseCoverage` | Có dữ liệu thiếu; **Thiếu BE**: hộp mực dấu theo glyph cho bảng chồng dấu |
| `save_pair({a, ra, b, rb, mode, preset, text, collectionId})`, `pairs({collectionId})` | S4, S5 | — | **Thiếu BE**: bảng `u.pairs` |
| `describe({text}) → {chips: [{word, axis, range, subjective}], conflicts, unknown}` | S8 | — | **Thiếu BE**: từ điển JSON có phiên bản (spec 8) + bộ phân tích câu |

### 8.5 Kích hoạt font (Plan 7, macOS)

| Lệnh / sự kiện | Màn | API (spike 04 §4) | Trạng thái |
|---|---|---|---|
| `activation_status({hashes?}) → [{hash, desired, scope, actual, conflictPs, lastError}]` | thẻ, S2, S5, sidebar | bảng `activation_state` | Plan 7 |
| `activation_preflight({hashes}) → [{hash, outcome: ok\|conflict{psName, existing, owner: system\|rightfont\|fontbase\|library\|other}\|missing_file\|not_downloaded}]` | D3 | `ActivationEngine::preflight` | Plan 7; `owner` là đề xuất UX |
| `activate({hashes, scope, onConflict: skip\|force})` + Channel `activation_progress{done, total}` | thẻ, S2, S5 | `activate` | Plan 7 |
| `deactivate({hashes})`, `deactivate_all()` | thẻ, sidebar, S7 | `deactivate` (dùng `registered_path` + scope đã lưu) | Plan 7 |
| Sự kiện `activation_reconciled{reactivated, missing, errors}` lúc khởi động | toast | `reconcile` | Plan 7 |
| `activation_set_default_scope({scope, applyToActive})` | O7, S7 | đổi scope từng dòng | Plan 7 |
| `detect_font_managers() → [{name, activeCount}]` | O7, S7 | đếm đăng ký CoreText trỏ tới thư viện RightFont/FontBase | **Thiếu BE** (Plan 7) |
| Kích hoạt chờ ổ: khi `rescan` thấy lại file của font `desired=active` thì tự kích hoạt | S5, S1 | `reconcile` sau rescan | Plan 7 |
| Thoát app: gỡ scope `session` | — | `shutdown` | Plan 7 |

### 8.6 Thứ tự khởi động (để 02 nối dây)
1. `Store::open`.
2. Mở `PreviewCache` và `PreviewService`.
3. Nếu có DB cũ và chưa nhập: hiện O2.
4. Quét lại nguồn (nếu bật "Quét lại khi mở app"), phát `scan_progress`.
5. `resolve_legacy`.
6. Join worker embedding cũ rồi mới mở worker mới (plan6-notes), phát `embed_progress`.
7. Plan 7: `reconcile` kích hoạt, phát `activation_reconciled`.
8. Nạp `Index` (trục + embedding int8).

UI không đợi bước 4–8: hiện thư viện từ `query_faces` ngay sau bước 2.

---

## 9. Câu hỏi còn mở (cho chủ app / Lead)
1. **Lưu cặp** (6.4): thêm bảng `u.pairs` ngay trong Plan 6a/6b, hay tạm lưu hai face vào collection kèm ghi chú?
2. **Lọc theo trục ở thư viện** (6.1): thuộc P0 theo B2 nhưng `FaceQuery` chưa có. Đưa vào Plan 6a không?
3. **Mặc định "Không kích hoạt đè font đã có cùng tên"** (4.3): giữ "Luôn hỏi", hay mặc định bỏ qua im lặng?
4. **Tự động tải model khi người dùng chọn "Để sau"**: có nhắc lại sau N ngày không? Spec này đề xuất chỉ nhắc ở Similar.
5. **Từ vựng chip trục** cần phỏng vấn 2–3 designer (research 06 §2.4) trước khi chốt chữ trong UI.
