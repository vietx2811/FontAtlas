# Hợp vai (role fit) trên Desk: gắn với kết quả bộ mù và nhãn thử nghiệm (04, 2026-11-14)

Bản đầu (c818c5c8) nói UI chưa có: **sai**, tôi chỉ tìm trong `UX/src`. Sửa theo Lead, đã kiểm lại trên `origin/dev/mvp`:
UI Picker và gợi ý combo **đã phát hành ở 0.4.0** (`UX/components/desk/pickerSources.ts`, `ComboSuggest.tsx`, `PickerBar.tsx`,
`intents.ts`; chuỗi trong `UX/i18n/desk.ts`). Không có UI mới cần dựng; phần còn lại là buộc nó vào bằng chứng và gắn nhãn đúng.

## 1. Hiện trạng thật (đọc từ `origin/dev/mvp`)

| Lớp | Trạng thái |
|---|---|
| Mô hình `fontatlas_core::pairing` (mức good/ok/weak/unknown + mã lý do, không điểm số) | Có |
| API (`role_fit`, `role_fit_faces`, `suggest_partners`, `suggest_combos`, `desk_suggest`) | Có |
| Picker: nguồn **Hợp vai** (`desk_suggest` nguồn `fits_role`), Ghép theo vai + phong cách (`suggestPartners`), popover Gợi ý combo | **Đã có trong 0.4.0** |
| Bằng chứng chất lượng | Chỉ số trên tập đã chỉnh (16/16 cặp, 11/11 vai); bộ mù chưa chấm (chờ JSON chủ app) |
| Nhãn "đang thử" cho nguồn Hợp vai | **Chưa có.** `pick.gate.experimental` ("Thử nghiệm") có sẵn nhưng là **cổng của cặp** (safe/controlled/experimental), nghĩa khác |

Hệ quả: người dùng 0.4.0 đang thấy gợi ý Hợp vai mà chưa có bằng chứng độc lập và không có dấu hiệu nào nói đó là thử nghiệm.

## 2. Cổng chất lượng: định trước ngưỡng, trước khi thấy kết quả

Bộ mù: 20 gợi ý của mô hình + 10 đối chứng, một người chấm (chủ app) theo Hợp / Tạm / Không. Chỉ 30 mục và một người chấm: đây là
kiểm khói, không phải thống kê. **Ngưỡng dưới đây đã được Lead đồng ý (2026-11-14) và được ghi vào repo TRƯỚC khi có JSON của chủ app**
(commit `b5152ff1`; dấu thời gian git là bằng chứng), áp dụng khi chạy `pairing_blind --score`:

- Gợi ý của mô hình được chấm Hợp hoặc Tạm: **≥ 70 %** (≥ 14/20), và Không: **≤ 15 %** (≤ 3/20).
- Đối chứng (cặp/vai do mô hình xếp thấp hoặc ngẫu nhiên) chấm Không: **≥ 60 %** (≥ 6/10), để chứng minh chủ app phân biệt được.
- Đạt cả hai → gỡ nhãn thử nghiệm khi có đợt chấm thứ hai hoặc bằng chứng thêm (mục 4). Chỉ đạt một phần → giữ nhãn, giữ riêng nhóm chấm Không để xem lại.
- **Không chỉnh ngưỡng mô hình theo kết quả này**; nếu cần chỉnh thì dựng bộ mù mới, key ngoài repo, chỉ commit hash.

## 3. Việc còn lại (nhỏ, không dựng UI mới)

| # | Việc | Ai | Ghi chú |
|---|---|---|---|
| 1 | Nhãn "Thử nghiệm" trên nguồn **Hợp vai** (và trên đầu popover Gợi ý combo) + TermTip | 05 (chuỗi vi+en: 01) | Lead chốt: nhãn là đủ cho lượt đầu. Tránh nhầm với `pick.gate.experimental` (cổng cặp): dùng khoá riêng (`pick.src.role.beta`), cân nhắc chữ "Beta" để khác nghĩa |
| 2 | Chấm bộ mù theo ngưỡng mục 2 khi JSON tới; báo bảng kết quả | 04 | Không chỉnh ngưỡng |
| 3 | Bổ sung chuỗi lý do còn thiếu (mục 5) | 05, 04 cấp danh sách | Hiện bị bỏ lặng lẽ |
| 4 | Đo thời gian `desk_suggest(fits_role)` ở 5.000 và 20.000 face (spec nêu < 200 ms ở 5k, tôi chưa thấy số đo) | 08, 04 hỗ trợ bench | Quét O(N), khả thi nhưng cần số thật |
| 5 | Cập nhật `docs/team/eval/pairing/README.md` bằng kết quả mù và dòng CHANGELOG 0.5.0 | 04 | |
| 6 | Báo cáo gộp trên thư viện thật của chủ app (tỉ lệ good/ok/weak/unknown theo vai) | 04 | **Chưa làm.** Dữ liệu của chủ app; Lead sẽ hỏi chủ app trước, tôi không chạy khi chưa có đồng ý |

