// Màn kết của onboarding (~1,5 s): glyph thu về góc, lưới đo tan. (Mở đầu là Splash dùng chung với lúc mở app.)
import { useEffect, useRef } from 'react';
import { t } from '../../i18n';
import { CAP_HEIGHT, X_HEIGHT } from './glyphOutlines';
import { GlyphArt, Guide, Sheet, glyphWidth } from './blueprint';

export const OUTRO_MS = 1500;
const SIZE = 200; const BASE = 214;
const emOf = (u: number) => (u * SIZE) / 2048;

function Mark(): React.JSX.Element {
  const x = 240 - glyphWidth('A', SIZE) / 2;
  return (
    <svg className="fa-bp" viewBox="0 0 480 320" aria-hidden preserveAspectRatio="xMidYMid meet">
      <Sheet fig="FIG.09 · CLOSE" />
      <Guide y={BASE - emOf(CAP_HEIGHT)} name="cap" />
      <Guide y={BASE - emOf(X_HEIGHT)} name="x-height" />
      <Guide y={BASE} name="baseline" tone="base" />
      <g className="fa-mark-glyph"><GlyphArt ch="A" x={x} y={BASE} size={SIZE} at={0} fillAt={0} /></g>
    </svg>
  );
}

/** Màn kết: glyph thu về góc, lưới tan; gọi onDone sau OUTRO_MS (hoặc ngay khi bấm phím). */
export function Outro({ onDone, ms = OUTRO_MS }: { onDone(): void; ms?: number }): React.JSX.Element {
  const done = useRef(onDone); done.current = onDone;
  useEffect(() => {
    const timer = setTimeout(() => done.current(), ms);
    const key = (e: KeyboardEvent) => { if (!e.repeat && (e.key === 'Escape' || e.key === 'Enter')) done.current(); };
    document.addEventListener('keydown', key);
    return () => { clearTimeout(timer); document.removeEventListener('keydown', key); };
  }, [ms]);
  return (
    <div className="fa-intro fa-outro" role="status" aria-label={t('onb.outro.label')} data-testid="onb-outro" style={{ ['--outro-ms' as string]: `${ms}ms` }}>
      <Mark />
    </div>
  );
}
