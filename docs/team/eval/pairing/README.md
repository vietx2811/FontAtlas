# Eval G4: hợp vai và phong cách pair (04, 2026-11-12)

Mô hình: `UX/src-tauri/crates/fontatlas-core/src/pairing.rs` (đề xuất: `docs/team/research/2026-11-12-g4-role-fit-and-pair-styles.md`).
Bộ vàng: `docs/team/eval/pairing_gold.csv` (16 cặp, 11 vai). Chạy (trong `UX/src-tauri`, cần `google/fonts@47d54ca1` như taxonomy v2):

```
cargo run --release -p fontatlas-eval --bin pairing_eval -- --gf ~/fontatlas-ref/google-fonts --gold ../../docs/team/eval/pairing_gold.csv --out ../../docs/team/eval/pairing/after.md
```

Kết quả mới nhất: `after.md`.

## Số liệu

| | cặp đúng kỳ vọng | vai đúng kỳ vọng |
|---|---|---|
| Ngưỡng khởi điểm (đoán, chưa đo) | 11/16 | 8/11 |
| Sau hiệu chuẩn | 15/16 | 10/11 |

## Cách hiệu chuẩn, và giới hạn của con số này

- Chạy lần đầu cho thấy ngưỡng khởi điểm lệch so với số đo thật: `aperture` thực tế nằm 37–118 (Open Sans 105, Roboto 53), `spacing` sans ≈ 0.18 (không phải 0.11), `aperture_e` sans thường 0.13–0.31.
  Vùng đích của Body/Caption/Heading/Label/Numerals được đặt lại theo phân vị 10–90 của 1722 họ OFL (Serif và Sans, mặt Regular; `docs/team/eval/classify-small-serif/after_core-0.9.0_regular.csv`).
- `distinctiveness` (Display) lấy thêm dấu hiệu từ số đo vì class hay là Uncertain với font cá tính: Pacifico (thân rất đậm, aperture_e ≈ 0.03), Bebas Neue (x-height bằng cap-height); thêm `x_height_cap` vào `Traits`.
- **Cảnh báo overfit:** các chỉnh trên làm sau khi thấy kết quả trên chính bộ vàng 27 dòng này. 15/16 và 10/11 vì thế là số "đã chỉnh trên tập kiểm", không phải ước lượng độc lập. Cần bộ giữ-ra-ngoài (cặp do chủ app chọn, mục 5 của tài liệu nghiên cứu) trước khi tin vào con số.
- Cặp loại `memory` là cặp hay được gợi ý trong các bài pairing, nhớ lại, chưa kiểm nguồn trực tiếp. Lead yêu cầu lấy từ Google Fonts pairings: tôi chưa có bản chép chính thức nên không trích; cần ai đó mở nguồn và bổ sung.

## Phát hiện đáng giữ

- Poppins thiếu 88 ký tự tiếng Việt, Abril Fatface và Bebas Neue thiếu 92: các cặp pairing phổ biến trên mạng hay dùng chúng, nhưng với người dùng Việt chúng không làm Body được. Mô hình bắt đúng (`vi_missing`), nên ở bộ vàng tôi thay Poppins bằng Source Sans 3.
- Còn sai 2 dòng: Oswald + Lora (`contrast`) ra `unknown` vì class Oswald là Uncertain; Pacifico ở vai Body ra `ok` thay vì `weak` (class Uncertain nên không có `class_unfit`). Cả hai cùng gốc: phân loại chưa nhận ra font cá tính/condensed. Để sau khi xử lý nợ `tailed_l_sans`/Uncertain.
- Lý do `distinctiveness_low` vẫn hiện ở Playfair Display (điểm cá tính 0.70, dưới ngưỡng 0.75 của lý do) dù mức là good: cần UI hiểu lý do là "điểm yếu nhất" chứ không phải "vì sao bị hạ".
