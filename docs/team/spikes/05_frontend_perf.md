# Spike 05 — Hiệu năng lưới font (Frontend Performance)

> Ngày 2026-10-15 · Thành viên: Frontend Performance · Gửi: FontAtlas · Lead
> Loại: **SPIKE**. Code bỏ đi nằm trong worktree `priceless-elgamal-c44134` tại `spikes/frontend-perf/` (chưa commit, chưa push; không sửa `UX/`).
> Quy ước: **[Đo]** = số liệu đo thật trong spike này · **[Suy luận]** = ước tính/lập luận, chưa đo.

## TL;DR

1. **Cách hiện tại (a) không chịu được thư viện lớn.** Trên WKWebView (engine Tauri dùng trên macOS), khi cuộn nhanh lưới 2.000 thẻ chỉ đạt **~11 FPS** (5.000 thẻ: **~6 FPS**), 95% ô trong khung nhìn bị trống, tiến trình WebContent lên **~2,7 GB**. [Đo]
2. **Virtualization + FontFace có cache (b)** giúp cuộn mượt (56–60 FPS), nhưng **bộ nhớ vẫn ~2,6 GB**: giới hạn LRU theo *số lượng* (400 face) là vô nghĩa vì file font to từ vài chục KB đến hàng chục MB. Bản cải tiến **b2** (ngân sách theo byte + chỉ tải khi tốc độ cuộn < 1.500 px/s) giảm xuống **~745 MB**, nhưng khi cuộn nhanh **97% ô trống**. [Đo]
3. **Virtualization + preview PNG dựng sẵn (c)** tốt nhất ở mọi chỉ số: hiển thị đầu **~140–150 ms**, **60 FPS**, **0% ô trống**, **~260–290 MB**, không giữ FontFace nào. Chỉ khi đổi chữ mẫu mới chậm hơn: ~100 ms (so với ~50 ms của b), vẫn dưới ngưỡng cảm nhận. [Đo]
4. **Khuyến nghị: chọn (c)** và dùng preview dạng **PNG alpha-mask** (đổi màu theo theme bằng `mask-image`). Chỉ tải FontFace thật cho font đang chọn, hover hoặc mở trong DetailPanel, qua bộ nạp kiểu b2. Preview sinh trong Rust (skrifa + tiny-skia): **~0,67 ms CPU/face**. Cache cho 10k font khoảng **~45–55 MB**.
5. **Hai phát hiện phụ ảnh hưởng đến thiết kế:**
   - Chromium (tức WebView2 trên Windows) **từ chối ~2,2%** font hệ thống macOS khi tải qua FontFace (OTS). [Đo]
   - **52% face trong pool nằm ở vị trí ≥1 trong file TTC.** FontFace không chọn được face index, nên cách hiện tại hiển thị sai các face này. [Suy luận từ API, chưa kiểm bằng ảnh]
   - Preview dựng bằng Rust tránh được cả hai lỗi trên.

---

## 1. Cấu hình đo

| Mục | Giá trị |
|---|---|
| Máy | Apple **M4 Pro**, 12 nhân, **24 GB** RAM, macOS **27.2** (26B5101f) |
| Engine chính | **WKWebView** của hệ thống (AppleWebKit/605.1.15). Harness Swift `harness/wk.swift` dùng cửa sổ 1280×800, `websiteDataStore = .nonPersistent()` (không có HTTP cache giữa các lần chạy), tắt phát hiện cửa sổ bị che |
| Engine phụ | **Chrome 154 headless** (`--headless=new`), coi như đại diện cho WebView2 trên Windows (cùng là Chromium nhưng khác bản, khác OS) |
| Frontend | React 19.3.1, react-window **2.3.4** (`List` theo hàng, `overscanCount=2`), Vite 6.4.4 dev server |
| Lưới | 4 cột, hàng cao 120 px, vùng preview 320×56 css px, cỡ chữ 36 px. Chữ mẫu “Hamburgefonts Ăn phở”, đổi sang “Đổi chữ mẫu 2026” |
| Dữ liệu font | `/System/Library/Fonts` (gồm cả Supplemental) + `/Library/Fonts`: **369 file → 786 face**. Đã loại 6 file > 40 MB (Apple Color Emoji 183 MB, Songti, STHeiti, AppleSDGothicNeo…), còn **pool 754 face**. Thẻ thứ i dùng face `i % 754` nhưng **mỗi thẻ có family/URL riêng** để mô phỏng 2.000–5.000 font khác nhau |
| Giả lập backend | `GET /font/:id` trả nguyên bytes của file (thay cho `readFile` qua IPC), `GET /png|svg/:id` trả preview dựng sẵn, `GET /render/:id.png?text=` gọi binary Rust render theo yêu cầu (mỗi lần một tiến trình, ~3 ms) |
| Kịch bản | (1) mở trang → đợi mọi thẻ trong khung nhìn sẵn sàng; (2) cuộn **nhanh 4.000 px/s** trong 6 s; (3) cuộn **chậm 600 px/s** trong 6 s; (4) dừng 1,5 s rồi đo bộ nhớ; (5) đổi chữ mẫu → đợi khung nhìn vẽ xong (2 rAF) |
| Lặp | WKWebView **3 lần**/ô, Chromium **2 lần**/ô; bảng dưới ghi **trung vị** |

