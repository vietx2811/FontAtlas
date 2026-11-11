import { getLocale } from '../../i18n';
/** Tên ngôn ngữ theo locale giao diện: Intl.DisplayNames(bcp47) → autonym → nameEn → mã. */
const cache = new Map<string, Intl.DisplayNames | null>();
function display(): Intl.DisplayNames | null {
  const loc = getLocale();
  if (!cache.has(loc)) { try { cache.set(loc, new Intl.DisplayNames([loc], { type: 'language' })); } catch { cache.set(loc, null); } }
  return cache.get(loc)!;
}
export function langName(item: { code: string; bcp47?: string; autonym?: string; nameEn?: string }): string {
  try { const n = display()?.of(item.bcp47 ?? item.code); if (n && n !== (item.bcp47 ?? item.code)) return n; } catch { /* mã lạ */ }
  return item.autonym || item.nameEn || item.code;
}
