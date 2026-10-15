# Sprint 0 – Repo hygiene & build xanh (QA/DevOps)

Ngày: 2026-10-15 · Nhánh: **`chore/sprint0-hygiene`** (tách từ `App` @ 63b6096) · Worktree riêng: `.claude/worktrees/silly-driscoll-89a075`
Chưa push, chưa merge. Không chạm thư mục làm việc của chủ app, không viết lại lịch sử, không xoá binary/_ARCHIVE.
(File này nằm trên nhánh, vì hook của worktree chặn ghi trực tiếp vào thư mục gốc.)

## Commit
| SHA | Nội dung |
|-----|----------|
| `4d81f92` | `chore: ignore node_modules, Vite cache, Rust target and sidecar build output` – bổ sung `.gitignore` gốc: `node_modules/`, `.vite/`, `*.tsbuildinfo`, `npm-debug.log*`, `target/`, `UX/src-tauri/gen/schemas`, `.venv/`, `UX/python-sidecar/build/`, `UX/python-sidecar/dist/` (đã có sẵn: `venv/`, `qdrant_storage/`, `.DS_Store`, `dist/`, `build/`). |
| `c707e17` | `chore: stop tracking UX/node_modules` – `git rm -r --cached UX/node_modules` (5.349 file, −1,31 triệu dòng). File trên đĩa giữ nguyên. |
| `80e5a74` | `fix(ux): complete mock data to satisfy FontMetrics/FontData/LibraryFont types` – `UX/constants.ts`, `UX/hooks/useLibrary.ts`. |
| (commit này) | `docs: add Sprint 0 QA/DevOps report` |

Tổng thay đổi code `App..80e5a74`: 5.352 file, +61 / −1.312.546 dòng.

## Sửa lỗi TypeScript
- `constants.ts` (6 mock `MOCK_FONTS`): thêm `curvature`, `terminalSharpness`, `stressAngle`, `counterDensity` = `0`.
- **Phát hiện thêm**: sau khi hết TS2739, tsc báo tiếp 6 lỗi TS2741 – cùng 6 mock thiếu `isFavorite` của `FontData` (bị che trong báo cáo 05). Thêm `isFavorite: false`.
- `hooks/useLibrary.ts:81` (mock web mode): thêm `is_favorite: false`.
- Giá trị chọn trùng với fallback mà `App.tsx:153-164` đã dùng khi map `LibraryFont → FontData` (`|| 0`, `|| false`) → không đổi hành vi. (Trước đây mock thiếu field sẽ crash ở `DetailPanel.tsx:276` khi gọi `.toFixed` trên `undefined`.)

## Kết quả chạy thật
Lưu ý: `node_modules` đang commit trong repo **không đầy đủ** (thiếu `lucide-react`, `framer-motion`, `vite`, `@vitejs/plugin-react`, `@tailwindcss/vite` → tsc báo TS2307). Trong worktree đã chạy `npm ci` (139 gói, 6 s) trước khi kiểm tra. Sau `npm ci`, `git status` sạch → `.gitignore` hoạt động.

**`npx tsc --noEmit`** (UX/)
```
tsc exit=0
```

**`npx vite build --outDir <scratchpad>/vite-out --emptyOutDir`**
```
✓ 2755 modules transformed.
index.html                   0.38 kB │ gzip:   0.27 kB
assets/index-BenQoQVm.css   57.41 kB │ gzip:   9.93 kB
assets/index-C-BjdCBw.js   769.97 kB │ gzip: 235.14 kB
✓ built in 2.25s          (exit 0)
```
Cảnh báo (không đổi so với báo cáo 05): `@import url(fonts.googleapis…Inter)` không đứng đầu `index.css`; `@tauri-apps/api/core.js` và `services/sources.ts` vừa import tĩnh vừa động; chunk JS 770 kB > 500 kB.

**`cargo check`** (UX/src-tauri, `CARGO_TARGET_DIR=<scratchpad>/cargo-target`, cargo/rustc 1.96.0, có mạng)
```
warning: `app` (lib) generated 11 warnings (run `cargo fix --lib -p app` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 35.94s   (exit 0)
```
11 cảnh báo: 4× deprecated `qdrant_client::qdrant::VectorOutput::data` (`src/qdrant.rs:162,165,…`); unused imports (`DateTime`, `QdrantError`, `Condition/FieldCondition/Filter/Match/PointStruct`); unused variable `app` (`src/lib.rs:934`); dead code `log_to_file` (`lib.rs:18`), `METRIC_VECTOR_SIZE` (`qdrant.rs:16`), `generate_rationale` (`qdrant.rs:767`). `Cargo.lock` không thay đổi.

## Tồn đọng – cần chủ app / Lead quyết định
1. **Dung lượng `.git` vẫn 113 MB**: gỡ khỏi index không xoá lịch sử. Muốn giảm phải `git filter-repo` (viết lại lịch sử đã push lên origin/main) → cần chủ app đồng ý và phối hợp cả team (mọi người clone lại).
2. **Binary sidecar 27 MB tracked, 2 bản trùng** (`UX/binaries/fontatlas-parser` và `-aarch64-apple-darwin`): chưa động tới theo lệnh cấm. Đề xuất: Git LFS hoặc build trong CI/Release asset, rồi ignore `UX/binaries/*`.
3. **Merge nhánh**: thư mục làm việc gốc (nhánh `App`) vẫn track node_modules và có thay đổi chưa commit. Merge `chore/sprint0-hygiene` vào `App` sẽ chỉ ngừng track node_modules, nhưng chủ app nên commit/xử lý thay đổi đang dở trước, sau đó chạy `npm ci` trong `UX/` vì bản commit thiếu gói.
4. CI chưa có: thêm job `npx tsc --noEmit` + `vite build` + `cargo check` (mẫu ở báo cáo 05 §3.2) để lỗi kiểu không quay lại.
5. Dọn 11 warning Rust (đặc biệt API qdrant deprecated) và 3 cảnh báo Vite – ngoài phạm vi Sprint 0.
6. Các mục khác của báo cáo 05 chưa làm: `data/` vs `src/data/` trùng, `.agent/.claude/` track hay ignore, `fonts/samples` mâu thuẫn ignore, pytest.
