# Round 2 — Độ chính xác analyzer (core-0.4.0)

Nguồn bằng chứng: `docs/team/eval/analyzer-audit-2026-11-06.md` (audit độc lập Sonnet). Backend Codex `62df14a1` viết code + đo; Codex hết quota trước khi commit, Lead (Claude) kiểm và commit.

## Thay đổi
| # | Vị trí | Thay đổi |
|---|---|---|
| 1 | `geom.rs` | `post.italicAngle`≈0 nhưng face **khai báo** italic/oblique (OS/2 fsSelection hoặc head.macStyle) và slant đo ≥3° → deskew theo slant đo. Script upright không bị dựng (giữ heuristic a/g). |
| 2 | `stroke.rs` stress | Ngưỡng LowContrast 1,3 → **1,7** (sans regular max 1,35; transitional min 1,89). |
| 3 | `stroke.rs` | \|tilt\|>45° → Failed(Degenerate). Contrast của o hẹp (bbox ≤0,65 hoặc usWidthClass 1–4): giữ tỉ lệ gốc, hạ **Estimated** — không bịa công thức chuẩn hoá. |
| 4 | `stroke.rs` serif_foot | Face nghiêng/khai báo italic → Estimated. |
| 5 | minor | −0,0 → +0,0; aperture 0 tia mở → Degenerate; cmap chọn subtable Unicode thật, bỏ Windows Symbol (Webdings không còn "có tiếng Việt"). |
| — | `lib.rs` | `ANALYZER_VERSION` core-0.4.0 → cơ chế boot/rescan hiện có phân tích lại thư viện cũ. |
| — | `fontatlas-index/data/reference-v1.json` | Regenerate bằng pipeline eval, google/fonts@47d54ca, 3813 face. |

## Trước/sau trên tập audit (975 face, Latin 639)
| Chỉ số | Trước | Sau |
|---|---|---|
| Sans upright có stress "Estimated" | 55/209 | 16/209 |
| Stress \|tilt\|>45° (±90° giả) | 11 | 0 |
| Vietnamese khớp fontTools | 970/975 | 975/975 |
| Mẫu 30 face kiểm độc lập | — | 0 lệch |
| x-height / cap / mono / slant raw | — | 0 face đổi |

Quality sau: contrast Estimated 35, serif_foot Estimated 151, aperture Failed 8.

## Reference percentile (p1 / p50 / p99) — UI percentile dịch theo
| Trục | n trước→sau | Trước | Sau |
|---|---|---|---|
| stress_deg | 2258→1195 | −90 / 0 / 90 | −37 / 2 / 43 |
| aperture | 3719→3636 | 0 / 83 / 135 | 20 / 84 / 135 |
| contrast | 3723 | 1,041 / 1,385 / 20,354 | 1,039 / 1,385 / 20,354 |
Các trục khác không đổi.

## Kiểm tra
- `cargo test --workspace`: 699 pass, 0 fail, 23 ignored (Lead chạy, 2026-11-06).
- `cargo clippy --workspace --all-targets -D warnings`: sạch.
- Test hồi quy `fontatlas-core/tests/accuracy.rs` trên 7 font OFL (license trong `fixtures/accuracy-licenses/`); test font hệ thống macOS là `#[ignore]`, chỉ chạy local.
- Script so sánh `fontatlas-core/examples/accuracy_compare.py` (đọc output collect của audit).

## Giới hạn
- Stress vẫn luôn "Ước lượng"; ngưỡng 1,7 là cổng độ tin, không phải bộ phân loại.
- Lead chạy lại pipeline audit gốc (bản sao, nguồn chỉ đọc) với dump tại dev/mvp 37bf14c sau gate B: a/g storeys **0 face đổi / 983**; danh sách nhãn tay 36 face giữ nguyên 33/36 như trước (3 lệch có từ trước: Cormorant SC, Monaco, Poppins — không do thay đổi này). Stress |tilt|>45°: 13 → 0.
- Contrast font condensed vẫn là tỉ lệ thô (Estimated), chưa có công thức chuẩn hoá kiểm chứng.

## Kiểm định trục nhấn trên font hệ thống — Lead, 2026-11-07
Font upright, `cargo run -p fontatlas-core --example dump`, core-0.4.1:

| Nhóm | Font → stress (°) |
|---|---|
| Didone / modern | Didot 0, Didot Bold 0, Bodoni 72 Book/Bold 0, Big Caslon 0 |
| Transitional | Baskerville 0 (cả 3 độ đậm), Charter 0–1, Georgia 1 |
| Old-style / humanist | Palatino 12, Times New Roman 12, Iowan Old Style 13, Athelas 10, Palatino Bold 5 |
| Ngoại lệ | Hoefler Text 0 (kỳ vọng nghiêng nhẹ) |

- Thử thêm một cách đo độc lập: lấy góc điểm mảnh nhất của `o`, làm trơn 5 mẫu. Kết quả cùng chiều với khớp điều hoà: Palatino và Iowan 12°, Times 10°, Didot, Baskerville, Georgia và Hoefler 0–4°. Chỉ thử tạm, đã gỡ, không đổi code.
- Kết luận:
  - Thứ tự đúng: Didone 0° < old-style.
  - Độ lớn thấp hơn mô tả trong sách (15–30°), vì tia đo đi qua tâm bbox chứ không vuông góc với nét.
  - Chưa có tập ground truth số độ để hiệu chỉnh, nên **giữ nhãn "Ước lượng"**. Đây là nhãn trung thực, không phải lỗi.
- Contrast của font condensed vẫn là tỉ lệ thô và được hạ xuống Estimated (Bebas 1,94; Oswald 1,91). Lý do giống trên: chưa có công thức chuẩn hoá đã kiểm chứng.
- Việc tiếp: lấy ground truth stress từ tag Google Fonts (/Serif/Old Style và /Serif/Didone) trong lúc làm D1 Phân loại (Font Desk plan), rồi hiệu chỉnh.

## Kiểm định định lượng bằng nhãn Google Fonts — Lead, 2026-11-07
**Tập dữ liệu:**
- Nhãn: `tags/all/families.csv` của google/fonts@47d54ca, chỉ lấy nhãn có điểm ≥ 50 và không mâu thuẫn.
- Font: bản Regular hoặc variable upright trong `~/fontatlas-ref/google-fonts`.
- Chương trình đo: dump `core-0.4.1`, bản release.
- Script: `fontatlas-core/examples/stress_eval.py`.

**Trục nhấn (stress), 279 family serif:**

| Nhóm (tag) | n đo được | Không đo được | p10 | Trung vị | p90 |
|---|---|---|---|---|---|
| Nhấn đứng (Didone, Modern, Scotch, Fat Face) | 59 | 7 | 0° | **0°** | 3° |
| Transitional | 127 | 7 | −1° | 0° | 10° |
| Nhấn nghiêng (Old Style Garalde, Humanist Venetian) | 74 | 5 | 0° | **11°** | 24° |

- Đứng hay nghiêng, ngưỡng |stress| ≥ 5°: **đúng 83%**. Nhóm đứng đúng 55/59, nhóm nghiêng 55/74.
- Các ca sai chủ yếu là font có nhãn mơ hồ hoặc đặc thù:
  - Cinzel: chỉ có chữ hoa.
  - Libre Caslon, Adamina, Alike: o gần đối xứng.
  - GFS Didot đo ra 12°, đúng là thiết kế Hy Lạp có trục nghiêng.
- "Không đo được" chủ yếu do contrast < 1,7, là cổng độ tin đã định.
- **Kết luận:**
  - Analyzer đo stress thật và tách được hai nhóm thiết kế.
  - Độ lớn (old-style trung vị 11°) thấp hơn sách (15–30°), vì phương pháp đo bị bias có hệ thống.
  - UI tiếp tục hiện số độ kèm nhãn "Ước lượng".

**Contrast của font hẹp: so với bản thường cùng family**

| Family | o-contrast condensed / thường | H-contrast condensed / thường |
|---|---|---|
| Barlow | 1,49 / 1,20 | 1,13 / 1,15 |
| Saira | 1,51 / 1,35 | 1,09 / 1,13 |
| PT Sans Narrow / PT Sans | 1,33 / 1,24 | 1,09 / 1,12 |
| Encode Sans | 1,37 / 1,28 | 1,08 / 1,10 |
| Fira Sans | 1,36 / 1,35 | 1,22 / 1,22 |
| IBM Plex Sans | 1,21 / 1,21 | 1,14 / 1,14 |
| Roboto | 1,24 / 1,23 | 1,17 / 1,23 |

