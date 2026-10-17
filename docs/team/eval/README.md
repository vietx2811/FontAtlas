# Bộ đánh giá Similar & Pairing — BẢN NHÁP

Soạn bởi: Embedding & Evaluation (ML) · Ngày: 2026-10-16 · Trạng thái: **BẢN NHÁP chờ chủ app duyệt** (spec MVP mục 1.5 S4–S5, 6, 11, 13)

Ký hiệu: **[ĐO]** = chạy thật · **[XM]** = đã mở nguồn và đọc · **[SUY]** = suy luận/đề xuất.

## Các file

| File | Nội dung | Ai điền |
|---|---|---|
| `similar_sets.csv` | 27 nhóm Similar kiểm chứng được (S01–S27) | Đội ML soạn; chủ app điền `owner_verdict` (ok / sửa / bỏ) |
| `pairing_judgements.csv` | 84 cặp: P01–P73 (53 tốt, 15 va chạm, 5 tệ) từ nguồn có thẩm quyền + V01–V11 liên quan tiếng Việt; cột `split` = tune / test / test-vi | Đội ML soạn; chủ app điền `owner_verdict`, `owner_mode`, `owner_note` |
| `gate_predictions_draft.csv` | Scorer dự đoán nhãn trục và cổng cho từng cặp, cấu hình A–D | Chỉ để tham khảo |
| `rules_plan3.md` | Bảng quy tắc R1–R9 + H1, ngưỡng và cổng, để Plan 3 cài thẳng | Lead / Plan 3 |

Script tái lập (spike, không phải code sản phẩm): `spikes/eval/gate_predict.py`, `rule_d.py`, `split.py`, `report.py`.

## 1. similar_sets.csv

**Cách chọn.** Chỉ lấy quan hệ có câu khẳng định trong nguồn:
- mô tả chính thức trong repo `google/fonts` (`DESCRIPTION.en_us.html`, cũng là phần About trên trang specimen);
- Wikipedia: Arial, Caslon, Didone, Futura.

Các loại nhóm (`group_type`):

| Loại | Số nhóm | Ví dụ |
|---|---|---|
| Cùng họ, khác độ đậm | 4 | Inter, Helvetica Neue, Source Serif 4, Baskerville |
| Cùng superfamily (bề rộng / cỡ quang học / biến thể) | 6 | Roboto ↔ Roboto Condensed; Red Hat Text ↔ Display; Nunito Sans ↔ Nunito |
| Dẫn xuất | 5 | Public Sans ← Libre Franklin; Carlito ← Lato; Caladea ← Cambo |
| Bản sao metric | 3 | Arial ↔ Helvetica/Arimo; Times New Roman ↔ Tinos; Georgia ↔ Gelasio |
| Revival cùng mẫu gốc | 6 | Garamond, Caslon, Bodoni/Didot, Baskerville, Goudy, Courier |
| Cùng thể loại và thời kỳ | 3 | Futura ↔ League Spartan/Jost; Avenir ↔ Futura; grotesque sớm |

- **Nhóm bẫy S21:** phần Latin của GFS Didot lấy cảm hứng từ Palatino, không phải Didot. Nhóm này kiểm tra hệ thống không bị tên font đánh lừa. [XM]
- **Quyết định có chủ ý ở S16:** không đưa Cousine vào nhóm Courier. Cousine chỉ cùng metric với Courier New, hình chữ khác.
- `verification`: 24 nhóm `verified`, 3 nhóm `rule-derived` (S19, S25, S26 — các nhóm này nới `k` lên 20). Riêng S20: nguồn không nêu đích danh Bodoni 72; Bodoni 72 được xếp vào nhóm Didone theo phân loại.
- `availability`: gf = Google Fonts OFL/Apache (được embedding theo spec 5.1); mac = font hệ thống macOS (chỉ dùng trục giải phẫu). Nhóm trộn gf+mac kiểm được cả hai chế độ.

