# 03 — Các cách tiếp cận và đánh giá

> **Cập nhật vòng 2 (2026-11-06):** chủ app đã chốt tên **Pairing Desk** (tính năng) và **Case** (đơn vị); không dùng "Plate". Bỏ định hướng F&B, đối tượng là Graphic Designer / Art Director / Layout Artist. Hệ từ, persona và layout mới: xem `05_layout_options_v2.md`. Nội dung dưới đây giữ nguyên làm lịch sử quyết định.


- **Ngày:** 2026-11-06 · **Đầu vào:** `01_research.md` (insight I1–I8), `02_naming.md` (thuật ngữ).
- **Kết luận:** chọn **hướng lai A + B** — "Trang + Sổ giọng + Khay" — đặt C (so nhiều phương án) và D (bản đồ) làm lớp sau.

Mọi sơ đồ vẽ phần phải của app (sidebar trái giữ nguyên, rộng ~240 px).

---

## A. Trang + cột tên font (ý của chủ app)

**Mô tả.** Phần phải là một **trang ấn phẩm hoàn chỉnh** (menu, nhãn…). Cạnh trang là một **cột** ghi từng vai và font đang dùng. Bấm một dòng trong cột → danh sách gợi ý thả xuống → chọn là thay ngay.

```
┌──────────────────────────────────────────────┬──────────────────┐
│  Thanh trên: tên · bố cục ▾ · chế độ        │                  │
├──────────────────────────────────────────────┤ CỘT VAI          │
│   THỰC ĐƠN · MÙA THU                         │ Eyebrow  Inter   │
│   PHỞ LÝ QUỐC SƯ                             │ Tên quán Playfair│
│   ───────────────                            │ Mục      Playfair│
│   MÓN NƯỚC                                   │ Tên món  Be Viet.│
│   Phở bò tái lăn ............. 65.000đ       │ Mô tả    Be Viet.│
│   Nước dùng ninh xương 12 tiếng…             │ Giá      Be Viet.│
│                                              │ [gợi ý ▾]        │
└──────────────────────────────────────────────┴──────────────────┘
```

**Luồng.** Chọn neo → chọn bố cục → bấm dòng "Tên món" → gợi ý → chọn → trang đổi.
**Ưu.** Dễ hiểu ngay; bố cục là nhân vật chính (I8); khớp cách Typeface làm.
**Nhược.** Chọn là thay ngay: mất phương án cũ, không so song song (vi phạm I3); gợi ý thả xuống hẹp, không chỗ cho giải thích theo trục; ghép theo từng vai dễ thành 6 font (vi phạm I1).
**Độ phức tạp:** thấp–vừa.

## B. Khay so sánh ba vùng (nghiên cứu → so → đặt)

**Mô tả.** Ba vùng trái → phải theo đúng thứ tự suy nghĩ: **Neo** (hồ sơ font gốc, trục) · **Khay** (ứng viên đặt cạnh nhau, chữ thật, nhãn trục) · **Trang** (bố cục). Kéo ứng viên từ khay sang trang.

```
┌──────────────┬────────────────────────────┬────────────────────┐
│ NEO          │ KHAY                       │ TRANG              │
│ Playfair     │ ┌────────┐┌────────┐┌────┐ │  PHỞ LÝ QUỐC SƯ    │
│ Display      │ │Be Viet.││ Lora   ││Inter│ │  Phở bò … 65.000đ  │
│ x-h 0,51     │ │Phở bò… ││Phở bò… ││…   │ │                    │
│ tương phản…  │ │● x-h   ││◆ nét   ││    │ │                    │
│ [Giống|Ghép] │ └────────┘└────────┘└────┘ │                    │
└──────────────┴────────────────────────────┴────────────────────┘
```

**Luồng.** Neo → nguồn gợi ý (Giống / Ghép) → khay đầy ứng viên → ghim 2–3 → kéo sang trang.
**Ưu.** Tách rõ thử và đặt (I3); khay có chỗ cho nhãn ●◐◆○ và câu giải thích; so song song thật.
**Nhược.** Trang bị ép còn 1/3 → không xem được bố cục đầy đủ (vi phạm I8); ba cột ngang không vừa 1100 px; nặng nhận thức lúc mới vào.
**Độ phức tạp:** vừa.

## C. Lưới hệ chữ (so nhiều phương án nguyên hệ)

**Mô tả.** Hàng là vai, cột là phương án (A, B, C). Mỗi ô render vai đó bằng font của phương án. Như nhân bản studio của Typeface, nhưng xếp thành ma trận.

