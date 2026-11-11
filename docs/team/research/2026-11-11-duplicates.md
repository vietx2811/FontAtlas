# Nghiên cứu: Giải quyết font trùng lặp (0.3.0)

Ngày: 2026-11-11. Người soạn: ML/Typography (04). Chưa code. Yêu cầu từ chủ app (2026-11-11): trang "Trùng lặp" hiện chỉ báo; cần hành động giải quyết TỰ ĐỘNG và trình bày thân thiện. Đổi quyết định cũ "chỉ báo, không xoá/di chuyển file".

Quy ước: **[V]** = đã đọc từ nguồn trong phiên này; **[I]** = suy luận, cần kiểm lại. Thuật ngữ font giữ tiếng Anh, kèm gloss.

---

## 0. Tóm tắt

1. Mọi font manager đều **chọn sẵn bản nên giữ rồi mới xoá/ẩn bản còn lại**; không ai bắt người dùng tự so từng cặp. Thứ tự ưu tiên thật sự được ghi lại: MainType xếp **version → variable hơn static → ngày sửa/tạo → số glyph → số ký tự → dung lượng** [V]; Typeface ưu tiên **OpenType hơn TrueType, WOFF2, WOFF, Type 1** [V]. Font Book có hai chế độ: chỉ **tắt** bản trùng (mặc định) hoặc **chuyển vào Trash** nếu bật tuỳ chọn [V].
2. Đề xuất cho FontAtlas: nút **"Giải quyết"** cho một nhóm và **"Giải quyết tất cả an toàn"**; chọn bản giữ theo quy tắc cố định bên dưới; bản còn lại mặc định **"Ẩn khỏi thư viện + không bao giờ bật"** (không đụng file). **"Chuyển vào Thùng rác"** là lựa chọn thứ hai, luôn có xác nhận và hoàn tác; **không bao giờ xoá vĩnh viễn**.
3. Phát hiện quan trọng từ code (mục 3): nhóm `same_file` có **cùng hash**, nên không thể ẩn/chặn theo hash (sẽ ẩn cả bản giữ) — hành động của nhóm này phải theo **đường dẫn**. Và yêu thích / collection / tag / Case đang gắn theo `(hash, face_index)`: ẩn bản thua sẽ làm mất chúng nếu không **chuyển sang bản giữ**.
4. Chỉ tự giải quyết hàng loạt những nhóm **không mơ hồ**; nhóm còn lại vào danh sách "Cần bạn xem". Trình bày: ngôn ngữ thường, mỗi nhóm một câu lý do, mọi chi tiết kỹ thuật (hash, PostScript name, revision, mã định dạng, đường dẫn đầy đủ) ẩn sau "Chi tiết", so sánh 2 bản cạnh nhau bằng cùng một dòng mẫu.

---

## 1. Các font manager xử lý trùng thế nào

