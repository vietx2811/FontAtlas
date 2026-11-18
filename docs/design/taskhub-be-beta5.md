# TaskHub BE — thiết kế beta.5 (chưa code)

Mục tiêu: một nơi duy nhất cho mọi việc nền (quét, phân tích, tải cloud, nhúng, kích hoạt hàng loạt…) để FE vẽ khay "Tác vụ" ở sidebar. Nút ẩn khi rảnh, không giữ lịch sử.

## DTO `TaskDto`
`{ id: string, kind: TaskKind, titleKey: string, titleArgs: Record<string,string|number>, done: number, total: number|null, state: "queued"|"running"|"cancelling", cancellable: boolean, startedAt: number }`
- `TaskKind`: scan | analyze | embed | cloudDownload | activation | export | other.
- `total=null` = không xác định (thanh chạy). Xong/huỷ/lỗi → task biến mất (event `removed`, kèm `outcome: "done"|"cancelled"|"failed"` để FE báo toast một lần). Không lưu lịch sử.

## Events
`task://upsert(TaskDto)` (throttle ≤4 Hz/task, bắt buộc phát khi đổi state), `task://removed {id, outcome, errorKey?}`.

## Commands (async, không giữ Mutex store)
- `list_tasks() -> TaskDto[]` — đọc từ `TaskHub` (Mutex riêng, không chạm store).
- `cancel_task(id)` — gọi `CancelToken` của task; idempotent; trả ngay.

## BE
`TaskHub { tasks: Mutex<BTreeMap<TaskId, Entry>> , emit }`; `Entry` giữ DTO + `Option<Arc<dyn Fn()>>` cancel. Dịch vụ đăng ký bằng `hub.begin(kind, title_key) -> TaskHandle` (RAII: drop = removed). `ScanManager` bọc lại: Queued→queued, Progress→done/total, Finished/Idle→removed. Giữ nguyên ScanEvent cho tương thích; TaskHub phát song song, sau đó FE bỏ dần.

## Việc đang chặn UI hoặc chạy đồng bộ trong command (cần rà, đã thấy)
- Tất cả store command là async qua `spawn_blocking` nhưng dùng chung 1 `AppState.store` Mutex → mọi truy vấn nặng chặn các lệnh khác (đã sửa list_sources; còn name_clashes ~53 ms, duplicate_summary nguội, query_faces lớn).
- `services/cloud.rs` (6 chỗ spawn), `cloud_download.rs` (2): tải/evict cloud chạy thread rời, chưa có tiến độ chung.
- `activation.rs`: kích hoạt hàng loạt / Case.
- Quét: tiến độ qua ScanEvent riêng; nhúng/analyze trong batch quét.
- Export / gói font (cần rà thêm khi code).

## Thứ tự làm
1. `TaskHub` + list/cancel + bọc scan (nhỏ nhất, FE thấy ngay). 2. cloud_download. 3. activation/export. 4. Tách đọc khỏi Mutex store (connection đọc riêng, WAL) nếu số đo còn chặn.

## Quyết định (Lead, 2026-11-19)
- Chỉ có cancel, không có pause_task ở beta.5 (state `paused` bỏ khỏi DTO).
- Cancel phải an toàn: task ghi theo từng đơn vị nguyên tử (một file/nhóm một lần). Resolve duplicates dừng giữa chừng → phần đã làm vẫn có undo/Trash, phần chưa làm giữ nguyên; huỷ chỉ kiểm tra token giữa các đơn vị, không bỏ dở một đơn vị. Cần test: huỷ ở giữa → danh sách undo khớp đúng phần đã làm.
- Khay ẩn khi rảnh, không lưu lịch sử.
- Chưa code cho tới khi beta.4 phát hành.
