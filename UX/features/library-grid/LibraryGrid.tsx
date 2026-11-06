import { useCallback, useEffect, useRef, useState } from 'react';
import { List, type RowComponentProps } from 'react-window';
import type { LibraryApi } from './api';
import { DEFAULT_PREVIEW_MODE, FaceCell, type PickMods, type PreviewMode } from './FaceCell';
import { FontFaceLoader, ScrollVelocity, browserLoaderDeps } from './fontFaceLoader';
import { usePagedFaces, type PagedFaces } from './pagedFaces';
import { PreviewBlobs } from './previewBlobs';
import { DEFAULT_SAMPLE, bucketFor, cellMinWidth, cssSize, previewBox, type FaceRow, type GridQuery, type PxBucket } from './types';

export { HOVER_DELAY_MS, DEFAULT_PREVIEW_MODE, type CellState, type PickMods, type PreviewMode } from './FaceCell';

export type LibraryGridProps = {
  api: LibraryApi; query: GridQuery; sampleText?: string; px?: PxBucket; previewMode?: PreviewMode;
  selectedFaceId?: number | null; selectedKey?: string | null; onSelect?(row: FaceRow): void; loader?: FontFaceLoader; blobs?: PreviewBlobs;
  width?: number; height?: number;
  /** Cỡ chữ mẫu 18–72: quyết định bucket ảnh, bề rộng ô và hộp ảnh (Plan 6b T6). */
  fontSize?: number; layout?: 'grid' | 'list';
  /** Hàng đệm ở đáy (px) cho ô nổi che thẻ cuối. */
  bottomInset?: number;
  isActive?(row: FaceRow): boolean; isFavorite?(row: FaceRow): boolean; onToggleFavorite?(row: FaceRow): void;
  /** Tập khoá `${hashHex}:${faceIndex}` đang được chọn (đa chọn). Có → thay cho selectedFaceId/selectedKey khi tô thẻ. */
  /** Bộ nạp dùng chung toàn app kèm ScrollVelocity của nó (lưới đưa tốc độ cuộn vào). Thiếu → lưới tự tạo bộ nạp riêng. */
  velocity?: ScrollVelocity;
  selectedKeys?: ReadonlySet<string>;
  /** Bấm thẻ kèm ⌘/Ctrl/Shift; index là vị trí trong truy vấn. Thiếu → onSelect. */
  onPick?(row: FaceRow, index: number, mods: PickMods): void;
  onContextMenu?(row: FaceRow, index: number, at: { x: number; y: number; keyboard: boolean; el: HTMLElement }): void;
  /** Thay cho dòng "Không có font nào khớp" khi kết quả rỗng. */
  emptyState?: React.ReactNode;
};

const NOOP = () => {};
type Applied = { rendererVersion: string; generation: number; text: string; px: PxBucket };

type RowProps = {
  cols: number; total: number; paged: PagedFaces | null; version: number; applied: Applied | null;
  api: LibraryApi; px: PxBucket; text: string; mode: PreviewMode; selectedFaceId: number | null; selectedKey: string | null;
  blobs: PreviewBlobs; loader: FontFaceLoader | null; onSelect(row: FaceRow): void; onStale(): void;
  fontSize?: number; layout: 'grid' | 'list'; boxHeight: number; rows: number;
  isActive?(row: FaceRow): boolean; isFavorite?(row: FaceRow): boolean; onToggleFavorite?(row: FaceRow): void;
  selectedKeys?: ReadonlySet<string>; onPick?: LibraryGridProps['onPick']; onContextMenu?: LibraryGridProps['onContextMenu'];
};

