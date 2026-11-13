# Changelog

Mọi thay đổi đáng chú ý của Font Atlas được ghi ở đây.
Định dạng theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/). Số phiên bản theo [Semantic Versioning](https://semver.org/lang/vi/).
Thuật ngữ typo giữ nguyên tiếng Anh, giống trong app.

## [0.4.0] — 2026-11-13

Pairing Desk mới, kéo thả font vào Sidebar, onboarding mới trên nền kính macOS, xử lý font VF trùng tên với bản static, theo dõi thư mục font trên ổ cloud.

### Thêm mới
- **Kéo thả font vào Sidebar:**
  - Kéo một hay nhiều font từ lưới thả vào Dự án hoặc Case. Con trỏ hiện số font đang kéo, vùng thả sáng lên, có Hoàn tác.
  - Case nay nằm ở Sidebar.
- **Pairing Desk:**
  - Nút **+** hiện khi rê chuột qua tên font, để thêm font vào combo.
  - **+ Role**: thêm vai tuỳ ý (gợi ý Display, Heading, Body, Caption…), đổi tên hoặc bỏ vai, có Hoàn tác.
  - Cột 2 là nơi gợi ý và chọn font, với 5 nguồn (phím 1–5): font đã pin trong Case hoặc Dự án, font tương tự, font hợp vai, font vừa xem…
  - Tên combo đặt lên đầu.
  - Chọn kiểu combo (UI, Editorial, Brand, Packaging/Menu, Poster/Campaign) để nhận combo gợi ý. Có thể nhận cả combo hoặc từng vai.
  - Tag trục biến thiên (variable axes), vòng tiếng Việt, hàng Glyphs/Δ.
- **Pair theo vai và phong cách:**
  - Mỗi font được đánh giá độ hợp vai: Hợp, Tạm hoặc Yếu, kèm lý do, không có điểm số.
  - Chọn phong cách pair: tương phản serif + sans, cùng họ/superfamily, cùng cấu trúc, cùng thời kỳ, display + chữ trung tính.
- **Font VF trùng tên với bản static:**
  - Tên hiển thị trong app thêm hậu tố "VF". File font không bị sửa.
  - Khi bật font trùng tên, app hỏi một lần: dùng VF, dùng static hay giữ cả hai (có cảnh báo khi giữ cả hai). Có "nhớ lựa chọn".
- **Ổ cloud:** app theo dõi thay đổi trong thư mục font trên ổ cloud và tự quét lại (bật/tắt trong Cài đặt), kèm gợi ý "Luôn giữ trên máy".
- **Onboarding mới:**
  - Bố cục hai mảng. Bên trái là nội dung trên nền kính thật của macOS; máy chưa hỗ trợ thì dùng nền mờ.
  - Bên phải là minh hoạ phủ kín, đổi theo từng bước.
  - Chữ đạt độ tương phản 4.5:1 ở cả chế độ sáng và tối.

### Cải thiện
- **Phân loại:**
  - Font sans có chữ l đuôi cong (Bellota, Mooli, Ruluko, Aboreto, Genos…) trước đây bị để "Chưa chắc chắn", nay được nhận là Sans.
  - Serif chân nhỏ (Alegreya, Cormorant Upright, DM Serif Display…) được nhận đúng là Serif.
  - Không cần quét lại thư viện: phân loại được tính lại từ số đo đã lưu.
  - Một số font viết tay vẫn có thể ra "Chưa chắc chắn": thà không gán còn hơn gán sai.
- **Gợi ý pair** dùng lớp phân loại dự đoán khi font còn "Chưa chắc chắn".
- **Sidebar** tự cập nhật Dự án và Case khi cửa sổ được chọn lại.

### Kiểm thử
- Bỏ phụ thuộc thời gian trong test.
- Native QA phủ các tính năng mới, gồm kiểm nền kính của onboarding: đúng một lớp kính, kính theo kích thước cửa sổ và theo chế độ sáng/tối, rời onboarding thì nền cũ trở lại.
- CI cục bộ bỏ qua các bước có nội dung không đổi, nên lượt chạy lại chỉ còn vài phút thay vì vài giờ.

### Lưu ý
- Auto-activation trên Illustrator, InDesign và Affinity thật chưa kiểm xong; Photoshop đã kiểm.
- Theo dõi ổ cloud đã kiểm bằng thư mục tạm, chưa kiểm trên OneDrive thật.
- Các ngưỡng phân loại mới được chọn sau khi xem kết quả trên chính bộ kiểm, nên số đo cải thiện là trên tập đã chỉnh.

## [0.3.1] — 2026-11-12

Bản sửa lỗi cho nguồn font trên ổ cloud, hộp thoại xung đột và kiểm thử.

### Sửa
- **Hiện trong Finder cho Source:** mở đúng thư mục Source. Trước đây báo "invalid" với mọi Source. Ổ chưa kết nối thì báo bằng lời dễ hiểu.
- **Tải về máy (Download now)** cho Source trên ổ cloud (iCloud, OneDrive, Google Drive, Dropbox):
  - Có ở menu Source, cạnh dòng "N chưa tải về" ở thanh bên và trong Cài đặt.
  - Tối đa 4 file cùng lúc (dùng chung giới hạn với xem trước), có tiến độ, huỷ trong khoảng một giây.
  - Tải xong app quét lại đúng Source đó; đang quét dở thì xếp thêm một lượt.
- **Hộp thoại xung đột kích hoạt** gọn hơn: luôn có một dòng cảnh báo hiển thị, chi tiết thì gập lại.
- **⌘A** chọn được trong thư viện ngay sau khi đóng hộp thoại Gỡ nguồn.

### Kiểm thử
- Native QA chờ theo điều kiện thay vì chờ cố định, nên không còn đỏ giả khi máy bận.
- Hai test tải model không còn phụ thuộc thời gian.

### Lưu ý
- "Tải về máy" đã kiểm bằng thư mục tạm và file cloud giả lập, chưa kiểm trên ổ cloud thật.

## [0.3.0] — 2026-11-12

Bản lớn: bật font tự động khi app thiết kế cần, tiếng Anh, giải quyết trùng lặp, thay bản đang bật bởi app khác, phân loại mới và nhiều cải tiến thư viện.

### Thêm mới

**Bật font tự động (Settings → Bật font tự động)**
- Khi Photoshop, After Effects và các app macOS khác hỏi một font chưa bật, Font Atlas tìm trong thư viện và bật font đó (theo phiên), không cần bạn làm gì.
- Illustrator: cài một panel nền nhỏ (Font Atlas tự ký trên máy bạn) để tự bật font khi mở tài liệu. InDesign và InCopy: cài một script khởi động. Cài/gỡ từ Settings, có xác nhận nói rõ sẽ ghi gì; gỡ chỉ xoá phần của Font Atlas.
- Nhật ký các lần tự bật; cảnh báo khi RightFont, Typeface, FontBase… đang chạy. Nếu một trình quản lý font khác nhận yêu cầu font trước Font Atlas, trang này báo rõ và có nút **Kiểm tra lại**.
- Font trên ổ cloud được tải về trước khi bật (hạn chờ ngắn để app thiết kế không bị treo).
- Đã kiểm với Photoshop thật. Panel Illustrator và script InDesign/InCopy đã kiểm từng phần (định dạng, chữ ký, cài/gỡ) nhưng chưa đo trọn trên app thật; Settings ghi rõ mức đã kiểm của từng app.

**Thay bản đang bật bởi app khác**
- Khi bật một font mà bản cùng tên đã được bật ở nơi khác, hộp xung đột có thêm **Thay bằng bản này**: tắt bản cũ và bật bản của Font Atlas, kèm cảnh báo khi app kia có thể bật lại. Có hướng dẫn tắt trong RightFont, Typeface, FontBase…
- Mọi lần thay đều có thể **Khôi phục bản cũ** (Settings → Kích hoạt). Font hệ thống và các thư mục Fonts không bao giờ bị đụng tới.

**Trùng lặp**
- Trang Trùng lặp viết lại bằng ngôn ngữ thường: "Bản sao giống hệt", "Bản cũ và bản mới", "Cùng font, khác định dạng"; hai bản đặt cạnh nhau cùng dòng mẫu, kèm lý do nên giữ bản nào.
- **Giải quyết** từng nhóm hoặc **Giải quyết các nhóm an toàn**, luôn có màn xem trước. Mặc định chỉ **ẩn** bản thừa (không đụng file); đưa vào **Thùng rác** là lựa chọn phụ, xác nhận từng tệp. Yêu thích, bộ sưu tập, thẻ, Case và cặp font được chuyển sang bản giữ lại.
- Hoàn tác được mọi lần giải quyết (tab Đã giải quyết); bỏ qua nhóm; thứ tự nguồn ưu tiên kéo-thả trong Settings.

**Tiếng Anh**
- Toàn bộ giao diện có tiếng Anh. Mặc định theo ngôn ngữ của macOS; đổi ở onboarding hoặc Settings → Chung, áp ngay không cần mở lại app.

**Phân loại**
- Thêm Slab, Handwriting và Display (chữ trang trí có cấu trúc riêng) thành class; nhiều subclass mới. Font lai có dấu "!" chỉ class phụ; bộ lọc **Gồm cả font lai** và **Chỉ nhãn chắc chắn**.

**Thư viện**
- **Nhóm theo Family** (⌥G, lưới và danh sách), mỗi nhóm có số kiểu, thu gọn được.
- Sắp xếp theo **Năm phát hành** (ước lượng từ bản quyền/ngày tạo trong font) và **Ngày tạo file**.
- Đổi tên (F2) và xoá thẻ; gỡ nguồn với màn xem trước số liệu (không xoá file, dự án/thẻ được giữ); **Bật tất cả** cho Dự án và Case.
- Chữ mẫu mặc định mới, trung tính, theo ngôn ngữ.

**Giao diện**
- Splash gọn hơn; onboarding với minh hoạ vuông, chỉ báo bước dạng chấm; màn **Phím tắt & tính năng** trước khi vào thư viện, mở lại bằng phím ?.

### Sửa lỗi
- Tooltip thuật ngữ đóng ngay khi rời chuột.
- Nhiều sửa nhỏ về bố cục, focus bàn phím và trình đọc màn hình.

### Lưu ý khi nâng cấp
- Lần mở đầu, dữ liệu người dùng được nâng lên phiên bản mới (có sao lưu trước). Bản 0.2.x không mở được dữ liệu đã nâng.

## [0.2.3] — 2026-11-11

### Thêm mới

**Pairing Desk: so sánh ngay trong Desk**
- Desk có ba chế độ: **Pairing**, **So sánh Font** và **So sánh Combo**; không phải rời Desk để mở Compare.
- So sánh Font: font đang dùng làm mốc, thêm cột từ gợi ý ghép hoặc tìm trong cả thư viện; ghim cột; bấm **Đặt** để đưa font vào vai đang chọn.
- So sánh Combo: các Combo của Case đặt cạnh nhau với trang mẫu thu nhỏ, quan hệ giữa các vai và độ phủ tiếng Việt.
- Font còn trên ổ cloud được tải về để xem như ở lưới (tối đa 4 font cùng lúc, có hạn chờ, theo công tắc trong Cài đặt); lỗi hoặc đang tải có ghi chú riêng.

## [0.2.2] — 2026-11-11

### Thêm mới

**Classification mới (10 class)**
- Thêm 3 class: **Slab** (chân dày, khối), **Handwriting** (chữ viết tay, các chữ rời nhau), **Display** (chữ trang trí có cấu trúc riêng: stencil, inline/outline, pixel).
- Subclass mới cho từng class: Serif (venetian oldstyle, garalde, transitional, didone, glyphic), Slab (clarendon, egyptian), Sans (grotesque, neo-grotesque, geometric, humanist, rounded), Mono (sans, serif, typewriter, coding), Script (copperplate, brush, monoline, upright), Handwriting (casual print, marker). Subclass nào máy chưa đo chắc thì để trống, không đoán.
- **Font lai**: font mang dáng của hai class (ví dụ Display có khung Sans) có dấu "!" cho biết class phụ.
- Bộ lọc thêm **Gồm cả font lai** (khớp cả class phụ) và **Chỉ nhãn chắc chắn** (bỏ nhãn máy chưa chắc; nhãn bạn tự sửa luôn tính là chắc).
- Mọi class và subclass có tooltip tên tiếng Anh kèm giải thích; các thuật ngữ mới có hình minh hoạ.
- Thẻ công dụng "Display" đổi tên thành **Headline** để không trùng với class Display.
- Class bạn đã tự sửa được chuyển sang nhãn mới, không mất (ví dụ slab → Slab, handwritten → Handwriting).

**Thư viện**
- Panel **Hiển thị** (⌘J): chọn thông tin hiện trên thẻ font — định dạng, ngôn ngữ, giấy phép, Variable.
- **Sắp xếp** theo tên, ngày thêm, Weight, hoặc độ khớp khi đang tìm; đổi chiều tăng/giảm. Ở Gần đây và Đang bật thứ tự giữ theo thời gian.
- **Menu chuột phải ở Sidebar**: nguồn (mở, hiện trong Finder, quét lại), dự án (mở, đổi tên F2, tắt tất cả font, xoá), thẻ, Gần đây (xoá lịch sử), Đang bật (tắt tất cả), Đang so sánh (tiếp tục, đóng phiên).
- Lọc theo thẻ nằm chung với các bộ lọc khác: hiện chip, được tính trên phễu và gỡ được như bộ lọc thường.

### Sửa lỗi
- Splash: phím bấm để bỏ qua splash không còn lọt xuống onboarding.
- Splash đảo cả các font trên ổ cloud đã tải về máy (font chưa tải vẫn không bị kéo về).
- "Tắt tất cả" báo số font đã tắt và số font lỗi; hỏi lại theo số font đang bật.
- Đổi tên dự án bị trùng hoặc để trống: app báo và giữ ô nhập.

## [0.2.1] — 2026-11-11

Bản này thêm Pairing Desk, tìm kiếm nâng cao, độ phủ ngôn ngữ, hỗ trợ font trên ổ cloud và giao diện mới (màu theo macOS, onboarding mới). Bật font tự động khi app thiết kế cần (Adobe, Affinity…) sẽ có ở 0.3.0.

### Thêm mới

**Tìm kiếm nâng cao (Advanced search)**
- Bảng Advanced với 18 trục số đo chia 6 nhóm (x-height, contrast, width, aperture…), mỗi trục có histogram của thư viện và thanh kéo hai đầu; gõ số trực tiếp được.
- Lọc theo Weight class / Width class, khớp cả font variable có trục wght/wdth phủ khoảng đang lọc.
- Tóm tắt bộ lọc ngay trong ô tìm kiếm; mục **Đang lọc** liệt kê từng điều kiện. Backspace hoặc ⌘⌫ bỏ một nhóm, có Hoàn tác; ⌘Z lùi tới 20 bước.
- Phím tắt ⌥⌘F mở bảng Advanced.

**Ngôn ngữ (Hyperglot)**
- App tính độ phủ ngôn ngữ của từng font theo dữ liệu Hyperglot. Lọc theo một hay nhiều ngôn ngữ (có tất cả / có ít nhất một).
- Thẻ font có vòng phủ ngôn ngữ thay cho số ký tự thiếu; tooltip nêu mức phủ. Font chưa đủ ngưỡng được ghi "dưới mức", không hiện số giả.

**Pairing Desk**
- Từ ô xem nhanh, bấm **Pair** để mở Pairing Desk với font đang xem làm neo: trang mẫu dàn sẵn tiêu đề và đoạn văn, danh sách ứng viên cho từng vai (tiêu đề, nội dung…).
- Dùng ↑↓ để thử từng ứng viên ngay trên trang, Enter để đặt vào vai.
- Dải chi tiết so font **Đang dùng** với font **Đang thử**: 8 số đo kèm Δ (mỗi thuật ngữ có tooltip). Phím I thu/mở dải; ⇧C mở Compare và đóng Compare là quay lại đúng chỗ trong Desk.
- **Combo**: nhiều bộ font cho cùng một Case; tạo mới, nhân bản, đổi tên, đặt làm chính, xoá (có hỏi lại). Phím [ ] chuyển Combo.

**Font trên ổ cloud (OneDrive, iCloud, Dropbox, Google Drive…)**
- App nhận ra font chỉ còn bản giữ chỗ trên cloud (chưa tải về máy). Khi quét, app không kéo các file này về; dữ liệu phân tích cũ vẫn dùng được.
- Thẻ font đang hiện trên màn hình tự tải font về để xem trước (tối đa 4 font cùng lúc, có hạn chờ), kèm dấu "Trên cloud — đang tải". Có thể tắt trong Cài đặt → Thư viện và nguồn.
- Bật font nằm trên cloud: app tải file về trước rồi mới bật, báo lỗi rõ khi mạng hoặc ổ cloud không phản hồi.
- Nguồn trên cloud hiện số % font chưa tải về và gợi ý chọn **Always keep on this device** trong Finder.

**Giao diện**
- Màu nhấn (accent) theo màu nhấn của macOS (System Settings → Appearance). Nền chuyển sang xám trung tính lạnh, cả sáng lẫn tối.
- Chấm trạng thái đứng đầu tên font: xanh là đang bật, xám là chưa bật; bấm chấm để bật/tắt. Vòng rỗng là font đã có sẵn trên máy.
- Badge ngôn ngữ dùng màu trung tính (màu xanh chỉ dành cho trạng thái bật).
- Advanced search gộp vào ô tìm kiếm thành icon phễu, có số bộ lọc đang dùng.
- Danh sách ngôn ngữ: tiếng Việt trước, rồi các ngôn ngữ Latin phổ biến, Cyrillic/Greek, còn lại theo ABC.
- **Splash** khi mở app: chữ FontAtlas đảo qua các font trên máy rồi dừng ở "Font" có chân + "Atlas" không chân cùng x-height. Bấm phím bất kỳ để bỏ qua; tắt được trong Cài đặt → Chung.
- **Onboarding** mới theo phong cách phòng lab: hình giải phẫu chữ có animation (cap height, x-height, baseline, dấu chồng tiếng Việt), bố cục mới, màn kết chuyển vào thư viện. Tôn trọng chế độ giảm chuyển động của macOS.


**Bật font (activation)**
- Hàng Active trong ô xem nhanh có nhãn rõ, chọn theo phiên làm việc hoặc giữ lâu dài, đổi phạm vi không cần tắt font.
- Trạng thái **Có sẵn trên máy**: font đã được hệ thống cài sẵn (cùng nội dung) không bị bật trùng.
- Bật/tắt cả họ font một lần; bật/tắt nhiều font đang chọn bằng ⌘⇧A.
- Hộp xung đột gọn hơn: gom theo họ, giải thích ai đang giữ font bằng lời thường; cùng một file thì không hỏi.

### Sửa lỗi
- **Classification**: font serif chân nhỏ, chân nhọn hoặc có bracket không còn bị nhận là Sans; tên font (Neue, Grotesk…) không còn lấn số đo thật. Thư viện tự phân tích lại (bỏ qua font chưa tải về từ cloud, phân tích sau khi tải).
- Hộp thoại mở từ Top Bar không còn bị lưới font che.
- Đếm "Đang bật" ở sidebar cập nhật ngay khi bật/tắt.
- Tiếng Việt không bao giờ được ghi "đủ" khi chưa kiểm được.

## [0.2.0] — 2026-11-09

Bản phát hành đầu tiên. App được dựng lại toàn bộ trên engine phân tích font viết bằng Rust và chạy hoàn toàn trên máy: không tải font hay số đo của bạn lên đâu cả.
Bản này dành cho macOS Apple Silicon.

### Thêm mới

**Thư viện font**
- Thêm nhiều thư mục nguồn, quét lại theo thay đổi, nhận ra file bị đổi chỗ. Font còn nằm trên iCloud (chưa tải về) được đếm riêng.
- Lưới font cuộn mượt với thư viện lớn. Mỗi ô hiện tên font bằng chính font đó; có chế độ danh sách.
- Tìm theo tên (gõ không dấu cũng được, ví dụ "d" tìm ra "đ"); phím ⌘K để tìm nhanh.
- Bộ lọc: Vietnamese, license, nguồn, yêu thích, class. Mỗi bộ lọc đang bật có chip để gỡ.
- Yêu thích, Gần đây, Đang bật, Dự án, Thẻ. Chọn nhiều font cùng lúc, menu chuột phải.
- **Trùng lặp**: gom các file giống hệt nhau, các phiên bản và các định dạng của cùng một font. Mỗi bản có nút mở đúng vị trí trong Finder. App chỉ báo, không bao giờ tự xoá.
- Bật/tắt font cho hệ thống (activation): theo phiên làm việc hoặc giữ lâu dài, báo xung đột khi trùng tên, tự dọn khi thoát app.

**Chi tiết font**
- Specimen dạng waterfall, tab Vietnamese với lưới 134 ký tự và cảnh báo dấu bị tràn.
- Tab Anatomy: số đo đo thật trên outline (x-height, cap height, contrast, stress, aperture, slant, serif, width…), có biểu tượng cho từng trục.
- Đường guide Hxg (cap height, x-height, baseline) khớp đúng nét chữ thật. Nếu số đo của bộ phân tích lệch với hình hiển thị, app ghi chú cho biết.
- **Classification**: class, subclass và độ tin cậy. Có thể tự sửa class, và class đã sửa được dùng cho cả bộ lọc. Display là thẻ công dụng (usage), không phải class.
- **Similar**: tìm font giống theo family, kèm giải thích "giống ở / khác ở". Có thể bật thêm phân tích hình ảnh (model chạy trên máy, tải một lần, tạm dừng hoặc tiếp tục được).
- **Pairing**: gợi ý ghép cặp font theo vai trò, lưu các cặp đã chọn.

**Compare** (so sánh 2–8 font)
- Bảng so sánh specimen, số đo, class và metadata. Font đầu là mốc, mỗi số đo có Δ so với mốc.
- Cột property đứng yên khi cuộn ngang. Pin font để luôn đứng đầu bảng.
- Nút **+** thêm font từ thư viện, nút × bỏ font. Mục **Đang so sánh · N** trên sidebar để quay lại phiên đang so sánh.
- Nhóm property thu gọn hoặc mở bằng −/+; ẩn hàng không cần và thêm lại khi muốn.
- Sửa chữ mẫu ngay trong ô specimen; sửa một cột thì mọi cột đổi theo.
- Ký tự font không có hiện màu xám, kèm dấu "!" đỏ. App đọc thẳng bảng ký tự (cmap) của file font nên không báo nhầm.
- Hàng **Vietnamese** đánh dấu các ký tự có dấu chồng (stacked diacritics, ví dụ ấ, ở, ặ) và báo khi dấu va chạm hay tràn.
- **Overlay** chồng nét các font lên nhau để so, bật/tắt cho từng font (tối đa 4).
- Thanh đo tô từ 0 đến giá trị, cùng thang cho các font đang so. Percentile so với thư viện tham chiếu nằm trong tooltip.
- Chế độ Same size hoặc Match x-height.
- Xuất **PNG** đúng như bảng đang hiện: thứ tự pin, hàng đã ẩn, overlay, đơn vị. Có cả bản sáng và tối.

**Giao diện**
- Onboarding 7 bước có minh hoạ, giải thích license, model và activation trước khi bắt đầu.
- Settings dạng tấm phủ trên thư viện: chung, nguồn, phân tích hình ảnh, activation, license, dữ liệu, phím tắt.
- Mọi thuật ngữ typo đều có tooltip giải thích, kèm hình minh hoạ.
- Cảnh báo và chi tiết phụ hiện bằng dấu "!" đỏ ở góc; rê chuột hoặc dùng bàn phím để xem nội dung.
- Dark mode: vùng chữ mẫu nền đen chữ trắng. Giao diện phối các tông đen, xám đậm và xám nhạt.
- Màn hình trống có hướng dẫn cho từng khu vực. Nguồn cần cấp quyền truy cập thư mục được đánh dấu riêng.

**Cài đặt**
- DMG kèm script **Cài Font Atlas.command**: double-click là cài và chạy được, chưa cần notarize.

### Ghi chú cài đặt
- Bản này chưa được Apple notarize (chỉ ký ad-hoc). Cách cài:
  1. Mở DMG, double-click **Cài Font Atlas.command**. Script chép app vào Applications, gỡ quarantine, ký ad-hoc lại rồi mở app; thư viện và cài đặt cũ được giữ nguyên.
  2. Nếu macOS chặn script ở lần đầu, vào System Settings → Privacy & Security, bấm **Open Anyway**, rồi double-click lại. Trên macOS 14 trở về trước, có thể chuột phải vào script rồi chọn **Open**.
  3. Cài bằng Terminal (thay cho script): kéo app vào Applications, rồi chạy `xattr -dr com.apple.quarantine "/Applications/Font Atlas.app"`.
- Font Atlas là công cụ: app không chặn việc xem hay phân tích font vì lý do license, nhưng hiển thị loại license để bạn tự chịu trách nhiệm khi dùng.

## 0.1.0

Bản prototype nội bộ, không phát hành.
