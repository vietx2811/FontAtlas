# Round 2 UI — brief và nghiệm thu

Ngày: 2026-11-06. Baseline: 0607f74, worktree `round2-ui`.

## Phạm vi và hướng đã duyệt

Người dùng quản lý thư viện font cho công việc thiết kế, cần thêm các thư mục độc lập rồi đọc số đo từng font. Giữ khung sidebar/library/detail, tiếng Việt và hướng visual hiện có. Lead duyệt brief qua Paseo trước triển khai.

- Miền nội dung: nguồn font, family, glyph, x-height, đường nền, phép đo, chất lượng đo.
- Màu: giữ màu mực `fg`, giấy `sheet`, nền `card`/`surface-2`, đường phân cách `border`, `accent` và trạng thái `warn` hiện có; không thêm palette.
- Dấu hiệu riêng: minh họa glyph và đường nền hiện có; hàng anatomy dùng nhãn phép đo và số có đơn vị.
- Thay các mặc định không phù hợp: không dùng wizard toàn màn rộng → cột đọc 560px; không dùng thang đánh giá thay số đo → số chính, percentile phụ; không ẩn nguồn sau import → giữ bước nguồn để bổ sung.

## Layout và trạng thái

Intent: thao tác nhanh với thư mục thật và số đo rõ nghĩa; cảm giác công cụ native yên tĩnh. Depth: border + surface hiện có. Typography: Inter UI hiện tại, h1 26px/600, nội dung 14px, metadata 12px; số tabular. Spacing: bội 4px, nhóm 8/16/24px.

- Onboarding rộng tối đa 560px, lề viewport 24px. Danh sách nguồn cao tối đa 240px, cuộn nội bộ; tên nguồn 13px, đường dẫn 12px có wrap và tooltip đầy đủ. Nội dung tổng thể cuộn để luôn tới CTA ở 1000×700.
- Cho chọn nhiều thư mục trong native dialog, xử lý tuần tự qua `addSource`; không đổi backend, không tự thay nguồn cũ. Hủy không lỗi; nguồn trùng/lồng nhau thông báo ngắn, giữ dữ liệu cũ. Trạng thái chọn/thêm/xóa khóa submit lặp và điều hướng. Nguồn đang quét/chờ quét không cho xóa; lỗi `busy` từ backend vẫn phải hiện.
- Tải nguồn lỗi có thử lại; scan status/events lấy từ API thật; nguồn không khả dụng hiện trạng thái, không tự suy đoán là đã cấp quyền.
- Anatomy: icon 16px, nhãn co giãn và wrap, số căn phải; chất lượng và percentile ở hàng phụ. Số có unit đã đối chiếu Rust; không dựng confidence %. Không có giá trị → “Chưa đo”, failed → lý do. Chỉ vẽ guide có giá trị thật.

## Quy ước dữ liệu

`onboarding_step=done` không bị reset; step đang dở tiếp tục như cũ. O3 luôn có, kể cả sau legacy import (Lead chấp thuận cập nhật quy ước cũ). API `addSource` tự enqueue scan; `removeSource` có thể báo busy. Không tự pause/cancel scan để xóa nguồn.

Baseline Rust `analysis.rs` trả tên trục canonical từ `AxisId`; mock cũ khác tên, phải đồng bộ. Baseline đổi `serif_foot` thành string, cần Backend giữ số trước khi tuyên bố số chân serif. `AxisReadingDto` không có confidence số. C/D/E chờ hợp đồng backend, không giả field.

## Kiểm tra dự kiến

Vitest onboarding: nhiều folder, nguồn sẵn/legacy, cancel, duplicate/nested, permission/error, busy, reload step, scan và navigation. Anatomy: canonical keys, unit/method, zero/negative, missing/failed/estimated, guide thật. `tsc --noEmit`, Vitest phù hợp, `npm run build`, `git diff --check`. Paseo browser tại :3012, 1400×920 và 1000×700, sáng/tối; ghi rõ dữ liệu mock, không có native IPC/activation thật.
