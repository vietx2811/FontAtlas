# Backend cho Library view options (R2, R3, R6, R7, activate theo Dự án)

Nhánh `feat/library-backend-r2r3` (rebase lên dev/mvp 3a3fbf9d; library.db vẫn ở v4 trên dev nên R2 = **v5**). Tiến độ: **R2, R3, R6, R7, activate theo Dự án: xong** (ghi chú triển khai ở cuối).
Spec: `docs/design/library-view-options/spec.md` (c2ce50e) mục B5, C6, R1–R7.

## Phạm vi lát này

| # | Việc | Spec |
|---|---|---|
| 1 | R2: `releaseYear`, `releaseYearSource`, `fileCreatedAt`, sort `released` / `file_created` | B5 |
| 2 | R3: lệnh `family_groups(query)` | B5 |
| 3 | R6: `rename_tag`, `delete_tag` (`tag.group` để sau nếu 05 chưa cần) | C6, D3 |
| 4 | R7: `removeSource` làm gì với dự án/thẻ/ghi đè/yêu thích + doc + test | C6 |
| 5 | Activate theo Dự án ("Bật tất cả") qua flow xung đột hiện có | C6 |
| sau | R1 (`badgeLangs`), R4 (trường tuỳ chọn), R5 (`trait_facets`, `traitsAll`) | không thuộc đợt này |

## 1. R2: năm phát hành ước lượng + ngày tạo file

**Nguồn có sẵn:** `fontatlas_core::extras::Years { created, modified, copyright }` (u16), có từ analyzer core-0.8.0, nằm trong JSON phân tích đã lưu.

**Quy tắc** (`fontatlas_core::extras::release_year(&Years) -> Option<(u16, ReleaseYearSource)>`, hàm thuần):
1. `copyright` nếu hợp lệ; 2. `created`; 3. `modified`. "Hợp lệ" = trong [1900, năm hiện tại + 1]; 0, 1904 (epoch Mac), và tương lai xa → bỏ, xuống nguồn kế. Nguồn: `copyright | head_created | head_modified`. Không có nguồn nào → `null`.
(Ghi chú: `created` = 1904 là giá trị rác phổ biến; ngưỡng 1900 vẫn cho qua 1904 nên loại riêng 1904 và mọi năm < 1980 cho `created`/`modified` vì chữ số hoá không thể sớm hơn; `copyright` được phép < 1980 vì font số hoá lại từ kiểu cũ.)

**Lưu để sắp xếp ở SQL** (store sắp xếp, không phải client): `library.db` migration mới (**v5**: v4 đã bị feat/cloud-fonts dùng cho `cloud_only`, đã ở dev/mvp): `faces.release_year INTEGER NULL`, `faces.release_year_src TEXT NULL`; backfill từ bản phân tích mới nhất có `extras` (không đọc file); face chưa có `extras` → NULL cho tới lần phân tích lại (analyzer < core-0.8.0). Ingest điền cột khi ghi phân tích. Migration có backup + test như v2/v3.

**Ngày tạo file:** `file_locations.created_ns INTEGER NULL` (birth time: `Metadata::created()` trên macOS/Windows, NULL nếu không đọc được); điền khi quét (stat, KHÔNG mở file nên an toàn với file dataless). Face có nhiều vị trí: `fileCreatedAt = MIN(created_ns)` trên các vị trí chưa missing (ngày sớm nhất file xuất hiện). Vị trí cũ (chưa có cột): NULL đến lần quét sau (mtime không đổi ⇒ quét không băm lại, nên cần một bước cập nhật stat nhẹ cho vị trí `created_ns IS NULL`).

**Nguồn cloud (Lead quyết 2026-11-11):** birth time của vị trí trên nguồn cloud vẫn lưu và sắp xếp được, kèm `fileCreatedSource: 'disk' | 'cloud_sync' | null` (`cloud_sync` = có thể là lúc đồng bộ, UI hiện NoteMark); không ẩn.

**DTO:** `FaceRow.releaseYear`, `releaseYearSource` (`copyright|head_created|head_modified`), `fileCreatedAt` (ms epoch), `fileCreatedSource` (`disk|cloud_sync`); sort `released` / `file_created` (SortKeyDto snake_case). `ROW_SCHEMA` trong `types.ts` + mock + bindings cùng lúc.

**Sort:** `SortKey::Released`, `SortKey::FileCreated`; `descending` giữ nghĩa; **NULL luôn cuối bất kể chiều**, rồi cùng bậc sắp theo Family (đúng spec). Test: thứ tự cả hai chiều, NULL cuối, tie-break Family→Weight→italic.

