# Review downloader ban đầu — working diff, chưa nghiệm thu

Worker Backend đang sửa stage3 sau `57694d5`. Không áp kết luận này vào SHA đã commit vì code vẫn biến động. Worker báo 25 model test qua; Lead đã đọc dependency diff/model.rs, chưa chạy test độc lập.

1. `ensure_resumable`: `File::open(&self.dir)?.sync_all()?` sau rename chạy vô điều kiện. Nguy cơ open directory trên Windows bị access denied rồi tải thành công lại báo lỗi. Đã yêu cầu helper durability theo nền tảng, mô tả hạn chế và test/compile phù hợp.
2. `ensure_resumable`: guard `Handle::try_current().is_ok()` xung đột với hướng dẫn gọi từ `spawn_blocking` (blocking thread có thể vẫn có Tokio context). Đã yêu cầu test tái hiện hoặc giới hạn sync API rõ ràng dedicated thread, không quảng cáo hỗ trợ spawn_blocking sai. Unsupported-call guard cần đặt trước lock/mutation.

Cả hai điểm đã gửi agent `62df14a1-7665-42eb-86b8-85aca09b22a8`, host `srv_K32ksUtIDMow`. Chờ bằng chứng xử lý trước stage3 commit. Cần review độc lập sau commit, ngoài kiểm reliability/cancel/resume/checksum và integration UI.
