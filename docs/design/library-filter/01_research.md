# 01 · Nghiên cứu — Filter thư viện, Search tích hợp, dữ liệu ngôn ngữ

Ngày tra cứu: **2026-11-06**. Mức xác minh ghi ở từng mục: **[nguồn]** = đọc được trang gốc hôm nay; **[gián tiếp]** = qua kết quả tìm kiếm/bên thứ ba; **[quan sát]** = hiểu biết sản phẩm, chưa mở lại trang hôm nay → chủ app nên kiểm tra nhanh trước khi "làm y hệt".

---

## 1. Hiện trạng FontAtlas (code `520410f`)

| Chỗ | Hiện tại | Vấn đề |
|---|---|---|
| Thẻ font `FaceCell.tsx:169` | `−68` đỏ khi thiếu ký tự tiếng Việt, `●` xanh khi đủ; nghĩa chỉ có trong tooltip | Số âm đọc như "điểm trừ"/lỗi đo; không biết thiếu gì, so với bao nhiêu (tổng 186 ký tự); chỉ nói về tiếng Việt |
| TopBar | Ô "Tìm font" (⌘K) · nút `Ặ vi` · nút `Lọc · n` · lưới/danh sách | Ba cửa vào rời nhau; điều kiện đang bật không hiện cạnh ô tìm (chỉ có số đếm) |
| FilterPanel | Popover 360px: Variable / In nghiêng, rồi 16 trục, mỗi trục = checkbox + "Toàn khoảng" + ô Từ/Đến + **2 slider rời** + dòng Min/max | Dài (cuộn ~4 màn), nhiều chữ, ô nhập luôn hiện; 2 slider rời không đọc như một khoảng; thiếu phân loại, ngôn ngữ, độ đậm/rộng, nguồn, giấy phép, yêu thích |
| Backend sẵn có | `FaceQueryDto`: text, vi (full/supported), license[], variable, italic, sourceIds[], favoritesOnly, collectionId, tagIds[], axisFilters, rawAxisRanges; `axis_stats` trả min/max/p1/p99/count/unit cho 16 trục; per-face: weight_class, width_class, `scripts` (OT tag), `cmap_scripts` (khối Unicode ≥ ngưỡng), vi_status, vi_missing | Chưa có: bảng ngôn ngữ, coverage theo ngôn ngữ, phân loại lưu trong `faces` (chỉ tính lúc mở chi tiết: Mono/Sans/Serif theo `width_cv`, `serif_foot`), facet counts, histogram |

---

## 2. Tham chiếu filter

### 2.1 Adobe Fonts (fonts.adobe.com + font browser trong app CC)

Cấu trúc (diễn đạt lại):
- **Thanh trái dạng các khối gập**, áp dụng ngay, các khối **giao nhau (AND)** [gián tiếp — Illustrator help].
- **Tags**: thẻ cảm xúc/công dụng (Friendly, Wedding, Futuristic, Clean, Geometric…) [nguồn — Adobe Blog 2020].
- **Classification**: **8 loại**, mỗi loại là một ô có chữ mẫu minh hoạ (sans serif, serif, slab, script, mono, hand, decorative, blackletter…) [nguồn: "eight type classifications" — Adobe Blog 2020; danh sách đủ 8 = quan sát].
- **Properties**: mỗi thuộc tính là **một hàng 3 nút minh hoạ bằng chữ** (thấp/vừa/cao): Weight, Width, x-height, Contrast; thêm Case (thường/chỉ hoa) và Figures (lining/oldstyle) [gián tiếp — Illustrator help: "weight, width, height, contrast, case, or figures… select multiple attributes"; "ba lựa chọn mỗi thuộc tính" — fontba.se].
- **Languages & Writing Systems**: dropdown chọn một ngôn ngữ/hệ chữ [gián tiếp — Adobe help; trang 403 khi fetch].
- **Variable**: bộ lọc "chỉ font variable" (font picker CC beta) [gián tiếp — Adobe Community].

Bài học: ẩn số đo sau **3 nấc trực quan có minh hoạ** → người không rành giải phẫu vẫn dùng được. Nấc của Adobe do Adobe phân loại tay cho catalog của họ; FontAtlas có số đo thật nên có thể tính nấc từ phân vị thư viện.

### 2.2 RightFont

> Chủ app nói "RightFont 5"; tài liệu hiện hành là bản 8.x (changelog 8.6) — cấu trúc filter không đổi về bản chất.

