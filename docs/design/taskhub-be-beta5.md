# TaskHub BE — thiết kế beta.5 (chưa code)

Mục tiêu: một nơi duy nhất cho mọi việc nền (quét, phân tích, tải cloud, nhúng, kích hoạt hàng loạt…) để FE vẽ khay "Tác vụ" ở sidebar. Nút ẩn khi rảnh, không giữ lịch sử.

## DTO `TaskDto` (đã chốt sau góp ý 05)
```
{ id: string, rev: number, kind: TaskKind, trigger: "watcher"|"user"|"launch",
  titleKey: string, titleArgs: Record<string,string|number>,   // không chứa đường dẫn đầy đủ, chỉ tên nguồn/Case
  done: number, total: number|null, unit: "items"|"percent"|"bytes",
  state: "queued"|"running"|"cancelling", cancellable: boolean,
  cancelConfirmKey?: string,                                    // có = FE hỏi nhỏ trong dòng; không = huỷ ngay
  groupKey?: string,                                            // vd kind+nguồn; FE tự gộp khi >3 dòng cùng kind
  startedAt: number }                                           // epoch ms
```
- `TaskKind`: scan | analyze | embed | cloudDownload | activation | resolveDuplicates | export | other.
- `rev`: bộ đếm đơn điệu toàn hub, tăng ở mọi upsert VÀ removed. `list_tasks` trả `{ rev, tasks }`; FE subscribe trước rồi list, bỏ sự kiện có rev ≤ rev đã có (chống đua đồng bộ đầu).
- `total=null` = không xác định, không bao giờ dùng 0 làm "chưa biết".
- `trigger`: Watcher chưa biết có đổi hay không lúc queued; FE ẩn task watcher cho tới khi done>0 hoặc có facesAdded (tránh khay nháy khi quét không đổi).
- `cancelling` vẫn gửi done/total cuối để dòng không nhảy.
- Xong/huỷ/lỗi → `removed`, không lưu lịch sử. Lỗi báo một lần (toast ngắn + NoteMark "!"), không giữ dòng.

## Commands (async, không giữ Mutex store)
- `list_tasks() -> { rev, tasks: TaskDto[] }` — đọc từ `TaskHub` (Mutex riêng, không chạm store).
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
- Hub chỉ để HIỂN THỊ: `removed outcome:"done"` không kích bump thư viện. Bump vẫn theo `ScanEvent::Finished.changed` cho tới khi bỏ ScanEvent (tránh bump đôi).
- titleKey: BE liệt kê key; 05 thêm bản vi+en vào i18n.
