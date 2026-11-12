import { useEffect, useMemo, useRef, useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { buildCompare, COMPARE_METRICS, type CompareFace, type CompareMetric } from '../../api/compare';
import type { ComboRoleDto, FaceRowDto } from '../../bindings';
import { t, getLocale, tn } from '../../i18n';
import { deltaOf, deltaText } from '../compare/compareMath';
import { numberText, unitText } from '../detail/tabs/measurements';
import { useFontStatus } from '../detail/useFontFamily';
import { Icon } from '../sidebar/icons';
import { NoteMark } from '../ui/noteMark';
import { Tooltip } from '../ui/Tooltip';
import { TermTip, termKey, termName } from '../ui/terms';
import { isHeadingLike, roleLabel } from './deskRoles';

export type FcFace = { face: FaceKeyDto; row: FaceRowDto };
type SortKey = 'order' | 'name' | 'vi' | (typeof COMPARE_METRICS)[number];
const keyOf = (f: FaceKeyDto) => `${f.hashHex}:${f.faceIndex}`;
const nameOf = (r: FaceRowDto) => `${r.family}${r.subfamily ? ` ${r.subfamily}` : ''}`;
const roleName = roleLabel;

/** Đọc số đo cho nhiều font; nhớ theo khoá, lỗi thì không nhớ (lần sau đọc lại). */
function useFacesData(api: Api, faces: FaceKeyDto[]): { cache: Map<string, CompareFace | null>; failed: ReadonlySet<string> } {
  const cache = useRef(new Map<string, CompareFace | null>());
  const failed = useRef(new Set<string>()); // lỗi đọc: báo ở cột, không gọi lại liên tục (chỉ thử lại khi danh sách cột đổi)
  const pending = useRef(new Set<string>());
  const alive = useRef(true);
  const [, bump] = useState(0);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const sig = faces.map(keyOf).join('|');
  useEffect(() => {
    for (const f of faces) {
      const k = keyOf(f);
      if (cache.current.has(k) || pending.current.has(k) || failed.current.has(k)) continue;
      pending.current.add(k);
      api.getFace(f, getLocale()).then(d => buildCompare([d]).faces[0] ?? null, () => undefined).then(cf => {
        pending.current.delete(k);
        if (cf !== undefined) cache.current.set(k, cf); else failed.current.add(k);
        if (alive.current) bump(n => n + 1);
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api, sig]);
  return { cache: cache.current, failed: failed.current };
}

function Sample({ api, row, partner, role, pageMode }: { api: Api; row: FaceRowDto; partner: FaceRowDto | null; role: ComboRoleDto; pageMode: boolean }): React.JSX.Element {
  // Font cloud đi qua cổng tải của backend (≤ 4, hạn chờ, theo công tắc): đang tải → skeleton + ghi chú; lỗi/tắt công tắc → ghi chú.
  const own = useFontStatus(api, row); const other = useFontStatus(api, partner);
  const fam = own.family; const pfam = other.family;
  if (row.onCloudOnly && own.status !== 'ready') {
    return own.status === 'loading'
      ? <div className="fa-cc-page" data-testid="fc-cloud-loading" aria-busy="true"><span className="fa-cloud-skeleton" aria-hidden /><NoteMark kind="info" inline text={t('cloud.loading')} /></div>
      : <div className="fa-cc-page" data-testid="fc-cloud"><NoteMark kind="warn" inline text={t('cloud.errFailed')} /> <span className="fa-cc-na">{t('fc.cloudOnly')}</span></div>;
  }
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
  api: Api; role: ComboRoleDto; base: FcFace | null; partner: FaceRowDto | null; suggestions: FcFace[]; /** true = đã đặt (cột sẽ bị bỏ vì font thành mốc); false = lỗi, giữ cột. */ onPlace(f: FcFace): Promise<boolean>; onTry(f: FcFace): void;
}): React.JSX.Element {
  const [cols, setCols] = useState<FcFace[]>(() => suggestions.slice(0, 4));
  const touched = useRef(false); // người dùng đã thêm/bỏ cột: không seed lại từ gợi ý
  const sugSig = suggestions.map(s => keyOf(s.face)).join('|');
  // Gợi ý về sau khi mở (hoặc đổi vì đặt font): chưa động tới cột thì dùng gợi ý mới.
  useEffect(() => { if (!touched.current) setCols(suggestions.slice(0, 4)); // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sugSig]);
  const [pinned, setPinned] = useState<string[]>([]);
  const [sort, setSort] = useState<SortKey>('order');
  const [desc, setDesc] = useState(false);
  const [view, setView] = useState<'table' | 'pages'>('table');
  const baseKey = base ? keyOf(base.face) : null;
  const colsNoBase = useMemo(() => cols.filter(c => keyOf(c.face) !== baseKey), [cols, baseKey]); // font vừa đặt thành mốc không được lặp thành cột
  const { cache: data, failed } = useFacesData(api, [...(base ? [base.face] : []), ...colsNoBase.map(c => c.face)]);
  const baseData = base ? data.get(keyOf(base.face)) ?? null : null;

  const value = (c: FcFace): number | string | null => {
    const d = data.get(keyOf(c.face));
    if (sort === 'name') return nameOf(c.row).toLowerCase();
    if (sort === 'vi') return d ? d.vi.missing : null;
    if (sort === 'order') return null;
    return d?.metrics[sort]?.value ?? null;
  };
  const ordered = useMemo(() => {
    const pins = pinned.map(k => colsNoBase.find(c => keyOf(c.face) === k)).filter((c): c is FcFace => !!c);
    const rest = colsNoBase.filter(c => !pinned.includes(keyOf(c.face)));
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
  }, [colsNoBase, pinned, sort, desc, data.size]);

  const shown = [...(base ? [base] : []), ...ordered];
  const addable = suggestions.filter(s => !cols.some(c => keyOf(c.face) === keyOf(s.face)) && !(base && keyOf(base.face) === keyOf(s.face)));
  // Thêm cột từ THƯ VIỆN (ngoài gợi ý ghép): tìm theo tên, tối đa 30 kết quả, bỏ font đã có cột / mốc.
  const [search, setSearch] = useState('');
  const [lib, setLib] = useState<FcFace[]>([]);
  useEffect(() => {
    let alive = true;
    const timer = setTimeout(() => {
      api.queryFaces({ text: search.trim() || undefined, limit: 30 }).then(p => { if (alive) setLib(p.items.map(r => ({ face: { hashHex: r.hashHex, faceIndex: r.faceIndex }, row: r as unknown as FaceRowDto }))); }, () => { if (alive) setLib([]); });
    }, search ? 200 : 0);
    return () => { alive = false; clearTimeout(timer); };
  }, [api, search]);
  const libAddable = lib.filter(s => !cols.some(c => keyOf(c.face) === keyOf(s.face)) && keyOf(s.face) !== baseKey && !addable.some(a => keyOf(a.face) === keyOf(s.face)));
  const root = useRef<HTMLDivElement>(null);
  // Nút vừa thao tác biến mất (bỏ cột, đặt xong): trả focus về Desk để phím tắt còn sống.
  const refocus = () => requestAnimationFrame(() => (root.current?.closest('.fa-desk') as HTMLElement | null)?.focus());
  const remove = (f: FcFace) => { touched.current = true; refocus(); setCols(cs => cs.filter(c => keyOf(c.face) !== keyOf(f.face))); setPinned(p => p.filter(k => k !== keyOf(f.face))); };
  const togglePin = (f: FcFace) => { touched.current = true; setPinned(p => (p.includes(keyOf(f.face)) ? p.filter(k => k !== keyOf(f.face)) : [...p, keyOf(f.face)])); };
  // Đặt vào vai thành công → font thành mốc: bỏ khỏi cột/ghim (không lặp), trả focus về Desk.
  const place = async (f: FcFace) => {
    const ok = await onPlace(f);
    if (ok) { touched.current = true; setCols(cs => cs.filter(c => keyOf(c.face) !== keyOf(f.face))); setPinned(p => p.filter(k => k !== keyOf(f.face))); refocus(); }
  };
  const dataOf = (f: FcFace) => data.get(keyOf(f.face)) ?? null;

  return (
    <div ref={root} className="fa-cc fa-fc" data-testid="font-compare">
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
          <input type="search" data-testid="fc-add-search" value={search} placeholder={t('fc.add.search')} aria-label={t('fc.add.search')} onChange={e => setSearch(e.target.value)} />
          <select data-testid="fc-add" value="" disabled={addable.length + libAddable.length === 0} onChange={e => { const f = [...addable, ...libAddable].find(a => keyOf(a.face) === e.target.value); if (f) { touched.current = true; setCols(cs => [...cs, f]); } }}>
            <option value="">{addable.length + libAddable.length ? t('fc.add.pick') : t('fc.add.none')}</option>
            {addable.length > 0 && <optgroup label={t('fc.add.group.pair')}>{addable.map(a => <option key={keyOf(a.face)} value={keyOf(a.face)}>{nameOf(a.row)}</option>)}</optgroup>}
            {libAddable.length > 0 && <optgroup label={t('fc.add.group.lib')}>{libAddable.map(a => <option key={keyOf(a.face)} value={keyOf(a.face)}>{nameOf(a.row)}</option>)}</optgroup>}
          </select>
        </label>
        <span className="fa-fc-role">{t('fc.role')}: <b>{roleName(role)}</b></span>
      </div>
      {!base && <p className="fa-desk-hint" data-testid="fc-nobase">{t('fc.noBase', { role: roleName(role) })}</p>}
      {shown.length === 0 && <p className="fa-desk-hint">{t('desk.noCands')}</p>}
      {shown.length > 0 && (
      <table>
        <thead><tr><td />{shown.map(c => {
          const isBase = !!base && keyOf(c.face) === keyOf(base.face); const pin = pinned.includes(keyOf(c.face));
          return (
            <th scope="col" key={keyOf(c.face)} data-testid="fc-col" data-base={isBase || undefined} data-pinned={pin || undefined}>
              <div className="fa-cc-title">{pin && <span className="fa-cc-ico" role="img" aria-label={t('fc.pinned')}><Icon name="pin" size={12} /></span>}{nameOf(c.row)}{failed.has(keyOf(c.face)) && <NoteMark inline kind="warn" testId="fc-readfail" text={t('fc.readFail')} />}{isBase && <small> · {t('compare.base')}</small>}</div>
              {!isBase && (
                <div className="fa-cc-actions">
                  <button type="button" data-testid="fc-pin" aria-pressed={pin} onClick={() => togglePin(c)}>{pin ? t('fc.unpin') : t('fc.pin')}</button>
                  <button type="button" data-testid="fc-place" onClick={() => void place(c)}>{t('desk.place')}</button>
                  <button type="button" data-testid="fc-try" onClick={() => onTry(c)}>{t('fc.try')}</button>
                  <button type="button" data-testid="fc-remove" aria-label={t('fc.remove')} onClick={() => remove(c)}>×</button>
                </div>)}
            </th>);
        })}</tr></thead>
        <tbody>
          <tr><th scope="row">{t('cc.specimen')}</th>{shown.map(c => <td key={keyOf(c.face)}><Sample api={api} row={c.row} partner={partner} role={role} pageMode={view === 'pages'} /></td>)}</tr>
          {view === 'table' && COMPARE_METRICS.map(k => (
            <tr key={k} data-metric={k}>
              <th scope="row" className="has-tip"><TermTip term={k}><span data-term={termKey(k)}>{termName(k)}</span></TermTip></th>
              {shown.map(c => <MetricCell key={keyOf(c.face)} m={dataOf(c)?.metrics[k]} base={baseData?.metrics[k]} />)}
            </tr>
          ))}
          {view === 'table' && <>
            <tr data-row="vi"><th scope="row" className="has-tip"><Tooltip label={t('cc.vi.tip')}><span>{t('cc.vi')}</span></Tooltip></th>{shown.map(c => { const d = dataOf(c); return <td key={keyOf(c.face)} data-testid="fc-vi">{!d ? '—' : d.vi.supports && d.vi.missing === 0 ? t('strip.viOk') : tn('strip.viMissing', { n: d.vi.missing })}</td>; })}</tr>
            <tr data-row="license"><th scope="row">{t('cc.license')}</th>{shown.map(c => <td key={keyOf(c.face)}>{dataOf(c)?.license ?? '—'}</td>)}</tr>
          </>}
        </tbody>
      </table>)}
    </div>
  );
}
