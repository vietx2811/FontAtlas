# G7: font variable (VF) trùng tên với bản static — hỏi một lần khi bật

Người viết: 06 · 2026-11-12 · Nhánh `feat/vf-static-choice`. Nghiên cứu: `docs/team/research/2026-11-12-variable-vs-static-names.md` (02, nhánh `research/g7-vf-static-names`): tra theo PostScript luôn ra bản VF; họ+kiểu ra cả hai. Ảnh hưởng tới việc bật: `docs/team/spikes/06_g7_activation_impact.md`.

## Hành vi
- Bật một font mà bản trùng tên PostScript đang bật (do Font Atlas hay app khác), và **một bên là VF, bên kia là static** → hộp thoại xung đột hiện có đổi sang chế độ "cặp VF↔static": ba lựa chọn gọi theo loại font (có tooltip thuật ngữ `variable font (VF)` / `static font`):
  - **Dùng bản {loại đang có}** = bỏ qua bản mới (mặc định, không đổi gì);
  - **Dùng bản {loại này}** = tắt bản kia rồi bật bản này (`ConflictPolicy::Replace` có sẵn: có xác nhận, luật bảo vệ của 03, sổ khôi phục);
  - **Giữ cả hai** = bật cả hai (`Proceed`), kèm cảnh báo: app thiết kế thường nhận bản VF khi tra theo PostScript và thấy hai mục giống nhau.
- **Không bao giờ tự thay.** Không chọn sẵn "dùng bản này". Font hệ thống vẫn luôn được ưu tiên ("giữ cả hai" bị khoá); app Adobe/Connect chỉ có hướng dẫn; thư mục được bảo vệ không đụng.
- Ô **"Nhớ lựa chọn này"** lưu theo **loại**, không theo từng bản: `vf` | `static` | `both`. Lần sau gặp cặp VF↔static: `vf` → dùng bản VF (bản mới là VF thì thay, là static thì giữ VF đang có); `static` tương tự; `both` → giữ cả hai. Làm theo lựa chọn nhớ **chỉ khi được phép**: "dùng bản này" cần mọi bản trùng ở mức `replace` và không có app lạ; "giữ cả hai" không có font hệ thống; không thì vẫn hỏi như thường. Có thông báo nói thật đã làm gì (và nút Khôi phục sau khi thay).
- Đổi/quên lựa chọn: Cài đặt → Kích hoạt font → "Cặp variable ↔ static trùng tên" (Hỏi mỗi lần / Luôn dùng VF / Luôn dùng static / Luôn giữ cả hai).
- Chỉ áp dụng khi người dùng bật font trong app. **Broker/auto-activation không đổi**: vẫn `ConflictPolicy::Ask`, không bao giờ thay hay giữ cả hai.
- Chỉ coi là cặp khi MỌI bản trùng (không tính bản giống hệt) khác loại với font sắp bật và mọi font cùng lượt cùng loại; không đọc được loại (`existingVariable = null`, file chưa tải, không phải sfnt) hoặc cùng loại → hộp thoại xung đột thường.

## Kỹ thuật
- `fontatlas-activate`: `is_variable_font(path)` đọc bảng tra cứu sfnt tìm `fvar` (vài trăm byte, chỉ khi file `Ready`: không mở file iCloud chưa tải; TTC xét font đầu). `Conflict` thêm `variable` và `existing_variable: Option<bool>` (DTO `variable`, `existingVariable`).
- **Lưu lựa chọn: không thêm migration.** `user.db` đã có bảng `settings` (key/value); dùng khoá `activation_vf_policy` (`vf`/`static`/`both`; thiếu hoặc giá trị lạ = hỏi mỗi lần; `Ask` xoá khoá). Một khoá chung không cần bảng/phiên bản riêng, tránh đụng số migration của nhánh khác. (Lead nói "có migration": đây là chỗ lệch có chủ ý; cần bảng riêng nếu muốn nhớ theo từng family.)
- Lệnh mới: `activation_vf_policy`, `activation_set_vf_policy`.
- Giao diện: `conflictModel.vfPairOf / vfActionOf / vfChoiceOf`, `ActivationDialogs` (`VfPairOptions`), `useActivationFlow.applyRemembered`, `applyReplace.runReplace` (dùng chung cho hộp thoại và lựa chọn nhớ), Settings. Chuỗi vi + en ở `i18n/vfpair.ts` (có test song ngữ).

## Kiểm
`fontatlas-activate/tests/vf_static.rs` (cờ hai chiều, bản VF của app khác, hai static không phải cặp, replace/proceed/ask); `src-tauri/tests/vf_static.rs` (setting, JSON); `components/activation/__tests__/vfPair.test.tsx` (mô hình, hộp thoại, luồng với mọi lựa chọn nhớ); Settings; i18n.

## Chưa làm / ghi chú
- Chưa đo trên app Adobe thật khi cả hai cùng bật (02 và 06 đều [SUY]).
- Đã làm theo duyệt của Lead: tie-break `pick()` của broker khi nhiều bản CÙNG TÊN: đang bật > khớp họ+kiểu app gửi > **static trước VF** > thấy sau cùng > hash nhỏ. **Hai tiêu chí giữa là SUY LUẬN, chưa đo trên app Adobe**: app chỉ gửi tên PostScript nên không biết tài liệu cần VF hay static; chọn static vì tra theo PostScript ra VF khi cả hai cùng bật (02). Test: `fontatlas-broker/tests/ensure.rs` (VF Oswald thật + static vá cùng tên PostScript). `Store::face_pick_traits` cung cấp (is_variable, khớp họ/kiểu).