**Chỉ số (spec S4):** R@k của `must_include` (mọi phần tử phải nằm trong top-k) và tỉ lệ vi phạm `must_not` trong top-k.

**Lưu ý:** style khớp theo tên hoặc `usWeightClass`. Font Google Fonts nên dùng file variable gốc; spike này dùng bản tĩnh theo từng độ đậm tải qua CSS API.

## 2. pairing_judgements.csv

**Nguồn — mọi trang đều đã được mở và đọc ngày 2026-10-16. Không cào dữ liệu, chỉ đọc bài.**
- Mô tả chính thức trong `google/fonts`: các superfamily; Playfair Display ghi rõ "có thể đi cùng Georgia hoặc Gelasio cho nội dung".
- Google Fonts Knowledge: bài *font matrix* và bài *cùng nhà thiết kế/foundry*.
- Typewolf (ô "Suggested Font Pairing" trên từng trang font).
- Canva Design School *The ultimate guide to font pairing*.
- Smashing Magazine *Best Practices of Combining Typefaces* (2010).
- Layers Magazine *Choosing type combinations* (quy tắc "concord hoặc contrast, đừng conflict").
- MyFonts (quy tắc không ghép hai font script).

**Kiểm tra chéo.** Tôi tự mở lại 3 nguồn chính (Smashing, Layers, GF font matrix): mọi cặp và nhận định trích dẫn đều khớp. [XM]

**Phân tầng tin cậy (`source_confidence`):**
- 3 = nguồn nêu đích danh cặp;
- 2 = suy từ quy tắc nêu trong nguồn. Ví dụ font matrix: cùng hàng nhưng khác "form model" thì gây khó chịu, cùng cột thì hợp nhau;
- 1 = các nguồn mâu thuẫn nhau:
  - Source Sans + Open Sans: Canva khen, nhưng trái quy tắc "cùng phân loại";
  - Lobster Two + Kaushan Script: GF khen, nhưng MyFonts khuyên không ghép hai script.

**Giới hạn cần biết:**
- **Vai trò:** phần lớn nguồn không nói font nào làm tiêu đề → `role_note` ghi "role inferred". Typewolf/Canva giữ nguyên thứ tự liệt kê. Có 3 cặp được giữ cả hai chiều (PT, Libre Franklin/Baskerville, Alegreya) để kiểm tính bất đối xứng.
- **Độ đậm:** nguồn hầu như không ghi độ đậm → mặc định Regular cho cả hai. Đây là nguồn sai số lớn (xem §3).
- **Thay tên:** "Bodoni" (Canva) → Bodoni Moda; "Garamond" (Smashing) → EB Garamond; "Source Sans/Serif Pro" → Source Sans 3/Source Serif 4. Có ghi trong `role_note`.
- **Thiếu dữ liệu:**
  - 9 cặp dùng font thương mại (Souvenir, Clarendon, Brioso, Myriad/Minion…) → bộ đánh giá báo `NO_DATA`. Giữ lại vì là ví dụ tệ/va chạm duy nhất được nêu đích danh. **Cả 5 cặp `bad` đều thuộc nhóm này**, nên hiện chưa đo được "≥ 80% cặp tệ có nhãn va chạm".
  - **Cặp tiếng Việt:** đã thêm V01–V11, xem §4 (vẫn cần chủ app bổ sung).
- `mode_expected` là suy luận của đội ML:
  - superfamily → An toàn;
  - khác họ → Có kiểm soát;
  - tiêu đề script → Thử nghiệm.

## 3. Đối chiếu với ngưỡng ban đầu (`research/07` §3.3–3.5)

