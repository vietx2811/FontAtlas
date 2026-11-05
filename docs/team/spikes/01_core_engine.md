# Spike 01 — Core Engine (Rust): fontations + kurbo thay phần Python?

Ngày: 2026-10-15 · Người làm: Core Engine (Rust) · Trạng thái: SPIKE (code bỏ đi)

- Code spike: worktree `claude/gracious-dijkstra-011ba7`, thư mục `spikes/core-engine/` (khoảng 790 dòng Rust, chưa commit, không đụng `UX/` hay `src/`).
- Máy đo: Apple M4 Pro, 12 nhân (8P), macOS, `cargo 1.96`, build `--release`.
- Ký hiệu: **[ĐÃ CHẠY]** = có số liệu từ lần chạy thật trong spike; **[SUY LUẬN]** = dựa trên đọc tài liệu/mã, chưa kiểm chứng.

---

## 0. Kết luận và khuyến nghị

1. **Có, thay được.** read-fonts + skrifa + kurbo + wuff đọc được mọi thứ phần Python đang đọc, và đọc thêm những gì Python chưa có: TTC, variable font, WOFF2. Không cần Python lúc chạy. [ĐÃ CHẠY] Đã chạy trên 375 file / 792 face (thư mục hệ thống macOS + 5 font mẫu): 0 lỗi, 0 panic.
2. **Kiểm tra tiếng Việt mới đúng và chi tiết hơn.** Có liệt kê từng ký tự thiếu, phân biệt glyph dựng sẵn với ghép dấu (GPOS `mark`/`mkmk`, GSUB `ccmp`). [ĐÃ CHẠY] Roboto, Open Sans, Playfair Display và Source Code Pro đều đủ 134/134 ký tự; Python báo `False` cho cả bốn. Trên 792 face, quy tắc Python cũ sai 89 face: 83 âm tính giả và 6 dương tính giả.
3. **Đo hình học thật trên outline, không cần raster.** Đo được x-height, cap-height, độ đậm của thân nét, độ phủ mực, stroke contrast (hai cách đo), độ rộng, độ nghiêng, chân serif, counter và aperture của `c`. Có hai số còn yếu: trục nhấn (stress) và aperture của `e`. Chi tiết ở mục 3.
4. **Đủ nhanh để phân tích ngay khi quét.** [ĐÃ CHẠY] Phân tích đủ cả metadata, tiếng Việt và hình học cho 792 face: **1.6–1.8 s đơn luồng, 0.24–0.31 s với rayon** (×5.9–6.5). Riêng metadata và tiếng Việt: 0.45 s đơn luồng, 0.11–0.16 s với rayon.
5. **Khuyến nghị:** dựng crate `fontatlas-core` trên read-fonts/skrifa/kurbo/wuff/blake3/rayon theo API ở mục 6. Không cần thêm crate shaping hay raster cho lõi phân tích. Có hai việc bắt buộc phải làm trước khi tin vào số đo: (a) bước kiểm tra toàn vẹn file (mục 5, R1), và (b) đo variable font ở một vị trí chuẩn chứ không đo ở instance mặc định (mục 3.3).

---

## 1. Bảng crate

