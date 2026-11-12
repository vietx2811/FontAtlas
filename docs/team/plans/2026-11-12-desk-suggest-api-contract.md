# Hợp đồng API gợi ý cho Pairing Desk (G4/G5/G6), đề xuất của 08

Trạng thái: **đề xuất, chưa code**. Chờ (1) mô hình hợp vai/phong cách của 04, (2) spec cột 2 của 01, (3) Lead duyệt.
Nhánh: `feat/desk-suggest-api` (từ `origin/dev/mvp` d6c65504). Ngày 2026-11-12.

## 1. API sẵn có, dùng lại được

| API | Việc nó làm | Dùng cho |
|---|---|---|
| `similar` / `similar_families` (`services/analysis.rs`) | Gần nhất theo 4 nhóm trục (proportion, weight, structure, detail) + cosine ảnh (embedding int8). Lọc `vi_only`, `source_ids`, `weight_range`, `FaceQueryDto` (qua `matching_keys`). Trả `same`/`differ` theo trục. | G5 "tương tự" |
| `pair_candidates` / `pair_candidates_gated` (`Index`) | Quét toàn bộ profile bằng rayon, `assess()` từng cặp, cổng `Safe/Controlled/Experimental/Blocked` áp **trước** top-k, tie-break bằng cos ảnh. Role chỉ có `Heading/Body/Code`. Có `collection_id`, `project_vi`. | G4/G5 "hợp vai" (nền) |
| `assess_pair`, `pair_these` (`compare.rs`) | Chấm một cặp; thử mọi cách gán heading/body trong 2–4 face. | G4 kiểm lại sau khi chọn |
| `describe` (`services/describe.rs`) | Văn bản → bộ lọc trục (từ điển, không LLM); có `matched`, `conflicts`, `unknownWords`, `hard`. | G6 "tên combo → ý định" |
| `query_faces` (`FaceQueryDto`) | Lọc class/subclass/trait/ngôn ngữ/vi/license/axis, `faceKeys` ≤ 2000, `sort=match`. | G5 lọc nhanh theo class, G6 nguồn ứng viên |
| Case/Combo (`dto/cases.rs`) | Case có `faces` (pin), `combos` với `slots` (role, key), `settings.vi/mode`, `brief` (uses, tones, needs). | G5 "đã pin", G6 combo mẫu |

Khoảng trống:
1. **Vai trò hai tầng không khớp.** UI/Case có 7 vai `ComboRoleDto` (display, heading, subheading, body, caption, accent, quote), G3 đòi vai tuỳ ý. Index chỉ biết `Heading/Body/Code`. Cần bảng ánh xạ vai → "hồ sơ vai" (xem 3.1).
2. **Không có điểm tổng, có chủ đích** (test `pair_candidates_mode_filter_and_no_score_field`, thứ tự xếp hạng là lexicographic `rank_key`). G4 nói "chấm điểm hợp vai". Đề xuất: trả **mức phù hợp theo từng tiêu chí** (`fit: good | ok | weak | unknown`) và **không** trả số 0–100. Cần Lead xác nhận.
3. **Chưa có "font vừa xem".** Không có bảng/lệnh nào ghi lịch sử xem. Chỉ có `recent_auto_log` (nhật ký tự kích hoạt, không liên quan).
4. **Chưa có gợi ý combo.** `pair_candidates` là 1 font → n font; chưa có dựng nhiều vai cùng lúc.
5. Chưa có phong cách pair (G4) trong index. Phụ thuộc mô hình 04.

## 2. Nguyên tắc hợp đồng

- Backend trả **mã** (`reason.code` + tham số số/enum). Frontend dịch vi/en, thuật ngữ font có TermTip. Không chuỗi người dùng nhìn thấy trong backend.
- Không điểm tổng. Xếp hạng giữ lexicographic, ổn định, tất định (cùng input → cùng thứ tự).
- Mọi lệnh đọc-chỉ (trừ `record_view`), không đụng file font. Chỉ gợi ý từ thư viện của người dùng (không tải gì).
- Cổng tiếng Việt: `projectVi=true` loại face thiếu glyph tiếng Việt trước khi xếp hạng (như `pair_candidates`).
- Giấy phép: không chặn gợi ý theo license (nguyên tắc "công cụ, không gác cổng"); chỉ trả `licenseKind` để UI hiện NoteMark.
- Hiệu năng: mục tiêu p95 < 200 ms với 2–5 nghìn face trên máy dev. `pair_candidates_gated` đã song song; mỗi lệnh mới phải có bench trong `tests/` (ngân sách ở mục 5), kết quả đo ghi vào báo cáo trước khi merge.

## 3. Lệnh đề xuất (Tauri, camelCase, ts-rs như hiện có)

### 3.1 `roleFit` (G4): độ hợp vai của một face