**Cách đo [ĐO].**
- Số đo: build lại spike `fontatlas-spike` (core-engine, Rust) từ bản sao trong scratchpad, không đụng worktree của nhóm khác.
- Dữ liệu: 455 face macOS + 364 bản tĩnh Google Fonts (92 họ), tổng 819 face.
- 64/73 cặp có đủ hai font; 9 cặp thương mại báo `NO_DATA`.
- Các giả định phải tự chọn (spec còn để ngỏ):
  - τ trong bảng 3.3 áp trực tiếp lên δ;
  - nhãn cứng, không dùng sigmoid;
  - D toàn cục dùng z = δ/τ_c;
  - D_fam = phân vị 95 của D giữa các độ đậm kề nhau trong cùng họ, đo trên kho này (A: 0,526 với Δw ≤ 200; B: 0,476 với Δw ≤ 100).

**Kết quả trên cả 64 cặp P đo được (51 tốt, 13 va chạm) — chỉ để tham khảo, số chính thức xem §3.1:**

| Cấu hình | Cặp tốt qua An toàn/Có kiểm soát (S5 ≥ 80%) | — cùng superfamily | — khác họ | Cặp va chạm có ≥ 1 nhãn va chạm | Cặp va chạm lọt An toàn/Có kiểm soát |
|---|---|---|---|---|---|
| **A — đúng bảng 3.3** | **5/51 (10%)** | 2/2* | 3/49 | 13/13 | 0 |
| B — A + 4 sửa lỗi | 22/51 (43%) | 19/19 | 3/32 | 12/13 | 0 |
| C — B + bỏ độ đậm khỏi va chạm + x-height τ_h 0,08 | 31/51 (61%) | 19/19 | 12/32 | 11/13 | 0 |
| **D — C + 4 quy tắc mới** | **36/51 (71%)** | 19/19 | 17/32 | **13/13** | **0** |

\*Ở A, superfamily chỉ được nhận ra khi cùng designer và cùng chữ đầu tên họ.

**⚠️ Cấu hình B–D được chỉnh trên chính 64 cặp này, không có tập kiểm riêng → số 71% là lạc quan.** Cần tách train/test theo `face_a` khi chủ app đã điền nhãn (research/07 §3.7).

### 3.1 Số trên tập kiểm (số chính thức) [ĐO]

**Phép chia cố định.** `spikes/eval/split.py`, **seed 20261016**:
- phân tầng theo `expected`;
- hai dòng cùng cặp font (kể cả đảo chiều) luôn ở cùng một phía;
- khoảng 1/3 thành `test`, còn lại là `tune`;
- kết quả: tune 35 tốt / 10 va chạm / 3 tệ; test 18 / 5 / 2.

V01–V11 được thêm **sau khi** đã chốt quy tắc A–D → `test-vi` là tập kiểm sạch duy nhất.

| Cấu hình | Tập | Cặp tốt qua An toàn/Có kiểm soát | Va chạm/tệ có ≥ 1 nhãn va chạm | Va chạm/tệ lọt An toàn/Có kiểm soát |
|---|---|---|---|---|
| A (bảng 3.3) | test | 1/17 (6%) | 4/4 | 0 |
| A | test-vi | 1/7 (14%) | 1/1 va chạm; 2/2 tệ bị H1 chặn | 0 |
| D (R1–R9) | test | **12/17 (71%)** | **4/4** | **0** |
| D | test-vi | **7/7 (100%)** | 1/1 va chạm (Be Vietnam Pro + Inter: va chạm toàn cục + aperture); 2/2 tệ bị H1 chặn | **0** khi bật H1 |

(Bảng đầy đủ theo tune/test/test-vi cho A–D: chạy `report.py`. Cặp `bad` của test-vi không có nhãn K nhưng bị chặn bởi H1 — đúng thiết kế.)

**⚠️ Đọc số này thế nào:**
- **`test` (P) không sạch.** Tôi thiết kế R1–R9 khi đã nhìn thấy cả 64 cặp, trước khi chia tập. 71% vì vậy vẫn là cận trên. Từ nay mọi chỉnh ngưỡng chỉ được nhìn `tune`.
- **`test-vi` sạch nhưng rất nhỏ** (7 tốt, 1 va chạm, 2 tệ). Trong đó 3/7 cặp tốt qua nhờ heuristic superfamily theo tiền tố tên (R2).
- **S5 "≥ 80% cặp tệ có nhãn va chạm" vẫn chưa đo được:** cả 5 cặp `bad` của P dùng font thương mại (NO_DATA), còn 2 cặp tệ của test-vi là lỗi tiếng Việt chứ không phải lỗi thẩm mỹ.
- Cần chủ app điền `owner_verdict` rồi chạy lại.

