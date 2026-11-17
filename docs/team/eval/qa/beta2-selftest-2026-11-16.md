# Tự kiểm 0.5.0-beta.2 (07 QA/DevOps, 2026-11-16/17)

Mã: `8407a0e0` (int/0.5.0-i). Chỉ bản **debug**, `FONTATLAS_DATA_DIR` mới trên VXData, font thử là fixture của repo (OFL) chép vào thư mục thử. Không đụng `~/Library/Fonts`, font hệ thống, thư mục font của chủ máy, OneDrive. Không chụp ảnh vào repo. Kịch bản và cách chạy: `docs/team/eval/qa/beta2-selftest/` (`qa07.js` điều khiển app qua IPC và DOM như `tests/qa/main_flows.js`; `run07.sh` mở app, kiểm đĩa, mở/thoát app thử).

**Cách đo:** gọi lệnh IPC thật của app (cùng lệnh giao diện dùng) và đọc chữ trong hộp thoại thật; kiểm cấu trúc đĩa/checksum bằng shell. Chỗ nào chỉ kiểm ở mức API (không đi qua nút bấm) được ghi rõ.

## Tóm tắt (5 dòng)
1. Organized Library: tạo kho, xem trước, chép, cấu trúc `Class/Họ/file`, nguồn nguyên vẹn, chuyển sang kho + Hoàn tác, xem trước sắp xếp lại, gói khôi phục: **Đạt**.
2. **Lỗi Medium (L1):** nút **Huỷ** khi chép không dừng được cuộc chép ở nguồn local (copy chạy hết rồi mới dừng).
3. **Lỗi High (L2, chưa tái hiện qua nút bấm vì Case cần model ảnh):** "Mở cùng app…" dùng **id của Case** như `collection_id`; backend kiểm bảng `collections` ⇒ Case mới báo "không tìm thấy: collection N"; nếu trùng id với một collection khác thì app bật font của collection đó.
4. App Font Sets (kiểm bằng collection thật + Calculator): mở app ⇒ bật sau ~2 s, thoát ⇒ tắt sau ~33 s, font bật tay / do collection khác giữ nguyên, lý do trong `activation_state` đúng. «Tắt lần này» **Đạt** (chạy lại từ trạng thái sạch). VF «Giữ cả hai» / «Gỡ bản sao này» + Hoàn tác: **Đạt**. Chuỗi tiếng Anh của Organized Library: **Đạt**, không khoá thô.
5. Auto-activation trong Illustrator/InDesign/Affinity: **Chưa thử** (app của chủ máy, bỏ qua theo yêu cầu).

## 1. Organized Library
Dữ liệu: nguồn `small` 12 font (1,7 MB), nguồn `bulk` 100 file ~16 MB (để thử Huỷ).

