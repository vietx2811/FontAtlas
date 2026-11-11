# Nghiên cứu: Taxonomy phân loại font cho FontAtlas

Ngày: 2026-11-11. Phạm vi: so sánh hệ thống phân loại có sẵn, catalog thực tế, khoảng trống của taxonomy hiện tại (`crates/fontatlas-core/src/classify.rs`), và đề xuất taxonomy 2 tầng.

Quy ước: **[V]** = đã xác minh từ nguồn đã fetch trong phiên này; **[I]** = suy luận / kiến thức nền chưa xác minh được từ nguồn (cần kiểm tra lại trước khi đưa vào UI hoặc tài liệu công khai). Thuật ngữ font giữ tiếng Anh, kèm gloss tiếng Việt ngắn.

---

## 0. Tóm tắt (TL;DR)

1. Không có hệ thống chuẩn nào còn "sống": Vox-ATypI bị ATypI bỏ năm 2021 [V], DIN 16518 vẫn là chuẩn Đức nhưng bản sửa đổi chưa thấy được áp dụng [V], BS 2961 chỉ là bản rút gọn của Vox [V]. Catalog hiện đại (Google Fonts) đã chuyển sang **ít class + nhiều tag độc lập** [V]. FontAtlas nên đi theo hướng đó.
2. Taxonomy hiện tại (4 sans + 5 serif + 2 mono + 3 script...) đã khá gần với Google Fonts, nhưng thiếu: **slab subtypes (Clarendon/Egyptian/Typewriter)**, **wedge/latin/tuscan**, **sans: rounded / superellipse(squared) / glyphic (flared)**, **script: upright, monoline, copperplate**, **display genres (Art Deco, Art Nouveau, wood type, techno...)**, và **pixel/stencil/inline/shaded** hiện chỉ là trait chung.
3. Đề xuất: 8 class, ~35 subclass, trait tag độc lập, mood/use tag cho người dùng/community. Chỉ tự động phát hiện những gì đo được từ outline; phần "feeling" (Calm, Playful...) KHÔNG đo được, để user/community tag.
4. Hybrid (semi-serif, sans with flare) và "small serif" bị đọc nhầm thành sans: dùng **primary + secondary class** (hoặc `hybrid_of`) + confidence, không ép vào một ô.

---

## 1. Các hệ thống phân loại đã có

### 1.1 Vox-ATypI (1954 Vox, ATypI 1962)

[V] Nguồn: https://en.wikipedia.org/wiki/Vox-ATypI_classification

| Class | Gloss | Ghi chú |
|---|---|---|
| Venetian | serif kiểu Renaissance Ý, bracketed, contrast thấp | ~ humanist/oldstyle |
| Garalde | serif kiểu Aldus + Garamond, contrast cao hơn, trục nghiêng | Garamond, Palatino-ish |
| Transitional | trục đứng, serif thẳng, contrast cao hơn | Baskerville, Times |
| Didone | contrast cực cao, hairline serif | Bodoni, Didot |
| Mechanistic (Slab) | contrast thấp, slab serif: Clarendon (bracketed), Egyptian (unbracketed) | |
| Lineal | sans | |
| Glyphic | gợi chữ khắc đá, serif tam giác nhỏ hoặc nét thon | Optima, Albertus |
| Script | giả chữ viết tay nghiêng, thường nối | |
| Graphic | dựa trên nét vẽ tay, cho display | |

ATypI sau đó thêm Blackletter, Gaelic, Non-Latin [V]. **BS 2961 (1967)**: bản rút gọn, khác biệt chính là tách Lineal thành **Grotesque, Neo-grotesque, Geometric, Humanist** [V] (chính là 4 sans subclass của FontAtlas hôm nay).

**ATypI bỏ Vox năm 2021**: board bỏ phiếu 18/3/2021, thông báo công khai ~27/4/2021 (nguồn nói lệch ngày), lập working group để xây hệ mới bao gồm nhiều script; Catherine Dixon phê bình hệ cũ không đổi từ 1967 và không đủ cho font đương đại [V]. Một bài bình luận 2023 ghi nhận sau 2 năm gần như chưa có hệ thay thế [V]. Nguồn: https://en.wikipedia.org/wiki/Vox-ATypI_classification ; https://atypi.org/tag/vox/ ; https://blazetype.eu/blog/voxatyp-i

Bài học: Vox mạnh ở serif (lịch sử), yếu ở sans/display/script hiện đại. Đừng bê nguyên.

### 1.2 DIN 16518

[V] Chuẩn Đức, 11 nhóm theo hình thức + lịch sử. Nhóm 1 Venezianische Renaissance-Antiqua (~1465-1530), 2 Französische Renaissance-Antiqua, 3 Barock-Antiqua (từ giữa TK17), 5 Serifenbetonte Linear-Antiqua (slab). Nguồn: https://studyflix.de/allgemeinwissen/klassifikation-der-schriften-nach-din-16518-8552 và kết quả tìm kiếm uni-hamburg.
[I] Nhóm 4 Klassizistische Antiqua (Didone), 6 Serifenlose Linear-Antiqua (sans), 7 Antiqua-Varianten, 8 Schreibschriften, 9 Handschriftliche Antiqua, 10 Gebrochene Schriften (blackletter), 11 Fremde Schriften (non-Latin). Chưa xác minh được số thứ tự nhóm 4, 6-11 từ nguồn.
[V] Bản nháp sửa đổi (bài Macwelt 1998) chia 5 nhóm chính: Blackletter, Roman serif, Linear (sans), Slab, Script/handwritten + subgroup phong cách. Nguồn: https://www.macwelt.de/article/940776/neue-klassifikation.html (bài cũ 1998; không rõ bản này đã thành chuẩn chưa). Kupferschmid tham gia ủy ban DIN về phân loại [V] (https://www.umprum.cz/... trang profile). Không tìm được bài "Some type genres explained" của Kupferschmid; **không trích nội dung bài đó** (xem mục 7).