- Contrast đo trên o bị thổi phồng tới +24% khi o hẹp. Contrast đo trên H (`contrast_h`) ổn định trong khoảng ±0,04.
- Bebas Neue (1,94) và Oswald (1,91) thực chất là sans contrast thấp, H-contrast lần lượt là 1,10 và 1,40.
- Quyết định giữ nguyên:
  - Face hẹp vẫn được gắn **Estimated** như đã làm từ core-0.4.0.
  - Tab Giải phẫu nay có thêm hàng **"Tương phản nét (H)"**, đơn vị tỷ lệ (`contrast_h`, không có percentile). Số này ổn định trên font hẹp và nằm cạnh o-contrast "Ước lượng".

## Kiểm chéo trục nhấn bằng counter của o
- Đo hướng trục chính (moment bậc 2) của lòng chữ o, thử tạm rồi gỡ:
  - Palatino: 7°.
  - Times New Roman: 7°.
  - Didot, Hoefler Text và Georgia: 0°.
- Ba phương pháp độc lập cùng chiều và cùng cỡ:
  - khớp điều hoà;
  - điểm mảnh nhất;
  - trục của counter.
- Kết luận: giá trị phản ánh đúng hình chữ thật. Mốc "15–30°" trong sách áp cho Venetian cổ, không áp cho các bản Garalde số hoá. Không cần hệ số hiệu chỉnh.
- Nhãn "Ước lượng" chỉ nói rằng đây là trục suy ra từ hình học, không phải số đo trực tiếp.
  - Đổi công thức o-contrast sẽ phải regenerate reference-v1. Việc này để vòng Phân loại (D1), khi có tập kiểm lớn hơn.

## core-0.4.2: kiểm bằng ground truth tổng hợp, sửa contrast font hẹp — Lead, 2026-11-07

**Ground truth tổng hợp.** Test `axes::stroke::synthetic_stress` dựng chữ o có giá trị đúng biết trước: viền ngoài là ellipse, counter là ellipse quay một góc định sẵn.
- **Stress:** đặt 0°, 10°, 20°, 30°, −15° → đo ra đúng đến 0,5°, ở cả hai độ dày nét.
  - Phép đo không có bias. Trung vị 11° của nhóm Old Style là giá trị thật của các font đó.
  - Mốc 15–30° trong sách mô tả kiểu Venetian cổ, không phải sai số của analyzer.
- **Contrast:** đặt 5,00 và 3,33 → đo ra đúng (sai số < 2%).
- **o hẹp, contrast thật 2,00:**

| Tỷ lệ rộng/cao của o | Cách đo cũ (tia qua tâm) | core-0.4.2 |
|---|---|---|
| 1,00 | 2,00 | 2,00 |
| 0,73 | 2,00 | 2,00 |
| 0,53 | 2,26 | 2,00 |
| 0,40 | 2,92 | 2,00 |
| 0,30 | 4,15 | 2,00 |

**Sửa contrast font hẹp** (`contrast_narrow_aware`):
- Khi o có tỷ lệ rộng/cao < 0,7: kéo giãn trục x cho o tròn lại, đo, rồi chia cho hệ số kéo giãn.
- Font thường không đổi.
- Nhãn chất lượng:
  - Measured cho tỷ lệ ≥ 0,3.
  - Estimated chỉ khi tỷ lệ < 0,3 (ngoài vùng đã kiểm).
- Cổng stress dùng contrast đã sửa, nên Bebas (thô 1,94, đã sửa 1,27) không còn bị coi là font có trục nhấn.

**Đối chiếu với bản thường cùng family (Google Fonts):**
- Sai số trung bình: 7,2% → **4,1%**.
- Barlow Condensed: 24,5% → 4,4%.
- Saira Condensed: 12,1% → 7,0%.
- Fira, Plex, Roboto không đổi.
- Bebas Neue: 1,94 → 1,27. Oswald: 1,91 → 1,46.

**Sửa thêm một lỗi có sẵn:** Avenir Next Condensed Heavy/Ultra Light Italic có italicAngle bằng 0 và không bật cờ italic, nên trước đây không được dựng thẳng.
- Nay tên (subfamily hoặc PostScript) chứa Italic/Oblique cũng được tính là khai báo nghiêng.
- Test `macos_oblique_condensed_and_symbol_regressions` (ignored, chạy local) đã đỏ từ trước, nay pass.

**Tham chiếu:**
- `ANALYZER_VERSION` = core-0.4.2; thư viện cũ sẽ được phân tích lại.
- reference-v1 dựng lại bằng refbuild (3.813 face).
- Contrast p1/p50/p99: 1,039 / 1,385 / 20,35 → 1,006 / 1,348 / 17,65.
- Stress không đổi (−37 / 2 / 43).