**Định nghĩa chỉ số**

| Chỉ số | Cách đo |
|---|---|
| Hiển thị đầu | Thời gian từ khi điều hướng đến khi mọi thẻ trong viewport có font/ảnh thật |
| FPS / p95 | Đếm số rAF và khoảng cách giữa hai frame trong lúc cuộn |
| % ô trống | Tỉ lệ (thẻ × frame) trong viewport còn hiển thị fallback (chưa có font/ảnh) |
| Bộ nhớ WK | `phys_footprint` của tiến trình WebContent (công cụ `footprint`) |
| Bộ nhớ Chromium | RSS của renderer |
| Đổi chữ | Thời gian từ `setText` đến khi viewport vẽ xong |

### Các chế độ

| Mã | Mô tả |
|---|---|
| **a** | Như `UX/` hiện tại: render toàn bộ DOM (tương đương trạng thái sau khi đã tải hết các trang), mỗi thẻ một `IntersectionObserver`, `fetch` → Blob → `FontFace`, không dedupe, không bao giờ gỡ |
| **b** | react-window v2 + cache `Map` (dedupe), tối đa 6 lượt tải đồng thời, huỷ khi unmount, LRU **400 face** (`document.fonts.delete`) |
| **b2** | Như b, nhưng LRU theo **ngân sách 128 MB** và chỉ bắt đầu tải khi tốc độ cuộn < 1.500 px/s hoặc đã ngừng cuộn 120 ms |
| **c** | react-window v2 + `<img>` PNG dựng sẵn (640×112, grayscale+alpha). Khi hover mới tải FontFace thật (dùng bộ nạp b2) |
| **csvg** | Như c, nhưng preview là file SVG path |

---

## 2. Kết quả [Đo]

### 2.1 WKWebView (engine của Tauri trên macOS) — trung vị 3 lần chạy

| n | Chế độ | Hiển thị đầu (ms) | Cuộn nhanh: FPS / p95 frame (ms) / % ô trống | Cuộn chậm: FPS / % ô trống | Đổi chữ (ms) | Bộ nhớ WebContent | Số FontFace còn giữ / tổng MB font đã tải | DOM nodes |
|---|---|---|---|---|---|---|---|---|
| 2.000 | a | 555 | **11,3** / 183 / 95% | 11 / 34% | 165 | **2.707 MB** | 956 / 1.650 MB | 6.512 |
| 2.000 | b | 332 | 56,5 / 31 / 16% | 60 / 0,2% | **49** | 2.593 MB | 400 / 1.633 MB | 170 |
| 2.000 | b2 | 361 | 60 / 18 / **97%** | 60 / 2,9% | 50 | 748 MB | 81 / 427 MB | 170 |
| 2.000 | **c** | **139** | **60 / 18 / 0%** | 60 / 0% | 100 | **287 MB** | 0 / 0 | 170 |
| 2.000 | csvg | 158 | 60 / 18 / 0% | 60 / 0% | 99 | 351 MB | 0 / 0 | 170 |
| 5.000 | a | 1.108 | **6** / 346 / 92% | 6,5 / 63% | 232 | 2.568 MB | 864 / 1.465 MB | 16.262 |
| 5.000 | b | 410 | 56,5 / 32 / 18% | 60 / 0% | 65 | 2.580 MB | 400 / 1.636 MB | 157 |
| 5.000 | b2 | 323 | 60 / 18 / 97% | 60 / 2,8% | 49 | 744 MB | 81 / 427 MB | 170 |
| 5.000 | **c** | **153** | **60 / 18 / 0%** | 60 / 0% | 99 | **264 MB** | 2 / 1,6 MB | 170 |
| 5.000 | csvg | 153 | 60,2 / 18 / 0% | 60 / 0% | 100 | 352 MB | 0 / 0 | 170 |

