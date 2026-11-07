/** Từ điển chuỗi giao diện (vi). Mọi chuỗi mới đi qua khoá; thuật ngữ font giữ tiếng Anh, chú giải để ở khoá `*.gloss`. */
const vi = {
  'settings.title': 'Cài đặt',
  'filter.vi': 'Tiếng Việt', 'filter.vi.full': 'Đủ dấu', 'filter.vi.supported': 'Có dấu cơ bản', 'filter.vi.clear': 'Bỏ',
  'filter.license': 'Giấy phép', 'filter.license.ofl': 'OFL', 'filter.license.apache': 'Apache', 'filter.license.other': 'Khác / thương mại', 'filter.license.unknown': 'Chưa rõ',
  'filter.source': 'Nguồn', 'filter.kind': 'Loại font', 'filter.favorites': 'Yêu thích', 'filter.variable': 'Variable', 'filter.variable.gloss': 'font biến thiên theo trục',
  'filter.italic': 'Italic', 'filter.italic.gloss': 'chữ nghiêng', 'filter.forced': 'Đang theo mục bên trái',
  'chips.label': 'Bộ lọc đang bật', 'chips.remove': 'Bỏ lọc {label}',
} as const;
export type I18nKey = keyof typeof vi;
export const t = (key: I18nKey, vars: Record<string, string | number> = {}): string => vi[key].replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? ''));
