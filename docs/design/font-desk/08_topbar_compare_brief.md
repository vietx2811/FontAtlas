# 08 — Top Bar Pair / Compare + màn Compare (brief cho build UI)

- **Ngày:** 2026-11-07 · **Người làm:** FontAtlas·05 Frontend · **Prototype:** `docs/design/prototype/topbar-compare-v1.html` · **Ảnh:** `docs/design/font-desk/shots-topbar/`
- **Căn cứ:** `docs/team/plans/2026-11-07-font-desk-plan.md` (mục A, C, D1 và "Chủ app đã trả lời"), `pairing-desk-v3.html`, `UX/components/main/TopBar.tsx`.
- **Quyết định của chủ app áp dụng:** Pair và Compare nằm ở **Top Bar**, không ở SelectionBar hay panel chi tiết.
- **Phạm vi:** chỉ prototype + brief. Chưa sửa React. Số đo, class, confidence trong prototype là **dữ liệu giả**; font Google là stand-in, không có font của chủ app.

## 1. Top Bar theo số font đang chọn

Cụm hành động nằm **sau nút Lọc, trước nút Lưới/Danh sách**, ngăn bằng một vạch dọc. Luôn có chip đếm `N chọn ✕` ở đầu cụm.

| Đang chọn | Cụm hiển thị | Nút chính (nền nhấn) |
|---|---|---|
| 0 | không có gì (Top Bar giữ nguyên như hiện tại) | — |
| 1 | `1 chọn ✕` · **Pair** `⇧P` · Find similar | Pair |
| 2–4 | `N chọn ✕` · **Compare** `⇧C` · Pair these `⇧P` | Compare |
| >4 | như 2–4, thêm **hàng gợi ý** dưới Top Bar: "N font — nên ≤ 4 để so rõ." + nút "Chỉ lấy 4 đầu" | Compare (không chặn) |
| Một Case đang mở | thêm nút viền đứt **Add to Case** ở cuối cụm (mọi số chọn ≥ 1) | giữ nguyên |

- **Compare với >4 font:** mở bình thường; bảng cuộn ngang. "Chỉ lấy 4 đầu" chỉ thu hẹp lựa chọn, không bắt buộc.
- **Vì sao 0 chọn không hiện gì:** nút bị vô hiệu chiếm chỗ mà không dẫn đường; Top Bar giữ gọn. (Chủ app có thể đổi sang hiện mờ.)
- **Hẹp (≤1100 px):** nút rút còn icon + tooltip (kèm phím tắt), kbd ẩn, tiêu đề co lại. Ở 1000 px cụm vẫn nằm cùng một hàng với ô tìm.
- **Phím tắt:** `⇧P` Pair (1 font) / Pair these (2–4), `⇧C` Compare (≥2). Hiện trong tooltip và ở nhãn khi đủ rộng.
- **SelectionBar hiện có** (`SelectionBar.tsx`) vẫn giữ việc đang làm (kích hoạt, thêm vào collection…); nút Pair/Compare **chuyển ra Top Bar**, không lặp.
- **Thông báo cho trình đọc màn hình:** cụm là `role="group"`, tên "Pair / Compare".

## 2. Màn Compare

Thay vùng nội dung (sidebar vẫn còn). `Esc` hoặc nút ‹ quay lại thư viện, giữ nguyên lựa chọn.

**Thanh điều khiển:** ‹ · `Compare · N` · **Same size | Match x-height** · **Overlay** (bật/tắt) · … · **PNG** · **PDF**.

- **Same size / Match x-height:** Same size = cùng cỡ chữ. Match x-height = chỉnh cỡ mỗi cột sao cho `x_height_em` bằng cột mốc (cỡ = cỡ gốc × `xh_mốc / xh_cột`). Áp dụng cho dòng specimen, dấu chồng và overlay.
- **Cột đầu là mốc** (nền sọc, icon neo). Mọi Δ tính so với mốc. Đổi mốc: bấm đầu cột (mở rộng sau, như v3).
- **Hàng, chia nhóm:**

