import { useMemo, useState } from 'react';
import type { Api } from '../../api';
import { LibraryGrid } from '../../features/library-grid';
import type { FaceRow } from '../../features/library-grid';
import { faceKey, toGridQuery, useUiStore } from '../../store/ui';

type Props = { api: Api; width?: number; height?: number };

/** Nối store → GridQuery → LibraryGrid. Không lọc ở client. */
export function Library({ api, width, height }: Props): React.JSX.Element {
  const view = useUiStore(s => s.view);
  const tag = useUiStore(s => s.tag);
  const query = useUiStore(s => s.query);
  const viOnly = useUiStore(s => s.viOnly);
  const sample = useUiStore(s => s.sample);
  const select = useUiStore(s => s.select);
  const gridQuery = useMemo(() => toGridQuery({ view, tag, query, viOnly }), [view, tag, query, viOnly]);
  const [selectedFaceId, setSelectedFaceId] = useState<number | null>(null);

  if (!gridQuery) {
    return (
      <div data-testid="library-pending" className="fa-library-pending">
        Mục này chờ backend hỗ trợ lọc (chưa có trong lệnh <code>query_faces</code>).
      </div>
    );
  }
  const onSelect = (row: FaceRow) => { setSelectedFaceId(row.faceId); select(faceKey(row)); };
  return <LibraryGrid api={api} query={gridQuery} sampleText={sample} width={width} height={height} selectedFaceId={selectedFaceId} onSelect={onSelect} />;
}