**Test đã định (viết trước khi code):**
- `release_year` thuần: copyright thắng created; created = 0/1904/tương lai → bỏ; chỉ modified → `head_modified`; không có gì → None.
- Migration: library.db v4 → v5 giữ dữ liệu, backfill đúng từ JSON mẫu, face không có `extras` → NULL.
- Query: `sort=released` với tập face tổng hợp có NULL xen kẽ → NULL cuối cả hai chiều.
- Quét: `created_ns` điền từ stat; file dataless vẫn có giá trị mà không bị mở (probe).

## 2. R3: `family_groups(query)`

`family_groups(query: FaceQuery) -> { family: string, start: number, count: number, matchCount: number }[]`.

- Cùng bộ lọc/scope với `query_faces` (dùng chung `scope_of`/`to_store_query`), nhưng **sắp theo khoá nhóm**, rồi trong nhóm Weight tăng, upright trước italic (đúng thứ tự `query_faces` khi `groupByFamily = true`: thêm `FaceQuery.groupByFamily: bool`; khi bật, ORDER BY = khoá nhóm → Weight → italic → tên).
- Khoá nhóm theo `sort`: `family` → tên family (A→Z / Z→A); `released`/`added`/`file_created` → giá trị cực trị của family theo chiều (mới nhất khi Mới→Cũ, cũ nhất khi Cũ→Mới; NULL family cuối); `weight` → nhẹ nhất/nặng nhất; `match` → điểm cao nhất của family. Một family **không bao giờ bị xé** thành hai nhóm.
- `start` = vị trí face đầu của nhóm trong thứ tự `query_faces` (để `pagedFaces` theo offset khớp); `count` = số face của family trong truy vấn; `matchCount` = số khớp bộ lọc; `styleTotal` = tổng kiểu của family KHÔNG lọc (chip "4/9 kiểu" = matchCount/styleTotal; Lead đã duyệt). `count` = số face trong truy vấn (= matchCount, giữ cho bố cục).
- Khoá nhóm theo family: `family_key` (đã có, `vi_fold(family_name)`) để "Café"/"Cafe" không tách; hiển thị `family_name`.
- Hiệu năng: một truy vấn GROUP BY trên cùng WHERE, không tải face; mục tiêu < 50 ms cho 2.300+ face (test perf như `class_histogram`).
- Test: tổng `count` = `total` của `query_faces`; `start` tăng dần liên tục (mỗi nhóm bắt đầu đúng nơi nhóm trước kết thúc); face đầu tại `start` thuộc đúng family (đối chiếu `query_faces`); mọi `sort` × `descending`; family không xé; bộ lọc làm `matchCount < count`.

## 3. R6: đổi tên / xoá Thẻ

- `rename_tag(id, name) -> TagDto`: trim, rỗng → `invalid_arg`; trùng tên (không phân biệt hoa/thường/dấu theo `name_key`) với thẻ KHÁC → `conflict` (không gộp ngầm); đổi tên chính nó (chỉ hoa/thường) hợp lệ. **Tiền tố dành riêng** (`style:`/`mood:`/`use:`): đổi tên thẻ HỆ THỐNG sang tên không thuộc danh mục, hoặc đổi thẻ thường thành tên tiền tố ngoài danh mục → `invalid_arg` (cùng luật `create_tag`).
- `delete_tag(id)`: xoá `tags` + `face_tags` (CASCADE); không đụng font/dự án. Trả số face đã bỏ thẻ để UI nêu trong xác nhận. Thẻ hệ thống xoá được (chỉ là lựa chọn của người dùng, tạo lại được).
- Cả hai bump cache thẻ nếu có; test: đổi tên giữ `face_tags`; trùng tên → conflict; xoá cascade đúng; id không tồn tại → `not_found`.

## 4. R7: `removeSource` làm gì với dữ liệu người dùng

Hiện (đọc `Store::remove_source`, `rescan.rs`): xoá `file_locations` của nguồn + bản ghi `u.sources`, rồi `gc()` dọn rác. Quyết định (để doc + test chốt hành vi):