| Nhóm | Hàng |
|---|---|
| Specimen | Dòng mẫu · **Overlay** (glyph `H x g ặ` của mốc xám + font cột màu nhấn, **chung baseline**, có đường x-height của mốc) · **Dấu chồng** (Ữ Ặ Ệ Ỗ Ở · ươ) |
| Số đo | x-height · cap height · contrast ⊥ · contrast H · slant I/T · stress · aperture · serif foot |
| Classification | class · subclass · confidence (xem mục 3) |
| Meta | license (chỉ ghi chú) |

- **Mỗi ô số đo:** số thật + đơn vị sát số (`em`, `°`, `×`, không đơn vị với tỉ số) · **Δ so với mốc** · thanh **percentile** `P{n}` (vị trí trong thư viện, từ `axis_stats`/`percentile`).
  - Δ: chiều dài/tỉ số → **Δ%**; slant → **Δ°** (độ lệch tuyệt đối, vì gốc 0). |Δ| nhỏ (<1% hoặc <0,5°) hiện `±0` xám.
  - Số theo `measurements.ts` (`precisionFor`, `numberText`): 3 số lẻ cho em, 2 cho tỉ số, 1 cho độ… dấu phẩy thập phân kiểu vi-VN, dấu trừ `−`.
  - **Đo thật đã có:** contrast ⊥ (vuông góc nét, core-0.4.3), `contrast_h`, slant đo trên I/T/\|.
  - **stress:** nhãn **"Ước lượng"** (chip nhấn) vì chưa đo trực tiếp. Không đo được (font sans, không có trục nhấn) → ô nét đứt "Không đo được" kèm lý do, không có Δ/percentile.
- **Dòng test tiếng Việt:** cỡ cố định, `line-height: 1`, đường đứt trên/dưới là hộp dòng. Dấu tràn khỏi hộp → viền trên đổi cam + chip cảnh báo **"Dấu tràn N%"**; vừa → "Dấu vừa khung". N lấy từ `vietnamese.rs` (tỉ lệ phần dấu vượt cao dòng, % của em).
- **License:** một dòng ghi chú ("Chỉ ghi chú, không chặn"). Không chặn bất cứ thao tác nào (memory *Tool, not gatekeeper*).
- **Xuất PNG/PDF:** nút ở góc phải; bản mẫu chỉ hiện toast "chưa xuất thật". Bản build: xuất đúng bảng đang xem (cả Δ, percentile, dòng dấu).

## 3. Classification trong Compare

DTO do 02 làm trên `feat/classify` (SHA `1359fdf`); prototype dùng đúng tên trường. Mỗi face: `FaceDetailDto.classificationInfo = {class, label, subclass, confidence, signals[], usage[], source: "auto"|"override", autoClass}`.

| Trường | Hiển thị |
|---|---|
| `label` | chữ đậm (Serif / Sans / Mono / Script / Display / Blackletter / Symbol) |
| `subclass` | chữ phụ, vd Didone/Modern, Geometric |
| `confidence` | thanh mảnh + `82%` |
| `source = "override"` | chip **Đã sửa**, tooltip `auto: {autoClass}` (kết quả tự động bị ghi đè) |
| `class = "uncertain"` | chip cam **Uncertain** + "Tín hiệu mâu thuẫn: …" lấy từ `signals[]`; không đoán |
| `usage[]` | Text / Display / UI; chưa hiện trong Compare, dành cho Detail |

- Sửa tay: `setClassificationOverride` / `clearClassificationOverride` (ô class sẽ có menu "Sửa…" / "Về tự động" ở bản build; prototype chưa vẽ).
- Lọc Thư viện: `FaceQueryDto.classes[]`, `subclasses[]` (OR trong nhóm, AND giữa nhóm, dùng phân loại hiệu lực); số đếm cho mặt lọc từ `classFacets(q) → [{class,count,subclasses:[{subclass,count}]}]` (đếm bỏ qua chính classes/subclasses).
- Để vòng sau: đánh dấu nhẹ cột có class khác mốc.

## 4. i18n