| Việc | Crate (phiên bản đã dùng) | Ghi chú | Trạng thái |
|---|---|---|---|
| Đọc bảng thô (name, OS/2, head, post, maxp, GSUB/GPOS feature list, glyf/loca, fvar/avar/STAT) | `read-fonts 0.45.1` | Zero-copy, đọc lười, có fuzz (Google Fonts/Chrome dùng) | [ĐÃ CHẠY] |
| API cấp cao: charmap, outline, metrics, trục VF, named instances, localized strings, attributes | `skrifa 0.48.0` | `MetadataProvider`; `OutlinePen` vẽ ở bất kỳ `Location` VF nào | [ĐÃ CHẠY] |
| Hình học: BezPath, giao đường thẳng, winding, affine | `kurbo 0.13.1` | `PathSeg::intersect_line`, `Shape::winding`, `Affine` | [ĐÃ CHẠY] |
| Giải nén WOFF2 (+ WOFF1) | `wuff 0.2.9` | Rust thuần (brotli-decompressor + flate2), một hàm `decompress_woff2(&[u8]) -> Vec<u8>` | WOFF2 [ĐÃ CHẠY] trên 1 file VF thật; WOFF1 chưa có mẫu để thử |
| TTC / OTC | `read-fonts::FileRef::Collection` | Không cần crate thêm | [ĐÃ CHẠY] 545/792 face nằm trong TTC |
| Băm danh tính | `blake3 1.8.7` | Băm file gốc (trước khi giải nén WOFF) | [ĐÃ CHẠY] |
| Song song | `rayon 1.12` | `par_iter` theo file | [ĐÃ CHẠY] |

Các lựa chọn khác đã cân nhắc (đều chưa chạy):
- `woff2-patched`, `woff2-no-std`, `oxifont-webfont`: chỉ giải nén, ít người dùng hơn `wuff`.
- `ttf2woff2`: chỉ nén, không cần cho FontAtlas.
- `allsorts` / `ttf-parser`: ttf-parser không vẽ được outline ở vị trí VF tuỳ ý dễ như skrifa, cũng không có API GSUB/GPOS thô tương đương. Tôi không chọn để giữ một hệ sinh thái duy nhất (fontations).

Footprint [ĐÃ CHẠY]: 7 dependency trực tiếp, 51 crate trong cây phụ thuộc. Binary spike release nặng 1.8 MB.

---

## 2. Tiếng Việt

### 2.1 Định nghĩa đã dùng (`spikes/core-engine/src/vi.rs`)
- **Bộ bắt buộc, 186 ký tự:** 67 chữ thường có dấu hoặc đặc thù Việt (12 nguyên âm cơ sở × 5 thanh, cộng ă â ê ô ơ ư đ), 67 chữ hoa tương ứng, và A–Z a–z. Bộ này gồm trọn U+1EA0–U+1EF9, cùng Ă ă Đ đ Ĩ ĩ Ũ ũ Ơ ơ Ư ư và các ký tự Latin-1 có dấu.
- **Ghép dấu (NFD):** cần đủ 24 chữ cơ sở, đủ 5 dấu kết hợp U+0300/0301/0303/0309/0323 và có GPOS `mark`. Ghi thêm `mkmk` (cần cho dấu chồng như ấ ổ) và `ccmp`.
- **Trạng thái:** `full` (đủ dựng sẵn + ghép được) / `precomposed_only` / `marks_only` / `none_or_partial`. Có thêm danh sách `missing`, số glyph dựng sẵn là composite (glyf), và có ₫ hay không.
- `supports_vietnamese` = đủ dựng sẵn. Đây là thứ quyết định hiển thị đúng văn bản NFC thông thường.

### 2.2 Năm font mẫu [ĐÃ CHẠY]

| Font | Python cũ (tỉ lệ / kết luận) | Mới: trạng thái | Thiếu | Composite / dựng sẵn | GPOS mark/mkmk | ₫ |
|---|---|---|---|---|---|---|
| Roboto | 0.869 / False | `precomposed_only` → **True** | 0 | 128/134 | không / không | có |
| Open Sans | 0.869 / False | `full` → **True** | 0 | 129/134 | có / có | có |
| Playfair Display | 0.869 / False | `full` → **True** | 0 | 130/134 | có / có | không |
| Source Code Pro | 0.869 / False | `full` → **True** | 0 | 129/134 | có / có | có |
| Libre Baskerville | 0.879 / False | `none_or_partial` → **False (đúng)** | 68: toàn bộ dấu hỏi, ơ ư và mọi biến thể | 64/66 | có / có (nhưng thiếu U+0309 nên không ghép được) | không |