### 3.2 Cặp nào làm ngưỡng sai, và vì sao

1. **Vùng va chạm của từng trục quá rộng, nên gần như mọi cặp khác họ đều dính ≥ 1 trục K.** Ở A, nguyên nhân khiến cặp tốt rơi xuống Thử nghiệm:

   | Nguyên nhân | Số cặp |
   |---|---|
   | contrast_o | 24 |
   | va chạm toàn cục | 24 |
   | width | 21 |
   | độ đậm | 16 |
   | aperture | 15 |
   | x-height | 13 |

   Ví dụ: Libre Baskerville + Libre Franklin (Typewolf) có 4 trục K.

2. **Va chạm toàn cục bắn nhầm cặp serif + sans** (Bitter + Montserrat D = 0,26; Source Serif 4 + Source Sans 3), vì D bỏ qua trục serif. → Sửa B: thêm serif vào D.
3. **Superfamily không nhận ra** khi designer khác nhau trong metadata (Source Serif 4 của Grießhammer, Source Sans 3 của Hunt). → B: nhận superfamily theo tiền tố tên họ (trừ "Libre", "Noto"). Đây chỉ là heuristic; nên có bảng superfamily tay.
4. **Contrast đổi theo serif là điều tất yếu** (serif vs sans cùng superfamily: δ contrast ≈ 0,3–0,6, rơi đúng vùng K). → B: khi trục serif đã là tương phản thì contrast không tính va chạm riêng (nhãn `C~`).
5. **Độ đậm:** nguồn không ghi độ đậm nên so Regular với Regular. Chênh lệch "màu" 10–40% giữa hai họ khác nhau là bình thường, và người dùng sẽ tự chọn instance tiêu đề. → C: độ đậm không gây va chạm (`w?`); thay vào đó gợi ý instance tiêu đề để đạt tương phản.
6. **x-height giữa tiêu đề và nội dung:** Playfair + Georgia (δ = 0,068), Bodoni Moda + Montserrat (0,132) đều là cặp được khen nhưng rơi vào vùng K. Hai font ở hai cỡ khác nhau thì x-height không cần khớp. → D1: chỉ kiểm x-height khi cùng cỡ (nội dung/nội dung, inline/code).
7. **Va chạm toàn cục xung đột với "concord" có chủ đích:** Playfair Display + Gelasio là cặp mà chính mô tả GF khuyên, nhưng D < D_fam. → D2: va chạm toàn cục chỉ khi không có trục biểu cảm nào tương phản rõ.
8. **Lọt lưới: "cùng da khác khung"** — đây là lý do font matrix bác các cặp cùng hàng.
   - Source Sans 3 + Helvetica và Zilla Slab + Bitter có mọi trục biểu cảm hoà hợp nhưng aperture/x-height tương phản → bảng 3.3 coi là ổn.
   - → D3: nếu mọi trục biểu cảm hoà hợp mà có trục cấu trúc tương phản thì gắn va chạm. Quy tắc này bắt thêm 4/13 cặp va chạm, chỉ báo nhầm 1 cặp (Bitter + Fira Code, vai trò code).
