# Hợp đồng cho UI: Giải quyết trùng lặp (0.3.0)

Dành cho 05 (UI). Backend: nhánh `feat/duplicates-resolve` (04). Nghiên cứu và lý do: `docs/team/research/2026-11-11-duplicates.md`.
Quyết định của Lead (2026-11-11): mặc định **Ẩn** (không đụng file); **Thùng rác** là lựa chọn thứ hai, xác nhận từng lần, hoàn tác được, chỉ tệp trong thư mục người dùng; không bao giờ xoá vĩnh viễn; thứ tự nguồn quyết định bản giữ; không làm Windows.

## 1. Bốn lệnh mới (đã có trong `bindings/commands.ts`)

| Lệnh | Tham số | Trả | Ghi chú |
|---|---|---|---|
| `resolve_preview` | `scope: ResolveScopeDto`, `q: FaceQueryDto` | `ResolvePlanDto` | Không đổi gì. `q` = bộ lọc hiện tại của trang (như `duplicate_groups`) |
| `resolve_apply` | `req: ResolveRequestDto` | `ResolveResultDto` | Từng nhóm độc lập; lỗi một nhóm không chặn nhóm khác (`failed`) |
| `resolve_undo` | `ids: number[]` | `ResolveResultDto` | `done` = các quyết định đã hoàn tác |
| `resolve_list` | `includeUndone: boolean` | `ResolutionDto[]` | Tab "Đã giải quyết"; mới nhất trước |

`duplicate_groups` / `duplicate_summary` giữ nguyên; nhóm đã giải quyết tự biến mất khỏi hai lệnh này và khỏi mọi truy vấn thư viện (xem mục 6).

### `ResolveScopeDto` (tag `type`)
- `{ type: 'group', kind, key }`: một nhóm; luôn trả quyết định mặc định, kể cả khi `autoSafe` = false (để người dùng đổi bản giữ).
- `{ type: 'all_safe' }`: chỉ nhóm tự động an toàn; số nhóm còn lại nằm ở `needsReview`.
- `{ type: 'all' }`: mọi nhóm, gồm cả nhóm cần xem (`autoSafe` = false, `review` có giá trị).

### `ResolvePlanDto`
`items: PlanItemDto[]`, `needsReview`, `keepCount`, `hideCount`, `reclaimableBytes` (tổng dung lượng bản thua, để nói "tiết kiệm X MB nếu bỏ vào Thùng rác").

### `PlanItemDto`
`kind`, `key` (định danh nhóm; gửi lại nguyên văn), `keeper: PlanMemberDto`, `losers: PlanMemberDto[]`, `reason`, `autoSafe`, `review`, `warnings`.

### `PlanMemberDto`
`face`, `family`, `style`, `path`, `format`, `version`, `revision`, `modified`, `numGlyphs`, `fileSize`, `viMissing`, `isVariable`, `sourceId`, `sourceName`, `sourceKind`, `active`, `trash: TrashEligibilityDto`.
- `trash.allowed = false` → ẩn nút/tuỳ chọn Thùng rác cho bản này, hiện lý do từ `trash.blocked`: `managed_source` (nguồn hệ thống / Google / Adobe / community), `system_path` (/System, /Library), `outside_source` (không nằm dưới gốc nguồn sau khi canonicalize), `symlink`, `collection_file` (tệp TTC/OTC), `missing_file`.
- `trash.needsExplicit = true` (vd `~/Library/Fonts`) → chỉ cho phép khi người dùng tick RIÊNG tệp đó trong hộp xác nhận, rồi gửi đường dẫn trong `explicitTrashPaths`.

