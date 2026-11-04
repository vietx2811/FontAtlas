# similar_sets: tách tune/test — báo cáo

Nhánh `eval/similar-split` (từ dev/mvp 308c6bb). Không push, không merge.

## Quy tắc chia (`fontatlas_eval::split_similar`, seed `similar-split-v1`)

1. **Chống rò.** Hai nhóm có chung một họ (so trên `query` ∪ `must_include`, không phân biệt hoa thường) thì luôn nằm cùng một phía. `must_not` không tính, vì các họ làm mồi nhử (Helvetica, Didot…) lặp lại ở gần như mọi nhóm.
2. **Phân tầng.** Mỗi thành phần liên thông được xếp vào tầng theo `group_type` của nhóm có id nhỏ nhất trong thành phần đó.
3. **Chọn tune.** Trong mỗi tầng có n nhóm, duyệt các thành phần theo thứ tự `blake3(seed|id nhỏ nhất)`. Một thành phần được đưa vào tune nếu thoả cả hai điều kiện:
   - tổng số nhóm tune của tầng không vượt round(n/3);
   - tầng vẫn còn ít nhất một nhóm test.
4. **Ghim CSV vào quy tắc.** Test `csv_split_column_matches_rule` bắt buộc cột `split` trong CSV phải khớp quy tắc. Khi sửa hoặc thêm nhóm, test sẽ báo nếu cần cập nhật cột này.

Kết quả: **9 tune** (S01, S03, S05, S08, S13, S17, S21, S23, S27) và **18 test**. Tầng nào cũng còn nhóm test. Các thành phần liên thông có nhiều hơn một nhóm: S02+S14, S04+S22, S09+S10+S18, S11+S26, S24+S25. Cả năm thành phần này đều rơi vào test.

## Số đo (eval, EMBED_WEIGHT 0,8, cache embed của Task 10)

| similar_sets | Chỉ số đo | Số đo + Hình ảnh |
|---|---|---|
| test (18) | 1/18 | **4/18** (S07, S09, S11, S15) |
| test, không tính † (17) | 1/17 | 3/17 |
| tune (9) | 4/9 | 3/9 |
| tổng (27), để so với số cũ | 5/27 | 7/27 |

Không có nhóm nào vi phạm must_not, ở cả hai phía. Con số S4 không đổi (R@1 70,9%, R@5 90,1% trên nửa test).

## Cần Lead lưu ý

- **similar_sets tune nghiêng về trọng số ảnh thấp hơn.** Ở w ≤ 0,60 đạt 4/9, ở w ≥ 0,65 còn 3/9. Mất đúng một nhóm, trên tổng 9 nhóm, nên chưa đủ căn cứ để đổi trọng số khỏi 0,8. Trọng số vẫn chọn theo bài cùng họ trên tune. Nếu sau này bổ sung thêm nhóm similar, nên chọn trọng số bằng tiêu chí kết hợp. Lead duyệt giữ 0,8 và xem lại khi có thêm nhóm.
- **Rò chéo từ bảng superfamily-v1 (Lead đã duyệt cách xử lý).** Bảng này dựng từ phần pairing tune và có nhóm `red-hat` (Display + Text), đúng cặp của **S07, một nhóm test** (S07 đang Hit). S07 vẫn để ở test và được gắn `informed_by = pairing-tune: superfamily-v1 red-hat (P16)`. Eval đánh dấu † và in thêm số nhóm đạt khi không tính S07: **Số đo + Hình ảnh còn 3/17, Số đo vẫn 1/17**.
- **Plex Serif / S06.** superfamily-v1 xếp IBM Plex Serif cùng nhóm `ibm-plex` với Plex Sans (P06), trong khi S06 (test) đặt Plex Serif vào `must_not`. Nếu Similar ưu tiên superfamily, Plex Serif có thể bị đẩy lên top-k và tạo vi phạm. Hiện S06 Miss nhưng không vi phạm `must_not`. S06 không gắn `informed_by`, vì bảng không liên kết query với `must_include` (Plex Sans Condensed không có trong bảng). Đây là rủi ro chiều ngược lại: thông tin tune có thể làm hỏng một nhóm test. Cần theo dõi nếu sau này bảng thêm Plex Sans Condensed hoặc tăng trọng số superfamily.
- **Cỡ mẫu.** 18 nhóm test nên mỗi nhóm tương đương khoảng 5,6 điểm %. Chỉ nên dùng để phát hiện thoái lui, không đủ làm ngưỡng chất lượng. Lead đã duyệt: ghi vào README eval.