```
roleFit(face: FaceKeyDto, role: DeskRole, opts: { projectVi: bool, lang }) -> RoleFitDto
DeskRole = ComboRoleDto | { custom: string }   // vai tuỳ ý rơi về hồ sơ gần nhất, xem dưới
RoleFitDto {
  face, role,
  fit: "good" | "ok" | "weak" | "unknown",
  criteria: [ { code: "x_height"|"aperture"|"small_size_legibility"|"vi_glyphs"|"contrast"|"weight_range"|"distinctiveness",
                fit, value: number|null, target: [lo,hi]|null } ],
  reasons: [ ReasonDto ]   // tối đa 3, đã sắp theo mức quan trọng
}
```
- Hồ sơ vai (tiêu chí và ngưỡng) **do 04 quyết định**. Khung theo plan: Body cần x-height, aperture, dễ đọc cỡ nhỏ, đủ glyph vi; Display cho phép tương phản cao và nét đặc trưng. Vai tuỳ ý (`custom`) ánh xạ theo bảng: display/heading/quote/accent → hồ sơ "display-like"; subheading/caption/label/numerals → hồ sơ "text-like"; không nhận ra → `fit: unknown`, `reasons: [ {code: "role_unknown"} ]`.
- Dựa trên `FaceProfile` sẵn có (x-height, aperture, contrast, stem, vi_missing) và `BodyWarning`. Nếu 04 cần số đo mới (ví dụ độ dễ đọc cỡ nhỏ), phải có trong analyzer trước.
- Trục thiếu dữ liệu → `unknown`, không đoán (như `insufficient_data` hiện có).

### 3.2 `suggestPartners` (G4/G5): font hợp đi cùng, theo phong cách

```
PairStyle = "contrast" | "same_family" | "same_period" | "same_structure" | "strong_opposition"   // danh sách cuối do 04 chốt
suggestPartners(anchor: { face, role }, target: { role }, style: PairStyle|"any",
                scope: SuggestScopeDto, opts: { projectVi, limit<=60, offset, lang }) -> SuggestPageDto
SuggestScopeDto = { query?: FaceQueryDto (class, vi, license…), collectionId?, caseId? }
SuggestPageDto { items: [ SuggestItemDto ], total, computedMs }
SuggestItemDto {
  face, row: FaceRowDto,
  gate: "safe"|"controlled"|"experimental",     // từ assess, không có "blocked"
  styleMatch: PairStyle[],                      // phong cách mà cặp thoả
  roleFit: RoleFitDto (tối giản: fit + 3 tiêu chí đầu),
  reasons: [ ReasonDto ]                        // tối đa 3
}
```
- Cài trên `pair_candidates_gated` (cổng + `filter` áp trước top-k, `projectVi`), thêm lọc `styleMatch` **trước** cắt top-k (cùng lý do với gate) và loại `roleFit == weak` cho `target.role`.
- Phân trang `offset/limit` ổn định như `similar_families`.

### 3.3 `deskSuggest` (G5): dữ liệu cho cột 2

Một lệnh, bốn nguồn; UI gọi lười theo tab đang mở:

```
deskSuggest(ctx: { anchor?: {face, role}, caseId?, targetRole, filter?: FaceQueryDto (classes…), limit, offset },
            source: "similar" | "fits_role" | "pinned")  // recent: client-side -> DeskSourcePageDto
DeskSourcePageDto { source, items: [ { face, row, reasons: [ReasonDto], badge?: "pinned"|"in_combo" } ], total }
```
- `similar`: `similar_families` với `useImage=true`; lý do = trục `same` nổi bật (`similar_axis`) và `image_close` khi so bằng embedding. Không có model embed thì dự phòng theo trục (`similar_works_without_model` đã có test).
- `fits_role`: có anchor → `suggestPartners` `style:"any"`; không anchor → quét thư viện theo `roleFit` (cần mức vai tính sẵn, mục 5).
- `pinned`: face của Case (`CaseDto.faces`) kèm `roleFit` cho `targetRole`; face `missing` đánh dấu, không loại.
- Lọc nhanh theo class dùng `FaceQueryDto.classes` đã có.

### 3.4 `suggestCombos` (G6): dựng combo từ thư viện

