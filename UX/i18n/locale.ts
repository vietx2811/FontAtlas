/** Ngôn ngữ giao diện hiện tại (vi | en). Module-level để `t()` đọc được ở mọi nơi; React đăng ký qua `useLocale`. */
import { useSyncExternalStore } from 'react';

export type Locale = 'vi' | 'en';
/** Dấu "người dùng đã chọn": backend trả `uiLang` mặc định "vi" khi chưa đặt nên không phân biệt được chọn hay chưa. */
export const LOCALE_CHOSEN_KEY = 'fa.lang';

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

const read = (): string | null => { try { return localStorage.getItem(LOCALE_CHOSEN_KEY); } catch { return null; } };
const write = (l: Locale): void => { try { localStorage.setItem(LOCALE_CHOSEN_KEY, l); } catch { /* WebView chặn lưu trữ: bỏ qua */ } };

/** Ngôn ngữ ưu tiên của hệ điều hành → vi* thì vi, còn lại en. */
export function localeFromSystem(languages: readonly string[] = typeof navigator !== 'undefined' ? navigator.languages ?? [navigator.language] : []): Locale {
  const first = (languages[0] ?? '').toLowerCase();
  return first.startsWith('vi') ? 'vi' : 'en';
}

/**
 * Chọn ngôn ngữ lúc khởi động: người dùng đã chọn (dấu cục bộ) → theo đó; backend lưu "en" → en (chắc chắn là lựa chọn,
 * vì mặc định backend là "vi"); còn lại theo ngôn ngữ hệ điều hành.
 */
export function resolveLocale(backendLang: string | null | undefined, chosen: string | null = read(), systemLanguages?: readonly string[]): Locale {
  if (chosen === 'vi' || chosen === 'en') return chosen;
  if (backendLang === 'en') return 'en';
  return localeFromSystem(systemLanguages);
}

export function initLocale(backendLang: string | null | undefined): Locale {
  const l = resolveLocale(backendLang);
  setLocale(l);
  return l;
}

/** Người dùng chọn ngôn ngữ (onboarding, Cài đặt): áp ngay và nhớ là đã chọn. Backend do nơi gọi ghi qua `setSettings({ uiLang })`. */
export function chooseLocale(l: Locale): void { write(l); setLocale(l); }

/** Định dạng số/ngày theo ngôn ngữ giao diện. */
export const localeTag = (l: Locale = current): string => (l === 'vi' ? 'vi-VN' : 'en-US');
export const formatNumber = (n: number, opts?: Intl.NumberFormatOptions): string => new Intl.NumberFormat(localeTag(), opts).format(n);
export const formatDate = (d: Date | number, opts: Intl.DateTimeFormatOptions = { dateStyle: 'medium' }): string => new Intl.DateTimeFormat(localeTag(), opts).format(d);
