# A2: "Luôn giữ trên máy" và theo dõi thay đổi thư mục cloud

Người viết: 02 · 2026-11-12 · Chưa code. FSEvents đo trên thư mục tạm; KHÔNG chạm ổ cloud thật hay font của chủ app. **[ĐO]** đo thật · **[SUY]** suy luận, chưa kiểm · **[CHỦ APP]** chỉ chủ app kiểm được (có checklist cuối file).

## (a) Gợi ý "Luôn giữ trên máy"
1. Mỗi dịch vụ có nút riêng, tên khác nhau: OneDrive "Always Keep on This Device", Google Drive "Available offline", Dropbox "Make available offline", Box "Make Available Offline", iCloud Drive "Keep Downloaded" (Finder). Tất cả đặt trên thư mục trong Finder (chuột phải).
2. Font Atlas KHÔNG tự đặt cờ này: nó là quyết định của người dùng về dung lượng ổ đĩa, nằm ở phía dịch vụ, và muốn đặt phải ghi thuộc tính vào thư mục của chủ app (vi phạm "không chạm file người dùng"). Chỉ GỢI Ý bằng chữ, đúng tên nút của dịch vụ đó (`cloud_provider_of` đã nhận ra provider).
3. Khi hiện: nguồn là thư mục cloud (`CloudProvider`), có font chưa tải (`notDownloaded > 0`) HOẶC người dùng vừa bấm "Tải về máy". Dòng gợi ý nằm cạnh dòng "N chưa tải về", dùng NoteMark kiểu info, có thể tắt cho nguồn đó (nhớ lựa chọn, không hỏi lại).
4. Hiệu quả: "Tải về máy" (0.3.1) tải một lần; "Luôn giữ trên máy" ngăn dịch vụ gỡ lại khi ổ đầy, nên font không quay lại trạng thái placeholder.

## (b) Theo dõi thay đổi bằng FSEvents
5. **[ĐO]** FSEvents (`kFSEventStreamCreateFlagFileEvents`, latency 0,3 s) báo đệ quy theo từng file, kể cả thư mục con sâu; tạo file ra chuỗi `Created|XattrMod` cho thư mục cha rồi sự kiện của file; ghi/đổi tên/xoá đều có; xoá = `Removed`, đổi tên = hai sự kiện `Renamed` (tên cũ, tên mới). Cờ cộng dồn nên một dòng có thể mang `Renamed|Modified`; không dựa vào một cờ đơn lẻ.
6. **[ĐO]** Đổi cờ BSD (`chflags`) và xattr đều ra `Modified|XattrMod`, không tách riêng. Dataless (`SF_DATALESS`) là cờ BSD nên [SUY] việc tải xong cũng ra dạng này; không phân biệt được với sửa nội dung từ sự kiện, vì vậy sự kiện chỉ để ĐÁNH THỨC, còn quyết định luôn dựa vào `stat`.
7. **Thiết kế**: chỉ theo dõi nguồn cloud đang bật; gom sự kiện theo nguồn (debounce 2 s, tối đa 30 s), rồi xếp MỘT lượt quét của nguồn đó bằng `enqueue_follow_up` (đã có ở 0.3.1: không xếp trùng, không bỏ lỡ lượt sau lượt đang chạy). Quét vẫn là nguồn sự thật (so size/mtime/cờ dataless), sự kiện không chứa dữ liệu. Bỏ qua sự kiện của thư mục `.`-ẩn, file tạm `*.sb-*` và đuôi không phải font. Khi dịch vụ ngắt kết nối (ổ không gắn) thì dừng luồng và quét lại lúc có lại.
8. **Phụ thuộc (Lead duyệt 2026-11-12): crate `notify` =8.2.0**, `default-features = false`, chỉ bật `macos_fsevent`. Lý do: ngắn, được dùng rộng rãi, backend macOS là FSEvents chuẩn (đúng thứ đã đo ở mục 5), có test; tự gọi CoreServices bằng `objc2` dài hơn và phải tự lo vòng đời luồng/CFRunLoop. Chọn 8.2.0 vì là bản ổn định mới nhất (9.x còn rc); khoá `=` như `blake3`/`ts-rs` để cập nhật là quyết định có chủ ý. Hệ quả trong `Cargo.lock`: thêm `notify`, `notify-types`, `fsevent-sys` (macOS) và các phụ thuộc theo nền tảng khác của nó (inotify, kqueue, mio…, không biên dịch trên macOS); `bitflags` nhích 2.10.0 → 2.13.2 vì `kqueue-sys` đòi bản mới (cùng major, tương thích). Không có crate đã dùng nào đổi major.
9. Test được trên thư mục tạm (không cần ổ cloud): tạo/xoá/đổi tên font OFL thì có đúng một lượt quét sau debounce; 100 sự kiện dồn trong 1 s chỉ ra một lượt quét; nguồn tắt thì không có luồng; thư mục biến mất thì dừng không panic.

## Phụ thuộc nhánh
- Cần `enqueue_follow_up` của 0.3.1: nhánh `feat/cloud-watch-keep` xếp trên `int/0.3.1` (d33eede9); 0.3.1 vào dev/mvp nguyên trạng nên rebase sau này gần như không có gì.

## Đã làm (code)
- `services/cloud_watch.rs`: watcher theo nguồn cloud đang bật; lọc sự kiện theo tên (font, stub `.icloud`, thư mục; bỏ ẩn/tạm); gom debounce 2 s, tối đa 30 s; một lượt `enqueue_follow_up` cho mỗi cụm. Đồng bộ khi mở app, thêm/xoá nguồn; dừng khi thoát. Đã đo trên thư mục tạm (FSEvents thật, `is_cloud` giả): font mới hiện, font xoá biến mất, 30 file dồn một lúc = đúng một lượt quét, file không phải font không gây quét.
- Chỗ `/var` → `/private/var` và thư mục tạm bắt đầu bằng `.`: FSEvents báo đường dẫn thật, nên bộ lọc so cả đường dẫn gốc lẫn dạng `canonicalize`, và chỉ xét phần BÊN DƯỚI gốc.

## [CHỦ APP] Checklist tự thử trên OneDrive thật (mất ~10 phút, dùng thư mục thử, KHÔNG dùng thư mục font chính)
1. Trong OneDrive tạo thư mục thử `FontAtlas-test` có 2–3 font OFL (Open Sans, Roboto…), đợi đồng bộ xong, chuột phải thư mục → "Free Up Space" để chúng thành placeholder (biểu tượng đám mây).
2. Font Atlas: thêm thư mục đó làm nguồn. Mong đợi: thẻ font hiện dấu "Trên cloud", sidebar có dòng "N chưa tải về".
3. Mở menu nguồn → "Tải về máy". Mong đợi: tiến độ x/y, xong thì font hiện bản xem trước thật và dòng "chưa tải về" biến mất.
4. Chuột phải thư mục → "Always Keep on This Device". Mong đợi: lần sau "Free Up Space" ở thư mục cha không làm font quay lại placeholder.
5. (Theo dõi, sau khi có code) Khi Font Atlas đang mở, thêm một font mới vào thư mục thử từ máy khác hoặc từ web OneDrive. Mong đợi: trong khoảng 1 phút font tự hiện ở Font Atlas, không cần bấm quét lại. Ghi lại: bao lâu mới hiện.
6. Xoá font ở bước 5 trên web OneDrive. Mong đợi: font biến mất khỏi thư viện sau lượt quét tự động.
7. Báo lại kết quả từng bước (đạt/không + ghi chú); bước nào không đạt chụp màn hình dòng trạng thái.
