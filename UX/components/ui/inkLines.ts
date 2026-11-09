import { useEffect, useState } from 'react';

/** Số đo mực thật của chữ đang vẽ (em, baseline = 0). null: không đo được (chưa nạp / không có canvas). */
export type Ink = { cap: number | null; x: number | null };
export type Analyzer = { cap: number | null; x: number | null };
export const GUIDE_TOLERANCE_EM = 0.015;

/** Đo `actualBoundingBoxAscent` của 'H' và 'x' bằng canvas với đúng chuỗi font đó. Không bao giờ ném lỗi. */
export function measureInkWith(ctx: Pick<CanvasRenderingContext2D, 'measureText'> & { font: string }, font: string, size = 100): Ink | null {
  try {
    ctx.font = font;
    const asc = (s: string): number | null => { const a = ctx.measureText(s).actualBoundingBoxAscent; return typeof a === 'number' && Number.isFinite(a) && a > 0 ? a / size : null; };
    const cap = asc('H'); const x = asc('x');
    return cap === null && x === null ? null : { cap, x };
  } catch { return null; }
}

/** Chuỗi font dùng CHUNG cho chữ vẽ và phép đo: đúng face đã nạp, không weight/italic theo style đang chọn. */
export const inkFont = (family: string, size = 100): string => `${size}px "${family}"`;

export type GuideState =
  | { status: 'pending' }
  | { status: 'unavailable' }
  | { status: 'ok' | 'mismatch'; cap: number | null; x: number | null; analyzer: Analyzer; ink: Ink };

/** Đường kẻ luôn theo chữ trên màn hình (ink); lệch analyzer quá ngưỡng thì vẫn vẽ theo chữ và báo `mismatch` (không giấu sai số). */
export function resolveGuides(analyzer: Analyzer, ink: Ink | null, tol = GUIDE_TOLERANCE_EM): GuideState {
  if (!ink) return { status: 'unavailable' };
  const off = (a: number | null, b: number | null) => a !== null && b !== null && Math.abs(a - b) > tol;
  return { status: off(analyzer.cap, ink.cap) || off(analyzer.x, ink.x) ? 'mismatch' : 'ok', cap: ink.cap, x: ink.x, analyzer, ink };
}

let probe: CanvasRenderingContext2D | null | undefined;
const ctx2d = (): CanvasRenderingContext2D | null => {
  if (probe !== undefined) return probe;
  try { probe = typeof document === 'undefined' ? null : document.createElement('canvas').getContext('2d'); } catch { probe = null; }
  return probe;
};

/** Chờ font nạp rồi đo; null nếu font không có thật trong `document.fonts` (canvas sẽ vẽ bằng font dự phòng) hoặc không có canvas. */
export async function measureLoadedInk(family: string): Promise<Ink | null> {
  const c = ctx2d();
  if (!c || typeof document === 'undefined' || !document.fonts) return null;
  const font = inkFont(family);
  // `check()` trả true cả khi chưa có FontFace nào khớp; chỉ tin kết quả của `load()` (rỗng = family chưa đăng ký → canvas sẽ vẽ bằng font dự phòng).
  try { const faces = await document.fonts.load(font, 'Hx'); if (!faces || faces.length === 0) return null; } catch { return null; }
  return measureInkWith(c, font);
}

/** Điểm gọi đo (đổi được trong test). */
export const inkProbe = { measure: measureLoadedInk };

/** Đo mực của một family đã nạp. `undefined` = đang đo; null = không đo được. */
export function useInk(family: string | null): Ink | null | undefined {
  // Lưu kèm family: lượt render đầu sau khi đổi family không bao giờ dùng số đo của font cũ.
  const [state, setState] = useState<{ family: string; ink: Ink | null } | null>(null);
  useEffect(() => {
    if (!family) return;
    let alive = true;
    void inkProbe.measure(family).then(v => { if (alive) setState({ family, ink: v }); });
    return () => { alive = false; };
  }, [family]);
  return family && state && state.family === family ? state.ink : undefined;
}

/**
 * Trạng thái đường kẻ cho một face. Không có family: face ≥ 1 của TTC → unavailable ngay; còn lại chờ `graceMs` rồi unavailable.
 * Không bao giờ trả số đo của font này để vẽ lên chữ của font khác.
 */
export function useGuides(family: string | null, analyzer: Analyzer, faceIndex: number, graceMs = 4000): GuideState {
  const ink = useInk(family);
  const [late, setLate] = useState(false);
  useEffect(() => {
    setLate(false);
    if (family || faceIndex > 0) return;
    const h = setTimeout(() => setLate(true), graceMs);
    return () => clearTimeout(h);
  }, [family, faceIndex, graceMs]);
  const cap = analyzer.cap; const x = analyzer.x;
  if (!family) return faceIndex > 0 || late ? { status: 'unavailable' } : { status: 'pending' };
  if (ink === undefined) return { status: 'pending' };
  return resolveGuides({ cap, x }, ink);
}
