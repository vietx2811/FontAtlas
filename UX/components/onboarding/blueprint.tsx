// Ngôn ngữ hình của onboarding: bản vẽ kỹ thuật / blueprint. Mọi đường viền glyph là đường vẽ thật của Inter (OFL), không phải <text>.
// Màu chỉ qua class (CSS dùng token theme) nên đổi sáng/tối theo ứng dụng; animation chỉ transform/opacity/stroke-dashoffset.
import { useEffect, useRef, useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { GLYPHS, UPM } from './glyphOutlines';
import './blueprint.css';

export const VB_W = 480;
export const VB_H = 320;
export const delay = (ms: number): CSSProperties => ({ ['--d' as string]: `${ms}ms` });

export function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    if (typeof matchMedia !== 'function') return;
    const q = matchMedia('(prefers-reduced-motion: reduce)');
    const on = () => setReduced(q.matches);
    on();
    q.addEventListener?.('change', on);
    return () => q.removeEventListener?.('change', on);
  }, []);
  return reduced;
}

/** Nền giấy kẻ ô + khung góc + thước ở mép. `fig` là nhãn bản vẽ (trang trí, ẩn với trình đọc màn hình). */
export function Sheet({ fig, children }: { fig: string; children?: ReactNode }): React.JSX.Element {
  const ticks: React.JSX.Element[] = [];
  for (let x = 40; x <= 440; x += 20) ticks.push(<line key={`t${x}`} className="bp-tick" x1={x} x2={x} y1={14} y2={x % 100 === 0 ? 22 : 18} />);
  for (let y = 40; y <= 280; y += 20) ticks.push(<line key={`l${y}`} className="bp-tick" x1={14} x2={y % 100 === 0 ? 22 : 18} y1={y} y2={y} />);
  return (
    <g aria-hidden>
      <defs>
        <pattern id="bp-grid-s" width="20" height="20" patternUnits="userSpaceOnUse"><path d="M20 0H0V20" className="bp-gridline" /></pattern>
        <pattern id="bp-grid-l" width="100" height="100" patternUnits="userSpaceOnUse"><path d="M100 0H0V100" className="bp-gridline strong" /></pattern>
      </defs>
      <rect x="0" y="0" width={VB_W} height={VB_H} className="bp-paper" />
      <rect x="0" y="0" width={VB_W} height={VB_H} fill="url(#bp-grid-s)" className="bp-grid" />
      <rect x="0" y="0" width={VB_W} height={VB_H} fill="url(#bp-grid-l)" className="bp-grid" />
      {ticks}
      <path className="bp-corner" d="M8 26V8H26M454 8H472V26M472 294V312H454M26 312H8V294" />
      <text className="bp-fig" x="34" y="306">{fig}</text>
      <text className="bp-fig" x="446" y="306" textAnchor="end">GRID 20 · 1:1</text>
      {children}
    </g>
  );
}