- Tỉ lệ 0.869 và 0.879 mà spike tái tạo khớp đúng con số trong audit 03. Vậy tôi đã tái tạo đúng quy tắc cũ.
- Nguyên nhân lỗi xác nhận từ mã (`font_parser.py:827-852`): tập cũ thiếu U+1EA0–1EF9, thiếu Đ đ Ĩ ĩ Ũ ũ. `range(0x102,0x103)` chỉ lấy Ă, `range(0x1A0,0x1B0)` bỏ sót ư. Ngoài ra tập cũ chứa cả × ÷ Æ Ø Þ ß…, nên điểm số nghiêng về font Tây Âu.
- Libre Baskerville trong repo **thực sự không hỗ trợ tiếng Việt**. Python ra đúng `False`, nhưng vì lý do sai.

### 2.3 Toàn bộ font hệ thống macOS: 792 face [ĐÃ CHẠY]
- Trạng thái: `full` 26, `precomposed_only` 116, `none_or_partial` 650. Nhóm cuối phần lớn là font CJK, Ấn, Ả Rập…; 331 face không có chữ Latin thường.
- So với quy tắc Python cũ: cùng True 59, cùng False 644, **dương tính giả 6, âm tính giả 83**.
  - Dương tính giả: Menlo (thiếu 46 ký tự), STIXGeneral (thiếu 88).
  - Âm tính giả, tức font hỗ trợ đủ nhưng Python báo không: Avenir Next (mọi độ đậm), Helvetica Neue, Georgia, Cochin, Big Caslon, Gill Sans (một số face), DIN Condensed, SF (New York, SF Compact, SF Mono, SF Rounded)…
- **"Gần đủ" là trường hợp thực tế cần hiện trên UI**, ví dụ: Helvetica Light thiếu `ởửỞỬ`, Palatino thiếu `ớ`, Palatino Bold Italic thiếu `ờớ`, DIN Alternate Bold thiếu `đ`, Superclarendon thiếu `ỷỹỵỶỸỴ`. Một cờ boolean không nói được điều này. Đề xuất UI: "Thiếu 4 ký tự: ở ử Ở Ử".
- Trong các face đạt `supports_vietnamese`, trung bình 64% glyph dựng sẵn là composite (glyf tham chiếu glyph gốc + dấu). Các face còn lại có outline phẳng, hoặc là CFF, vốn không có composite.

### 2.4 Chưa làm [SUY LUẬN]
- **Chất lượng đặt dấu** (dấu chồng ế ộ ữ có đè nhau không, có tràn ascender không): cần đo bbox các glyph như `ế` so với `ê` và ascender, hoặc shape bằng `harfrust`/`rustybuzz` khi có NFD. Có thể làm bằng hình học như mục 3 mà không cần raster.
- Kiểm tra theo `cmap` không đảm bảo font có GPOS anchor đúng cho chữ thường có dấu. Việc này cần kiểm tra mức anchor.

---

## 3. Trục đo từ outline

Cách đo chung ([ĐÃ CHẠY], `src/geom.rs`):
1. Vẽ glyph bằng `skrifa` (unhinted, đơn vị font) vào `kurbo::BezPath`.
2. Với chữ nghiêng, "dựng thẳng" bằng phép affine theo `post.italicAngle` trước khi đo.
3. Cắt bằng đường thẳng (`intersect_line`). Đoạn "mực" xác định bằng **winding number tại trung điểm**, nên đo đúng cả khi contour chồng nhau (variable font không gỡ overlap).
4. Diện tích tính bằng tích phân scanline (48 dòng), không dùng signed area, vì signed area sai khi có overlap.

