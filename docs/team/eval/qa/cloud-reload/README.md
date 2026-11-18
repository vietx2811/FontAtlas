# Tái hiện: lưới trống / tải lại không rõ lý do / thanh bên chậm khi file cloud «tải xong» (07 QA, 2026-11-19)

Bản: DEBUG `85384aac` (bundle do `local_ci` dựng, chép sang thư mục thử). `FONTATLAS_DATA_DIR` mới trên ổ dữ liệu, chỉ font fixture của repo (tên duy nhất `FAQ…`), không dùng thư mục cloud thật, không chạy release, giữ khoá QA (`/tmp/fontatlas-native-qa.lock`). Máy lúc đo có CI cổng của Lead đang chạy: load average 4–9, nên mọi độ trễ là «máy bận vừa phải»; chiều so sánh vẫn đúng (xem cột nhỏ/lớn).

## Kết quả (thư viện 1512 font: nguồn `base` 1512 + nguồn `cloud` 61 file; đối chứng: 100 font + `cloud`)
| Kịch bản | Quét (queued→finished) | Tải lại thư viện | Lưới rỗng khi ô tìm rỗng (đếm thẻ DOM = 0) |
|---|---|---|---|
| 0. Không làm gì 45 s | 0 | 0 | 0 |
| A. 1 file cloud «tải xong» (stub `.icloud` → font thật) | 1 | 2 đợt (query_faces 8, list_sources 7, duplicate_summary 2) | 1 lần, **0,86 s** (đối chứng 100 font: 1,0 s) |
| B. 30 file liên tiếp, cách 1 s | 1 (watcher gom, chạy ở mốc `MAX_WAIT` 30 s) | 2 đợt | 1 lần, **3,0 s** (đối chứng: 0) |
| C. 30 file, cách 4 s (ghi đè file 0 byte bằng font thật) | **30** (queued 30, finished 30) | **30 đợt** (query_faces 240, list_sources 181, duplicate_summary 60, name_clashes 30) | **30 lần**, mỗi lần 1,8–3,2 s (trung vị 3,0 s), tổng **82,7 s trong cửa sổ 154,9 s (53 %)**; vùng cuộn bị gắn lại 37 lần (cộng dồn từ đầu lượt chạy; đối chứng 100 font: chỉ 2 lần rỗng ~1 s, 34 lần gắn lại) |
| D. 10 lần ghi đè file đã có bằng đúng nội dung cũ (chỉ đổi mtime), cách 4 s, KHÔNG có font mới | 10 | 10 đợt (query_faces 40) | **10 lần**, 1,0–2,2 s, tổng 19,5 s trong 65 s |

Khi lưới rỗng: thanh trên vẫn ghi đúng tổng (ví dụ «1573»), vùng cuộn tồn tại (`library-grid-scroller`, `scrollTop=0`) nhưng không có hàng thẻ nào (`face-name` = 0), không có thông báo rỗng. Thanh bên (Sources, All, Duplicates) KHÔNG nhấp nháy sau khi đã hiện (0 mẫu rỗng).

Độ trễ IPC (p50) trong lúc quét/tải lại, thư viện 1512: `list_sources` 1,9–3,1 s, `query_faces` 0,2–2,3 s, `duplicate_summary` 1,8–3,3 s, `name_clashes` ~1 s; lúc yên (kịch bản 0) `list_sources` 0,48 s. Thư viện 100 font: 25–130 ms. Thời gian lưới rỗng tăng theo độ trễ này (sàn ~1 s).

### Khởi động (4 lần, cùng dữ liệu, thư viện 1512, quét lại 2 nguồn khi khởi động)
| | lần 1 | 2 | 3 | 4 |
|---|---|---|---|---|
| Khung thanh bên hiện (chưa có số) | 0,59 s | 0,46 | 0,41 | 0,42 |
| Thanh bên có số All + Sources + mục Duplicates | **6,15 s** | 5,99 | 5,60 | 6,23 |
| Thẻ đầu tiên ở lưới | 6,15 s | 5,00 | 5,60 | 5,23 |
Lúc khởi động có 3 đợt nạp (t ≈ 0,84 / 1,08 / 1,22 s: tổng 9 `list_sources`, 16–17 `query_faces`, 4 `duplicate_summary`); các lệnh đầu trả sau 3,1–4,9 s (`list_sources` 3,1–4,9 s, `duplicate_summary` 4,9 s, `query_faces` 3,4 s). Có 2 lượt quét (queued/finished) ở mỗi lần khởi động.

