// Nền tảng của giao diện: macOS là mặc định (không đoán được = macOS) nên bản mac không đổi gì; Windows/Linux đổi chữ, phím, vật liệu cửa sổ.
export type Os = 'mac' | 'windows' | 'linux';

let forced: Os | null = null;
/** Test: ép nền tảng (null = đọc lại từ trình duyệt). */
export function setOsForTest(os: Os | null): void { forced = os; }

export type NavLike = { userAgentData?: { platform?: string }; platform?: string; userAgent?: string };
/** Đọc nền tảng từ một đối tượng kiểu `navigator` (export để test bảng WebView thật). Không nhận ra = macOS. */
export function detect(nav: NavLike | undefined = typeof navigator === 'undefined' ? undefined : navigator): Os {
  if (!nav) return 'mac';
  const s = `${nav.userAgentData?.platform ?? ''} ${nav.platform ?? ''} ${nav.userAgent ?? ''}`;
  if (/mac|darwin|iphone|ipad/i.test(s)) return 'mac';
  if (/\bwin/i.test(s)) return 'windows';
  if (/linux|x11|cros/i.test(s) && !/android/i.test(s)) return 'linux';
  return 'mac';
}
let cached: Os | null = null;
export function getOs(): Os { return forced ?? (cached ??= detect()); }
export const isMac = (): boolean => getOs() === 'mac';

/** Phím chính của lối tắt (⌘ trên mac, Ctrl nơi khác): đã giữ chưa. */
export const modKey = (e: { metaKey?: boolean; ctrlKey?: boolean }): boolean => (isMac() ? e.metaKey === true : e.ctrlKey === true);
/** Tên phím modifier mà sự kiện keydown/keyup báo cho `modKey`. */
export const modKeyName = (): string => (isMac() ? 'Meta' : 'Control');

const WORD: Record<string, string> = { '⌘': 'Ctrl', '⌃': 'Ctrl', '⌥': 'Alt', '⇧': 'Shift' };
const ORDER = ['Ctrl', 'Alt', 'Shift'];
/** Chữ phím tắt cho nền tảng hiện tại: "⌥⌘F" → "Alt+Ctrl+F" trên Windows/Linux (thứ tự Ctrl, Alt, Shift); mac giữ nguyên. */
export function shortcutText(s: string): string {
  if (isMac()) return s;
  const mods: string[] = []; let i = 0;
  while (i < s.length && WORD[s[i]]) { const w = WORD[s[i]]; if (!mods.includes(w)) mods.push(w); i++; }
  if (i === 0) return s;
  mods.sort((a, b) => ORDER.indexOf(a) - ORDER.indexOf(b));
  return [...mods, s.slice(i)].join('+');
}
/** Danh sách phím rời (['⌥','⌘','F']) → cùng thứ tự hiển thị đã đổi cho nền tảng. */
export function shortcutKeys(keys: readonly string[]): string[] {
  return isMac() ? [...keys] : shortcutText(keys.join('')).split('+');
}
