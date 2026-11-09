import { useEffect, useState } from 'react';

/**
 * Ký tự nào của một font không có glyph: so độ rộng của ký tự khi dự phòng là `monospace` với khi dự phòng là `serif`.
 * Nếu font có glyph thì hai độ rộng bằng nhau (advance của chính glyph); nếu thiếu, trình duyệt vẽ bằng hai font dự phòng khác nhau → rộng khác nhau.
 * Giới hạn: hai font dự phòng bằng nhau về bề rộng ở một ký tự hiếm (thì coi như có glyph) — chỉ làm lỡ một cảnh báo, không báo nhầm.
 */
export function missingFrom(ctx: Pick<CanvasRenderingContext2D, 'measureText'> & { font: string }, family: string, chars: readonly string[], size = 64): Set<string> | null {
  try {
    const w = (c: string, fb: string): number => { ctx.font = `${size}px "${family}", ${fb}`; return ctx.measureText(c).width; };
    const out = new Set<string>();
    for (const c of chars) {
      if (!c.trim() || c.length === 0) continue;
      if (Math.abs(w(c, 'monospace') - w(c, 'serif')) > 0.01) out.add(c);
    }
    return out;
  } catch { return null; }
}

let probe: CanvasRenderingContext2D | null | undefined;
const ctx2d = (): CanvasRenderingContext2D | null => {
  if (probe !== undefined) return probe;
  try { probe = typeof document === 'undefined' ? null : document.createElement('canvas').getContext('2d'); } catch { probe = null; }
  return probe;
};

const cache = new Map<string, boolean>(); // `${family}\u0000${char}` → thiếu?
export const clearCoverageCache = (): void => cache.clear();

/** Nạp font rồi tìm ký tự thiếu. null = không phát hiện được (chưa đăng ký family / không có canvas). */
export async function detectMissing(family: string, text: string): Promise<Set<string> | null> {
  const c = ctx2d();
  if (!c || typeof document === 'undefined' || !document.fonts) return null;
  const chars = [...new Set([...text.normalize('NFC')])].filter(ch => ch.trim());
  if (chars.length === 0) return new Set();
  try { const faces = await document.fonts.load(`64px "${family}"`, text); if (!faces || faces.length === 0) return null; } catch { return null; }
  const todo = chars.filter(ch => !cache.has(`${family}\u0000${ch}`));
  if (todo.length) {
    const m = missingFrom(c, family, todo);
    if (!m) return null;
    for (const ch of todo) cache.set(`${family}\u0000${ch}`, m.has(ch));
  }
  return new Set(chars.filter(ch => cache.get(`${family}\u0000${ch}`)));
}

/** Điểm gọi (đổi được trong test). */
export const coverageProbe = { detect: detectMissing };

const NONE: ReadonlySet<string> = new Set();
/** Ký tự của `text` mà font `family` không có. Rỗng khi đang đo hoặc không đo được. */
export function useMissingChars(family: string | null, text: string): ReadonlySet<string> {
  const [state, setState] = useState<{ family: string; text: string; missing: ReadonlySet<string> } | null>(null);
  useEffect(() => {
    if (!family) return;
    let alive = true;
    void coverageProbe.detect(family, text).then(m => { if (alive) setState({ family, text, missing: m ?? NONE }); });
    return () => { alive = false; };
  }, [family, text]);
  return family && state && state.family === family && state.text === text ? state.missing : NONE;
}

/** Ghi chú chữ cho tooltip/PNG: "ấ ợ …" (tối đa `max` ký tự). */
export function missingList(missing: ReadonlySet<string>, max = 12): string {
  const a = [...missing];
  return a.slice(0, max).join(' ') + (a.length > max ? ' …' : '');
}
