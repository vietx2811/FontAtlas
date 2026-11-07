# Plan: Font Desk — lối vào, Compare, dữ liệu tự phát hiện (Phân loại)

- **Ngày:** 2026-11-07 · **Người giao:** session fontatlas-63 → **FontAtlas·00 Lead [d89cc0]**
- **Nghiên cứu & quyết định:** `.claude/worktrees/font-desk/docs/design/font-desk/07_entry_points_research.md` (nhánh docs/font-desk, chưa commit)
- **Đã chốt (chủ app):** Brief starter 1 màn thay wizard · "Pair these" khi chọn 2–4 font · AI = Claude Code CLI local, **làm ở session khác — không thuộc plan này**
- **Yêu cầu chủ app:** bổ sung mọi thông tin còn thiếu mà công cụ detect tự động được; **Phân loại phải làm đủ**.
- Code tham chiếu: worktree `lead-dev` (dev/mvp).

## A. Lối vào từ Library (UI)
Code nằm ở `lead-dev/UX/components/main/SelectionBar.tsx`, `TopBar.tsx`, `useLibrarySelection.ts` và `faces/faceMenuModel.ts`.
- SelectionBar hiện chỉ xuất hiện khi chọn từ 2 font trở lên. Cần cho nó hiện cả khi chọn **1 font**, hoặc thêm nút Pair vào detail panel. Lead quyết định.
- Nút hiển thị theo số font đang chọn:

  | Đang chọn | Nút |
  |---|---|
  | 1 font | Pair · Find similar |
  | 2–4 font | Compare · Pair these |
  | >4 font | Compare, kèm gợi ý thu hẹp, không chặn |
  | Đang mở một Case | thêm Add to Case |

- Phím tắt `⇧P` / `⇧C`. Mọi chuỗi giao diện đi qua i18n key. Thuật ngữ font giữ tiếng Anh, kèm chú giải.
- "Pair these": engine thử mọi cách gán vai giữa các font đã chọn, rồi trả về 2–3 Combo. Dùng lại `pairing.rs`, `pair_axes.rs` và `explain.rs`, nên cần thêm một command mới ở backend.

## B. Brief starter (Font Desk, không dùng AI)
- Chỉ hiện khi Case còn trống. Gồm 3 nhóm chip:
  - **Dùng cho**: Editorial / Branding / Poster / UI-Web / Packaging
  - **Giọng**: 2–3 thanh trượt dạng cặp đối lập
  - **Ràng buộc**: tiếng Việt (bật mặc định), chỉ font đã cài, có italic, variable, license
- Ánh xạ chip sang filter: dùng `fontatlas-describe` cùng `data/descriptor-dictionary.json` (mở rộng thêm từ khóa) và các class mới ở mục D.
- Kết quả là 3 font neo hoặc Combo khởi đầu, sau đó vào luồng Pairing bình thường.
- Chừa sẵn một chỗ "Kết nối AI" để session AI làm sau.

## C. Bổ sung cho Compare (theo nghiên cứu)
- Bật/tắt "Same size / Match x-height" (dùng `x_height_em` đã đo).
- Overlay glyph của hai font, căn chung baseline.
- Mỗi chỉ số hiện Δ% so với baseline, kèm percentile.
- Một dòng test tiếng Việt cố định, cảnh báo khi dấu chồng bị tràn (đã có `vietnamese.rs`).
- Hiện license, chỉ để ghi chú, không chặn.
- Xuất bảng so sánh ra PNG/PDF.

## D. Dữ liệu còn thiếu mà công cụ phát hiện tự động được

### D1. Phân loại đầy đủ (ưu tiên cao nhất)
Hiện tại chỉ có Mono/Sans/Serif hoặc null (`services/analysis.rs` khoảng dòng 641). Cần đổi sang taxonomy hai tầng, dựa trên Vox-ATypI có chỉnh lại cho dùng trong thực tế:

| Class | Subclass |
|---|---|
| Serif | Humanist/Old-style · Transitional · Didone/Modern · Slab (Egyptian, Clarendon) · Glyphic/Wedge |
| Sans | Grotesque · Neo-grotesque · Geometric · Humanist |
| Mono | (Serif mono / Sans mono) |
| Script | Formal · Casual/Brush · Handwritten |
| Display | Decorative · Stencil · Inline/Outline · Rounded · Fat face |
| Blackletter | — |
| Symbol/Dingbat | — |

Ngoài class còn có **tag dùng cho mục đích sử dụng**: Text / Display / UI, suy ra từ opsz, x-height, contrast, spacing và stem.

**Tín hiệu dùng để phân loại, theo thứ tự ưu tiên:**
1. Chỉ số đo được đã có:
   - `serif_foot`, `contrast`, `contrast_h`, `stress_deg`
   - `aperture`, `aperture_e`
   - `a_storeys`, `g_storeys`
   - `terminal`, `width`, `width_cv`, `slant_deg`, `darkness`
2. **Panose** (đã trích nhưng chưa dùng): family kind (2 = Latin text, 3 = script, 4 = decorative, 5 = symbol) và serif style.
3. **OS/2 `sFamilyClass`** (IBM class), **chưa trích, cần thêm**.
4. Từ khóa trong tên font: Slab, Mono, Script, Stencil, Rounded, Display, Text, Caption, Grotesk, Didone…
5. Kiểm tra cmap, để nhận ra font Symbol/Dingbat.
6. Tín hiệu đo thêm cho Script: các nét chữ thường nối liền nhau, phát hiện bằng cách so sánh độ chồng giữa bounding box và advance width.