| # | Bước | Kết quả | Ghi chú / thông báo |
|---|---|---|---|
| 1 | Tạo kho trên VXData: không cảnh báo cloud, có dung lượng trống | **Đạt** | `isCloud=false`, `writable=true`, còn trống ~159 GB; không có cảnh báo cloud. Cảnh báo cloud với ổ cloud thật: **Chưa thử** (không tạo thư mục ghi thử trong OneDrive của chủ máy). |
| 2 | Xem trước chép | **Đạt** | Hộp thoại: «Font được CHÉP, không di chuyển: bản gốc nguyên vẹn, tên file giữ nguyên, tốn thêm khoảng 1.7 MB…», «Sẽ chép 12 · 1.7 MB», «Đã có sẵn (cùng nội dung) 0», «Còn trống ở nơi đến 159.2 GB ✓», nhắc trách nhiệm giấy phép. Số «chưa tải về máy» chỉ hiện khi có (nguồn thử không có placeholder cloud ⇒ dòng đó **Chưa thử**). |
| 3 | Tải về rồi chép: tiến độ, Huỷ | Chép + tiến độ **Đạt**; **Huỷ: Lỗi (L1)**; «tải về máy từ cloud» **Chưa thử** | Chép 12 font 0,2–0,4 s, 14 sự kiện tiến độ, 0 lỗi, tự chụp 1 gói khôi phục. |
| 4 | Cấu trúc `Class/Tên font/`, tên giữ nguyên, nguồn gốc nguyên vẹn | **Đạt** | Dạng `Sans/Roboto/roboto.ttf`, `Serif/DM Serif Display/…`; đủ 12 tên file; mỗi file khớp nguồn từng byte (`cmp`); checksum nguồn không đổi; không file `.part`/tạm. Có thêm file đánh dấu `Library/.fontatlas-library` (dự kiến). |
| 5 | Chuyển font đang bật sang bản trong kho; Hoàn tác | **Đạt** | 2 font đang bật từ nguồn gốc → đường dẫn đổi sang `…/Library/…`; Hoàn tác → về đúng đường cũ. Hộp thoại: «2 font đang bật sẽ chuyển sang bản chép trong Organized Library: đăng ký đường mới trước, gỡ đường cũ sau nên không bị gián đoạn… Có thể Hoàn tác sau khi chuyển.» «Vẫn dùng được trong app thiết kế»: **Chưa thử** (không mở Illustrator/InDesign của chủ máy). |
| 6 | Sắp xếp lại: xem trước rồi Huỷ | **Đạt** | Hộp thoại «Sắp xếp lại thư viện — Mọi font đã nằm đúng thư mục theo class hiện tại.», nút «Huỷ» đóng hộp thoại; số file kho không đổi (113 → 113). Kiểm với danh sách di chuyển khác rỗng: **Chưa thử** (không có font nào lệch class). |
| 7 | Chụp gói khôi phục | **Đạt** | Số gói +1 (1→2); thư mục gói chứa `library.db`, `user.db`, `manifest.json`. |

### L1 — Huỷ khi chép không dừng (Medium)
Repro: tạo kho, thêm nguồn gồm 100 file ~16 MB, bấm chép, sau 4 s gọi `organized_cancel` (nút Huỷ gọi lệnh này). Lệnh trả `true` lúc 17/100 file đã chép nhưng cuộc chép chạy hết 100 file (đến 20,9 s) và kết quả `copied=100`. Một lần thử khác trả `cancelled:true, copied:100` (cờ bật nhưng không dừng).
Nguyên nhân (`UX/src-tauri/src/services/organized.rs`, vòng lặp trong của mỗi lô, ~dòng 530): sau mỗi file `if cancel.load() { cancelled = true; } continue;` — chỉ thoát sau khi cả lô xong, mà mọi font có sẵn trên máy nằm trong một lô duy nhất. Huỷ chỉ có hiệu lực giữa các lô tải cloud.
Gợi ý sửa: thoát vòng lặp trong ngay khi `cancelled` (rồi giữ nguyên phần dọn/ghi sổ sau vòng lặp); thêm test với nguồn nhiều file local.

## 2. App Font Sets
App thử: **Calculator** (không có tài liệu). TextEdit của chủ máy đang chạy nên không dùng (không đụng tới). Font thử: DM Serif Display và Chonburi (các fixture khác trùng PostScript với font đã cài trên máy chủ nên bị «xung đột, không bật»; đó là hành vi đúng).

| # | Việc | Kết quả | Ghi chú |
|---|---|---|---|
| 1 | "Mở cùng app…": danh sách app đúng, không có trình duyệt | **Đạt** (mức API + regex của giao diện) | `installed_apps` 202 mục; bộ lọc của `AppPicker.tsx` loại 12 (Safari, Google Chrome và các app web của Chrome); còn 190. Còn sót 2 tiện ích Safari «Eagle for Safari», «Notion Web Clipper» (không phải trình duyệt; mức **Low**). Bật hộp thoại bằng nút bấm: **Chưa thử** (xem L2). |
| 0 | Lưu app cho một Case mới tạo | **Lỗi (L2)** | `case_set_apps` → «không tìm thấy: collection 1» |
| 2 | Mở app → font của Case tự bật, có thông báo | **Đạt** (collection) | Sự kiện `on` sau ~2 s, `changed=1`; thông báo hiển thị của giao diện: **Chưa thử** (giao diện cần Case ⇒ cần model ảnh). |
| 3 | App đã mở sẵn | **Đạt** | Gắn app sau khi app đã chạy ⇒ font bật trong ≤ 40 s (sự kiện `on`). |
| 4 | Thoát app → ~30 s sau font tắt; «Bật lại» | **Đạt** | `off` sau **32,9 s**; bật lại Case/collection bật được. Nút «Bật lại» trên thông báo: **Chưa thử** (giao diện). |
| 5 | Font bật tay / do Case khác bật không bị tắt theo | **Đạt** | Font bật tay + thuộc collection khác vẫn bật sau khi app thoát. |
| 6 | «Tắt lần này», «Cài đặt…» | «Tắt lần này»: **Đạt** (gọi `case_app_suppress` ở mức API). Chạy từ trạng thái sạch (`activation_deactivate_all`): mở app ⇒ font bật, «Tắt lần này» ⇒ 0 font còn bật; thoát rồi mở lại app ⇒ font bật lại (sự kiện `on`, `changed=2`). Lần chạy đầu font còn bật chỉ vì còn lý do khác (đã giải thích). Nút bấm «Tắt lần này» trên thông báo và «Cài đặt…»: **Chưa thử** (cần Case ⇒ model ảnh). | |