```
suggestCombos(intent: { name?: string, brief?: BriefDto }, roles: DeskRole[],
              style: PairStyle|"any", opts: { projectVi, caseId?, limit<=6, lang }) -> ComboSuggestionDto[]
ComboSuggestionDto {
  style: PairStyle,
  slots: [ { role, face, row, fit } ],
  reasons: [ ReasonDto ],            // tối đa 4, gồm lý do cả combo
  interpreted: { matchedTerms[], unknownWords[], usedBrief: bool }
}
```
- Hiểu ý định: tên combo → `describe` (từ điển hiện có, không LLM) → `AxisFilterDto` cho vai Display/Heading; vai Body dùng hồ sơ vai mặc định. Tên không nhận ra (ví dụ "Menu nhà hàng") → việc nhận ra ý đồ từ tên chạy ở client (`intents.ts`, spec 01); backend chỉ nhận `styles` + `roles` + `locked`, không đoán ý đồ.
- Dựng: chọn neo Display theo bộ lọc ý định; mỗi neo chạy `suggestPartners` cho các vai còn lại (cắt sớm: ≤ 12 neo × ≤ 5 đối tác), ghép, loại combo trùng face hoặc `Blocked`, xếp lexicographic (gate → số clash → fit → tên), trả ≤ `limit`.
- Nhận cả combo hay từng vai: UI dùng `slots[]`, backend không ghi gì. Ghi vào Case vẫn qua `create_combo`/`add_slot` hiện có.
- Có `caseId`: ưu tiên face đã pin (`pinned_in_case`).

## 4. Dữ liệu mới và mã lý do

### 4.1 Font vừa xem
Client-side (`recentViews` trong ui store), backend **không** lưu và không có `recordView`. Nguồn `recent` của `deskSuggest` bỏ; UI tự ghép. (Đã chỉnh theo spec 01, d381297d.)

### 4.2 `ReasonDto`
```
ReasonDto { code: string, params: { axis?, a?, b?, delta?, count?, role?, style? } }
```
Mã dự kiến (frontend dịch vi/en, thuật ngữ font có TermTip):
`similar_axis`, `image_close`, `x_height_ok`, `x_height_low`, `aperture_open`, `small_size_ok`, `vi_complete`, `vi_missing`, `contrast_high_ok` (Display), `contrast_high_bad` (Body), `serif_vs_sans`, `same_superfamily`, `same_designer`, `same_structure`, `weight_gap`, `pinned_in_case`, `in_combo`, `role_unknown`, `insufficient_data`. Danh sách chính thức chốt cùng 04 khi có mô hình. Mỗi mã có test bảo đảm đủ khoá vi và en.

## 5. Hiệu năng (ngân sách < 200 ms)

- `pair_candidates_gated` là O(N) × `assess`, song song rayon; N = 2–5 nghìn nên khả thi nhưng **chưa đo trên thư viện thật**. Việc đầu tiên khi được duyệt: bench N = 2k/5k/10k face tổng hợp, ghi p50/p95.
- Nếu `fits_role` không anchor vượt ngân sách: tính sẵn `roleFit` mỗi face khi nạp bundle (mảng song song `profiles`, như `facts`), vô hiệu khi index đổi. Mức vai tính sẵn là mức rời rạc, không phải số tổng.
- `suggestCombos` giới hạn cứng neo × đối tác (3.4) và trả `computedMs`; vượt ngân sách thì giảm neo, không thất bại.
- Cache theo (anchor, role, style, scope-hash, indexVersion) trong `AppState`, xoá khi index/Case đổi. Tất định nên cache an toàn.

## 6. Phụ thuộc và câu hỏi

1. **04**: hồ sơ vai (tiêu chí, ngưỡng), danh sách `PairStyle` và điều kiện thoả từng phong cách, số đo analyzer mới nếu có (độ dễ đọc cỡ nhỏ). 3.1/3.2 sẽ theo mô hình của 04.
2. **01**: spec cột 2 (bốn tab, lọc class) và G6; tab ánh xạ vào 3.3, nêu rõ nếu cần thêm nguồn.
3. **03**: G3 lưu vai tuỳ ý. `ComboRoleDto` hiện là enum đóng; chuyển sang chuỗi tự do thì 3.1 và 3.4 dùng `DeskRole`. Cần thống nhất kiểu.
4. **Lead quyết**: (a) không điểm số, chỉ mức `fit` + cổng, có chấp nhận không; (b) [đã giải quyết: Recent ở client, bảng ý đồ ở client].

## 7. Thứ tự làm sau khi được duyệt

1. Bench nền (N = 2k/5k/10k) cho `pair_candidates_gated` và `similar_families`.
2. `ReasonDto` + test khoá vi/en; `roleFit` (3.1) theo mô hình 04.
3. `suggestPartners` (3.2), rồi `deskSuggest` ba nguồn (3.3).
4. `suggestCombos` (3.4): nhận `styles`, `roles`, `locked`, `seed`; tối đa 1 gợi ý mỗi family tiêu đề; ổn định theo seed (xếp hạng tất định, xáo có seed chỉ trong nhóm hạng bằng nhau); `viStatus` trả count + ≤ 8 ký tự.
Mỗi bước: test Rust, clippy, đo thời gian; QA native khi UI nối vào.
