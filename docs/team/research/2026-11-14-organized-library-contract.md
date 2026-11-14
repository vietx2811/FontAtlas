# B7 Organized Library — hợp đồng backend cho 05 (onboarding, Cài đặt) và phối hợp với 06

Người viết: 02 · 2026-11-14 · Nhánh `feat/organized-library-050`. Mặc định theo chủ app + 01: **CHÉP**, file gốc không đụng tới; thư viện ở ổ máy/ổ ngoài (chọn ổ cloud: CẢNH BÁO, không chặn); GIỮ tên file gốc; class đổi sau khi xếp thì KHÔNG tự di chuyển, chỉ "Sắp xếp lại" có xem trước + Hoàn tác; gói khôi phục = dữ liệu + embedding + cài đặt (không có model), chụp sau mỗi lần chép. Nút "dọn bản gốc" để sau (nếu có, chỉ chuyển vào Thùng rác).

## Bố cục trên đĩa
```
<gốc>/Library/.fontatlas-library          dấu hiệu (JSON)
<gốc>/Library/<Class>/<Họ>/<tên file gốc> Class ∈ Serif Slab Sans Mono Script Handwriting Display Blackletter Symbol Unclassified
<gốc>/Library/Derived/                    DÀNH RIÊNG cho G7b (bản sao đổi tên VF); resort/chép không đụng
<gốc>/Snapshots/<thời điểm>/              gói khôi phục (manifest.json + library.db + user.db), giữ 5 gói mới nhất
```
`<gốc>/Library` được đăng ký làm MỘT nguồn thư mục (xuất hiện trong danh sách nguồn, tên "Organized Library", watcher B8 theo dõi). Trùng tên file khác nội dung → `Tên-<8 ký tự hash>.ttf`. Bản chép KHÔNG bị tính là "trùng lặp" của bản gốc trong Duplicates.

## Lệnh (bindings/commands.ts đã sinh)
| Lệnh | Việc |
|---|---|
| `organized_check_location(path) → OrganizedLocationCheckDto` | `writable`, `isCloud` + `cloudProvider` (CẢNH BÁO nếu true), `freeBytes`, `nestedSource`, `problem` (`not_a_directory`/`not_writable`/`nested_source`/null). Không để lại gì trên đĩa. |
| `organized_create(path) → OrganizedStatusDto` | tạo cấu trúc + đăng ký nguồn + lưu cài đặt. Lỗi: `nested_source`, `not_writable`, `not_a_directory` (detail.path), `already_configured`. |
| `organized_status() → OrganizedStatusDto` | `configured`, `root`, `libraryDir`, `sourceId`, `files`, `bytes`, `freeBytes`, `isCloud`, `available` (false khi ổ ngoài chưa cắm), `snapshots`, `lastSnapshot`. |
| `organized_plan(scope: {sourceIds?: number[]\|null}) → OrganizePlanDto` | XEM TRƯỚC, không ghi gì: `toCopy`, `bytes`, `alreadyThere`, `cloudNotDownloaded` (placeholder: không chép, bảo tải về máy trước), `freeBytes`, `enoughSpace`, `sample` (50 mục đầu `{src,dest,classFolder}`). |
| `organized_import(scope, onProgress) → OrganizeResultDto` / `organized_cancel() → bool` | chép từng file an toàn (file tạm → đồng bộ → kiểm BLAKE3 = hash đã biết → đổi tên nguyên tử). Tiến độ `{done,total,bytesDone,failed}`. Kết quả `{copied, bytes, alreadyThere, issues[≤200]{path,code}, issueCount, cancelled, snapshot}`; `code` ∈ `cloud_not_downloaded|source_missing|verify_failed|disk_full|io`. Lỗi lệnh: `busy`, `not_configured`, `source_unavailable`, `disk_full` (detail needed/free). Huỷ dừng giữa các file; đã chép giữ nguyên. Một lúc một lượt. |
| `organized_resort_preview() → ResortPlanDto` | `{total, moves[≤200]{face,from,to,classFolder,family}}`; đường dẫn tương đối trong `Library/`. KHÔNG tự chạy. |
| `organized_resort_apply() → ResortResultDto` / `organized_resort_undo(batchId)` | `{batchId, moved, failed}`; Hoàn tác trả đúng các file của lô về chỗ cũ (UI giữ `batchId` để nút Hoàn tác). |
| `organized_snapshot_now() → string` | chụp gói khôi phục ngay (đường dẫn). |
| `pack_inspect(path) → PackInfoDto` | kiểm toàn vẹn gói + thông tin: `createdAt`, `appVersion`, số `files/faces/embeddings/sources`, `sourceRoots`. Lỗi `invalid_pack`. |
| `pack_restore_stage(path, remap[{from,to}]) → PackInfoDto` / `pack_restore_pending()` / `pack_restore_cancel()` | xếp khôi phục chạy ở LẦN KHỞI ĐỘNG SAU (trước khi mở DB); DB hiện tại được DỜI sang `data/pre-restore-*`, không xoá. UI phải bảo người dùng khởi động lại app. `remap` đổi tiền tố đường dẫn cũ → mới (gợi ý lấy từ `sourceRoots`). |