### L2 — Case ↔ collection (High, chưa tái hiện qua giao diện)
- Giao diện: `Sidebar.tsx` (`openWithApp`) và `PairingDesk` mở bộ chọn với **id của Case**; `AppPicker` gọi `case_apps/case_set_apps(collectionId = id Case)`; thông báo `appSetNotice` (`activationSync.ts`) gọi `getCase(ev.collectionId)` và `activationActivateCase(ev.collectionId)`.
- Backend: `case_set_apps` (`fontatlas-store/src/case_apps.rs:42`) kiểm `u.collections`; `create_case` chỉ ghi `u.cases`. Dữ liệu mới: Case 1 tồn tại, `collections` rỗng ⇒ lưu app thất bại «không tìm thấy: collection 1».
- Nếu có collection trùng id: lưu thành công và khi app mở, backend bật **font của collection**, không phải của Case (thấy ở `reasons: case_app` gắn collection). Người dùng có collection id 1 sẽ thấy font lạ bật khi mở app của Case 1.
- Repro API: `create_case` → `case_set_apps({collectionId: caseId, apps:[…]})`. Qua giao diện: cần model ảnh (app báo «Chưa tải model hình ảnh» khi tạo/mở Case trên dữ liệu mới, không có model), nên **chưa bấm tay được**; chạy lại khi có model.

## 3. Font VF trùng tên với bản static
| # | Việc | Kết quả | Ghi chú |
|---|---|---|---|
| 1 | «Giữ cả hai» tạo bản sao «<tên> VF», bật cùng bản static | **Đạt** (mức API) | Static Roboto đang bật; «Giữ cả hai» cho VF cùng PostScript ⇒ tạo bản sao «Roboto FAQA VF Regular» trong dữ liệu app (`derived/roboto_vf-VF.ttf`), `warning: renamed_copy`, bản sao bật, static vẫn bật, VF gốc không bật. Preflight báo 1 cần xác nhận trước đó (đúng). |
| 2 | Thấy cả hai trong app thiết kế | **Chưa thử** | Không mở app của chủ máy. |
| 3 | «Gỡ bản sao này» → Thùng rác, Hoàn tác | **Đạt** (mức API) | `remove_derived_copy`: `trashed=true`, bản sao hết bật; Hoàn tác: `restore_derived_copy` đưa file về đúng đường cũ trong `derived/`. Nút bấm: **Chưa thử**. |

Lưu ý khi thử: bản đầu tiên của bước 1 ra «Lỗi» giả vì static `roboto.ttf` xung đột với Roboto của thư viện RightFont của chủ máy (`needs_confirmation`, `owner: right_font`), nên static không bật được. Đây là nhiễu môi trường chứ không phải lỗi app. Sau khi đổi tên bản sao fixture trong thư mục thử bằng `tests/qa/patch_font_names.py` (hậu tố « FAQA», VF và static vẫn cùng PostScript), bước đạt.

