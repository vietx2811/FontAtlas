# Round 2: tốc độ tải model

Thay đổi trong `fontatlas-embed/src/model.rs`:
- Redirect chỉ giải quyết một lần: URL cuối của lần probe header (206) được nhớ trong `Http.cdn`; các Range sau đi thẳng tới CDN. CDN trả 401/403/410 thì bỏ URL đó và giải lại từ URL manifest (một lần cho mỗi request). Chỉ chấp nhận URL cuối cùng cùng scheme hoặc https.
- Cửa sổ trượt: `buffered(CONCURRENCY)` giữ tối đa 3 request đang bay, trả kết quả đúng thứ tự, tối đa 3 chunk trong RAM (48 MiB). Hủy = drop stream.
- `RANGE_CHUNK` 4 -> 16 MiB (đo 8/16/32 MiB chênh trong nhiễu mạng). Checkpoint vẫn chỉ ghi phần liền mạch đã bền.
- Test hook `with_range_chunk_for_test` (fixture cũ dùng 4 MiB).

Đo (đường của owner, 371.6 MB, gồm SHA):
- Trước: 62.6 s, 65.7 s (5.7-5.9 MB/s).
- Sau (16 MiB): 47.0 s, 43.2 s, 42.0 s, 41.9 s (7.9-8.9 MB/s). 8 MiB: 45.6 s; 32 MiB: 42.9 s.
- curl một luồng cùng URL: 37.8 s (9.8 MB/s), chỉ phần vision, không SHA.

Giới hạn: feed+checkpoint (SHA, ghi, fsync) chạy đồng bộ trên runtime current-thread, chặn I/O ~2.3 s / 42 s; head-of-line: một chunk chậm chặn cửa sổ sau 3 chunk (đúng yêu cầu bộ đệm có biên).
