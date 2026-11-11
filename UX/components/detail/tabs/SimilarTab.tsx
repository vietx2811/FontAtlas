import { useEffect, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import type { Api, FaceDetailDto, SimilarSource, SimilarFamilyPageDto } from '../../../api';
import { useModelStore, useModelUi, type ModelStore } from '../../../store/model';
import { useScanStore } from '../../../store/scan';
import { faceKey, toMetadataQuery, useUiStore } from '../../../store/ui';
import { ModelDetails, ModelSpinner } from '../../model/ModelFeedback';
import { ZeroState } from '../../zero/ZeroState';
import { faceMenuProps, useFaceActions } from '../../faces/FaceActions';
import { FaceTitle } from '../../ui/FaceTitle';
import { Tooltip } from '../../ui/Tooltip';
import { AXIS_META } from './AnatomyTab';
import { t, formatNumber, tn } from '../../../i18n';

// Getter: chuỗi theo ngôn ngữ hiện tại tại thời điểm đọc.
export const SOURCE_BADGE: Record<SimilarSource, string> = { get axes() { return t('sim.badge.axes'); }, get axes_and_image() { return t('sim.badge.both'); }, get image() { return t('sim.badge.image'); } };
export const SOURCE_TIP: Record<SimilarSource, string> = {
  get axes() { return t('sim.tip.axes'); },
  get axes_and_image() { return t('sim.tip.both'); },
  get image() { return t('sim.tip.image'); },
};
const names = (keys: string[]): string => keys.map(k => (AXIS_META[k]?.label ?? k).toLowerCase()).join(', ');

/** Ba font gần nhất. Không điểm, không phần trăm: chỉ "Giống ở / Khác ở" và nguồn phân tích. */
const HINT_OFF = 'ui.similar_hint_off';

/** Dải gợi ý phân tích hình ảnh theo trạng thái model/embed (UX 03 §3.3). */
function ImageStrip({ api, store }: { api: Api; store: ModelStore }): React.JSX.Element | null {
  const ui = useModelUi(store);
  const embedEnabled = store(s => s.embedEnabled);
  const [off, setOff] = useState(true); // mặc định ẩn cho tới khi đọc xong setting
  useEffect(() => { let alive = true; api.getSetting(HINT_OFF).then(v => { if (alive) setOff(v === '1'); }, () => { if (alive) setOff(false); }); return () => { alive = false; }; }, [api]);
  const act = (id: Parameters<ReturnType<typeof store.getState>['act']>[1]) => void store.getState().act(api, id);

  let body: React.ReactNode = null; let actions: React.ReactNode = null;
  if (ui.code === 'E2' || ui.code === 'E3') {
    body = t('sim.strip.measureOnly', { text: ui.text ?? '' });
    actions = ui.actions.map(a => <button key={a.label} type="button" onClick={() => act(a.id)}>{a.label}</button>);
  }
  else if (ui.code === 'E8' || ui.code === 'E9') body = t(ui.code === 'E9' ? 'sim.strip.pausedProgress' : 'sim.strip.progress', { n: ui.text!.match(/(\d[\d.,]*\/\d[\d.,]*)/)![1] });
  else if (ui.severity === 'error') {
    body = ui.text;
    actions = ui.actions.map(a => <button key={a.label} type="button" onClick={() => act(a.id)}>{a.label}</button>);
  } else if (!ui.imageReady && !off) {
    body = t('sim.strip.hint');
    actions = <>
      <button type="button" onClick={() => void store.getState().enable(api)}>{embedEnabled ? t('sim.strip.downloadModel') : t('sim.strip.enable')}</button>
      <button type="button" onClick={() => { setOff(true); void api.setSetting(HINT_OFF, '1'); }}>{t('sim.strip.dismiss')}</button>
    </>;
  }
  if (body === null) return null;
  return <div className="fa-strip" data-testid="image-strip" role="note"><span><ModelSpinner ui={ui} />{body}</span><span className="fa-strip-actions">{actions}</span><ModelDetails detail={ui.detail} /></div>;
}

const PAGE_SIZE = 6;

export function SimilarTab({ api, detail, store = useModelStore }: { api: Api; detail: FaceDetailDto; store?: ModelStore }): React.JSX.Element {
  const state = useUiStore(useShallow(s => ({ view: s.view, query: s.query, viOnly: s.viOnly, filters: s.filters, select: s.select, setPanelTab: s.setPanelTab, openFamily: s.openFamily, setView: s.setView, setQuery: s.setQuery, setViOnly: s.setViOnly, setFilters: s.setFilters })));
  const scanVersion = useScanStore(s => s.version);
  const faceActions = useFaceActions();
  const queryKey = JSON.stringify(toMetadataQuery(state));
  const q = useMemo(() => JSON.parse(queryKey) as ReturnType<typeof toMetadataQuery>, [queryKey]);
  const [paging, setPaging] = useState({ scope: '', offset: 0 });
  const [openingFamily, setOpeningFamily] = useState<string | null>(null);
  const [familyError, setFamilyError] = useState<string | null>(null);
  const familyLock = useRef(false);
  const [page, setPage] = useState<SimilarFamilyPageDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [loading, setLoading] = useState(true);
  const key = faceKey(detail.row);
  const imageReady = useModelUi(store).imageReady;
  // Gắn offset với face/query để không gửi trang cũ cho font hoặc bộ lọc mới.
  const scope = `${key}:${queryKey}:${imageReady}:${scanVersion}`;
  const live = useRef(true);
  const scopeRef = useRef(scope); scopeRef.current = scope;
  useEffect(() => { live.current = true; return () => { live.current = false; }; }, []);
  const currentOffset = paging.scope === scope ? paging.offset : 0;
  useEffect(() => { setPaging({ scope, offset: 0 }); setFamilyError(null); }, [scope]);
  useEffect(() => {
    let alive = true; let redirected = false;
    setLoading(true); setError(null);
    api.similarFamilies(detail.row, { useImage: imageReady }, { ...q, offset: currentOffset, limit: PAGE_SIZE })
      .then(p => {
        if (!alive) return;
        if (!p.items.length && p.total > 0 && currentOffset > 0) {
          redirected = true;
          const last = Math.floor((p.total - 1) / PAGE_SIZE) * PAGE_SIZE;
          setPaging({ scope, offset: Math.min(last, Math.max(0, currentOffset - PAGE_SIZE)) });
        } else setPage(p);
      })
      .catch(e => { if (alive) { setPage(null); setError(e instanceof Error ? e.message : String(e)); } })
      .finally(() => { if (alive && !redirected) setLoading(false); });
    return () => { alive = false; };
  }, [api, key, q, imageReady, currentOffset, retry, scanVersion]);
  const go = (to: number) => setPaging({ scope, offset: to });
  const openFamily = async (family: string) => {
    if (familyLock.current) return;
    const openingScope = scope;
    familyLock.current = true; setOpeningFamily(family); setFamilyError(null);
    try {
      // Lấy toàn bộ family, không khóa membership theo bộ lọc của kết quả Similar.
      const styles = await api.getFamilyStyles(family);
      if (!live.current || scopeRef.current !== openingScope) return;
      const faces = [...new Map(styles.map(s => [faceKey(s.face), s.face])).values()];
      if (!faces.length) throw new Error(t('sim.famEmpty'));
      if (faces.length > 2000) throw new Error(t('faces.famTooBig'));
      state.openFamily({ name: family, faces });
    } catch (e) { if (live.current && scopeRef.current === openingScope) setFamilyError(e instanceof Error ? e.message : String(e)); }
    finally { familyLock.current = false; if (live.current) setOpeningFamily(null); }
  };
  const open = (id: string) => { state.select(id); state.setPanelTab('spec'); };
  return <>
    <ImageStrip api={api} store={store} />
    {familyError && <p role="alert" className="fa-soon">{familyError}</p>}
    {openingFamily && <p role="status" className="fa-soon">{t('sim.opening')}</p>}
    {loading ? <p role="status" className="fa-soon">{t('sim.finding')}</p>
      : error ? <div role="alert" className="fa-panel-error">{t('sim.findFail', { msg: error })} <button type="button" onClick={() => setRetry(n => n + 1)}>{t('det.retry')}</button></div>
      : !page?.items.length ? <ZeroState compact glyph="ơ" text={t('sim.none')} action={{ id: 'clear', label: t('sim.viewAll') }} onAction={() => { state.setView('all'); state.setQuery(''); state.setViOnly(false); state.setFilters({}); }} />
      : <>
        <p className="fa-soon">{t('sim.summary', { n: formatNumber(page.total) })}</p>
        <ul className="fa-sim">{page.items.map(group => {
          const h = group.representative;
          const tooLarge = group.faces.length > 2000 || group.count > 2000;
          return <li key={group.family} data-testid="similar-hit" className="fa-sim-hit" {...faceMenuProps(faceActions, [h.row])}>
            <FaceTitle as="strong" api={api} face={h.row} className="fa-sim-name">{group.family}</FaceTitle>
            <Tooltip label={SOURCE_TIP[h.source]}><span className="fa-badge" data-testid="source-badge">{SOURCE_BADGE[h.source]}</span></Tooltip>
            <p className="fa-sim-style">{h.row.subfamily ?? t('sim.repStyle')} · {tn('sim.styles', { n: formatNumber(group.count) })}</p>
            <p className="fa-sim-why">{h.same.length ? t('sim.same', { names: names(h.same) }) : t('sim.noAxes')}{h.differ.length ? t('sim.differ', { names: names(h.differ) }) : ''}</p>
            <div className="fa-sim-actions"><button type="button" onClick={() => open(faceKey(h.face))}>{t('sim.openFont')}</button><Tooltip label={t('sim.tooLargeTip')} disabled={!tooLarge}><button type="button" aria-disabled={tooLarge || openingFamily !== null || undefined} onClick={() => { if (!tooLarge && openingFamily === null) void openFamily(group.family); }}>{t('sim.viewFamily')}</button></Tooltip></div>
            {tooLarge && <p className="fa-sim-why">{t('sim.tooLarge')}</p>}
          </li>;
        })}</ul>
        <nav className="fa-pair-nav" aria-label={t('sim.pageNav')}>
          <button type="button" aria-label={t('sim.prevPage')} disabled={page.offset === 0} onClick={() => go(Math.max(0, page.offset - PAGE_SIZE))}>‹</button>
          <span>{page.offset + 1}–{page.offset + page.items.length} / {page.total}</span>
          <button type="button" aria-label={t('sim.nextPage')} disabled={page.offset + page.items.length >= page.total} onClick={() => go(page.offset + PAGE_SIZE)}>›</button>
        </nav>
      </>}
  </>;
}
