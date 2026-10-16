# Plan 1 — Báo cáo thực thi `fontatlas-core`

(Phần tổng kết sẽ được viết ở Task 11.)

## Trục hoãn

### Đầu nét (terminal) chữ `c` — hoãn sang Plan 3 (Lead quyết định B, 2026-10-16)

**Định nghĩa trong plan:** `terminal_c` = bề dày đoạn mực đầu nét trên của `c`, đo trên đường dọc ở 92% bề rộng bbox, chia cho bề dày thân trái `c` (đo ở tâm dọc). Test yêu cầu Playfair Display (đầu nét ball) > Roboto.

**Kết quả thật:** Playfair 1.088, Roboto 1.194. Test fail.

**Quét vị trí 70–98% bề rộng** (đầu nét ÷ thân trái). Glyph `c` được dựng thẳng, đo ở instance mặc định:

| Font | thân trái | nét trên ở tâm | 70% | 75% | 80% | 85% | 88% | 90% | 92% | 95% | 98% |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Roboto | 185.0 | 152.3 | 0.97 | 1.12 | 1.52 | 1.57 | 1.43 | 1.32 | 1.19 | 0.95 | 0.59 |
| Open Sans | 172.0 | 149.2 | 0.85 | 0.87 | 0.89 | 0.90 | 0.91 | 0.92 | 0.92 | 0.75 | 0.30 |
| Playfair Display | 97.0 | 22.9 | 0.21 | 0.25 | 0.34 | 1.40 | 1.30 | 1.21 | 1.09 | 0.81 | — |
| Libre Baskerville | 114.1 | 46.4 | 0.55 | 0.72 | 0.91 | 0.97 | 0.94 | 0.89 | 0.83 | 0.69 | 0.42 |
| Source Code Pro | 85.0 | 69.6 | 0.82 | 0.86 | 0.91 | 0.96 | 0.99 | 0.86 | 0.65 | 0.31 | — |
| Helvetica (TTC face 0) | 192.1 | 161.4 | 0.95 | 1.12 | 1.53 | 1.58 | 1.46 | 1.35 | 1.22 | 0.96 | 0.54 |

"—" = đường dọc không cắt đầu nét.

Không vị trí nào cho Playfair > Roboto. Lý do:
- Đầu nét `c` của Roboto và Helvetica cong. Đường dọc cắt xiên qua nét nên đoạn mực dài ra.
- Thân trái của Playfair dày, vì là font tương phản cao, nên tỉ số bị kéo xuống.

Kết luận: tỉ số "đầu nét ÷ thân" không tách được khỏi độ cong.

**Định nghĩa (A), đã đề xuất nhưng không dùng:** max(đoạn mực đầu nét trên, quét 75–95%) ÷ bề dày nét trên ở tâm `c`. Số tính tay từ bảng trên:

| Font | (A) |
|---|---|
| Playfair Display | 1.40×97.0/22.9 ≈ 5.9 |
| Libre Baskerville | 0.97×114.1/46.4 ≈ 2.4 |
| Roboto | 1.57×185.0/152.3 ≈ 1.9 |
| Helvetica | 1.58×192.1/161.4 ≈ 1.9 |
| Source Code Pro | 0.99×85.0/69.6 ≈ 1.2 |
| Open Sans | 0.92×172.0/149.2 ≈ 1.06 |

Thứ tự đúng, nhưng mẫu số là nét mảnh nên (A) chủ yếu lặp lại contrast. Đưa vào pairing thì contrast bị tính hai lần. Vì vậy Lead chọn hoãn: xoá `terminal_c`, không giữ trường luôn `Failed` trong `Anatomy`.

**Hướng cho Plan 3 (04 ML, có nhãn tay):**
- Đo độ dày đầu nét theo **pháp tuyến của đường tâm nét**, không theo đường dọc.
- So với độ dày nét mảnh lân cận.
- Kiểm tra tương quan với `contrast_o` trước khi dùng (ví dụ Spearman trên thư viện tham chiếu). Chỉ đưa vào pairing nếu phần dư sau khi hồi quy theo contrast vẫn mang thông tin.
