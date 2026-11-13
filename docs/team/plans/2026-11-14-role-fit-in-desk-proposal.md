# Đề xuất: đưa "Hợp vai" (role fit) lên Desk (04, 2026-11-14)

Chỉ là đề xuất, chưa có code. Dựa trên spec G5/G6 (`docs/team/plans/2026-11-12-desk-suggest-spec.md`, Lead đã duyệt) và
`docs/team/eval/pairing/README.md`. Điều kiện bật: bộ kiểm mù xác nhận chất lượng (mục 2).

## 1. Hiện trạng (đã kiểm trên `int/0.4.1` @`ba0c61da` và `1d85d98e`)

| Lớp | Trạng thái |
|---|---|
| Mô hình `fontatlas_core::pairing` (`role_fit`, `pair_style`, mức good/ok/weak/unknown + mã lý do, không điểm số) | **Có**, trong 0.4.0 |
| API Tauri: `role_fit`, `role_fit_faces`, `suggest_combos` (08, bọc mô hình) | **Có** trong `commands.rs`/bindings |
| UI: nguồn **Hợp vai** trong Picker, popover Gợi ý combo | **Chưa có** trong `UX/src` (không có `roleFit`/`suggestCombos`; nhánh Picker của 05 chưa vào `int/0.4.1`) |
| Bằng chứng chất lượng | Chỉ **số trên tập đã chỉnh** (16/16 cặp, 11/11 vai) và bộ mù chưa chấm (chờ JSON chủ app) |

Nghĩa là phần còn lại chủ yếu là UI + chuỗi + bằng chứng, không phải thuật toán.

## 2. Cổng chất lượng: định trước ngưỡng, trước khi thấy kết quả

Bộ mù: 20 gợi ý của mô hình + 10 đối chứng, một người chấm (chủ app) theo Hợp / Tạm / Không. Chỉ 30 mục và một người chấm: đây là
kiểm khói, không phải thống kê. Đề xuất ngưỡng (Lead và chủ app chốt, chốt **trước** khi chạy `pairing_blind --score`):

- Gợi ý của mô hình được chấm Hợp hoặc Tạm: **≥ 70 %** (≥ 14/20), và Không: **≤ 15 %** (≤ 3/20).
- Đối chứng (cặp/vai do mô hình xếp thấp hoặc ngẫu nhiên) chấm Không: **≥ 60 %** (≥ 6/10), để chứng minh chủ app phân biệt được.
- Đạt cả hai → bật. Đạt một phần → bật có nhãn thử nghiệm (mục 4) và giữ riêng nhóm chấm Không để xem lại ngưỡng.
- **Không chỉnh ngưỡng mô hình theo kết quả này**; nếu cần chỉnh thì dựng bộ mù mới (key ngoài repo, theo quyết định của Lead).

## 3. Cần làm gì để hiện trên Desk

| # | Việc | Ai | Ghi chú |
|---|---|---|---|
| 1 | Nguồn **Hợp vai** trong Picker (mặc định khi đã chọn vai, theo spec §2.2), dòng ứng viên + câu "vì sao" | 05 | Dùng `role_fit_faces`; thứ tự mảng = xếp hạng, không hiện điểm |
| 2 | Chuỗi `why.*` vi + en cho **mọi mã lý do** + TermTip (có hình minh hoạ nếu được) | 05, 04 cấp danh sách | Danh sách chốt ở mục 5; mã kèm tham số (vd `vi_missing` + số ký tự) |
| 3 | Trạng thái `unknown`/`insufficientData` hiển thị trung thực ("chưa đo"), không đoán | 05 | Mô hình trả `unknown` khi phủ tiêu chí < 50 % trọng số hoặc class `uncertain` không có `candidate` |
| 4 | Đo thời gian `role_fit_faces` ở 5.000 và 20.000 face (spec nêu mục tiêu < 200 ms ở 5k, chưa thấy số đo) | 08 (04 hỗ trợ bench) | Tôi đã có bench Index; role_fit quét O(N) nên khả thi, nhưng cần số thật |
| 5 | Gợi ý combo (`suggest_combos`) chỉ bật sau khi Hợp vai ổn, vì dùng cùng nền | 05, 08 | Có thể tách làm hai lượt phát hành |
| 6 | Bước QA native: Picker nguồn Hợp vai có dữ liệu, rỗng, `unknown`; thư viện < 12 family | 07 | Theo bảng trạng thái spec §6 |
| 7 | Cập nhật `docs/team/eval/pairing/README.md` bằng kết quả mù, bỏ cụm "nội bộ" khỏi CHANGELOG khi bật | 04, 01 | |