### Gửi `resolve_apply`
```ts
{ items: [{ kind, key, keeperPath, loserPaths, memberPaths }], action: 'hide' | 'trash',
  explicitTrashPaths?: string[], mode?: 'chosen' | 'all_safe' }   // mode mặc định 'chosen'
```
- **`memberPaths` bắt buộc ở `chosen`**: gửi lại nguyên văn `PlanItemDto.memberPaths` của kế hoạch người dùng đang xem. Nhóm hiện tại có tập đường dẫn khác (đã quét lại, có người xoá/thêm tệp) → `plan_stale`, không áp dụng; UI nên lấy lại `resolve_preview` rồi cho người dùng xem lại.
- **`mode: 'all_safe'`** ("Giải quyết tất cả an toàn"): backend TÍNH LẠI quy tắc cho từng nhóm, chỉ áp dụng nhóm `autoSafe` và bỏ qua `keeperPath`/`loserPaths`/`memberPaths` UI gửi (chỉ cần `kind`+`key`). Nhóm đã không còn an toàn → `not_auto_safe`.
- Người dùng bấm "Đổi bản giữ" (`chosen`) thì hoán đổi `keeperPath` với một đường dẫn trong nhóm. Một đường dẫn không thể vừa làm bản giữ vừa làm bản thua (trong cùng request hay với quyết định đã có): `keeper_conflict` / `keeper_is_hidden`.
- Mỗi nhóm giữ ít nhất 1 bản. Bản giữ phải còn trên đĩa, đọc được ngay trên máy (không phải placeholder cloud: `keeper_not_local`) và vẫn là tệp đã quét (`keeper_changed`; với `trash` còn kiểm hash).
- Với `trash`, bản thua phải còn đúng là tệp đã quét (kích thước, mtime và hash; sai → `file_changed`); đường dẫn được canonicalize và phải nằm dưới gốc của nguồn (`outside_source`), không phải symlink (`symlink`), không dưới /System, /Library.
- Luôn hiện hộp xác nhận cho `trash`: liệt kê TỪNG tệp (tên + thư mục rút gọn + dung lượng), tổng dung lượng, câu "Tệp vào Thùng rác của máy, bạn lấy lại được. Font có thể là bản bạn đã mua." Không có "nhớ lựa chọn".

### `ResolveResultDto`
`batchId`, `done: ResolutionDto[]`, `failed: ResolveFailureDto[]`, `needsRescan`.
- **Thành công một phần là bình thường**: mỗi BẢN THUA được xử lý độc lập (`done` + `failed` cùng có thể khác rỗng). Thứ tự cho mỗi bản thua: kiểm tra → ghi sổ `pending` (đã ẩn) → tắt font đang bật → chuyển Thùng rác → `done` → bật bản giữ. Bước nào lỗi thì đảo các bước trước; lỗi lúc đảo được báo riêng (`rollback_failed`, `reactivate_failed`), không bị nuốt. Sự cố giữa chừng (app tắt) để lại dòng `pending` (`ResolutionDto.status`); lần gọi `resolve_apply`/`resolve_undo`/`resolve_list` kế tiếp tự dọn: `hide` thì hoàn tất, `trash` mà tệp còn ở chỗ cũ thì huỷ, tệp đã đi mà không biết đâu thì hoàn tất không có `trashPath` (không hoàn tác tự động được).
- `failed[].code` — **thất bại hẳn** (không đổi gì): `plan_stale`, `not_auto_safe`, `keeper_missing`, `keeper_not_local`, `keeper_changed`, `keeper_not_in_group`, `keeper_is_hidden`, `keeper_conflict`, `loser_not_in_group`, `not_allowed`, `collection_file`, `active_copy_in_use`, `file_changed`, `deactivate_failed`, `trash_failed`, `not_found`, `already_undone`, `path_occupied`, `trash_emptied`, `trash_mismatch`, `newer_resolution_depends`, `db_error`. **Thành công một phần** (quyết định ĐÃ ghi, cần chú ý): `rollback_failed`, `reactivate_failed`, `keeper_activation_failed`, `keeper_deactivate_failed`. App có cả vi và en: backend KHÔNG trả câu, UI dịch theo `code` (key `dup.fail.<code>`) kèm tên font; `detail` chỉ là chẩn đoán thô không dịch (lỗi hệ điều hành/DB, hoặc lý do chặn của `not_allowed`: `managed_source`, `system_path`, `collection_file`, `missing_file`, `needs_explicit`), để trong "Chi tiết". Toàn bộ chuỗi hiển thị (lý do, lỗi, tên loại) dịch ở UI; hiển thị thành công một phần ("Đã xử lý 12 nhóm, 1 nhóm chưa làm được").
- `needsRescan = true` sau hoàn tác một tệp từ Thùng rác: gọi `rescan_sources(null)` để thư viện thấy tệp đã về.
- Sau mọi `resolve_apply`/`resolve_undo` có `done` không rỗng: tăng `libraryRev` (như các thao tác đổi thư viện khác) để tải lại danh sách, lưới và sidebar "Trùng lặp N".

