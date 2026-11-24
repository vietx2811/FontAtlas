# Hợp đồng mã lỗi embedding (04 → 05, 2026-11-14)

Nhánh `feat/embed-error-codes` (từ `1d85d98e`). Chỉ **hai mã mới** cho phần model/worker; không đổi hình dạng DTO
(`ErrorInfoDto = { code, message }`, `embedStatus.loadError` vẫn là chuỗi mã). `message` là chi tiết kỹ thuật, không dịch.
Phần mạng khi tải model (`http`, `tls`, `network`…) giữ nguyên theo `round2-backend-contract.md`.

Mục tiêu của 05: viết thông báo vi + en, mỗi mã một câu "chuyện gì" + một câu "làm gì". Chỉ tách theo việc người dùng làm khác nhau.

| Nhóm | Mã | Ý nghĩa | Hành động gợi ý cho người dùng | Tự thử lại |
|---|---|---|---|---|
| A. File model hỏng/thiếu | `model_file` **(mới)** | Không nạp được file model: header safetensors hỏng, thiếu tensor, sai shape, không đọc được file | **Tải lại model** (nút Tải lại/Redownload) | Không |
| A. | `checksum` (có sẵn) | Dữ liệu tải về không khớp bản phát hành | **Tải lại model** | Không |
| B. Hết tài nguyên | `resources` **(mới)** | Hết bộ nhớ hoặc bộ nhớ GPU khi nạp/chạy model | **Đóng bớt ứng dụng rồi thử lại** (bật lại Embedding/Tiếp tục) | Không (người dùng bấm) |
| C. Font không đọc/render được | `missing_file`, `stale_file`, `render_failed`, `no_glyphs` (có sẵn, theo từng face) | Face đó không đọc được, đã đổi sau khi quét, không render được, hoặc thiếu ký tự mẫu | **Không cần làm gì**: face bị đánh dấu và bỏ qua, các face khác vẫn chạy; hiển thị ở danh sách "chưa có ảnh nhúng" nếu có | `missing_file` tự thử lại khi file có lại |
| D. Lỗi nội bộ khác | `inference` (có sẵn) | Lỗi khi chạy model không thuộc A/B/C (kích thước tensor, whitening, kết quả không hợp lệ, worker panic) | "Có lỗi nội bộ, thử khởi động lại ứng dụng; nếu lặp lại hãy báo lỗi kèm chi tiết" | Không |
| D. | `store`, `io` (có sẵn) | Lỗi đọc/ghi thư viện hoặc đĩa | Kiểm tra ổ đĩa còn chỗ và quyền truy cập | Không |
| (trạng thái) | `disabled`, `cancelled` (có sẵn) | Người dùng tắt embedding / dừng | Không phải lỗi: không hiện như lỗi | Không |

## Nơi mã xuất hiện

- Tải model: `ModelEvent::Failed.error.code` và `model status.lastError.code` (`checksum`, mạng, `model_file` nếu header tải về hỏng).
- Nạp model khi khởi động worker: `embedStatus.loadError` (`model_file`, `resources`, `inference`).
- Worker dừng giữa chừng: `loadError` mang đúng mã của lỗi (từ D14, không còn gắn cứng `inference`).
- Mã C nằm theo từng face trong `pipeline_state.error_code`, không phải lỗi toàn cục.

## Cách phân loại (để 05 hiểu giới hạn)

- `model_file`: mọi lỗi của candle lúc **nạp** trọng số trừ hết tài nguyên (file thiếu, cụt, tensor/shape sai), cùng lỗi header safetensors lúc ghép file tải về.
- `resources`: lỗi nạp hoặc chạy do hết bộ nhớ: `io::ErrorKind::OutOfMemory`, hoặc Metal báo "Insufficient Memory" / "Failed to create metal resource".
  Phần Metal khớp theo **chuỗi** vì candle không có kiểu lỗi riêng; một thông điệp khác có thể rơi vào `inference`.
  Hết RAM CPU thường làm tiến trình bị hệ điều hành dừng chứ không trả lỗi: `resources` chủ yếu bắt lỗi GPU.
- `inference`: phần còn lại, gồm lỗi lúc chạy (kích thước tensor sai, đầu ra sai) và sai cấu hình whitening.
