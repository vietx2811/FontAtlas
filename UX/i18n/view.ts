/** Chuỗi panel "Hiển thị" (spec library-view-options A3). */
export const VIEW_I18N = {
  'view.title': 'Hiển thị', 'view.open': 'Mở Hiển thị', 'view.tip': 'Chọn thông tin hiện trên thẻ font', 'view.info': 'Thông tin trên thẻ', 'view.reset': 'Đặt lại',
  'view.field.format': 'Định dạng file', 'view.field.format.gloss': 'Đuôi file của font: TTF, OTF, TTC, WOFF, WOFF2…',
  'view.field.lang': 'Ngôn ngữ', 'view.field.lang.gloss': 'Huy hiệu độ phủ của font với ngôn ngữ đang xem (tiếng Việt mặc định).',
  'view.field.license': 'Giấy phép', 'view.field.license.gloss': 'Nhóm giấy phép: OFL, Apache, Khác, Chưa rõ. Chỉ để tham khảo, không chặn bật font.',
  'view.field.variable': 'Variable', 'view.field.variable.gloss': 'Variable font: một file chứa nhiều độ đậm, độ rộng… chạy liên tục theo trục.',
  'view.license.ofl': 'OFL', 'view.license.apache': 'Apache', 'view.license.other': 'Khác', 'view.license.unknown': 'Chưa rõ',
  'view.var.tip': 'Variable font: có trục biến thiên',
  'view.sort': 'Sắp xếp theo', 'view.sort.family': 'Tên', 'view.sort.added': 'Ngày thêm vào thư viện', 'view.sort.weight': 'Độ đậm (Weight)', 'view.sort.match': 'Khớp với tìm kiếm',
  'view.sort.dir': 'Chiều sắp xếp',
  'view.sort.family.asc': 'A → Z', 'view.sort.family.desc': 'Z → A', 'view.sort.added.asc': 'Cũ → Mới', 'view.sort.added.desc': 'Mới → Cũ', 'view.sort.weight.asc': 'Nhẹ → Nặng', 'view.sort.weight.desc': 'Nặng → Nhẹ', 'view.sort.match.asc': 'Khớp nhất trước', 'view.sort.match.desc': 'Khớp nhất sau',
  'view.sort.src.family': 'Theo tên font; trong một family xếp Weight tăng dần.', 'view.sort.src.added': 'Lúc Font Atlas lần đầu thấy font. Lần quét đầu cả thư viện cùng giờ, nên hữu ích nhất cho font thêm sau.',
  'view.sort.src.weight': 'Theo usWeightClass trong bảng OS/2 của font.', 'view.sort.src.match': 'Độ liên quan với chữ đang tìm.',
  'view.sort.locked.recent': 'Thứ tự theo lần mở gần nhất', 'view.sort.locked.active': 'Thứ tự theo lần bật',
} as const;
