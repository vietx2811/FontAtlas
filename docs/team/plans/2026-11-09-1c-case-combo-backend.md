# 1c — Case + Combo + Brief starter (backend, worker 02)

Nhánh `feat/case-combo`, xếp trên `feat/advanced-filters`. Tham chiếu: `font-desk/06_v3_pairing_compare.md`, `07_entry_points_research.md`, plan 0.3.0 mục 1c và 2d.

## 1. Schema user.db v6 (migration có backup, test v5→v6 giữ dữ liệu cũ)
```
cases(id PK, name, brief TEXT NULL /*JSON BriefDto*/, settings TEXT NOT NULL DEFAULT '{}' /*{vi, mode}*/,
      view_state TEXT NOT NULL DEFAULT '{}' /*Compare/Pairing: cột, ghim, sắp, nhóm đóng — 06 §8*/,
      created_at, updated_at, opened_at, deleted_at NULL /*thùng rác 30 ngày*/)
case_faces(case_id FK cascade, hash BLOB(32), face_index, origin 'added'|'pinned'|'brief'|'peek', note, added_at,
           PK(case_id,hash,face_index))
combos(id PK, case_id FK cascade, idx, name, is_main, note, created_at, updated_at, UNIQUE(case_id,idx))
combo_slots(combo_id FK cascade, role, ord, hash, face_index, note, style TEXT NOT NULL DEFAULT '{}',
            PK(combo_id,role))
```
Lệch so với đề bài: `combos` tách thành `combos` + `combo_slots` (một Combo có nhiều vai). Role ∈ {display, heading, subheading, body, caption, accent, quote}; giọng ①②③ suy ra ở frontend từ vai. Giới hạn: ≤500 face/Case, ≤50 Combo/Case. Face không còn trong thư viện vẫn giữ (hash+index); DTO báo `missing`.

## 2. Command + DTO (bindings sinh bằng ts-rs)
- `create_case(name?, brief?, settings?) -> CaseDto`, `list_cases(include_deleted?) -> CaseSummaryDto[]` (tên, số face, số combo, opened_at), `get_case(id) -> CaseDto` (faces: FaceRowDto + origin/note, combos + slots), `update_case(id, patch{name,brief,settings,viewState}) -> CaseDto`, `delete_case(id)` (mềm) / `restore_case(id)`; dọn >30 ngày khi mở.
- `add_to_case(case_id: Option<i64>, faces: FaceKeyDto[], origin?) -> AddToCaseDto{caseId, added, alreadyIn}`: `None` = Case đang mở gần nhất, không có thì tạo mới (nút Add to Case ở Top Bar). `remove_from_case(case_id, faces)`.
- `create_combo(case_id, name?, from_combo?) / update_combo(id, patch{name,isMain,note,idx}) / delete_combo(id) / reorder_combos(case_id, ids)`, `set_combo_slot(combo_id, role, face: FaceKeyDto|null, note?, style?) -> ComboDto` (đặt/bỏ font vào vai; tự thêm face vào `case_faces`), `create_combos(case_id, combos[]) -> ComboDto[]` (một transaction; dùng cho starter/"Pair these").
- `case_compare(case_id, combo_ids: i64[]) -> CaseCompareDto`: mỗi cột = Combo (slots + FaceRowDto) kèm hàng "Quan hệ giữa các giọng" lấy từ `assess_pair` (nhãn ●◐◆○, Δ x-height, VI thấp nhất, số font, license) — Compare Combo đọc thẳng. Compare Font vẫn dùng command hiện có (+ `faces_detail(keys[])` batch của 06 §8 nếu 05 cần; hỏi trước khi làm).
- Mọi mã lỗi: `invalid_arg` (role lạ, quá giới hạn), `not_found`.

