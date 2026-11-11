import { termName } from '../../ui/termNames';
import { t, localeTag } from '../../../i18n';
// Nhãn UI cho các phép đo canonical. Đơn vị thực tế của từng reading do Rust trả.
/** Nhãn trục = tên EN dùng chung với Compare (components/ui/termNames.ts); phần tiếng Việt nằm ở gloss trong TermTip. */
const AXIS_KEYS = ['x_height', 'weight', 'serif', 'stress', 'slant', 'x_height_em', 'x_height_cap', 'cap_height_em', 'ascender_em', 'descender_em', 'stem', 'darkness', 'contrast', 'contrast_h', 'stress_deg', 'slant_deg', 'aperture', 'aperture_e', 'width', 'mean_advance', 'width_cv', 'spacing', 'serif_foot', 'counter', 'terminal', 'a_storeys', 'g_storeys'];
export const AXIS_META: Record<string, { label: string }> = Object.fromEntries(AXIS_KEYS.map(k => [k, { label: termName(k) }]));
export const unitLabels = (): Record<string, string> => ({
  em: 'em', deg: '°', cap_height_ratio: '× cap-height', x_height_ratio: '× x-height', area_ratio: t('unit.area_ratio'), ratio: t('unit.ratio'),
});
export const precisionFor = (unit: string | null | undefined): number => unit === 'deg' ? 1 : 3;
export function roundedValue(value: number, unit: string | null | undefined, direction: 'nearest' | 'down' | 'up' = 'nearest'): number {
  const digits = precisionFor(unit);
  // Dịch dấu thập phân qua exponent để 1.001 × 1000 không thành 1000.9999999999.
  const [coefficient, exponent = '0'] = String(value).split('e');
  const shifted = Number(`${coefficient}e${Number(exponent) + digits}`);
  const integer = direction === 'down' ? Math.floor(shifted) : direction === 'up' ? Math.ceil(shifted) : Math.round(shifted);
  return integer / 10 ** digits;
}
export const numberText = (value: number, unit?: string | null): string => new Intl.NumberFormat(localeTag(), { useGrouping: false, minimumFractionDigits: precisionFor(unit), maximumFractionDigits: precisionFor(unit) }).format(value);
export const unitText = (unit: string | null): string => unit ? unitLabels()[unit] ?? unit : '';
