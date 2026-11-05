# Plan 7 — `fontatlas-activate`: báo cáo

Nhánh `feat/fontatlas-activate` (tách từ `dev/mvp`). Plan: `docs/superpowers/plans/2026-11-04-plan7-fontatlas-activate.md`.
Cơ sở kỹ thuật: `docs/team/spikes/04_activation.md`.

## Trạng thái

| Phần | Kết quả |
|---|---|
| Store: `user.db` v2 (`activation_state` + `activation_reasons`), API có kiểu | Xong, 11 test |
| Crate `fontatlas-activate`: trait, `FakeEngine`, `paths`, service, deactivate, reconcile/shutdown | Xong |
| `MacCoreTextEngine` | Xong; test chỉ đọc chạy mặc định |
| `WindowsGdiEngine` | Chỉ stub (`Unsupported`) + CI biên dịch Windows chưa chạy |
| Hợp đồng IPC (fixture JSON) | Xong |

## Test

Chạy bằng `FakeEngine`, không đăng ký font nào lên hệ thống:

`cargo test --locked -p fontatlas-activate` → 10 (unit) + 3 (CoreText chỉ đọc) + 3 (contract) + 13 (reconcile) + 36 (service) = **65 pass, 8 ignored**. Clippy `-D warnings` sạch.

## Test sống CoreText: CHƯA CHẠY hết

Lý do: chưa có đồng ý của chủ app. Chỉ **4 test của Task 3** đã chạy một lần, có sự đồng ý của chủ app, trên macOS 27.2 (build 26B118): 4/4 pass, font không còn sót.
**8 test sống còn lại (gồm 4 test mới của Task 8) chưa chạy trong lần này.** Task 3 là bản đã chạy; riêng 4 test Task 8 chưa bao giờ chạy.

| Test (tests/mac_live.rs) | Kiểm | Trạng thái |
|---|---|---|
| `live_session_register_is_visible_and_idempotent` | session, 105/201, hiện/biến mất | đã chạy (Task 3) |
| `live_wrong_scope_unregister_is_not_registered` | gỡ sai scope → 201 | đã chạy (Task 3) |
| `live_same_url_other_scope` | cùng URL, hai scope → `Ok` | đã chạy (Task 3) |
| `live_provider_paths_are_canonical` | `providers` trả path canonical | đã chạy (Task 3) |
| `live_service_roundtrip_session` | bật → thoát → mở lại → tắt qua service | **chưa chạy** |
| `live_vietnamese_filename_roundtrip` | tên file "Phở Đậm.ttf", gỡ khi file đã xoá | **chưa chạy** |
| `live_preflight_of_active_file_has_no_conflict` | preflight font đã bật không báo xung đột | **chưa chạy** |
| `live_duplicate_ps_name_is_flagged` | bản trùng PS name → owner `FontAtlas` | **chưa chạy** |

Điều chưa được kiểm chứng trên CoreText thật: tên file tiếng Việt (NFC/NFD), vòng đời service, và xử lý trùng tên qua service.

**Cách chạy sau khi chủ app đồng ý** (font: Libre Baskerville Regular, OFL, bản copy trong thư mục tạm; scope session và persistent; mỗi đăng ký tự gỡ qua guard; test tự bỏ qua nếu máy đã có font này; gỡ thủ công: Font Book → chuột phải Libre Baskerville → Remove):

```bash
cd UX/src-tauri
FONTATLAS_LIVE_ACTIVATION=1 cargo test -p fontatlas-activate --test mac_live -- --ignored --test-threads=1
```

Kỳ vọng: 8 pass. Sau đó kiểm `CTFontManagerCopyAvailablePostScriptNames` không còn `LibreBaskerville-Regular`.

## Lệch plan (đều có `Ruling:` trong ledger)

- `FileOutcome::Ready` thêm mới (preflight không xung đột). Cộng thêm, 6a/6b cần biết.
- `create_user_v1_for_test` là `#[doc(hidden)] pub`, vì test tích hợp không thấy `cfg(test)`.
- `providers()` lọc theo tên chính xác thay vì dùng `CFSet` bắt buộc.
- Không dùng let-chains (MSRV 1.85).
- Phát hiện bug khoá `in_flight` nhờ test song song (deadlock do `then_some`), đã sửa.
- Thêm test ngoài brief: `reconcile_finishes_interrupted_deactivate`, `reconcile_reports_left_collection`, `set_scope_register_failure_keeps_old_scope`, `state_lists_rows_with_reasons`.
- Chưa chạy được job `windows-latest` cục bộ.

## Hợp đồng cho Plan 6a

API (`ActivationService<E>`, nhận `&Mutex<Store>`, khoá theo từng file): `preflight`, `activate_faces`, `activate_collection`, `deactivate_faces`, `deactivate_collection`, `deactivate_all`, `set_scope`, `state`, `reconcile`, `shutdown`, `default_scope`. `platform_engine()` chọn engine theo nền tảng; `FakeEngine` công khai để 6a test lớp lệnh không đụng hệ thống.
Fixture JSON: `crates/fontatlas-activate/tests/fixtures/{activation_report,activation_state,reconcile_report}.json`.
Cài đặt dùng bảng `settings` có sẵn: `activation_keep_after_quit`, `activation_onboarded`. Plan 7 là nơi duy nhất nâng `user.db` lên v2; 6a thêm `pairs` ở v3.

## Chưa làm

- Windows thật (Plan 7b): `AddFontResourceExW` + `WM_FONTCHANGE`, ghi chú trong `windows.rs`.
- Checklist Adobe / Figma của spike §1.3 (cần người dùng mở app).
- Đăng xuất / khởi động lại máy với scope persistent.
