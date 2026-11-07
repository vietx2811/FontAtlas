# Kiểm định analyzer — bằng chứng ban đầu, 2026-11-06

Phạm vi: chỉ hai fixture trong `UX/src-tauri/crates/fontatlas-core/tests/fixtures`, baseline `core-0.3.0`. Không suy rộng thành kiểm định mọi font/axis. Chưa thay công thức.

## Đã tái lập

Chạy `target/release/examples/dump` trên Libre Baskerville và Source Code Pro; JSON tạm ở `/private/tmp/fontatlas-analyzer-audit.json`. Dùng Python fontTools 4.61.1, `TTFont`, `BoundsPen` để đọc outline x và OS/2 độc lập với Rust.

| Fixture | Core x_height_em | Outline x top / unitsPerEm | OS/2.sxHeight / unitsPerEm | Core contrast | Core stress |
|---|---|---|---|---|---|
| libre_baskerville.ttf | 0.530 | 530/1000 | 530/1000 | 2.2823281 | 0°, estimated |
| source_code_pro.ttf | 0.486 | 486/1000 | 486/1000 | 1.2512604 | null, failed/low_contrast |

SHA-256 kiểm trực tiếp khớp fixture README: Libre `f5bc4341f15de5e877d8d95b6b14b33e9a3da1f8fef4ed0700ed407f096cffb6`; Source Code `66a11c481af5bdcdb91a72282bdb0b9bb5992208a72d89e1dd393e5e8255c7a2`. Core `hash_hex` là BLAKE3, không so trực tiếp với SHA-256. Lead đã đính chính cho Backend một ghi chú so nhầm hai loại hash, không có mismatch fixture.

## Đọc mã và giới hạn

- `stress_deg` luôn gắn Estimated khi có số; low contrast trả failed. Kết quả Libre 0° tái lập được, chưa đủ chứng minh sai; cần nhãn góc độc lập và font tổng hợp có stress biết trước. Chưa được UI trình bày như đo chính xác đã kiểm định.
- `a_storeys/g_storeys` dùng số đoạn mực trên một đường dọc giữa bbox, là heuristic hình học. FontTools đếm g Source Code có 3 closed contours; không dùng số contour đơn lẻ làm nhãn ground truth. Chưa xác nhận bằng mắt fixture này.
- Đã tạo proof outline fontTools trực tiếp ở `analyzer-g-proof.html` (cùng thư mục). Paseo snapshot đọc được nội dung nhưng screenshot báo chưa paint; vì vậy không coi là bằng chứng visual đã duyệt.
- Quan sát pipeline thật: Rust axes dùng x_height_em/stem/width/spacing... trong khi mock/UI cũ dùng x_height/stem_weight/avg_width..., và `services/analysis.rs` đổi serif_foot raw thành class string. Hai worker đang sửa theo hợp đồng dưới đây.

## Hợp đồng Lead đã duyệt, chờ triển khai

Backend thêm `AxisReadingDto.rawValue: number|null`, `unit: string|null`, giữ `value` legacy/class và quality/reason/method; bổ sung cap_height_em, descender_em từ Core thật. UI hiển thị rawValue, đơn vị/độ tin cậy; không suy ra missing. Descender giữ số âm theo Core.

Similar families: phân trang theo family, loại family nguồn, stable tie FaceKey, không dùng cap pool heuristic. `axis_stats(q)` thống kê theo metadata filters, bỏ axis/hard/paging để domain không tự co; min/max đầy đủ và p1/p99 focus. `rawAxisRanges` inclusive vật lý, AND, invalid args phải báo lỗi. Chi tiết ở nhánh backend `docs/team/plans/round2-backend-contract.md`; Frontend đã nhận cùng hợp đồng.

Tiếp theo: reviewer độc lập kiểm phương pháp với fixture tổng hợp/nhãn tay khi có worker slot; kiểm raw-to-DTO-to-UI và tương tác lọc trên backend thật trước nghiệm thu.