function Row({ index, style, rows, cols, total, paged, applied, api, px, text, mode, selectedFaceId, selectedKey, selectedKeys, blobs, loader, onSelect, onPick, onContextMenu, onStale, fontSize, layout, boxHeight, isActive, isFavorite, onToggleFavorite }: RowComponentProps<RowProps>) {
  if (index >= rows) return <div role="presentation" style={style} />; // hàng đệm đáy
  const cells = [];
  for (let c = 0; c < cols; c++) {
    const i = index * cols + c;
    // Ô trống cùng hộp với thẻ (padding + viền) để hàng cuối không kéo giãn thẻ.
    if (i >= total) { cells.push(<div key={c} className="fa-cell" aria-hidden style={{ visibility: "hidden", height: boxHeight }} />); continue; }
    const row = paged?.rowAt(i);
    // URL dùng chữ/cỡ "đã áp dụng" (cùng thế hệ backend) → ảnh cũ giữ nguyên trong lúc xin thế hệ mới.
    const url = row && applied
      ? api.previewUrl(row, { text: applied.text, px: applied.px, generation: applied.generation, rendererVersion: applied.rendererVersion })
      : null;
    cells.push(
      <FaceCell key={row ? `${i}:${row.faceId}` : `p${i}`} row={row} url={url} px={px} text={text} mode={mode}
        selected={!!row && (selectedKeys ? selectedKeys.has(`${row.hashHex}:${row.faceIndex}`) || `${row.hashHex}:${row.faceIndex}` === selectedKey : selectedKey ? `${row.hashHex}:${row.faceIndex}` === selectedKey : row.faceId === selectedFaceId)} blobs={blobs} loader={loader} onSelect={onSelect} onStale={onStale}
        index={i} onPick={onPick} onContextMenu={onContextMenu}
        fontSize={fontSize} layout={layout} boxHeight={boxHeight} active={!!row && !!isActive?.(row)} favorite={row ? (isFavorite ? isFavorite(row) : row.isFavorite) : false} onToggleFavorite={onToggleFavorite} />,
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
  api, query, sampleText = DEFAULT_SAMPLE, px: pxProp = 48, previewMode = DEFAULT_PREVIEW_MODE,
  selectedFaceId = null, selectedKey = null, onSelect = NOOP, loader: loaderProp, velocity: velocityProp, blobs: blobsProp, width, height,
  fontSize, layout = 'grid', bottomInset = 0, emptyState, isActive, isFavorite, onToggleFavorite, selectedKeys, onPick, onContextMenu,
}: LibraryGridProps): React.JSX.Element {
  const px: PxBucket = fontSize ? bucketFor(fontSize) : pxProp;
  const box = useRef<HTMLDivElement>(null);
  const size = useSize(box, { width, height });
  const { paged, snap } = usePagedFaces(api, query);
  // velocity/blobs không giữ tài nguyên cần huỷ → khởi tạo lười an toàn với StrictMode.
  const [ownVelocity] = useState(() => new ScrollVelocity());
  const velocity = velocityProp ?? ownVelocity;
  const [ownBlobs] = useState(() => (blobsProp ? null : new PreviewBlobs()));
  const blobs = blobsProp ?? ownBlobs!;
  // Loader tự tạo giữ FontFace trong document.fonts → tạo và huỷ trong cùng một effect.
  const [ownLoader, setOwnLoader] = useState<FontFaceLoader | null>(null);
  useEffect(() => {
    if (loaderProp) return;
    const l = new FontFaceLoader(browserLoaderDeps(api, velocity));
    setOwnLoader(l);
    return () => { l.dispose(); setOwnLoader(null); };
  }, [api, velocity, loaderProp]);
  const loader = loaderProp ?? ownLoader;

  // Thế hệ preview do backend sở hữu: lấy giá trị đầu qua previewInfo, xin mới khi chữ/cỡ đổi.
  const [applied, setApplied] = useState<Applied | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const want = useRef({ text: sampleText, px });
  want.current = { text: sampleText, px };
  const [infoTry, setInfoTry] = useState(0);
  useEffect(() => {
    let alive = true;
    api.previewInfo().then(
      info => {
        if (!alive) return;
        setPreviewError(null);
        setApplied({ rendererVersion: info.rendererVersion, generation: info.generation, ...want.current });
      },
      err => { if (alive) setPreviewError(err?.message ?? String(err)); },
    );
    return () => { alive = false; };
  }, [api, infoTry]);
  // Ô gặp 410: lấy lại thế hệ hiện tại của backend (gộp nhiều ô thành một lần gọi).
  const refreshing = useRef(false);
  const onStale = useCallback(() => {
    if (refreshing.current) return;
    refreshing.current = true;
    api.previewInfo().then(
      info => setApplied(a => (a && info.generation > a.generation ? { ...a, generation: info.generation, rendererVersion: info.rendererVersion } : a)),
      err => setPreviewError(err?.message ?? String(err)),
    ).finally(() => { refreshing.current = false; });
  }, [api]);
  const pending = useRef<string | null>(null);
  useEffect(() => {
    if (!applied || (applied.text === sampleText && applied.px === px)) return;
    const key = `${px}\0${sampleText}`;
    if (pending.current === key) return;
    pending.current = key;
    let alive = true;
    api.bumpGeneration().then(
      g => {
        if (pending.current === key) pending.current = null;
        if (alive) setApplied(a => (a ? { ...a, generation: g, text: sampleText, px } : a));
      },
      err => {
        if (pending.current === key) pending.current = null;
        if (alive) setPreviewError(err?.message ?? String(err));
      },
    );
    return () => { alive = false; if (pending.current === key) pending.current = null; };
  }, [api, applied, sampleText, px]);

  let cols: number; let rowH: number; let boxHeight: number;
  if (fontSize) {
    const dispH = previewBox(px, fontSize).height;
    cols = layout === 'list' ? 1 : Math.max(1, Math.floor((size.width - 28) / (cellMinWidth(fontSize) + 8)));
    boxHeight = Math.ceil(layout === 'list' ? Math.max(48, dispH + 20) : Math.max(132, dispH + 78));
    rowH = boxHeight + 8;
  } else {
    const { width: cw, height: ch } = cssSize(px);
    cols = Math.max(1, Math.floor(size.width / (cw + 24)));
    rowH = ch + 64;
    boxHeight = ch + 48;
  }
  const total = snap.total ?? 0;
  const rowCount = Math.ceil(total / cols);

  return (
    <div ref={box} role="grid" data-layout={layout} aria-rowcount={rowCount} aria-colcount={cols} style={{ width: width ?? '100%', height: height ?? '100%' }}>
      {previewError && (
        <div role="alert">
          Không tải được preview: {previewError}{' '}
          <button type="button" onClick={() => { setPreviewError(null); setInfoTry(n => n + 1); }}>Thử lại</button>
        </div>
      )}
      {snap.error && snap.total !== null && (
        // Lỗi ở một trang khi đã có dữ liệu: giữ lưới, cho thử lại.
        <div role="alert">Lỗi tải thư viện: {snap.error} <button type="button" onClick={() => paged?.retry()}>Thử lại</button></div>
      )}
      {snap.error && snap.total === null ? (
        <div role="alert">Lỗi tải thư viện: {snap.error} <button type="button" onClick={() => paged?.retry()}>Thử lại</button></div>
      ) : snap.total === 0 ? (
        emptyState ?? <p>Không có font nào khớp</p>
      ) : (
        <List
          data-testid="library-grid-scroller"
          rowComponent={Row}
          rowCount={rowCount + (bottomInset ? 1 : 0)}
          rowHeight={(i: number) => (i >= rowCount ? bottomInset : rowH)}
          overscanCount={2}
          defaultHeight={size.height}
          style={{ height: size.height, width: size.width }}
          rowProps={{ cols, total, paged, version: snap.version, applied, api, px, text: sampleText, mode: previewMode, selectedFaceId, selectedKey, selectedKeys, blobs, loader, onSelect, onPick, onContextMenu, onStale, fontSize, layout, boxHeight, rows: rowCount, isActive, isFavorite, onToggleFavorite }}
          onRowsRendered={({ startIndex, stopIndex }) => paged?.ensureRange(startIndex * cols, (stopIndex + 1) * cols - 1)}
          onScroll={e => { velocity.sample((e.currentTarget as HTMLElement).scrollTop, performance.now()); loader?.onScroll(); }}
        />
      )}
    </div>
  );
}