## 4. Màn "Đang bật"
Qua `activation_state` (lúc app thử đang chạy): font bật tay + thuộc collection khác + thuộc app của collection ⇒ `reasons = [manual, collection, case_app]` với tên «QA07 Set B» và «QA07 Set A · Calculator»; font chỉ do app ⇒ `[case_app]`. **Lý do đúng.** Màn hình **Đang bật** liệt kê đúng 2 font («DM Serif Display Regular», «Chonburi Regular») và số «2», nhưng trong danh sách **không thấy nhãn lý do** (cần mở chi tiết); nên ghi nhận: người dùng khó biết vì sao font đang bật nếu không mở panel. Lý do «tự bật (auto)»: **Chưa thử** (cần app thiết kế).

## 5. Auto-activation (Illustrator / InDesign / Affinity)
| App | Kết quả |
|---|---|
| Illustrator | **Chưa thử** (app của chủ máy) |
| InDesign | **Chưa thử** |
| Affinity | **Chưa thử** |

## 6. Chung (chữ khó hiểu, thiếu vi/en, chậm)
- Định dạng số không nhất quán: «159,3 GB» (dấu phẩy) ở một lần chạy và «159.2 GB» (dấu chấm) ở lần khác, cùng màn Organized Library, cùng ngôn ngữ `vi` (Low; nghi chuỗi định dạng phụ thuộc thời điểm đặt locale).
- Màn Organized Library hiển thị mục «Library · 0 font · Sẵn sàng» như một nguồn bình thường, cạnh nguồn `small` (có thể gây nhầm: người dùng tưởng là nguồn riêng).
- Chuỗi tiếng Anh/khoá thô (`QA_LANG` en, kịch bản `i18n`): **Đạt**. Panel và hộp thoại Organized Library đọc trọn tiếng Anh («ORGANIZED LIBRARY», «Choose where to keep the library…», «Restore from a pack…», «Re-sort»…); 0 khoá i18n thô trong chữ hiển thị. Các màn khác (App Font Sets, Pairing Desk) bằng tiếng Anh: **Chưa thử** (cần Case ⇒ model ảnh). Trên màn tiếng Việt đã chụp chữ của Organized Library và hộp thoại: **không thấy khoá thô**.
- Chậm: bản debug chép 12 font 0,2–0,4 s; quét nguồn 100 file × 16 MB (1,5 GB) mất vài chục giây trước khi liệt kê; chép 1,5 GB ~21 s. Không thấy chỗ chậm bất thường khác.

## Danh sách lỗi
| Mã | Mức | Mô tả | Cách tái hiện |
|---|---|---|---|
| L2 | **High** | App Font Sets: id Case dùng như collection id (lưu app thất bại hoặc bật nhầm font của collection trùng id) | `create_case` rồi `case_set_apps({collectionId: caseId})` ⇒ «không tìm thấy: collection N»; hoặc có collection cùng id ⇒ lưu được, mở app thì bật font của collection |
| L1 | Medium | «Huỷ» khi chép Organized Library không dừng được với nguồn local | Chép nguồn ≥ 100 file, huỷ giữa chừng ⇒ vẫn chép đủ |
| L3 | Low | Danh sách app còn tiện ích Safari «Eagle for Safari», «Notion Web Clipper» | `installed_apps` sau bộ lọc trình duyệt |
| L4 | Low | Định dạng số dấu phẩy/dấu chấm không nhất quán | Mở Cài đặt ▸ Thư viện nhiều lần |
| L5 | Low | Màn «Đang bật» không hiện lý do bật ngoài chi tiết | Bật font qua nhiều đường rồi mở «Đang bật» |

## Ghi chú về quy trình (07)
Lần đầu instance debug của tôi chạy trùng giờ CI native QA của Lead; Lead yêu cầu dừng, tôi đã dừng và quét CoreText. Các phần còn lại chạy sau khi Lead báo «QA window free»: không có khoá QA nào bị giữ, không app thử nào của chủ máy đang chạy (Calculator), mỗi lượt giữ khoá `/tmp/fontatlas-native-qa.lock` và nhả khi xong, dữ liệu mỗi lượt ở thư mục mới trên VXData.

Còn lại cho lần chạy sau, khi có model ảnh: bấm tay «Mở cùng app…» để tái hiện L2 qua giao diện, thông báo «Bật lại» / «Tắt lần này», chuỗi tiếng Anh của Pairing Desk và App Font Sets, cảnh báo cloud với ổ cloud thật.
