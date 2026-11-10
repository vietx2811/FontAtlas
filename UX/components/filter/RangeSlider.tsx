import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { t } from '../../i18n';
import './rangeSlider.css';

export type Range = [number, number];
type Props = {
  min: number; max: number;
  /** Bước nhỏ nhất; với trục rời rạc (`ticks`) giá trị luôn bị kéo về vạch gần nhất. */
  step: number;
  ticks?: number[];
  /** null = chưa lọc: hai đầu nằm ở hai mép, thanh nhạt. */
  value: Range | null;
  label: string;
  format?(v: number): string;
  onChange(next: Range): void;
  /** Nháy đúp lên thanh = bỏ lọc. */
  onReset(): void;
  /** Lớp vẽ sau thanh (histogram). */
  children?: ReactNode;
};

export const KEY_COMMIT_MS = 300;
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Slider hai đầu: kéo chuột/cảm ứng, bàn phím (←→ một bước, PgUp/PgDn 10 bước, Home/End), snap theo vạch, nháy đúp = bỏ lọc. Chỉ gọi onChange khi thả/nhấn phím. */
export function RangeSlider({ min, max, step, ticks, value, label, format = String, onChange, onReset, children }: Props): React.JSX.Element {
  const track = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<Range | null>(null);
  const raw: Range = drag ?? value ?? [min, max];
  const cur: Range = [clamp(raw[0], min, max), clamp(raw[1], min, max)]; // giá trị lưu ngoài miền hiển thị (cận mở) vẽ ở mép
  const span = Math.max(max - min, 1e-9);
  const pct = (v: number) => `${((clamp(v, min, max) - min) / span) * 100}%`;
  const nearestTick = (c: number) => ticks!.reduce((best, t) => Math.abs(t - c) < Math.abs(best - c) ? t : best, ticks![0]);
  const snap = (v: number): number => {
    const c = clamp(v, min, max);
    if (ticks?.length) return nearestTick(c);
    // Mép thật: khi (max − min) không chia hết cho step, kéo hết cỡ phải ra đúng max/min, không phải max − step/2.
    if (c >= max - step / 2) return max;
    if (c <= min + step / 2) return min;
    return clamp(Math.round((c - min) / step) * step + min, min, max);
  };
  const fromX = (x: number): number => {
    const r = track.current!.getBoundingClientRect();
    return snap(min + ((x - r.left) / Math.max(r.width, 1)) * span);
  };
  const next = (i: 0 | 1, v: number, from: Range): Range => i === 0 ? [Math.min(v, from[1]), from[1]] : [from[0], Math.max(v, from[0])];
  const move = (i: 0 | 1, dir: number, big: boolean): number => {
    if (ticks?.length) {
      const at = ticks.indexOf(nearestTick(cur[i])); // giá trị lẻ (từ ô nhập) thì tính từ vạch gần nhất
      return ticks[clamp(at + dir * (big ? 3 : 1), 0, ticks.length - 1)];
    }
    return snap(cur[i] + dir * step * (big ? 10 : 1));
  };

  const onThumbDown = (i: 0 | 1) => (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    e.currentTarget.focus();
    setDrag(cur);
    const base = cur;
    let latest: Range = base;
    const moveTo = (ev: PointerEvent) => { latest = next(i, fromX(ev.clientX), base); setDrag(latest); };
    const stop = () => { window.removeEventListener('pointermove', moveTo); window.removeEventListener('pointerup', up); window.removeEventListener('pointercancel', stop); setDrag(null); };
    const up = () => { const done = latest; stop(); if (done[0] !== base[0] || done[1] !== base[1]) onChange(done); };
    window.addEventListener('pointermove', moveTo); window.addEventListener('pointerup', up); window.addEventListener('pointercancel', stop);
  };
  // Phím: giữ giá trị tạm, chỉ ghi vào bộ lọc sau KEY_COMMIT_MS không có phím mới (hoặc khi rời thanh): một bước hoàn tác, một truy vấn.
  const pending = useRef<Range | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const onChangeRef = useRef(onChange); onChangeRef.current = onChange;
  const flush = () => { clearTimeout(timer.current); const p = pending.current; pending.current = null; if (p) { setDrag(null); onChangeRef.current(p); } };
  useEffect(() => () => { const p = pending.current; pending.current = null; clearTimeout(timer.current); if (p) onChangeRef.current(p); }, []);
  const queue = (r: Range) => { pending.current = r; setDrag(r); clearTimeout(timer.current); timer.current = setTimeout(flush, KEY_COMMIT_MS); };
  const onKey = (i: 0 | 1) => (e: React.KeyboardEvent) => {
    const big = e.key === 'PageUp' || e.key === 'PageDown';
    let v: number | null = null;
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp' || e.key === 'PageUp') v = move(i, 1, big);
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown' || e.key === 'PageDown') v = move(i, -1, big);
    else if (e.key === 'Home') v = min;
    else if (e.key === 'End') v = max;
    if (v === null) return;
    e.preventDefault();
    queue(next(i, snap(v), cur));
  };
  const thumb = (i: 0 | 1) => (
    <div key={i} role="slider" tabIndex={0} className="fa-rs-thumb" data-testid={i === 0 ? 'rs-lo' : 'rs-hi'} style={{ left: pct(cur[i]) }}
      aria-label={`${label} ${t(i === 0 ? 'slider.min' : 'slider.max')}`} aria-valuemin={i === 0 ? min : cur[0]} aria-valuemax={i === 0 ? cur[1] : max} aria-valuenow={cur[i]} aria-valuetext={format(cur[i])}
      onPointerDown={onThumbDown(i)} onKeyDown={onKey(i)} onBlur={flush} />
  );
  return (
    <div className="fa-rs" data-active={value !== null} data-dragging={drag !== null}>
      <div className="fa-rs-bg" aria-hidden>{children}</div>
      <div ref={track} className="fa-rs-track" data-testid="rs-track" onDoubleClick={() => { if (value) onReset(); }}>
        {ticks?.map(t => <i key={t} className="fa-rs-tick" style={{ left: pct(t) }} aria-hidden />)}
        <div className="fa-rs-fill" style={{ left: pct(cur[0]), width: `calc(${pct(cur[1])} - ${pct(cur[0])})` }} />
        {thumb(0)}{thumb(1)}
      </div>
    </div>
  );
}