- **Unified Search (⌘F)**: một ô tìm ở toolbar; kết quả hiện **ngay dưới ô** khi đang gõ; tìm theo tên, family, style, designer [nguồn — rightfontapp.com/docs/search-fonts].
- **Advanced filters**: panel gắn với search, gồm **Category** (serif, sans-serif, script, display…), **Weight**, **Width**, **Language** (popup chọn 1 ngôn ngữ), thuộc tính x-height; nút **↻** xoá mọi filter [nguồn].
- Font chưa phân loại → **chuột phải → chọn category** (phân loại tay bổ sung) [nguồn].
- **Smart Filters** ở sidebar: lưu bộ điều kiện thành mục tự cập nhật (kiểu, trạng thái bật, ngôn ngữ…) [nguồn — docs/organize-fonts/create-smart-filters].

Bài học: filter **sống trong search**, không phải nút riêng; filter đã dùng nhiều lần → lưu thành mục sidebar.

### 2.3 Tham chiếu phụ (hữu ích cho Search + Advanced)

- **Typeface (Mac)** [nguồn — typefaceapp.com/help/articles/search]: ô tìm là nơi gõ **token**: `#tag` (AND trong ô, OR khi chọn ở sidebar), `#!tag` loại trừ, `"PT` khớp đầu, `"PT Sans"` khớp đúng, `pro"` khớp cuối; filter Weight 5 nấc, Width 3 nấc, Slant, **Height ratio** (x/ascender), Figures, tính năng OT, Variable/Color/Bitmap; **Option = đảo filter**; ngôn ngữ + "chỉ font vẽ được câu preview"; Reset giữ nguyên chữ tìm.
- **FontBase SuperSearch** [nguồn — fontba.se/blog/super-search]: đo quang học x-height, weight (tỷ lệ phủ mực), contrast, proportion trên **font cục bộ**; mỗi tham số có **biểu đồ phân bố + range selector** → xác nhận ý tưởng histogram mật độ trong slider.

### 2.4 Insight cho FontAtlas
1. **Một cửa vào**: Search chứa chip filter; Basic là popover của Search; Advanced mở rộng từ Basic (không phải nút thứ ba).
2. **Hai tầng ngôn ngữ thị giác**: Basic = nấc minh hoạ (như Adobe) cho người lướt; Advanced = slider số đo thật (lợi thế riêng của FontAtlas — Adobe/RightFont không có).
3. **Token có alias tiếng Việt** + autocomplete, không bắt nhớ cú pháp (Typeface chứng minh token dùng được với designer khi có gợi ý).
4. **Đếm kết quả cạnh mỗi lựa chọn** để tránh ngõ cụt "0 font".
5. **Ngôn ngữ = dữ liệu**, không đoán theo tên font; FontAtlas đã có cmap → chỉ cần bảng ngôn ngữ→ký tự.

---

## 3. Dữ liệu đa ngôn ngữ (OSS)

Đo bằng `scripts/lang_extract.py` (chạy ngoài app, dữ liệu tải vào `/tmp`), wheel PyPI tải ngày 2026-11-06.

| Nguồn | Giấy phép | Độ phủ (đo) | Định dạng | Ghi chú |
|---|---|---|---|---|
| **Hyperglot 0.8.2** (Rosetta) | **Apache-2.0** (LICENSE trong wheel) — nhúng được vào app đóng hay mở, giữ NOTICE/ghi công | 806 file; **783 orthography chính, còn dùng**; validity: verified 204 · preliminary 500 · draft 79; Latin 550, Cyrillic 92, Arabic 47, Devanagari 15… | YAML theo ISO 639-3: `orthographies[].base / auxiliary / marks / script / status`, `validity`, `speakers`, `design_requirements` | Chuyên cho **kiểm tra font** (base tối giản, auxiliary tách riêng, có chữ hoa, marks riêng). Có kế thừa `<eng>`, `<default>` → cần resolver. Có CLI `hyperglot` (Python) |
| **gflanguages 0.7.11** (Google Fonts) | **Apache-2.0** (LICENSE.txt); một phần dữ liệu gốc CLDR | 1693 textproto; **1068 có exemplar base** (1042 không lịch sử); Latn 888 | textproto (protobuf): `exemplar_chars{base, auxiliary, marks, numerals, punctuation, index}`, `sample_text` (câu mẫu UDHR), `population`, `region` | Có **câu mẫu theo ngôn ngữ** (tiếng Việt: "Nhân quyền", "Sự xâm phạm…") — dùng làm preview khi lọc ngôn ngữ |
| **Unicode CLDR** exemplars | Unicode License v3 (permissive, giữ thông báo) | Locale chính phủ tốt, thiểu số ít hơn | XML/JSON | Hyperglot tự đánh giá: exemplar CLDR có ký tự thừa/thiếu khi dùng để kiểm font [nguồn — README_comparison] → dùng làm đối chứng, không làm nguồn chính |
| fontTools / skrifa | MIT / Apache-MIT | Không có dữ liệu ngôn ngữ | — | Chỉ là công cụ đọc cmap (Rust đã dùng `skrifa`/`read-fonts`) |