| Công cụ | Cách phát hiện | Chọn bản giữ | Hành động với bản còn lại | Nguồn |
|---|---|---|---|---|
| macOS Font Book | trùng tên font | gợi ý một bản ("Active"), người dùng có thể đổi; tài liệu không nói rõ tiêu chí (báo cáo người dùng mâu thuẫn: có khi giữ bản cũ) | tuỳ chọn "Resolve duplicates by moving files to the Trash when possible"; không bật thì chỉ **tắt** bản trùng [V]; có báo cáo "Resolve" không hiệu quả | [Apple Font Book prefs](https://support.apple.com/guide/font-book/fntbk1004/mac), [Apple Communities](https://discussions.apple.com/thread/3770773) |
| Typeface | cùng **PostScript name**; công tắc "Exact" thêm khớp kind + manufacturer + version | menu "Select Duplicates → Automatic": OpenType hơn TrueType/WOFF2/WOFF/Type 1; chọn theo Kind / Location (vd System); chuột phải thư mục "Auto Select Duplicates in this Folder"; luôn chừa ít nhất một bản không chọn | "Remove from Library" (giữ file, có thể được nhập lại trừ khi bật "Prevent Importing New Duplicates") hoặc "Move to Trash"; bài hướng dẫn không nhắc undo [V] | [Typeface help](https://typefaceapp.com/help/articles/finding-and-removing-duplicate-fonts) |
| MainType (High-Logic) | "Library → Select Duplicates" theo phạm vi Library / View / Group | xếp hạng tự động: **version cao hơn → variable hơn static → ngày Modified/Created mới hơn → nhiều Glyphs → nhiều Characters → dung lượng lớn hơn**; Ctrl+Click đảo lựa chọn | chọn rồi Delete: gỡ khỏi group/library và/hoặc xoá file khỏi đĩa [V] | [MainType: clean up outdated versions](https://www.high-logic.com/font-manager/maintype/tutorials/clean-up-outdated-font-versions) |
| Suitcase Fusion / FontDoctor | "Find Exact Duplicates" (giống hệt) tách khỏi "Duplicate Fonts" (cùng tên, khác đặc tính); FontSense dùng fingerprint | — | Font Vault **không cho** bản trùng thật sự vào vault; FontDoctor cách ly (quarantine) bản trùng [V, nguồn cũ] | [CreativePro](https://creativepro.com/troubleshoot-font-problems/), [Extensis](https://www.extensis.com/blog/top-5-suitcase-fusion-tips-you-need-to-know) |
| RightFont | cùng PostScript name | — | khi nhập, tuỳ chọn "Skip duplicate fonts": bỏ qua font cùng PostScript name bất kể version/định dạng [V, qua release notes] | [RightFont updates](https://rightfontapp.com/updates) |
| FontBase | cùng nội dung ở nhiều đường dẫn; có chỗ xem/tìm bản trùng | — | người dùng tự xoá; diễn đàn nói tính năng chưa rõ [V yếu] | [FontBase forum](https://forum.fontba.se/t/finding-and-deleting-duplicates/75) |
| FontExplorer X Pro | khi nhập, **copy vào kho riêng** nên tự loại bản trùng/hỏng; ngưng hỗ trợ (Monotype) [V yếu] | — | — | [CreativePro review](https://creativepro.com/review-fontexplorer-x-pro-5/) |

Nhận xét:
- Hai mô hình: **dọn khi nhập** (Fusion Vault, RightFont, FontExplorer) và **dọn sau** (Font Book, Typeface, MainType). FontAtlas theo dõi thư mục tại chỗ (không copy) nên là mô hình "dọn sau" + nên có công tắc ngăn bản trùng mới xuất hiện lại (xem 4.7).
- Chỉ Typeface và Font Book phân biệt "bỏ khỏi thư viện" với "bỏ vào Trash"; Typeface cảnh báo rõ bản bị bỏ khỏi thư viện **có thể nhập lại** — FontAtlas phải ghi nhớ quyết định để không hiện lại sau rescan.
- Không nguồn nào hỗ trợ hoàn tác rõ ràng; đây là chỗ FontAtlas làm tốt hơn.

## 2. Designer thực sự muốn gì

Nguồn trực tiếp rất thưa (phần lớn là diễn đàn); các điểm sau có bằng chứng, phần còn lại là [I] cần kiểm với 3-5 designer trước khi chốt mặc định.

- **Xung đột là vấn đề thật**: khi hai font cùng PostScript name, InDesign không bao giờ hiện cả hai, bất kể định dạng; Monotype ghi cùng xung đột kể cả khác version/nguồn [V]. ([Typekit 2008](https://blog.typekit.com/2008/05/08/indesign_font_conflicts/), [Monotype](https://support.monotype.com/en/articles/12811909-font-conflict-and-resolution), [Adobe Community](https://community.adobe.com/questions-671/font-missing-tt-otp-font-versions-appearing-and-reinstall-font-conflict-865600))
- **"Mới nhất" không luôn là "đúng"**: cộng đồng Adobe khuyên giữ đúng "flavor" (TrueType/OpenType) và đúng release mà **tài liệu đang dùng**; đổi font có thể làm văn bản reflow; khi làm việc nhóm, cả chuỗi phải dùng đúng cùng file [V]. → Quy tắc tự động phải có **cảnh báo "có thể làm văn bản cũ nhảy dòng"** khi hai bản khác version/metrics, và không tự động cho nhóm mơ hồ.
- **Adobe Fonts / system trùng bản cài local** gây xung đột; khuyên bỏ bản local nếu dùng bản cloud [V]. → Nguồn do app khác quản lý là **ứng viên giữ**, bản trong thư mục của người dùng là ứng viên ẩn, và không bao giờ trash file nằm ngoài thư mục người dùng.
- [I] Lý do trùng phổ biến nhất của designer: tải lại cùng gói nhiều lần (`Font (1).ttf`), cùng font ở thư mục dự án cũ và thư mục chung, gói TTF+OTF cùng lúc, bản foundry cũ và bản Google Fonts. Với nhóm thứ nhất và hai (cùng nội dung) không có cân nhắc nào ngoài "giữ một".
- [I] Ưu tiên khi chọn: (1) cùng nội dung → giữ bản ở thư mục "chính"; (2) khác định dạng → **OTF (CFF) hơn TTF**, định dạng web (WOFF/WOFF2) không dành cho cài đặt desktop nên xếp cuối; (3) khác version → bản mới hơn, trừ khi bản cũ có nhiều glyph hơn (mất chữ Việt là lỗi nặng nhất với người dùng FontAtlas).

## 3. Code hiện có (đã đọc)

- `UX/src-tauri/src/services/duplicates.rs`: `group_rows` tạo 3 loại nhóm: `same_file` (cùng BLAKE3, nhiều đường dẫn), `versions` (cùng PostScript name, khác nội dung, cùng định dạng hoặc lẫn), `formats` (khác định dạng; gồm cả nhóm cùng family+style+weight+italic khi PostScript khác nhau). `mark_newest` đánh dấu bản mới nhất theo (fontRevision, head.modified), **chỉ khi mọi thành viên có đủ dữ liệu và chỉ có một đỉnh**; file iCloud chưa tải / mất thì không đọc head.
- `DuplicateMemberDto`: `row: FaceRowDto` (có `isVariable`, `viStatus`, `supportsVi`, `viMissing`, `fileSize`, `path`, `missing`, `isFavorite`, `licenseClass`), `path`, `format`, `version`, `revision`, `modified`, `isNewest`. **Chưa có**: số glyph (`faces.num_glyphs` có trong DB nhưng không lên DTO), nguồn (`sources.kind/name` — `file_locations.source_id`), trạng thái bật/tắt, số mục yêu thích/collection/tag/Case đang dùng bản đó, độ phủ chữ Việt chi tiết.
- `DuplicatesPage.tsx`: danh sách nhóm theo loại, mỗi thành viên hiện tên, mã định dạng, nhãn "mới nhất", đường dẫn, version/revision/ngày, nút "Hiện trong Finder"/"Mở". Ghi rõ "chỉ để xem". Chuỗi i18n `dup.*`.
- Store (`schema.rs`): **không có** khái niệm "ẩn" hay "thùng rác" cho face/file. Có `activation_state` khoá theo `hash` (state active/inactive, status registered/conflict…, `activation_reasons`). Cases có thùng rác riêng (`TRASH_DAYS = 30`, cho Case, không phải file). `Cargo.toml` chưa có crate trash; chỉ có `tauri-plugin-opener` (reveal).
- Hệ quả thiết kế:
  1. `same_file`: các thành viên **chung hash**. Mọi cơ chế khoá theo hash (ẩn, chặn bật) sẽ ảnh hưởng cả bản giữ. Với nhóm này, hành động đúng là theo **vị trí tệp** (`file_locations.location_id`): đánh dấu vị trí bị bỏ qua, hoặc trash file tại đường dẫn đó. Nội dung vẫn còn ở vị trí giữ, nên mất mát = 0.
  2. `versions`/`formats`: khác hash nên ẩn theo `(hash, face_index)` được.
  3. Favorites, collection_items, face_tags, case_faces, combos, pairs, activation_state đều khoá theo hash/face. Ẩn bản thua = các mục này "biến mất" khỏi thư viện nếu không xử lý. Cần bước **"chuyển liên kết sang bản giữ"** (INSERT OR IGNORE sang key mới, ghi sổ để hoàn tác).
  4. TTC/OTC: nhiều face một file. Chỉ cho phép trash file khi **mọi face** trong file đều là bản thua; nếu không chỉ ẩn từng face.
  5. Rescan: `file_locations` quét lại sẽ thấy lại file bị ẩn → cần sổ quyết định bền (xem 4.5), không phải cờ tạm.

## 4. Đề xuất

### 4.1 Quy tắc chọn bản giữ (cố định, giải thích được bằng một câu)

Chấm theo thứ tự, bản đầu tiên thắng ở một tiêu chí rõ ràng thì dừng; bằng nhau thì sang tiêu chí sau:

| # | Tiêu chí | Áp dụng cho | Câu giải thích cho người dùng |
|---|---|---|---|
| 0 | Chặn: chỉ xét bản còn file đọc được (không `missing`, iCloud đã tải) | tất cả | — |
| 1 | **Nguồn ưu tiên**: nguồn hệ thống / do app khác quản lý > thư mục "chính" người dùng chọn > thư mục khác. Người dùng sắp thứ tự được (mặc định theo thứ tự nguồn đã thêm) | tất cả, quyết định `same_file` | "Giữ bản trong thư mục Fonts của bạn" |
| 2 | **Không mất chữ**: bản nào hỗ trợ tiếng Việt đầy đủ (`viStatus`) hoặc nhiều glyph hơn rõ rệt thì thắng; bản kia có `viMissing` > 0 thì không bao giờ là bản giữ khi bản này đủ | `versions`, `formats` | "Bản này có đủ chữ tiếng Việt, bản kia thiếu 12 ký tự" |
| 3 | **Version cao hơn** (fontRevision, rồi head.modified) — dùng lại `mark_newest`, chỉ khi `newestKnown` | `versions` | "Bản mới hơn (2.137 so với 2.001)" |
| 4 | **Định dạng**: OTF/CFF > TTF > TTC/OTC > WOFF2 > WOFF; variable ngang static không thắng thua (cảnh báo, xem 4.2) | `formats` | "OpenType thường đầy đủ tính năng hơn TrueType" |
| 5 | Dung lượng / số glyph lớn hơn | hoà ở trên | "Bản đầy đủ hơn" |
| 6 | Vẫn hoà → **không chọn**, chuyển sang "Cần bạn xem" | tất cả | "Hai bản giống nhau về mọi mặt đo được" |

Luôn **giữ ít nhất một**, kiểu Typeface. Mỗi nhóm lưu `reason` (mã + tham số) để UI hiện câu giải thích và để kiểm thử.

### 4.2 Nhóm nào tự động được, nhóm nào phải hỏi

- **Tự động (an toàn)**: `same_file` toàn bộ (nội dung giống hệt); `formats` khi chỉ khác định dạng và tiêu chí 2 không mâu thuẫn; `versions` khi `newestKnown`, bản mới không kém bản cũ về chữ Việt/số glyph.
- **Cần bạn xem**: `newestKnown = false`; bản mới nhưng ít glyph hơn; variable vs static (axes khác nhau, text có thể đổi); hoà ở bước 6; khác nhau về `licenseClass`; bản nằm ngoài thư mục người dùng.
- Cảnh báo reflow (không chặn): khi `versions` có version khác nhau, ghi "Văn bản dùng bản cũ có thể xuống dòng khác đi".

### 4.3 Hành động cho các bản còn lại (ba mức, mặc định thấp nhất)

| Mức | Tên đề xuất (UI) | Làm gì | File | Hoàn tác |
|---|---|---|---|---|
| A (mặc định) | **Ẩn bản trùng** | Ẩn khỏi thư viện, không cho bật (activation chặn), vẫn nằm ở trang "Đã gộp" | **không đụng** | một chạm "Khôi phục", không giới hạn thời gian |
| B | **Chuyển vào Thùng rác** | Như A + chuyển file vào Trash của hệ điều hành | di chuyển vào Trash | "Hoàn tác" ngay (đưa lại đúng chỗ cũ bằng đường dẫn Trash trả về) + người dùng tự lấy ra từ Trash; hết hạn khi họ đổ rác |
| C | (không có) | Xoá vĩnh viễn | — | **Không làm** |

Quy tắc an toàn cho mức B:
- Luôn hộp xác nhận liệt kê **từng file** sẽ vào Thùng rác (số lượng, đường dẫn rút gọn, dung lượng), mặc định tắt "nhớ lựa chọn".
- Chỉ áp dụng cho file trong thư mục do người dùng thêm (source `directory`); **không bao giờ** trash file thuộc nguồn hệ thống, Google Fonts/Adobe/community (chỉ ẩn).
- Bản đang **bật** (activation) phải tắt trước; nếu bản giữ chưa bật mà bản thua đang bật, hỏi "Bật bản giữ để tài liệu đang mở không mất font?" (mặc định có).
- Nếu trash thất bại một phần: dừng, báo file nào xong/chưa, không để trạng thái nửa vời (xem sổ ở 4.5).
- Phù hợp nguyên tắc "tool, not gatekeeper": không chặn, không đòi giấy phép; nhưng ghi chú "Font có thể là bản bạn đã mua; kiểm tra trước khi bỏ" ở xác nhận mức B.

### 4.4 Giữ liên kết khi ẩn

Khi giải quyết, trong một giao dịch: chuyển favorite / collection_items / face_tags / case_faces / combo slots / pairs từ `(hash thua)` sang `(hash giữ)` bằng INSERT OR IGNORE; ghi từng thay đổi vào sổ để hoàn tác chính xác (không chuyển ngược cả những mục vốn đã có sẵn ở bản giữ).

### 4.5 Sổ quyết định (schema user, bền qua rescan)

Bảng đề xuất `dup_resolutions(resolution_id, group_kind, group_key, keeper_hash, keeper_face, loser_hash, loser_face, loser_location_id NULL, action ('hide'|'trash'), trash_path NULL, reason_code, reason_args, links_moved JSON, created_at, undone_at NULL)`.
- Rescan: face/vị trí nằm trong sổ chưa `undone_at` thì **không hiện lại** trong thư viện và trang Trùng lặp (nhưng hiện ở "Đã giải quyết" để khôi phục).
- `same_file` dùng `loser_location_id` / đường dẫn; còn lại dùng `loser_hash`.
- Hoàn tác = áp ngược sổ (kể cả đưa file từ `trash_path` về), `undone_at = now`. Dùng nhật ký để toàn bộ thao tác nguyên tử: xong sổ mới cập nhật thư viện, hỏng giữa chừng thì phục hồi được khi mở app.
- Cần crate trash (macOS: `NSFileManager.trashItem` trả URL mới; Windows: Recycle Bin — chưa có API tương đương dễ lấy lại đường dẫn [I], phải kiểm lúc làm). Windows không phải ưu tiên MVP nên mức B ban đầu có thể chỉ macOS.

### 4.6 Trình bày lại trang "Trùng lặp"

Nguyên tắc: thiết kế cho GD/AD, không phải cho kỹ sư; tên loại là ngôn ngữ thường, thuật ngữ font kèm tooltip (có hình khi được), đúng quy ước i18n của dự án.

Đầu trang: một câu tóm tắt + một nút chính.
> "Tìm thấy 41 nhóm font bị trùng. 34 nhóm xử lý được ngay, 7 nhóm cần bạn xem." — **[Giải quyết 34 nhóm an toàn]**

Bấm nút → màn xem trước (không làm gì cho tới khi xác nhận): "Giữ 34 bản, ẩn 38 bản, tiết kiệm 112 MB nếu bỏ vào Thùng rác", chọn Ẩn / Thùng rác, danh sách nhóm sẽ bị đụng.

Tên loại (thay cho mã `same_file/versions/formats`):

| Cũ | Mới (gợi ý) | Gloss/tooltip |
|---|---|---|
| same_file | **Bản sao giống hệt** | Cùng một tệp, nằm ở nhiều chỗ |
| versions | **Bản cũ và bản mới** | Cùng font nhưng khác phiên bản |
| formats | **Cùng font, khác định dạng** | Ví dụ bản OTF và bản TTF của cùng một font |

Một nhóm = một thẻ:
- Tiêu đề: tên người đọc được ("Inter Bold"), "2 bản".
- Hai cột cạnh nhau (A | B), **cùng một dòng mẫu** (mặc định lấy từ ô xem trước hiện có; có thể gõ chữ riêng, có sẵn "Việt Nam" để thấy dấu), cỡ lớn để thấy khác biệt nét. Bản giữ có nhãn **"Nên giữ"** và một câu lý do (bảng 4.1). Cạnh nhau hiện 3-4 dòng thực chất: *Phiên bản*, *Số ký tự*, *Tiếng Việt*, *Dung lượng*, *Nằm ở* (tên thư mục thân thiện, không đường dẫn dài).
- Với `same_file` không hiện so sánh hình (giống hệt) mà chỉ liệt kê nơi chứa: "Có ở 2 chỗ: Fonts/Inter, Downloads — giữ bản trong Fonts/Inter."
- Nút trên thẻ: **[Giải quyết]** (áp quy tắc), **[Đổi bản giữ]** (chọn lại), **[Bỏ qua nhóm này]** (nhớ để không hỏi lại).
- Hàng nhỏ **"Chi tiết"** (thu gọn mặc định): đường dẫn đầy đủ, hash, PostScript name, revision/ngày sửa, mã định dạng, mở trong Finder. Hiện cho ai cần, không hiện cho người dùng thường.
- Nhóm "Cần bạn xem" tách riêng, mỗi thẻ nói **vì sao máy không tự quyết** ("Không biết bản nào mới hơn", "Bản mới thiếu 12 ký tự tiếng Việt").
- Thanh dưới (toast) sau khi giải quyết: "Đã ẩn 38 bản. **[Hoàn tác]**" (kéo dài ~10 giây trên màn hình; hoàn tác lâu dài nằm ở tab "Đã giải quyết").
- Tab "Đã giải quyết": từng quyết định, nút **Khôi phục**, ghi rõ nếu file đã nằm trong Thùng rác.

So sánh trực quan: dùng chính renderer đang có cho thẻ font; chỉ vẽ lại khi người dùng mở nhóm (không dựng ngay cả trăm nhóm). Hai bản có thể không đọc được (iCloud chưa tải / mất) → hiện chỗ trống với lý do, không dựng sai.

### 4.7 Ngăn bản trùng quay lại (tuỳ chọn, mặc định bật)

Bản tương tự "Prevent Importing New Duplicates" của Typeface: bản trùng mới xuất hiện sau rescan được tự gắn nhãn "Mới trùng" thay vì lặng lẽ vào thư viện; có công tắc "Tự giải quyết bản trùng an toàn khi quét" (mặc định tắt cho tới khi người dùng đã tự dùng nút ít nhất một lần).

### 4.8 Việc cần làm ở backend (ước lượng, để Lead chia)

1. Gom nguồn dữ liệu cho quy tắc: `num_glyphs`, source kind/name/ưu tiên, activation state, số liên kết đang dùng → thêm vào `DuplicateMemberDto` (cập nhật bindings; có test bindings-drift).
2. Hàm thuần `pick_keeper(group) -> Decision{keeper, losers, reason, auto_safe}` + test đầy đủ cho từng nhánh bảng 4.1 và từng nhóm "Cần bạn xem". Đây là phần nên TDD đầu tiên.
3. Schema: `dup_resolutions` (migration user mới), lọc "đã giải quyết" ở truy vấn thư viện và `duplicate_rows`.
4. Giao dịch resolve/undo (liên kết, activation, sổ), trash macOS với đường dẫn trả về, kiểm tra TTC.
5. Lệnh Tauri: `duplicate_plan(groups?) -> Plan` (xem trước, không đổi gì), `duplicate_apply(plan, action)`, `duplicate_undo(resolution_ids)`, `duplicate_resolved_list`.
6. UI theo 4.6; thêm i18n key (EN thuật ngữ + gloss), tooltip có hình cho thuật ngữ khó (PostScript name, fontRevision, variable vs static).

### 4.9 Rủi ro và câu hỏi cho chủ app

- **Mặc định Ẩn hay Thùng rác?** Đề xuất mặc định Ẩn (không đụng file), vì reflow/tài liệu cũ và font có thể đã mua; Thùng rác là lựa chọn cố ý.
- **Thư mục "chính"**: có cần màn thiết lập "nguồn nào ưu tiên giữ" ngay từ 0.3.0, hay mặc định theo thứ tự thêm nguồn?
- **Hành vi khi file thua nằm ngoài thư mục người dùng** (hệ thống, Adobe): chỉ ẩn, đề xuất như trên.
- **Windows**: trash với đường dẫn hoàn tác chưa kiểm chứng; MVP macOS-first?
- Kiểm với vài designer thật trước khi chốt thứ tự định dạng (OTF > TTF) và mặc định Ẩn/Thùng rác. Nguồn công khai chỉ cho thấy Typeface chọn OTF trước và MainType chọn version trước; chưa có số liệu hành vi người dùng.
- Rủi ro lớn nhất: làm mất liên kết (yêu thích, collection, Case) và tài liệu cũ nhảy dòng. Cả hai có biện pháp ở 4.3-4.4; cần test tích hợp trên thư viện thật nhỏ (không dùng font của chủ app).

## 5. Nguồn

- Apple, Font Book preferences: https://support.apple.com/guide/font-book/fntbk1004/mac
- Apple Communities, "Font Book and duplicates": https://discussions.apple.com/thread/3770773
- Typeface, finding and removing duplicate fonts: https://typefaceapp.com/help/articles/finding-and-removing-duplicate-fonts
- High-Logic MainType, clean up outdated font versions: https://www.high-logic.com/font-manager/maintype/tutorials/clean-up-outdated-font-versions
- CreativePro, troubleshoot font problems (Suitcase/FontDoctor): https://creativepro.com/troubleshoot-font-problems/
- Extensis, top 5 Suitcase Fusion tips: https://www.extensis.com/blog/top-5-suitcase-fusion-tips-you-need-to-know
- RightFont updates: https://rightfontapp.com/updates
- FontBase forum, duplicates: https://forum.fontba.se/t/finding-and-deleting-duplicates/75
- CreativePro, FontExplorer X Pro 5 review: https://creativepro.com/review-fontexplorer-x-pro-5/
- Typekit blog, InDesign font conflicts (2008): https://blog.typekit.com/2008/05/08/indesign_font_conflicts/
- Monotype, font conflict and resolution: https://support.monotype.com/en/articles/12811909-font-conflict-and-resolution
- Adobe Community, TT/OTF version conflict: https://community.adobe.com/questions-671/font-missing-tt-otp-font-versions-appearing-and-reinstall-font-conflict-865600
- Mã nguồn đã đọc: `UX/src-tauri/src/services/duplicates.rs`, `UX/src-tauri/src/dto/analysis.rs` (DuplicateKindDto…), `UX/src-tauri/src/dto/library.rs` (FaceRowDto), `UX/src-tauri/crates/fontatlas-store/src/schema.rs`, `UX/components/duplicates/DuplicatesPage.tsx`.