### `ResolutionDto` (một dòng trong "Đã giải quyết")
`id`, `batchId`, `kind`, `groupKey`, `keeper`/`keeperPath`, `loser`/`loserPath`, `action`, `trashPath`, `reason`, `wasActive`, `keeperActivated`, `status` (`pending` | `done`), `createdAt`, `undoneAt`, `linksMoved`, `family`, `style`, `canUndo`.
- **Hoàn tác theo thứ tự ngược**: hoàn tác riêng một quyết định cũ khi còn quyết định mới hơn dùng bản giữ của nó làm bản thua (chuỗi A→B→C) → `newer_resolution_depends`; hãy hoàn tác cả mẻ (`resolve_undo` tự xếp mới nhất trước). Hoàn tác tệp Thùng rác kiểm hash trước khi đưa về (`trash_mismatch` nếu là tệp khác), và khôi phục activation (bật lại bản cũ cùng phạm vi, tắt bản giữ nếu CHÍNH ta đã bật, `keeperActivated`).
- `canUndo = false` khi tệp không còn trong Thùng rác (người dùng đã dọn): hiện "Tệp đã bị dọn khỏi Thùng rác, không khôi phục được" thay vì nút Khôi phục.
- Gom theo `batchId` để toast "Đã ẩn 38 bản" hoàn tác cả mẻ (`resolve_undo(batch.map(r => r.id))`).

## 2. Câu lý do: `ResolveReasonDto { code, args }`

UI dựng câu thường từ `code` + `args` (i18n key `dup.reason.<code>`). Mọi `code`:

**Vì sao giữ bản này (`reason`)**
| code | args | Câu gợi ý |
|---|---|---|
| `preferred_source` | – | "Giữ bản trong thư mục ưu tiên của bạn" |
| `same_content_tidiest_path` | – | "Hai bản giống hệt nhau; giữ bản có tên và đường dẫn gọn hơn" |
| `more_complete_vi` | `missing` | "Bản này đủ chữ tiếng Việt, bản kia thiếu {missing} ký tự" |
| `newer_version` | `keep`, `other` | "Bản mới hơn: {keep} so với {other}" |
| `preferred_format` | `format` | "Giữ bản {format} (OpenType thường đầy đủ tính năng hơn TrueType)" |
| `more_glyphs` | `glyphs` | "Bản này có nhiều ký tự hơn ({glyphs})" |
| `larger_file` | – | "Bản này đầy đủ hơn (dung lượng lớn hơn)" |
| `chosen_by_user` | – | "Bạn đã chọn giữ bản này" |
| `tie` | – | "Hai bản giống nhau về mọi mặt đo được" |

**Vì sao máy không tự quyết (`review`, khi `autoSafe` = false)**
| code | Câu gợi ý |
|---|---|
| `newest_unknown` | "Không biết bản nào mới hơn" |
| `newer_has_fewer_glyphs` | "Bản mới hơn nhưng có ít ký tự hơn" |
| `preferred_source_is_worse` | "Thư mục ưu tiên đang giữ bản cũ hơn / kém đầy đủ hơn" |
| `keeper_less_complete_vi` | "Bản định giữ thiếu chữ tiếng Việt hơn bản kia" |
| `variable_vs_static` | "Một bản là font biến thiên (variable), bản kia là font tĩnh" |
| `different_license` | "Hai bản có giấy phép khác nhau" |
| `collection_file` | "Một bản là tệp nhiều font (TTC/OTC); ẩn nó sẽ ẩn cả các font khác trong tệp" |
| `tie` | "Hai bản giống nhau về mọi mặt đo được" |

**Lưu ý không chặn (`warnings`)**: `may_reflow` → "Văn bản dùng bản cũ có thể xuống dòng khác đi".

Thuật ngữ font (PostScript name, revision, variable/static…) theo quy ước i18n của dự án: thuật ngữ tiếng Anh kèm gloss + tooltip.

## 3. Trình bày (gợi ý từ nghiên cứu §4.6)

- Đầu trang: "Tìm thấy N nhóm font bị trùng. M nhóm xử lý được ngay, K nhóm cần bạn xem." + nút chính "Giải quyết M nhóm an toàn" → `resolve_preview({type:'all_safe'}, q)` → màn xem trước (số bản giữ/ẩn, `reclaimableBytes`, chọn Ẩn / Thùng rác) → `resolve_apply`.
- Thẻ nhóm: hai cột cạnh nhau, cùng một dòng mẫu (gồm "Việt Nam"); bản giữ có nhãn "Nên giữ" + câu từ `reason`; nút Giải quyết / Đổi bản giữ / Bỏ qua. Chi tiết kỹ thuật (đường dẫn đầy đủ, hash, `key`, `revision`, `modified`, mã định dạng, Mở trong Finder) sau "Chi tiết".
- Tên loại: `same_file` "Bản sao giống hệt", `versions` "Bản cũ và bản mới", `formats` "Cùng font, khác định dạng".
- "Bỏ qua nhóm này": `duplicate_ignore` (mục 5); tab "Đã bỏ qua" dùng `duplicate_ignored`.

