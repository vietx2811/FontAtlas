# Bảng quy tắc pairing cho Plan 3 (bản nháp, 2026-10-16)

Bảng này chỉnh lại `research/07` §3.3–3.5 và spec §6.3, dựa trên số đo ở `README.md` §3. Mã nguồn nguyên lý lấy từ `research/09`. Mọi số là **[SUY từ ĐO]**: chỉnh trên tập `tune` + `test` của `pairing_judgements.csv` (xem cảnh báo ở README §3.1).

Quy ước:
- δ = chênh lệch thô (ln tỉ lệ / độ / phân vị);
- nhãn H = hoà hợp, C = tương phản, K = va chạm;
- trục **biểu cảm** = độ đậm, contrast, serif, độ nghiêng;
- trục **cấu trúc** = x-height, độ rộng, aperture, stress.

## 1. Ngưỡng theo trục (thay bảng 3.3)

| Trục | δ | τ_h | τ_c | Thay đổi so với 3.3 | Áp dụng khi |
|---|---|---|---|---|---|
| x-height/em | ln tỉ lệ | **0,08** | 0,15 | τ_h 0,04 → 0,08 | **chỉ khi hai font cùng cỡ**: nội dung/nội dung, inline, code (R6) |
| Độ đậm `stem_n_xh` | ln tỉ lệ | — | — | **không gắn K** (R5) | dùng để gợi ý instance tiêu đề |
| Contrast `contrast_o` | ln tỉ lệ | 0,20 | 0,70 | giữ | nếu trục serif = C thì K → `C~` (R3) |
| Serif (`serif_foot`) | lớp sans < 1,2 < chưa chắc < 1,6 < serif | cùng lớp | khác lớp | giữ | — |
| Độ rộng `width_o_xh` | ln tỉ lệ | **0,10** | 0,25 | τ_h 0,06 → 0,10 | — |
| Aperture `c` | phân vị | **0,15** | 0,35 | τ_h 0,10 → 0,15 | — |
| Stress | độ | 8 | 25 | giữ | khi cả hai contrast ≥ 1,3 |
| Độ nghiêng | độ | 2 | 8 | giữ | — |

## 2. Quy tắc

