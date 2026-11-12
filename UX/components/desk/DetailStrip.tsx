import { useEffect, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { buildCompare, COMPARE_METRICS, type CompareFace, type CompareMetric } from '../../api/compare';
import type { FaceDetailDto, FaceRowDto } from '../../bindings';
import { t, getLocale, tn } from '../../i18n';
import { deltaOf, deltaText } from '../compare/compareMath';
import { numberText, unitText } from '../detail/tabs/measurements';
import { NoteMark } from '../ui/noteMark';
import { TermTip, termKey, termName } from '../ui/terms';
import { Tooltip } from '../ui/Tooltip';
import { ViRing } from '../ui/ViRing';
import type { VariableAxisDto } from '../../bindings';

const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const nameOf = (r: FaceRowDto) => `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}`;

/** Đọc số đo của hai font (đang dùng + đang thử), nhớ theo khoá để ↑↓ không đọc lại. Lỗi một font → null cho lần này, KHÔNG nhớ (lần sau đọc lại). */
export function useStripFaces(api: Api, inUse: FaceKeyDto | null, trial: FaceKeyDto | null) {
  const cache = useRef(new Map<string, CompareFace | null>());
  const pending = useRef(new Set<string>());
  const failed = useRef(new Set<string>());
  const mounted = useRef(true);
  const [, bump] = useState(0);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const ik = inUse ? keyOf(inUse) : null, tk = trial ? keyOf(trial) : null;
  useEffect(() => {
    for (const [f, k] of [[inUse, ik], [trial, tk]] as const) {
      if (!f || !k || cache.current.has(k) || pending.current.has(k)) continue;
      pending.current.add(k); failed.current.delete(k);
      api.getFace(f, getLocale()).then((d: FaceDetailDto) => buildCompare([d]).faces[0] ?? null, () => undefined).then(cf => {
        pending.current.delete(k);
        if (cf !== undefined) cache.current.set(k, cf); else failed.current.add(k);
        if (mounted.current) bump(n => n + 1);
      });
    }
  }, [api, inUse, trial, ik, tk]);
  // Đang đọc = có khoá, chưa có trong bộ nhớ, chưa lỗi (kể cả lượt render đầu, trước khi effect đánh dấu pending).
  const busy = (k: string | null) => !!k && !cache.current.has(k) && !failed.current.has(k);
  const get = (k: string | null): CompareFace | null => (k ? (cache.current.get(k) ?? null) : null);
  return { a: get(ik), b: get(tk), loading: busy(ik) || busy(tk), aLoading: busy(ik), bLoading: busy(tk) };
}

const nf = (n: number) => new Intl.NumberFormat(getLocale() === 'vi' ? 'vi-VN' : 'en-US').format(n);
const axisText = (a: VariableAxisDto) => `${a.tag} ${a.min}–${a.max}`;
const MAX_AXES_INLINE = 2;

/** Tag Variable với các trục min–max (A4). Font static → không có. */
function VariableTag({ axes }: { axes: VariableAxisDto[] }): React.JSX.Element {
  const shown = axes.slice(0, MAX_AXES_INLINE); const more = axes.length - shown.length;
  return (
    <TermTip term="variable_font" extra={axes.length ? t('strip.varTip', { axes: axes.map(axisText).join(' · ') }) : undefined}>
      <span className="fa-dstrip-var" data-testid="strip-variable">{termName('variable_font')}{shown.length > 0 && <> <small>{shown.map(axisText).join(' · ')}{more > 0 && ` ${t('strip.varMore', { n: more })}`}</small></>}</span>
    </TermTip>
  );
}

function Card({ title, row, face, tone, baseGlyphs }: { title: string; row: FaceRowDto | null; face: CompareFace | null | undefined; tone: 'use' | 'try'; baseGlyphs?: number | null }): React.JSX.Element {
  const vi = face?.vi;
  const glyphs = face?.glyphs ?? null;
  const dg = tone === 'try' && glyphs !== null && baseGlyphs != null ? glyphs - baseGlyphs : null;
  const variable = row?.isVariable || (face?.variableAxes?.length ?? 0) > 0;
  return (
    <div className="fa-dstrip-card" data-tone={tone} data-testid={`strip-card-${tone}`}>
      <h3>{title}</h3>
      {row ? <>
        <div className="fa-dstrip-name">{nameOf(row)}{vi && <ViRing missing={vi.missing} total={vi.total} supports={vi.supports} />}</div>
        <div className="fa-dstrip-meta">
          {face?.designer && <Tooltip label="Designer"><span>{face.designer}</span></Tooltip>}
          {face?.foundry && face.foundry !== face.designer && <Tooltip label="Foundry"><span>{face.foundry}</span></Tooltip>}
          {face?.version && <Tooltip label="Version"><span>{face.version}</span></Tooltip>}
          {face?.license && <Tooltip label="License"><span>{face.license}</span></Tooltip>}
          {glyphs !== null && (
            <TermTip term="glyph"><span data-testid="strip-glyphs">{nf(glyphs)} {t('strip.glyphsRow')}{dg !== null && dg !== 0 && <b className={`fa-dstrip-dg ${dg > 0 ? 'up' : 'down'}`} data-testid="strip-glyphs-dg"> Δ {dg > 0 ? '+' : '−'}{nf(Math.abs(dg))}</b>}</span></TermTip>
          )}
          {variable && <VariableTag axes={face?.variableAxes ?? []} />}
          {vi?.stackedMarksOverflow && <NoteMark inline kind="warn" text={t('compare.viWarn')} />}
        </div>
      </> : <p className="fa-dstrip-empty">{tone === 'try' ? t('strip.tryHint') : t('desk.roleEmpty')}</p>}
    </div>
  );
}

function Cell({ m, loading }: { m: CompareMetric | undefined; loading?: boolean }): React.JSX.Element {
  if (!m && loading) return <td className="sk" aria-hidden="true"><i /></td>;
  if (!m || m.value === null) return <td className="na">{t('compare.na')}</td>;
  return <td><b>{numberText(m.value, m.unit)}</b><u>{unitText(m.unit)}</u>{m.quality === 'estimated' && <NoteMark inline kind="info" text={t('compare.est')} />}</td>;
}

/** Dải chi tiết dưới trang (06 §2.1): thẻ Đang dùng / Đang thử và bảng Số đo với Δ (mốc = font đang dùng). */
export function DetailStrip({ api, inUse, trial, collapsed, onToggle, onCompare, compareReason }: {
  api: Api; inUse: FaceRowDto | null; trial: FaceRowDto | null; collapsed: boolean; onToggle(): void; onCompare?(): void; compareReason?: string | null;
}): React.JSX.Element {
  const same = !!inUse && !!trial && keyOf(inUse) === keyOf(trial);
  const { a, b, loading, aLoading, bLoading } = useStripFaces(api, inUse, trial && !same ? trial : null);
  const showTrial = !!trial && !same;
  return (
    <section className="fa-dstrip" data-testid="detail-strip" data-collapsed={collapsed || undefined} aria-label={t('strip.title')}>
      <header>
        <b>{t('strip.title')}</b>
        {onCompare && trial && (compareReason
          ? <Tooltip label={compareReason}><button type="button" className="fa-dstrip-cmp" data-testid="strip-compare" aria-disabled="true" onClick={e => e.preventDefault()}>{t('strip.compare')}</button></Tooltip>
          : <button type="button" className="fa-dstrip-cmp" data-testid="strip-compare" onClick={onCompare}>{t('strip.compare')}</button>)}
        <button type="button" className="fa-dstrip-toggle" data-testid="strip-toggle" aria-expanded={!collapsed} aria-label={collapsed ? t('strip.expand') : t('strip.collapse')} onClick={onToggle}>{collapsed ? '⌃' : '⌄'}</button>
      </header>
      {!collapsed && (
        <div className="fa-dstrip-body">
          <Card title={t('desk.inUse')} row={inUse} face={a} tone="use" />
          <Card title={t('desk.trying')} row={showTrial ? trial : null} face={b} tone="try" baseGlyphs={a?.glyphs ?? null} />
          <table className="fa-dstrip-table" data-testid="strip-metrics" aria-busy={!!loading}>
            <thead><tr><th>{t('strip.metrics')}</th><th>{t('desk.inUse')}</th><th>{t('desk.trying')}</th><th><Tooltip label={t('strip.deltaTip')}><span data-testid="strip-delta-head">Δ</span></Tooltip></th></tr></thead>
            <tbody>
              {COMPARE_METRICS.map(k => {
                const ma = a?.metrics[k], mb = showTrial ? b?.metrics[k] : undefined;
                const d = ma && mb ? deltaOf(k, mb, ma) : null; const dt = d ? deltaText(d) : null;
                return (
                  <tr key={k} data-metric={k}>
                    <th scope="row"><TermTip term={k}><span data-term={termKey(k)}>{termName(k)}</span></TermTip></th>
                    <Cell m={ma} loading={aLoading} />
                    {showTrial ? <Cell m={mb} loading={bLoading} /> : <td className="na">—</td>}
                    <td className={`dl ${dt?.tone ?? 'zero'}`} data-testid="strip-delta">{dt ? dt.text.replace(/^Δ\s*/, '') : '—'}</td>
                  </tr>
                );
              })}
              <tr data-metric="glyphs" data-testid="strip-glyphs-row">
                <th scope="row"><TermTip term="glyph"><span>{t('strip.glyphsRow')}</span></TermTip></th>
                <td className={a?.glyphs == null && !aLoading ? 'na' : undefined}>{a?.glyphs != null ? <b>{nf(a.glyphs)}</b> : aLoading ? <i className="sk-i" aria-hidden /> : t('compare.na')}</td>
                {showTrial ? <td className={b?.glyphs == null && !bLoading ? 'na' : undefined}>{b?.glyphs != null ? <b>{nf(b.glyphs)}</b> : bLoading ? <i className="sk-i" aria-hidden /> : t('compare.na')}</td> : <td className="na">—</td>}
                {(() => { const d = a?.glyphs != null && b?.glyphs != null && showTrial && a.glyphs > 0 ? deltaOf('glyphs', { key: 'glyphs', value: b.glyphs, unit: null, quality: 'measured', reason: null, percentile: null }, { key: 'glyphs', value: a.glyphs, unit: null, quality: 'measured', reason: null, percentile: null }) : null; const dt = d ? deltaText(d) : null;
                  return <td className={`dl ${dt?.tone ?? 'zero'}`} data-testid="strip-glyphs-delta">{dt ? dt.text.replace(/^Δ\s*/, '') : '—'}</td>; })()}
              </tr>
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
