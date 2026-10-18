import { useEffect, useRef, useState } from 'react';
import { List, type RowComponentProps } from 'react-window';
import type { LibraryApi } from './api';
import { DEFAULT_PREVIEW_MODE, FaceCell, type PreviewMode } from './FaceCell';
import { FontFaceLoader, ScrollVelocity, browserLoaderDeps } from './fontFaceLoader';
import { usePagedFaces, type PagedFaces } from './pagedFaces';
import { PreviewBlobs } from './previewBlobs';
import { DEFAULT_SAMPLE, cssSize, type FaceRow, type GridQuery, type PxBucket } from './types';

export { HOVER_DELAY_MS, DEFAULT_PREVIEW_MODE, type CellState, type PreviewMode } from './FaceCell';

export type LibraryGridProps = {
  api: LibraryApi; query: GridQuery; sampleText?: string; px?: PxBucket; previewMode?: PreviewMode;
  selectedFaceId?: number | null; onSelect?(row: FaceRow): void; loader?: FontFaceLoader; blobs?: PreviewBlobs;
  width?: number; height?: number;
};

type Applied = { rendererVersion: string; generation: number; text: string; px: PxBucket };

type RowProps = {
  cols: number; total: number; paged: PagedFaces | null; version: number; applied: Applied | null;
  api: LibraryApi; px: PxBucket; text: string; mode: PreviewMode; selectedFaceId: number | null;
  blobs: PreviewBlobs; loader: FontFaceLoader; onSelect(row: FaceRow): void;
};

function Row({ index, style, cols, total, paged, applied, api, px, text, mode, selectedFaceId, blobs, loader, onSelect }: RowComponentProps<RowProps>) {
  const cells = [];
  for (let c = 0; c < cols; c++) {
    const i = index * cols + c;
    if (i >= total) { cells.push(<div key={c} style={{ flex: 1, margin: 4 }} />); continue; }
    const row = paged?.rowAt(i);
    // URL dùng chữ/cỡ "đã áp dụng" (cùng thế hệ backend) → ảnh cũ giữ nguyên trong lúc xin thế hệ mới.
    const url = row && applied
      ? api.previewUrl(row, { text: applied.text, px: applied.px, generation: applied.generation, rendererVersion: applied.rendererVersion })
      : null;
    cells.push(
      <FaceCell key={row ? row.faceId : `p${i}`} row={row} url={url} px={px} text={text} mode={mode}
        selected={!!row && row.faceId === selectedFaceId} blobs={blobs} loader={loader} onSelect={onSelect} />,
    );
  }
  return <div role="row" style={{ ...style, display: 'flex' }}>{cells}</div>;
}

function useSize(ref: React.RefObject<HTMLDivElement | null>, fixed: { width?: number; height?: number }) {
  const [size, setSize] = useState({ width: fixed.width ?? 1280, height: fixed.height ?? 800 });
  useEffect(() => {
    if (fixed.width !== undefined && fixed.height !== undefined) return;
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setSize({ width: fixed.width ?? e.contentRect.width, height: fixed.height ?? e.contentRect.height }));
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref, fixed.width, fixed.height]);
  return size;
}

export function LibraryGrid({
  api, query, sampleText = DEFAULT_SAMPLE, px = 48, previewMode = DEFAULT_PREVIEW_MODE,
  selectedFaceId = null, onSelect = () => {}, loader: loaderProp, blobs: blobsProp, width, height,
}: LibraryGridProps): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null);
  const size = useSize(box, { width, height });
  const { paged, snap } = usePagedFaces(api, query);
  // Các đối tượng này không có tài nguyên cần huỷ → khởi tạo lười an toàn với StrictMode.
  const [velocity] = useState(() => new ScrollVelocity());
  const [ownLoader] = useState(() => (loaderProp ? null : new FontFaceLoader(browserLoaderDeps(api, velocity))));
  const [ownBlobs] = useState(() => (blobsProp ? null : new PreviewBlobs()));
  const loader = loaderProp ?? ownLoader!;
  const blobs = blobsProp ?? ownBlobs!;

  // Thế hệ preview do backend sở hữu: lấy giá trị đầu qua previewInfo, xin mới khi chữ/cỡ đổi.
  const [applied, setApplied] = useState<Applied | null>(null);
  const want = useRef({ text: sampleText, px });
  want.current = { text: sampleText, px };
  useEffect(() => {
    let alive = true;
    api.previewInfo().then(info => {
      if (alive) setApplied({ rendererVersion: info.rendererVersion, generation: info.generation, ...want.current });
    });
    return () => { alive = false; };
  }, [api]);
  const pending = useRef<string | null>(null);
  useEffect(() => {
    if (!applied || (applied.text === sampleText && applied.px === px)) return;
    const key = `${px}\0${sampleText}`;
    if (pending.current === key) return;
    pending.current = key;
    let alive = true;
    api.bumpGeneration().then(g => {
      if (pending.current === key) pending.current = null;
      if (alive) setApplied(a => (a ? { ...a, generation: g, text: sampleText, px } : a));
    });
    return () => { alive = false; if (pending.current === key) pending.current = null; };
  }, [api, applied, sampleText, px]);

  const { width: cw, height: ch } = cssSize(px);
  const cols = Math.max(1, Math.floor(size.width / (cw + 24)));
  const rowH = ch + 64;
  const total = snap.total ?? 0;
  const rowCount = Math.ceil(total / cols);

  return (
    <div ref={box} role="grid" aria-rowcount={rowCount} aria-colcount={cols} style={{ width: width ?? '100%', height: height ?? '100%' }}>
      {snap.error ? (
        <p role="alert">Lỗi tải thư viện: {snap.error}</p>
      ) : snap.total === 0 ? (
        <p>Không có font nào khớp</p>
      ) : (
        <List
          data-testid="library-grid-scroller"
          rowComponent={Row}
          rowCount={rowCount}
          rowHeight={rowH}
          overscanCount={2}
          defaultHeight={size.height}
          style={{ height: size.height, width: size.width }}
          rowProps={{ cols, total, paged, version: snap.version, applied, api, px, text: sampleText, mode: previewMode, selectedFaceId, blobs, loader, onSelect }}
          onRowsRendered={({ startIndex, stopIndex }) => paged?.ensureRange(startIndex * cols, (stopIndex + 1) * cols - 1)}
          onScroll={e => { velocity.sample((e.currentTarget as HTMLElement).scrollTop, performance.now()); loader.onScroll(); }}
        />
      )}
    </div>
  );
}
