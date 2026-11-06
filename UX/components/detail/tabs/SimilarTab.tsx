import { useEffect, useMemo, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import type { Api, FaceDetailDto, SimilarSource, SimilarFamilyPageDto } from '../../../api';
import { useModelStore, useModelUi, type ModelStore } from '../../../store/model';
import { useScanStore } from '../../../store/scan';
import { faceKey, toMetadataQuery, useUiStore } from '../../../store/ui';
import { ModelDetails, ModelSpinner } from '../../model/ModelFeedback';
import { ZeroState } from '../../zero/ZeroState';
import { AXIS_META } from './AnatomyTab';

export const SOURCE_BADGE: Record<SimilarSource, string> = { axes: 'Số đo', axes_and_image: 'Số đo + Hình ảnh', image: 'Hình ảnh' };
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
    body = `${ui.text}. Kết quả hiện chỉ dựa trên số đo.`;
    actions = ui.actions.map(a => <button key={a.label} type="button" onClick={() => act(a.id)}>{a.label}</button>);
  }
  else if (ui.code === 'E8' || ui.code === 'E9') body = `${ui.code === 'E9' ? 'Phân tích hình ảnh tạm dừng' : 'Đang phân tích hình ảnh'} ${ui.text!.match(/(\d[\d.]*\/\d[\d.]*)/)![1]} — kết quả sẽ đầy đủ hơn khi xong`;
  else if (ui.severity === 'error') {
    body = ui.text;
    actions = ui.actions.map(a => <button key={a.label} type="button" onClick={() => act(a.id)}>{a.label}</button>);
  } else if (!ui.imageReady && !off) {
    body = `Kết quả chỉ dựa trên số đo giải phẫu. Bật phân tích hình ảnh để so cả hình dáng chữ (~371 MB).`;
    actions = <>
      <button type="button" onClick={() => void store.getState().enable(api)}>{embedEnabled ? 'Tải model' : 'Bật'}</button>
      <button type="button" onClick={() => { setOff(true); void api.setSetting(HINT_OFF, '1'); }}>Không hiện lại</button>
    </>;
  }
  if (body === null) return null;
  return <div className="fa-strip" data-testid="image-strip" role="note"><span><ModelSpinner ui={ui} />{body}</span><span className="fa-strip-actions">{actions}</span><ModelDetails detail={ui.detail} /></div>;
}

const PAGE_SIZE = 6;

export function SimilarTab({ api, detail, store = useModelStore }: { api: Api; detail: FaceDetailDto; store?: ModelStore }): React.JSX.Element {
  const state = useUiStore(useShallow(s => ({ view: s.view, tag: s.tag, query: s.query, viOnly: s.viOnly, filters: s.filters, select: s.select, setPanelTab: s.setPanelTab, openFamily: s.openFamily, setView: s.setView, setQuery: s.setQuery, setViOnly: s.setViOnly, setFilters: s.setFilters })));
  const scanVersion = useScanStore(s => s.version);
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
      if (!faces.length) throw new Error('Family chưa có kiểu font khả dụng.');
      if (faces.length > 2000) throw new Error('Family có hơn 2.000 kiểu; hãy mở font đại diện.');
      state.openFamily({ name: family, faces });
    } catch (e) { if (live.current && scopeRef.current === openingScope) setFamilyError(e instanceof Error ? e.message : String(e)); }
    finally { familyLock.current = false; if (live.current) setOpeningFamily(null); }
  };
  const open = (id: string) => { state.select(id); state.setPanelTab('spec'); };
  return <>
    <ImageStrip api={api} store={store} />
    {familyError && <p role="alert" className="fa-soon">{familyError}</p>}
    {openingFamily && <p role="status" className="fa-soon">Đang mở family…</p>}
    {loading ? <p role="status" className="fa-soon">Đang tìm family tương tự…</p>
      : error ? <div role="alert" className="fa-panel-error">Không tìm được family tương tự: {error} <button type="button" onClick={() => setRetry(n => n + 1)}>Thử lại</button></div>
      : !page?.items.length ? <ZeroState compact glyph="ơ" text="Không có family tương tự khớp bộ lọc và dữ liệu hiện có" action={{ id: 'clear', label: 'Xem toàn thư viện' }} onAction={() => { state.setView('all'); state.setQuery(''); state.setViOnly(false); state.setFilters({}); }} />
      : <>
        <p className="fa-soon">{page.total.toLocaleString('vi-VN')} family · mỗi family một font đại diện</p>
        <ul className="fa-sim">{page.items.map(group => {
          const h = group.representative;
          const tooLarge = group.faces.length > 2000 || group.count > 2000;
          return <li key={group.family} data-testid="similar-hit" className="fa-sim-hit">
            <strong className="fa-sim-name">{group.family}</strong>
            <span className="fa-badge" data-testid="source-badge">{SOURCE_BADGE[h.source]}</span>
            <p className="fa-sim-style">{h.row.subfamily ?? 'Kiểu đại diện'} · {group.count.toLocaleString('vi-VN')} kiểu</p>
            <p className="fa-sim-why">{h.same.length ? `Giống: ${names(h.same)}` : 'Chưa có trục giống để giải thích'}{h.differ.length ? ` · Khác: ${names(h.differ)}` : ''}</p>
            <div className="fa-sim-actions"><button type="button" onClick={() => open(faceKey(h.face))}>Mở font</button><button type="button" disabled={tooLarge || openingFamily !== null} title={tooLarge ? 'Family vượt giới hạn 2.000 kiểu của truy vấn thư viện' : undefined} onClick={() => void openFamily(group.family)}>Xem family</button></div>
            {tooLarge && <p className="fa-sim-why">Hơn 2.000 kiểu; hãy mở font đại diện để xem family.</p>}
          </li>;
        })}</ul>
        <nav className="fa-pair-nav" aria-label="Trang family tương tự">
          <button type="button" aria-label="Trang trước" disabled={page.offset === 0} onClick={() => go(Math.max(0, page.offset - PAGE_SIZE))}>‹</button>
          <span>{page.offset + 1}–{page.offset + page.items.length} / {page.total}</span>
          <button type="button" aria-label="Trang sau" disabled={page.offset + page.items.length >= page.total} onClick={() => go(page.offset + PAGE_SIZE)}>›</button>
        </nav>
      </>}
  </>;
}