### 2.2 Chrome 154 headless (đại diện WebView2) — trung vị 2 lần chạy

| n | Chế độ | Hiển thị đầu (ms) | Cuộn nhanh: FPS / p95 / % ô trống | Cuộn chậm: % ô trống | Đổi chữ (ms) | RSS renderer | JS heap | Font bị từ chối |
|---|---|---|---|---|---|---|---|---|
| 2.000 | a | 756 | 41,5 / 41,7 / 67% | 8% | 91 | 1.082 MB | 65 MB | 21 |
| 2.000 | b | 761 | 54,4 / 25 / 15% | 2,4% | 32 | 962 MB | 43 MB | 21 |
| 2.000 | b2 | 646 | 60 / 16,8 / 97% | 3% | 33 | 538 MB | 69 MB | 5 |
| 2.000 | c | 290 | 60 / 16,8 / 0% | 0% | 182* | 220 MB | 9 MB | 0 |
| 2.000 | csvg | 316 | 60 / 16,8 / 0% | 0% | 108 | 330 MB | 7 MB | 0 |
| 5.000 | a | 1.393 | **15** / 108 / 96% | 34% | 168 | 1.512 MB | 69 MB | 21 |
| 5.000 | b | 779 | 53,8 / 16,8 / 14% | 2,2% | 33 | 1.378 MB | 48 MB | 21 |
| 5.000 | b2 | 728 | 60 / 16,8 / 97% | 2,6% | 32 | 597 MB | 77 MB | 5 |
| 5.000 | c | 206 | 60 / 16,8 / 0% | 0% | 82 | 198 MB | 10 MB | 0 |
| 5.000 | csvg | 229 | 60 / 16,8 / 0% | 0% | 74 | 299 MB | 6 MB | 0 |

\* Hai lần đo cho 215 ms và 148 ms. Lần đầu bị chậm vì tiến trình Rust khởi động lạnh.

### 2.3 Đọc số liệu

- **Mô hình (a) chết vì main thread**, không phải vì I/O. DOM có 6–16k node, mỗi thẻ một observer, và mỗi lần `FontFace` nạp xong thì WebKit phải relayout/repaint. Kết quả: p50 frame 80–100 ms trên WebKit. WebKit tệ hơn Chromium rõ rệt (11 FPS so với 41 FPS ở 2.000 thẻ). **Đo trên Chrome sẽ đánh giá quá lạc quan so với app Tauri thật trên macOS.** [Đo]
- **Thời gian đổi chữ của (a) tăng tuyến tính theo số thẻ** (165 → 232 ms), vì cả grid re-render. [Đo]
- **Bộ nhớ không được trả lại khi gỡ FontFace trên WebKit.**
  - Ở b2 chỉ còn 81 FontFace sống (đã evict 778), nhưng footprint vẫn 745 MB.
  - Ở b, footprint ~2,6 GB với 400 face, xấp xỉ **1,5× tổng số byte font đã từng tải**.
  - Trong một lần chạy thử b2 bản đầu (gating theo thời gian chờ thay vì theo tốc độ): footprint 2,3 GB dù chỉ còn 62 face.

  [Đo]. *Nguyên nhân* (GC chưa chạy, cache font nội bộ của WebKit/CoreText, hay allocator không trả trang về hệ thống) chưa xác định. [Suy luận] **Hệ quả thiết kế: không thể dựa vào việc gỡ FontFace để kiểm soát bộ nhớ, phải tránh tải ngay từ đầu.**
