import { useEffect, useMemo, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { buildCompare, COMPARE_METRICS, type CompareFace, type CompareMetric } from '../../api/compare';
import type { ComboRoleDto, FaceRowDto } from '../../bindings';
import { t } from '../../i18n';
import { deltaOf, deltaText } from '../compare/compareMath';
import { numberText, unitText } from '../detail/tabs/measurements';
import { useFontFamily } from '../detail/useFontFamily';
import { NoteMark } from '../ui/noteMark';
import { TermTip, termKey, termName } from '../ui/terms';
import { isHeadingLike } from './deskRoles';

export type FcFace = { face: FaceKeyDto; row: FaceRowDto };
type SortKey = 'order' | 'name' | 'vi' | (typeof COMPARE_METRICS)[number];
const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const nameOf = (r: FaceRowDto) => `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}`;
const roleName = (r: ComboRoleDto) => t(`role.${r}` as 'role.display');

/** Đọc số đo cho nhiều font; nhớ theo khoá, lỗi thì không nhớ (lần sau đọc lại). */
function useFacesData(api: Api, faces: FaceKeyDto[]): Map<string, CompareFace | null> {
  const cache = useRef(new Map<string, CompareFace | null>());
  const pending = useRef(new Set<string>());
  const alive = useRef(true);
  const [, bump] = useState(0);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const sig = faces.map(keyOf).join('|');
  useEffect(() => {
    for (const f of faces) {
      const k = keyOf(f);
      if (cache.current.has(k) || pending.current.has(k)) continue;
      pending.current.add(k);
      api.getFace(f, 'vi').then(d => buildCompare([d]).faces[0] ?? null, () => undefined).then(cf => {
        pending.current.delete(k);
        if (cf !== undefined) cache.current.set(k, cf);
        if (alive.current) bump(n => n + 1);
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api, sig]);
  return cache.current;
}

function Sample({ api, row, partner, role, pageMode }: { api: Api; row: FaceRowDto; partner: FaceRowDto | null; role: ComboRoleDto; pageMode: boolean }): React.JSX.Element {
  const fam = useFontFamily(api, row); const pfam = useFontFamily(api, partner);
  const css = (f: string | null, r: FaceRowDto | null) => ({ fontFamily: f ? `"${f}", var(--font-ui)` : undefined, fontWeight: r?.weightClass ?? undefined, fontStyle: r?.isItalic ? 'italic' as const : undefined });
  const heading = isHeadingLike(role);
  if (!pageMode) {
    return <div className="fa-cc-page" data-testid="fc-sample" style={{ ...css(fam, row), fontSize: heading ? 22 : 13, lineHeight: heading ? 1.15 : 1.5 }}>{heading ? t('desk.pageHeading') : t('desk.pageBody').slice(0, 110)}</div>;
  }
  return (
    <div className="fa-cc-page" data-testid="fc-page">
      <div style={{ ...css(heading ? fam : pfam, heading ? row : partner), fontSize: 20, lineHeight: 1.15 }}>{t('desk.pageHeading')}</div>
      <p style={{ ...css(heading ? pfam : fam, heading ? partner : row), fontSize: 11, lineHeight: 1.5, margin: '6px 0 0' }}>{t('desk.pageBody').slice(0, 120)}</p>
    </div>
  );
}

function MetricCell({ m, base }: { m: CompareMetric | undefined; base: CompareMetric | undefined }): React.JSX.Element {
  if (!m || m.value === null) return <td className="na">{t('compare.na')}</td>;
  const d = base && m !== base ? deltaOf(m.key, m, base) : null; const dt = d ? deltaText(d) : null;
  return <td><b>{numberText(m.value, m.unit)}</b><u>{unitText(m.unit)}</u>{m.quality === 'estimated' && <NoteMark inline kind="info" text={t('compare.est')} />}{dt && <span className={`fa-fc-d ${dt.tone}`} data-testid="fc-delta"> {dt.text}</span>}</td>;
}

/** Compare Font trong Desk (06 §3.1): mốc = font đang dùng cho vai; cột ghim đứng sau mốc; sắp theo số đo; Bảng | Song bản. */
export function FontCompare({ api, role, base, partner, suggestions, onPlace, onTry }: {
  api: Api; role: ComboRoleDto; base: FcFace | null; partner: FaceRowDto | null; suggestions: FcFace[]; onPlace(f: FcFace): void; onTry(f: FcFace): void;
}): React.JSX.Element {
  const [cols, setCols] = useState<FcFace[]>(() => suggestions.slice(0, 4));
  const [pinned, setPinned] = useState<string[]>([]);
  const [sort, setSort] = useState<SortKey>('order');
  const [desc, setDesc] = useState(false);
  const [view, setView] = useState<'table' | 'pages'>('table');
  const data = useFacesData(api, [...(base ? [base.face] : []), ...cols.map(c => c.face)]);
  const baseData = base ? data.get(keyOf(base.face)) ?? null : null;

  const value = (c: FcFace): number | string | null => {
    const d = data.get(keyOf(c.face));
    if (sort === 'name') return nameOf(c.row).toLowerCase();
    if (sort === 'vi') return d ? d.vi.missing : null;
    if (sort === 'order') return null;
    return d?.metrics[sort]?.value ?? null;
  };
  const ordered = useMemo(() => {
    const pins = pinned.map(k => cols.find(c => keyOf(c.face) === k)).filter((c): c is FcFace => !!c);
    const rest = cols.filter(c => !pinned.includes(keyOf(c.face)));
    if (sort !== 'order') {
      rest.sort((a, b) => {
        const x = value(a), y = value(b);
        if (x === null && y === null) return 0; if (x === null) return 1; if (y === null) return -1; // thiếu số luôn xuống cuối
        const r = typeof x === 'string' ? x.localeCompare(y as string) : x - (y as number);
        return desc ? -r : r;
      });
    }
    return [...pins, ...rest];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cols, pinned, sort, desc, data.size]);

  const shown = [...(base ? [base] : []), ...ordered];
  const addable = suggestions.filter(s => !cols.some(c => keyOf(c.face) === keyOf(s.face)) && !(base && keyOf(base.face) === keyOf(s.face)));
  const remove = (f: FcFace) => { setCols(cs => cs.filter(c => keyOf(c.face) !== keyOf(f.face))); setPinned(p => p.filter(k => k !== keyOf(f.face))); };
  const togglePin = (f: FcFace) => setPinned(p => (p.includes(keyOf(f.face)) ? p.filter(k => k !== keyOf(f.face)) : [...p, keyOf(f.face)]));
  const dataOf = (f: FcFace) => data.get(keyOf(f.face)) ?? null;

  return (
    <div className="fa-cc fa-fc" data-testid="font-compare">
      <div className="fa-fc-bar">
        <label>{t('fc.sort')}{' '}
          <select data-testid="fc-sort" value={sort} onChange={e => setSort(e.target.value as SortKey)}>
            <option value="order">{t('fc.sort.order')}</option>
            <option value="name">{t('fc.sort.name')}</option>
            <option value="vi">{t('fc.sort.vi')}</option>
            {COMPARE_METRICS.map(k => <option key={k} value={k}>{termName(k)}</option>)}
          </select>
        </label>
        <button type="button" data-testid="fc-dir" aria-pressed={desc} aria-label={t('fc.dir')} disabled={sort === 'order'} onClick={() => setDesc(d => !d)}>{desc ? '↓' : '↑'}</button>
        <span className="fa-desk-mode" role="group" aria-label={t('fc.view')}>
          <button type="button" data-testid="fc-view-table" aria-pressed={view === 'table'} onClick={() => setView('table')}>{t('fc.view.table')}</button>
          <button type="button" data-testid="fc-view-pages" aria-pressed={view === 'pages'} onClick={() => setView('pages')}>{t('fc.view.pages')}</button>
        </span>
        <label>{t('fc.add')}{' '}
          <select data-testid="fc-add" value="" disabled={addable.length === 0} onChange={e => { const f = addable.find(a => keyOf(a.face) === e.target.value); if (f) setCols(cs => [...cs, f]); }}>
            <option value="">{addable.length ? t('fc.add.pick') : t('fc.add.none')}</option>
            {addable.map(a => <option key={keyOf(a.face)} value={keyOf(a.face)}>{nameOf(a.row)}</option>)}
          </select>
        </label>
        <span className="fa-fc-role">{t('fc.role')}: <b>{roleName(role)}</b></span>
      </div>
      {!base && <p className="fa-desk-hint" data-testid="fc-nobase">{t('fc.noBase', { role: roleName(role) })}</p>}
      {shown.length === 0 && <p className="fa-desk-hint">{t('desk.noCands')}</p>}
      {shown.length > 0 && (
      <table>
        <thead><tr><th />{shown.map(c => {
          const isBase = !!base && keyOf(c.face) === keyOf(base.face); const pin = pinned.includes(keyOf(c.face));
          return (
            <th key={keyOf(c.face)} data-testid="fc-col" data-base={isBase || undefined} data-pinned={pin || undefined}>
              <div className="fa-cc-title">{pin ? '📌 ' : ''}{nameOf(c.row)}{isBase && <small> · {t('compare.base')}</small>}</div>
              {!isBase && (
                <div className="fa-cc-actions">
                  <button type="button" data-testid="fc-pin" aria-pressed={pin} onClick={() => togglePin(c)}>{pin ? t('fc.unpin') : t('fc.pin')}</button>
                  <button type="button" data-testid="fc-place" onClick={() => onPlace(c)}>{t('desk.place')}</button>
                  <button type="button" data-testid="fc-try" onClick={() => onTry(c)}>{t('fc.try')}</button>
                  <button type="button" data-testid="fc-remove" aria-label={t('fc.remove')} onClick={() => remove(c)}>×</button>
                </div>)}
            </th>);
        })}</tr></thead>
        <tbody>
          <tr><th scope="row">{t('cc.specimen')}</th>{shown.map(c => <td key={keyOf(c.face)}><Sample api={api} row={c.row} partner={partner} role={role} pageMode={view === 'pages'} /></td>)}</tr>
          {view === 'table' && COMPARE_METRICS.map(k => (
            <tr key={k} data-metric={k}>
              <th scope="row"><TermTip term={k}><span data-term={termKey(k)}>{termName(k)}</span></TermTip></th>
              {shown.map(c => <MetricCell key={keyOf(c.face)} m={dataOf(c)?.metrics[k]} base={baseData?.metrics[k]} />)}
            </tr>
          ))}
          {view === 'table' && <>
            <tr data-row="vi"><th scope="row">{t('cc.vi')}</th>{shown.map(c => { const d = dataOf(c); return <td key={keyOf(c.face)} data-testid="fc-vi">{!d ? '—' : d.vi.supports && d.vi.missing === 0 ? t('strip.viOk') : t('strip.viMissing', { n: d.vi.missing })}</td>; })}</tr>
            <tr data-row="license"><th scope="row">License</th>{shown.map(c => <td key={keyOf(c.face)}>{dataOf(c)?.license ?? '—'}</td>)}</tr>
          </>}
        </tbody>
      </table>)}
    </div>
  );
}