| Trục | Định nghĩa trong spike | Loại | Nhận xét từ dữ liệu |
|---|---|---|---|
| x-height, cap, ascender, descender | đỉnh bbox của `x`/`H`/`d`, đáy `p` | **Đo hình học thật** | Khớp OS/2 trên 5 mẫu (Playfair 515 so với 514 của Python, do overshoot). Không bao giờ phải trả 0.5 "mặc định" |
| x-height/em, x-height/cap | tỉ lệ | Thật | Có cả hai. Audit nói Python dùng nhầm x/cap như x/em; nay tách riêng |
| Độ đậm: `stem_n_xh` | bề dày thân trái `n` ở 35% x-height ÷ x-height | Thật | Helvetica 0.116 / 0.168 / 0.261 (Light/Regular/Bold); SF wght 1→1000: 0.037→0.365 |
| Độ đậm: `darkness_no` | diện tích mực `n`,`o` ÷ (advance × x-height) | Thật (tích phân số) | Tăng đơn điệu theo độ đậm; Futura Condensed ExtraBold 0.70 |
| Tương phản `contrast_o` | `o`: quét tia qua tâm mỗi 2°, độ dày max/min | Thật, xấp xỉ với `o` dẹt | Didot 7.2, Bodoni 72 6.2 (Bold 11), Playfair 4.9, Baskerville 3.6, Times 3.0, Georgia 2.7, Libre Baskerville 2.3, sans 1.1–1.3. Xếp hạng hợp lý |
| Tương phản `contrast_H` | thân `H` ÷ thanh ngang `H` | Thật | Cùng xu hướng với `contrast_o` (Didot 7.1, Playfair 4.7, sans ~1.1–1.2). Dùng để đối chiếu chéo |
| Trục nhấn (stress) | khớp điều hoà bậc 2 lên độ dày theo góc; null nếu contrast < 1.3 | Đo, nhưng **chưa đáng tin** | Times 12°, Cochin 6°. Hoefler Text và Baskerville lại ra 0° (lẽ ra nghiêng vừa). Cần thử bằng `e`/`O` và kiểm định với nhãn tay. Hiển thị là "ước lượng" |
| Aperture `c` | tia từ khe trong `c`, số độ thoát ra ngoài không chạm mực | Thật, phụ thuộc chọn gốc | Open Sans 105°, Libre Baskerville 92°, Helvetica 49°, Roboto 53°. Đúng trực giác humanist mở / grotesque khép |
| Aperture `e` | như trên, gốc trong counter dưới | **Không ổn định** | Ra 0–221° vô lý (Futura 0, Helvetica Bold 221). Chọn gốc bằng dòng quét bị sai. Cần thiết kế lại (đo khoảng hở đầu nét–thanh ngang) |
| Độ rộng | `o` width ÷ xh; advance trung bình a–z ÷ xh | Thật | Futura Condensed 0.63 so với Futura 1.07 |
| Độ nghiêng | lệch trung điểm thân `l`/`I` ở 25% và 75% chiều cao | Thật | Helvetica Oblique −12.0 (`post` −12), Bodoni Italic −22.3, Menlo Italic −11.7 |
| Chân serif | bề rộng mực sát baseline ÷ thân, trung vị `i l n` | Thật | Sans đúng 1.0. Serif 1.9–3.0. Menlo 2.1 vì có chân slab ở `i l` thật. Chữ nghiêng serif cho ~0.7–1.4, cần xử lý riêng |
| Counter `o` | diện tích lòng ÷ diện tích bao, tích phân scanline | Thật | Giảm theo độ đậm (Gill Sans Light 0.62 → UltraBold 0.05) |

**Chưa làm**, đều làm được bằng cùng kỹ thuật [SUY LUẬN]:
- `a`/`g` một hay hai tầng: đếm counter kín theo winding.
- Độ cong và độ sắc terminal.
- Spacing trung bình (side bearing).
- Phân biệt `Il1`, `O0`.
- Đặt dấu tiếng Việt.

