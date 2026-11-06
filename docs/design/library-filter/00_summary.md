# 00 · Tóm tắt — Library filter + Search + chỉ báo ngôn ngữ

**Tài liệu**: `01_research.md` (Adobe Fonts, RightFont, Typeface, FontBase; dữ liệu ngôn ngữ OSS) · `02_spec.md` (spec + backend + pha) · prototype `../prototype/library-filter-v1.html` (mở trực tiếp; thanh demo góc phải: Trống / Basic / Nâng cao / Gõ "ser" / 0 kết quả; `?theme=dark`) · ảnh `shots/01…14` (1400×920, 1100×920, sáng/tối).

## Đề xuất chính
1. **Số `−68` = 68 ký tự tiếng Việt font thiếu** (trên tổng 186). Thay bằng nhãn **`VI` + vòng độ phủ**: vòng đầy (đủ) · cung cam (thiếu một phần) · vòng rỗng + gạch (gần như không). **Không còn số trên thẻ**; số + ký tự thiếu nằm trong tooltip; bấm → tab Tiếng Việt. Nhãn đổi theo ngôn ngữ đang lọc.
2. **Search là cửa vào duy nhất**: chip điều kiện nằm trong ô tìm, gỡ bằng `×`/Backspace; token `lang:vi`, `class:serif`, `weight:600-900`, `x:0.5-0.55`, `var`, `italic`, `fav` (+ alias không dấu tiếng Việt); autocomplete có đếm kết quả. Nút `Ặ vi` và `Lọc` gộp vào nút bộ lọc cuối ô tìm.
3. **Basic** (popover 560px, kiểu RightFont/Adobe): Phân loại (ô "Ag"), Ngôn ngữ (pill + tìm, Đủ dấu/Gần đủ), 4 thuộc tính 3 nấc minh hoạ (đậm, rộng, x-height, tương phản), Kiểu, Thư viện, Giấy phép — mỗi lựa chọn có số đếm.
4. **Advanced** (sheet ngang): 5 nhóm có icon, 16 trục, **một slider 2 đầu**/trục, bấm số để nhập, tooltip minh hoạ SVG; histogram mật độ đã chừa chỗ (bật "Mật độ (xem trước)").
5. **Dữ liệu ngôn ngữ: Hyperglot (Apache-2.0)**, dự phòng **gflanguages (Apache-2.0)**. 783 orthography (704 verified/preliminary), ~70 KB gzip nhúng. Bảng tiếng Việt 186 ký tự trùng khít giữa hai nguồn và khớp số liệu app. Đã đo bằng `scripts/lang_extract.py` (ngoài app).

## Cần chủ app quyết định
1. **Phân loại**: backend hiện chỉ suy ra Sans/Serif/Mono từ số đo. Slab/Script/Trang trí cần (a) heuristic mới, (b) phân loại tay bằng chuột phải như RightFont, hay (c) cả hai? *Đề xuất: (c), P2 bắt đầu bằng phân loại tay + heuristic Slab.*
2. **Ngôn ngữ ưu tiên mặc định trên thẻ** = Tiếng Việt, đổi được trong Cài đặt? *Đề xuất: có.*
3. **"Gần đủ"** = thiếu ≤ 5 ký tự (base) — chấp nhận ngưỡng này?
4. **Bỏ nút `Ặ vi` ở TopBar** (thay bằng chip/lựa chọn nhanh "Tiếng Việt")? Người quen thao tác 1 click sẽ cần 2 click (mở gợi ý → chọn) hoặc gõ `lang:vi`.
5. **Advanced phủ lên lưới** (lưới vẫn cập nhật, đếm ở footer) hay **đẩy lưới xuống**? *Đề xuất: phủ, để giữ vị trí cuộn.*
6. Dữ liệu ngôn ngữ chỉ nhận **verified + preliminary** của Hyperglot (bỏ 79 draft)?

## Giới hạn
- Trang help Adobe trả 403 khi tra cứu; chi tiết Properties (Case/Figures, danh sách đủ 8 phân loại) ghi rõ là gián tiếp/quan sát.
- Prototype dùng font hệ thống macOS và dữ liệu giả (24 font); minh hoạ SVG trong tooltip là phác thảo vị trí, cần vẽ chuẩn khi triển khai.
- Số liệu cmap đo trên font mẫu subset trong `fonts/samples` — chỉ minh hoạ.