## Luồng UI gợi ý
- **Onboarding:** hỏi "Tạo Organized Library" hay "Tự quản theo thư mục" (giữ cách cũ). Chọn tạo → chọn thư mục → `organized_check_location` (cảnh báo cloud, thiếu chỗ) → `organized_create` → `organized_plan` hiện số font/dung lượng/`enoughSpace` → xác nhận → `organized_import` có thanh tiến độ + Huỷ. Chữ xác nhận NÊN nói: chép chứ không di chuyển, file gốc nguyên vẹn, tên file giữ nguyên, tốn thêm `bytes` dung lượng.
- **Cài đặt ▸ Dữ liệu:** trạng thái thư viện; "Chép thêm font mới" (plan → import); "Sắp xếp lại" (preview → xác nhận → apply → thông báo có Hoàn tác); gói khôi phục gần nhất + "Khôi phục từ gói…" (chọn thư mục gói → `pack_inspect` → remap → stage → nhắc khởi động lại). Ổ ngoài chưa cắm (`available=false`): ẩn nút chép, hiện lý do.
- Sau `organized_import` thư viện đổi qua quét nền (nguồn Organized Library được xếp quét tiếp): UI nghe `ScanEvent` như thường.

## Phối hợp với 06 (kích hoạt)
- Mặc định chép giữ gốc ⇒ font đang bật vẫn trỏ file gốc, KHÔNG gián đoạn. Việc "chuyển font đang bật sang bản trong thư viện" (cần khi sau này gỡ gốc) dùng `ActivationService::relocate(hashes)` của 06: đăng ký đường mới TRƯỚC, gỡ cũ sau, giữ cũ nếu lỗi.
- Quy ước: 02 chỉ cập nhật sổ (`organized_files`: hash → dest) và để file gốc nguyên, rồi gọi `relocate` MỘT lần theo lô cho các hash đang bật. Điểm móc: cuối `services::organized::import` (và `resort_apply`/`resort_undo`, vì chúng đổi đường dẫn). Chờ 06 có hàm; tôi sẽ nối khi có chữ ký.
- `file_locations.path` của bản chép tự có ở lần quét nguồn Organized Library (cùng hash, đường dẫn thứ hai).

## Ghi chú kỹ thuật
- Migration user.db mới `USER_ORGANIZED` ở CUỐI `USER_MIGRATIONS` (bảng `organized_files`, `organized_moves`); Lead xếp thứ tự khi merge cạnh B5.
- Khóa cài đặt `organized_root`. Khôi phục chạy trong `boot::run_boot` TRƯỚC `Store::open`.
- Chưa làm: font cloud chưa tải (bị bỏ qua, báo `cloud_not_downloaded`), nút dọn bản gốc, "chuyển font đang bật" (đợi 06), Windows (ADR đường dẫn).

## Bổ sung: "Tải về rồi chép" cho font còn là placeholder trên ổ cloud (OneDrive/iCloud…)
Thư viện của chủ app nằm trên OneDrive, nên font chưa tải KHÔNG chỉ bị bỏ qua nữa.
- `OrganizeScopeDto` thêm `downloadFirst: boolean` (mặc định `false`). `false` = như cũ: bỏ qua font cloud chưa tải và báo số lượng. `true` = tải về máy rồi chép.
- `OrganizePlanDto` thêm: `needsDownload` (số font phải tải, = `cloudNotDownloaded`), `downloadBytes` (tổng dung lượng), `enoughSpaceForDownload` (ổ chứa thư mục cloud còn đủ chỗ + 5%). `toCopy`/`bytes`/`enoughSpace` đã tính cả font cloud khi `downloadFirst = true`. Xem trước phải nói rõ: "N font (X GB) chưa tải về máy; chọn Tải về rồi chép sẽ tải chúng, file tải về nằm lại trên máy theo cài đặt OneDrive của bạn" và cảnh báo khi `enoughSpaceForDownload = false`.
- Cơ chế: dùng ĐÚNG làn tải của "Tải về máy" 0.3.1 (cổng chung, tối đa 4 lần tải thật đồng thời — mỗi lượt dùng 3 luồng chừa 1 chỗ cho xem trước, mỗi font hạn 120 s, huỷ được). Font đã có trên máy được chép TRƯỚC; font cloud xử lý theo lô 16 (tải lô → chép lô đó → lô kế), nên font đầu tiên vào thư viện sớm và Huỷ dừng ngay giữa các lô. File tải về KHÔNG bị đổi trạng thái OneDrive ("Always keep on this device" không đụng tới; chỉ đọc để hệ điều hành tải).
- `OrganizeProgressDto` thêm `phase` (`downloading` | `copying`), `downloaded`, `downloadTotal`. `done/total` vẫn là số font đã xử lý/tổng cần chép; thanh tiến độ nên hiện "Đang tải n/m" khi `phase = downloading`.
- `OrganizeResultDto` thêm `issueGroups[{code, count, firstPath}]` (lỗi GỘP THEO MÃ, tổng thật, nhiều nhất trước), `downloaded` (số font đã tải về trong lần này), `skippedCloud` (số font cloud bị bỏ qua khi không chọn `downloadFirst`). Mã lỗi tải: `cloud_timeout | cloud_download_failed | cloud_stub | missing_file | cloud_busy`; mã chép như cũ. Huỷ = `cancelled: true`, không tính là lỗi; font đã tải xong vẫn được chép. Font tải lỗi không để lại file chép dở và lần sau thử lại chỉ còn các font đó.

## Bổ sung: bản sao đổi tên trong chi tiết font (G7b)
`FaceDetailDto.derivedFrom: { faceKey, name } | null` — `null` với font thường; với bản sao đổi tên là font gốc ("Copy of «name»").
