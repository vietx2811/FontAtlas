# FontAtlas anatomy audit (core-0.3.0, nhánh dev/mvp)
Mẫu: 975 face (566 file: Google OFL 190, hệ thống ~370, fixture 6); 639 face Latin a–z dùng cho đo trục; 8 face fontTools không đọc được (thiếu OS/2/head, gồm Courier.ttc) bị loại. Dump chạy bằng CARGO_TARGET_DIR riêng (không ghi vào repo). Variable font đo ở đúng `location` của tool (wght400/wdth100/opsz14), fontTools getGlyphSet(location=).

| # | Trục | Chuẩn độc lập | n | Kết quả (ĐO) |
|---|------|---------------|---|--------------|
|1|x_height_em|bbox 'x' fontTools|639|100% trong 0,1%|
|1|x_height_em|OS/2 sxHeight|588|<=1%: 76,7%; <=3%: 86,7%; <=10%: 94,9%. Lệch >10%: Salsa, UnifrakturCook, Boogaloo (OS/2 sai), Khmer/Gurmukhi MN|
|1|cap_height_em|bbox 'H'|639|100%|
|1|cap_height_em|OS/2 sCapHeight|590|<=1%: 81,4%; <=3%: 93,1%; <=10%: 96,9%|
|2|slant_deg|post.italicAngle|115 nghiêng|100/115 trong 2°. Lệch: Brush Script +16,8, Snell Roundhand +6..9, Didot Italic -7,6, Hoefler Italic -7,5 (chữ nghiêng cursive hoặc italicAngle khai sai)|
|2|slant_deg|upright (ia=0)|523|487 trong ±2°. 36 còn lại: script/oblique mà post.italicAngle=0 (Helvetica Oblique -12, Avenir Next Italic -10, Roboto Italic -9,8): tool đúng, metadata sai|
|3|width_cv|monospace (isFixedPitch hoặc advance a–z bằng nhau)|38|38/38 = 0,000; non-mono min 0,143. Tính lại bằng fontTools: lệch max 0,0004 (trừ Skia VF 0,006)|
|4|serif_foot|nhãn: gcat Google, Panose 11–13, tên|291 upright|Sans <1,3: 172/173; serif >1,6: 115/118; 1 mid. Sans p90=1,00; serif p10=1,88|
|4|serif_foot (italic)|như trên|85|serif italic: 37/45 bị đọc thành sans (<1,3), chỉ 6 đúng|
|5|contrast|tính lại bằng khoảng cách gần nhất outer–inner contour|507|Spearman 0,958, tỉ lệ trung vị 0,999|
|5|contrast theo nhóm (median)|Didone 6,2; oldstyle 2,9; transitional 2,3; sans 1,22 (min 1,08, max 1,35)|69|thứ tự đúng; transitional/oldstyle chồng nhau|
|6|stress_deg|tính lại độc lập (lỗi tuyến tính hài hòa trên khoảng cách gần nhất)|260|trung vị |lệch| 2,3°; 53% trong 3°; tương quan thấp do outlier ±90|
|6|stress_deg|phân bố upright, contrast>=1,3|243|=0: 83 (34%), 1..25: 137, <-1: 9, >25: 5|
|7|a_storeys|danh sách tay (kiểm bằng render)|35|35/35|
|7|g_storeys|danh sách tay|35|35/35|
|8|aperture|humanist vs grotesque|59|AUC 0,85; trung vị 98° vs 56° (Helvetica 49, Arial 56, Open Sans 105, PT Sans 110)|
|8|aperture_e|như trên|59|AUC 0,75; 0,26 vs 0,18; Spearman với aperture 0,47|
|9|VietnameseCoverage.missing|cmap fontTools|975|970 trùng tuyệt đối; 5 lệch = font symbol (Webdings, Wingdings x3, Hoefler Ornaments): skrifa ánh xạ symbol cmap. supports_vietnamese nhất quán 100%. 30 font (15 thiếu + 15 đủ) lệch 0|
|10|Failed|toàn mẫu Latin 639|—|stress_deg low_contrast 299 (47%, theo thiết kế); aperture_e degenerate 11; terminal 2; Raleway Dots 5 trục; còn lại 0. Non-Latin: 331 no_latin_lowercase. Không lỗi tool|

