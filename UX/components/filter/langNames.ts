/** Tên ngôn ngữ theo locale giao diện: Intl.DisplayNames(bcp47) → autonym → nameEn → mã. */
const cache = new Map<string, Intl.DisplayNames | null>();
function display(): Intl.DisplayNames | null {
  if (!cache.has('vi')) { try { cache.set('vi', new Intl.DisplayNames(['vi'], { type: 'language' })); } catch { cache.set('vi', null); } }
  return cache.get('vi')!;
}
export function langName(item: { code: string; bcp47?: string; autonym?: string; nameEn?: string }): string {
  try { const n = display()?.of(item.bcp47 ?? item.code); if (n && n !== (item.bcp47 ?? item.code)) return n; } catch { /* mã lạ */ }
  return item.autonym || item.nameEn || item.code;
}
