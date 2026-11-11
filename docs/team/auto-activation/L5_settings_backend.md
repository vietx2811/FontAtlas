# L5 — Backend cho Settings → Tự bật font (0.3.0)

Nhánh `feat/auto-activation`. Dịch vụ `services/auto.rs`, DTO `dto/auto.rs` (bindings `UX/bindings/Auto*.ts`, `OtherManagerDto.ts`), lệnh Tauri trong `commands.rs`. Frontend chưa làm (thuộc worker UI); phần dưới là HỢP ĐỒNG cho UI.

## Hợp đồng cho UI

### Lệnh (đều `async`, lỗi là `CmdError`)

| Lệnh | Đối số | Trả | Ghi chú |
|---|---|---|---|
| `auto_status` | – | `AutoStatusDto` | Gọi khi mở trang Settings và sau mỗi lệnh dưới; không có sự kiện đẩy (poll 5–10 s nếu trang đang mở để thấy app chạy/tắt). Chỉ ĐỌC máy. |
| `auto_set_enabled` | `on: bool` | `AutoStatusDto` | Công tắc chung. Tắt: broker trả 503, nguồn font request trả rỗng, panel/script nằm im; không gỡ gì. Có hiệu lực ngay. |
| `auto_install` | `target: "illustrator" \| "indesign"` | `AutoStatusDto` | Illustrator: ký + chép panel CEP. `indesign`: chép script vào MỌI phiên bản InDesign và InCopy đã chạy ít nhất một lần. Lỗi → `CmdError{code:"io"}`. |
| `auto_remove` | `target` | `AutoStatusDto` | Gỡ sạch phần của Font Atlas; không đụng tệp/thư mục lạ. Không có gì để gỡ vẫn thành công. |
| `auto_forget_key` | – | `AutoStatusDto` | Xoá khoá ký panel Illustrator của máy này (`signingKeyPresent` → false). Từ chối nếu panel còn cài (gỡ trước). |
| `auto_log` | `limit?: u32` (mặc định 50, 1–500) | `AutoLogDto[]` mới nhất trước | Nhật ký "app nào hỏi font gì, kết quả". |

Photoshop, After Effects, Affinity không có nút cài: chạy theo công tắc (nguồn font request nằm trong app).

### `AutoStatusDto`

`{ enabled, brokerRunning, fontRequestRunning, signingKeyPresent, isolatedBuild, apps: AutoAppDto[6], otherManagers: OtherManagerDto[] }`

`brokerRunning` là kết quả PING thật (không chỉ cổng còn mở). `signingKeyPresent`: Settings phải nói rõ "Font Atlas giữ một khoá ký riêng của máy này ở thư mục dữ liệu để cài lại panel Illustrator" và cho nút **Xoá khoá** (`auto_forget_key`) khi panel đã gỡ.

`apps` luôn đủ 6 mục, thứ tự cố định: `photoshop, aftereffects, illustrator, indesign, incopy, affinity`. UI ẩn/làm mờ mục có `appInstalled=false`.

`AutoAppDto`: `id, label, method (font_request | cep_panel | startup_script), state, evidence (measured | partial | untested), appInstalled, appRunning, installable, installed, restartNeeded, detail?, lastActivityMs?`

`state` và cách UI nên hiển thị:

| `state` | Nghĩa | UI |
|---|---|---|
| `off` | Công tắc chung tắt | "Tắt" (xám); nút Cài/Gỡ vẫn dùng được |
| `ready` | Hoạt động | "Sẵn sàng" + "lần cuối <lastActivityMs>" nếu có |
| `needs_install` | Chưa cài (Illustrator; InDesign/InCopy còn phiên bản chưa cài) | Nút **Cài** |
| `stale` | Đã cài nhưng cũ (khác phiên bản app, `broker.json` chuyển chỗ, bị sửa) | "Cần cài lại" + nút **Cài lại**; `detail` là lý do |
| `foreign` | Có tệp/thư mục cùng tên không phải của Font Atlas | Cảnh báo, KHÔNG cho cài; nói rõ không đè |
| `waiting_first_launch` | InDesign/InCopy chưa chạy lần nào (chưa có thư mục locale) | "Mở <app> một lần rồi quay lại" |
| `error` | Hỏng (xem `detail`: nguồn font request không mở được, broker không chạy, không thấy thư mục Adobe) | Cảnh báo đỏ + `detail` |
| `isolated_build` | Bản chạy với thư mục dữ liệu riêng (QA/test): nguồn font request cố ý không mở | Chỉ thấy ở bản dev; ghi chú nhỏ |

