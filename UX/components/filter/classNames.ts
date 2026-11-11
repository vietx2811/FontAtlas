/** Tên chuẩn của usWeightClass / usWidthClass (thuật ngữ typo giữ tiếng Anh). */
export const WEIGHT_NAMES: Record<number, string> = { 100: 'Thin', 200: 'ExtraLight', 300: 'Light', 400: 'Regular', 500: 'Medium', 600: 'SemiBold', 700: 'Bold', 800: 'ExtraBold', 900: 'Black' };
export const WIDTH_NAMES: Record<number, string> = { 1: 'UltraCondensed', 2: 'ExtraCondensed', 3: 'Condensed', 4: 'SemiCondensed', 5: 'Normal', 6: 'SemiExpanded', 7: 'Expanded', 8: 'ExtraExpanded', 9: 'UltraExpanded' };
export const WEIGHT_TICKS = [100, 200, 300, 400, 500, 600, 700, 800, 900];
export const WIDTH_TICKS = [1, 2, 3, 4, 5, 6, 7, 8, 9];
export const weightName = (v: number): string => WEIGHT_NAMES[v] ?? String(v);
export const widthName = (v: number): string => WIDTH_NAMES[v] ?? String(v);