- **b2 cho thấy giới hạn của mọi cách dùng FontFace.** Muốn không trống ô khi cuộn nhanh thì phải tải mọi thứ lướt qua (b → GB bộ nhớ). Muốn tiết kiệm thì phải chấp nhận ô trống. (c) không phải đánh đổi như vậy.
- **Mỗi lần tải FontFace là đọc nguyên file**: trung vị 170 KB/file, p90 2,3 MB, có file hơn 20 MB. Cuộn 12 giây đã kéo về **1,6 GB**. Qua IPC `readFile` thật của Tauri thì còn tốn thêm chi phí sao chép/serialize. [Đo kích thước; chi phí IPC là Suy luận]
- **PNG tốt hơn SVG** về bộ nhớ (264–287 MB so với 343–360 MB trên WK). FPS ngang nhau. SVG có những file phức tạp tới 148 KB. [Đo]
- **Chromium/OTS từ chối 21 font** trong ~950 lần tải (~2,2%). WebKit từ chối 0. Trên Windows (WebView2), FontFace sẽ hỏng với một phần thư viện của người dùng. Preview dựng bằng skrifa thì không gặp vấn đề này. [Đo trên font macOS; mức độ với font Windows là Suy luận]
- **TTC**: 513/754 face nằm trong file `.ttc`, trong đó **390 face có index ≥ 1**. Cả `FontFace(url)` lẫn `FontFace(buffer)` đều không có tham số face index. [Suy luận: theo API, chưa chụp ảnh so sánh] Vì vậy `useFontLoader` hiện tại hiển thị face 0 cho mọi face trong cùng file (ví dụ cả Helvetica Bold lẫn Helvetica Oblique đều hiện thành Helvetica Regular). skrifa thì render đúng face theo index. [Đo: preview (c) đúng face]

---

## 3. Câu 2 — Cache preview cho 10k font

### 3.1 Số đo bộ sinh (Rust: skrifa 0.48 + tiny-skia 0.12 + png 0.18, `spikes/frontend-perf/gen`) [Đo]

| Hạng mục | Kết quả trên 786 face |
|---|---|
| Liệt kê + đếm face (369 file) | 28–67 ms khi file đã ở page cache. Lần chạy lạnh đầu tiên: 369–416 ms |
| Render đơn luồng | **528 ms → 0,67 ms/face** (gồm đọc file, parse, outline, raster, nén PNG) |
| Render 12 luồng (rayon) | **100–142 ms** cho toàn bộ |
| Render 1 face theo yêu cầu (spawn tiến trình + render) | trung vị **3,0 ms**, p90 3,6 ms |
| PNG 640×112 grayscale+alpha (2× vùng 320×56), nén `Fast` | trung bình **4,2 KB**, trung vị 5,2 KB, p90 8,0 KB, max 14,7 KB |
| SVG path (toạ độ 1 chữ số thập phân) | trung bình 6,2 KB, p90 13,4 KB, **max 148 KB**. Gzip: trung bình **2,2 KB** |
| Overhead khi lưu mỗi preview một file | 786 file: 3,3 MB dữ liệu nhưng `du` = 5,1 MB (**+55%**, do block 4 KB) |
| Face thiếu glyph cho chữ “Hamburgefonts Ăn phở” | **635/786**. Riêng “Hamburgefonts”: 331/786 (chủ yếu Noto Sans <script>, font CJK, font ký hiệu) |

### 3.2 Ước tính cho 10k font [Suy luận, ngoại suy tuyến tính từ số đo trên]

| Định dạng | Dung lượng 10k face |
|---|---|
| PNG 2× (khuyến nghị) | **~42–52 MB** |
| PNG 1× (320×56) | ~15–20 MB (ước tính: số pixel giảm 4×, nén kém hơn một chút) |
| SVG thô / gzip | ~62 MB / ~22 MB |

Thời gian dựng toàn bộ: **~6,7 s đơn luồng hoặc ~1,3–2 s trên 12 luồng** với file đã nằm ở page cache. Đọc lạnh từ HDD/NAS sẽ chậm hơn nhiều, nên chạy trong luồng nền, cùng hàng đợi với bước “phân tích nhanh” của Phần 2.

### 3.3 Đề xuất thiết kế

1. **Định dạng: PNG grayscale+alpha, dùng như alpha-mask.**
   - FE vẽ bằng `background: currentColor; mask-image: url(...)`, nên một ảnh dùng được cho cả light/dark theme và màu nhấn.
   - Dựng ở **2× kích thước lớn nhất** của vùng preview, theo vài *bucket* cỡ chữ (ví dụ 48/96 px), ứng với slider số cột 1–8 (mục B trong 01_frontend).
   - SVG chỉ nên dùng cho chế độ phóng to hoặc xuất bản.
