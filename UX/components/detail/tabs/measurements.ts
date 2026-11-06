// Nhãn UI cho các phép đo canonical. Đơn vị thực tế của từng reading do Rust trả.
export const AXIS_META: Record<string, { label: string }> = {
  // PairLabelDto dùng tên nhóm riêng; giữ nhãn Việt khi anatomy chuyển sang keys canonical.
  x_height: { label: 'Chiều cao x' }, weight: { label: 'Độ đậm nét' }, serif: { label: 'Chân serif' }, stress: { label: 'Trục nhấn' }, slant: { label: 'Độ nghiêng' },
  x_height_em: { label: 'Chiều cao x' }, x_height_cap: { label: 'x / chữ hoa' },
  cap_height_em: { label: 'Chiều cao chữ hoa' }, ascender_em: { label: 'Nét lên' }, descender_em: { label: 'Nét xuống' },
  stem: { label: 'Độ dày thân nét' }, darkness: { label: 'Mật độ mực' }, contrast: { label: 'Tương phản nét' },
  stress_deg: { label: 'Trục nhấn' }, slant_deg: { label: 'Độ nghiêng' }, aperture: { label: 'Độ mở c' }, aperture_e: { label: 'Độ mở e' },
  width: { label: 'Độ rộng o' }, mean_advance: { label: 'Bước chữ trung bình' }, width_cv: { label: 'Biến thiên bước chữ' },
  spacing: { label: 'Khoảng cách chữ' }, serif_foot: { label: 'Chân serif' }, counter: { label: 'Lòng chữ o' },
  terminal: { label: 'Đầu nét c' }, a_storeys: { label: 'Chữ a' }, g_storeys: { label: 'Chữ g' },
};
export const UNIT_LABEL: Record<string, string> = {
  em: 'em', deg: '°', cap_height_ratio: '× cap-height', x_height_ratio: '× x-height', area_ratio: 'tỷ lệ diện tích', ratio: 'tỷ lệ',
};
export const precisionFor = (unit: string | null | undefined): number => unit === 'deg' ? 1 : 3;
export function roundedValue(value: number, unit: string | null | undefined, direction: 'nearest' | 'down' | 'up' = 'nearest'): number {
  const scale = 10 ** precisionFor(unit);
  return (direction === 'down' ? Math.floor(value * scale) : direction === 'up' ? Math.ceil(value * scale) : Math.round(value * scale)) / scale;
}
export const numberText = (value: number, unit?: string | null): string => new Intl.NumberFormat('vi-VN', { useGrouping: false, minimumFractionDigits: precisionFor(unit), maximumFractionDigits: precisionFor(unit) }).format(value);
export const unitText = (unit: string | null): string => unit ? UNIT_LABEL[unit] ?? unit : '';