## 3. Brief starter: `brief_starter(brief: BriefDto, case_id?) -> BriefStarterDto` (thuần đọc, không ghi)
`BriefDto{ uses[]: editorial|branding|poster|ui_web|packaging, tones[]: {pair: classic_contemporary|warm_cool|quiet_bold, value -1..1}, needs{ vi(default true), installedOnly, italic, variable, openLicense }, text? }`.
Ánh xạ (bảng trong `fontatlas-describe::brief`, kiểm thử từng dòng):
- Tone → term của `descriptor-dictionary.json` (dùng `Dictionary::parse` ra `AxisFilter`): classic↔`classic`/`modern`, warm↔`warm`/`cool`, quiet↔`delicate`/`strong`; |value|<0.25 bỏ; ≥0.6 → strength `required`, còn lại `preferred`. Xung đột cùng trục trả về nguyên `conflicts` (không tự chọn).
- Uses → `editorial`→term `editorial`; `ui_web`→`readable_small`+`clean`; `poster`→`display`+`heavy`; `branding`→chỉ class; `packaging`→**không có term** (`unmappable`, nói rõ, không đoán). Ngoài ra gán FaceQuery cho từng vai: vai display `weightClass [600,900]`, vai body `[400,500]`, `varAxisMatch` bật (field 1b).
- Needs → `vi:true` ⇒ `hard: vi_supported` + `langs [{vie, near}]`; `variable`/`italic` ⇒ field trực tiếp; `openLicense` ⇒ `license [ofl, apache]` (chỉ biết 2 lớp này là mở; ghi rõ trong ghi chú); `installedOnly` ⇒ face có `activation` đang bật (nếu rỗng thì bỏ qua và báo).
- Kết quả: `{ query: FaceQueryDto-patch (đưa thẳng cho Library), matched[], conflicts[], unmappable[], notes[], anchors: FaceRowDto[≤3] /*top theo scope_for + sắp tên ổn định*/, combos: PairComboDto[≤3] /*mỗi neo + pair_candidates(body) hạng 1, ưu tiên khác family*/ }`. Thiếu dữ liệu (thư viện rỗng/ít) trả mảng ngắn + note, không lỗi. Chừa trường `ai: null` cho "Kết nối AI".

## 4. Pair from peek (2d): `open_case_from_peek(PeekOpenDto) -> PeekOpenResultDto`
`PeekOpenDto{ face, as: 'heading'|'body', candidate?: FaceKeyDto, case_id?: i64, settings{vi, mode} }`. Một transaction: lấy `case_id` (tham số, rồi Case đang mở, rồi tạo "Case mới"), `add_to_case(face, origin 'peek')`, đảm bảo có Combo 1 (is_main) và `set_combo_slot(role = heading→display… hoặc body)` cho font neo, ghi `settings` (mang theo mode và lựa chọn tiếng Việt). Trả `{ case: CaseDto, comboId, anchorRole, otherRole, trial: candidate|null }`. Candidate **không** ghi vào slot: là trạng thái "thử" của 05 (đứng đầu Cuộn chọn của `otherRole`); chỉ được thêm vào `case_faces` (origin 'peek') để khỏi mất. Nút "Lưu cặp" cũ vẫn dùng `save_pair`; "So sánh" từ peek dùng `compare_faces` hiện có.

## 5. Thứ tự commit và kiểm thử
1. user.db v6 + store (CRUD, giới hạn, cascade, backup + test v5→v6, thao tác trên Case đã xoá mềm).
2. Services + DTO + command + bindings + capabilities_minimal + test IPC, `open_case_from_peek`, `case_compare`.
3. Brief starter: test bảng ánh xạ, xung đột, packaging unmappable, thư viện rỗng, 3 neo/3 combo trên fixture.
Mỗi bước `local_ci` PASS trước khi báo. Không đụng library.db.

## Câu hỏi cho Lead
1. `combos` tách 2 bảng — chấp nhận?
2. Xoá Case mềm 30 ngày (theo spec 04) hay xoá thật?
3. `faces_detail(keys[])` (batch cho Compare) làm trong 1c hay để sau khi 05 đo thực tế?
4. `packaging` không có term trong từ điển: để unmappable, hay muốn thêm term vào từ điển (cần nguồn)?
