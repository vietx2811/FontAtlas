# Settings → Bật font tự động: các app khác trình bày thế nào (truy cập 2026-11-11)

Chỉ ghi điều đã thấy trong kết quả tìm kiếm; chỗ chưa kiểm chứng ghi rõ.

| App | Cách trình bày | Nguồn |
|---|---|---|
| RightFont | Settings có ngăn "Auto Activation": danh sách app được hỗ trợ ở bên trái, bật cho từng app; có "Silent Mode" (bật font nền, không hộp thoại); chọn có tắt font tự bật khi thoát RightFont hay không. Hướng dẫn nhắc quyền macOS Automation cho các app. | [rightfontapp.com/docs/auto-activate-missing-fonts](https://rightfontapp.com/docs/auto-activate-missing-fonts), [rightfontapp.com/docs/settings](https://rightfontapp.com/docs/settings) |
| Typeface | Settings → Activation: công tắc theo từng app; InDesign, InCopy, Illustrator, QuarkXPress cần cấp quyền MỘT LẦN (nút Allow), có mục "Other apps" cho phần còn lại. InDesign dùng kích hoạt tạm, font chỉ thấy trong app đang hỏi. | [typefaceapp.com/help/articles/auto-activation](https://typefaceapp.com/help/articles/auto-activation) |
| Extensis / Insider (FontAgent) | Plugin Adobe cài riêng cho từng app; tài liệu hướng dẫn cài và bật plugin, và cảnh báo khi tắt auto-activation. | [fawin.docs.insidersoftware.com/kbase/using-adobe-and-quark-auto-activation-plugins](https://fawin.docs.insidersoftware.com/kbase/using-adobe-and-quark-auto-activation-plugins/), [help.extensis.com](https://help.extensis.com/hc/en-us/articles/12133832754203) |

## Điều rút ra cho Font Atlas
- Danh sách theo app, mỗi hàng có trạng thái riêng, đúng kiểu RightFont/Typeface; thêm nhãn **bằng chứng** trung thực (đã kiểm với app thật / một phần / chưa kiểm) vì 06 mới đo một phần.
- Việc cần quyền hay ghi vào máy (cài panel/script) luôn có bước xác nhận nói rõ sẽ ghi gì, như Typeface hỏi quyền một lần.
- Hai trình quản lý cùng chạy có thể tranh nhau: RightFont/Typeface/FontBase/Suitcase/FontExplorer X đang chạy thì báo, không tắt hộ.
- Chưa kiểm chứng: vị trí chính xác của cài đặt trong từng bản RightFont/Typeface; tài liệu Typeface về Adobe có chỗ mâu thuẫn về việc font có thấy ở app khác hay không.