## 4. Cách trình bày để không hứa quá

- Nhãn theo từng tiêu chí (good/ok/weak/unknown ↔ ●◐◆○ đã dùng ở Desk), **không điểm tổng**, **không %**.
- Lần phát hành đầu gắn nhãn **"Thử nghiệm" / "Beta"** ở nguồn Hợp vai (TermTip: "ngưỡng hiệu chuẩn trên font mã nguồn mở, có thể
  lệch với thư viện của bạn"). Gỡ nhãn khi có thêm bằng chứng (mục 6).
- Gợi ý là công cụ, không chặn: nút Đặt luôn bấm được kể cả `weak` hoặc thiếu tiếng Việt (đúng tinh thần "công cụ, không gác cổng").
- Font thiếu tiếng Việt: mô hình đã hạ mức (Body/Caption/Label → weak, Heading/Quote/Display → tối đa ok); UI hiện NoteMark đỏ "!" kèm số ký tự thiếu.

## 5. Mã lý do UI phải dịch (04 chốt danh sách cuối khi 05 bắt đầu)

Từ `fontatlas-core/src/pairing.rs` (tiêu chí theo vai): `x_height_low/high`, `aperture_low/high`, `contrast_low/high`,
`distinctiveness_low` (giải thích như "điểm yếu nhất", **không** là "vì sao bị hạ"; xem `eval/pairing/README.md`), `vi_missing` (+`count`),
`class_unfit`, `tnum_missing`, `caps_only`; phong cách pair: `xheight_mismatch` (+`delta`), `too_close_not_same` (+`cosine`). Tham số số
chỉ để hiển thị, không để tính lại.

## 6. Rủi ro và cách giảm

| Rủi ro | Giảm bằng |
|---|---|
| Ngưỡng hiệu chuẩn trên Google Fonts OFL (1722 họ), thư viện người dùng (font thương mại, hệ thống macOS) phân bố khác | Chạy báo cáo **gộp** trên thư viện chủ app (chỉ tỉ lệ good/ok/weak/unknown theo vai, không file, không tên, không commit); nếu `unknown` quá cao thì xem lại phủ tiêu chí trước khi bật |
| Class `uncertain` ở font biểu cảm → gợi ý lệch | Đã dùng `candidate`; font không có candidate → `unknown`, không đoán |
| Một người chấm, 30 mục, ngưỡng "hypothesis" | Gắn nhãn thử nghiệm; ghi rõ trong CHANGELOG; thêm nút "Hợp/Không hợp" chỉ lưu cục bộ (`user.db`, không mạng) để chủ app tự xem lại sau, **tuỳ chọn**, không bắt buộc cho lần đầu |
| Bộ mù hiện đã công khai (key.json trong repo) | Chỉ chủ app làm bài, Lead đã yêu cầu chủ app không mở key; bộ mù sau này để key ngoài repo, chỉ commit hash |
| Hiệu năng ở thư viện lớn | Mục 3 #4; nếu > 200 ms: tính trước `Traits` theo từng face khi dựng Index (đã có), cache theo vai |

## 7. Câu hỏi mở cho Lead

1. Ngưỡng ở mục 2 (70 % / 15 % / 60 %) có chấp nhận được không, hay Lead và chủ app muốn con số khác? Cần chốt trước khi có JSON.
2. Bật lần đầu với nhãn "Thử nghiệm" có đủ, hay chỉ bật sau khi có đợt chấm thứ hai (bộ mù mới)?
3. Có cho phép chạy báo cáo gộp trên thư viện thật của chủ app (không commit gì) để xem phân bố `unknown` không?
