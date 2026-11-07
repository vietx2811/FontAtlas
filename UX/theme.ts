export const THEME_KEY = 'fa.theme';
export type ThemeMode = 'system' | 'light' | 'dark';

/** Áp theme ngay và nhớ lại (nếu bộ nhớ trình duyệt dùng được). `system` = bỏ ghi đè, theo prefers-color-scheme. */
export function applyTheme(mode: ThemeMode): void {
  if (mode === 'system') document.documentElement.removeAttribute('data-theme');
  else document.documentElement.dataset.theme = mode;
  try { if (mode === 'system') localStorage.removeItem(THEME_KEY); else localStorage.setItem(THEME_KEY, mode); } catch { /* bộ nhớ bị chặn */ }
  syncNativeTheme(mode);
}

/** Đọc theme đã lưu; không có hoặc hỏng → 'system'. */
export function savedTheme(): ThemeMode {
  let saved: string | null = null;
  try { saved = localStorage.getItem(THEME_KEY); } catch { /* bộ nhớ bị chặn */ }
  return saved === 'light' || saved === 'dark' ? saved : 'system';
}

/**
 * Vibrancy của sidebar (NSVisualEffectView) theo appearance của cửa sổ native, không theo CSS.
 * Đặt theme cửa sổ để lớp kính sáng/tối khớp theme trong app; null = theo hệ thống.
 * Chỉ chạy trong Tauri; trình duyệt/test bỏ qua.
 */
export function syncNativeTheme(mode: ThemeMode): void {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return;
  void import('@tauri-apps/api/window')
    .then(({ getCurrentWindow }) => getCurrentWindow().setTheme(mode === 'system' ? null : mode))
    .catch(() => { /* không đổi được thì CSS vẫn đúng, chỉ lớp kính lệch */ });
}
