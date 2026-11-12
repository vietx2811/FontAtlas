# C10 — rà test phụ thuộc thời gian (2026-11-12)

Nguyên tắc sửa: không đo "mất bao lâu" làm điều kiện đạt. Dùng (1) điều kiện/sự kiện/latch (cờ `finished`, kênh, `waitFor`),
hoặc (2) chờ có hạn dài (đạt thì trả về ngay, hỏng mới tốn hạn). Mỗi sửa phải chứng minh đỏ trước / xanh sau dưới tải giả lập
(4 bản chạy song song), hoặc bằng đột biến.

## 1. vitest (đã đo)
Chạy 4 bản `vitest run` song song trên cùng cây, mỗi bản 1357 test:

| | trước | sau lô 1 (config) | sau lô 2 (BootGate + outro) |
|---|---|---|---|
| bản 1/2/3/4 đỏ | 18 / 7 / 8 / 4 | 1 / 2 / 1 / 1 | xem mục cập nhật cuối tệp |

Nguyên nhân gốc: timeout mặc định (test 5 s, `waitFor`/`findBy` 1 s) hụt khi máy bận; test hụt để lại trạng thái dở dang làm
đỏ lây test kế (chip thừa ở FilterFacets, "không thấy gridcell" ở LibrarySelection). Sửa tập trung: `testTimeout`/`hookTimeout` 30 s,
`configure({ asyncUtilTimeout: 8000 })` trong `test/setup.ts`.

Lỗi thật phát hiện:
- `i18n/liveSwitch` (BootGate): mock phát sự kiện pha `scanning` sau khi `startupStatus` đã `ready` → cổng lùi về "loading". Máy chậm hơn 100 ms là đỏ.
  Sửa: pha đến muộn không kéo cổng khỏi trạng thái đã sẵn sàng. Test mới + đột biến.
- `Onboarding` outro: `outroMs={50}` rồi khẳng định "chưa gọi onDone" → máy bận đã quá 50 ms. Sửa: outro 1500 ms + `waitFor` có hạn dài.
- `Settings` "Phân tích hình ảnh 800/3.800": bản 4-song-song từng thấy 1.000/3.800 (số tiến độ chạy theo đồng hồ thật). Theo dõi; nếu còn đỏ sau lô 2 thì đổi sang đồng hồ giả.

## 2. Rust (danh sách, chưa sửa) — cận trên thời gian đo thật
Rủi ro cao (cận chặt, đo `elapsed`): 
- `fontatlas-store/tests/concurrency.rs:56,107` (< 200 ms mỗi lần)
- `fontatlas-fontreq/tests/responder.rs:207` (< 400 ms), `:211` (< 50 ms)
- `fontatlas-fontreq/tests/coretext_e2e.rs:127` (< 2,5 s đóng nguồn)
- `fontatlas-activate/src/paths.rs:345,366` (< 2 s; 300 ms ≤ x < 3 s)
- `src-tauri/src/services/access.rs:106` (< 900 ms)
- `tests/langs.rs:206` (< 3 s)
- `fontatlas-broker/tests/http.rs:354` (shutdown < 4 s)
Rủi ro vừa (cận 5 s, kiểu "huỷ phải kịp"): `tests/embed.rs:164,202`, `tests/review_fixes.rs:133`, `tests/analysis.rs:371`.
Đã sửa trước đó: `fontatlas-embed` `cancelling_slow_blocking_dns…` (b37176f0, c75221fa, d6c65504).
Rủi ro thấp (vòng chờ có hạn 30–60 s đã đúng kiểu): `tests/cases.rs:230`, `tests/langs.rs:41,172`, `tests/embed.rs:115`, `tests/scan.rs:142`, embed `model.rs:932,998`.
Còn ~54 chỗ `thread::sleep`/`time::sleep` trong `tests/` cần xét từng chỗ (chờ sự kiện hay chờ đồng hồ).

## 3. Native QA `UX/src-tauri/tests/qa/main_flows.js`
121 lệnh `sleep(N)`; 43 chỗ là "sleep rồi đọc kết quả để `log`" không qua `wait(...)` (rủi ro: máy bận thì chưa kịp). Dòng:
92, 119, 125, 165, 171, 189, 196, 229, 231, 283, 286, 299, 307, 314, 554, 673, 890, 895, 940, 952, 955–956, 961, 966, 972, 975 (có kiểm kết quả ngay sau); các chỗ còn lại chỉ chờ trước `snap`
(chụp ảnh, ít rủi ro đỏ). Cách sửa: thay bằng `wait(() => điều kiện)` có hạn ≥ 8 s như đã làm cho `sidebar-context-menu`.
Lần đỏ gần nhất (CI 0.3.1): script dừng sớm trong khối Pairing Desk (sau `desk-is-the-only-page`) khi máy tải ~21 — chưa rõ chỗ treo; cần log theo từng bước để khoanh.

## Cập nhật (cuối lô 1)
- vitest, 4 bản song song sau sửa BootGate + outro: 0 / 0 / 1 / 0 test đỏ trên 1358 (từ 18 / 7 / 8 / 4). Test đỏ còn lại (Onboarding O6) đã sửa: `click` chờ nút, O6 dùng `findByText`.
- Rust: đã bỏ cận trên thời gian đo thật ở `store/concurrency` (giữ khoá bằng latch, hỏng thì `busy_timeout` làm unwrap hỏng), `fontreq/responder` (việc nền chốt bằng latch),
  `services/access` và `activate/paths` (cờ `finished`: lúc hàm trả về, việc treo chưa xong), cận dưới 300 ms vẫn giữ. Các cận còn lại (embed, langs, review_fixes, analysis, broker, coretext)
  nới thành 20–60 s chỉ để bắt treo; điều kiện thật là trạng thái. Hạn chế: `responder` mất kiểm "lần trả lời thứ hai không chờ thêm" (trước đây < 50 ms); còn kiểm `calls == 1` và việc nền bị chốt.
- QA `main_flows.js`: khối Pairing Desk (ArrowDown/`i`/`[`/Combo mới) đổi `sleep` cố định sang `wait` theo điều kiện. Chưa chạy native QA (chỉ qua khoá CI). Còn ~35 chỗ `sleep` rồi đọc kết quả ở các khối khác (danh sách ở mục 3).