```
┌──────────┬───────────────┬───────────────┬───────────────┐
│          │ Phương án A   │ Phương án B   │ Phương án C   │
├──────────┼───────────────┼───────────────┼───────────────┤
│ Tên quán │ PHỞ LÝ (Playf)│ PHỞ LÝ (Fraun)│ PHỞ LÝ (Oswal)│
│ Tên món  │ Phở bò (BeViet│ Phở bò (Lora) │ Phở bò (Inter)│
│ Giá      │ 65.000đ       │ 65.000đ       │ 65.000đ       │
└──────────┴───────────────┴───────────────┴───────────────┘
```

**Luồng.** Có neo → hệ tự đề xuất 3 phương án → người dùng sửa từng ô → chọn một cột.
**Ưu.** So nguyên hệ, đúng lúc chốt với khách (J4); gọn khi trình bày.
**Nhược.** Mất bố cục thật (chỉ thấy từng vai rời) nên không cảm được nhịp trang; hệ "tự đề xuất 3 phương án" dễ thành hộp đen; khó khi bắt đầu.
**Độ phức tạp:** vừa.

## D. Bản đồ quanh neo (Atlas map)

**Mô tả.** Neo ở giữa; các font trong thư viện toả theo hai trục người dùng chọn (ví dụ tương phản nét × x-height). Vòng gần = giống, vùng có màu theo nhãn = ghép được. Bấm một điểm → xem cặp.

```
┌──────────────────────────────────────────────┬──────────────────┐
│        tương phản nét ↑                      │ CẶP ĐANG XEM     │
│     ·   ◐Lora   ·                            │ Playfair + Lora  │
│  ·   ·   (●NEO)   ●Fraunces                  │ ● x-h  ◐ chân    │
│    ◆Gelasio  ·  ·    ·                       │ Phở bò … 65.000đ │
│               → x-height                     │ [Đưa vào trang]  │
└──────────────────────────────────────────────┴──────────────────┘
```

**Luồng.** Neo → khám phá không gian → chọn điểm → xem cặp → đưa vào trang.
**Ưu.** Signature nhất với "Atlas"; cho thấy cả thư viện; hợp người thích khám phá.
**Nhược.** Bản đồ 2 trục làm mất các trục khác, dễ hiểu sai (spec 7.2 tránh radar làm hình chính vì lý do tương tự); không có bố cục; chi phí render và trực quan hoá cao; khó với bàn phím và trình đọc màn hình.
**Độ phức tạp:** cao.

## E. Trình hỏi đáp theo bước (wizard)

**Mô tả.** Hỏi lần lượt: "Ấn phẩm gì? Font neo? Muốn an toàn hay phá cách? Nội dung tiếng Việt?" → ra 3 hệ đề xuất.

```
┌──────────────────────────────────────────────┐
│ Bước 2/4 · Bạn đang làm gì?                  │
│ ( ) Menu  ( ) Bảng hiệu  ( ) Bao bì  …       │
│                         [Quay lại] [Tiếp]    │
└──────────────────────────────────────────────┘
```

**Ưu.** Thân thiện người mới; thu được ngữ cảnh rõ.
**Nhược.** Chậm với AD có kinh nghiệm; quyết định bị đẩy về hệ thống (trái triết lý "người dùng quyết định"); sau bước cuối vẫn cần một màn làm việc → không thay được A/B.
**Độ phức tạp:** thấp, nhưng không giải quyết việc chính.

---

## Ma trận đánh giá

Thang 1–5 (5 = tốt nhất). Trọng số theo insight ở `01_research.md` §6.

