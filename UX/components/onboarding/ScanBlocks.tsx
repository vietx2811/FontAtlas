import { useEffect, useRef, useState } from 'react';
import { pickBlocks, rerollBlocks, type Block } from './glyphs';
import './scanBlocks.css';

export const BEAT_MS = 1400;
const FADE_MS = 500;

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

type Props = { seed: number; scanning: boolean; progress: number | null; done: boolean; count?: number };

/** Bước quét: vài khối vuông mỗi khối một chữ (xoay nhẹ, đổi chữ/font theo nhịp), thanh tiến độ, xong thì vòng tròn xanh có dấu tick. */
export function ScanBlocks({ seed, scanning, progress, done, count = 9 }: Props): React.JSX.Element {
  const reduced = usePrefersReducedMotion();
  const [blocks, setBlocks] = useState<Block[]>(() => pickBlocks(seed, count));
  const [fading, setFading] = useState<ReadonlySet<number>>(new Set());
  const tick = useRef(0);
  const live = scanning && !done;

  useEffect(() => {
    if (!live) return;
    let clear: ReturnType<typeof setTimeout> | undefined;
    const beat = setInterval(() => {
      tick.current += 1;
      setBlocks(prev => {
        const next = rerollBlocks(prev, seed, tick.current);
        setFading(new Set(next.flatMap((b, i) => (b.char !== prev[i].char || b.family !== prev[i].family ? [i] : []))));
        return next;
      });
      clearTimeout(clear);
      clear = setTimeout(() => setFading(new Set()), FADE_MS);
    }, BEAT_MS);
    return () => { clearInterval(beat); clearTimeout(clear); };
  }, [live, seed]);

  const pct = progress === null ? null : Math.round(Math.min(1, Math.max(0, progress)) * 100);
  return (
    <div className="fa-scan-wrap">
      <div className="fa-blocks" data-testid="scan-blocks" data-spinning={live && !reduced} data-motion={reduced ? 'reduced' : 'full'}>
        {blocks.map((b, i) => (
          <span key={i} className="fa-block" data-testid="scan-block" data-fade={fading.has(i)} aria-hidden
            style={{ fontFamily: `"${b.family}", var(--font-ui)`, ['--d' as string]: `${b.delayMs}ms` }}>{b.char}</span>
        ))}
      </div>
      {done ? (
        <svg className="fa-check" role="img" aria-label="Đã quét xong" viewBox="0 0 32 32" width="32" height="32">
          <circle cx="16" cy="16" r="14" fill="var(--on)" />
          <path d="M9.5 16.5l4.2 4.2 8.8-9" fill="none" stroke="var(--sheet)" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      ) : (
        <div className="fa-bar" role="progressbar" aria-label="Tiến độ quét" aria-valuemin={0} aria-valuemax={100} {...(pct === null ? {} : { 'aria-valuenow': pct })} data-indeterminate={pct === null}>
          <i style={pct === null ? undefined : { width: `${pct}%` }} />
        </div>
      )}
    </div>
  );
}
