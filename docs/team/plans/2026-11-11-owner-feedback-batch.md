# Góp ý chủ app 2026-11-11 — thư viện, topbar, phân loại, ổ cloud

Nguồn: chủ app nhắn Lead ngày 2026-11-11. Lead ghi lại và chia việc. Mỗi mục làm theo
"nghiên cứu hành vi người dùng trước": nêu app cùng loại làm thế nào (Finder, Lightroom,
RightFont, Typeface, Connect, FontBase) trước khi chốt giao diện.

## A. Thẻ font: chấm trạng thái và dải thông tin (Frontend, 05)

1. **Chấm bật/tắt đứng đầu tên font.** Xanh là đang bật, xám là chưa bật. Chấm chính là
   nút bật/tắt (bấm được, có tooltip và aria). Bỏ nút "on" riêng có icon.
2. **Badge ngôn ngữ không dùng chấm xanh.** Hiện vòng phủ tiếng Việt có màu xanh trùng
   tín hiệu "đang bật". Đổi sang màu trung tính hoặc dạng chữ (VI, RU, EL…), để chỉ chấm
   bật/tắt mang màu xanh.
3. **Panel "Hiển thị" (View options) chọn thông tin hiện trên thẻ.** Bật/tắt từng loại:
   tag ngôn ngữ (nhiều ngôn ngữ, không chỉ VI), phân loại, số kiểu, biến thiên, giấy phép,
   nguồn/ổ… Lưu theo người dùng. Tham khảo cách Finder/Lightroom làm "View Options".

## B. Topbar (Frontend, 05)

4. **Bỏ nút Advanced search riêng.** Gộp vào ô tìm kiếm thành icon phễu ở mép phải ô.
   Có bộ lọc thì phễu có chấm đếm số bộ lọc; bấm phễu mở AdvancedSheet như hiện nay.
5. **View options: nhóm và sắp xếp.**
   - Nhóm theo Family (bật/tắt).
   - Sắp xếp theo: tên, ngày thêm vào thư viện, ngày phát hành (`head.created` / version /
     năm trong copyright), ngày tạo file… Chiều tăng/giảm.
   - Backend (02) cung cấp các trường ngày còn thiếu, có ghi rõ nguồn và độ tin cậy.

## C. Sidebar (Frontend, 05)

6. **Menu ngữ cảnh (chuột phải) cho mục Sidebar.** Nguồn: hiện trong Finder, quét lại,
   gỡ nguồn. Bộ sưu tập: đổi tên, xoá, bật/tắt cả bộ. Smart/Case: tương tự. Dùng chung
   Menu của dự án; có phím tắt và focus trả về đúng chỗ.

## D. Phân loại sai: serif chân nhỏ bị nhận là sans (Rust/analyzer, 02)

Bằng chứng (đọc thư viện thật của chủ app, chỉ đọc; 15 họ BeauType, 217 face):
`serif_foot` (trung bình các face thẳng):

| Họ | serif_foot | contrast |
|---|---|---|
| Absinotte | 1.96 | 3.96 |
| Amibus | 1.60 | 4.69 |
| Steinhart | 1.48 | 1.25 |
| Glyphius | 1.43 | 1.24 |
| Annam Neue | 0.56 | 5.51 |
| Noirist | 0.26 | 2.95 |
| Beau Sans, Elf50, Grotesk, Nouvoty, Oateria, Rohbeton, Solisiert, Widesonik, Wifting | **1.00 đúng** | 1.06–5.76 |

- Chín họ có `serif_foot` = 1.00 tròn: nghi là giá trị mặc định khi đo hỏng chứ không
  phải số đo thật (Nouvoty contrast 5.76, Rohbeton 3.85 khó là sans thuần). Kiểm phép đo
  `baseline_foot_over_stem_median_iln` trên các font này.
- Ngưỡng `serif_foot > 1.6 → serif`, `< 1.2 → sans` (classify.rs:367-371): chân nhỏ
  (1.4–1.6) rơi vào vùng giữa, trong khi tên có "Neue"/"Grotesk" đẩy phiếu sans.
- Việc cần làm: tìm lỗi đo, thêm phép đo chân nhỏ (bracket/flare, so độ rộng chân với nét
  ở đáy i/l/n), thêm bộ kiểm có các serif chân nhỏ trong font OFL (ví dụ Cormorant,
  Spectral, EB Garamond Light, Newsreader, Fraunces nhỏ…) vào eval; không đưa font
  BeauType vào repo. Báo cáo trước/sau trên eval và trên thư viện thật (chỉ con số).

## E. Font trên ổ cloud (Rust, 02 + 06 phần bật font)

Toàn bộ 2.342 vị trí file trong thư viện của chủ app nằm dưới `~/Library/CloudStorage/`
(OneDrive). Cần:
- Nhận biết nguồn là ổ cloud (CloudStorage, iCloud `Mobile Documents`, Dropbox,
  Google Drive) và file chỉ là placeholder chưa tải (dataless, `SF_DATALESS`/
  `NSURLUbiquitousItemDownloadingStatusKey`).
- Quét không được kéo cả ổ về: không đọc nội dung file dataless khi quét, hoặc chỉ đọc
  khi người dùng đồng ý; hiện trạng thái "trên cloud" bằng NoteMark.
- Bật font: file dataless phải được tải về trước khi đăng ký với CoreText (bộ điều phối
  auto-activation cũng cần điều này). Có tiến độ và báo lỗi khi mạng/ổ cloud không có.
- Gợi ý "Luôn giữ trên máy" (Always keep on this device) cho thư mục font.
- Theo dõi thay đổi của ổ cloud (FSEvents vẫn chạy với File Provider; kiểm thực tế).

## Thứ tự và người làm

Giữ tối đa 2 worker chạy song song.
- 05: xong sửa review Desk lát 1–3 và lát 4, rồi làm A + B + C (spec ngắn trước, theo
  mục A–C ở trên).
- 06: Spike 2 InDesign, sửa L1, L2–L5 auto-activation. Phần "tải file cloud trước khi
  bật" nằm trong L1/L2.
- 02: D rồi E, khi có chỗ trống (một trong hai worker xong việc).
