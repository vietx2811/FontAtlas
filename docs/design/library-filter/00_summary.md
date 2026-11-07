# 00 · Tóm tắt — Library filter + Search + chỉ báo ngôn ngữ

**Vòng 2 (2026-11-06).** Tài liệu: `02_spec.md` (§0 liệt kê thay đổi vòng 2; §9 quy tắc nhận diện phân loại; §10 thuật ngữ & i18n) · `01_research.md` · prototype `../prototype/library-filter-v2.html` (v1 giữ nguyên) · ảnh `shots-v2/01…18` (1400×920, 1100×920, sáng/tối, VI/EN). Thanh demo góc phải: Trống · Basic · Advanced · Nhiều lọc · Danh sách lọc · Gõ "ser" · 0 kết quả · UI: EN. URL: `?theme=dark`, `?loc=en`, `?demo=adv`.

## Đã áp dụng quyết định chủ app
1. **Phân loại tự nhận diện**: Sans · Serif · Slab · Script · Decorative + cờ Mono, từ số đo (serif_foot, contrast, width_cv…), PANOSE, OS/2 sFamilyClass, `isFixedPitch`, tên font; nhãn tin cậy high/medium/low; "low" ẩn trừ khi bật "Kể cả nhận diện chưa chắc". Backend cần 4 số đo mới + đọc 2 trường metadata (spec §7.2, §9).
2. **Weight/Width đầy đủ trong Advanced**: usWeightClass 100–900 và usWidthClass 1–9 (slider có bước), nút `wght`/`wdth` cho font Variable (khớp khi dải trục giao khoảng chọn); Darkness/Stem đặt cạnh làm độ đậm đo được.
3. **Advanced dày hơn**: một lưới masonry cột 208px chung cho Basic + 18 trục → 5 cột @1400, 3 cột @1100, không cuộn ở 1400×920; hàng trục 40px, histogram nằm sau track.
4. **Tóm tắt thay dãy chip**: `Sans +1 · 5 ngôn ngữ · Weight 400–700 +3 ×` ≤ 60% ô tìm, ô nhập luôn ≥ 40%; bấm → "Đang lọc" (gỡ từng giá trị / cả nhóm / sửa); ⌫ lần 1 chọn nhóm cuối, lần 2 bỏ nhóm; toast + ⌘Z hoàn tác.
5. **Thuật ngữ font tiếng Anh, UI qua khoá i18n**; tên ngôn ngữ theo locale (CLDR hệ thống qua `Intl.DisplayNames`, fallback autonym Hyperglot). "Gần đủ" = ≥ 90% `base`; chỉ dùng Hyperglot; bỏ `Ặ vi`; Advanced phủ lưới.

## Câu hỏi còn mở
1. **"≥ 90%" với tiếng Việt**: 90% của 186 = được thiếu tới 18 ký tự, có thể là đúng 4 chữ Ơ ơ Ư ư + vài dấu chồng → font không gõ được tiếng Việt vẫn "gần đủ". Có muốn **bắt buộc đủ bộ lõi** (chữ cái riêng + `marks`) rồi mới xét 90% không? *Đề xuất: có, áp dụng cho mọi ngôn ngữ.*
2. **Câu mẫu theo ngôn ngữ** (preview khi lọc ngôn ngữ, trước định lấy từ gflanguages): bỏ hẳn, hay tự sinh từ `base` của Hyperglot (vd "Ăâđêôơư")?
3. **Locale đầu tiên ngoài tiếng Việt** (để chốt độ dài chuỗi): English?
4. Có cần lớp con cho Serif (Oldstyle / Transitional / Didone, suy từ `stress_deg` + `contrast`) ở P3 không?

## Giới hạn
- Prototype dùng font hệ thống macOS và 24 font giả; nhãn phân loại/tin cậy là dữ liệu mẫu, chưa chạy quy tắc §9 trên font thật.
- Tên ngôn ngữ phụ thuộc ICU của WebKit; ngôn ngữ hiếm có thể chỉ có autonym/tên Anh.
- Minh hoạ SVG trong tooltip là phác thảo vị trí.
