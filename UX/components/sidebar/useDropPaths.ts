import { useEffect } from 'react';

/** Kéo thả file/thư mục vào cửa sổ (chỉ có trong Tauri; trình duyệt thường bỏ qua). */
export function useDropPaths(onPaths: (paths: string[]) => void): void {
  useEffect(() => {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return;
    let off: (() => void) | undefined;
    let dead = false;
    import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent(e => { if (e.payload.type === 'drop') onPaths(e.payload.paths); }),
    ).then(u => { if (dead) u(); else off = u; }).catch(err => console.warn('drag-drop unavailable', err));
    return () => { dead = true; off?.(); };
  }, [onPaths]);
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
