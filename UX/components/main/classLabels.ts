import { t, type I18nKey } from '../../i18n';

/** `display` là tag mục đích dùng (usage), không phải class: không hiện ở bộ lọc phân loại. */
export const HIDDEN_CLASSES = new Set(['display']);

const has = (k: string): k is I18nKey => k.startsWith('class.');
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