| Tiêu chí | Trọng số | A Trang+cột | B Khay 3 vùng | C Lưới hệ | D Bản đồ | E Wizard | **A+B lai** |
|---|---|---|---|---|---|---|---|
| Xem bố cục use case đầy đủ (I8, J3) | 3 | 5 | 2 | 2 | 1 | 2 | **5** |
| So song song trước khi đặt (I3, J4) | 3 | 1 | 5 | 4 | 2 | 1 | **4** |
| Luôn có gợi ý, người dùng quyết (triết lý) | 3 | 3 | 5 | 3 | 4 | 2 | **5** |
| Giải thích theo trục có chỗ hiện (●◐◆○) | 2 | 2 | 5 | 3 | 4 | 2 | **5** |
| Giữ ≤ 3 font, nghĩ theo giọng (I1) | 2 | 2 | 3 | 4 | 3 | 4 | **5** |
| Kiểm tiếng Việt trên chữ thật (I6) | 2 | 5 | 3 | 4 | 1 | 2 | **5** |
| Học nhanh, ít ngợp lần đầu | 2 | 5 | 2 | 3 | 2 | 5 | **4** |
| Vừa 1100 px, dùng được bằng bàn phím | 1 | 4 | 2 | 3 | 1 | 5 | **4** |
| Chi phí xây (5 = rẻ) | 1 | 4 | 3 | 3 | 1 | 4 | **3** |
| Signature với "Atlas" | 1 | 3 | 3 | 2 | 5 | 1 | **4** |
| **Tổng có trọng số (tối đa 100)** | | **66** | **70** | **63** | **48** | **51** | **91** |

Cách tính: Σ(điểm × trọng số). Ví dụ A = 15+3+9+4+4+10+10+4+4+3 = 66.

## Đề xuất: lai A + B — "Trang + Sổ giọng + Khay"

```
┌─────────────────────────────────────────────────────────┬──────────────────┐
│ ← Plates · Phở Lý ☆ · [Menu|Bảng hiệu|Bao bì|Brand|Edit]│  Đã lưu · 1/2/3  │
├─────────────────────────────────────────────────────────┼──────────────────┤
│                 TRANG (bố cục A)                         │ SỔ GIỌNG (cột A) │
│   THỰC ĐƠN · MÙA THU 2026                                │ ① Giọng chính    │
│   PHỞ LÝ QUỐC SƯ                                         │   Playfair ⚓     │
│   MÓN NƯỚC                                               │   Tên quán · Mục │
│   Phở bò tái lăn ................. 65.000đ              │ ② Giọng đọc      │
│   Nước dùng ninh xương 12 tiếng, bánh phở tươi…         │   Be Vietnam Pro │
│                                                          │   Tên món · Mô tả│
├──────────────────────────────────────────────────────────┴──────────────────┤
│ KHAY (B) cho ② Giọng đọc · [Ghép | Giống | Cùng họ | Thư viện | Đã ghim]    │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐                         │
│ │Lora      │ │Inter     │ │Alegreya  │ │Montserrat│  ← → duyệt · Space thử   │
│ │Phở bò tái│ │Phở bò tái│ │Phở bò tái│ │Phở bò tái│                          │
│ │● x-h ◐ch.│ │● x-h ◐ch.│ │◐ chân ○ │ │◆ khung   │                          │
│ │[Thử][Đặt]│ │          │ │          │ │          │                          │
│ └──────────┘ └──────────┘ └──────────┘ └──────────┘                         │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Vì sao lai.**
- Lấy của A: trang đầy đủ là trung tâm, cột bên ghi font (đúng ý chủ app, I8).
- Lấy của B: khay so song song, thử tạm trước khi đặt, giải thích trên từng ứng viên (I3, I4).
- Sửa nhược của A: cột không liệt kê font theo từng vai mà theo **giọng** (2–3 font), vai là dòng con. Đổi font một giọng thì mọi vai thuộc giọng đó đổi cùng (I1).
- Sửa nhược của B: khay nằm **dưới** trang, cao ~230 px, thu được (`T`), nên trang vẫn rộng. Ở 1100 px khay chỉ hiện 3 thẻ và cuộn ngang.

**Lấy gì từ C, D, E cho sau.**
- **C → "Nhân bản Plate"** (P1): một nút tạo bản sao để so phương án; sau này có màn "So Plate" xếp 2–3 Plate cạnh nhau ở cùng bố cục.
- **D → nguồn "Bản đồ"** trong khay (Sau): thêm một tab xem không gian quanh neo, vẫn đổ kết quả vào khay chứ không thay màn chính.
- **E → zerostate hỏi một câu** duy nhất: "Bắt đầu từ font nào?" — không phải wizard nhiều bước. Bố cục mặc định là Menu, đổi được ngay trên thanh trên.

**Rủi ro của hướng chọn.**
- Ba vùng trên một màn có thể chật ở 1100 × 720. Giảm: khay thu gọn được, sổ giọng thu thành dải 56 px chỉ còn tên font viết tắt.
- Người dùng có thể muốn đổi font riêng cho một vai (ví dụ giá dùng font số khác). Cho phép "Tách vai thành giọng mới" nhưng cảnh báo nhẹ khi vượt 3 giọng (09 D2), không chặn.
