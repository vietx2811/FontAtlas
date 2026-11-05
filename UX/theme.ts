export const THEME_KEY = 'fa.theme';
export type ThemeMode = 'system' | 'light' | 'dark';

/** Áp theme ngay và nhớ lại (nếu bộ nhớ trình duyệt dùng được). `system` = bỏ ghi đè, theo prefers-color-scheme. */
export function applyTheme(mode: ThemeMode): void {
  if (mode === 'system') document.documentElement.removeAttribute('data-theme');
  else document.documentElement.dataset.theme = mode;
  try { if (mode === 'system') localStorage.removeItem(THEME_KEY); else localStorage.setItem(THEME_KEY, mode); } catch { /* bộ nhớ bị chặn */ }
}
