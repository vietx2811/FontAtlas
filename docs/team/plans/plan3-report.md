# Plan 3 — Báo cáo thực thi `fontatlas-index` + `fontatlas-eval`

Nhánh `feat/fontatlas-index`, tách từ `dev/mvp` @ 7840126 (Lead chỉ định thay cho `feat/fontatlas-store`). Chưa push, chưa merge.
Máy: Apple M4 Pro, 12 luồng. Phiên bản: `scorer pair-0.1.0 · reference ref-v1 · analyzer core-0.2.0`.
Ký hiệu: **[ĐÃ CHẠY]** = số thật do chương trình in ra.

## 1. Kết quả bộ đánh giá [ĐÃ CHẠY]

Lệnh chạy (corpus 8 068 face, 0 file lỗi):

```
eval --fonts google/fonts@47d54ca --system-fonts \
     --similar docs/team/eval/similar_sets.csv --pairing docs/team/eval/pairing_judgements.csv
```

Corpus gồm google/fonts@47d54ca (ofl + apache) và font hệ thống macOS; variable font được mở rộng ra 300–900.

Các số dưới đây chép **đúng như chương trình in ra**, kể cả khi thấp hơn kỳ vọng.

### Similar (split test)

| chỉ số | giá trị |
|---|---|
| nhóm đạt (Hit) | 5/27 (19%) |
| trượt (Miss) | 22 |
| vi phạm must_not | 0 |
| NO_DATA | 0 |
| cùng họ R@1 / R@5 (6345 truy vấn) | 43.8% / 64.8% |

### Pairing

### Pairing — split `test`, nhãn Expected

| nhãn | n | pass (Safe/Controlled) | flagged (có va chạm) | leaked (lọt Safe/Controlled) | blocked (H1) |
|---|---|---|---|---|---|
| good | 17 | 12/17 (71%) | 11 | — | 0 |
| clash | 4 | — | 4/4 (100%) | 0/4 (0%) | 0 |
| bad | 0 | — | — | — | 0 |

NO_DATA: 4

### Pairing — split `test-vi`, nhãn Expected

| nhãn | n | pass (Safe/Controlled) | flagged (có va chạm) | leaked (lọt Safe/Controlled) | blocked (H1) |
|---|---|---|---|---|---|
| good | 7 | 6/7 (86%) | 3 | — | 0 |
| clash | 1 | — | 1/1 (100%) | 0/1 (0%) | 0 |
| bad | 2 | — | 0/2 (0%) | 0/2 (0%) | 2 |

NO_DATA: 0

**So với spike Python (cấu hình D)** — không chỉnh ngưỡng để khớp, chỉ ghi chênh lệch:

| split | spike Python | Rust (bản này) |
|---|---|---|
| test | good 12/17 qua cổng; clash 4/4 có nhãn | good 12/17 (71%) qua cổng; clash 4/4 có nhãn, 0/4 lọt |
| test-vi | good 7/7 qua cổng; clash 1/1; bad 0/2 có nhãn nhưng 2/2 bị H1 chặn | good 6/7 (86%); clash 1/1, 0/1 lọt; bad 0/2 có nhãn, 2/2 bị H1 chặn |

**Similar thấp:**
- 5/27 nhóm đạt; cùng họ R@1 43,8%, R@5 64,8% (6 345 truy vấn).
- Similar hiện chỉ dùng 16 trục, chưa có embedding (Plan 4).
- Nhiều nhóm `same_family_weight` (vd. S01: Inter Medium/Bold phải nằm trong top-5 của Inter Regular) trượt vì khoảng cách độ đậm trong cùng họ lớn hơn khoảng cách tới các grotesque khác cùng độ đậm.
- Đây là số trước khi có embedding. **Không chỉnh trọng số** để khớp.

## 2. 5 cặp lệch nhiều nhất so với spike (`gate_predictions_draft.csv`, cột `gate_D`) [ĐÃ CHẠY]

9/84 hàng khác cổng. Năm hàng lệch nhiều nhất:

| id | split | cặp | nhãn nguồn | spike `gate_D` | Rust | va chạm (Rust) | ghi chú |
|---|---|---|---|---|---|---|---|
| V10 | test-vi | Josefin Slab + Josefin Sans | bad | safe | Blocked | — | Rust áp H1 vào cổng; spike báo H1 riêng, `gate_D` không áp |
| V09 | test-vi | Libre Baskerville + Be Vietnam Pro | bad | controlled | Blocked | — | như trên (Libre Baskerville thiếu 68 ký tự) |
| P18 | tune | Epilogue + Anybody | good | experimental | Safe | Contrast, Width | R2 cùng designer (Tyler Finck) → `related` → An toàn dù có 2 va chạm trục |
| P19 | tune | Lobster Two + Kaushan Script | good | experimental | Safe | Contrast | R2 cùng designer (Impallari) |
| V02 | test-vi | Noto Serif + Noto Sans | good | controlled | Safe | Aperture | R2 cùng designer (Monotype Design Team) |

Bốn hàng còn lại lệch 1 bậc: V05, P21, P34, P58.
- **P58** (Bodoni Moda + Alegreya, nhãn clash, tập tune) ra Controlled, tức là một clash lọt. Hàng này thuộc tập `tune`, nên không ảnh hưởng số `test`.

## 3. Trục tiếng Việt mới (Task 1) [ĐÃ CHẠY]

Đo ở vị trí chuẩn; cả hai trục đều `Estimated`.

| fixture | stack_style | mark_weight_ratio |
|---|---|---|
| roboto | both_sides | 0.579 |
| open_sans | both_sides | 0.671 |
| playfair_display | stacked | 0.496 |
| source_code_pro | both_sides | 0.647 |
| libre_baskerville | None (Failed missing_glyph: thiếu ấ/ầ) | 0.530 |
| public_sans_vf (wght 400) | other | 0.550 |

## 4. Đầu nét (terminal) — quyết định [ĐÃ CHẠY]

- `terminal_c_normal`: độ dày đầu nét trên của `c` đo theo pháp tuyến đường tâm nét, chia cho nét mảnh ở cánh trên.
- Trên fixture: Roboto 1.62, Open Sans 1.09, Playfair 7.22, Libre Baskerville 2.29, Source Code Pro 1.21, Public Sans 1.88, Helvetica 1.65, Didot 9.08. Thứ tự Playfair > mọi sans đạt ngay ở khoảng quét của plan.
- **Tương quan với contrast** trên google/fonts@47d54ca (refbuild `--terminal-check`): **n = 3 709, Spearman r = 0.447**.
- **Quyết định: |r| < 0.7 → nhánh "đưa vào".**
  - `Anatomy::terminal` (Estimated, method `c_upper_terminal_normal_over_thin`) và `FaceProfile::terminal`.
  - **Không** vào 16 chiều Similar, **không** là `PairAxis`: trục Estimated luôn "chưa chắc" cho tới khi được kiểm định với nhãn tay.
  - `ANALYZER_VERSION` giữ `core-0.2.0`.
- Lưu ý: r = 0.447 vẫn là tương quan vừa. Trước khi dùng cho pairing nên kiểm phần dư sau khi hồi quy theo contrast (gợi ý ở `plan1-report.md`).

## 5. Thư viện tham chiếu `reference-v1` [ĐÃ CHẠY]

- Nguồn: google/fonts@47d54ca1ebb14fd864695f1d39dc734f39017c33 (ofl + apache), 3 894 file.
- **3 813 face** mã nguồn mở, 0 file lỗi, **16 bảng (trục, method)**.
  - Mọi bảng đều thuộc method nhánh chữ thường.
  - Các method nhánh chữ hoa có ít hơn 30 mẫu nên không có bảng; với chúng, phân vị trả `None` (đúng thiết kế).
- **`d_fam` = 0.459** (p95 trên 2 506 cặp độ đậm kề nhau; kho mở rộng VF 6 938 profile). Spike Python ra 0.476.
- File `reference-v1.json` nặng 15 KB, nhúng bằng `include_str!`.
- Kho được clone vào `/Volumes/VXData/Users/VietX/fontatlas-ref/google-fonts` (3,9 GB), không phải `~/fontatlas-ref`, vì ổ home chỉ còn 19 GB.

## 6. Hiệu năng S6 (release, 50 000 profile giả) [ĐÃ CHẠY]

