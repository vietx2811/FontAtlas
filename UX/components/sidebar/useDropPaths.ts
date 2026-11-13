import { useEffect } from 'react';

/** Kéo thả file/thư mục vào cửa sổ (chỉ có trong Tauri; trình duyệt thường bỏ qua). */
export type DropAt = { x: number; y: number };
/** `at` = vị trí thả theo px CSS của trang (Tauri báo px vật lý nên đã chia cho devicePixelRatio). `onHover(at|null)` khi đang lơ lửng/rời. */
export function useDropPaths(onPaths: (paths: string[], at?: DropAt) => void, onHover?: (at: DropAt | null) => void): void {
  useEffect(() => {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return;
    let off: (() => void) | undefined;
    let dead = false;
    import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent(e => {
        const p = e.payload;
        const dpr = window.devicePixelRatio || 1;
        const at = 'position' in p && p.position ? { x: p.position.x / dpr, y: p.position.y / dpr } : undefined;
        if (p.type === 'drop') { onHover?.(null); onPaths(p.paths, at); }
        else if (p.type === 'enter' || p.type === 'over') onHover?.(at ?? null);
        else onHover?.(null);
      }),
    ).then(u => { if (dead) u(); else off = u; }).catch(err => console.warn('drag-drop unavailable', err));
    return () => { dead = true; off?.(); };
  }, [onPaths, onHover]);
}

const FONT_EXT = /\.(ttf|otf|ttc|otc|woff2?)$/i;
export const isFontFile = (p: string): boolean => FONT_EXT.test(p);
export const parentDir = (p: string): string => p.slice(0, Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\')));

export async function pickFolderWithDialog(): Promise<string | null> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const r = await open({ directory: true, multiple: false });
  return typeof r === 'string' ? r : null;
}

/** Onboarding có thể nhập nhiều nguồn độc lập trong một lần chọn. */
export async function pickFoldersWithDialog(): Promise<string[]> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return [];
  // QA nội bộ (bản debug + FONTATLAS_QA_SCRIPT): hộp thoại gốc không điều khiển được bằng script.
  const qa = (window as { __FA_QA_PICKS__?: string[][] }).__FA_QA_PICKS__;
  if (Array.isArray(qa)) return qa.shift() ?? [];
  const { open } = await import('@tauri-apps/plugin-dialog');
  const r = await open({ directory: true, multiple: true });
  return typeof r === 'string' ? [r] : r ?? [];
}