| # | Tên | Quy tắc (cài thẳng) | Lý do (số đo) | Nguyên lý `research/09` |
|---|---|---|---|---|
| R1 | Serif vào khoảng cách toàn cục | Khi tính D, thêm thành phần serif: z = 0 nếu cùng lớp, z = 2 nếu khác lớp, bỏ qua nếu "chưa chắc". D = √(mean z²) với z = δ/τ_c. D_fam = p95 của D giữa hai độ đậm **kề nhau (Δw ≤ 100)** trong cùng họ, đo trên kho OFL (spike: 0,476). | Không có R1 thì Bitter + Montserrat có D = 0,26, Source Serif 4 + Source Sans 3 bị coi là "gần như nhau" → 24/49 cặp tốt khác họ dính va chạm toàn cục | D3 concord/contrast/conflict; D4 serif + sans |
| R2 | Nhận superfamily | `superfamily(a,b)` khi trùng designer (nameID 9), **hoặc** trùng từ đầu của tên họ (luôn loại: libre, google, red, zen, big, old; chỉ tính khi cùng manufacturer: noto, source, ibm), **hoặc** cùng nhóm trong bảng superfamily tay. **Bảng tay chỉ dựng từ tập `tune`** (cập nhật 2026-10-17 sau review: bản trước có family của tập kiểm → rò rỉ). Superfamily → luôn qua cổng An toàn. | Metadata designer không đủ: Source Serif 4 (Grießhammer) và Source Sans 3 (Hunt) khác designer. Tiền tố tên bắt được 19/19 superfamily trong tập | C2 superfamily; C3 cùng designer/foundry |
| R3 | Contrast đi kèm serif | Nếu serif = C thì contrast_o ở vùng K đổi thành `C~`: không tính va chạm, không tính là tương phản chủ đích | Serif vs sans cùng superfamily có δ contrast 0,3–0,6, rơi đúng vùng K | A5 contrast nét; D4 |
| R4 | Nới width và aperture | τ_h width 0,10, aperture 0,15 (bảng §1) | Ở bảng gốc, width gây K cho 21 cặp tốt, aperture cho 15 | A4 tỉ lệ; A8 độ mở |
| R5 | Độ đậm là gợi ý, không phải va chạm | Không gắn K cho `stem_n_xh`. Thay vào đó gợi ý instance tiêu đề để ln(stem_h/stem_b) ≥ 0,40. Giữ cờ "đảo vai" khi tiêu đề mảnh hơn | Nguồn không ghi độ đậm, nên phải so Regular với Regular; chênh lệch 10–40% giữa hai họ là bình thường (16 cặp tốt dính K) | D8 độ đậm không quá gần; G2 đủ style |
| R6 | x-height chỉ xét khi cùng cỡ | Với cặp tiêu đề/nội dung: x-height không gây K. Với nội dung/nội dung, inline, code: dùng bảng §1 | Playfair + Georgia (δ 0,068), Bodoni Moda + Montserrat (0,132) là cặp được khen nhưng rơi vùng K | A2 "khi hai font đứng gần nhau"; A3 |
| R7 | Va chạm toàn cục cần thiếu tương phản | `va_chạm_toàn_cục = khác superfamily ∧ D < D_fam·1,1 ∧ có ≥ 1 trục biểu cảm xác định ∧ không có trục biểu cảm C rõ` (`C~` không tính) | Playfair Display + Gelasio (chính mô tả GF khuyên ghép) có D < D_fam nhưng contrast tương phản rõ | D3; D5 giữ một, đổi một |
| R8 | **Cùng da khác khung** (va chạm loại 3) | Khác superfamily ∧ có ≥ 1 trục biểu cảm xác định ∧ mọi trục biểu cảm ∈ {H, chưa chắc, w?} ∧ ≥ 1 trục cấu trúc = C ⇒ gắn va chạm "khung khác nhưng bề mặt giống" | Bắt Source Sans 3 + Helvetica, Zilla Slab + Bitter, Helvetica + Outfit, Montserrat + Poppins (quy tắc font matrix của GF); chỉ báo nhầm 1 cặp (Bitter + Fira Code, vai trò code) | A1 cùng mô hình dựng chữ; nguồn GF font matrix |
| R9 | Sàn x-height cho nội dung chỉ cảnh báo | Ràng buộc vai trò nội dung bỏ điều kiện x-height ≥ p30; chuyển thành cảnh báo khi < 0,43 | p30 ≈ 0,48 loại oldstyle kinh điển (EB Garamond ≈ 0,40, Alegreya) | G4 môi trường; A3 ngoại lệ |
| H1 | Phủ tiếng Việt (giữ, đã có trong spec) | Dự án tiếng Việt: chặn cặp có font thiếu ký tự | Tập test-vi: Libre Baskerville (thiếu 68 ký tự), Josefin Slab (thiếu 102) bị chặn đúng, kể cả khi là superfamily | H1 |

**Ghi chú về số lượng.** Báo cáo trước gọi là "8 điều chỉnh"; khi viết tách ra thành R1–R9:
- R4 gồm hai ngưỡng;
- R9 thay sàn 0,43 tạm thời của bước B;
- H1 không phải điều chỉnh mới, chỉ được kiểm lại.

## 3. Cổng chế độ (thay bảng 3.5)

| Chế độ | Điều kiện |
|---|---|
| **An toàn** | superfamily (R2) **hoặc** (không có K nào, kể cả va chạm toàn cục R7 và R8 ∧ mọi trục cấu trúc ∈ {H, chưa chắc} ∧ ràng buộc vai trò đạt) |
| **Có kiểm soát** | ≤ 1 K và K đó thuộc {width, aperture, stress} ∧ có ≥ 1 trục biểu cảm C rõ (không tính `C~`) ∧ không có va chạm toàn cục / R8 ∧ ràng buộc vai trò đạt |
| **Thử nghiệm** | còn lại; vẫn hiện mọi nhãn K |
| (chặn) | H1 khi dự án dùng tiếng Việt |

## 4. Việc Plan 3 cần làm thêm (không chặn)

- **Bảng superfamily tay** (R2): khoảng 50 dòng đầu, bắt đầu từ các cặp trong `similar_sets.csv` và `pairing_judgements.csv`.
- **Font display hẹp** (Bebas Neue, Oswald, Anton, League Gothic): contrast_o bị nhiễu khi width cực đoan. Đề xuất hạ độ tin cậy của contrast khi `width_o_xh` < p10. Chưa đo.
- **Serif "chưa chắc"** (Alegreya, Alegreya Sans, DM Serif): cần đo serif tốt hơn (chân serif theo hình, không chỉ tỉ lệ bề rộng).
- **Trọng số trục:** chưa dùng. Bảng trên chỉ là nhãn và cổng.