9. **Sàn x-height cho nội dung (p30 ≈ 0,48 hoặc 0,43) loại oldstyle kinh điển** (EB Garamond x-height ≈ 0,40, Alegreya). → D4: chỉ cảnh báo, không chặn cổng.
10. **Còn hỏng ở D (15 cặp tốt vẫn ở Thử nghiệm):**
    - Font display hẹp (Bebas Neue, Oswald, Anton, League Gothic, Cinzel): contrast_o δ 0,2–0,7 là nhiễu của dáng hẹp. Gợi ý: trục contrast cần độ tin cậy thấp khi width cực đoan, hoặc tiêu đề display có ngưỡng riêng.
    - Script (Sacramento, Yellowtail): đúng là nên ở Thử nghiệm theo `mode_expected`.
    - 2 cặp mơ hồ (`source_confidence` = 1).
    - Playfair + Georgia: contrast_o δ = 0,58 và width 0,10 vẫn là K.
11. **Trục đo chưa tin cậy ảnh hưởng thật:** `serif=unsure` với Alegreya, Alegreya Sans, DM Serif (serif_foot rơi vùng 1,2–1,6); stress chỉ có giá trị ở rất ít cặp. Khớp với rủi ro đã ghi ở spec 12.

### 3.3 Đề xuất
Xem **`rules_plan3.md`**: bảng quy tắc R1–R9 + H1, kèm ngưỡng, cổng, lý do và mã nguyên lý trong research/09.

## 4. Cặp tiếng Việt (V01–V11)

- **Nguồn Việt có thẩm quyền** (Donny Trương, *Vietnamese Typography*, trang Type Recommendations) [XM]: chỉ nêu cặp giữa font thương mại (Skolar + Skolar Sans, Tenon + Mortise, Job Clarendon Display + Text). Không có cặp OFL nào.
- **Vì vậy V01–V11 lấy từ:**
  - mô tả chính thức Google Fonts của các họ OFL có subset `vietnamese` (Wix Madefor Display/Text, Bellota/Bellota Text, Noto Serif/Sans);
  - quy tắc "cùng nhà thiết kế" (GF Knowledge) áp vào font của nhà thiết kế Việt trên GF: Viaoda Libre, Darker Grotesque và Xanh Mono đi với Be Vietnam Pro (chung ViệtAnh Nguyễn hoặc Lâm Bảo); Gowun Batang/Dodum;
  - quy tắc font matrix: Be Vietnam Pro + Inter → va chạm;
  - quy tắc H1 (Trương + research/09): Libre Baskerville và Josefin Slab thiếu tiếng Việt → tệ cho dự án Việt, kể cả khi là superfamily.
- **Còn mơ hồ:**
  - V06 Darker Grotesque + Be Vietnam Pro: cùng nhà thiết kế, nhưng cùng loại grotesque (`source_confidence` = 1);
  - V11 Phudu + Be Vietnam Pro: không có nguồn, `expected = ?` → **cần chủ app**.
- Đã kiểm độ phủ tiếng Việt bằng core engine: mọi font của V01–V08 và V11 đều `full` / thiếu 0 ký tự; Libre Baskerville thiếu 68 ký tự, Josefin Slab thiếu 102.
- Mới có 11 cặp, chưa tới 15: không có thêm nguồn cho font OFL. Chủ app nên bổ sung cặp từ kinh nghiệm dự án (F&B, branding) vào tập `tune`.

## 5. Tái lập
```bash
# số đo (spike core-engine, build từ bản sao): fontatlas-spike jsonl <fonts> > metrics.jsonl
CONFIG=A python3 spikes/eval/gate_predict.py docs/team/eval/pairing_judgements.csv pred_A.csv mac_metrics.jsonl gf_metrics.jsonl
CONFIG=C python3 spikes/eval/gate_predict.py docs/team/eval/pairing_judgements.csv pred_C.csv mac_metrics.jsonl gf_metrics.jsonl
python3 spikes/eval/split.py docs/team/eval/pairing_judgements.csv   # chỉ chạy lại khi thêm dòng; seed cố định
python3 spikes/eval/report.py pred_A.csv pred_B.csv pred_C.csv        # bảng theo tune/test/test-vi, gồm D
```
Font Google Fonts: bản tĩnh theo từng độ đậm tải qua `fonts.googleapis.com/css2` (TTF), lưu ở scratchpad, không commit.
