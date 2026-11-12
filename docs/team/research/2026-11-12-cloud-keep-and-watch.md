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
8. Phụ thuộc: chưa có watcher nào trong app. Hai cách: (i) crate `notify` (backend FSEvents trên macOS), (ii) gọi thẳng CoreServices FSEvents qua `objc2`/`core-foundation`, đã có `objc2` trong app. (i) ngắn và có test, nhưng thêm phụ thuộc mới vào Cargo.lock (cần chủ app/Lead duyệt); (ii) không thêm crate nhưng dài hơn. **Cần Lead chọn.** Đề xuất (i), khoá phiên bản chính xác như `blake3`/`ts-rs`.
9. Test được trên thư mục tạm (không cần ổ cloud): tạo/xoá/đổi tên font OFL thì có đúng một lượt quét sau debounce; 100 sự kiện dồn trong 1 s chỉ ra một lượt quét; nguồn tắt thì không có luồng; thư mục biến mất thì dừng không panic.

## Phụ thuộc nhánh
- Cần `enqueue_follow_up`, `cloud_placeholders`, `CloudJobs` của 0.3.1 (nhánh `fix/cloud-source-actions`). Nhánh này cắt từ dev/mvp hiện tại (chưa có 0.3.1); sẽ rebase khi 0.3.1 vào dev/mvp, chưa viết code tới lúc đó.

## [CHỦ APP] Checklist tự thử trên OneDrive thật (mất ~10 phút, dùng thư mục thử, KHÔNG dùng thư mục font chính)
1. Trong OneDrive tạo thư mục thử `FontAtlas-test` có 2–3 font OFL (Open Sans, Roboto…), đợi đồng bộ xong, chuột phải thư mục → "Free Up Space" để chúng thành placeholder (biểu tượng đám mây).
2. Font Atlas: thêm thư mục đó làm nguồn. Mong đợi: thẻ font hiện dấu "Trên cloud", sidebar có dòng "N chưa tải về".
3. Mở menu nguồn → "Tải về máy". Mong đợi: tiến độ x/y, xong thì font hiện bản xem trước thật và dòng "chưa tải về" biến mất.
4. Chuột phải thư mục → "Always Keep on This Device". Mong đợi: lần sau "Free Up Space" ở thư mục cha không làm font quay lại placeholder.
5. (Theo dõi, sau khi có code) Khi Font Atlas đang mở, thêm một font mới vào thư mục thử từ máy khác hoặc từ web OneDrive. Mong đợi: trong khoảng 1 phút font tự hiện ở Font Atlas, không cần bấm quét lại. Ghi lại: bao lâu mới hiện.
6. Xoá font ở bước 5 trên web OneDrive. Mong đợi: font biến mất khỏi thư viện sau lượt quét tự động.
7. Báo lại kết quả từng bước (đạt/không + ghi chú); bước nào không đạt chụp màn hình dòng trạng thái.