So với Python:
- Python trả "mặc định" khi lỗi (0.5, 1.0…). Spike trả `null` có lý do, ví dụ `no_latin_lowercase`.
- Python có hai thang contrast lệch nhau (1.0–3.5 so với 1.5–6.5). Spike dùng một định nghĩa hình học, cùng cho TTF và CFF, vì đã flatten về BezPath; không đếm số lệnh path.

### 3.1 Phân bố null [ĐÃ CHẠY]
Trên 461 face có chữ Latin thường: `stress_tilt_deg` null ở 227 face (cố ý, vì contrast < 1.3); `contrast_H` null ở 4 face. Các trục khác luôn có giá trị.

### 3.2 Chuẩn hoá 0–1 theo phân vị
Spike chưa làm. Số thô ở trên dùng được làm đầu vào trực tiếp cho bước chuẩn hoá phân vị trong thiết kế phần 3.

### 3.3 Variable font: phát hiện quan trọng [ĐÃ CHẠY]
- `.New York` có trục `opsz` mặc định = 256, tức bản display. Ở instance mặc định, nét mảnh của `o` chỉ dày 3/2048 em, nên **contrast_o = 68**. Đo ở `opsz=14, wght=400` thì contrast_o = **2.1**, hợp lý.
- **Quy tắc đề xuất:** đo ở vị trí chuẩn `wght=400, wdth=100, opsz=14` (kẹp vào khoảng của trục). Lưu thêm số đo ở `wght` min/max để gợi ý instance khi ghép cặp.
- Ví dụ: Greta Sans (WOFF2, VF) có `stem_n_xh` từ 0.031 đến 0.547 khi wght đi từ 1 đến 1000.
- 58/792 face là VF; đọc được trục và named instances. `STAT`/`avar` mới chỉ phát hiện là có, chưa phân tích.

---

## 4. Hiệu năng [ĐÃ CHẠY]

Tập dữ liệu: `/System/Library/Fonts` (đệ quy), `/Library/Fonts` và `fonts/samples`.
- 375 file, 836.5 MB, **792 face**.
- Định dạng: 545 face trong TTC, 247 SFNT; 654 glyf, 128 CFF, 9 CFF2.
- Page cache đã làm nóng trước, nên số dưới đây là chi phí CPU cộng chép bộ nhớ, không phải đọc đĩa lạnh.
- Mỗi lần chạy đều gồm `fs::read` toàn bộ file.

| Pha | Đơn luồng | rayon (12 luồng) | Tăng tốc |
|---|---|---|---|
| metadata + tiếng Việt | 452–490 ms (0.57–0.62 ms/face) | 112–160 ms | ×2.8–4.4 |
| metadata + tiếng Việt + hình học (+ 2 instance cho mỗi VF) | 1 591–1 819 ms (2.0–2.3 ms/face) | 243–310 ms (0.31–0.39 ms/face) | ×5.9–6.5 |
| đọc + BLAKE3 toàn bộ 836 MB | — | 115–141 ms | — |

Số trong bảng đo trên bản đã có vị trí VF chuẩn (mục 3.3). Mỗi face VF được đo 4 lần: vị trí chuẩn, instance mặc định, wght min và wght max.

Đối chiếu Python (`FontParser.extract_full_profile`, chỉ heuristic outline, không raster, không CLIP), trên cùng 50 file (5 mẫu + 45 TTF trong `Supplemental`):
- **Python: 824 ms, 16.5 ms/file.** Chưa tính thời gian khởi động interpreter hay sidecar.
- **Rust đơn luồng: 218 ms (4.4 ms/face); rayon: 77 ms.**
- Rust nhanh hơn khoảng 3.8 lần khi chạy đơn luồng, dù đo nhiều thứ hơn, và khoảng 10 lần với rayon.