## Source Code Pro g
Render wght=400 bằng PIL/FreeType, đếm lỗ nền kín: 2 lỗ (bát trên + vòng dưới) và ảnh thấy g binocular có ear. g=2 ĐÚNG, không phải lỗi.

## Sai nhãn của chính tôi (hand list) đã sửa sau khi render
Poppins a=1, Monaco a=1, Lato g=2, Trebuchet MS g=2 là đúng (tool đúng, trí nhớ tôi sai). Cormorant SC loại (chữ thường là small cap).

## Lỗi/Điểm yếu xác định
1. [Important] geom.rs:56-58,87-88: deskew dùng post.italicAngle; face oblique có ia=0 (Avenir Next Condensed Italic, Helvetica Oblique, Roboto Italic, script) không được dựng thẳng -> stress_deg bẩn (Avenir Next Cond Medium Italic ra 20°, Caveat 25°). Sửa: nếu |ia|<0,5 thì dùng slant đo từ 'l'/'I' (stroke.rs:143) làm deskew.
2. [Important] stroke.rs:84 ngưỡng c<1.3 quá thấp: 55/209 font sans có stress "Estimated" (sans median contrast 1,22, tới 1,35; Lato 1,32, Fira 1,35). Nâng 1,6–1,8 (transitional min thực 1,89).
3. [Important] stroke.rs:38-42,86: font condensed ra ±90° (Bebas Neue, DIN Condensed Bold, Oswald/Impact có contrast giả 1,9–2,4). Radial rays trên 'o' hẹp đo sai. Gate |tilt|>45 -> Failed(Degenerate); contrast của font hẹp cần chuẩn hóa theo tỉ lệ bbox.
4. [Important] stroke.rs:166-172: serif_foot trên italic serif: 82% đọc thành sans vì italic không có chân. Gate theo slant/italic (Estimated hoặc đo serif đầu ở 'i','l' đỉnh).
5. [Minor] stroke.rs:86: tilt.round() sinh -0.0 (38 face). Dùng `+0.0` hoặc `if r==0.0 {0.0}`.
6. [Minor] serif_foot: Inconsolata 4,11 (i/l slab trong mono sans), Montserrat Underline 3,85 (gạch chân là mực ở baseline), Alegreya 1,20 (serif oldstyle, chân nhỏ).
7. [Minor] aperture=0 Measured cho 8 face decorative (Chalkduster, Phosphate Inline…) nên là Degenerate. aperture_e Degenerate 11 face script/decor: chấp nhận.
8. [Minor] vietnamese.rs: cmap skrifa với font symbol cho present sai (Webdings). Lọc theo cmap platform symbol.

## Nghi vấn "stress_deg luôn 0" -> BÁC BỎ
Garamond 13°, Cormorant 14°, Palatino 12°, Cardo 12°, Sorts Mill Goudy 13°, Iowan 13°, Alegreya 21° (render EB Garamond nghiêng trái thấy rõ). Dương = nghiêng trái đúng quy ước. Giá trị 0 là đúng cho Playfair/Bodoni/Didot/Baskerville/Libre Caslon/Hoefler. Giá trị oldstyle thấp hơn kỳ vọng 10–25° chỉ với Caslon (0), Hoefler (0), Gelasio (1), IM FELL (-2) (độc lập cho Caslon -0,2, Hoefler 0,0, Gelasio 1,5: đồng thuận; IM FELL độc lập -30: bất đồng).

## Suy luận (không đo)
- Nhãn serif/sans lấy từ metadata Google/Panose/tên, có sai số nhãn.
- Nhóm Didone/oldstyle là phân loại tay theo tên.
- Aperture "humanist/grotesque" theo tên họ font, Hiragino bị nhóm nhầm (làm AUC thấp, khoảng 0,85).
