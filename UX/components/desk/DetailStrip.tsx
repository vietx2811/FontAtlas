import { useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { buildCompare, COMPARE_METRICS, type CompareFace, type CompareMetric } from '../../api/compare';
import type { FaceDetailDto, FaceRowDto } from '../../bindings';
import { t } from '../../i18n';
import { deltaOf, deltaText } from '../compare/compareMath';
import { numberText, unitText } from '../detail/tabs/measurements';
import { NoteMark } from '../ui/noteMark';
import { termName } from '../ui/termNames';

const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const nameOf = (r: FaceRowDto) => `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}`;

/** Đọc số đo của hai font (đang dùng + đang thử), nhớ theo khoá để ↑↓ không đọc lại. Lỗi một font → null, không chặn font kia. */
export function useStripFaces(api: Api, inUse: FaceKeyDto | null, trial: FaceKeyDto | null) {
  const cache = useRef(new Map<string, CompareFace | null>());
  const [, bump] = useState(0);
  const ik = inUse ? keyOf(inUse) : null, tk = trial ? keyOf(trial) : null;
  useEffect(() => {
    let alive = true;
    const need = ([[inUse, ik], [trial, tk]] as const).filter(([f, k]) => f && k && !cache.current.has(k)) as Array<[FaceKeyDto, string]>;
    for (const [f, k] of need) {
      api.getFace(f, 'vi').then((d: FaceDetailDto) => buildCompare([d]).faces[0] ?? null, () => null).then(cf => {
        cache.current.set(k, cf); if (alive) bump(n => n + 1);
      });
      cache.current.set(k, undefined as unknown as null); // đang đọc: không gọi lại; `undefined` = loading
    }
    return () => { alive = false; };
  }, [api, inUse, trial, ik, tk]);
  const get = (k: string | null) => (k ? cache.current.get(k) : null);
  return { a: get(ik), b: get(tk), loading: (ik && get(ik) === undefined) || (tk && get(tk) === undefined) };
}

function Card({ title, row, face, tone }: { title: string; row: FaceRowDto | null; face: CompareFace | null | undefined; tone: 'use' | 'try' }): React.JSX.Element {
  const vi = face?.vi;
  return (
    <div className="fa-dstrip-card" data-tone={tone} data-testid={`strip-card-${tone}`}>
      <h3>{title}</h3>
      {row ? <>
        <div className="fa-dstrip-name">{nameOf(row)}</div>
        <div className="fa-dstrip-meta">
          {face?.designer && <span>{face.designer}</span>}
          {face?.license && <span>{face.license}</span>}
          {vi && <span data-vi={vi.supports && vi.missing === 0 ? 'ok' : 'warn'}>{vi.supports && vi.missing === 0 ? t('strip.viOk') : t('strip.viMissing', { n: vi.missing })}</span>}
          {vi?.stackedMarksOverflow && <NoteMark inline kind="warn" text={t('compare.viWarn')} />}
        </div>
      </> : <p className="fa-dstrip-empty">{tone === 'try' ? t('strip.tryHint') : t('desk.roleEmpty')}</p>}
    </div>
  );
}

function Cell({ m }: { m: CompareMetric | undefined }): React.JSX.Element {
  if (!m || m.value === null) return <td className="na">{t('compare.na')}</td>;
  return <td><b>{numberText(m.value, m.unit)}</b><u>{unitText(m.unit)}</u>{m.quality === 'estimated' && <NoteMark inline kind="info" text={t('compare.est')} />}</td>;
}

/** Dải chi tiết dưới trang (06 §2.1): thẻ Đang dùng / Đang thử và bảng Số đo với Δ (mốc = font đang dùng). */
export function DetailStrip({ api, inUse, trial, collapsed, onToggle, onCompare }: {
  api: Api; inUse: FaceRowDto | null; trial: FaceRowDto | null; collapsed: boolean; onToggle(): void; onCompare?(): void;
}): React.JSX.Element {
  const same = !!inUse && !!trial && keyOf(inUse) === keyOf(trial);
  const { a, b, loading } = useStripFaces(api, inUse, trial && !same ? trial : null);
  const showTrial = !!trial && !same;
  return (
    <section className="fa-dstrip" data-testid="detail-strip" data-collapsed={collapsed || undefined} aria-label={t('strip.title')}>
      <header>
        <b>{t('strip.title')}</b>
        {onCompare && trial && <button type="button" className="fa-dstrip-cmp" data-testid="strip-compare" onClick={onCompare}>{t('strip.compare')}</button>}
        <button type="button" className="fa-dstrip-toggle" data-testid="strip-toggle" aria-expanded={!collapsed} aria-label={collapsed ? t('strip.expand') : t('strip.collapse')} onClick={onToggle}>{collapsed ? '⌃' : '⌄'}</button>
      </header>
      {!collapsed && (
        <div className="fa-dstrip-body">
          <Card title={t('desk.inUse')} row={inUse} face={a} tone="use" />
          <Card title={t('desk.trying')} row={showTrial ? trial : null} face={b} tone="try" />
          <table className="fa-dstrip-table" data-testid="strip-metrics" aria-busy={!!loading}>
            <thead><tr><th>{t('strip.metrics')}</th><th>{t('desk.inUse')}</th><th>{t('desk.trying')}</th><th>Δ</th></tr></thead>
            <tbody>
              {COMPARE_METRICS.map(k => {
                const ma = a?.metrics[k], mb = showTrial ? b?.metrics[k] : undefined;
                const d = ma && mb ? deltaOf(k, mb, ma) : null; const dt = d ? deltaText(d) : null;
                return (
                  <tr key={k} data-metric={k}>
                    <th scope="row" data-term={k}>{termName(k)}</th>
                    <Cell m={ma} />
                    {showTrial ? <Cell m={mb} /> : <td className="na">—</td>}
                    <td className={`dl ${dt?.tone ?? 'zero'}`} data-testid="strip-delta">{dt ? dt.text.replace(/^Δ\s*/, '') : '—'}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
