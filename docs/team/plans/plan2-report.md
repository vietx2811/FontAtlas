# Báo cáo Plan 2 — `fontatlas-store`

Nhánh `feat/fontatlas-store` (tách từ `feat/fontatlas-core` @ 8965152). Chưa push, chưa merge.
Máy đo: Apple M4 Pro, 24 GB, macOS 27.2, rustc 1.96.0.

## Kết quả
- `cargo test -p fontatlas-store`: **58 test qua, 2 bỏ qua** (đo hiệu năng, chạy bằng `--ignored`).
- `cargo test -p fontatlas-core`: vẫn qua. `cargo check -p app`: qua — `rusqlite =0.31.0` dùng chung `libsqlite3-sys` với app, không xung đột.
- `cargo clippy -p fontatlas-store --all-targets -- -D warnings`: sạch.

## Số đo (release)
| Đo | Kết quả |
|---|---|
| `query_faces` 50.000 face, trang đầu | 12,3 ms |
| `query_faces` offset 40.000 | 24,4 ms |
| `query_faces` tìm "phong 4999" (FTS) | 0,6 ms |
| `query_faces` lọc `vi=Full`, sắp theo weight | 29,7 ms |
| `rescan_source` `/System/Library/Fonts` lần 1 (369 file, phân tích hết) | 0,50 s |
| `rescan_source` lần 2 (không đổi gì) | 3,1 ms |

Đều dưới S6 (< 50 ms) và S1 (1.000 font < 3 s, ngoại suy từ 369 file/0,5 s). Chưa cần phân trang keyset.

## Lệch so với plan (đều có trong ledger)
1. Áp toàn bộ mục sửa A1–A13 của Lead (ghi trong plan, phần "Sửa sau review").
2. Test tìm "dep": trigram khớp chuỗi con nên "dep" cũng ra "SourceCodePro" (co-**dep**-ro). Đổi assert thành "có Đẹp Sans" và thêm "dep sans" khớp đúng một face. Hành vi tìm kiếm là đúng.
3. Sau khi dựng lại `library.db`: `user.db` giống từng byte; `user.db-wal` chỉ so độ dài (0), vì SQLite tạo lại file `-wal` rỗng khi mở DB WAL.
4. `resolve_legacy` chạy một transaction cho mọi dòng (plan ghi mỗi dòng một transaction). Vẫn chỉ `INSERT OR IGNORE`, chạy lại an toàn.
5. `import_legacy` bỏ qua nguồn cũ lồng trong nguồn khác (theo quyết định chặn nguồn lồng nhau); rescan nguồn cha đã phủ font của nguồn con.
6. Thêm `hash_from_blob()` để đọc blob 32 byte từ DB không panic.

## Ghi chú cho các plan sau
- Lỗi chỉ ghi ở **cấp file** (`pipeline_state.face_index = -1`), vì core trả `Result` theo file; trạng thái theo face hiện chỉ có `analysis=done`. Plan 4 (embedding) sẽ dùng lỗi theo face.
- Rescan đọc file mới **hai lần** (băm BLAKE3 rồi `analyze_paths` đọc lại). Để tối ưu sau: cho core nhận bytes đã đọc, hoặc chỉ băm khi (size, mtime) đổi mà path đã biết.
- `Store` không `Sync`: Plan 6 bọc `Mutex` hoặc mở thêm một `Store` đọc cho UI (WAL cho phép đọc song song).
- Thứ tự khởi động cho Plan 6: `Store::open` → `import_legacy` (nếu có `fontatlas.db` và chưa có `u.settings.legacy_import`) → `rescan_source` mọi nguồn thư mục → `resolve_legacy`.
- `RescanStatus::SourceUnavailable` (ổ ngoài đã rút): UI nên hiện "Nguồn không truy cập được", không coi là font bị xoá.

## Review toàn nhánh (subagent Opus, sau Task 8)
Kết luận: 0 Critical, 4 Important, 11 Minor. Đã sửa cả 4 Important (commit 593e2b8), mỗi mục có test đỏ trước rồi xanh; suite 63/63 qua, clippy sạch.
- I1: nguồn cũ lồng nhau đọc theo thứ tự id chuỗi có thể giữ con bỏ cha → sắp theo độ sâu đường dẫn, thêm `LegacyImport.skipped_sources`.
- I2: thư mục con hoặc file không đọc được (hoặc ổ rút giữa lúc quét) bị đánh missing → bỏ qua vị trí dưới thư mục lỗi, giữ file đã biết không đọc được, kiểm lại gốc trước bước đánh missing (nhánh "rút ổ giữa lúc quét" chưa có test tự động).
- I3: lỗi `io` tạm thời bị ghi `failed` vĩnh viễn → ghi `pending`, lần quét sau tự thử lại.
- I4: `SQLITE_BUSY` khi migration library bị coi là hỏng → dựng lại library; giờ đặt `busy_timeout` ngay sau khi mở và không coi bận/khoá là hỏng.
- Thêm: test Review Focus 1 kiểm cả collection, tag và vị trí cũ `missing=1`.

Minor để lại (cho Plan 6 hoặc sau): `remove_source` ghi hai file trong một transaction (không nguyên tử khi crash); rescan không tự gọi `gc`; rescan giữ mọi kết quả trong RAM rồi mới ghi; rescan giữ `&mut self` suốt phần I/O; lọc theo nguồn chưa gắn "không missing" với đúng nguồn đó; `total` và trang không chung read transaction; transaction DEFERRED (nên IMMEDIATE khi có kết nối thứ hai); so path DB cũ chỉ NFC chưa canonicalize; TTC không khớp rơi về face 0; file `library.broken-*` không được dọn.
