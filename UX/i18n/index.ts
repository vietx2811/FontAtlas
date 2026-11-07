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
  'compare.sec.specimen.gloss': 'mẫu chữ', 'compare.sec.metrics.gloss': 'phép đo', 'compare.sec.class.gloss': 'phân loại', 'compare.sec.meta.gloss': 'thông tin', 'compare.row.class.gloss': 'phân loại', 'compare.row.license.gloss': 'giấy phép',
  'compare.percentile': 'Percentile {p}', 'compare.baseNa': 'Mốc không đo được, không có Δ', 'compare.noXh': 'Không đo được x-height', 'compare.viNone': 'Không hỗ trợ tiếng Việt',
  'compare.skipped': 'Không đọc được {n} font: {reason}', 'compare.pngSaved': 'Đã lưu {name}', 'compare.pngCancelled': '', 'compare.footnote2': '±0: lệch < 1% (hoặc < 0,5°)',
  'compare.fontsPending': 'Đang nạp font để xuất…', 'topbar.compareCut': 'Compare tối đa 8 font — đang lấy 8 font đầu trong {n}.',
  'compare.title': 'Compare', 'compare.back': 'Quay lại thư viện', 'compare.sameSize': 'Same size', 'compare.matchXh': 'Match x-height',
  'compare.sameSize.gloss': 'cùng cỡ chữ', 'compare.matchXh.gloss': 'cùng chiều cao chữ thường', 'compare.overlay': 'Overlay', 'compare.overlay.gloss': 'chồng glyph, chung baseline',
  'compare.png': 'PNG', 'compare.pdf': 'PDF', 'compare.pdfSoon': 'PDF chưa có', 'compare.pngFail': 'Không xuất được PNG: {msg}',
  'compare.loading': 'Đang đọc số đo…', 'compare.error': 'Không mở được Compare: {msg}', 'compare.retry': 'Thử lại', 'compare.base': 'mốc',
  'compare.sec.specimen': 'Specimen', 'compare.sec.metrics': 'Số đo', 'compare.sec.class': 'Phân loại', 'compare.sec.meta': 'Meta',
  'compare.row.line': 'Dòng mẫu', 'compare.row.overlay': 'Overlay', 'compare.row.vi': 'Dấu chồng', 'compare.row.class': 'Classification', 'compare.row.license': 'License',
  'compare.est': 'Ước lượng', 'compare.na': 'Không đo được', 'compare.viOk': 'Dấu vừa khung', 'compare.viWarn': 'Dấu tràn khung', 'compare.viWarnEm': 'Dấu tràn khung · {em} em', 'compare.viMissing': 'Thiếu {n} ký tự',
  'compare.licenseNote': 'Chỉ ghi chú, không chặn', 'compare.footnote': 'Δ so với mốc · thanh = vị trí trong thư viện (percentile)',
  'compare.m.x_height_em': 'x-height', 'compare.m.x_height_em.gloss': 'cao chữ thường', 'compare.m.cap_height_em': 'cap height', 'compare.m.cap_height_em.gloss': 'cao chữ hoa',
  'compare.m.contrast': 'contrast ⊥', 'compare.m.contrast.gloss': 'tương phản nét (vuông góc)', 'compare.m.contrast_h': 'contrast H', 'compare.m.contrast_h.gloss': 'tương phản ngang',
  'compare.m.slant_deg': 'slant I/T', 'compare.m.slant_deg.gloss': 'độ nghiêng (I, T)', 'compare.m.stress_deg': 'stress', 'compare.m.stress_deg.gloss': 'trục nhấn',
  'compare.m.aperture': 'aperture', 'compare.m.aperture.gloss': 'độ mở', 'compare.m.serif_foot': 'serif foot', 'compare.m.serif_foot.gloss': 'chân serif',
  'chips.label': 'Bộ lọc đang bật', 'chips.remove': 'Bỏ lọc {label}',
} as const;
export type I18nKey = keyof typeof vi;
/** Khoá có trong từ điển hay không (dùng cho khoá ghép động như `class.${x}`). */
export const hasKey = (key: string): key is I18nKey => Object.prototype.hasOwnProperty.call(vi, key);
/** Thiếu khoá thì trả chính khoá thay vì ném lỗi: dữ liệu thật có thể mang giá trị mới (vote, trait…) mà UI chưa dịch. */
export const t = (key: I18nKey, vars: Record<string, string | number> = {}): string => ((vi as Record<string, string>)[key] ?? key).replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? ''));
