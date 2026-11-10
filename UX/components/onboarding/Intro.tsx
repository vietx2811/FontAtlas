// Mở đầu (~2,8 s) và kết (~1,5 s) của onboarding. Cùng ngôn ngữ blueprint: lưới đo dựng chữ A, rồi lưới tan khi vào thư viện.
import { useEffect, useRef } from 'react';
import { t } from '../../i18n';
import { CAP_HEIGHT, X_HEIGHT } from './glyphOutlines';
import { GlyphArt, Guide, Sheet, delay, glyphWidth } from './blueprint';

export const INTRO_MS = 2800;
export const OUTRO_MS = 1500;

const SIZE = 200; const BASE = 214;
const emOf = (u: number) => (u * SIZE) / 2048;

function Mark({ outro }: { outro?: boolean }): React.JSX.Element {
  const x = 240 - glyphWidth('A', SIZE) / 2;
  return (
    <svg className="fa-bp" viewBox="0 0 480 320" aria-hidden preserveAspectRatio="xMidYMid meet">
      <Sheet fig={outro ? 'FIG.09 · CLOSE' : 'FIG.00 · FONT ATLAS'} />
      <Guide y={BASE - emOf(CAP_HEIGHT)} name="cap" at={outro ? 0 : 250} />
      <Guide y={BASE - emOf(X_HEIGHT)} name="x-height" at={outro ? 0 : 400} />
      <Guide y={BASE} name="baseline" at={outro ? 0 : 550} tone="base" />
      <g className="fa-mark-glyph"><GlyphArt ch="A" x={x} y={BASE} size={SIZE} at={outro ? 0 : 700} fillAt={outro ? 0 : 1500} /></g>
      {!outro && <g aria-hidden><text x="240" y="268" textAnchor="middle" className="bp-word" style={delay(1700)}>Font Atlas</text></g>}
    </svg>
  );
}

/** Màn mở đầu: tự qua sau INTRO_MS; Esc / Enter / Space / bấm nút "Bỏ qua" qua ngay. */
export function Intro({ onDone, ms = INTRO_MS }: { onDone(): void; ms?: number }): React.JSX.Element {
  const done = useRef(onDone); done.current = onDone;
  const skip = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const timer = setTimeout(() => done.current(), ms);
    const key = (e: KeyboardEvent) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') { e.preventDefault(); done.current(); } };
    document.addEventListener('keydown', key);
    skip.current?.focus();
    return () => { clearTimeout(timer); document.removeEventListener('keydown', key); };
  }, [ms]);
  return (
    <div className="fa-intro" role="status" aria-label={t('onb.intro.label')} data-testid="onb-intro">
      <Mark />
      <p className="fa-intro-tag" style={delay(2000)}>{t('onb.intro.tag')} <span>— {t('onb.intro.tag.gloss')}</span></p>
      <button ref={skip} type="button" className="fa-intro-skip" onClick={() => done.current()}>{t('onb.intro.skip')}</button>
    </div>
  );
}

/** Màn kết: glyph thu về góc, lưới tan; gọi onDone sau OUTRO_MS (hoặc ngay khi bấm phím). */
export function Outro({ onDone, ms = OUTRO_MS }: { onDone(): void; ms?: number }): React.JSX.Element {
  const done = useRef(onDone); done.current = onDone;
  useEffect(() => {
    const timer = setTimeout(() => done.current(), ms);
    const key = (e: KeyboardEvent) => { if (e.key === 'Escape' || e.key === 'Enter') done.current(); };
    document.addEventListener('keydown', key);
    return () => { clearTimeout(timer); document.removeEventListener('keydown', key); };
  }, [ms]);
  return (
    <div className="fa-intro fa-outro" role="status" aria-label={t('onb.outro.label')} data-testid="onb-outro" style={{ ['--outro-ms' as string]: `${ms}ms` }}>
      <Mark outro />
    </div>
  );
}