Diễn giải [SUY LUẬN]:
- Pha "metadata + tiếng Việt" bị chi phối bởi `fs::read` các TTC CJK lớn (vài chục MB) và việc đếm `cmap` lớn. Dùng mmap (`memmap2`) và bỏ đếm toàn bộ cmap sẽ giảm thêm.
- Ước tính cho 10 000 face: khoảng 4 s với rayon, chưa tính đĩa lạnh. Đủ để "font hiện ngay" như thiết kế phần 2; phần hình học có thể chạy cùng lúc thay vì đưa xuống hàng đợi nền.
- Embedding (ONNX) mới là pha chậm. Spike này không đo pha đó.

---

## 5. Rủi ro

| # | Rủi ro | Bằng chứng | Giảm thiểu |
|---|---|---|---|
| R1 | **File cắt cụt vẫn "parse được".** read-fonts đọc lười nên bảng bị cụt chỉ biến mất lặng lẽ | [ĐÃ CHẠY] Roboto cắt còn 5 KB vẫn ra name/cmap, nhưng hình học báo `no_latin_lowercase` thay vì "file hỏng" | Bước `validate()`: mọi table record nằm trong file, các bảng bắt buộc (head, maxp, cmap, hmtx, glyf/CFF) parse đủ, `loca` khớp `glyf`. Trả mã `truncated` / `missing_table` |
| R2 | Đo VF ở instance mặc định cho số sai lệch lớn | [ĐÃ CHẠY] New York: contrast 68 so với 2.1 | Vị trí đo chuẩn (mục 3.3), lưu `location` cùng mỗi kết quả |
| R3 | Stress và aperture `e` chưa đáng tin | [ĐÃ CHẠY] mục 3 | Gắn nhãn "ước lượng" kèm độ tin cậy; kiểm định với nhãn tay (bộ đánh giá ở thiết kế phần 3) trước khi dùng cho pairing |
| R4 | API fontations còn ở 0.x, đổi thường xuyên (skrifa 0.48) | [SUY LUẬN] | Ghim phiên bản; bọc trong `fontatlas-core` để chỉ một chỗ phải sửa |
| R5 | WOFF1 chưa thử thực tế; `wuff` ít người dùng hơn bản C của Google | Không có file WOFF1 để thử | Thêm fixture WOFF1 và WOFF2 (OFL) vào test; fuzz `decompress_*` bằng cargo-fuzz |
| R6 | Font chỉ có bitmap/màu (sbix, CBDT, emoji) | [ĐÃ CHẠY] 1 face `outline_format=none` | Trả mã `no_outlines`; metadata vẫn có |
| R7 | Các trục dùng một vài glyph cố định (`o n H c i l`). Font có `o` trang trí hoặc font chỉ có chữ hoa cho số đo lệch | [SUY LUẬN] | Trung vị trên nhiều glyph; nhánh chỉ chữ hoa (`O H C I`); cờ `quality` |
| R8 | Tốn bộ nhớ khi đọc nguyên file TTC CJK lớn trên nhiều luồng | [SUY LUẬN] | mmap; giới hạn số luồng I/O |

---

## 6. Đề xuất API cho `fontatlas-core`

