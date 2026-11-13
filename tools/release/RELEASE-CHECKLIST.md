# Release checklist (QA/DevOps, chỉ gồm các bước đã có công cụ)

Chạy từ gốc repo, trên nhánh phát hành đã qua `local_ci` (PASS, kèm native QA).

1. **Phiên bản:** `bash tools/release/bump_version.sh <x.y.z>` rồi `bash tools/release/bump_version.sh --check` (6 vị trí phải cùng số); `cargo check` trong `UX/src-tauri` để `Cargo.lock` khớp.
2. **Giấy phép bên thứ ba:** `bash tools/release/third_party_notices.sh` để tạo lại `UX/src-tauri/resources/THIRD-PARTY-NOTICES.txt`, commit nếu đổi, rồi chạy `bash tools/release/third_party_notices.sh --check`. Bước này phải xanh: nó thất bại khi có giấy phép ngoài danh sách cho phép (GPL/LGPL/AGPL, không khai báo…), khi có tệp dữ liệu mới trong `UX/public`, `crates/*/data`, `resources` chưa được kiểm giấy phép (thêm vào `DATA_ITEMS`), hoặc khi file đã commit lệch `Cargo.lock`/`node_modules`. Cần `npm ci` trong `UX/` và cargo đã fetch. Không tải gì.
3. **CI đầy đủ** trên đúng SHA sẽ phát hành: `bash tools/ci/local_ci.sh <ref>` (note `pass` cùng cây).
4. **Build release** (`cargo tauri build`) và kiểm `Font Atlas.app/Contents/Resources/THIRD-PARTY-NOTICES.txt` có mặt (tauri.conf.json đã ánh xạ nó).
5. **Đóng DMG:** `bash tools/release/make_dmg.sh "<Font Atlas.app>" <out.dmg>`; script từ chối app thiếu `THIRD-PARTY-NOTICES.txt`.

Màn Credits trong app trỏ tới `Contents/Resources/THIRD-PARTY-NOTICES.txt` (cạnh `NOTICE-hyperglot.md`, `LICENSE-hyperglot.txt`).
