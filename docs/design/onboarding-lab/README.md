# Onboarding "phòng lab" — hình minh hoạ và layout

Ngôn ngữ hình: bản vẽ kỹ thuật (blueprint) — lưới đo 20 px, đường guide cap/x-height/baseline, callout đánh số kèm legend có `TermTip`, đo kích thước, đường viền glyph THẬT của Inter (SIL OFL, đã đóng gói trong `UX/public/fonts`), tia quét, khay mẫu, wireframe → prototype.

- Glyph: `tools/design/extract_glyph_outlines.py` → `UX/components/onboarding/glyphOutlines.ts` (không dùng font của người dùng).
- Primitives: `blueprint.tsx`/`blueprint.css`; mỗi bước: `Illustrations.tsx`; quét: `ScanBlocks.tsx`; mở đầu/kết: `Intro.tsx`.
- Animation: CSS, chỉ `transform` / `opacity` / `stroke-dashoffset`; trạng thái mặc định = khung cuối, nên `prefers-reduced-motion` chỉ cần tắt animation (và bỏ intro, outro).
- Intro ≈ 2,8 s, nút "Bỏ qua"/Esc/Enter. Outro ≈ 1,5 s sau "Vào thư viện" ("Bỏ qua hướng dẫn" bỏ qua outro). Chỉ lần đầu và khi chuyển động được bật.
- Phím: ← quay lại; → hoặc Enter (khi không focus gì) chỉ kích hoạt nút an toàn (`data-keynav`): KHÔNG xác nhận giấy phép (O5), KHÔNG đổi cài đặt (O6, O7); Esc ở bước 1 = Bỏ qua hướng dẫn.
- Dev/QA (chỉ `vite dev`): `#first=1&ob=o1…end|intro|outro` (+`&hold=1` giữ intro/outro).
- Ảnh: `capture.sh` chụp bằng Chrome headless ở khung cuối (reduced-motion), sáng và tối, rộng 1280 và hẹp 720. `*-intro.png` là khung cuối của intro; outro không có ảnh tĩnh có nghĩa.