```rust
// Nạp
pub struct FontFile { pub hash: blake3::Hash, pub container: Container, /* bytes|mmap */ }
pub enum Container { Sfnt, Collection, Woff1, Woff2 }
pub fn open(path: &Path) -> Result<FontFile, CoreError>;           // đọc + băm + giải nén
impl FontFile {
    pub fn faces(&self) -> Result<Vec<Face<'_>>, CoreError>;       // TTC → n face
    pub fn validate(&self) -> Result<(), CoreError>;               // R1
}

// Phân tích (thuần, không I/O, Send + Sync → rayon)
pub struct Face<'a> { pub index: u32, font: read_fonts::FontRef<'a> }
impl Face<'_> {
    pub fn metadata(&self) -> Metadata;                            // name, OS/2, head, post, panose, fs_type, outline_format, color tables
    pub fn variation(&self) -> Option<VariationInfo>;              // axes, named instances, STAT, avar
    pub fn features(&self) -> Features;                            // GSUB/GPOS tags
    pub fn coverage(&self) -> Coverage;                            // scripts/blocks + VietnameseCoverage
    pub fn vietnamese(&self) -> VietnameseCoverage;                // status, missing: Vec<char>, mark/mkmk/ccmp, composite count
    pub fn measure(&self, at: MeasureAt) -> Anatomy;               // trục hình học
}
pub enum MeasureAt { Standard, Default, Location(Vec<(Tag, f32)>) }

pub struct Anatomy {
    pub location: Vec<(Tag, f32)>,                                 // đo ở đâu (bắt buộc cho VF)
    pub x_height: Metric, pub cap_height: Metric, pub x_height_em: Metric,
    pub stem: Metric, pub darkness: Metric,
    pub contrast: Metric, pub stress_deg: Metric,
    pub aperture: Metric, pub width: Metric, pub slant_deg: Metric,
    pub serif_foot: Metric, pub counter: Metric,
}
pub struct Metric { pub value: Option<f32>, pub quality: Quality, pub method: &'static str }
pub enum Quality { Measured, Estimated, Failed(FailReason) }       // không còn giá trị mặc định im lặng

#[derive(Debug, thiserror::Error)]
pub enum CoreError { Io, NotAFont, Truncated, MissingTable(Tag), WoffDecode, NoOutlines, Parse(String) }
impl CoreError { pub fn code(&self) -> &'static str; }             // mã cấu trúc cho UI/DB

pub const ANALYZER_VERSION: &str = "…";                            // gắn vào mọi bản ghi `analyses`
```

Nguyên tắc thiết kế:
- Hàm phân tích nhận `&Face`, không có trạng thái toàn cục, nên chạy song song được.
- Tauri chỉ gọi `open → validate → faces → (metadata, vietnamese, measure)`, rồi ghi vào bảng `files`/`faces`/`analyses`.
- Mỗi `Metric` mang `method` và `quality` để UI ghi "ước lượng" đúng như thiết kế phần 3.
- Phân vị toàn thư viện nằm ở tầng trên (DB), không nằm trong core.

---

## 7. Khác biệt chính so với Python

| Hạng mục | Python hiện tại | fontations spike |
|---|---|---|
| TTC | Lỗi (`TTLibError`), Rust còn không quét `.ttc` | 545 face TTC đọc được |
| WOFF2 | Sidecar thiếu brotli | `wuff`, Rust thuần |
| Variable font | Chỉ instance mặc định, không đọc fvar | Trục + instance; đo ở bất kỳ location nào |
| Tiếng Việt | Tập sai, ngưỡng 0.9, cờ boolean | 186 ký tự, danh sách thiếu, phân biệt dựng sẵn và ghép dấu |
| x-height fallback | Mã chết (`bbox.yMax` trên tuple) | Đo trực tiếp từ outline |
| Stroke contrast | Hai thang lệch nhau; heuristic đếm lệnh path | Một định nghĩa hình học, hai cách đo đối chiếu |
| Lỗi | Nuốt lỗi → 0.5/1.0 | `null` + mã/`quality` |
| Tốc độ (50 file) | 16.5 ms/file | 4.4 ms/face đơn luồng, ~1.5 ms/face rayon |

Python còn hơn ở các điểm sau, và chúng chỉ nên giữ như công cụ ngoại tuyến:
- Raster EDT/skeleton.
- Embedding CLIP. Việc này chuyển sang `ort` theo thiết kế, không thuộc spike này.

---

## 8. Tái lập

```bash
cd spikes/core-engine && cargo build --release
./target/release/fontatlas-spike bench /System/Library/Fonts /Library/Fonts ../../fonts/samples
./target/release/fontatlas-spike jsonl ../../fonts/samples/*.ttf     # một dòng JSON / face
./target/release/fontatlas-spike report <file>                       # JSON chi tiết
```
