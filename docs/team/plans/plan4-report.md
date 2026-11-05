# Báo cáo Plan 4 — `fontatlas-embed` (Task 1–9)

Nhánh `feat/fontatlas-embed` (tách từ `dev/mvp` @ 7840126). Gồm 12 commit, chưa push, chưa merge.
Máy đo: Apple M4 Pro, 24 GB, macOS 27.2, rustc 1.96.0, `ort =2.0.0-rc.14` (ORT 1.31, CPU).

**Chưa làm:**
- **Task 10** (index/eval): chờ Plan 3 merge.
- **Host model:** chưa upload, chờ chủ app quyết. Trường `url` trong manifest để rỗng. Model chỉ export cục bộ, không commit.

## Kết quả
- `cargo test --workspace`: **175 test qua, 0 lỗi**. Test bị bỏ qua mặc định là parity với model thật; đã chạy riêng bằng `--include-ignored` và qua.
- Test `fontatlas-embed`:
  - render 5, encoder 6, whitening 5, model 7, worker 13, source 2, parity 2;
  - store `tests/embed.rs`: 17.
- Hai test `cancel_mid_run_writes_nothing` và `pause_then_resume_deterministic` chạy lặp 20 lần: 20/20 qua.
- `cargo clippy` cho `fontatlas-embed` và `fontatlas-store`: sạch. Các cảnh báo còn lại đều thuộc crate `app` có sẵn.
- Ràng buộc store (phương án (a) của Lead) được giữ:
  - chỉ thêm `src/embed.rs` và 2 dòng `mod`/`pub use` trong `lib.rs`;
  - không đổi file store nào khác, không đổi schema, không cần migration v2.

## Model
- Model: `google/siglip2-base-patch16-224` @ `75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2`, giấy phép Apache-2.0.
- Export: tự export `get_image_features` (768 chiều), lô tĩnh 4, opset 17, fp32.
- File: 371.694.332 B, sha256 `584d14ed…6698`. Kèm `model-NOTICE.txt` và `model-LICENSE-Apache-2.0.txt`.
- `raw_version` = `siglip2-base-p16-224@75de2d5|onnx-fp32-b4@584d14edb97b|ort1.31-cpu|render-v2`
- Whitening: `white-v1`, 396.459 B, sha256 `a45f1eed…`.
  - Fit trên nửa tune của Google Fonts @47d54ca, chỉ lấy font OFL/Apache.
- Parity Rust ↔ PyTorch trên 6 ảnh golden: cos **0,999967–0,999977** (ngưỡng ≥ 0,9999).

## Số đo thật (release, model thật, kho Google Fonts)
Đo bằng tool tạm `whitenfit --report-test` với 4 luồng ORT:
- 3.894 file → 3.744 face; 77 face `no_glyphs` (thiếu chữ Latin).
- Tách nửa: tune 1.729 face, test 2.015 face.

| Đo | Kết quả |
|---|---|
| Encode | **26,3 ms/face** (lô 4) |
| Render 3.744 face (song song) | 0,9 s |
| Tổng (render + encode + fit) | 108 s |
| RAM đỉnh (model + toàn bộ vector) | **0,93 GB** |

**Similar: cùng họ trong top-k, trên nửa test (1.284 truy vấn).**
**Chỉ dùng ảnh, chưa kết hợp các trục số đo (kết hợp ở Task 10).**

| | R@1 | R@5 | R@10 |
|---|---|---|---|
| Thô 768 | 0,691 | 0,834 | 0,871 |
| Whiten 128 | **0,764** | **0,895** | 0,921 |

So với spec S4 (R@1 ≥ 0,6, R@5 ≥ 0,9):
- R@1 đạt.
- **R@5 = 0,895 chưa đạt** khi chỉ dùng ảnh, thiếu 0,005.
- Cần đo lại ở Task 10 khi kết hợp với các trục.

## Thay đổi so với plan (đầy đủ trong ledger)
1. **Dung sai test encoder là 5e-3·|b| + 1e-5.**
   - Bản ORT tĩnh của pyke trên macOS arm64 tính MatMul ở độ chính xác khoảng bf16 (lệch tương đối tới 0,4%). ORT Python thì chính xác.
   - Model thật vẫn đạt cos ≥ 0,99996.
   - Điều này có lẽ giải thích cos 0,9969 thấy ở spike 03.
2. **`EmbedReader` thay cho `Store::open_read_only`.** Trường của `Store` là private, mà không được sửa `open.rs`.
3. **Tải model: timeout tính theo từng chunk.** Dùng luồng đọc riêng với `recv_timeout`, vì ureq 3 không có timeout cho từng lần đọc.
4. **`enqueue` giữ nguyên dòng `failed` cùng version.**
   - Trước đây khi hàng đợi rỗng, worker gọi lại `enqueue`, đưa `stale_file` về `pending` và lặp vô hạn.
   - Lỗi `missing_file` tự được thử lại khi khởi động nếu file đã có lại vị trí (ổ ngoài được cắm lại).
   - Các lỗi khác chỉ thử lại qua `retry_failed`.
5. Thêm fixture `roboto_punct_only.ttf` (8 KB, subset của Roboto) để test `no_glyphs`.

## Review cuối (subagent, model mạnh nhất): đã sửa
- **Critical:** khi chỉ đổi whitening, toàn bộ vector whiten cũ bị GC mà không tạo lại.
  - Sửa: worker gọi `rewhiten` từ vector thô lúc khởi động và trước khi GC, không chạy model.
  - Test: `whitening_only_change_rewhitens_without_model`, dùng encoder panic nếu bị gọi.
- **Important:** face trên ổ ngoài bị rút ra thì `failed` vĩnh viễn.
  - Sửa: thêm `requeue_missing_embeddings`, chạy lúc khởi động.
- **Important:** lỗi hệ thống của encoder đánh `failed` cả thư viện.
  - Sửa: dừng lượt chạy, trả lô về `pending`, phát `Error`.
  - Kiểm số chiều whitening ngay ở `start`.
- **Important:** lỗi store (SQLITE_BUSY) sau khi claim để lô kẹt ở `running`.
  - Sửa: trả lô về `pending` trước khi thoát.

Mỗi sửa có test viết trước (đỏ → xanh). Sau khi sửa: suite 175/175.

## Hoãn lại (minor)
- `claim` quét mọi dòng `pending` mỗi lần claim (O(N²) với thư viện rất lớn): thêm `LIMIT`.
- Luồng canh huỷ của encoder thêm tới 20 ms mỗi lô.
- Socket tải model bị treo có thể rò một luồng: cần thêm `timeout_global` dự phòng.
- Khi không có whitening, vector thô không được kiểm hữu hạn. `gc_embeddings` cũng chỉ chạy khi có whitening.
- `prepare` đọc và hash lại cả file cho từng face (TTC lớn).
- Đường kiểm nhanh `.ok` chỉ tin size + mtime.
- `peak_rss` của `whitenfit` sai đơn vị trên Linux (chỉ ảnh hưởng tool dev).

## Ghi chú cho Plan 6 (tích hợp app)
- Phải `join` worker cũ trước khi khởi động worker mới. Claim không có token phiên; vẫn an toàn nhưng làm việc thừa.
- Tắt `embed_enabled` giữa lúc chạy không tự dừng worker: app phải gọi `cancel()`.
- Mở worker sau khi `Store` chính đã mở, vì `Store::open` của worker có thể chạy migration.
- `url` rỗng trong manifest nghĩa là model chưa phát hành → chế độ "Số đo".