## Nguyên nhân nghi ngờ (cho 02 BE / 05 FE, chưa sửa gì)
1. `UX/store/scan.ts`: sự kiện `finished` luôn `bumpNow()` (và `facesAdded` `bumpSoon()` mỗi 1,5 s) → `version` đổi → `Sidebar.refresh` (7 lệnh IPC trong một `Promise.all`, `Sidebar.tsx` ~dòng 102–115) và `LibraryGrid` nạp lại. Kịch bản D cho thấy lượt quét không đổi gì cũng bump và gây lưới rỗng.
2. Lưới bỏ nội dung cũ trong lúc chờ `query_faces` mới: thanh trên còn đúng tổng nhưng không có hàng thẻ. Nên giữ ảnh chụp cũ tới khi có trang mới.
3. Watcher (`cloud_watch.rs`): `QUIET` 2 s / `MAX_WAIT` 30 s; file về cách nhau > 2 s sinh MỘT lượt quét mỗi file (kịch bản C), mỗi lượt kéo theo một lần nạp lại đầy đủ.
4. Thanh bên (Sources, All, Duplicates…) chờ lệnh chậm nhất (`duplicate_summary` ~4,9 s lúc khởi động) vì chung một `Promise.all`.
5. `list_sources`/`duplicate_summary` chậm 2–5 s với 1,5 nghìn font trong khi quét/khởi động (đối chứng 100 font: dưới 130 ms): nghi tranh chấp khoá/việc tính đếm; chủ có thư viện lớn hơn nên sẽ tệ hơn.

## Chạy lại
```bash
Q=/Volumes/<ổ dữ liệu>/tmp-cloud-repro; mkdir -p $Q
python3 mkpool.py UX/src-tauri/crates/fontatlas-core/tests/fixtures UX/src-tauri/tests/qa/patch_font_names.py $Q/base 1500 b
python3 mkpool.py UX/src-tauri/crates/fontatlas-core/tests/fixtures UX/src-tauri/tests/qa/patch_font_names.py $Q/pool-cloud 61 c
python3 prep.py $Q && python3 resetcloud.py $Q
cp -R "<bundle debug>/Font Atlas.app" $Q/        # bản DEBUG của SHA cần thử (local_ci dựng ở target/debug/bundle/macos)
export RUN=$Q/run-main
Q=$Q bash run08.sh setup.js                      # thêm nguồn base + cloud, chờ quét xong
KEEP_DATA=1 Q=$Q bash run08.sh startup.js        # đo khởi động (lặp 3–4 lần)
KEEP_DATA=1 Q=$Q MAXWAIT=700 bash run08.sh scen.js   # A, B, C, D; ONLY="A B" để chạy một phần
```
`run08.sh` giữ khoá QA native, chỉ dừng PID của chính nó, ghi kết quả vào bảng `settings` của dữ liệu tạm (`ui.qa.NNN`) và in ra. Với đối chứng nhỏ: `ONLY=cloudonly` khi chạy `setup.js` (thêm `small100` thay vì `base`), `resetcloud.py` trước, rồi `ONLY="A B C"` cho `scen.js`.
Bộ đo (`lib8.js`): móc `fetch` (mọi lệnh IPC của app đi qua `ipc://`, ghi thời điểm + độ trễ), kênh `scan_subscribe` riêng (đếm queued/progress/facesAdded/finished/idle), thăm dò DOM 50 ms (số thẻ `face-name`, ô tìm, số trên thanh bên, mục Duplicates, chỉ báo đang quét). Giới hạn: không đặt được cờ `SF_DATALESS` thật nên «cloud» chỉ là stub `.icloud` và file 0 byte; kịch bản đo đúng luồng watcher → quét → bump → nạp lại, không đo việc tải thật.