Bài học: DIN coi "Schreibschrift/Handschrift" và "Gebrochene" là nhóm top-level riêng, ủng hộ giữ Blackletter làm class.

### 1.3 Thibaudeau, Bringhurst, Dixon

- **Thibaudeau (1921)**: [I] phân theo hình dạng serif (Elzévirs, Didots, Égyptiennes, Antiques). Tiền thân của việc phân theo hình serif. Chưa fetch nguồn.
- **Bringhurst** (*Elements of Typographic Style*): [I] ưu tiên mô tả lịch sử/ngữ cảnh (Renaissance, Baroque, Neoclassical, Romantic, Realist, Modernist, Geometric Modernist, Postmodernist) thay vì hộp cứng. Chưa fetch nguồn.
- **Catherine Dixon**: [V] bài "Typeface classification" (St Bride Conference, 2002) → framework kết hợp **sources (lịch sử)** + **tám facet phân tích hình thức (formal attributes)**, công bố InfoDesign 2008 và *Typeform dialogues* 2018. Bình luận: vẫn "data-centric" hơn user-centric. Nguồn: https://ualresearchonline.arts.ac.uk/id/eprint/1099/ ; https://ualresearchonline.arts.ac.uk/id/eprint/5726/ ; https://infodesign.emnuvens.com.br/infodesign/article/view/53
  Bài học áp dụng: **tách "thuộc tính hình thức đo được" (facets) khỏi "nhãn thể loại"** → đúng với kiến trúc FontAtlas (anatomy đo được → class + trait).

### 1.4 PANOSE (Family Kind 2 Latin Text)

[V] https://monotype.github.io/panose/pan2.htm. 10 digit: Family Kind, Serif Style, Weight, Proportion, Contrast, Stroke Variation, Arm Style, Letterform, Midline, X-height. Family kind: 2 Latin Text, 3 Latin Hand Written, 4 Latin Decorative, 5 Latin Symbol (https://monotype.github.io/panose/pan1.htm).

Serif Style digit [V]: 0 Any, 1 No Fit, 2 Cove, 3 Obtuse Cove, 4 Square Cove, 5 Obtuse Square Cove, 6 Square, 7 Thin, 8 Oval, 9 Exaggerated, 10 Triangle, 11 Normal Sans, 12 Obtuse Sans, 13 Perpendicular Sans, 14 Flared, 15 Rounded.
- Hữu ích cho FontAtlas: 10 Triangle (wedge/glyphic), 14 Flared (glyphic sans), 15 Rounded, 9 Exaggerated (display/slab đặc), 6 Square (slab). PANOSE trong file thực tế thường bị để 0/1 hoặc sai [I - kinh nghiệm chung; classify.rs đã dùng Panose như tín hiệu phụ].

### 1.5 OS/2 sFamilyClass (IBM)

[V] https://learn.microsoft.com/en-us/typography/opentype/spec/ibmfc. byte cao = class, byte thấp = subclass.

| Class | Tên | Subclass đáng chú ý |
|---|---|---|
| 0 | No classification | |
| 1 | Oldstyle Serifs | 2 Garalde, 3 Venetian, 4 Modified Venetian, 5 Dutch Modern, 6 Dutch Traditional, 7 Contemporary, 8 Calligraphic |
| 2 | Transitional | 1 Direct Line, 2 Script |
| 3 | Modern Serifs | 1 Italian (Didone), 2 Script |
| 4 | Clarendon Serifs | 1 Clarendon, 2 Modern, 3 Traditional, 4 Newspaper, 5 Stub Serif, 6 Monotone, 7 Typewriter |
| 5 | Slab Serifs | 1 Monotone, 2 Humanist, 3 Geometric, 4 Swiss, 5 Typewriter |
| 7 | Freeform Serifs | 1 Modern (Souvenir) |
| 8 | Sans Serif | 1-2 IBM Neo-grotesque, Humanist, 3 Low-x Round Geometric (Futura), 4 High-x Round Geometric (Avant Garde), 5 Neo-grotesque (Helvetica), 6 Modified Neo-grotesque (Univers), 9 Typewriter Gothic, 10 Matrix |
| 9 | Ornamentals | 1 Engraver (Copperplate), 2 Black Letter, 3 Decorative, 4 Three Dimensional |
| 10 | Scripts | 1 Uncial, 2 Brush Joined, 3 Formal Joined, 4 Monotone Joined, 5 Calligraphic, 6 Brush Unjoined, 7 Formal Unjoined, 8 Monotone Unjoined |
| 12 | Symbolic | 3 Mixed Serif, 6 Oldstyle Serif, 7 Neo-grotesque Sans |