| Dữ liệu | Khi gỡ nguồn |
|---|---|
| Face còn ở nguồn khác | Giữ nguyên (chỉ bớt một vị trí) |
| Face không còn vị trí nào | `faces`/`analyses` bị `gc()` dọn; |
| Dự án (Case/Combo), Collection, Pair, Tag, Favorite, ghi đè phân loại | **GIỮ** (khoá theo `(hash, face_index)` ở user.db, không phụ thuộc nguồn): font hiện `missing` (Case) hoặc ẩn khỏi lưới nhưng còn trong collection/tag; thêm lại nguồn ⇒ khớp lại theo hash |
| Font đang bật từ nguồn đó | Cảnh báo ở xác nhận; không tự tắt (người dùng tắt qua "Đang bật") |

Việc làm: (a) test xác nhận từng dòng bảng trên (nhất là: gỡ nguồn rồi thêm lại ⇒ Case/tag/ghi đè còn nguyên); (b) `remove_source` trả `RemoveSourceReportDto { removedLocations, facesLeftWithoutLocation, casesAffected, collectionsAffected, tagsAffected, activeFonts }` để câu chữ xác nhận dùng số thật; (c) doc cho 05 (câu chữ gợi ý). Nếu hiện `gc()` xoá dữ liệu người dùng, đó là lỗi cần sửa trong lát này.

## 5. Activate theo Dự án ("Bật tất cả")

`activate_case(case_id, scope, proceed/force) -> ActivationReport` = lấy `faces` của Case (chỉ phần không `missing`), gọi flow `activate_faces` hiện có (xung đột tên family/PostScript, file dataless được tải trước qua API cloud của 03, báo cáo theo face) — **không** viết nhánh xung đột riêng. Face `missing` được liệt kê trong báo cáo (`skipped: missing`). Test: Case với 3 face (1 missing, 1 xung đột) ⇒ báo cáo đúng; `proceed=false` dừng ở xung đột như `activate_faces`.

## Thứ tự làm đề xuất

1 (R2: core hàm thuần → migration → ingest/scan → DTO/sort → bindings/mock) → 2 (R3) → 3 (R6) → 4 (R7) → 5. `local_ci` sau mỗi cặp 1–2 và 3–5 (khoá dùng chung); bindings regen từ `UX/src-tauri`.

## Quyết định (Lead, 2026-11-11)

1. Birth time cloud: lưu + sắp xếp, cờ `fileCreatedSource = 'cloud_sync'`. 2. `family_groups` trả thêm `styleTotal`.

## Ghi chú triển khai R2 (đã làm)

- `fontatlas_core::extras::release_year(&Years, now_year)`; head.created/modified hợp lệ từ 1985, copyright từ 1900, tối đa `now+1`.
- library.db **v5**: `faces.release_year`, `faces.release_year_src` (`copyright|head_created|head_modified|none`; NULL = chưa tính), `file_locations.created_ns`, `created_src`. Backfill Rust ở `Store::open` từ bản phân tích local mới nhất có `extras` (không đọc file); bản không có `extras` (analyzer < core-0.8.0) giữ NULL tới lần phân tích lại. Ingest ghi cùng lúc phân tích.
- `created_ns` lấy từ `Metadata::created()` khi quét (chỉ stat, an toàn với file dataless); vị trí không đổi được cập nhật ở mỗi lần quét. `created_src = cloud_sync` khi đường dẫn nằm dưới ổ cloud (`cloud_provider_of`).
- Face nhiều vị trí: `fileCreatedAt` = MIN các vị trí chưa missing; `fileCreatedSource` lấy của vị trí sớm nhất.
- NULL luôn cuối bất kể chiều; cùng bậc → Family → Weight → nghiêng → face_id.

## Ghi chú triển khai R3 / R6 / R7 / activate (đã làm)

