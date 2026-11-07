import { hasKey, t } from '../../i18n';

/** `display` là tag mục đích dùng (usage), không phải class: không hiện ở bộ lọc phân loại. */
export const HIDDEN_CLASSES = new Set(['display']);

// Chỉ coi là có nhãn khi khoá thật sự có trong từ điển (vote/trait mới từ backend không làm crash).
const has = hasKey;
export function classLabel(cls: string): string {
  const k = `class.${cls}`;
  return has(k) ? (t as (k: string) => string)(k) : cls;
}
export function classGloss(cls: string): string | undefined {
  const k = `class.${cls}.gloss`;
  return has(k) ? (t as (k: string) => string)(k) : undefined;
}
/** snake_case → "Old style"; thuật ngữ font giữ tiếng Anh. */
export const subclassLabel = (s: string): string => { const x = s.replace(/_/g, ' '); return x.charAt(0).toUpperCase() + x.slice(1); };

/** Trait từ backend (vd `fat_face`, `Inline-Outline`): bỏ gạch dưới, viết hoa chữ đầu; trait mới chưa dịch vẫn đọc được. */
export const traitLabel = (x: string): string => { const y = x.replace(/_/g, ' ').trim(); return y.charAt(0).toUpperCase() + y.slice(1); };
