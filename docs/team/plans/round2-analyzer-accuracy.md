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
- Bảng trước/sau đo trước khi chốt gate "khai báo italic" (phương án B). Sau gate, test `upright_script_is_not_deskewed_from_a_curved_l` xác nhận script upright không bị dựng; chưa chạy lại toàn bộ 35 nhãn a/g.
- Contrast font condensed vẫn là tỉ lệ thô (Estimated), chưa có công thức chuẩn hoá kiểm chứng.