Đo thêm:
- **Tiếng Việt**: Hyperglot `vie` base **186 ký tự** = gflanguages `vi_Latn` base 186, **trùng hoàn toàn**; Hyperglot ghi `validity: verified`, kèm `design_requirements` (dấu chồng cần thiết kế riêng) → khớp với tổng 186 của `VietnameseDto.total` hiện có.
- **Kích thước nhúng** (map id → chuỗi ký tự base, JSON): Hyperglot 783 mục = 188 KB thô / **75 KB gzip**; chỉ verified+preliminary (704) = 166 KB / **69 KB gzip**; gflanguages 1068 mục = 139 KB / 42 KB gzip. Không đáng kể với app desktop.
- **Đối chiếu cmap** 5 font mẫu (bản subset trong `fonts/samples`, số liệu minh hoạ): Libre Baskerville đạt 450/500 ngôn ngữ Latin tin cậy nhưng **thiếu 68 ký tự Việt** (Ơ ơ Ư ư Ả ả Ấ…) — đúng con số `−68` trên thẻ hiện tại; Open Sans/Roboto/Playfair/Source Code Pro đủ tiếng Việt. Toàn bộ chạy < 1 s/5 font bằng Python → Rust per-face là vi giây.
- **Nhiễu dữ liệu**: `sdh_Arabic` (Southern Kurdish) có base toàn chữ Latin → font Latin "hỗ trợ Arabic". Cần **kiểm tra chéo script** (ký tự base phải thuộc script khai báo, trừ ký tự chung) và chỉ nhận `verified|preliminary` mặc định.

### Đề xuất
- **Nguồn chính: Hyperglot** (Apache-2.0, chuyên cho kiểm font, có validity + design notes). Xuất ở build-time: resolver chính chủ (`hyperglot-export`/thư viện Python) → `languages.json.gz` (id, tên Anh, autonym, script, validity, speakers, base, auxiliary, marks) → `include_bytes!` trong crate core. Bổ sung **tên tiếng Việt** cho ~40 ngôn ngữ hay dùng (bảng tay nhỏ).
- **Dự phòng/bổ sung: gflanguages** (Apache-2.0) — cùng định dạng ký tự, thêm `sample_text` để preview theo ngôn ngữ và fallback cho ngôn ngữ Hyperglot chưa có.
- **Cập nhật**: ghim phiên bản (vd `hyperglot==0.8.2`) trong script build `tools/langdata/`; cập nhật khi nâng cấp app; thay đổi bảng → tăng `lang_db_version` → backend tính lại coverage (chỉ đọc cmap đã cache, không quét lại file).
- **Ghi công**: mục "Giới thiệu → Dữ liệu mở": Hyperglot © Rosetta (Apache-2.0), Google Fonts language data (Apache-2.0), Unicode CLDR (Unicode License v3).

---

## Nguồn
- Adobe Blog — Using tags to find the perfect font for the job (2020-08-14): https://blog.adobe.com/en/publish/2020/08/14/using-tags-to-find-the-perfect-font-for-the-job
- Adobe Illustrator help — Font browser overview: https://helpx.adobe.com/lv/illustrator/using/fonts.html (403 khi fetch; nội dung qua kết quả tìm kiếm)
- Adobe Community — Variable font filter in font picker (beta): https://community.adobe.com/announcements-532/new-in-beta-variable-font-filter-in-the-font-picker-26-3x46-1557740
- RightFont docs — Search fonts: https://rightfontapp.com/docs/search-fonts · Smart Filters: https://rightfontapp.com/docs/organize-fonts/create-smart-filters
- Typeface — Searching & Filtering fonts: https://typefaceapp.com/help/articles/search
- FontBase — SuperSearch: https://fontba.se/blog/super-search
- Hyperglot — README_database / README_comparison: https://github.com/rosettatype/hyperglot ; PyPI `hyperglot` 0.8.2
- Google Fonts language data — PyPI `gflanguages` 0.7.11: https://github.com/googlefonts/lang
- Unicode CLDR: https://cldr.unicode.org