- Mọi chuỗi UI đi qua khoá i18n; prototype dùng `t(key)` với khoá `topbar.*` và `compare.*` (xem dict `T` trong file), build React dùng đúng các khoá này.
- Thuật ngữ font giữ tiếng Anh kèm chú giải tiếng Việt (x-height / "cao chữ thường", contrast ⊥ / "tương phản nét (vuông góc)", stress / "trục nhấn", aperture / "độ mở", serif foot / "chân serif"…). Chú giải là chữ phụ dưới nhãn hàng.
- Tên tính năng giữ EN: Pair, Pair these, Compare, Find similar, Add to Case, Case, Classification, Uncertain.

## 5. Cần từ backend

| Việc | Trạng thái | Ghi chú |
|---|---|---|
| Lọc phân loại + `classificationInfo` + override | **đã có** trên `feat/classify` (`1359fdf`): `classes`, `subclasses`, `classFacets`, `setClassificationOverride`/`clear…` | UI Thư viện thêm mặt lọc Phân loại khi merge |
| Command Compare: trả số đo + percentile + Δ cho N face một lần | mới | tránh N lần `get_face`; kèm `vi_overflow_pct` cho từng face |
| `vi_overflow_pct` (dấu tràn khỏi hộp dòng) | có logic ở `vietnamese.rs`, chưa lộ DTO | cần DTO |
| `stress_deg` có cờ `estimated` | cần chốt | để hiện nhãn "Ước lượng" |
| Command "Pair these" (2–4 font → 2–3 Combo) | mới (kế hoạch mục A) | dùng lại `pairing.rs`, `pair_axes.rs`, `explain.rs` |
| Xuất PNG/PDF | UI/Tauri | có thể làm phía client (canvas/print) |
| `formats`, `hasImage`, "thiếu dấu" trong bộ lọc Thư viện | vòng sau | ghi theo yêu cầu Lead |

## 6. Mở / để vòng sau

- Đổi mốc trong Compare; kéo đổi chỗ cột (theo v3).
- Hàng Meta đầy đủ (Designer, Foundry, Version, Glyphs, VI x/186) lấy từ v3 khi build.
- Sau khi bấm Pair these: màn Pairing theo v3 (không thuộc brief này).

## 7. Cách xem prototype

Mở file HTML trong trình duyệt (cần mạng để tải font Google). Tham số URL:

```
?screen=library|compare  &sel=0|1|2|3|4|6  &case=1  &n=2|3|4  &theme=dark  &fit=xh  &overlay=0  &cls=0  &scroll=1
```

Ảnh 1400×920 và 1000×700, sáng + tối, trong `shots-topbar/` (`01`–`05` Top Bar, `07`–`09` Compare). Ảnh dùng font Google stand-in, không phải font của chủ app.

## 8. Trạng thái build — `feat/fontdesk-ui` (bước 3)

Đã làm trong app (mock + Tauri), ảnh ở `docs/design/font-desk/shots-fontdesk-ui/`:

| Phần | Ở đâu | Ghi chú |
|---|---|---|
| Cụm Pair/Compare trên Top Bar | `UX/components/main/TopBarActions.tsx` | 0 chọn: ẩn · 1: Pair · Find similar · 2–4: Compare · Pair these · >4: thêm hàng gợi ý + "Chỉ lấy 4 đầu". ⇧P/⇧C (bỏ qua khi đang gõ). Bên hẹp (panel chi tiết mở) nút chỉ còn icon, tooltip giữ nhãn + phím tắt |
| Chip phân loại ở đầu panel chi tiết | `components/detail/ClassChip.tsx` | `class · subclass`; Uncertain → "có lẽ là {candidate}" mờ viền đứt; tooltip: độ chắc, signals, usage (Display nằm ở usage, **không** hiện như class); chip **Đã sửa** + tooltip "Máy nghĩ: …"; menu đổi class / "Về tự động" |
| Nhóm "Phân loại" trong panel Lọc | `components/main/FilterPanel.tsx` | từ `classFacets` (đếm); subclass hiện khi chọn class; ẩn class `display`; chip gỡ ở dưới Top Bar |
| API | `api/index.ts`, `tauri.ts`, `mock.ts` | `classFacets`, `setClassificationOverride`, `clearClassificationOverride` |
| Chuỗi mới | `UX/i18n/index.ts` | khoá `topbar.*`, `class.*`, `filter.*`, `chips.*` |

