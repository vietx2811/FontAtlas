// Trạng thái "splash đang mở" dùng chung: Onboarding và phím tắt toàn cục không được nhận phím khi lớp phủ còn hiện.
import { useSyncExternalStore } from 'react';

let open = false;
const subs = new Set<() => void>();
export const isSplashOpen = (): boolean => open;
export function setSplashOpen(v: boolean): void { if (open !== v) { open = v; subs.forEach(f => f()); } }
export const useSplashOpen = (): boolean => useSyncExternalStore(f => { subs.add(f); return () => { subs.delete(f); }; }, () => open, () => false);

/** Công tắc Settings → Chung "Hiện splash khi mở app" (mặc định bật). Lưu ở khoá chung `ui.splash` ("off" = tắt), không cần đổi backend. */
export const SPLASH_KEY = 'ui.splash';
export async function readSplashEnabled(api: { getSetting(k: string): Promise<string | null> }): Promise<boolean> {
  try { return (await api.getSetting(SPLASH_KEY)) !== 'off'; } catch { return true; }
}
export const writeSplashEnabled = (api: { setSetting(k: string, v: string): Promise<void> }, on: boolean): Promise<void> => api.setSetting(SPLASH_KEY, on ? 'on' : 'off');