Quan sát [I]: IBM class đã có "Slab/Clarendon/Typewriter", "Stub serif" (Cheltenham), "Geometric low-x vs high-x", Script chia theo joined/unjoined × brush/formal/monotone — cấu trúc script này hữu ích hơn "formal/casual_brush/handwritten" hiện tại. Nhược điểm: cực ít foundry điền đúng, nhiều font để 0.

---

## 2. Catalog thực tế

### 2.1 Google Fonts (số liệu quan trọng nhất, đã xác minh từ repo)

[V] Nguồn: https://raw.githubusercontent.com/google/fonts/main/tags/tags_metadata.csv và https://raw.githubusercontent.com/google/fonts/main/tags/tags_ui_details.textproto ; hướng dẫn: https://googlefonts.github.io/gf-guide/tags.html (mỗi font thường 2-6 tag, ít nhất 1 tag appearance/feeling + 1 tag cấu trúc như serif).

7 nhóm filter UI: Feeling, Appearance, Serif, Sans Serif, Calligraphy, Technology, Seasonal. Đường dẫn tag thực tế:

| Nhóm (UI) | Tag |
|---|---|
| **Sans** | Geometric, Humanist, Rounded, Neo Grotesque, Glyphic, Grotesque, Superellipse |
| **Serif** | Modern, Transitional, Humanist Venetian, Fat Face, Old Style Garalde, Scotch, Didone |
| **Slab** (gộp vào "Serif" ở UI) | Humanist, Geometric, Clarendon |
| **Script / "Calligraphy"** | Formal, Handwritten, Informal, Upright Script |
| **Feeling (/Expressive)** | Artistic, Futuristic, Stiff, Competent, Business, Sincere, Vintage, Loud, Cute, Excited, Playful, Fancy, Sophisticated, Awkward, Childlike, Innovative, Happy, Rugged, Active, Calm |
| **Appearance (/Theme)** | Wacky, Techno, Blobby, Shaded, Art Nouveau, Stencil, Blackletter, Woodtype, Distressed, Inline, Brush (hiển thị "Marker"), Medieval, Art Deco, Pixel, Tuscan; + Monospaced; + Not text (Symbols) |
| Seasonal | Halloween, Christmas, Diwali, Hanukkah, Kwanzaa, Valentine's Day, Lunar New Year, Holi |
| Technology | Variable, Color |
| Khác | Quality (Drawing, Spacing, Concept, Wordspace), Purpose (Learn To Write, Easy Reading), Special use (Barcode, Symbols, Redaction), Display |