2. **Lưu trong SQLite** (một bảng blob) hoặc một file pack, **không lưu mỗi ảnh một file**. Gợi ý: `previews(face_hash, text_hash, px, renderer_version, png BLOB, PRIMARY KEY(face_hash,text_hash,px))`. Khoá theo **mã băm BLAKE3** (Phần 1), nên rescan và đổi đường dẫn đều không làm mất cache.
3. **Pipeline Rust:**
   - skrifa `outline_glyphs()` → `OutlinePen` → `tiny_skia::PathBuilder` → `fill_path` (anti-alias) → lấy kênh alpha → PNG `GrayscaleAlpha`. Toàn bộ khoảng 150 dòng (xem `gen/src/main.rs`).
   - **Phải bổ sung:**
     - (i) **Shaping.** Spike chỉ dùng cmap + advance, không kerning, không GSUB, nên sai với Ả Rập/Ấn và dấu tiếng Việt dạng tổ hợp. Đề xuất `harfrust` (bản port HarfBuzz chạy trên fontations) hoặc `rustybuzz`. [Suy luận: cần spike nhỏ để chọn]
     - (ii) **Chọn chữ mẫu theo độ phủ cmap**: tiếng Việt → Latin → mẫu theo script chính (từ OS/2 `ulUnicodeRange` hoặc cmap). Lý do: 635/786 face thiếu ít nhất một glyph của câu mẫu tiếng Việt.
     - (iii) **Font bitmap/màu** (sbix/CBDT/COLR, ví dụ Apple Color Emoji) không có outline, nên hiện tại ra ảnh trống. Cần đánh dấu “không có preview” hoặc dùng `skrifa` ColorPainter cho COLR.
     - (iv) **Variable font**: render ở default instance, mỗi named instance là một entry riêng (`LocationRef`).
4. **Khi người dùng đổi văn bản mẫu:**
   - FE debounce ~150 ms rồi gửi `set_preview_text(text, px)`. Backend tăng **generation token** (huỷ các job cũ).
   - **Ưu tiên các id đang trong viewport** (FE gửi danh sách khi `onRowsRendered`), sau đó đến overscan. Không dựng trước cả 10k cho chữ tuỳ chỉnh; các thẻ khác chỉ được dựng khi cuộn tới.
   - Số đo: render một face ~0,7 ms CPU in-process. Khung nhìn ~40 thẻ trên 6+ luồng nên mất **< 20 ms**. [Suy luận từ 3.1] Spike đo được ~100 ms đầu-cuối vì mỗi ảnh spawn một tiến trình và đi qua HTTP. [Đo]
   - Chữ mẫu mặc định: cache **bền** trên đĩa. Chữ tuỳ chỉnh: **LRU** khoảng 64 MB trong bộ nhớ hoặc trên đĩa, giữ N chuỗi gần nhất.
   - Phương án thay thế để đổi chữ thấy ngay (< 50 ms): tạm dùng FontFace (bộ nạp b2) cho ≤ 40 thẻ trong viewport, rồi thay bằng ảnh khi có. Phương án này thêm độ phức tạp và lại vướng TTC/OTS, nên **không khuyến nghị** khi 100 ms đã đạt.

---

## 4. Câu 3 — Ghi chú IPC Tauri

1. **Một lệnh truy vấn, lọc và sắp xếp ở SQLite**, thay cho `get_library_fonts` + lọc phía client (P0-3 của 01_frontend):
   ```
   query_fonts({ q, classes[], tags[], metrics:{xHeight:[a,b],…}, favorites, sourceId, sort, requestId })
     -> { requestId, total, ids }   // ids: toàn bộ id khớp, đã sắp xếp
   get_font_rows({ ids: [..≤200] }) -> SlimRow[]   // chỉ các hàng sắp hiển thị
   ```
   - **Trả toàn bộ danh sách id đã lọc trong một lần.** Với 10k font: JSON số khoảng 60–80 KB, hoặc 40 KB nếu dùng `Uint32` thô qua `tauri::ipc::Response`. Nhờ vậy `rowCount` đúng ngay, thanh cuộn đúng, không còn vòng lặp `loadMore` (P0-1/P0-2). [Suy luận]
   - **Hàng gọn (`SlimRow`)**: `{id, family, style, weight, italic, classId, flags, previewKey}`. Bản 3 trường của spike đo được **61 B/hàng JSON** [Đo]; bản đầy đủ ước khoảng 120–150 B. Hàng `FontRecord` hiện tại (~35 trường, có `file_path`, metrics, ngày tháng) chỉ nên lấy khi mở DetailPanel bằng `get_font_by_id`.
   - **`requestId` + debounce 150 ms** ở FE để bỏ các phản hồi đã cũ khi người dùng gõ nhanh.
   - Lọc theo khoảng metric cần index (hoặc bảng phân vị đã chuẩn hoá 0–1 theo Phần 3). 10k hàng thì quét toàn bộ vẫn chỉ vài ms. [Suy luận]