type GlyphProps = { ch: string; x: number; y: number; size: number; at?: number; fill?: boolean; fillAt?: number; sw?: number; className?: string; label?: string };
/** Glyph vẽ bằng nét (stroke-dashoffset) rồi đổ đặc dần (wireframe → prototype). x là mép trái ô chữ, y là baseline, size là em (px trong viewBox). */
export function GlyphArt({ ch, x, y, size, at = 0, fill = true, fillAt, sw = 1.4, className = '', label }: GlyphProps): React.JSX.Element {
  const g = GLYPHS[ch];
  const s = size / UPM;
  return (
    <g transform={`translate(${x} ${y}) scale(${s})`} data-glyph={ch} data-label={label} className={className}>
      {fill && <path d={g.d} className="bp-solid" style={delay(fillAt ?? at + 900)} />}
      <path d={g.d} pathLength={1} className="bp-outline" strokeWidth={sw / s} style={delay(at)} />
    </g>
  );
}
/** Đường tròn dạng path (xem ghi chú ở Guide). */
export const circlePath = (cx: number, cy: number, r: number): string => `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
export const glyphWidth = (ch: string, size: number): number => (GLYPHS[ch].adv * size) / UPM;

/** Đường dẫn ngang (baseline, x-height, cap…). */
export function Guide({ y, x1 = 36, x2 = 444, name, at = 0, tone = 'dash' }: { y: number; x1?: number; x2?: number; name: string; at?: number; tone?: 'dash' | 'base' }): React.JSX.Element {
  // <path>, không <line>: pathLength chỉ đáng tin trên path ở mọi WebView (WKWebView), và animation vẽ nét cần nó.
  return <path data-guide={name} data-y={y} d={`M${x1} ${y}H${x2}`} pathLength={1} className={`bp-guide ${tone}`} style={delay(at)} />;
}

/** Số trong vòng tròn nối bằng đường dẫn tới điểm cần chú thích; legend HTML bên dưới giải thích số đó. */
export function Callout({ n, px, py, cx, cy, at = 0 }: { n: number; px: number; py: number; cx: number; cy: number; at?: number }): React.JSX.Element {
  return (
    <g className="bp-callout" data-callout={n} style={delay(at)}>
      <line x1={px} y1={py} x2={cx} y2={cy} className="bp-lead" />
      <circle cx={px} cy={py} r="2.4" className="bp-dot" />
      <circle cx={cx} cy={cy} r="9" className="bp-bubble" />
      <text x={cx} y={cy + 3.6} textAnchor="middle" className="bp-num">{n}</text>
    </g>
  );
}

/** Đếm số nhảy lên giá trị đo thật. Giảm chuyển động hoặc không có rAF: hiện luôn giá trị cuối. */
export function useCountUp(to: number, ms = 700, startAt = 0): number {
  const reduced = usePrefersReducedMotion();
  const [v, setV] = useState(reduced || typeof requestAnimationFrame !== 'function' ? to : 0);
  const raf = useRef(0);
  useEffect(() => {
    if (reduced || typeof requestAnimationFrame !== 'function') { setV(to); return; }
    let t0 = 0;
    const tick = (t: number) => {
      if (!t0) t0 = t;
      const k = Math.min(1, Math.max(0, (t - t0 - startAt) / ms));
      setV(to * (1 - (1 - k) ** 3));
      if (k < 1) raf.current = requestAnimationFrame(tick);
    };
    raf.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf.current);
  }, [to, ms, startAt, reduced]);
  return v;
}
export function Readout({ x, y, to, digits = 3, unit = 'em', at = 0, anchor = 'start' }: { x: number; y: number; to: number; digits?: number; unit?: string; at?: number; anchor?: 'start' | 'end' | 'middle' }): React.JSX.Element {
  const v = useCountUp(to, 700, at);
  return <text x={x} y={y} textAnchor={anchor} className="bp-read" data-final={to.toFixed(digits)}>{v.toFixed(digits)} {unit}</text>;
}

/** Dấu đo kích thước dọc: hai vạch chặn, đường nối, mũi tên. */
export function Dimension({ x, y1, y2, at = 0 }: { x: number; y1: number; y2: number; at?: number }): React.JSX.Element {
  const [a, b] = y1 < y2 ? [y1, y2] : [y2, y1];
  return (
    <g className="bp-dim" style={delay(at)} aria-hidden>
      <path d={`M${x - 5} ${a}H${x + 5}M${x - 5} ${b}H${x + 5}`} className="bp-dimline" />
      <path d={`M${x} ${a}V${b}`} pathLength={1} className="bp-dimline draw" style={delay(at)} />
      <path d={`M${x - 3} ${a + 6}L${x} ${a}L${x + 3} ${a + 6}M${x - 3} ${b - 6}L${x} ${b}L${x + 3} ${b - 6}`} className="bp-dimline" />
    </g>
  );
}
