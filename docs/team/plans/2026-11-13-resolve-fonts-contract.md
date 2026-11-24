# Hợp đồng `resolve_fonts` và nhập từ Figma (bản nháp trên giấy, 08)

> Liên quan: `docs/team/spikes/08_figma_deeplink.md` §3. Chưa có mã. Lối vào MVP: **clipboard / "Dán từ Figma"**; link `fontatlas://` thêm sau khi chủ app duyệt.

## 1. Tải trọng nhập (payload)
Cùng dạng cho clipboard và link (`d` = base64url của JSON):
```json
{ "v": 1, "src": "figma", "file": "Menu v3", "fonts": [["Playfair Display","Bold",0], ["Inter","Regular",1]] }
```
Phần tử `fonts`: `[family, style, missing(0|1)]`. `file` tuỳ chọn. Giới hạn: ≤200 font, mỗi chuỗi ≤128 ký tự, tổng ≤64 KB, UTF-8 hợp lệ.

## 2. Lệnh
### `parse_figma_import(text: String) -> FigmaImportDto`
Nhận văn bản dán (JSON thuần hoặc URL `fontatlas://fonts?v=1&d=…`). Chỉ phân tích, **không** ghi gì. Lỗi theo mã (`err.<mã>`, `detail.value`): `import_empty`, `import_format`, `import_version` (v lạ), `import_too_large`, `import_too_many_fonts`.
```
FigmaImportDto { file: Option<String>, items: Vec<FigmaFontItemDto> }
FigmaFontItemDto { family, style, figma_missing: bool }
```

### `resolve_fonts(items: Vec<FontNameDto{family,style}>) -> Vec<FontMatchDto>`
Một phần tử vào, một phần tử ra, cùng thứ tự.
```
FontMatchDto {
  index: u32,
  status: "exact" | "family_only" | "ambiguous" | "none",
  faces: Vec<FaceMatchDto>,   // đã xếp hạng, tối đa 5
}
FaceMatchDto { face: FaceKeyDto, family, style, via: "style" | "ps_name" | "family",
               active: bool,        // đang kích hoạt (nếu app biết)
               missing: bool }      // file nguồn thiếu
```
- `exact`: đúng 1 face khớp family + style. `ambiguous`: ≥2 (nhiều bản/nguồn); UI cho chọn, không tự chọn. `family_only`: có họ nhưng không có style (vd có Regular, thiếu Bold); trả các style của họ để UI nêu rõ "thiếu style". `none`: không thấy.
- Khớp theo thứ tự: (1) family + style chuẩn hoá; (2) PostScript name khi chuỗi có dạng `Family-Style`; (3) family. Chuẩn hoá: Unicode NFC, hạ chữ thường, gộp khoảng trắng, bỏ dấu `-`/`_` ở style, đồng nghĩa style thường gặp (`Semibold`=`Semi Bold`, `Book`≈`Regular`? **không** gộp mặc định: chỉ `Semibold/Semi Bold/SemiBold`, `Extrabold/Extra Bold`, `Italic/Oblique` KHÔNG gộp).
- Không có điểm 0–100; chỉ `status` và thứ tự. Không đoán: `family_only` không bao giờ thành `exact`.
- Chỉ đọc Index; không đụng file font; bỏ face `missing` khỏi `exact` nhưng vẫn trả kèm `missing=true` để báo "mất file".
- Hiệu năng: 200 mục < 50 ms (tra theo bảng băm family chuẩn hoá dựng sẵn trong `IndexBundle`, không quét tuyến tính).

### `take_pending_imports() -> Vec<PendingImportDto>` (đã làm, SHA ở báo cáo)
Hàng đợi link `fontatlas://` (trần 10). Backend nhận URL qua `RunEvent::Opened` của macOS (không cần crate deep-link: Info.plist do bundler tạo từ `plugins.deep-link.desktop.schemes` trong `tauri.conf.json`), phân tích, xếp hàng, phát sự kiện `figma-import-pending` và focus cửa sổ. Giao diện gọi lệnh này **lúc khởi động** (mở lạnh) và mỗi khi nhận sự kiện. Link sai định dạng vẫn xếp, với `error` = mã lỗi. Chỉ nhận scheme của chính build này (`fontatlas` ở bản phát hành, `fontatlas-dev` ở debug, xem spike §2.3); scheme khác bị bỏ.

### Tạo Case: dùng lệnh sẵn có
`create_case(name = "Figma: <file>")` rồi `add_to_case(case_id, faces, origin)`. **Dùng origin `added`**: cột `case_faces.origin` có CHECK ('added','pinned','brief','peek'), thêm `figma` cần migration dựng lại bảng. Nguồn Figma ghi trong tên Case. Font `exact` tự vào; `ambiguous` chỉ vào sau khi người dùng chọn; `none`/`family_only` không vào, nằm trong danh sách "thiếu" của hộp thoại.

### Font thiếu → hành động sẵn có
"Tìm font tương tự" = `similar` (anchor là font gần nhất cùng họ nếu có, nếu không thì không có anchor và UI nói rõ). "Kích hoạt" = lệnh kích hoạt hiện có cho `active=false`.

## 3. Mã lỗi mới (vi + en trong `i18n/err.ts`, `i18n/en/err.ts`)
`import_empty`, `import_format`, `import_version`, `import_too_large`, `import_too_many_fonts`.

## 4. Chuỗi UI (cho 01 và 05; backend không hiển thị)
Hộp thoại dùng `status` để chọn nhãn: "Có trong thư viện", "Có nhiều bản, chọn một", "Có họ này nhưng thiếu style Bold", "Không có trong thư viện", và `figma_missing` → ghi chú "Figma báo thiếu font này". Thuật ngữ font giữ tiếng Anh, có TermTip. Cảnh báo dùng NoteMark.

## 5. Kiểm thử đề xuất (đỏ trước)
parse: JSON thuần, URL, base64 sai, `v` lạ, quá lớn, quá nhiều font, chuỗi Unicode tiếng Việt. resolve: exact, ambiguous (hai nguồn cùng font), family_only, none, PostScript name, chuẩn hoá `Semibold`, thứ tự đầu ra, 200 mục dưới 50 ms trên thư viện 5k face dựng từ fixture.

## 6. Câu hỏi mở
1. Khi `ambiguous`, mặc định ưu tiên face đang kích hoạt hay nguồn thư viện chính? (đề xuất: không mặc định, người dùng chọn).
2. Có cần nhớ tên file Figma trong Case (`brief`) hay chỉ trong tên Case? Riêng tư: đề xuất cho người dùng bỏ.