| truy vấn | thời gian | ngưỡng |
|---|---|---|
| `similar()` top-20 | 1,2–1,3 ms | 50 ms |
| `pair_candidates()` top-20 | 40–42 ms | 50 ms |

`pair_candidates` đạt nhưng **biên mỏng**: trên runner CI chậm hơn có thể vượt 50 ms. Test S6 chỉ chạy ở `--release` và bị `ignore` ở debug, nên CI không chặn vì nó.

## 7. Kiểm thử

- Trước rebase, `cargo test` đều qua: core 59 (+1 ignored), store 66 (+2 ignored), index 48 (+2 ignored, S6), eval 12. Sau rebase xem mục 9.
- `cargo clippy --all-targets -- -D warnings` sạch trên core, store, index, eval.
- CI: `.github/workflows/index-eval.yml`. Job `test` không cần mạng; job `eval-report` chỉ báo cáo (`continue-on-error`).

## 8. Lệch plan (đã ghi ledger)

1. **Task 8:**
   - Plan gắn `#[test]` lên helper `eval_dir()` có trả giá trị, nên không biên dịch được. Đã bỏ attribute.
   - Test `must_not_violation_beats_hit` của plan dùng k = 4 với giả định có 5 fixture. Thư mục fixture nay có 8 profile, nên đổi thành k = số profile − 1, giữ đúng ý của test.
   - VF khi mở rộng được **thay** bằng các profile 300–900, không giữ bản gốc, để tránh bản trùng gần y hệt Regular làm sai R@1.
   - Style không có trong bảng tên → độ đậm thì trả NO_DATA, không đoán.
2. **Task 10:**
   - Kho clone vào ổ VXData (ổ home gần đầy).
   - `standard_location` của core chuyển thành `pub`, vì `refbuild` đo terminal ở vị trí chuẩn.
   - JSON cũ thiếu `terminal` thì `FaceProfile::terminal` = Failed, không loại cả profile.
   - CLI `eval` mặc định dùng bảng nhúng.
3. **Vặt:**
   - Thêm `#![allow(dead_code)]` / `pub` cho helper test.
   - Sửa vài lint clippy (`useless_vec`, `type_complexity`, `double_ended_iterator_last`, `redundant_closure`).
   - Test tiếng Việt của plan import `StackStyle` mà không dùng, đã bỏ import.
4. **Phát hiện, chưa sửa:** Public Sans khi không mở rộng VF có style "Thin" (tên instance mặc định), dù được đo ở wght 400. Nên lấy tên style theo vị trí đo ở Plan 6.

## 9. Rebase lên `dev/mvp` (295e930) [ĐÃ CHẠY]

Thực hiện bằng cherry-pick lần lượt 13 commit lên `dev/mvp`, thay cho `git rebase`. Lý do:
- `dev/mvp` (Plan 4) cũng thêm `pub fn standard_location` và fixture `public_sans_vf.ttf`.
- Rebase thẳng làm hàm bị định nghĩa hai lần từ commit Task 1 trở đi, tức là các commit giữa không biên dịch được.

Cách giải:
- Commit Task 1 bỏ bản `standard_location` của mình, dùng bản của `dev/mvp`. Thân hàm giống hệt.
- Commit Task 10 giữ cả hai test trong `tests/anatomy.rs`.
- `[workspace].members` gồm `core, store, embed, index, eval`. `Cargo.lock` được tạo lại bằng `cargo update -w` ở mỗi bước.
- Fixture `public_sans_vf.ttf` trùng từng byte với bản của Plan 4, nên không có xung đột.
- Nhánh dự phòng trỏ vào HEAD cũ: `backup/index-pre-rebase` (6c6e507), chỉ ở local.

Sau rebase:
- `ANALYZER_VERSION` chỉ có một giá trị: `core-0.2.0`.
- `cargo test -p fontatlas-core -p fontatlas-store -p fontatlas-embed -p fontatlas-index -p fontatlas-eval`: **257 passed, 0 failed, 10 ignored**.
  - Phần ignored gồm test hiệu năng S6 và các test của embed cần mô hình hoặc GPU.
- `cargo clippy --all-targets -- -D warnings` sạch trên cả 5 crate.
- S6 đo lại ở bản release: `similar` 1,3 ms; `pair_candidates` **45,1 ms**, càng sát ngưỡng 50 ms.