### TODO còn lại (không giả dữ liệu trong app thật)

- **Compare:** chưa có màn Compare và chưa có command backend (số đo + percentile + Δ cho N face, `vi_overflow_pct`, cờ ước lượng stress). Nút **Compare** hiện mở tab **Giải phẫu** của font đầu tiên và báo bằng notice "Compare chưa có…". Khi có DTO: thay `compare()` trong `TopBarActions.tsx` bằng điều hướng sang màn Compare (prototype `topbar-compare-v1.html`).
- **Pair these:** chưa có command (2–4 font → 2–3 Combo). Hiện mở tab **Pair** của font đầu tiên; ⇧P cũng vậy.
- **Add to Case:** app chưa có khái niệm Case (chỉ có "dự án"/collection), nên chưa có nút; thêm khi Pairing Desk có Case mở.
- **traits[]:** 02 đang làm trên `feat/classify-display-tag`. Hiện Display chỉ nằm trong `usage`; chỗ hiển thị traits để trống (tooltip của chip phân loại là nơi dự kiến).
- **CONF_MIN 0.88:** áp dụng ở backend; UI chỉ hiển thị `class = uncertain` + `candidate` như backend trả.
- Mục `formats`, `hasImage`, "thiếu dấu" của bộ lọc vẫn để vòng sau.

## 9. Trạng thái build — `feat/compare-ui` (màn Compare)

- **Màn:** `UX/components/compare/ComparePage.tsx` (+ `CompareCells.tsx`, `compareMath.ts`, `exportPng.ts`). Mở từ Top Bar (Compare / ⇧C) với 2–8 face; Esc hoặc ✕ quay lại, thư viện vẫn mount nên vùng chọn và vị trí cuộn còn nguyên. Ảnh: `shots-compare-ui/`.
- **Dữ liệu thật, không giả:** `api.compareFaces(faces)` hiện **ghép từ `getFace`** (số đo `axes` + percentile + `quality`, `vietnamese.stackedMarksOverflow/TopEm`, `classificationInfo`, license). Một face lỗi → cả lệnh lỗi, màn hiện lỗi + "Thử lại". Số đo không đo được → ô "Không đo được" + lý do, không điền số.
- **Khi 02 giao `compare_faces`:** chỉ đổi thân `compareFaces` trong `api/tauri.ts` (giữ kiểu `CompareDto` ở `api/compare.ts`; map DTO mới sang đó). Không phải đổi UI.
- **Dấu chồng:** backend chỉ có cờ `stackedMarksOverflow` + `stackedMarksTopEm` (em), không có "% tràn" → UI hiện "Dấu tràn khung · {em} em". Nếu muốn % cần `vi_overflow_pct` trong DTO.
- **Δ:** % cho độ dài/tỷ lệ; **độ tuyệt đối** cho slant/stress (mốc 0 vẫn có nghĩa). |Δ| < 1% hoặc < 0,5° → "±0".
- **Match x-height:** cỡ cột = cỡ gốc × `x_height_em(mốc) / x_height_em(cột)`; thiếu hoặc giá trị vô lý → không đổi cỡ.
- **TODO:** PDF (nút đang khoá). **PNG:** vẽ canvas (chờ font nạp xong, ×devicePixelRatio) rồi gọi `api.savePng(blob, tênGợiÝ)`. Mock/trình duyệt: tải xuống. **Tauri: cần lệnh `save_png`** (đề xuất `save_png(suggested_name: String, png_base64: String) -> Option<String>`: hộp thoại Lưu lọc .png + ghi file; trả đường dẫn hoặc `null` nếu huỷ). Chỉ báo "Đã lưu" khi có đường dẫn; huỷ thì im; lỗi báo lỗi.
- **Face không đọc được:** cột đó được bỏ, banner ghi lý do; còn < 2 face thì cả màn lỗi + "Thử lại". Tối đa 8 font (Top Bar báo khi cắt).
- **Chưa có:** đổi mốc, kéo đổi cột, thêm/bớt cột trong màn (làm khi cần). Hàng Meta chỉ có license (Designer, Foundry, Version… theo v3 để vòng sau).