2. **Ảnh preview không đi qua `invoke`.** Đăng ký **custom URI scheme** (`register_asynchronous_uri_scheme_protocol`), ví dụ `fontatlas://preview/{face_hash}/{text_hash}/{px}.png`. Trên Windows/Android URL có dạng `http://fontatlas.localhost/...`, phải xử lý khác biệt này.
   - Lợi ích: `<img>`/`mask-image` tự cache, decode ngoài main thread, huỷ được khi unmount, không có base64 hay JSON. [Suy luận theo kiến trúc. Spike đã chứng minh đường HTTP-ảnh cho 60 FPS, nhưng chưa đo bản custom protocol]
   - Cần thêm scheme này vào CSP (`img-src`). CSP hiện tại mới cho phép `asset:`.
3. **FontFace thật** (font đang chọn, hover, DetailPanel, Pairing/Similar preview):
   - Dùng **asset protocol** (`convertFileSrc(path)` → `new FontFace(name, url(...))`) để WebView tự đọc file, không chép bytes vào JS heap. Nếu cần thì trả bytes thô bằng `tauri::ipc::Response`, không trả `Vec<u8>` dạng JSON.
   - Với **TTC**, backend phải tách face thành một sfnt độc lập (ví dụ dùng `write-fonts`/`read-fonts` sao chép bảng), hoặc dùng ảnh. [Suy luận: cần spike]
   - Bộ nạp: cache `Map<faceHash, Promise>`, tối đa 6 lượt tải đồng thời, **ngân sách theo byte**, chỉ tải khi tốc độ cuộn thấp (b2). Vì WebKit không trả bộ nhớ ngay (mục 2.3), giữ số FontFace tải mỗi phiên ở mức nhỏ.
4. **Tiến độ scan/analyze/preview**: dùng `tauri::ipc::Channel` theo lô (ví dụ mỗi 100 ms hoặc mỗi 200 item), không phát event cho từng font. Event `rescan-progress` cho từng font hiện đang làm App re-render toàn bộ (01_frontend). FE nhận `{phase, done, total, changedIds[]}`, rồi chỉ invalidate các preview/hàng bị đổi.
5. **Bỏ N lần IPC cho mỗi thẻ** (`readFile` cho từng card, `add_font_to_library` tuần tự). Nguyên tắc: mỗi lần cuộn tối đa một `get_font_rows`; ảnh đi qua protocol.

---

## 5. Khuyến nghị

1. **Lưới = react-window v2 `List` theo hàng** (số cột lấy từ slider; `rowHeight` tính từ số cột) + **`React.memo` FontCard** + **preview PNG alpha-mask từ cache Rust**.
   - Card mặc định không tạo FontFace.
   - Khi chưa có preview (font mới quét), hiển thị tên font bằng font hệ thống kèm khung xám. Không hiện “Loading…” vô hạn.
