import { useEffect, useRef, useState } from 'react';
import { t } from '../../i18n';
import { GLYPHS, UPM } from './glyphOutlines';
import { Sheet, circlePath, delay, usePrefersReducedMotion } from './blueprint';
import './scanBlocks.css';

export { usePrefersReducedMotion };
export const BEAT_MS = 1400;
const FADE_MS = 500;
/** Chín ô chữ (Latin thường, chữ Việt có dấu) cho khay mẫu; vẽ bằng đường viền Inter nên giống nhau trên mọi máy. */
export const SPECIMENS = ['A', 'a', 'g', 'h', 'H', 'x', 'ê', 'ẫ', 'ự', 'ă', 'Ặ', 'l'] as const;
type Block = { char: string; rot: number };

const rng = (s: number) => { let x = (s ^ 0x9e3779b9) >>> 0; x = Math.imul(x ^ (x >>> 15), 0x85ebca6b); x = Math.imul(x ^ (x >>> 13), 0xc2b2ae35); return ((x ^ (x >>> 16)) >>> 0) / 2 ** 32; };
export const pickBlocks = (seed: number, count: number): Block[] => {
  const pool = [...SPECIMENS]; const out: Block[] = [];
  for (let i = 0; i < count; i++) { const k = Math.floor(rng(seed + i * 31) * pool.length); out.push({ char: pool.splice(k, 1)[0], rot: 0 }); if (!pool.length) pool.push(...SPECIMENS); }
  return out;
};
/** Mỗi nhịp đổi 3 ô sang chữ chưa có trên khay. */
export const rerollBlocks = (prev: Block[], seed: number, tick: number): Block[] => {
  const next = prev.map(b => ({ ...b }));
  for (let j = 0; j < 3; j++) {
    const i = Math.floor(rng(seed + tick * 7 + j * 13) * next.length);
    const free = SPECIMENS.filter(c => !next.some(b => b.char === c));
    next[i].char = free[Math.floor(rng(seed + tick * 11 + j) * free.length)] ?? next[i].char;
  }
  return next;
};

const CELL_W = 112; const CELL_H = 62; const GAP = 12; const X0 = 68; const Y0 = 56;
type Props = { seed: number; scanning: boolean; progress: number | null; done: boolean; count?: number };

/** Bước quét: khay mẫu 3×3 chữ Inter vẽ nét, tia quét chạy ngang, số đo nhảy theo từng ô; xong thì vòng tick. */
export function ScanBlocks({ seed, scanning, progress, done, count = 9 }: Props): React.JSX.Element {
  const reduced = usePrefersReducedMotion();
  const [blocks, setBlocks] = useState<Block[]>(() => pickBlocks(seed, count));
  const [fading, setFading] = useState<ReadonlySet<number>>(new Set());
  const [focus, setFocus] = useState(0);
  const tick = useRef(0);
  const live = scanning && !done;
  useEffect(() => {
    if (!live) return;
    let clear: ReturnType<typeof setTimeout> | undefined;
    const beat = setInterval(() => {
      tick.current += 1;
      setBlocks(prev => {
        const next = rerollBlocks(prev, seed, tick.current);
        setFading(new Set(next.flatMap((b, i) => (b.char !== prev[i].char ? [i] : []))));
        return next;
      });
      setFocus(Math.floor(rng(seed + tick.current * 3) * count));
      clearTimeout(clear);
      clear = setTimeout(() => setFading(new Set()), FADE_MS);
    }, BEAT_MS);
    return () => { clearInterval(beat); clearTimeout(clear); };
  }, [live, seed, count]);

  const pct = progress === null ? null : Math.round(Math.min(1, Math.max(0, progress)) * 100);
  const cur = GLYPHS[blocks[Math.min(focus, blocks.length - 1)].char];
  const size = 50; const s = size / UPM;
  return (
    <div className="fa-scan-wrap">
      <svg className="fa-bp" viewBox="0 0 480 480" role="img" aria-label={t('onb.art.o4')} data-testid="scan-blocks" data-spinning={live && !reduced} data-motion={reduced ? 'reduced' : 'full'}>
        <Sheet fig="FIG.04 · SCAN" h={480}>
        {blocks.map((b, i) => {
          const g = GLYPHS[b.char];
          const x = X0 + (i % 3) * (CELL_W + GAP); const y = Y0 + Math.floor(i / 3) * (CELL_H + GAP);
          return (
            <g key={i} data-testid="scan-block" data-char={b.char} data-fade={fading.has(i)} data-focus={live && i === focus} aria-hidden>
              <rect x={x} y={y} width={CELL_W} height={CELL_H} rx="4" className="bp-well" />
              <path d={`M${x + 6} ${y + 6}h8M${x + 6} ${y + 6}v8M${x + CELL_W - 6} ${y + CELL_H - 6}h-8M${x + CELL_W - 6} ${y + CELL_H - 6}v-8`} className="bp-corner small" />
              <g transform={`translate(${x + CELL_W / 2 - (g.adv * s) / 2} ${y + CELL_H - 12}) scale(${s})`} className="bp-cell-glyph">
                <path d={g.d} className="bp-solid on" />
                <path d={g.d} className="bp-outline on" strokeWidth={1.2 / s} />
              </g>
            </g>
          );
        })}
        <rect x={X0 - 4} y={Y0 - 8} width="26" height={3 * CELL_H + 2 * GAP + 16} className="bp-beam scan" style={{ ...delay(0), ['--travel' as string]: `${3 * (CELL_W + GAP) - 18}px` }} />
        <text x={X0} y="288" className="bp-read" aria-hidden>{blocks[Math.min(focus, blocks.length - 1)].char}  adv {(cur.adv / UPM).toFixed(3)} em · h {((cur.top - cur.bottom) / UPM).toFixed(3)} em</text>
        </Sheet>
      </svg>
      {done ? (
        <svg className="fa-check" role="img" aria-label={t('onb.scan.done')} viewBox="0 0 32 32" width="32" height="32">
          <path d={circlePath(16, 16, 14)} className="ok-disc" pathLength={1} />
          <path d="M9.5 16.5l4.2 4.2 8.8-9" className="ok-tick" pathLength={1} />
        </svg>
      ) : (
        <div className="fa-bar" role="progressbar" aria-label={t('onb.scan.progress')} aria-valuemin={0} aria-valuemax={100} {...(pct === null ? {} : { 'aria-valuenow': pct })} data-indeterminate={pct === null}>
          <i style={pct === null ? undefined : { width: `${pct}%` }} />
        </div>
      )}
    </div>
  );
}