**Cách hoạt động:**
- Mỗi lần phân loại trả về `class`, `subclass`, `confidence` và `signals[]` (để giải thích lý do).
- Nếu các tín hiệu mâu thuẫn nhau thì gán "Uncertain", không đoán.
- Người dùng được ghi đè. Bản ghi đè lưu ở user DB và được ưu tiên hơn kết quả tự động.
- Kết quả được lưu kèm phiên bản analyzer.

**Kiểm chứng:**
- Dùng `google/fonts` → `tags/all/families.csv` (có các tag như /Sans/Geometric, /Serif/Didone) và `category` trong METADATA.pb làm ground truth.
- Chạy qua eval harness có sẵn (`docs/team/eval/`).
- Báo cáo confusion matrix. Ngưỡng nghiệm thu: class ≥ 90%, subclass ≥ 70%, phần còn lại rơi vào Uncertain chứ không bị gán sai. Lead xác nhận lại các ngưỡng này với anh/chị.

### D2. Các dữ liệu khác

| Dữ liệu | Cách phát hiện | Dùng cho |
|---|---|---|
| **Độ phủ ngôn ngữ (Hyperglot)** | Dữ liệu Hyperglot (đã chốt là nguồn duy nhất). **Lead kiểm tra license của Hyperglot** (code và data) trước khi bundle hoặc port sang Rust | Filter ngôn ngữ, Brief starter, Compare |
| Vendor ID | OS/2 `achVendID` | Hiện foundry, gom theo foundry, "same foundry" trong pairing |
| sFamilyClass | OS/2 | D1 |
| Năm | `head.created/modified`, regex lấy năm trong copyright, version | Thời kỳ, sắp xếp |
| Nhóm OpenType feature | Ánh xạ tag sang nhóm dễ đọc: Figures (lnum/onum/tnum/pnum/zero), Small caps (smcp/c2sc), Fractions, Case, Ligatures (liga/dlig), Swash, Stylistic sets (lấy tên ss01–20 từ name table) | Compare, kiểm tra parity trong Combo |
| Số kerning pair | GPOS `kern` / bảng `kern`, đếm số cặp và số class | Cờ chất lượng trong Compare |
| Cấu trúc family | Có italic thật hay chỉ oblique, dải weight, các width, family có opsz | Compare, Pairing (italic parity) |
| Diễn giải fsType và mở rộng `license_class` | Giải mã bit fsType. Luật từ khóa để nhận OFL / Apache / UFL / EULA thương mại / không rõ | Ghi chú license (không chặn) |
| Trùng lặp và phiên bản | Cùng PostScript name, khác version hoặc hash | Dọn library, tránh so trùng |
| Lịch sử dùng | Không đọc được từ file. Ghi log trên máy: lần mở trong Case, lần activate, lần xuất | Gợi ý theo thói quen. Ưu tiên thấp |

**Các file chính sẽ đụng tới:**
- `fontatlas-core/src/metadata.rs` (thêm achVendID, sFamilyClass, ngày tháng)
- `anatomy.rs` (tín hiệu Script)
- module classify mới trong `fontatlas-core`, thay đoạn phân loại trong `services/analysis.rs`
- `fontatlas-store/src/schema.rs` (migration)
- các DTO trong `bindings/`
- `fontatlas-describe/data/descriptor-dictionary.json`
- UI: filter Library và các hàng trong Compare

## E. Thứ tự gợi ý cho Lead
Theo memory team-throttle: tối đa 2 worker chạy song song, worker và reviewer dùng Sonnet.
1. **Backend dữ liệu**: D1 Phân loại, rồi D2 metadata, OT groups, family structure. Hyperglot làm sau khi đã rõ license.
2. **Visual**: prototype HTML cho SelectionBar, Brief starter và Compare (phần C), dựa trên `pairing-desk-v3.html`. Chạy song song với bước 1.
3. **Build UI**: làm A, B, C khi đã có brief visual và DTO từ bước 1.
4. **Review độc lập**: reviewer khác model với người viết, cộng eval phân loại.

---

## Nghiệm thu
- Phân loại: confusion matrix trên Google Fonts tags; class ≥90%, subclass ≥70%, còn lại Uncertain (Lead xác nhận ngưỡng với chủ app).
- UI: kiểm tra bằng ảnh chụp app thật (không chỉ build pass); i18n key cho mọi chuỗi; thuật ngữ font EN + gloss.
- Hyperglot: chỉ bundle sau khi xác nhận license.

## Chủ app đã trả lời (2026-11-07)
1. **Pair** và **Compare** đặt ở **Top Bar**, không đặt ở SelectionBar hay detail panel.
2. Về độ chính xác: chủ app yêu cầu "cần giảm thêm sai số". Lead đã làm ngay ở analyzer core-0.4.3, xem `round2-analyzer-accuracy.md`. Ngưỡng nghiệm thu Phân loại giữ class ≥ 90%, subclass ≥ 70% cho tới khi chủ app nói khác.
3. Thứ tự làm D trước hay UI trước: chưa trả lời. Mặc định theo mục E.

## Câu hỏi ban đầu
1. SelectionBar hiện khi chọn 1 font, hay chỉ thêm nút Pair ở detail panel?
2. Ngưỡng accuracy phân loại.
3. Thứ tự ưu tiên: dữ liệu (D) trước UI (A–C) như mục E?