2. **FontFace chỉ dùng cho tương tác**: hover (sau ~150 ms), font đang chọn, DetailPanel, so sánh pairing. Dùng bộ nạp b2: dedupe, 6 đồng thời, ngân sách byte ~128 MB, chỉ tải khi tốc độ cuộn thấp, đọc qua asset protocol.
3. **Backend**: `query_fonts` → ids, `get_font_rows`, `get_font_by_id`, custom protocol cho preview, job dựng preview ưu tiên theo viewport với generation token. Thêm shaping (harfrust/rustybuzz) và logic chọn chữ mẫu theo cmap trước khi chốt.
4. **Đo trên WebKit/WKWebView**, không chỉ trên Chrome: Chrome cho kết quả lạc quan gấp ~3–4 lần ở chế độ (a). Đưa harness `wk.swift` + `run_all.py` vào CI như một perf smoke test (ngưỡng gợi ý: ≥55 FPS cuộn nhanh, < 500 MB footprint với 5k font). Cần thêm lần đo trên **WebView2/Windows thật**.
5. **Thứ tự triển khai**: phần virtualization (mục C của 01_frontend) có thể làm ngay với (b2) làm bước đệm. Khi backend có preview thì chuyển sang (c). Toàn bộ code phía card đều nằm sau một hook `usePreview(faceId)`, nên việc chuyển đổi không đụng tới grid.

## 6. Rủi ro

| Rủi ro | Mức | Giảm thiểu |
|---|---|---|
| Preview Rust khác cách WebView render (hinting, gamma, shaping, OpenType features) → thẻ khác chữ thật | TB | Không hinting, cùng px, cho phép xem live bằng FontFace khi hover. Thêm shaping. Có bộ ảnh golden để so sánh |
| Thiếu shaping hoặc chữ mẫu sai script → thẻ trống hoặc tofu (635/786 face thiếu glyph của câu mẫu tiếng Việt) | Cao | Chọn chữ mẫu theo cmap. Hiển thị nhãn “không hỗ trợ tiếng Việt” (đã có cờ `supports_vietnamese`) |
| Font màu/bitmap (emoji, sbix) → không có outline | Thấp | Đánh dấu “không có preview”, hoặc dùng COLR painter |
| Cache lỗi thời khi file đổi hoặc renderer đổi | TB | Khoá theo BLAKE3 + `renderer_version`. Xoá cache bẩn trong nền |
| Đổi chữ tuỳ chỉnh với 10k font tốn CPU | Thấp | Chỉ dựng cho viewport, debounce, huỷ theo generation |
| Bộ nhớ WebKit không giảm sau khi gỡ FontFace | Cao (đã đo) | Giới hạn số lần tải FontFace mỗi phiên. Cân nhắc reload webview trong trường hợp đặc biệt. Cần đo dài hơn (vài phút) để biết bộ nhớ có được thu hồi về sau không |
| Windows/WebView2 (OTS) từ chối font; khác biệt đường dẫn custom protocol | TB | Preview ảnh không phụ thuộc OTS. Test riêng trên Windows. Fallback hiển thị ảnh khi FontFace lỗi |
| TTC: FontFace không chọn được face | TB (52% face trong pool) | Ảnh preview đúng face. Tách sfnt cho FontFace (spike riêng) |
| Số đo chỉ trên M4 Pro (máy mạnh); máy Intel/Windows yếu có thể tệ hơn nhiều với (a)/(b) | TB | Lặp lại harness trên máy yếu; giữ (c) làm mặc định |

## 7. Giới hạn của spike

- Spike **không chạy trong Tauri thật**: IPC `readFile`, asset protocol và custom protocol được giả lập bằng HTTP của Vite. Hướng sai lệch dự kiến: (a)/(b) sẽ tệ hơn trong Tauri (vì chi phí IPC), còn (c) giữ nguyên hoặc tốt hơn (vì render in-process). [Suy luận]
- Pool chỉ có 754 face khác nhau, mỗi face lặp lại khoảng 3–7 lần dưới tên family khác nhau. Font hệ thống macOS to hơn font Latin thông thường của designer (đa số CJK/Noto). Bộ nhớ tuyệt đối của (a)/(b) với thư viện thật toàn font Latin nhỏ có thể thấp hơn, nhưng xu hướng tăng tuyến tính theo số font đã duyệt không đổi.
- Không đo mức tiêu thụ GPU process, và không đo trong khoảng thời gian dài (rò rỉ theo phút/giờ).
- Code: `spikes/frontend-perf/{gen,web,harness}`, số liệu thô ở `harness/runs.jsonl` (50 lần chạy), log ở `harness/log_*.txt`. Chạy lại: `cargo build --release` trong `gen`, chạy `gen ../out "Hamburgefonts Ăn phở" 48`, khởi động Vite (`npx vite` trong `web`, cổng 5199), rồi `python3 harness/run_all.py a,b,b2,c,csvg 2000,5000 wkwebview,chromium 3`.