- **R3**: `FaceQuery.groupByFamily` (DTO `groupByFamily`) + `Store::family_groups(q, scope)` / lệnh `family_groups(q)` → `FamilyGroupDto { family, start, count, matchCount, styleTotal }`. Khoá nhóm = cực trị của family theo `sort`/chiều (hàm cửa sổ cho từng hàng, hàm gộp cho GROUP BY — cùng một biểu thức nên `start` luôn khớp `query_faces`); family sort → tên; weight/added/released/file_created → nhỏ nhất khi tăng, lớn nhất khi giảm, NULL cuối; match có scope → điểm cao nhất; Auto/Match không scope → tên (khớp đầu từ lên trước). Trong nhóm: Weight → thẳng trước nghiêng → tên → face_id. `family_key` (gập dấu) nên "Café"/"Cafe" không tách. `styleTotal` = số face của family còn ít nhất một vị trí hiện diện, chưa bị ẩn bởi Trùng lặp, không áp bộ lọc. 2.300 face: chạy dưới 50 ms (test).
- **R6**: `rename_tag(id, name) -> TagDto`, `delete_tag(id) -> number`. Mã lỗi mới `conflict` (`StoreError::Conflict`; thêm vào `CmdErrorCode`). Luật tiền tố dành riêng như `create_tag`; Tag hệ thống không đổi thành tên thường. Xoá Tag hệ thống được.
- **R7**: kiểm tra thực tế: `gc()` không đụng user.db và dữ liệu người dùng khoá theo `(hash, face_index)` — **không có lỗi**, test chốt hành vi (gỡ rồi thêm lại ⇒ Dự án hết `missing`, Tag/Collection/Yêu thích/ghi đè còn). `remove_source` nay trả `RemoveSourceReportDto { removedLocations, facesLeftWithoutLocation, casesAffected, collectionsAffected, tagsAffected, activeFonts }`; thêm `remove_source_preview(sourceId)` (chỉ đọc, cùng số liệu) vì câu xác nhận cần số TRƯỚC khi gỡ.
- **Activate theo Dự án**: `activation_activate_case(caseId, scope?, proceed, onProgress) -> CaseActivationDto { report, skippedMissing }`. Font không `missing` đi qua `activate` (cùng luồng xung đột/cloud); font `missing` không gửi xuống engine, liệt kê ở `skippedMissing`, không tính `failed`. Kích hoạt thủ công (reason `Manual`), tắt Dự án không tự tắt font.

## Sửa theo review (Lead, @052a97fe)

1. **Quét lại file không đổi** nay ghi `created_ns/created_src` (COALESCE, không ghi đè bằng NULL) ở cả nhánh unchanged lẫn nhánh placeholder (chỉ `stat`, không đọc nội dung). Test: xoá `created_ns` rồi quét lại ⇒ có giá trị.
2. **Backfill `release_year`**: chạy MỘT lần (cờ `meta.release_backfill`), theo lô 2.000 face (`face_id` tăng dần), mỗi lô một transaction, đọc `json_extract(metadata_json,'$.extras.years')` nên không nạp JSON vào RAM. Đo 100k face (release): backfill + mở 316 ms, mở lại (có cờ) 0,8 ms. Face chưa có `extras` giữ NULL tới lần phân tích lại (ingest ghi cột). Chưa chạy nền: 316 ms/100k chấp nhận được; chuyển nền nếu lớn hơn.
3. **Sort `file_created`** dùng JOIN GROUP BY một lần (`fc.m`) với CÙNG điều kiện ẩn Trùng lặp/`include_hidden` như cột `fileCreatedAt` hiển thị. Đo 100k (release, sort giảm dần): `released` 73 ms (ungrouped) / 130 ms + groups 96 ms (grouped); `file_created` ~400 ms / 320 ms + groups 140–160 ms. Không denormalize `faces.file_created_min` (phải giữ đồng bộ ở ingest/rescan/missing/resolve): với thư viện thật vài nghìn tới vài chục nghìn face là < 100 ms; nếu cần ở 100k thì làm thành cột có trigger — để lại cho lát sau.
4. **`copyright_year`**: mốc `©`/`Copyright`/`(c)` ĐẦU TIÊN trong chuỗi, năm đầu sau mốc (≤ 24 byte); năm phải đứng riêng (không dính chữ/chữ số: loại `font2012.com`), nằm trong [1900, năm hiện tại + 1], ngoài khoảng thì thử ứng viên kế (năm kế, mốc kế); không có mốc ⇒ năm hợp lệ đầu tiên (không còn `.min()`). Không bump analyzer: bản phân tích đã lưu giữ giá trị cũ tới lần phân tích lại.
5. `rename_tag`: NFC → trim → lưu; "Phở" vs "Pho" xung đột là chủ ý (cùng khoá gập dấu với `create_tag`).
6. `rename_tag` store: kiểm tồn tại trước ⇒ id lạ luôn `not_found`, không báo `conflict`.
7. `removal_report`: "rời lưới" = không còn vị trí nào ở nguồn khác mà `missing = 0` và chưa bị ẩn bởi Trùng lặp.
8. Test `family_groups` với tagIds, class + includeSecondary, confidentOnly, sort match + faceKeys (cả hai chiều); test mapping lệnh TS (`family_groups`, `rename_tag`, `delete_tag`, `remove_source(_preview)`, `activation_activate_case`, lỗi `conflict`). Benchmark: `cargo test -p fontatlas-store --release --test bench_100k -- --ignored --nocapture`.