## 4. Hành vi backend UI nên biết

- **Ẩn** theo VỊ TRÍ TỆP. Hai bản `same_file` chung hash nên chỉ ẩn đường dẫn thua; font vẫn hiện nhờ đường dẫn giữ. Không có thay đổi nào ở tệp.
- **Liên kết đi theo bản giữ**: yêu thích, collection, tag, Case, cặp đã lưu, slot Combo của bản thua được chép/đổi trỏ sang bản giữ; `links_moved` cho biết số lượng. Hoàn tác trả đúng những gì đã đổi, không đụng dữ liệu người dùng đã có.
- **Đang bật**: bản thua khác hash đang bật sẽ được tắt, rồi bật bản giữ cùng phạm vi. `same_file` mà font đang bật chính bằng bản sao bị ẩn: từ chối với `active_copy_in_use` (yêu cầu người dùng tắt font trước).
- **Không thể bật bản đã ẩn** (activation thấy mọi vị trí của nó như đã mất).
- **Tệp nhiều face (TTC/OTC)**: không tự ẩn / không Thùng rác (`collection_file`).
- **Rescan**: quyết định nằm trong sổ bền theo đường dẫn; quét lại không làm bản đã ẩn hiện lại.
- **Thùng rác**: macOS `NSFileManager.trashItem`; hoàn tác đưa tệp về đúng đường dẫn cũ; nếu chỗ cũ đã có tệp khác (`path_occupied`) hoặc Thùng rác đã dọn (`trash_emptied`) thì không làm gì và quyết định vẫn giữ nguyên.

## 5. Bỏ qua nhóm và thứ tự nguồn (lệnh bổ sung)

| Lệnh | Tham số | Trả | Ghi chú |
|---|---|---|---|
| `duplicate_ignore` | `kind`, `key` | `null` | "Bỏ qua nhóm này": nhóm biến mất khỏi `duplicate_groups`, `duplicate_summary` và mọi `resolve_preview`; gọi lại không lỗi |
| `duplicate_unignore` | `kind`, `key` | `null` | Hiện lại nhóm |
| `duplicate_ignored` | – | `IgnoredGroupDto[]` | `{ group: DuplicateGroupDto, ignoredAt }`, mới bỏ qua trước; nhóm đã hết trùng thì không hiện. Dùng cho tab "Đã bỏ qua" có nút "Hiện lại" |
| `source_priority` | – | `number[]` | Id nguồn, ưu tiên giữ bản nhất đứng đầu. Mặc định: nguồn thêm trước |
| `set_source_priority` | `ids: number[]` | `number[]` | `ids` là phần đầu của danh sách; nguồn không nêu xếp sau theo thứ tự cũ. Id lạ hoặc lặp → lỗi, không đổi gì. Trả thứ tự đầy đủ mới. Kéo-thả xong gửi toàn bộ danh sách. Sau khi đổi, gọi lại `resolve_preview` (bản giữ có thể đổi) |

Bỏ qua nhóm theo `(kind, key)`; `key` đổi (vd đổi tên PostScript) thì coi như nhóm mới. Nguồn mới thêm tự xếp cuối.

## 6. Chưa có trong 0.3.0 (đã biết)

- Số `user.db`: migration `USER_DUP_RESOLVE` (cũng thêm cột `sources.priority` và bảng `dup_ignored`) nằm cuối `USER_MIGRATIONS`; dự kiến là v10 khi merge (v8 của auto-activation, v9 của replace-activation). Lead đánh lại theo vị trí trong `USER_MIGRATIONS`.
- Ngưỡng "nhiều glyph hơn rõ rệt" của kiểm tra bản mới ít glyph hơn là 2% (hằng số trong `dup_pick.rs`).
- Chưa kiểm với designer thật thứ tự OTF > TTF và mặc định Ẩn (xem nghiên cứu §4.9).

## 7. Với phần còn lại của app (không cần UI sửa)

`FaceQuery` có cờ nội bộ `include_hidden` (mặc định false) nên mọi truy vấn lưới thư viện và tra cứu face (kể cả face trong Case) tự bỏ qua bản đã ẩn. Chỉ mục embedding (Similar/Pairing) KHÔNG đổi: bản đã ẩn có thể còn nằm trong chỉ mục nhưng kết quả cuối đi qua truy vấn thư viện nên không hiện ra; nếu 05 thấy ngoại lệ, báo 04. Trang "Đã giải quyết" lấy tên qua `resolve_list`, không cần cờ này.
