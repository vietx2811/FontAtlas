# Eval phân loại: serif chân nhỏ (core-0.8.0 → core-0.9.0)

Dữ liệu: `google/fonts@47d54ca1ebb14fd864695f1d39dc734f39017c33` (sparse, `~/fontatlas-ref/google-fonts`), tag `docs/team/eval/gf-tags-families.csv`.
CSV chỉ có tên họ GF và số đo; không có font hay đường dẫn của chủ app.

Lệnh (trong `UX/src-tauri`; "trước" chạy ở commit `14f0435`, "sau" ở tip nhánh):

```
cargo run --release -p fontatlas-eval --bin classify_eval -- \
  --gf ~/fontatlas-ref/google-fonts --tags ../../docs/team/eval/gf-tags-families.csv \
  --split all --out out.md --dump dump.csv            # mặt Regular
# thêm --faces all cho mọi mặt; --split tune | test cho tách chỉnh/kiểm
```

| Tập | | đúng | Uncertain | sai | đúng/đã gán |
|---|---|---|---|---|---|
| Regular, 1722 họ | trước | 1208 | 451 | 63 | 95.0% |
| | sau | 1208 | 461 | 53 | 95.8% |
| mọi mặt, 3392 | trước | 2052 | 1257 | 83 | 96.1% |
| | sau | 2064 | 1264 | 64 | 97.0% |
| mặt Italic (639) | trước | 137 | 496 | 6 | |
| | sau | 151 | 486 | 2 | |
| tune (827 họ, dùng chỉnh) | trước → sau | 571 → 569 | 227 → 236 | 29 → 22 | 95.2 → 96.3% |
| **test (895 họ, giữ ra ngoài)** | trước → sau | 637 → 639 | 224 → 225 | 34 → 31 | 94.9 → 95.4% |

`before_*.csv` / `after_*.csv`: mỗi dòng một họ (số đo, đáp án, dự đoán); bản "trước" không có cột `flare`, `head`.

## 0.4.0·A3b: chân một phía → Sans (tailed_l_sans)

Luật mới trong `classify.rs`: khi `serif_foot` > 1.6 (hoặc 1.2–1.6 không có flare/cờ) mà `serif_flare` < 1.15 và `serif_head` < 0.05, bỏ phiếu Sans 1.5 (trước: không phiếu nào, ra Uncertain). Chỉ áp cho chữ văn bản: `spacing` ≥ 0.12 và `bounce` < 0.04 (bản đầu không có điều kiện này làm 5 script đúng thành Uncertain: Edu AU VIC WA NT Dots, Ephesis, Licorice, Ole, The Girl Next Door).

| Tập | | đúng | Uncertain | sai |
|---|---|---|---|---|
| Regular, 1722 họ có đáp án (`classify`) | core-0.9.0 | 1208 | 461 | 53 |
| | + luật chân một phía | 1213 | 456 | 53 |
| taxonomy v2, `all` (nay 229 họ) | trước | 140 | 42 | 13 |
| | sau | 148 | 38 | 13 |

Đổi trạng thái (1722 họ): Aboreto, Bellota, Genos, Mooli, Ruluko Uncertain → Sans (đều đúng); Redaction 70 Uncertain → Sans (không có đáp án). Không họ nào mất. Fixture `tailed_l_sans`: 0/4 → 3/4 (còn Port Lligat Sans: flare 1.32 và cờ 0.20 là bằng chứng serif yếu, nên Uncertain).
Ngưỡng `spacing`/`bounce` chọn sau khi thấy 5 script bị ảnh hưởng, trên cùng tập: là số trên tập đã chỉnh.
