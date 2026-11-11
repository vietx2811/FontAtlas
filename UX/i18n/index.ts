/** Từ điển chuỗi giao diện (vi). Mọi chuỗi mới đi qua khoá; thuật ngữ font giữ tiếng Anh, chú giải để ở khoá `*.gloss`. */
import { TERM_GLOSS } from './terms';
import { COMPARE_V2 } from './compareV2';
import { ACTIVATION_I18N } from './activation';
import { CLOUD_I18N } from './cloud';
import { REPLACE_I18N } from './replace';
import { ONBOARDING_I18N } from './onboarding';
import { FILTER_I18N, FILTER_I18N_2, FILTER_I18N_3, FILTER_I18N_4 } from './filter';
import { DESK_I18N } from './desk';
import { VIEW_I18N } from './view';
import { DUP_I18N } from './dup';
<<<<<<< HEAD
import { AUTO_I18N } from './auto';
=======
import { SETTINGS_I18N } from './settings';
import { DETAIL_I18N } from './detail';
import { UI_I18N } from './ui';
import { FACES_I18N } from './faces';
import { SIDEBAR_I18N } from './sidebar';
>>>>>>> 1d6bb03e (feat(i18n): move hard-coded Vietnamese UI strings into i18n (sidebar, faces, main, grid, detail, settings, model, zero, boot))
import { getLocale } from './locale';
import { en } from './en';
export { getLocale, setLocale, useLocale, chooseLocale, initLocale, resolveLocale, formatNumber, formatDate, localeTag, type Locale } from './locale';

