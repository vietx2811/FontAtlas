# Đề xuất cho chủ app: máy chậm làm CI chậm (07 QA/DevOps, 2026-11-13)

**Tóm tắt.** Cùng mã, build + test chạy riêng mất ~4 phút (đo 2026-11-13), nhưng trong CI mất 2,4–3,8 giờ. Sampler 30 giây của CI (hơn 1.000 mẫu qua 3 lượt đủ) cho thấy máy không bận vì cargo mà vì các tiến trình khác. Cần chủ app quyết ba việc dưới đây; tôi không chạm cài đặt hệ thống nào.

## Số liệu
| Lượt CI | cargo-test | tauri-debug-build | tải trung bình / tối đa | top CPU (nhiều mẫu nhất) |
|---|---|---|---|---|
| int/0.4.1 (ba0c61da) | 13607 s | 528 s | 5,8 / 19,8 | Adobe Illustrator |
| dev/mvp (cbb5aff4) | 11749 s | 527 s | 6,4 / 24,5 | Adobe Illustrator |
| dev/mvp (1b64934b), chỉ build lại crate `app` | cache | 1496 s | 6,4 / 11,2 | Adobe Illustrator |

- Adobe Illustrator (pid 10820) chạy liên tục hơn 1 ngày ở 75–95% CPU, có mặt trong top 3 ở mọi mẫu.
- `syspolicyd` (kiểm tra Gatekeeper) có mặt trong top 3 ở 48/62 mẫu của lượt 1b64934b (lên tới 82%), `suggestd` 37/62. `rustc` không lọt top 3 ở mẫu nào: trình biên dịch chờ chứ không thiếu việc.
- Thử riêng: lần chạy đầu một binary mới chỉ chậm thêm 0,3–0,7 giây ở trạng thái máy bình thường. Vậy chi phí chỉ phình ra khi `syspolicyd`/`suggestd` bị nghẽn; chưa chứng minh Illustrator là nguyên nhân.

## Ba đề xuất (ưu tiên từ rẻ đến tốn)
1. **Thoát hoặc khởi động lại Adobe Illustrator khi không thiết kế.** Rủi ro: mất việc chưa lưu (cần lưu trước). Không đổi hệ thống. Kiểm chứng: chạy một lượt CI sau đó và so dòng `sampler` trong `summary.json` (max/avg load, top process).
2. **Chạy CI vào khung giờ máy rảnh** (ví dụ ban đêm, không mở Illustrator/Eagle). Rủi ro: không có; chỉ là lịch. Có thể đặt `local_ci.sh` vào hàng đợi trước khi đi ngủ.
3. **(Tuỳ chọn, quyết định của chủ app) Cấp "Developer Tools" trong Cài đặt hệ thống → Quyền riêng tư & Bảo mật cho ứng dụng Terminal/Claude chạy CI.** Tiến trình chạy từ app đó được miễn bước kiểm tra Gatekeeper cho binary tự build. Rủi ro nói thẳng: nó hạ một lớp bảo vệ cho mọi thứ khởi chạy từ app đó (một file tải về chạy từ terminal đó cũng không bị kiểm), nên chỉ nên cấp cho một terminal chuyên dùng cho CI; có thể thu hồi bất cứ lúc nào. Lợi ích chưa đo được và có thể nhỏ (xem trên: ở trạng thái bình thường chi phí chỉ khoảng 1 phút cho 134 binary). Chỉ nên làm nếu sau bước 1–2 `syspolicyd` vẫn nghẽn.

## Không làm
Không tắt Gatekeeper/`spctl`, không đổi TCC, không giết hay renice tiến trình của người khác.
