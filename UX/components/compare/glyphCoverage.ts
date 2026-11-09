import { useEffect, useRef, useState } from 'react';

type Seg = new (l?: string, o?: object) => { segment(s: string): Iterable<{ segment: string }> };
/** Tách theo grapheme (chữ + dấu đi cùng nhau, kể cả dạng NFD); không có Segmenter thì theo ký tự sau NFC. */
export function graphemes(s: string): string[] {
  const seg = (Intl as { Segmenter?: Seg }).Segmenter;
  return seg ? [...new seg('vi', { granularity: 'grapheme' }).segment(s)].map(x => x.segment) : [...s.normalize('NFC')];
}

type Ctx = Pick<CanvasRenderingContext2D, 'measureText'> & { font: string } & Partial<Pick<CanvasRenderingContext2D, 'fillText' | 'clearRect' | 'getImageData' | 'fillStyle'>>;
const PX = { w: 80, h: 64, size: 40 } as const;
const inkOf = (ctx: Ctx, font: string, c: string): Uint8ClampedArray => {
  ctx.clearRect!(0, 0, PX.w, PX.h); ctx.font = font; ctx.fillStyle = '#000'; ctx.fillText!(c, 4, 48);
  return ctx.getImageData!(0, 0, PX.w, PX.h).data;
};
const sameInk = (a: ArrayLike<number>, b: ArrayLike<number>): boolean => { if (a.length !== b.length) return false; for (let i = 3; i < a.length; i += 4) if (a[i] !== b[i]) return false; return true; };

/**
 * Grapheme nào font không có glyph. Hai phép thử:
 * 1) độ rộng khi dự phòng là monospace / serif / cursive khác nhau → trình duyệt đang vẽ bằng font dự phòng;
 * 2) (canvas thật) hình vẽ của riêng "font" trùng hệt hình của một font dự phòng chuẩn (serif / sans-serif / monospace / mặc định) → glyph đó là glyph hệ thống, không phải của font.
 * Đo trong WKWebView: ký tự CJK thiếu trong web font vẽ ra giống hệt `sans-serif` hệ thống bất kể danh sách font dự phòng, và độ rộng cả ba fallback đều bằng nhau, nên phép 1 bó tay.
 */
export function missingFrom(ctx: Ctx, family: string, chars: readonly string[], size = 64): Set<string> | null {
  try {
    const w = (c: string, fb: string): number => { ctx.font = `${size}px "${family}", ${fb}`; return ctx.measureText(c).width; };
    const canPx = typeof ctx.getImageData === 'function' && typeof ctx.fillText === 'function' && typeof ctx.clearRect === 'function';
    const out = new Set<string>();
    for (const c of chars) {
      if (!c.trim()) continue;
      const a = w(c, 'monospace'); const b = w(c, 'serif'); const d = w(c, 'cursive');
      if (Math.abs(a - b) > 0.01 || Math.abs(b - d) > 0.01) { out.add(c); continue; }
      if (canPx) {
        const own = inkOf(ctx, `${PX.size}px "${family}"`, c);
        if (['serif', 'sans-serif', 'monospace', ''].some(fb => sameInk(own, inkOf(ctx, `${PX.size}px "__fa_no_such_font__"${fb ? `, ${fb}` : ''}`, c)))) out.add(c);
      }
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
  const chars = [...new Set(graphemes(text))].filter(ch => ch.trim());
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
export const MISSING_DEBOUNCE_MS = 150;
/**
 * Grapheme của `text` mà font `family` không có. Rỗng khi chưa đo lần nào / không đo được.
 * Gõ liên tục: GIỮ kết quả cũ của cùng family trong lúc đo lại (không nháy chữ xám / NoteMark), đo lại sau ~150 ms.
 */
export function useMissingChars(family: string | null, text: string): ReadonlySet<string> {
  const [state, setState] = useState<{ family: string; missing: ReadonlySet<string> } | null>(null);
  const had = useRef<string | null>(null);
  useEffect(() => {
    if (!family) return;
    let alive = true;
    const run = () => { void coverageProbe.detect(family, text).then(m => { if (alive) { had.current = family; setState({ family, missing: m ?? NONE }); } }); };
    const h = setTimeout(run, had.current === family ? MISSING_DEBOUNCE_MS : 0);
    return () => { alive = false; clearTimeout(h); };
  }, [family, text]);
  return family && state && state.family === family ? state.missing : NONE;
}

/** Ghi chú chữ cho tooltip/PNG: "ấ ợ …" (tối đa `max` ký tự). */
export function missingList(missing: ReadonlySet<string>, max = 12): string {
  const a = [...missing];
  return a.slice(0, max).join(' ') + (a.length > max ? ' …' : '');
}
