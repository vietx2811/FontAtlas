import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Api, SourceDto } from '../../api';
import { LibraryGrid } from '../../features/library-grid';
import { sharedFontLoader } from '../../features/library-grid/sharedLoader';
import type { FaceRow } from '../../features/library-grid';
import { openPrivacySettings } from '../../privacy';
import { useScanStore } from '../../store/scan';
import { refreshActivation } from '../../store/activationSync';
import { dropFromSelection, rowKey, useFavoriteStore, useSelectionStore } from '../../store/selection';
import { useFaceActions } from '../faces/FaceActions';
import { useLibrarySelection } from './useLibrarySelection';
import { ZeroState } from '../zero/ZeroState';
import { libraryEmpty, type Empty } from '../zero/zeroStates';
import { COMPOSER_INSET } from './SampleComposer';
import { faceKey, singleTagId, toGridQuery, useUiStore } from '../../store/ui';

type Props = { api: Api; width?: number; height?: number };

/** Nối store → GridQuery → LibraryGrid. Không lọc ở client. */
export function Library({ api, width, height }: Props): React.JSX.Element {
  const filters = useUiStore(s => s.filters);
  const familyScope = useUiStore(s => s.familyScope);
  const view = useUiStore(s => s.view);
  const tag = useUiStore(s => { const id = singleTagId(s.filters); return id === null ? null : String(id); });
  const query = useUiStore(s => s.query);
  const viOnly = useUiStore(s => s.viOnly);
  const recentViews = useUiStore(s => s.recentViews);
  const sample = useUiStore(s => s.sample);
  const select = useUiStore(s => s.select);
  const selectedKey = useUiStore(s => s.selectedId);
  const fontSize = useUiStore(s => s.sampleSize);
  const layout = useUiStore(s => s.layout);
  const active = useUiStore(s => s.active);
  const scanVersion = useScanStore(s => s.version);
  const scanning = useScanStore(s => s.scanning);
  const access = useScanStore(s => s.access);
  const cloud = useScanStore(s => s.cloud);
  const [sources, setSources] = useState<SourceDto[]>([]);
  useEffect(() => { let alive = true; api.listSources().then(x => { if (alive) setSources(x); }, () => {}); return () => { alive = false; }; }, [api, scanVersion]);
  const gridRev = useUiStore(s => s.gridRev); // bỏ sao trong view Yêu thích, gỡ khỏi dự án → nạp lại để thẻ biến mất ngay
  const favs = useFavoriteStore(s => s.overrides);
  const actions = useFaceActions();
  const wrap = useRef<HTMLDivElement>(null);
  useEffect(() => { void refreshActivation(api); }, [api]);
  const isActive = useCallback((r: FaceRow) => active.has(r.hashHex), [active]);
  const activeScope = useUiStore(s => s.activeScope);
  const available = useUiStore(s => s.available);
  const activeScopeOf = useCallback((r: FaceRow) => activeScope.get(r.hashHex) ?? (available.has(r.hashHex) ? 'available' as const : undefined), [activeScope, available]);
  const toggleActive = useCallback((r: FaceRow) => { void actions?.setActive([r], !useUiStore.getState().active.has(r.hashHex)); }, [actions]);
  const isFavorite = useCallback((r: FaceRow) => favs.get(r.faceId) ?? r.isFavorite, [favs]);
  const toggleFavorite = useCallback((r: FaceRow) => {
    const next = !(useFavoriteStore.getState().overrides.get(r.faceId) ?? r.isFavorite);
    api.setFavorite({ hashHex: r.hashHex, faceIndex: r.faceIndex }, next)
      .then(() => {
        useFavoriteStore.getState().set([r], next);
        const inStar = !next && useUiStore.getState().view === 'star';
        if (inStar) dropFromSelection([r]); // thẻ sắp biến khỏi truy vấn
        useUiStore.getState().bumpLibrary(inStar);
      })
      .catch(e => console.warn('set_favorite failed', e));
  }, [api]);
  // "Gần đây": chọn/mở font đẩy nó lên đầu danh sách, nhưng thứ tự trên lưới đứng yên cho tới khi rời view (hoặc mở lại thư viện).
  const sort = useUiStore(s => s.sort);
  const locked = view === 'recent' || view === 'active'; // thứ tự khoá: đổi cách sắp xếp không được xoá vùng chọn
  const inRecent = view === 'recent';
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const frozenRecents = useMemo(() => recentViews, [inRecent]);
  const gridQuery = useMemo(() => toGridQuery({ view, query, viOnly, recentViews: frozenRecents, active, filters, familyScope, sort }), [view, query, viOnly, frozenRecents, active, filters, familyScope, sort]);
  // Vùng chọn gắn với view/truy vấn; "Gần đây"/"Đang bật" đổi thành viên khi bấm chọn nên không tính vào khoá.
  const scopeKey = useMemo(() => JSON.stringify([view, query, viOnly, filters, familyScope?.name ?? null, locked ? null : gridQuery.sort ? [gridQuery.sort, gridQuery.descending ?? false] : null]), [view, query, viOnly, filters, familyScope, locked, gridQuery.sort, gridQuery.descending]);
  const selection = useLibrarySelection(api, gridQuery, scopeKey, wrap);
  const onContextMenu = useMemo(() => actions && ((row: FaceRow, index: number, at: { x: number; y: number; keyboard: boolean; el: HTMLElement }) => {
    const st = useSelectionStore.getState();
    let target: FaceRow[];
    if (st.rows.has(rowKey(row))) target = [...st.rows.values()];
    else { st.replace([row], { key: rowKey(row), index }); useUiStore.getState().select(faceKey(row)); target = [row]; }
    actions.openMenu(target, { x: at.x, y: at.y }, { keyboard: at.keyboard, returnFocus: at.el });
  }), [actions]);
  const [selectedFaceId, setSelectedFaceId] = useState<number | null>(null);

  const filtered = !!(filters.variable != null || filters.italic != null || filters.rawAxisRanges?.length || familyScope);
  const empty: Empty = libraryEmpty({ view, tag, query, viOnly, sources, scanning, access, cloud, filtered, familyName: familyScope?.name });
  const onEmptyAction = (id: string) => {
    const ui = useUiStore.getState();
    if (id === 'add-folder') ui.requestAddFolder();
    else if (id === 'go-all') ui.setView('all');
    else if (id === 'clear-filters') { ui.setQuery(''); ui.setViOnly(false); ui.setTag(null); ui.setFilters({}); }
    else if (id === 'grant') void openPrivacySettings();
    else if (id === 'rescan') void api.rescanSources(view.startsWith('src:') ? [Number(view.slice(4))] : Object.keys(access).length ? Object.keys(access).map(Number) : undefined);
  };
  const onSelect = useCallback((row: FaceRow) => { setSelectedFaceId(row.faceId); select(faceKey(row)); }, [select]);
  const shared = useMemo(() => sharedFontLoader(api), [api]);
  return (
    <div ref={wrap} className="fa-lib" style={{ position: 'relative', width: width ?? '100%', height: height ?? '100%' }}>
      <LibraryGrid key={`${gridRev}:${scanVersion}`} api={api} loader={shared.loader} velocity={shared.velocity} query={gridQuery} sampleText={sample.trim() ? sample : 'Ặa' /* i18n-ok: chữ mẫu */} bottomInset={COMPOSER_INSET} emptyState={<ZeroState glyph={empty.glyph} text={empty.text} action={empty.action} secondary={empty.secondary} onAction={onEmptyAction} />} fontSize={fontSize} layout={layout} isActive={isActive} activeScopeOf={activeScopeOf} onToggleActive={toggleActive} isFavorite={isFavorite} onToggleFavorite={toggleFavorite} width={width} height={height} selectedFaceId={selectedFaceId} selectedKey={selectedKey} onSelect={onSelect}
        selectedKeys={selection.keys} onPick={selection.pick} onContextMenu={onContextMenu ?? undefined} />
    </div>
  );
}