**Kiểm tra:**
- cargo test --workspace: 707 pass, 0 fail.
- clippy -D warnings sạch.
- Google Fonts stress eval không đổi (83%).
- QA native có thêm bước `filter-full-library`. Ảnh chụp cho thấy slider x-height 0,343–0,686 em (p1–p99) trên min/max 0,332–0,700, tức là khoảng slider theo dữ liệu thư viện, không còn 0–1.
- QA native 26/26 hai lượt; một lượt khác treo ở bước `dark-snapshots` (sau 24 bước OK), dừng PID của chính QA và chạy lại thì qua — ghi nhận flake, chưa rõ nguyên nhân.

## core-0.4.3: contrast đo vuông góc với nét, độ nghiêng theo I/T — Lead, 2026-11-07 (theo yêu cầu chủ app)

**Định nghĩa contrast**
- Typography: tỉ lệ độ dày nét dày nhất / nét mảnh nhất, đo vuông góc với hướng nét. PANOSE "Contrast" cũng đo trên chữ O.
- Cách cũ:
  - Đo dọc tia đi qua tâm bbox, nên chỉ xấp xỉ.
  - Khi tia cắt xiên vào nét hai bên của o hẹp, độ dày bị thổi phồng.
- Cách mới, `contrast_perpendicular`:
  - Lấy mẫu 720 điểm đều trên viền ngoài của o.
  - Độ dày tại mỗi điểm là khoảng cách ngắn nhất tới viền trong, tức vuông góc với nét trên đường cong trơn.
  - contrast = (trung bình 3% mẫu dày nhất) / (trung bình 3% mẫu mảnh nhất).
  - Không phụ thuộc độ rộng o hay hướng trục nhấn.
- Lùi về cách kéo giãn (0.4.2) khi o không phải một vòng hai contour, ví dụ stencil hoặc contour VF chồng nhau. Trên 278 serif có 5 face phải lùi.
- Method đổi tên thành `o_stroke_width_max_min` (chữ hoa: `O_stroke_width_max_min`).

**Sai số so với bản thường cùng family** (15 cặp condensed / semi / extra-condensed với bản thường):

| Cách đo | Trung bình | Trung vị | Lớn nhất |
|---|---|---|---|
| Tia qua tâm (≤ 0.4.1) | 8,1% | 6,6% | 24,5% |
| Kéo giãn (0.4.2) | 5,2% | 5,6% | 9,5% |
| **Vuông góc (0.4.3)** | **2,6%** | **1,8%** | **6,8%** |

- Chữ o tổng hợp: sai < 3% ở mọi độ rộng và mọi góc nhấn.
- Bebas Neue 1,10, khớp H stem/bar (1,10). Oswald 1,35 (H 1,40).
- Serif không đổi bản chất: trung vị tỉ lệ vuông góc / tia = 1,00; Spearman 0,978.

**Độ nghiêng**
- Thứ tự ưu tiên:
  1. Thân **I** và **T** hoa: nếu lệch nhau < 1,5° thì lấy trung bình, không thì lấy I, rồi T.
  2. Dấu **"|"**.
  3. `l` thường, vì l có đuôi/móc làm lệch kết quả.
- Đối chiếu italicAngle, chỉ lấy sans/serif/mono và bỏ italic khai báo 0°:
  - 297 italic: MAE **0,87° → 0,48°**, p90 2,07° → 0,98°.
  - 72 upright: MAE 0,37° → 0,31°.
- Dấu "|" (gợi ý của chủ app):
  - 42/126 italic giữ "|" đứng, nên "|" không được ghi đè I/T.
  - Khi thiếu I/T, "|" đúng hơn l: Sirivennela có l −19,9° nhưng "|" 0°.
- Các ca lệch lớn còn lại phần lớn do metadata khai sai, cả ba glyph đều đồng ý với nhau: Tinos ghi −16,3° nhưng vẽ −10°; Arvo Bold Italic ghi 0° nhưng vẽ −13°.
- Tham chiếu slant: p1/p99 −28,7°/6,0° → −24,0°/2,1°, bớt nghiêng giả trên font đứng. n tăng 3.671 → 3.719.
- Method đổi tên thành `I_T_stem_midline`.

**Kiểm tra:**
- cargo test --workspace: 707 pass, 0 fail; clippy sạch.
- Test macOS (ignored, chạy local) pass.
- Vitest 559; QA native 26/26.
- reference-v1 dựng lại (core-0.4.3, 3.813 face).
