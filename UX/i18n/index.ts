/** Từ điển chuỗi giao diện (vi). Mọi chuỗi mới đi qua khoá; thuật ngữ font giữ tiếng Anh, chú giải để ở khoá `*.gloss`. */
const vi = {
  'settings.title': 'Cài đặt',
  'filter.vi': 'Tiếng Việt', 'filter.vi.full': 'Đủ dấu', 'filter.vi.supported': 'Có dấu cơ bản', 'filter.vi.clear': 'Bỏ',
  'filter.license': 'Giấy phép', 'filter.license.ofl': 'OFL', 'filter.license.apache': 'Apache', 'filter.license.other': 'Khác / thương mại', 'filter.license.unknown': 'Chưa rõ',
  'filter.source': 'Nguồn', 'filter.kind': 'Loại font', 'filter.favorites': 'Yêu thích', 'filter.variable': 'Variable', 'filter.variable.gloss': 'font biến thiên theo trục',
  'filter.italic': 'Italic', 'filter.italic.gloss': 'chữ nghiêng', 'filter.forced': 'Đang theo mục bên trái',
  'filter.class': 'Phân loại',
  'class.serif': 'Serif', 'class.sans': 'Sans', 'class.mono': 'Mono', 'class.script': 'Script', 'class.blackletter': 'Blackletter', 'class.symbol': 'Symbol', 'class.uncertain': 'Uncertain', 'class.unclassified': 'Chưa phân loại',
  'class.serif.gloss': 'có chân', 'class.sans.gloss': 'không chân', 'class.mono.gloss': 'cùng độ rộng', 'class.script.gloss': 'chữ viết tay', 'class.blackletter.gloss': 'chữ Gothic', 'class.symbol.gloss': 'ký hiệu', 'class.uncertain.gloss': 'chưa đủ chắc', 'class.unclassified.gloss': 'chưa phân tích',
  'class.edited': 'Đã sửa', 'class.auto': 'Về tự động', 'class.maybe': 'có lẽ là {label}', 'class.machine': 'Máy nghĩ: {label}', 'class.menu': 'Đổi phân loại',
  'class.error': 'Không đổi được phân loại: {msg}',
  'class.signal.measure': 'Số đo', 'class.signal.panose': 'Panose', 'class.signal.os2_class': 'OS/2 class', 'class.signal.name': 'Tên font', 'class.signal.cmap': 'cmap', 'class.signal.post': 'post',
  'class.usage.text': 'Text (đoạn văn)', 'class.usage.display': 'Display (tiêu đề lớn)', 'class.usage.ui': 'UI (giao diện)',
  'class.tip.conf': 'Độ chắc {pct}%', 'class.tip.usage': 'Dùng cho: {usage}',
  'topbar.group': 'Pair / Compare', 'topbar.pair': 'Pair', 'topbar.pairThese': 'Pair these', 'topbar.compare': 'Compare', 'topbar.similar': 'Find similar',
  'topbar.pair.gloss': 'ghép cặp', 'topbar.pairThese.gloss': 'ghép các font đã chọn', 'topbar.compare.gloss': 'so sánh cạnh nhau', 'topbar.similar.gloss': 'tìm font giống',
  'topbar.pairSoon': 'Pair these chưa có — đang mở tab Pair của font đầu tiên.',
  'topbar.selected': '{n} chọn', 'topbar.clear': 'Bỏ chọn', 'topbar.narrow': '{n} font — nên ≤ 4 để so rõ.', 'topbar.useFirst': 'Chỉ lấy 4 đầu',
  'topbar.compareSoon': 'Compare chưa có — đang mở Giải phẫu của font đầu tiên.',
  'chips.label': 'Bộ lọc đang bật', 'chips.remove': 'Bỏ lọc {label}',
} as const;
export type I18nKey = keyof typeof vi;
export const t = (key: I18nKey, vars: Record<string, string | number> = {}): string => vi[key].replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? ''));