- `restartNeeded=true`: panel/script chỉ nạp lúc app khởi động; app đang chạy thì nói "khởi động lại <app> để dùng".
- `evidence`: `measured` = đã đo với app thật; `partial` = cơ chế đã thử một phần; `untested` = chưa thử với app thật. UI nên hiện nhãn trung thực (vd "Chưa kiểm với After Effects") thay vì hứa chắc.
- `installable=false` (font request): không hiện nút.

`OtherManagerDto`: `{ name, installed, running }` cho RightFont, FontBase, Typeface, Suitcase Fusion, FontExplorer X. Khi có cái `running=true`: hiện ghi chú "<Tên> đang chạy: nó cũng phản ứng với tài liệu thiếu font, bạn có thể thấy hai hộp thoại" (spike 06-3 thấy RightFont và hộp Missing Fonts của Adobe cùng hiện). Chưa có logic tắt hộ.

`AutoLogDto`: `{ id, atMs, app (slug), doc?, requested, outcome, hashHex? }`. `outcome`: `activated | missing | conflict | unavailable | not_downloaded | failed | busy | ambiguous` (`ambiguous`: nhiều bản cùng tên, đã chọn theo ưu tiên; `requested` dạng "<tên> ×N"). Nhật ký giữ 500 dòng; không có `already_active`/`available` (xem L1). `hashHex` mở được font trong thư viện.

### Bản cô lập
`AutoStatusDto.isolatedBuild: bool` cho biết rõ; UI vô hiệu nút Cài/Gỡ/Xoá khoá khi true, đừng suy từ `state` của từng app (với Photoshop/AE `isolated_build` nghĩa là nguồn font request không mở; với Illustrator `state` vẫn có thể là `needs_install`).
`auto_install`, `auto_remove`, `auto_forget_key` từ chối với `CmdError{code:"isolated_build"}` khi app chạy với dữ liệu riêng (QA/test/`FONTATLAS_DATA_DIR`): các bản đó không được ghi vào thư mục Adobe/Preferences THẬT của người dùng.

### Điều UI KHÔNG được giả định
- `auto_install(indesign)` có thể thành công với một phần phiên bản (chỉ các bản đã chạy); xem `state` sau đó, đừng tự coi là xong.
- `state=ready` của Illustrator/InDesign nghĩa là "đã cài đúng", chưa phải "đã thấy app hỏi" (xem `lastActivityMs` và `evidence`).
- Không có sự kiện đẩy cho trạng thái app chạy/tắt.

## Phát hiện thế nào (backend)

- App có trên máy: tên gói `.app` hoặc thư mục `Adobe …` trong `/Applications`, `~/Applications`, khớp tiền tố ("Adobe Photoshop" khớp "Adobe Photoshop 2026"; "Affinity").
- App đang chạy / trình quản lý khác đang chạy: `proc_listallpids` + `proc_pidpath`, lấy thành phần `.app` của đường dẫn file chạy.
- `lastActivityMs`: dòng mới nhất của app đó trong `auto_activation_log` (500 dòng gần nhất).
- `fontRequestRunning`: nguồn font request của L2 đang mở. Chỉ app thật (`lib.rs`) mở; `AppState` dựng trong test không mở (cờ `system_hooks`). Bản chạy với `FONTATLAS_DATA_DIR` cũng không mở trừ khi đặt `FONTATLAS_FONTREQ=1`.

## Kiểm

`tests/auto_status.rs` (9): thứ tự/phương thức/bằng chứng của 6 app; font request theo công tắc/nguồn/bản cô lập; trạng thái panel Illustrator (chưa cài, đã cài, cũ, lạ, không xác định); panel đã cài im khi tắt công tắc và báo lỗi khi mất broker; InDesign và InCopy báo riêng, chờ lần chạy đầu, một phiên bản chưa cài thì `needs_install`, cũ, lạ; gợi ý khởi động lại; trình quản lý khác; `lastActivityMs`; công tắc và nhật ký qua `AppState` thật (chỉ đọc máy, không cài gì). `cargo test -p app` xanh, bindings đã sinh lại, clippy sạch.

## Chưa làm / chưa đo
- Frontend (trang Settings) – worker UI.
- Tắt hộ hộp thoại của RightFont/Adobe: không làm.
- Sự kiện đẩy khi trạng thái đổi: chưa có (poll).
- Mọi mục "Chưa đo" ở `L2_fontreq.md` và `L3_L4_illustrator_indesign.md` vẫn đúng.
