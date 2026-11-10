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
