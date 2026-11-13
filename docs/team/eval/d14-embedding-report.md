# D14: embedding và tải model (04, 2026-11-13)

Nhánh `feat/d14-embed-hardening` (base dev/mvp `1b64934b`). Số đo: build `--release`, máy dự án, thư viện tổng hợp N face
(mỗi face một file, đường dẫn chỉ có trong DB); không có ngưỡng thời gian trong test, chỉ bench chạy tay.

## 1. Truy vấn O(N²): `claim_embedding_batch`

**Nguyên nhân.** Worker claim theo lô `BATCH = 4`. Mỗi lần gọi, `claim_embedding_batch` nạp **mọi** dòng `pending` của
`pipeline_state` (không `LIMIT`, `ORDER BY updated_at, hash, face_index` không có chỉ mục) rồi chỉ dùng 4 dòng đầu.
Nạp N face = N/4 lần gọi × O(N) dòng → O(N²/BATCH).

**Sửa** (`fontatlas-store/src/embed.rs`): bộ đệm ứng viên trong `Store`, nạp theo trang 1024 dòng bằng con trỏ keyset
`(updated_at, hash, face_index)`. Mỗi dòng vẫn được kiểm lại còn `pending` đúng version lúc claim (bộ đệm có thể cũ).
Thứ tự claim, giữ `pending` cho bản chỉ-cloud và đánh `failed/missing_file` giữ nguyên. Không đổi schema (không migration,
tránh đụng số phiên bản với nhánh khác). `Store::embed_claim_scans()` đếm số lần quét, dùng cho test và bench.

Claim toàn bộ hàng đợi theo lô 4 (`embed_queue_bench`):

| N face | trước | sau | số lần quét sau |
|---|---|---|---|
| 2.000 | 0,16 s | 0,063 s | 4 |
| 5.000 | 2,9 s | 0,168 s | 7 |
| 10.000 | 4,1 s | 0,438 s | 12 |
| 20.000 | 13,0 s | 0,700 s | 22 |

Trước: ×10 face → ×80 thời gian (bậc hai). Sau: tăng gần tuyến tính (10k → 20k: ×1,6). Chi phí còn lại là việc theo từng face
(kiểm lại, tra đường dẫn, ghi `running`). Số "trước" 5k nhiễu (2,3 ms/lần gọi so với 1,6 ms ở 10k); xu hướng thì rõ.

Lệnh: `cargo test -p fontatlas-store --release --test embed_queue_bench -- --ignored --nocapture`.
Test: `tests/embed_claim.rs` (9 test; RED trước khi sửa là test đếm quét: 12 lần cho 41 face).

**Đã đo và không phải nguyên nhân.** `similar_families` trên Index (không có store), `suggest_bench` (thêm N = 20k):
p50 0,8 / 2,3 / 6,6 / 21,7 ms tại 2k / 5k / 10k / 20k: gần tuyến tính. `pair_candidates_gated` 0,5 / 1,2 / 1,8 / 8,0 ms.

**Chưa đo.** Trong app, `similar_families` gọi `similar_with(k = toàn thư viện)` nên bước xếp lại f32 đọc embedding của *mọi*
ứng viên từ SQLite (N tra cứu theo khoá mỗi truy vấn): vẫn tuyến tính, nhưng nên đo với store thật nếu truy vấn Similar chậm ở
20k+. Gợi ý nếu cần: chỉ xếp lại f32 cho các trang họ đang hiển thị.

## 2. Luồng tải model kẹt

Kiểm bằng server giả:
- Server nhận kết nối nhưng không bao giờ trả header: timeout đúng hạn, không để `.part`. **Đã đúng sẵn** (test mới xanh).
- Hủy khi đang chờ header (timeout 30 s): trả `Cancelled` < 3 s. **Đã đúng sẵn.**
- Hủy khi tiến trình khác (app thứ hai, công cụ dòng lệnh) giữ khoá `.lock`: luồng `fontatlas-model` **kẹt vô hạn** ở
  `lock.lock()` (chặn, bỏ qua cờ huỷ), `wait_download` báo `busy` mãi. **Đây là chỗ rò thật** (ADR round 2 đã ghi là chưa xử lý).

Sửa (`fontatlas-embed/src/model.rs`): `lock_unless_cancelled` thăm dò `try_lock` mỗi 50 ms theo cờ hủy. Test mới ở `tests/model.rs`
(3 test); 38/38 test `model` xanh. Giới hạn: chưa có timeout chờ khoá khi không ai huỷ (chờ đến khi bên kia nhả).

## 3. Phân loại mã lỗi

`EmbedWorker` phát `Progress::Error(String)` làm mất mã lỗi và `services/embed.rs` gắn cứng `load_error = "inference"` cho **mọi**
lỗi của worker (lỗi store, đĩa, render…). Nay `Progress::Error { code, message }` mang `EmbedError::code()`, app dùng mã đó.
Test `worker_error_keeps_the_typed_error_code` (RED trước: không biên dịch được vì thiếu trường). DTO `load_error` giữ kiểu chuỗi.

**Còn lại, cần quyết định (đụng mã hiển thị cho UI):** `EmbedError::Model(String)` là hố chứa, mã `inference`, dùng cho ≥ 8 loại
khác nhau (định dạng safetensors, whitening lệch chiều, kích thước tensor, encoder panic, gọi API đồng bộ sai ngữ cảnh…).
Tách mã (vd. `model_format`, `whitening`, `internal`) cần frontend ánh xạ thêm thông báo; tôi chưa làm.
