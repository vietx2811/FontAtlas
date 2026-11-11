/** Ngôn ngữ giao diện hiện tại (vi | en). Module-level để `t()` đọc được ở mọi nơi; React đăng ký qua `useLocale`. */
import { useSyncExternalStore } from 'react';

export type Locale = 'vi' | 'en';
let current: Locale = 'vi'; // mặc định cho test; ứng dụng gọi `initLocale` lúc khởi động
const listeners = new Set<() => void>();

export const getLocale = (): Locale => current;
export const subscribeLocale = (fn: () => void): (() => void) => { listeners.add(fn); return () => { listeners.delete(fn); }; };
export const useLocale = (): Locale => useSyncExternalStore(subscribeLocale, getLocale, getLocale);

/** Đổi ngôn ngữ ngay (không ghi setting). Mọi component dùng `useLocale` render lại; `<LocaleBoundary>` dựng lại cây giao diện. */
export function setLocale(next: Locale): void {
  if (next === current) return;
  current = next;
  if (typeof document !== 'undefined') document.documentElement.lang = next;
  for (const fn of [...listeners]) fn();
}

/** Ngôn ngữ ưu tiên của hệ điều hành → vi* thì vi, còn lại en. */
export function localeFromSystem(languages: readonly string[] = typeof navigator !== 'undefined' ? navigator.languages ?? [navigator.language] : []): Locale {
  const first = (languages[0] ?? '').toLowerCase();
  return first.startsWith('vi') ? 'vi' : 'en';
}

/** Chọn ngôn ngữ lúc khởi động: `uiLang` đã lưu ("vi" | "en") → theo đó; chưa chọn (null) → ngôn ngữ hệ thống. */
export function resolveLocale(backendLang: string | null | undefined, systemLanguages?: readonly string[]): Locale {
  if (backendLang === 'vi' || backendLang === 'en') return backendLang;
  return localeFromSystem(systemLanguages);
}

export function initLocale(backendLang: string | null | undefined): Locale {
  const l = resolveLocale(backendLang);
  setLocale(l);
  return l;
}

/** Người dùng chọn ngôn ngữ (onboarding, Cài đặt): áp ngay. Nơi gọi ghi `setSettings({ uiLang })` để nhớ. */
export function chooseLocale(l: Locale): void { chosenThisSession = true; setLocale(l); }
let chosenThisSession = false;
/** Đã có lựa chọn trong phiên này (đọc lại setting sau BootGate không được ghi đè). */
export const hasChosenThisSession = (): boolean => chosenThisSession;
/** Chỉ cho test: xoá dấu "đã chọn". */
export const resetChosenForTests = (): void => { chosenThisSession = false; };

/** Định dạng số/ngày theo ngôn ngữ giao diện. */
export const localeTag = (l: Locale = current): string => (l === 'vi' ? 'vi-VN' : 'en-US');
export const formatNumber = (n: number, opts?: Intl.NumberFormatOptions): string => new Intl.NumberFormat(localeTag(), opts).format(n);
export const formatDate = (d: Date | number, opts: Intl.DateTimeFormatOptions = { dateStyle: 'medium' }): string => new Intl.DateTimeFormat(localeTag(), opts).format(d);