export const vi = {
  ...TERM_GLOSS,
  ...COMPARE_V2,
  ...ACTIVATION_I18N,
  ...CLOUD_I18N,
  ...REPLACE_I18N,
  ...FILTER_I18N,
  ...FILTER_I18N_2,
  ...FILTER_I18N_3,
  ...FILTER_I18N_4,
  ...DESK_I18N,
  ...VIEW_I18N,
<<<<<<< HEAD
  ...AUTO_I18N,
=======
  ...SETTINGS_I18N,
  ...DETAIL_I18N,
  ...UI_I18N,
  ...FACES_I18N,
  ...SIDEBAR_I18N,
>>>>>>> 1d6bb03e (feat(i18n): move hard-coded Vietnamese UI strings into i18n (sidebar, faces, main, grid, detail, settings, model, zero, boot))
  ...ONBOARDING_I18N,
  ...DUP_I18N,
  'anat.rawValue': 'Giá trị gốc: {v}', 'anat.failed': 'Không đo được', 'anat.unmeasured': 'Chưa đo', 'anat.reasonUnknown': 'không rõ lý do', 'anat.noData': 'Chưa có dữ liệu', 'anat.noRaw': 'Chưa có số đo', 'anat.estimated': 'Ước lượng',
  'anat.percentileTip': 'Vị trí so với tập tham chiếu; không phải độ tin cậy', 'anat.hxgAria': 'Glyph Hxg với đường chữ hoa, chiều cao x và đường nền',
  'anat.guides.mismatch': 'Số đo analyzer khác hình hiển thị', 'anat.guides.mismatchTip': 'Analyzer: cap {capA} · x {xA} em — hình hiển thị: cap {capI} · x {xI} em', 'anat.guides.unavailable': 'Chưa nạp được font để minh hoạ đường kẻ',
  'settings.title': 'Cài đặt',
  'filter.vi.full': 'Đủ dấu', 'filter.vi.supported': 'Có dấu cơ bản', 'filter.vi.clear': 'Bỏ',
  'filter.license.ofl': 'OFL', 'filter.license.apache': 'Apache', 'filter.license.other': 'Khác / thương mại', 'filter.license.unknown': 'Chưa rõ',
  'filter.favorites': 'Yêu thích', 'filter.variable': 'Variable', 'filter.variable.gloss': 'font biến thiên theo trục',
  'filter.italic': 'Italic', 'filter.italic.gloss': 'chữ nghiêng',
  'class.serif': 'Serif', 'class.sans': 'Sans', 'class.mono': 'Mono', 'class.script': 'Script', 'class.slab': 'Slab', 'class.handwriting': 'Handwriting', 'class.display': 'Display', 'class.blackletter': 'Blackletter', 'class.symbol': 'Symbol', 'class.uncertain': 'Uncertain', 'class.unclassified': 'Chưa phân loại',
  'class.serif.gloss': 'có chân', 'class.sans.gloss': 'không chân', 'class.mono.gloss': 'cùng độ rộng', 'class.script.gloss': 'chữ viết nối nét', 'class.slab.gloss': 'chân vuông dày', 'class.handwriting.gloss': 'chữ viết tay, các chữ rời nhau', 'class.display.gloss': 'cấu trúc đặc biệt, cỡ lớn', 'class.blackletter.gloss': 'chữ Gothic', 'class.symbol.gloss': 'ký hiệu', 'class.uncertain.gloss': 'chưa đủ chắc', 'class.unclassified.gloss': 'chưa phân tích',
  'class.edited': 'Đã sửa', 'class.auto': 'Về tự động', 'class.maybe': 'có lẽ là {label}', 'class.machine': 'Máy nghĩ: {label}', 'class.menu': 'Đổi phân loại',
  'class.error': 'Không đổi được phân loại: {msg}',
  'class.signal.measure': 'Số đo', 'class.signal.panose': 'Panose', 'class.signal.os2_class': 'OS/2 class', 'class.signal.name': 'Tên font', 'class.signal.cmap': 'cmap', 'class.signal.post': 'post',
  'class.usage.text': 'Text (đoạn văn)', 'class.usage.display': 'Headline (tiêu đề cỡ lớn)', 'class.usage.ui': 'UI (giao diện)',
  'class.secondary': 'Có thể cũng là {label}', 'class.tip.conf': 'Độ chắc {pct}%', 'class.tip.usage': 'Dùng cho: {usage}',
  'topbar.group': 'Pair / Compare', 'topbar.pair': 'Pair', 'topbar.pairThese': 'Pair these', 'topbar.compare': 'Compare', 'topbar.similar': 'Find similar',
  'topbar.pair.gloss': 'ghép cặp', 'topbar.pairThese.gloss': 'ghép các font đã chọn', 'topbar.compare.gloss': 'so sánh cạnh nhau', 'topbar.similar.gloss': 'tìm font giống',
  'topbar.selected': '{n} chọn', 'topbar.clear': 'Bỏ chọn', 'topbar.narrow': '{n} font đang chọn — nên ≤ {max} để so cho rõ.', 'topbar.narrow.short': 'nên ≤ {max}', 'topbar.narrow.title': 'Chọn quá nhiều để so sánh', 'topbar.useFirst': 'Chỉ lấy {max} đầu',
  'compare.sec.specimen.gloss': 'mẫu chữ', 'compare.sec.metrics.gloss': 'phép đo', 'compare.sec.meta.gloss': 'thông tin', 'compare.row.class.gloss': 'phân loại', 'compare.row.license.gloss': 'giấy phép',
  'compare.percentile': 'Percentile {p}', 'compare.baseNa': 'Mốc không đo được, không có Δ', 'compare.noXh': 'Không đo được x-height', 'compare.viNone': 'Không hỗ trợ tiếng Việt',
  'compare.skipped': 'Không đọc được {n} font: {reason}', 'compare.pngSaved': 'Đã lưu {name}', 'compare.footnote2': '±0: lệch < 1% (hoặc < 0,5°)',
  'topbar.compareCut': 'Compare tối đa 8 font — đang lấy 8 font đầu trong {n}.',
  'compare.viWarnPct': 'Dấu tràn khung · {pct}% em', 'compare.viOkTip': 'Dấu cao hơn chữ hoa {pct}% em', 'compare.noExtras': 'Chưa có dữ liệu — quét lại để cập nhật',
  'compare.rescan': 'Cần quét lại', 'compare.noDelta': 'Không có Δ cho số đo này',
  'meta.vendor': 'Vendor', 'meta.vendor.gloss': 'nhà phát hành (OS/2 vendor ID)', 'meta.years': 'Years', 'meta.years.gloss': 'năm tạo · sửa · bản quyền',
  'meta.style': 'Style', 'meta.style.gloss': 'kiểu chữ', 'meta.family': 'Family', 'meta.family.gloss': 'cả họ font', 'meta.ot': 'OpenType features', 'meta.ot.gloss': 'tính năng chữ',
  'meta.kerning': 'Kerning', 'meta.kerning.gloss': 'chỉnh khoảng cách cặp chữ', 'meta.embedding': 'Embedding', 'meta.embedding.gloss': 'quyền nhúng font (fsType)',
  'meta.upright': 'Upright', 'meta.italic': 'Italic', 'meta.oblique': 'Oblique', 'meta.upright.gloss': 'đứng', 'meta.italic.gloss': 'nghiêng thật', 'meta.oblique.gloss': 'nghiêng giả (xiên)',
  'meta.axis.wght': 'Weight', 'meta.axis.wdth': 'Width', 'meta.axis.opsz': 'Optical size', 'meta.axis.ital': 'Italic axis', 'meta.axis.slnt': 'Slant axis',
  'meta.faces': '{n} kiểu', 'meta.weights': 'Weight {range}', 'meta.hasItalic': 'Có Italic', 'meta.hasOblique': 'Có Oblique', 'meta.hasVariable': 'Variable', 'meta.hasOpsz': 'Optical size',
  'meta.kernPairs': '{pairs} cặp · {classes} cặp lớp', 'meta.kernNone': 'Không có kerning', 'meta.created': 'Tạo {y}', 'meta.modified': 'Sửa {y}', 'meta.copyright': '© {y}',
  'ot.figures': 'Figures', 'ot.figures.gloss': 'kiểu số', 'ot.small_caps': 'Small caps', 'ot.small_caps.gloss': 'chữ hoa nhỏ', 'ot.fractions': 'Fractions', 'ot.fractions.gloss': 'phân số',
  'ot.case': 'Case', 'ot.case.gloss': 'dấu theo chữ hoa', 'ot.ligatures': 'Ligatures', 'ot.ligatures.gloss': 'chữ ghép', 'ot.swash': 'Swash', 'ot.swash.gloss': 'nét hoa mỹ',
  'ot.stylistic_sets': 'Stylistic sets', 'ot.stylistic_sets.gloss': 'bộ kiểu thay thế', 'ot.other': 'Other',
  'emb.installable': 'Installable', 'emb.installable.gloss': 'nhúng tự do', 'emb.editable': 'Editable', 'emb.editable.gloss': 'nhúng và sửa tài liệu', 'emb.preview_print': 'Preview & print', 'emb.preview_print.gloss': 'chỉ xem và in',
  'emb.restricted': 'Restricted', 'emb.restricted.gloss': 'hạn chế nhúng', 'emb.unknown': 'Unknown', 'emb.unknown.gloss': 'chưa rõ', 'emb.no_subsetting': 'Không được cắt tập con', 'emb.bitmap_only': 'Chỉ bitmap',
  'lic.ofl': 'OFL', 'lic.apache': 'Apache', 'lic.ufl': 'UFL', 'lic.eula': 'EULA', 'lic.other': 'Other', 'lic.unknown': 'Unknown',
  'pair.title': 'Pair these', 'pair.back': 'Quay lại thư viện', 'pair.vi': 'Dự án dùng tiếng Việt', 'pair.loading': 'Đang ghép…', 'pair.error': 'Không ghép được: {msg}', 'pair.retry': 'Thử lại',
  'pair.combo': 'Combo {n}', 'pair.heading': 'Tiêu đề', 'pair.body': 'Nội dung', 'pair.empty': 'Không có Combo phù hợp', 'pair.emptyVi': 'Tắt dự án tiếng Việt',
  'pair.gate.safe': 'An toàn', 'pair.gate.controlled': 'Có kiểm soát', 'pair.gate.experimental': 'Thử nghiệm', 'pair.gate.passed': 'Đạt', 'pair.gate.blocked': 'Bị chặn',
  'pair.clash': 'Va chạm: {axes}', 'pair.overflow': 'Tiêu đề dễ tràn dấu', 'pair.unsure': 'Chưa đủ dữ liệu', 'pair.warn.low_x_height': 'Nội dung có x-height thấp', 'pair.warn.low_x_height.gloss': 'chữ thường nhỏ so với em: khó đọc ở cỡ nhỏ (chỉ cảnh báo)',
  'pair.warn.high_contrast': 'Tương phản nét cao', 'pair.warn.high_contrast.gloss': 'nét dày–mảnh chênh mạnh, đoạn dài dễ mỏi mắt',
  'pair.warn.weight_out_of_range': 'Độ đậm ngoài khoảng đọc tốt', 'pair.warn.weight_out_of_range.gloss': 'quá mảnh hoặc quá đậm cho đoạn văn',
  'pair.warn.missing_regular_or_bold': 'Thiếu Regular hoặc Bold', 'pair.warn.missing_regular_or_bold.gloss': 'font nội dung cần đủ Regular và Bold để in đậm',
  'pair.warn.no_vietnamese': 'Thiếu tiếng Việt',
  'pair.clash.global': 'Quá giống nhau', 'pair.clash.skeleton': 'Cùng nét, khác khung',
  'pair.block.vi.heading': 'Tiêu đề thiếu {n} ký tự tiếng Việt', 'pair.block.vi.body': 'Nội dung thiếu {n} ký tự tiếng Việt',
  'pair.legend': '● hoà hợp · ◐ tương phản · ◆ va chạm · ○ chưa chắc · ◑ gợi ý độ đậm', 'pair.cut': 'Pair these tối đa 4 font — đang lấy 4 font đầu trong {n}.',
  'compare.title': 'Compare', 'compare.back': 'Quay lại thư viện', 'compare.sameSize': 'Same size', 'compare.matchXh': 'Match x-height',
  'compare.sameSize.gloss': 'cùng cỡ chữ', 'compare.matchXh.gloss': 'cùng chiều cao chữ thường', 'compare.overlay': 'Overlay', 'compare.overlay.gloss': 'chồng glyph, chung baseline',
  'compare.png': 'PNG', 'compare.pdf': 'PDF', 'compare.pdfSoon': 'PDF chưa có', 'compare.pngFail': 'Không xuất được PNG: {msg}',
  'compare.loading': 'Đang đọc số đo…', 'compare.error': 'Không mở được Compare: {msg}', 'compare.retry': 'Thử lại', 'compare.base': 'mốc',
  'compare.sec.specimen': 'Specimen', 'compare.sec.metrics': 'Số đo', 'compare.sec.class': 'Phân loại', 'compare.sec.meta': 'Meta',
  'compare.row.line': 'Dòng mẫu', 'compare.row.overlay': 'Overlay', 'compare.row.vi': 'Vietnamese', 'compare.row.vi.gloss': 'dấu chồng', 'compare.row.class': 'Classification', 'compare.row.license': 'License',
  'compare.est': 'Ước lượng', 'compare.na': 'Không đo được', 'compare.viOk': 'Dấu vừa khung', 'compare.viWarn': 'Dấu tràn khung', 'compare.viWarnEm': 'Dấu tràn khung · {em} em', 'compare.viMissing': 'Thiếu {n} ký tự',
  'compare.licenseNote': 'Chỉ ghi chú, không chặn', 'compare.footnote': 'Δ so với mốc · thanh = độ lớn so với 0 (hoặc ±0 với góc) trong các font đang xem; percentile thư viện ở tooltip',
  'compare.m.x_height_em': 'x-height', 'compare.m.x_height_em.gloss': 'cao chữ thường', 'compare.m.cap_height_em': 'cap height', 'compare.m.cap_height_em.gloss': 'cao chữ hoa',
  'compare.m.contrast': 'contrast ⊥', 'compare.m.contrast.gloss': 'tương phản nét (vuông góc)', 'compare.m.contrast_h': 'contrast H', 'compare.m.contrast_h.gloss': 'tương phản ngang',
  'compare.m.slant_deg': 'slant I/T', 'compare.m.slant_deg.gloss': 'độ nghiêng (I, T)', 'compare.m.stress_deg': 'stress', 'compare.m.stress_deg.gloss': 'trục nhấn',
  'compare.m.aperture': 'aperture', 'compare.m.aperture.gloss': 'độ mở', 'compare.m.serif_foot': 'serif foot', 'compare.m.serif_foot.gloss': 'chân serif',
} as const;
export type I18nKey = keyof typeof vi;
/** Khoá có trong từ điển hay không (dùng cho khoá ghép động như `class.${x}`). */
export const hasKey = (key: string): key is I18nKey => Object.prototype.hasOwnProperty.call(vi, key);
/** Thiếu khoá thì trả chính khoá thay vì ném lỗi: dữ liệu thật có thể mang giá trị mới (vote, trait…) mà UI chưa dịch. */
const lookup = (key: string): string | undefined => (getLocale() === 'en' ? en[key as I18nKey] : undefined) ?? (vi as Record<string, string>)[key];
export const t = (key: I18nKey, vars: Record<string, string | number> = {}): string => (lookup(key) ?? key).replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? ''));
