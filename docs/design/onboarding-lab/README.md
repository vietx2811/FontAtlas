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

## 0.3.0 — chỉnh gọn (feat/onboarding-polish)
- Splash: wordmark nhỏ hơn (`--wm` 34–76 px) và xám nhạt (pha `--fg` vào `--sheet`, token nên sáng/tối đều đúng); vạch đo nhạt theo. Vạch baseline/x-height đo lại khi font nạp xong (`document.fonts.ready`) và khi đổi cỡ cửa sổ — trước đó có thể lệch baseline khi frame cuối hiện trước khi font sẵn sàng.
- Onboarding: chỉ báo bước là chấm tròn đơn giản; bỏ chữ "Bước x/y" và dòng gợi ý phím (phím ← → Enter Esc vẫn chạy; mô tả cho trình đọc màn hình giữ nguyên, ẩn). Ô minh hoạ vuông (viewBox 480×480, tối đa 400 px), KHÔNG viền khung và bỏ góc khung; lưới, thước, vạch đo, callout giữ nguyên.
- Trước màn kết: bảng "Phím tắt & tính năng chính" (`UX/components/help`) — lưới 9 thẻ lấy từ danh sách phím thật (`shortcuts.ts`, mỗi mục ghi nơi đăng ký phím); Esc / bấm nền / "Bắt đầu" đều vào thư viện. Mở lại bất kỳ lúc nào bằng phím `?` (trừ khi đang gõ chữ, có hộp thoại/menu hoặc đang trong hướng dẫn).
- Chụp: `capture.sh` có thêm `-tips` (hash dev `ob=end&tips=1`).
- Cập nhật sau góp ý chủ app: BỎ khối chú giải dài dưới hình ở mọi bước. Trên hình chỉ còn nhãn ngắn cạnh vạch (tên thuật ngữ + số đo, vd "cap height 0.728 em"); rê chuột/focus vào nhãn mới hiện TermTip. Bố cục cân lại: lưới 48/52, hình vuông ≤ 520 px (≈45% bề ngang) canh giữa theo chiều dọc, lề trái hình = lề trái nút "Quay lại" (cùng cột 1120 px), khối chữ cùng lề cột phải, nút nằm đúng đường chân của hình; cửa sổ hẹp (≤ 880 px) xếp dọc. Ảnh kiểm: 1280×800, 1440×900 (`*-wide-o1`), 720×900 (`*-narrow-o1`).
- Chỉnh thẩm mỹ lần 2: cả cụm (hình + chữ) canh giữa ngang (lề ngoài hai bên bằng nhau, nút "Quay lại" theo lề cụm: biến `--onb-w`); nút đứng ngay sau nội dung (cách ~32 px) và cả khối chữ canh giữa dọc so với hình; bỏ vạch "top" đè lên đỉnh dấu breve của Ặ (dấu là bát mở nên hai đầu cụt là đúng hình glyph; vạch cũ làm nó trông bị cắt).
