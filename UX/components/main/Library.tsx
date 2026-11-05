import { useCallback, useEffect, useMemo, useState } from 'react';
import type { Api, SourceDto } from '../../api';
import { LibraryGrid } from '../../features/library-grid';
import type { FaceRow } from '../../features/library-grid';
import { useScanStore } from '../../store/scan';
import { ZeroState } from '../zero/ZeroState';
import { libraryEmpty } from '../zero/zeroStates';
import { COMPOSER_INSET } from './SampleComposer';
import { faceKey, toGridQuery, useUiStore } from '../../store/ui';

type Props = { api: Api; width?: number; height?: number };

/** Nối store → GridQuery → LibraryGrid. Không lọc ở client. */
export function Library({ api, width, height }: Props): React.JSX.Element {
  const view = useUiStore(s => s.view);
  const tag = useUiStore(s => s.tag);
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
  const [sources, setSources] = useState<SourceDto[]>([]);
  useEffect(() => { let alive = true; api.listSources().then(x => { if (alive) setSources(x); }, () => {}); return () => { alive = false; }; }, [api, scanVersion]);
  const [reload, setReload] = useState(0); // bỏ sao trong view Yêu thích → nạp lại để thẻ biến mất ngay
  const [favs, setFavs] = useState<ReadonlyMap<number, boolean>>(new Map());
  useEffect(() => { useUiStore.getState().refreshActive(api).catch(e => console.warn('activation_state failed', e)); }, [api]);
  const isActive = useCallback((r: FaceRow) => active.has(r.hashHex), [active]);
  const isFavorite = useCallback((r: FaceRow) => favs.get(r.faceId) ?? r.isFavorite, [favs]);
  const toggleFavorite = useCallback((r: FaceRow) => {
    const next = !(favs.get(r.faceId) ?? r.isFavorite);
    api.setFavorite({ hashHex: r.hashHex, faceIndex: r.faceIndex }, next)
      .then(() => { setFavs(m => new Map(m).set(r.faceId, next)); if (!next && useUiStore.getState().view === 'star') setReload(n => n + 1); })
      .catch(e => console.warn('set_favorite failed', e));
  }, [api, favs]);
  const gridQuery = useMemo(() => toGridQuery({ view, tag, query, viOnly, recentViews, active }), [view, tag, query, viOnly, recentViews, active]);
  const [selectedFaceId, setSelectedFaceId] = useState<number | null>(null);

  const empty = libraryEmpty({ view, tag, query, viOnly, sources, scanning });
  const onEmptyAction = (id: string) => {
    const ui = useUiStore.getState();
    if (id === 'add-folder') ui.requestAddFolder();
    else if (id === 'go-all') ui.setView('all');
    else if (id === 'clear-filters') { ui.setQuery(''); ui.setViOnly(false); ui.setTag(null); }
    else if (id === 'rescan') void api.rescanSources(view.startsWith('src:') ? [Number(view.slice(4))] : undefined);
  };
  const onSelect = (row: FaceRow) => { setSelectedFaceId(row.faceId); select(faceKey(row)); };
  return <LibraryGrid key={`${reload}:${scanVersion}`} api={api} query={gridQuery} sampleText={sample.trim() ? sample : 'Ặa'} bottomInset={COMPOSER_INSET} emptyState={<ZeroState glyph={empty.glyph} text={empty.text} action={empty.action} onAction={onEmptyAction} />} fontSize={fontSize} layout={layout} isActive={isActive} isFavorite={isFavorite} onToggleFavorite={toggleFavorite} width={width} height={height} selectedFaceId={selectedFaceId} selectedKey={selectedKey} onSelect={onSelect} />;
}
