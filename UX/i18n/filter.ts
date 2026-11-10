/** Chuỗi cho bộ lọc nâng cao (2a). Tên nhóm là từ thường dùng; thuật ngữ font giữ tiếng Anh. */
export const FILTER_I18N = {
  'fgroup.class': 'Phân loại', 'fgroup.style': 'Style', 'fgroup.vi': 'Tiếng Việt', 'fgroup.library': 'Thư viện', 'fgroup.license': 'Giấy phép', 'fgroup.axes': 'Số đo',
  'fgroup.more': '+{n}', 'fgroup.edit': 'Sửa', 'fgroup.remove': 'Bỏ nhóm', 'fgroup.removeValue': 'Bỏ {label}', 'fgroup.clearAll': 'Xoá tất cả',
  'fhist.undo': 'Hoàn tác', 'fhist.toast': 'Đã bỏ bộ lọc “{label}”', 'fhist.toastAll': 'Đã xoá tất cả bộ lọc',
} as const;

export const FILTER_I18N_2 = {
  'adv.open': 'Advanced', 'adv.title': 'Advanced filters', 'adv.close': 'Đóng', 'adv.reset': 'Đặt lại', 'adv.error': 'Không tải được miền số đo: {msg}', 'adv.retry': 'Thử lại',
  'adv.class': 'Phân loại', 'adv.style': 'Style', 'adv.library': 'Thư viện', 'adv.license': 'Giấy phép', 'adv.vi': 'Tiếng Việt',
  'adv.measured': 'measured', 'adv.osClass': 'OS/2 class', 'adv.varMatch': 'Font Variable cũng khớp khi dải trục {axis} giao khoảng này',
  'adv.noData': 'Chưa có số đo', 'adv.count': '{n} điều kiện',
} as const;

export const FILTER_I18N_3 = {
  'adv.lang': 'Ngôn ngữ', 'lang.search': 'Tìm ngôn ngữ', 'lang.incomplete': 'Đang tính độ phủ ngôn ngữ ({done}/{total} font): số đếm chỉ là phần đã tính.',
  'lang.mode': 'Cách ghép ngôn ngữ', 'lang.mode.all': 'Đủ tất cả', 'lang.mode.any': 'Một trong các ngôn ngữ', 'lang.counts': '{full} font đủ 100% · {near} font gần đủ (≥ 90%) trở lên',
  'lang.levels': '● đủ 100% · ◐ gần đủ (≥ 90%, đủ dấu và chữ riêng)', 'lang.full': 'đủ', 'lang.near': 'gần đủ',
  'fgroup.lang': 'Ngôn ngữ',
  'badge.full': '{lang} · đủ ký tự', 'badge.near': '{lang} · gần đủ ({have}/{total} ký tự)', 'badge.partial': '{lang} · thiếu {miss}/{total} ký tự', 'badge.low': '{lang} · thiếu hầu hết ({have}/{total} ký tự)', 'badge.below': '{lang} · chưa đủ ký tự (dưới 90%)', 'badge.unknown': '{lang} · chưa phân tích',
  'credit.hyperglot': 'Dữ liệu ngôn ngữ: Hyperglot © Rosetta, Apache-2.0',
} as const;

export const FILTER_I18N_4 = {
  'fgroup.title': 'Đang lọc', 'fgroup.lang.all': 'đủ tất cả', 'fgroup.lang.any': 'một trong',
  'search.placeholder': 'Tìm font', 'search.clear': 'Xoá tìm kiếm', 'slider.min': 'tối thiểu', 'slider.max': 'tối đa',
  'lang.state.off': 'chưa chọn', 'lang.state.full': 'đủ', 'lang.state.near': 'gần đủ', 'lang.row': '{name}: {state}',
} as const;
