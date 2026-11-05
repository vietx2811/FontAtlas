# bindings — hợp đồng TypeScript của backend Tauri (Plan 6a)

Mọi file ở đây **do máy sinh**. Không sửa tay.

| File | Nguồn |
|---|---|
| `<Kiểu>.ts` (DTO, sự kiện, `CmdError`) | `ts-rs`, xuất khi chạy `cargo test -p app` (env `TS_RS_EXPORT_DIR`, xem `src-tauri/.cargo/config.toml`) |
| `commands.ts` (`TauriCommandMap`, `CommandName`) | bảng `COMMANDS` trong `src-tauri/src/commands.rs` (`src-tauri/src/bindings.rs`) |
| `index.ts` | re-export mọi kiểu |
| `fixtures/*.json` | JSON mẫu của từng kết quả chính; `activation_*`/`reconcile_report` khớp fixture hợp đồng của Plan 7 |

## Sinh lại

```bash
cd UX/src-tauri
UPDATE_BINDINGS=1 cargo test -p app
```

CI chạy `cargo test -p app` rồi `git diff --exit-code UX/bindings`: kiểu Rust đổi mà chưa sinh lại thì CI đỏ.

## Quy ước

- Tên lệnh snake_case; khoá đối số và trường JSON camelCase; enum serde `snake_case`.
- Sự kiện đẩy qua `Channel<T>` dạng `{ type: "…", … }` (`EmbedEvent` dùng `kind`).
- Số nguyên 64-bit được khai báo `number` (JSON không có bigint).
- `FaceRowDto` giữ đúng 19 khoá của `FaceRow` (parser Plan 5 từ chối khoá lạ).
- `ActivationStateDto` = hợp đồng Plan 7 + `faces` (các `faceIndex` của file; file một face là `[0]`).
