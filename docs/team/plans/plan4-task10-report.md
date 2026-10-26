# Báo cáo Plan 4 Task 10 — nối embedding ảnh vào index/eval

Nhánh `feat/embed-index`, rebase lên `dev/mvp` @ 5481995. Bản trước rebase giữ ở nhánh `backup/embed-index-pre-rebase`. Chưa push, chưa merge.
Máy đo: Apple M4 Pro, macOS, candle 0.11 trên Metal. Lúc đo, máy đang tải nặng (load average 7–41).

## Cách làm
- **Một đường duy nhất qua `FaceProfile.embedding`:**
  - `Index::new` lượng tử int8 (scale theo từng vector, khoảng 128 B/face) rồi bỏ f32 khỏi profile.
  - `similar()` tính khoảng cách kết hợp cho mọi ứng viên, dùng cos int8, giữ `max(k, 400)` ứng viên tốt nhất, rồi xếp lại bằng f32 qua `ExactEmbeddings` (app bọc `StoreEmbeddingSource`).
  - Công thức: d = (1 − w)·d_trục + w·(1 − cos)/2.
- **Ảnh không lấn hết trục:**
  - `MAX_EMBED_WEIGHT = 0,8`; `set_embed_weight` kẹp trọng số trong [0; 0,8].
  - "Giống ở / Khác ở" vẫn chỉ lấy từ trục (có test).
  - Face không có ảnh, hoặc truy vấn không có ảnh, cho kết quả y hệt Index chỉ-trục (có test).
- **Pairing:** cos int8 chỉ dùng làm tie-break ở bước 5 của rank_key. Khối Pairing của eval giống hệt từng byte giữa chế độ có ảnh và chỉ trục.
- **Eval:**
  - nhúng mọi face của corpus, trong đó instance VF được render đúng vị trí `wght`/`wdth`/`opsz`;
  - các cờ mới: `--embed-model`, `--whitening`, `--embed-cache`, `--embed-weight`;
  - chọn `EMBED_WEIGHT` trên nửa **tune** (`family_split`), rồi báo trên nửa **test**.

## Dữ liệu
- **Corpus:** cùng corpus Plan 3, gồm `google/fonts` @47d54ca1 (đường dẫn `/Users/vietx/fontatlas-ref/google-fonts`, **giữ nguyên, không xoá**) và font hệ thống. Tổng **8.068 face**, 0 file lỗi.
- **Embedding:** 7.384 face có embedding; 684 face bỏ qua (không render được chuỗi mẫu Latin) dùng chỉ trục.
- **Model:** `siglip2_base_p16_224_75de2d5_vision.safetensors` (sha `9c0ed27…`), whitening `white-v2` đóng gói sẵn.
- **Thời gian và bộ nhớ:** embed hết 191 s trên Metal; RSS đỉnh của eval là 1,36 GB.
- `similar_sets.csv` **không có dòng `tune`** (cả 27 dòng đều là `test`), nên trọng số chỉ được chỉnh trên nửa tune của Google Fonts.

## Chọn EMBED_WEIGHT (tune, truy vấn họ tune, gallery cả corpus)

| w | 0 | 0,2 | 0,4 | 0,5 | 0,6 | 0,7 | **0,8** |
|---|---|---|---|---|---|---|---|
| R@1 | 43,3% | 57,4% | 64,6% | 67,0% | 69,0% | 70,1% | **70,7%** |
| R@5 | 63,7% | 76,1% | 82,8% | 85,6% | 87,2% | 88,4% | **89,1%** |

→ Kết quả tăng đều tới tận trần, nên chọn **w = 0,8** (bằng trần). Đây là điểm chặn do yêu cầu "không để ảnh lấn hết trục", không phải điểm tối ưu tự nhiên.

## Kết quả trên tập test (đúng như chương trình in)

**S4 — cùng họ, 3.372 truy vấn thuộc họ `test`, gallery cả corpus, cùng một tập truy vấn cho cả 3 chế độ:**

| chế độ | R@1 | R@5 |
|---|---|---|
| Số đo (mốc Plan 3) | 44,3% | 65,8% |
| Chỉ ảnh | 71,0% | 89,6% |
| **Số đo + Hình ảnh** (w 0,8) | **70,9%** | **90,1%** |

So với spec S4 (R@1 ≥ 0,6, R@5 ≥ 0,9):
- R@1 **đạt**.
- R@5 **đạt sát ngưỡng**: 90,1%, chỉ dư 0,1 điểm.