## 4. Cách trình bày để không hứa quá

- Nhãn theo từng tiêu chí (●◐◆○), không điểm tổng, không %. Đã đúng ở UI hiện tại.
- Gợi ý là công cụ, không chặn: nút Đặt luôn bấm được kể cả `weak` hoặc thiếu tiếng Việt.
- Nhãn "Thử nghiệm" kèm TermTip ngắn: "ngưỡng hiệu chuẩn trên font mã nguồn mở, có thể lệch với thư viện của bạn". Gỡ khi: đạt mục 2 **và** có đợt chấm thứ hai hoặc báo cáo gộp trên thư viện thật cho thấy tỉ lệ `unknown` chấp nhận được.

## 5. Mã lý do: mô hình phát ra so với chuỗi đã có

Mô hình có thể phát: `both_expressive`, `class_not_contrasting`, `class_unfit`, `display_vs_neutral`, `image_close`, `no_expressive_side`,
`not_same_family`, `same_designer`, `same_era`, `same_structure`, `same_superfamily`, `serif_vs_sans`, `tnum_missing`,
`too_close_not_same`, `vi_missing`, `xheight_mismatch`, và các cặp `*_low`/`*_high` của tiêu chí: `aperture`, `contrast`, `x_height`,
**`distinctiveness_low`**. `whyText` bỏ dòng khi không có khoá (không hiện mã thô). Đối chiếu với `UX/i18n/desk.ts`:

- Đã có chuỗi: tất cả mã trên **trừ** `distinctiveness_low` (UI có `why.distinctive`, không khớp tên mã nên bị bỏ, kể cả khi fallback cắt hậu tố `_low`).
- Cần 05 xác nhận cho vai Display: lý do "cá tính yếu" đang không hiện. Lưu ý ngữ nghĩa: `distinctiveness_low` là "điểm yếu nhất" của vai, không phải "vì sao bị hạ" (xem `eval/pairing/README.md`); câu chữ nên là trung tính ("Cá tính chữ nhẹ, hợp hơn với chữ đọc") thay vì phán xét.
- Mã đã có chuỗi nhưng tôi không thấy mô hình phát (`why.spacing`, `why.stem`, `why.width`, `why.aperture_open`, `why.contrast_ok`, `why.x_height_ok`, `why.role_unknown`, `why.distinctive`, `why.image_close`) có thể do 08 sinh ở lớp API; không phải việc của tôi, chỉ ghi để khỏi sót khi rà.

## 6. Rủi ro

| Rủi ro | Giảm bằng |
|---|---|
| Ngưỡng hiệu chuẩn trên Google Fonts OFL (1722 họ); thư viện thật có font thương mại và font hệ thống macOS phân bố khác | Nhãn thử nghiệm; báo cáo gộp (khi chủ app đồng ý, mục 3 #6) |
| Class `uncertain` ở font biểu cảm | Đã dùng `candidate`; không có candidate → `unknown`, không đoán |
| Một người chấm, 30 mục | Nhãn thử nghiệm; ghi rõ trong CHANGELOG; đợt chấm thứ hai sau |
| Bộ mù đã công khai trong repo (key.json) | Lead: giữ bộ này (chỉ chủ app làm bài, đã dặn không mở key), không viết lại lịch sử. Bộ sau để key ngoài repo, commit hash |
| Hiệu năng ở thư viện lớn | Mục 3 #4 |

## 7. Quyết định đã có (Lead, 2026-11-14)

1. Ngưỡng 70 / 15 / 60: đồng ý, đăng ký trước, không chỉnh sau.
2. Nhãn "Thử nghiệm" đủ cho lượt đầu; đợt chấm thứ hai để sau.
3. Báo cáo trên thư viện thật của chủ app: Lead hỏi chủ app trước; chưa chạy.
