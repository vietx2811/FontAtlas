# Câu hỏi phỏng vấn nhà thiết kế — thư viện font, Hiển thị, sắp xếp, menu

Dành cho chủ app dùng với 2–3 người thiết kế (graphic designer, art director, layout artist). Liên quan: `spec.md` cùng thư mục. Ngày 2026-11-11.

Cách dùng: 20–30 phút mỗi người. Hỏi theo thứ tự, **đừng gợi ý đáp án**; xin họ mở thư viện font thật của họ (Font Book, FontBase, RightFont… tuỳ họ dùng) và làm cho xem. Ghi lại họ **làm gì**, không chỉ họ **nói gì**. Hỏi "lần gần nhất bạn làm việc đó là khi nào?" để lấy hành vi thật.

Trước khi bắt đầu: xin phép ghi chú; nói rõ đây là kiểm tra thiết kế, không phải kiểm tra họ; không cho xem bản FontAtlas trước các câu 1–4.

| # | Câu hỏi | Để quyết điều gì (mục trong spec) | Cách ghi |
|---|---|---|---|
| 1 | Khi duyệt font, ngoài tên và mẫu chữ, bạn muốn thấy thêm gì ngay trên mỗi ô? Sắp theo mức bạn cần thấy hằng ngày. Cái nào bạn chỉ cần khi mở chi tiết? | Danh sách thông tin trên thẻ và mặc định (A3) | Danh sách xếp hạng; trường họ tự nêu mà chưa có trong spec |
| 2 | Bạn tìm "font mới tôi vừa thêm" bằng cách nào? Bạn tìm "font cổ điển" hoặc "font đời mới" bằng cách nào? Nếu app cho sắp theo năm phát hành nhưng chỉ ước lượng (có thể sai, nhiều font không rõ), bạn thấy có dùng được không, hay bạn tự gắn thẻ thời kỳ? | Sắp xếp theo ngày thêm / năm phát hành / ngày tạo file (B5, Q4) | Hành vi hiện tại; họ có tin con số ước lượng không |
| 3 | Bạn kỳ vọng chuột phải vào một nguồn hoặc một dự án trong thanh bên có những gì? Có việc nào bạn mong làm được từ đó mà chưa nghĩ ra cách? | Menu Sidebar (C6); mục này không có nguồn công khai | Danh sách mục họ nêu; mục họ sợ bấm nhầm (xoá, gỡ) |
| 4 | Khi muốn xem các kiểu của một family: bạn thích thấy mọi kiểu xếp dưới tên family, hay chỉ một ô đại diện mỗi family rồi mở ra? Khi nào bạn muốn bỏ nhóm? | Nhóm theo Family (B5, Q2) | Chọn A/B và lý do; tình huống họ cần cả hai |
| 5 | Bạn dùng từ nào để ghi cảm xúc hoặc phong cách của một font (tiếng Việt hay Anh)? Có tag nào bạn hay dùng mà danh sách mood gợi ý (Business, Calm, Refined, Playful, Friendly/Cute, Loud/Bold, Vintage/Retro, Futuristic/Tech, Rugged, Artistic, Formal, Handmade) không có? Bạn có chịu gắn tag cho nhiều font không, hay muốn app gợi ý? | Tag phong cách/cảm xúc (D3) | Danh sách từ nguyên văn; mức sẵn lòng gắn tay |

Câu hỏi phụ nếu còn thời gian:
- Bật/tắt font: bạn muốn bật từng font, cả family, hay cả dự án? Bạn có từng bật nhầm nhiều nghìn font không?
- Khi bộ lọc đang bật, bạn biết thế nào là mình đang lọc? Nếu một nút phễu cạnh ô tìm kiếm hiện số, bạn đoán nó làm gì?
- Phân loại: bạn gọi một font "Sans" bằng những từ nào? Bạn có tin nhãn tự động như "Sans · Geometric · có thể" không?

Sau mỗi buổi: ghi 3 phát hiện bất ngờ nhất và 1 điều muốn đổi trong spec. Gửi lại PM để cập nhật `spec.md`.
