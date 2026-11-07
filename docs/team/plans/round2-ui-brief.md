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

## Brief C/D theo hợp đồng backend đã duyệt

- Similar giữ panel hiện có, mỗi family một hàng/card có tên family, style đại diện + số kiểu, badge nguồn, lý do ngắn. Hai hành động “Mở font” chuyển sang specimen của representative thật, “Xem family” hiện danh sách faceKeys backend trả trong library, đóng panel; sidebar vẫn đưa về view thường được. Phân trang 6 family, tổng và offset backend; loading/error/retry/empty riêng, bỏ phản hồi cũ khi đổi face/filter.
- Filter mở từ nút “Lọc” cạnh toggle Việt ở toolbar, popover 360px giới hạn chiều cao viewport và cuộn, không thay layout/sidebar/detail. Chia Loại font và Số đo, body 13px, nhãn 12px. Checkbox variable/italic; mỗi trục có 2 input số + 2 range control có accessible label khác nhau. Domain lấy `axis_stats`: mặc định focus p1–p99, nút “Toàn khoảng” mở min–max; input số luôn nhận outlier trong min–max; constant/empty vô hiệu hóa range có giải thích ngắn. Không padding domain giả.
- Điều kiện số raw inclusive AND qua query backend; missing không khớp range; reset và số điều kiện hiện cạnh trigger. Giữ lọc khi đổi sidebar như query hiện có. Bộ lọc được áp dụng vào library/count/Similar, không chỉ hàng đang hiển thị. Error stats có retry; no-match có thao tác xóa lọc.
- Shared formatter dùng unit từ backend, số thập phân Việt; query luôn giữ số vật lý không đổi %.

## Quyết định sau visual review của Lead

- Số đo/input dùng vi-VN: deg 1 số lẻ; em và mọi ratio 3 số lẻ. Tooltip anatomy giữ nguyên rawValue/method. Input nhận cả dấu phẩy và chấm; query dùng số đã làm tròn đang hiển thị.
- Domain focus/full được làm tròn biên ra ngoài theo cùng precision. “Toàn khoảng” chọn [floor(min), ceil(max)] để inclusive raw filter giữ cả font biên. Constant vẫn không cho kéo slider, khoảng chọn bao quanh số raw theo precision. Test bao biên -10.04..15.04 → -10.1..15.1.
- Stress trong Core luôn estimated khi đo được (`o_harmonic_fit`); mock đã khớp và UI giữ nhãn Ước lượng từ quality. Không sửa analyzer.
- WindowShell có `data-native`: chỉ browser mock (`false`) thêm nền sheet để sidebar tối đọc được. Native vẫn transparent cho vibrancy. Theme dùng color-scheme đúng sáng/tối cho scrollbar.
- Detail trở về đầu khi đổi font/tab/style; có nút đóng. Cả Mở font/Xem family có focus-visible 2px accent.

## Follow-up sau review độc lập

Lead duyệt ngày 2026-11-06: discovery/stats chỉ dựa metadata + filters/viOnly, bỏ membership family/Gần đây/Đang bật. Library vẫn dùng faceKeys. Chuyển trang theo scope mới về 0 vĩnh viễn; nếu total giảm làm trang rỗng thì quay về trang hợp lệ.

Giữ filters/viOnly khi Xem family để tôn trọng ý định lọc đang có. Nhưng membership lấy TOÀN family bằng getFamilyStyles không kèm filter, dedupe named instances theo FaceKey, kiểm giới hạn 2000. Chip riêng hàng trong TopBar “Family: X · có lọc”, tooltip n/N kiểu, nút × thoát scope; nới/bỏ lọc sẽ thấy lại các kiểu còn lại.

Giữ stats/draft/full mode khi refetch. Cận đã chọn ngoài domain mới vẫn hiển thị nguyên, blur không sửa là no-op. Slider chỉ commit cuối pointer/key gesture; aria-valuetext có unit. Checkbox mặc định bao focus bằng floor/ceil; nhập cận dưới floor, cận trên ceil. Trạng thái dữ liệu thật (quét/rỗng/thiếu quyền) ưu tiên hơn thông báo không khớp bộ lọc. Lỗi refresh onboarding giữ cùng lỗi từng thư mục. Chưa bổ sung hủy batch folder giữa chừng theo phạm vi Lead.

## Downloader mới — brief cuối Round 2

Nhập nguyên checkpoint 357ec1b, không sửa Rust/bindings. Giữ tokens/font/spacing hiện có. Dùng cùng deriveModelUi cho sidebar, Settings, onboarding và Similar: transfer có số MB/s (decimal), phần trăm tối đa99 trước khi đủbyte; done==total>0 chuyển text “Đang kiểm tra tệp…” và spinner không xác định, chỉ Ready xác nhận sẵn sàng. Channel cập nhật cả done/total, không lấy max qua hai phase; snapshot cũ không đè event mới.

Thông báo theo ErrorInfo.code trong contract; message kỹ thuật chỉ ở details/summary “Chi tiết” mặc định đóng. HTTP429 có Retry-After nếu đọc được, cancelled là info với “Tiếp tục tải”; lỗi khác có “Thử lại”. O6 Tải ngay bắt đầu ngay tại bước model, cho xem tiến độ/lỗi và bấm Tiếp tục để chuyển bước trong lúc tải nền. Không tải model thật trong QA; dev hash thêm checking/tls/cancelled cho mock.
