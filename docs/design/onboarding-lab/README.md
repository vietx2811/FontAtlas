# Onboarding "phòng lab" — hình minh hoạ và layout

Ngôn ngữ hình: bản vẽ kỹ thuật (blueprint) — lưới đo 20 px, đường guide cap/x-height/baseline, callout đánh số kèm legend có `TermTip`, đo kích thước, đường viền glyph THẬT của Inter (SIL OFL, đã đóng gói trong `UX/public/fonts`), tia quét, khay mẫu, wireframe → prototype.

- Glyph: `tools/design/extract_glyph_outlines.py` → `UX/components/onboarding/glyphOutlines.ts` (không dùng font của người dùng).
- Primitives: `blueprint.tsx`/`blueprint.css`; mỗi bước: `Illustrations.tsx`; quét: `ScanBlocks.tsx`; mở đầu/kết: `Intro.tsx`.
- Animation: CSS, chỉ `transform` / `opacity` / `stroke-dashoffset`; trạng thái mặc định = khung cuối, nên `prefers-reduced-motion` chỉ cần tắt animation (và bỏ intro, outro).
- Mở đầu = **Splash** (`UX/components/splash`), dùng chung khi mở app và cho onboarding lần đầu: "FontAtlas" đảo font ngẫu nhiên (từ thư viện, chỉ font có file thật, không ở ổ cloud, đủ glyph, ≤ 14; thư viện trống thì đảo font đóng gói), nhịp chậm dần, khoá lệch nhịp; frame cuối "Font" = Newsreader, "Atlas" = Inter, cùng x-height, chung baseline, không khoảng trắng, vạch baseline/x-height. ~2 s, phím/bấm bất kỳ bỏ qua, reduced-motion = frame cuối. Số đo x-height hard-code (tools/design/measure_wordmark_fonts.py) và test kiểm.
- Outro ≈ 1,5 s sau "Vào thư viện" ("Bỏ qua hướng dẫn" bỏ qua outro); tắt khi giảm chuyển động.
- Phím: ← quay lại; → hoặc Enter (khi không focus gì) chỉ kích hoạt nút an toàn (`data-keynav`): KHÔNG xác nhận giấy phép (O5), KHÔNG đổi cài đặt (O6, O7); Esc ở bước 1 = Bỏ qua hướng dẫn.
- Dev/QA (chỉ `vite dev`): `#first=1&ob=o1…end|outro` (+`&hold=1` giữ outro); `#splash=hold` giữ splash.
- Ảnh: repo chỉ giữ 6 ảnh tiêu biểu (dark-o1, light-o1, dark-o4, dark-splash, light-splash, light-narrow-o1); bộ đầy đủ chụp ra ngoài repo bằng `OUT_DIR=/tmp/onb-shots bash capture.sh`. `capture.sh` chụp bằng Chrome headless ở khung cuối (reduced-motion), sáng và tối, rộng 1280 và hẹp 720. `*-splash.png` là frame cuối của splash; outro không có ảnh tĩnh có nghĩa.
