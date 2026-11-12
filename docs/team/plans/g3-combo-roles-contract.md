# G3 backend — vai tuỳ ý trong Combo (hợp đồng đã làm, 2026-11-12)

user.db v11 (`USER_COMBO_ROLES`, số do vị trí trong `USER_MIGRATIONS`; nhánh khác lấy cùng số thì đổi thứ tự mục).
`combo_slots.role` bỏ CHECK 7 khoá: chuỗi 1–40 ký tự, khoá chính `COLLATE NOCASE`; `hash`/`face_index` NULL cùng lúc = vai rỗng giữ chỗ.
Dữ liệu cũ giữ nguyên (7 khoá preset là chuỗi `display`…`quote`). Test round-trip: `fontatlas-store/tests/combo_roles.rs::migration_v10_to_v11_round_trips_legacy_slots`.

## DTO
- `ComboRoleDto` = `string` (Rust `type ComboRoleDto = String`; `bindings/ComboRoleDto.ts` viết tay vì ts-rs không xuất alias).
- `ComboSlotDto.role`, `NewSlotDto.role`, `ComboRelationDto.from/to`, `PeekOccupiedDto.role`, `PeekOpenResultDto.anchorRole/otherRole`: `string`.
- `ComboDto.roles: string[]` MỚI: mọi vai theo thứ tự hiển thị, KỂ CẢ vai rỗng giữ chỗ. `ComboDto.slots` vẫn chỉ gồm vai CÓ font (nên các chỗ dùng `slots` không đổi).
  Combo mới tạo có `roles = []`: giao diện tự dùng bộ vai mặc định khi rỗng (backend không tự thêm).

## Lệnh (api + `bindings/commands.ts`)
| Lệnh | Hành vi | Lỗi (code) |
|---|---|---|
| `setComboSlot(comboId, role, face\|null, note?, style?)` | vai chưa có thì tạo; `face = null` chỉ gỡ font, VAI Ở LẠI (trước đây xoá vai) | `role_name_invalid`, `role_limit` |
| `addComboRole(comboId, name) → ComboDto` | thêm vai rỗng giữ chỗ ở cuối `roles` | `role_name_invalid`, `role_duplicate`, `role_limit` |
| `removeComboRole(comboId, role) → ComboDto` | bỏ hẳn vai (font vẫn ở trong Case) | `role_not_found` |
| `renameComboRole(comboId, role, newName) → ComboDto` | THÊM NGOÀI hợp đồng: đổi tên, giữ font/ghi chú/kiểu/vị trí; đổi chỉ hoa/thường chính nó được | `role_not_found`, `role_name_invalid`, `role_duplicate` |
| `reorderComboRoles(comboId, roles: string[]) → ComboDto` | `roles` phải đúng bằng tập hiện có (so không phân biệt hoa thường) | `role_set_mismatch` |

Quy tắc: tên được `trim`, 1–40 ký tự, không ký tự điều khiển; trùng không phân biệt hoa thường (Unicode) trong cùng Combo kể cả trùng khoá preset (`BODY` = `body`);
khi so khớp lệnh dùng lại CHỮ ĐÃ LƯU. Tối đa 30 vai/Combo. Thứ tự: preset mới thêm đứng theo vị trí preset; vai tuỳ ý xếp sau mọi vai hiện có; `reorderComboRoles` ghi lại thứ tự rõ ràng.
Bỏ font khỏi Case (`removeFromCase`) biến các vai đang dùng font đó thành vai rỗng (trước đây xoá vai). Nhân bản Combo và `createCombos` mang theo vai rỗng/thứ tự.
Tên gợi ý (Display, Heading, Subheading, Body, Caption, Quote, Label, Numerals…) nằm ở UI; backend chỉ lưu chuỗi.

## Mock
`api/mockCases.ts` mirror đủ (roles, placeholder, mã lỗi, giới hạn); `api/__tests__/mockCases.test.ts` có nhóm "custom combo roles".