**Cùng họ trên toàn corpus** (cách đo của Plan 3, 6.345 truy vấn, gồm cả họ tune):

| chế độ | R@1 | R@5 |
|---|---|---|
| Số đo | 43,8% | 64,8% |
| Số đo + Hình ảnh | 70,8% | 89,6% |

Số chỉ-trục khớp đúng mốc Plan 3. Theo cách đo này, R@5 89,6% vẫn **thiếu 0,4 điểm** so với ngưỡng 0,9.

**`similar_sets` test:**

| | nhóm đạt | vi phạm must_not |
|---|---|---|
| Số đo | 5/27 | 0 |
| Số đo + Hình ảnh | **7/27** | 0 |

- Đạt thêm 4 nhóm: S07 (Red Hat Text → Display), S11 (Public Sans → Libre Franklin), S15 (Times New Roman → Tinos/Times), S23 (Sorts Mill Goudy → Goudy Bookletter).
- Trượt thêm 2 nhóm: S05 (Roboto → Roboto Condensed: khác độ rộng nên ảnh khác hẳn), S21 (Palatino → GFS Didot).
- Kết quả ròng: +2 nhóm.

**So với mốc 4b chỉ ảnh (76,3% / 89,6%):** hai số này không so trực tiếp được. Mốc 4b có gallery chỉ gồm nửa test, chỉ font OFL/Apache, không tách VF, không có font hệ thống. Dòng "Chỉ ảnh" ở bảng S4 phía trên là phép so đúng.

## S6 (release, 50.000 profile giả)
- `similar()` có ảnh cho mọi face, có bước xếp lại f32: **1,5 ms**.
- Chỉ trục: 1,4 ms.
- `pair_candidates`: 12,5 ms, nhờ tối ưu của Lead ở 5481995.

## Kiểm thử
- Test của `fontatlas-index` và `fontatlas-eval` đều xanh, kể cả các test chỉ chạy ở bản release (golden `pair_candidates`, S6).
- Clippy: 0 cảnh báo. fmt: sạch.
- Test mới cho index (`tests/embed_index.rs`):
  - xếp lại int8 → f32 cho đúng thứ hạng f32;
  - int8 đứng một mình đạt ≥ 18/20 top-20;
  - không có ảnh → chỉ trục;
  - vector sai số chiều hoặc bằng 0 bị bỏ qua;
  - truy vấn không có ảnh cho kết quả y hệt Index chỉ-trục;
  - giải thích lấy từ trục;
  - trần trọng số;
  - f32 bị bỏ khỏi profile;
  - tie-break pairing bằng cos, không đổi cổng;
  - S6 có ảnh.
- Test mới cho eval (`tests/embed.rs`): origin của từng face (instance VF đúng `wght`), embed corpus, cùng họ có lọc query/gallery, chế độ chỉ ảnh.

## Nhận xét trung thực và rủi ro còn mở
1. **Ở w = 0,8, trục gần như không thêm gì cho việc tìm cùng họ.** Kết hợp ≈ chỉ ảnh: R@1 kém 0,1 điểm, R@5 hơn 0,5 điểm. Vai trò của trục lúc này là giải thích, lọc, và làm phương án dự phòng khi thiếu ảnh. Trục thua ảnh rõ ở các bộ đếm "cùng họ" vì độ đậm/độ nghiêng khác nhau trong cùng họ làm trục xa nhau.
2. **R@5 trên test chỉ dư 0,1 điểm**, và trên toàn corpus thì chưa đạt (89,6%). Đổi corpus hoặc renderer có thể làm rơi xuống dưới ngưỡng.
3. **Thang khoảng cách bị trộn.** Khi truy vấn có ảnh, ứng viên không có ảnh (khoảng 8,5% corpus) mang khoảng cách chỉ-trục, còn ứng viên có ảnh mang khoảng cách kết hợp. Hai thang này phân bố khác nhau. Phần lớn font không có ảnh là font CJK/không Latin, vốn đã khó so trục, nên ảnh hưởng thực tế chưa đo.
4. `family_split` dùng `profile.family` (typographic family). whitenfit cũng dùng tên đó, nên nhất quán.
5. Trần 0,8 là quyết định sản phẩm, không phải tối ưu số liệu. Nếu Lead muốn ưu tiên R@k thì cần nới trần (đường cong vẫn đang tăng).
