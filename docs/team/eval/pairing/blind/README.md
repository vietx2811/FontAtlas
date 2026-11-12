# Bộ kiểm mù cặp chữ (G4)

30 thẻ: 20 gợi ý của mô hình `fontatlas_core::pairing` (mức Good ở phong cách của nó, vai đều ≥ Ok) xen kẽ ngẫu nhiên với 10 cặp ngẫu nhiên làm đối chứng. Chỉ font OFL (google/fonts@47d54ca1), mỗi font đủ tiếng Việt, mỗi font chỉ xuất hiện một lần, bỏ font > 400 KB.
Thẻ chỉ hiện vai, chữ mẫu có dấu chồng và tên font; không hiện phong cách, mức hay nguồn. Chủ app chấm Hợp / Tạm / Không.

Phân bố 20 gợi ý: contrast 7 (5 serif→sans, 2 sans→serif), superfamily 2, same_structure 3, same_era 3, display_vs_neutral 5. `harmony` chưa có vì chưa dùng embedding.

## Các tệp

| Tệp | Vai trò |
|---|---|
| `index.html` | trang tĩnh, mở bằng trình duyệt (cần thư mục `fonts/` cùng cấp) |
| `fonts/` | font OFL cần cho trang, ~12 MB, **không commit** (`.gitignore`); dựng lại bằng lệnh dưới |
| `key.json` | đáp án: thẻ nào là mô hình hay đối chứng, phong cách, mức mô hình cho cả 6 phong cách. Không đưa cho người chấm |

## Dựng lại (trong `UX/src-tauri`, cùng seed thì cùng kết quả)

```
cargo run --release -p fontatlas-eval --bin pairing_blind -- --gf ~/fontatlas-ref/google-fonts --out ../../docs/team/eval/pairing/blind
```

Để gửi chủ app: nén `index.html` + `fonts/` (không kèm `key.json`).

## Tính kết quả

Chủ app bấm "Xuất kết quả (JSON)" sau khi chấm đủ 30 thẻ, được `pairing-blind-results.json`:

```
cargo run --release -p fontatlas-eval --bin pairing_blind -- --score pairing-blind-results.json --key ../../docs/team/eval/pairing/blind/key.json
```

In bảng Hợp / Tạm / Không theo nhóm (mô hình, đối chứng, từng phong cách). Mục tiêu đã ghi ở tài liệu nghiên cứu: ≥ 70% Hợp+Tạm ở nhóm mô hình và cao hơn rõ rệt đối chứng; số mẫu nhỏ (20 và 10) nên chỉ là tín hiệu, không phải ước lượng chặt.

**Không chỉnh ngưỡng theo bộ này** (đây là bộ giữ ra ngoài). Nếu kết quả xấu thì ghi lại và quyết định sửa mô hình bằng bộ dữ liệu khác.
Trang chỉ có tiếng Việt vì là công cụ nghiên cứu cho một người chấm, không phải giao diện của app.