Chi tiết quan trọng:
- Google dùng 4 "category" cũ (Serif, Sans, Display, Handwriting, Monospace) cho API, nhưng filter UI mới dùng tag trên [V partly: dataset 4 nhãn Serif/Sans/Handwriting/Display từ kết quả tìm kiếm, nguồn https://service.tib.eu/ldmservice/dataset/google-fonts-dataset].
- Có kho **Fontquant** (`/Fontquant/appearance/...`): x_height, cap_height, weight, width, slant, monospaced, **stencil**, lowercase a/g single/double story, **stroke_contrast (antiqua, antiqua_angle, raycaster)**, numerals style, features [V]. Đây là bộ đo tự động mã nguồn mở của chính Google → có thể tham khảo/đối chiếu thuật toán đo contrast/stress cho FontAtlas. Repo: https://github.com/googlefonts/fontquant [I - chưa fetch repo, chỉ thấy tên trong tags_metadata].
- Tag Google do **người gán thủ công qua GF Tagger** (không tự động), kèm điểm trọng số [I - theo gf-guide mô tả quy trình tag; không fetch phần quy trình].

### 2.2 Adobe Fonts

[V một phần] Blog Adobe 2020: có 8 classification (chỉ nêu 4: Serif, Sans serif, Script, Decorative), Properties (Weight, Width, Contrast, x-height), Tags (Friendly, Wedding, Futuristic, Clean, College, Shaded, Geometric...) — https://blog.adobe.com/en/publish/2020/08/14/using-tags-to-find-the-perfect-font-for-the-job . Cộng đồng Adobe liệt kê filter cơ bản: sans serif, serif, slab serif, script, mono, hand (https://community.adobe.com/t5/adobe-fonts-discussions/search-by-keyword/m-p/12686794). Không xác minh được 8 class đầy đủ; Blackletter không thấy xác nhận.
Điểm đáng học: **Properties là thuộc tính đo được dạng thang (weight/width/contrast/x-height) tách khỏi class** — khớp ý tưởng "usage Text/Display/UI" + anatomy.

### 2.3 Khác

| Nguồn | Phân loại | Mức xác minh |
|---|---|---|
| Font Squirrel | Blackletter, Calligraphic, Comic, Dingbat, Display, Grunge, Handdrawn, Monospaced, Novelty, Pixel, Programming, Retro, Sans Serif, Script, Serif, Slab Serif | [V] qua search snippet, https://www.fontsquirrel.com/fonts/list/classification/slab_serif |
| type.co.uk | Sans, Serif, Slab Serif, Script - Formal, Script - Casual, Handwriting | [V] snippet, https://www.type.co.uk/categories |
| FontBase | SuperSearch với preset (low contrast, heavy weight...); User Tags + Crowd Tags (≥3 user cùng gán thì thành crowd tag) | [V] snippet, https://fontba.se/learn/awesome |
| RightFont | Smart Tags theo tiêu chí (style, activation, languages), tag/category do user tạo | [V] snippet, https://rightfontapp.com/docs/organize-fonts/create-tags-and-smart-tags |
| MyFonts, Fontstand, Monotype, Fontshare, Future Fonts, Typewolf, Fonts In Use, Typeface, Connect, Suitcase | **Không xác minh được** (fetch lỗi hoặc không có nội dung). Đừng trích dẫn danh mục của các nguồn này trong spec. | chưa có |

Điểm chung rút ra:
- Class top-level ổn định xoay quanh: Serif, Sans, Slab, Script/Handwriting, Display/Decorative, Mono, Blackletter, Symbol.
- Font manager (FontBase) dùng **crowd tags** → mô hình hợp cho phần không đo được.
- Không ai (trong nguồn đã thấy) tự động hoá hoàn toàn: Google tag thủ công; manager dựa user tag. FontAtlas tự động hoá từ anatomy là lợi thế, nhưng phải hiển thị confidence.

---

## 3. Khoảng trống so với taxonomy hiện tại

Hiện tại: Serif{humanist_oldstyle, transitional, didone, slab, glyphic}, Sans{grotesque, neo_grotesque, geometric, humanist}, Mono{serif_mono, sans_mono}, Script{formal, casual_brush, handwritten}, Blackletter, Symbol, Uncertain; trait: stencil, inline_outline, rounded, fat_face, pixel, decorative; usage Text/Display/UI.

| Thiếu | Tồn tại ở đâu | Đề xuất |
|---|---|---|
| Garalde vs Venetian (đang gộp vào `humanist_oldstyle`) | Vox, Google (Old Style Garalde / Humanist Venetian), IBM | Tách 2 subclass `venetian`, `garalde` hoặc giữ gộp + tag. Với designer VN, gộp "oldstyle" vẫn đủ; tách nếu đo được trục stress |
| Slab subtypes | Vox (Clarendon/Egyptian), Google (Clarendon, Humanist, Geometric), IBM (Typewriter, Stub) | `slab` → `clarendon`, `egyptian` (geometric), `typewriter`, `humanist_slab` |
| Wedge serif / Latin / Tuscan | Google (Tuscan), Vox-ish (Glyphic), IBM 5-6 Dutch (wedge) | `wedge` (subclass nếu tách được), `latin` (display: serif tam giác), `tuscan` là trait/display |
| Flare / glyphic sans | Google /Sans/Glyphic; PANOSE 14 Flared | Sans `flared` (Optima-like) tách khỏi serif `glyphic`; **inscribed/Roman capitals** là serif glyphic |
| Small serif / micro serif | không có hệ nào có nhãn riêng [I] | Xem mục 5.3 |
| Reverse contrast / "Italienne" | Dixon facets [I]; Google không có | Trait `reverse_contrast` (đo được: ngang dày hơn dọc) |
| Semi-serif / semi-sans | không chuẩn; một số catalog gọi "Semi-serif" [I] | Hybrid (mục 5) |
| Sans: rounded, squared/techno, superelliptical | Google (Rounded, Superellipse, Techno), PANOSE 15 Rounded | Sans `rounded` (hiện chỉ trait), `squarish` (superellipse/techno) |
| Display genres: Art Deco, Art Nouveau, Wood type, Western, Psychedelic, Retro, Brutalist | Google (Art Deco, Art Nouveau, Woodtype, Techno, Wacky, Distressed, Medieval, Tuscan) | Display class + `genre` tag (không đo được tự động ở mức chắc chắn) |
| Script: copperplate/roundhand, monoline, signature, upright, uncial | IBM (Uncial, Monotone Joined, Brush), Google (Upright, Informal, Formal) | Script{formal_copperplate, brush, monoline, signature_casual, upright, calligraphic} |
| Mono: coding vs typewriter | IBM Typewriter | Mono{sans_mono, serif_mono, typewriter, coding} — "coding" đo bằng ligature `calt` + slashed zero [I] |
| Pixel/bitmap, stencil | Google (Pixel, Stencil), Font Squirrel (Pixel) | Giữ trait; thêm class Display nếu chủ đạo |
| Handwriting (không nối, marker) | Google "Handwriting" | giữ `handwritten`, thêm `marker`/`monoline` |
| Gaelic / Uncial / Insular | Vox mở rộng | `uncial` dưới Script hoặc Blackletter-adjacent; hiếm với designer VN, ưu tiên thấp |

**Vietnamese-relevant [V/I]**:
- Hãng VN: BeauType (Beau Agency, Hà Nội; BT Beau Sans "geometric humanist sans", BT Oateria "display sans") [V]: https://designbeep.com/2026/03/17/bt-beau-sans-font/ ; https://www.myfonts.com/collections/bt-oateria-font-beautype . Be Vietnam Pro, Phudu (Dương Trần, trên RightFont/Google) là font VN đáng làm fixture [I: Be Vietnam Pro là OFL/Google Fonts từ kiến thức nền; Phudu xác nhận tên qua https://rightfontapp.com/family/phudu].
- Không tìm thấy thuật ngữ chuẩn tiếng Việt cho taxonomy; dân thiết kế VN dùng "có chân / không chân (serif/sans-serif)", "thư pháp", "viết tay", "display / trang trí" [V: nguồn tìm kiếm chỉ xác nhận phân loại quốc tế, không có thuật ngữ VN chuẩn]. **Quyết định**: giữ EN label + gloss VN, đúng memory "Font terms in English".
- Quan trọng hơn phân loại: **hỗ trợ tiếng Việt (stacked diacritics)** – với font script/display, nhiều font không có glyph VN hay dấu chồng lên nhau xấu. Không thuộc taxonomy nhưng nên là bộ lọc/badge riêng (đã có language badge).
- Non-Latin (tóm tắt): Vox/ATypI thêm nhóm "Non-Latin" [V]; DIN có "Fremde Schriften" [I]. Với FontAtlas: gán class theo **script chính** (Latin/CJK/Arabic/Devanagari...) làm trục riêng, không cố áp serif/sans Latin cho CJK (Mincho/Song/Ming = serif, Gothic/Hei = sans là quy ước thông dụng [I]).

---

## 4. Đề xuất taxonomy FontAtlas (2 tầng + trait + tag)

Nguyên tắc: (a) class/subclass chỉ gồm thứ **đo được từ outline hoặc metadata đủ tin cậy**; (b) những thứ chủ quan (mood, genre thời đại) là tag; (c) mỗi font có `class`, `subclass` (có thể null), `secondary_class` (hybrid), `confidence`, `source` (auto/panose/os2/name/user); (d) designer thấy ≤ 8 class + ≤ ~5 subclass mỗi class.

### 4.1 Cột "Đo được?" ký hiệu
- **A** = tự động đáng tin (đo từ outline, ngưỡng rõ)
- **A~** = tự động nhưng cần confidence, dễ nhầm; hiện kèm "có thể"
- **M** = cần user/community tag (không đo được)
Fixture ưu tiên font OFL/Google Fonts.

### 4.2 Serif

| Subclass | Định nghĩa 1 dòng | Ví dụ OFL/Google | Tín hiệu đo | Mức |
|---|---|---|---|---|
| `venetian_oldstyle` (humanist oldstyle) | Contrast thấp, stress nghiêng, e có gạch xiên (Venetian) | EB Garamond (garalde), Cardo, IM Fell | contrast thấp-vừa, stress angle ≥ ~15°, bracketed foot, x-height vừa; **bar nghiêng của e** (phân biệt Venetian) | A~ (garalde vs venetian: A~/M) |
| `garalde` (tuỳ chọn tách) | Aldine/Garamond: stress nghiêng nhẹ, contrast vừa, e ngang | EB Garamond, Cormorant, Crimson Pro | như trên, e bar ngang | A~ |
| `transitional` | Stress gần đứng, contrast vừa-cao, serif bracketed mảnh | Libre Baskerville, Source Serif, Merriweather (Google gán Transitional) | stress 0-8°, contrast vừa-cao, bracket có | A |
| `didone` | Contrast cực cao, serif hairline không bracket, stress đứng | Playfair Display, Bodoni Moda, Abril Fatface (fat face) | contrast ratio rất cao, serif thickness rất mỏng, stress ~0° | A |
| `slab` (đã có) → xem 4.3 | | | | |
| `glyphic` | Gợi chữ khắc đá: serif nhỏ/tam giác, nét thon, hay chỉ có caps đẹp | Cinzel, Marcellus, Trajan-like | serif nhỏ, modulation theo "nét thon", caps rộng, lowercase yếu/không có | A~ |
| `wedge` (Latin/Wedge serif) | Serif hình nêm/tam giác nhọn, thường display | Cinzel Decorative? (không chắc) → fixture cần chọn tay; Google tag Tuscan/Medieval cho vài font | độ nhọn serif (góc nêm) | A~/M |
| `scotch`/`fat_face` | Didone dày, contrast mạnh, TK19 | Abril Fatface, Chonburi | weight rất nặng + contrast cao | A (đã có trait `fat_face`; giữ trait) |

Ghi chú: Google có `Scotch`, `Modern`, `Fat Face`, `Didone` riêng [V]; với designer, `didone` + trait `fat_face` + tag `scotch` đủ.

### 4.3 Slab

Tách khỏi Serif thành class riêng (Vox "Mechanistic", DIN 5 "Serifenbetont", IBM 4+5, Google "Slab" đều có) [V].

| Subclass | Định nghĩa | Ví dụ | Tín hiệu | Mức |
|---|---|---|---|---|
| `clarendon` | Slab có bracket, contrast nhẹ, dáng "báo chí/miền Tây" | Arvo? (không phải; geometric) → dùng **Alfa Slab One**? (egyptian dày); fixture Clarendon OFL: *Rokkitt*? sai. Lưu ý: ít Clarendon OFL; **cần chọn fixture thủ công** | bracketed + contrast nhẹ + serif dày | A~ |
| `egyptian` (geometric/monoline slab) | Serif vuông không bracket, nét đều | Roboto Slab, Zilla Slab, Arvo, Bitter | unbracketed, contrast ≈ thấp | A |
| `humanist_slab` | Slab có nét thủ công/axis nhẹ | Bitter, Crete Round? (rounded), Josefin Slab (geometric) | contrast nhẹ + stress nghiêng | A~ |
| `typewriter` | Slab + monospace hình máy chữ | Courier Prime, Special Elite (distressed) | monospaced + slab | A (Mono∩Slab) |

Ghi chú: thành thật là tôi chưa xác minh từng tên font ở bảng này có tag Google tương ứng; nhóm fixture cần kiểm tra bằng `google/fonts/tags/all/families.csv` (không fetch được thư mục `all/` trong phiên này).

### 4.4 Sans

| Subclass | Định nghĩa | Ví dụ OFL | Tín hiệu | Mức |
|---|---|---|---|---|
| `grotesque` | Sans TK19, contrast vừa, terminals ngang, G có spur | Work Sans, Libre Franklin, Public Sans | contrast nhẹ-vừa, terminal ngang, bầu dục, g double/single | A~ |
| `neo_grotesque` | Đều đặn, terminals ngang, đơn giản, Helvetica-ish | Inter, Roboto, Arimo, Albert Sans | contrast rất thấp, terminals ngang, aperture hẹp, x-height cao | A |
| `geometric` | Cấu trúc tròn/thẳng, o gần tròn, a single-story hay gặp | Montserrat, Poppins, Outfit, Jost, Nunito Sans? (humanist) | o tỉ lệ gần tròn, a/g single-storey, aperture lớn, uniform stroke | A |
| `humanist` | Gốc thư pháp, contrast nhẹ, terminals nghiêng | Open Sans, Source Sans 3, Noto Sans, Fira Sans | stroke modulation nhẹ, terminal chéo, a/g double | A |
| `flared` (glyphic sans) | Nét loe ở cuối như Optima, không có serif thật | Marcellus SC? → fixture: Belleza, Federo; Google /Sans/Glyphic | stroke width phình ở cuối nét, taper, contrast nhẹ | A~ |
| `rounded` | Terminal bo tròn | Nunito, Varela Round, Quicksand | terminal bán kính ≈ nửa stroke (hiện là trait `rounded`) | A (nâng lên subclass; giữ trait) |
| `squarish` (superellipse/techno) | Bowls vuông bo góc (squircle), hơi "công nghệ" | Orbitron? (techno), Michroma, Exo 2, Saira | độ "vuông" của O/C | A~ |

Tên fixture tôi liệt kê từ kiến thức nền [I]; cần kiểm tra bằng tag Google trước khi đóng băng test eval.

### 4.5 Script (nên bỏ "casual_brush" chung chung, dùng trục joined × contrast)

| Subclass | Định nghĩa | Ví dụ OFL | Tín hiệu | Mức |
|---|---|---|---|---|
| `formal_copperplate` | Roundhand/spencerian, contrast cao, nghiêng đều, nối | Pinyon Script, Great Vibes, Italianno | nghiêng ≥ ~12°, contrast cao, kết nối (`curs`/ligature), swash | A~ |
| `brush` | Nét cọ, contrast vừa, bất thường | Kaushan Script, Yellowtail, Pacifico | stroke variation không đều, terminal thon | A~ |
| `monoline_script` | Nối, nét đều | Satisfy?, Courgette? (casual), Sacramento | contrast ≈ 0 + nghiêng + nối | A |
| `signature` (casual/handwriting-script) | Chữ ký, nhịp nhanh | Mr Dafoe, Allura? (formal) | khó phân biệt với brush | M |
| `upright_script` | Script nối nhưng đứng | Google /Script/Upright [V] | nghiêng ≈ 0 | A |
| `calligraphic_unjoined` | Chữ rời, bút bẹt | IM Fell?, Uncial Antiqua (uncial) | contrast kiểu broad-nib, không nối | A~ |

Ví dụ dùng tên font "?" là chưa chắc; chỉ Pinyon Script/Great Vibes/Sacramento/Kaushan Script/Pacifico/Yellowtail là font Google phổ biến [I]. Tag Google chỉ có Formal/Informal/Handwritten/Upright [V], nên subclass mịn hơn Google phải tự gán.

### 4.6 Handwriting (tách khỏi Script)

Google và Font Squirrel đều tách "Handwriting/Handdrawn" khỏi script nối [V]. Subclass: `casual_print` (Caveat, Patrick Hand, Kalam), `marker` (Permanent Marker, Rock Salt), `childlike`/`comic` (Gochi Hand, Comic Neue). Tín hiệu: không nối, stroke jitter, baseline wobble (đo được khi so sánh 2 lần vẽ cùng glyph? không có; dùng độ không đều giữa các glyph giống nhau như l/I/1) A~; "childlike/comic" là M.

### 4.7 Mono

| Subclass | Định nghĩa | Ví dụ | Tín hiệu | Mức |
|---|---|---|---|---|
| `sans_mono` / `serif_mono` (đã có) | | JetBrains Mono, IBM Plex Mono, Fira Code, Courier Prime | advance width đồng nhất (`post.isFixedPitch` + đo thực) | A |
| `coding` | Mono có tính năng lập trình: slashed/dotted zero, ligature `calt`, phân biệt l/1/I | JetBrains Mono, Fira Code, Source Code Pro | feature `calt`/`zero`, glyph khác nhau cho 0/O, l/1/I | A~ |
| `typewriter` | Mono + slab, nét không đều | Courier Prime, Special Elite | xem 4.3 | A~ |

### 4.8 Display / Decorative (class mới, thay cho trait `decorative` đơn)

Đây là chỗ Google chỉ dùng tag "Theme" [V], không phải class. Đề xuất: class `Display` gồm vài subclass **cấu trúc** đo được và một tầng `genre` tag không đo được.

| Subclass (đo được) | Định nghĩa | Ví dụ OFL | Tín hiệu | Mức |
|---|---|---|---|---|
| `stencil` | Cầu nối cắt ngang nét | Stardos Stencil, Allerta Stencil | topology: glyph O có > 1 contour dạng arc rời; Fontquant có chỉ số `stencil` [V] | A |
| `inline_outline` | Đường viền trong nét hoặc chỉ viền | Monoton, Bungee Inline | counter chứa contour lồng thêm | A |
| `shaded_3d` | Bóng/khối | Bungee Shade | contour phụ lệch cố định | A~ |
| `pixel_bitmap` | Lưới điểm | Press Start 2P, Silkscreen | toạ độ lưới đều, góc 90° | A |
| `blackletter` (giữ class riêng) | Gothic/textura | UnifrakturMaguntia, Pirata One | broken strokes, angular, x-height thấp | A~ |
| `genre` tag (M): Art Deco, Art Nouveau, Wood type, Western, Retro/Vintage, Psychedelic, Brutalist, Techno, Medieval, Tuscan, Distressed, Blobby, Wacky | Google có sẵn Art Deco, Art Nouveau, Woodtype, Techno, Medieval, Tuscan, Distressed, Blobby, Wacky [V] | Poiret One (Art Deco), Monoton, Rye (Western), Abril/Alfa Slab (wood-ish) | Không đo được ổn định; xin tag user/community; có thể gợi ý từ tên (Deco, Wood...) |

### 4.9 Symbol & Uncertain

Giữ `Symbol` (PANOSE kind 5, IBM class 12, Google "Not text") và `Uncertain`. Thêm `Emoji/Color` là trait kỹ thuật (Google /Technology/Color) [V].

### 4.10 Trait độc lập (giữ + mở rộng)

| Trait | Đo? | Ghi chú |
|---|---|---|
| stencil, inline_outline, pixel, rounded, fat_face | A | đã có |
| shaded, tuscan | A~/M | |
| reverse_contrast | A | ngang dày hơn dọc |
| high_contrast, monoline | A | thay vì chỉ trong class |
| condensed, extended, slab_numerals... | A | đã có dưới dạng width; không đưa vào class |
| variable, color, has_italic, has_vietnamese | A | metadata, không phải taxonomy |
| decorative | giữ như cờ tổng quát "ornamental" | |

### 4.11 Mood / Use tags (M, community hoặc user)

Lấy cảm hứng từ Google Feeling [V]; **rút gọn xuống ~12** cho designer:
`Business`, `Calm`, `Elegant`(thay Sophisticated), `Playful`, `Friendly/Cute`, `Loud/Bold`, `Vintage/Retro`, `Futuristic/Tech`, `Rugged`, `Artistic`, `Formal`, `Handmade`.
Use: `Text`, `Display`, `UI` đã có; thêm `Editorial`, `Logo/Branding`, `Packaging`, `Signage` (M). Seasonal chỉ là tag phụ (Lunar New Year/Tết có giá trị cho designer VN [V: có trong Google]).

---

## 5. Hybrid, ranh giới, và "small serif"

### 5.1 Mô hình dữ liệu

```
class: Serif | Slab | Sans | Script | Handwriting | Mono | Display | Blackletter | Symbol | Uncertain
subclass: string | null
secondary: { class, subclass } | null      // hybrid
confidence: 0..1
evidence: [signal -> value]                // để debug/QA
```

Hiển thị: nếu `confidence < ngưỡng` hoặc có `secondary`, UI hiển thị "Sans · flared" hay "Serif/Sans (semi-serif)", kèm "!" note-mark + tooltip theo motif đã chốt (warning = red "!" ở góc).

### 5.2 Hybrid cụ thể

| Trường hợp | Ví dụ | Xử lý |
|---|---|---|
| Sans với flare (Optima-like) | Belleza, Federo (Google /Sans/Glyphic [V]) | class Sans, subclass `flared`; secondary Serif/glyphic |
| Semi-serif (serif ở vài glyph/độ dài nhỏ) | font "Rotis Semi Serif"-kiểu [I] | class theo **đa số glyph**; secondary = lớp còn lại; không tạo class mới |
| Serif nhưng slab mảnh | Josefin Slab, Zilla Slab | tín hiệu bracket + contrast; nếu mơ hồ → Slab/Serif với confidence thấp |
| Script ↔ Handwriting ↔ Display | Pacifico | dựa trên có nối + contrast; mơ hồ → secondary |
| Mono ∩ Slab (typewriter) | Courier Prime | class Mono, trait typewriter, secondary Slab |
| Display ∩ x | Bungee (sans display) | class Sans + trait display/shaded, hoặc class Display nếu ornament chiếm ưu thế |

Quy tắc: **một `primary` duy nhất để lọc/nhóm; `secondary` chỉ để hiển thị và tìm kiếm** (search "serif" có thể khớp font có secondary = serif nhưng ít ưu tiên).

### 5.3 "Small serif" bị đọc nhầm thành sans

Vấn đề: serif rất nhỏ/mảnh (micro serif; ví dụ các sans với terminals có "foot" nhỏ như một số font "sans with tiny serifs" hoặc glyphic sans) khiến bộ đo `serif_foot` ra ≈ 0 → nhận là sans. Không thấy hệ phân loại nào có nhãn riêng cho "small serif" [I: không tìm thấy trong nguồn], nên đây là bài toán của FontAtlas.

Đề xuất:
1. Đo **serif_foot ở độ phân giải outline** (không dựa vào pixel): tỉ lệ chiều dài serif / stem width tại chân I, l, n, h; và **độ lệch của chân so với stem trong glyph "i", "l", "I" ở cap** vs lowercase (một số font chỉ có serif ở caps/chỉ có ở i,j,l).
2. Tính serif score **theo từng glyph** rồi lấy phân phối: nếu median thấp nhưng p90 cao → `semi_serif` (secondary Serif).
3. Dùng tín hiệu phụ không phụ thuộc kích thước serif: **contrast + stress** (serif thực thường có contrast ≥ ngưỡng và bracket), **PANOSE serif style** (≠11-13 là có serif; 14 Flared, 15 Rounded không phải serif) và **OS/2 sFamilyClass** (class 1-5,7 là serif), nhưng coi chúng là bằng chứng phụ do dữ liệu nhiều font sai [I].
4. Ngưỡng hai tầng: `serif_foot < a` → Sans chắc; `a ≤ serif_foot < b` → "Uncertain/hybrid", không ép Sans; `≥ b` → Serif.
5. Eval riêng: bộ fixture "ranh giới" (small-serif, flared sans, semi-serif, slab mảnh) để tránh hồi quy khi chỉnh ngưỡng.
6. Với user: cho phép **override và lưu tag thủ công**, ghi `source=user`, không bị auto-classify ghi đè lại (khớp memory "Tool, not gatekeeper").

---

## 6. Kế hoạch triển khai gợi ý (ngắn)

1. **Đợt 1 (rẻ, giá trị cao)**: tách `slab` thành `clarendon/egyptian/typewriter`; thêm Sans `rounded`, `flared`; thêm class `Handwriting` tách khỏi Script; thêm `Display` với subclass cấu trúc (stencil/inline/pixel/shaded).
2. **Đợt 2**: `secondary_class` + confidence + evidence; test "boundary fixtures".
3. **Đợt 3**: mood/use tag + user tag + crowd tag (kiểu FontBase) cho phần M; `genre` tag cho Display.
4. Dùng Fontquant của Google để đối chiếu cách đo contrast/stencil/a-g storey [V: tồn tại trong tags_metadata]; xác minh thêm licence repo trước khi tái sử dụng code.

Nhãn UI: EN term + gloss VN ngắn + tooltip có hình minh hoạ (theo memory).

---

## 7. Giới hạn của nghiên cứu này

- Không lấy được: bài "Some type genres explained" (Kupferschmid), Thibaudeau/Bringhurst nguyên văn, danh mục chi tiết MyFonts/Fontstand/Monotype/Fontshare/Future Fonts/Typewolf/Fonts In Use/Typeface/Connect/Suitcase. Các mục đó đánh dấu [I] hoặc "không xác minh".
- Danh sách font ví dụ trong mục 4 chủ yếu từ kiến thức nền [I]; **cần đối chiếu với `google/fonts/tags/all/families.csv`** (thư mục `all/` không đọc được qua WebFetch; dùng `git clone --depth 1` trong worktree tạm) để biết font nào Google gán tag nào trước khi đóng băng fixture eval. Các ô có dấu "?" là chưa chắc.
- Bảng Google (mục 2.1) và IBM class (mục 1.5), PANOSE serif style (mục 1.4) là **[V] từ nguồn gốc**; phần còn lại là tổng hợp.

## Nguồn

- Vox-ATypI: https://en.wikipedia.org/wiki/Vox-ATypI_classification ; https://atypi.org/tag/vox/ ; https://blazetype.eu/blog/voxatyp-i
- DIN 16518: https://studyflix.de/allgemeinwissen/klassifikation-der-schriften-nach-din-16518-8552 ; https://www.macwelt.de/article/940776/neue-klassifikation.html
- Dixon: https://ualresearchonline.arts.ac.uk/id/eprint/1099/ ; https://ualresearchonline.arts.ac.uk/id/eprint/5726/ ; https://infodesign.emnuvens.com.br/infodesign/article/view/53
- PANOSE: https://monotype.github.io/panose/pan1.htm ; https://monotype.github.io/panose/pan2.htm
- OS/2 IBM classes: https://learn.microsoft.com/en-us/typography/opentype/spec/ibmfc
- Google Fonts tags: https://googlefonts.github.io/gf-guide/tags.html ; https://raw.githubusercontent.com/google/fonts/main/tags/tags_metadata.csv ; https://raw.githubusercontent.com/google/fonts/main/tags/tags_ui_details.textproto
- Adobe Fonts: https://blog.adobe.com/en/publish/2020/08/14/using-tags-to-find-the-perfect-font-for-the-job ; https://community.adobe.com/t5/adobe-fonts-discussions/search-by-keyword/m-p/12686794
- Font Squirrel: https://www.fontsquirrel.com/fonts/list/classification/slab_serif ; type.co.uk: https://www.type.co.uk/categories
- FontBase / RightFont: https://fontba.se/learn/awesome ; https://rightfontapp.com/docs/organize-fonts/create-tags-and-smart-tags
- BeauType / VN: https://designbeep.com/2026/03/17/bt-beau-sans-font/ ; https://www.myfonts.com/collections/bt-oateria-font-beautype ; https://rightfontapp.com/family/phudu
- Kupferschmid (profile, không phải bài genres): https://www.umprum.cz/en/web/about-umprum/science-and-research/research-projects/european-typographic-proofreader/indra-kupferschmid-linear-serif-typefaces-with-a-classicist-character
