import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { List, useListRef, type RowComponentProps } from 'react-window';
import type { LibraryApi } from './api';
import { CARD_ROW2_H, DEFAULT_PREVIEW_MODE, FaceCell, type PickMods, type PreviewMode } from './FaceCell';
import { useCardFields } from '../../store/cardFields';
import { FontFaceLoader, ScrollVelocity, browserLoaderDeps } from './fontFaceLoader';
import { usePagedFaces, type PagedFaces } from './pagedFaces';
import { GroupHeader } from './GroupHeader';
import { useFamilyGroups } from './familyGroups';
import { HEADER_H, buildLayout, faceSpansOf, groupsCover, rowOfFace, type FamilyGroup, type LayoutRow } from './groupLayout';
import { PreviewBlobs } from './previewBlobs';
import { DEFAULT_SAMPLE, bucketFor, cellMinWidth, cssSize, previewBox, type FaceRow, type GridQuery, type PxBucket } from './types';
import { t } from '../../i18n';

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
  activeScopeOf?(row: FaceRow): 'session' | 'persistent' | 'available' | undefined; onToggleActive?(row: FaceRow): void;
  /** Tập khoá `${hashHex}:${faceIndex}` đang được chọn (đa chọn). Có → thay cho selectedFaceId/selectedKey khi tô thẻ. */
  /** Bộ nạp dùng chung toàn app kèm ScrollVelocity của nó (lưới đưa tốc độ cuộn vào). Thiếu → lưới tự tạo bộ nạp riêng. */
  velocity?: ScrollVelocity;
  selectedKeys?: ReadonlySet<string>;
  /** Bấm thẻ kèm ⌘/Ctrl/Shift; index là vị trí trong truy vấn. Thiếu → onSelect. */
  onPick?(row: FaceRow, index: number, mods: PickMods): void;
  onContextMenu?(row: FaceRow, index: number, at: { x: number; y: number; keyboard: boolean; el: HTMLElement }): void;
  /** Nhóm theo Family: khoảng chỉ số face của các nhóm đang thu gọn (ẩn khỏi lưới) — để vùng chọn dải / chọn tất cả bỏ qua. */
  onHiddenRanges?(ranges: ReadonlyArray<readonly [number, number]>): void;
  /** Nhóm đang thu gọn (theo tên family) + cách đổi: điều khiển từ ngoài để trạng thái sống qua lần dựng lại; thiếu → lưới tự giữ. */
  collapsed?: ReadonlySet<string>; onToggleGroup?(group: FamilyGroup, collapse: boolean): void;
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
  activeScopeOf?(row: FaceRow): 'session' | 'persistent' | 'available' | undefined; onToggleActive?(row: FaceRow): void;
  selectedKeys?: ReadonlySet<string>; onPick?: LibraryGridProps['onPick']; onContextMenu?: LibraryGridProps['onContextMenu'];
  /** Nhóm theo Family (R3): có → hàng là tiêu đề nhóm hoặc hàng thẻ của một nhóm. */
  gl: readonly LayoutRow[] | null; groups: readonly FamilyGroup[] | null; collapsed: ReadonlySet<string>; onToggleGroup(group: FamilyGroup, collapse: boolean): void;
  headFamily: string | null; onHeadFocus(family: string): void; onHeadNav(groupIndex: number, delta: number): void;
};

function Row({ index, style, rows, cols, total, paged, applied, api, px, text, mode, selectedFaceId, selectedKey, selectedKeys, blobs, loader, onSelect, onPick, onContextMenu, onStale, fontSize, layout, boxHeight, isActive, isFavorite, onToggleFavorite, activeScopeOf, onToggleActive, gl, groups, collapsed, onToggleGroup, headFamily, onHeadFocus, onHeadNav }: RowComponentProps<RowProps>) {
  if (index >= rows) return <div role="presentation" style={style} />; // hàng đệm đáy
  const lr: LayoutRow | null = gl ? gl[index] ?? null : null;
  if (lr && lr.kind === 'head') return <GroupHeader group={groups![lr.g]} rep={paged?.rowAt(groups![lr.g].start)} loader={loader} collapsed={collapsed.has(groups![lr.g].family)} onToggle={onToggleGroup} tabStop={headFamily === groups![lr.g].family} onFocusHead={onHeadFocus} onNavigate={d => onHeadNav(lr.g, d)} style={style} />;
  const cardsRow = lr && lr.kind === 'cards' ? lr : null;
  const base = cardsRow ? cardsRow.first : index * cols; const end = cardsRow ? cardsRow.first + cardsRow.n : total;
  const cells = [];
  for (let c = 0; c < cols; c++) {
    const i = base + c;
    // Ô trống cùng hộp với thẻ (padding + viền) để hàng cuối không kéo giãn thẻ.
    if (i >= end) { cells.push(<div key={c} className="fa-cell" aria-hidden style={{ visibility: "hidden", height: boxHeight }} />); continue; }
    const row = paged?.rowAt(i);
    // URL dùng chữ/cỡ "đã áp dụng" (cùng thế hệ backend) → ảnh cũ giữ nguyên trong lúc xin thế hệ mới.
    const url = row && applied
      ? api.previewUrl(row, { text: applied.text, px: applied.px, generation: applied.generation, rendererVersion: applied.rendererVersion })
      : null;
    cells.push(
      <FaceCell key={row ? `${i}:${row.faceId}` : `p${i}`} row={row} url={url} px={px} text={text} mode={mode}
        selected={!!row && (selectedKeys ? selectedKeys.has(`${row.hashHex}:${row.faceIndex}`) || `${row.hashHex}:${row.faceIndex}` === selectedKey : selectedKey ? `${row.hashHex}:${row.faceIndex}` === selectedKey : row.faceId === selectedFaceId)} blobs={blobs} loader={loader} onSelect={onSelect} onStale={onStale}
        index={i} onPick={onPick} onContextMenu={onContextMenu}
        fontSize={fontSize} layout={layout} boxHeight={boxHeight} active={!!row && !!isActive?.(row)} favorite={row ? (isFavorite ? isFavorite(row) : row.isFavorite) : false} onToggleFavorite={onToggleFavorite} activeScope={row ? activeScopeOf?.(row) : undefined} onToggleActive={onToggleActive} />,
    );
  }
  return <div role="row" style={{ ...style, display: 'flex' }}>{cells}</div>;
}

/**
 * Kích thước khung chứa. Đo ĐỒNG BỘ sau mỗi lần render (trước khi vẽ) bằng clientWidth/clientHeight, nên không phụ thuộc thời điểm
 * ResizeObserver chạy (WebView ở nền/bị che có thể hoãn RO rất lâu, để lại số cột cũ khi panel/thanh chọn đổi bề rộng).
 * ResizeObserver vẫn giữ để bắt các thay đổi không kéo theo render (kéo cửa sổ).
 */
function useSize(ref: React.RefObject<HTMLDivElement | null>, fixed: { width?: number; height?: number }) {
  const [size, setSize] = useState({ width: fixed.width ?? 1280, height: fixed.height ?? 800 });
  const apply = useCallback((w: number, h: number) => {
    const width = fixed.width ?? w; const height = fixed.height ?? h;
    setSize(s => (s.width === width && s.height === height ? s : { width, height }));
  }, [fixed.width, fixed.height]);
  const both = fixed.width !== undefined && fixed.height !== undefined;
  useLayoutEffect(() => {
    const el = ref.current;
    if (both || !el) return;
    if (el.clientWidth > 0 && el.clientHeight > 0) apply(el.clientWidth, el.clientHeight); // 0 = chưa có layout (ẩn/jsdom): giữ số cũ
  });
  useEffect(() => {
    if (both) return;
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => apply(e.contentRect.width, e.contentRect.height));
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref, both, apply]);
  return size;
}

export function LibraryGrid({
  api, query, sampleText = DEFAULT_SAMPLE, px: pxProp = 48, previewMode = DEFAULT_PREVIEW_MODE,
  selectedFaceId = null, selectedKey = null, onSelect = NOOP, loader: loaderProp, velocity: velocityProp, blobs: blobsProp, width, height,
  fontSize, layout = 'grid', bottomInset = 0, emptyState, isActive, isFavorite, onToggleFavorite, activeScopeOf, onToggleActive, selectedKeys, onPick, onContextMenu, onHiddenRanges, collapsed: collapsedProp, onToggleGroup: onToggleGroupProp,
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

  const row2 = useCardFields(s => s.fields.includes('license') || s.fields.includes('variable'));
  let cols: number; let rowH: number; let boxHeight: number;
  if (fontSize) {
    const dispH = previewBox(px, fontSize).height;
    cols = layout === 'list' ? 1 : Math.max(1, Math.floor((size.width - 28) / (cellMinWidth(fontSize) + 8)));
    boxHeight = Math.ceil(layout === 'list' ? Math.max(48, dispH + 20) : Math.max(132, dispH + 78) + (row2 ? CARD_ROW2_H : 0));
    rowH = boxHeight + 8;
  } else {
    const { width: cw, height: ch } = cssSize(px);
    cols = Math.max(1, Math.floor(size.width / (cw + 24)));
    rowH = ch + 64;
    boxHeight = ch + 48 + (layout === 'grid' && row2 ? CARD_ROW2_H : 0);
    rowH = rowH + (layout === 'grid' && row2 ? CARD_ROW2_H : 0);
  }
  const total = snap.total ?? 0;
  // Nhóm theo Family (R3): chỉ dùng khi chỉ mục nhóm phủ đúng toàn bộ kết quả; chưa có/lệch/lỗi → lưới phẳng.
  // Theo truy vấn mà các trang đang thuộc về (snap.query), không theo prop mới nhất: chỉ mục nhóm và các trang luôn cùng một truy vấn.
  const pagedQuery = snap.query;
  const wantGroups = !!pagedQuery?.groupByFamily;
  const fam = useFamilyGroups(api, pagedQuery ?? query, wantGroups);
  const groups = useMemo(
    () => (wantGroups && fam.queryKey === snap.queryKey && groupsCover(fam.groups, snap.total) ? fam.groups : null),
    [wantGroups, fam.groups, fam.queryKey, snap.queryKey, snap.total],
  );
  // Thu gọn: điều khiển từ ngoài (store, sống qua lần dựng lại) khi có `collapsedProp`; không thì giữ trong lưới và mở hết khi truy vấn đổi.
  const [innerCollapsed, setInnerCollapsed] = useState<ReadonlySet<string>>(() => new Set());
  const collapsed = collapsedProp ?? innerCollapsed;
  const queryKey = JSON.stringify(query);
  useEffect(() => { if (!collapsedProp) setInnerCollapsed(new Set()); }, [queryKey, collapsedProp]);
  const toggleGroup = useCallback((g: FamilyGroup, collapse: boolean) => {
    if (onToggleGroupProp) { onToggleGroupProp(g, collapse); return; }
    setInnerCollapsed(prev => { const n = new Set(prev); if (collapse) n.add(g.family); else n.delete(g.family); return n; });
  }, [onToggleGroupProp]);
  useEffect(() => {
    onHiddenRanges?.(groups ? groups.filter(g => collapsed.has(g.family)).map(g => [g.start, g.start + g.count - 1] as const) : []);
  }, [groups, collapsed, onHiddenRanges]);
  const glayout = useMemo(() => (groups ? buildLayout(groups, cols, collapsed) : null), [groups, cols, collapsed]);
  const headRows = useMemo(() => { const out: number[] = []; glayout?.forEach((r, i) => { if (r.kind === 'head') out[r.g] = i; }); return out; }, [glayout]);
  const rowCount = glayout ? glayout.length : Math.ceil(total / cols);
  // Neo cuộn theo FACE đầu tiên đang thấy: bật/tắt nhóm, thu gọn, đổi số cột, đổi cao hàng… đều giữ đúng font đó trong tầm nhìn.
  const listRef = useListRef(null);
  const topFace = useRef(0);
  const anchorQuery = useRef(snap.queryKey);
  useLayoutEffect(() => {
    if (anchorQuery.current !== snap.queryKey) { anchorQuery.current = snap.queryKey; topFace.current = 0; return; } // truy vấn khác: bắt đầu từ đầu
    if (topFace.current <= 0) return;
    const index = glayout && groups ? rowOfFace(glayout, groups, topFace.current) : Math.floor(topFace.current / cols);
    listRef.current?.scrollToRow({ index: Math.min(index, Math.max(0, rowCount - 1)), align: 'start' });
  }, [glayout, groups, cols, rowH, rowCount, snap.queryKey, listRef]);
  // Tiêu đề nhóm: một tab-stop cho cả dải (roving tabindex); ↑/↓ chuyển giữa các tiêu đề (cuộn tới nếu chưa dựng).
  const [activeHead, setActiveHead] = useState<string | null>(null);
  const headFamily = groups && groups.some(g => g.family === activeHead) ? activeHead : groups?.[0]?.family ?? null;
  const navHead = useCallback((gi: number, delta: number) => {
    if (!groups) return;
    const t = gi + delta; if (t < 0 || t >= groups.length) return;
    setActiveHead(groups[t].family);
    listRef.current?.scrollToRow({ index: headRows[t], align: 'smart' });
    let tries = 0;
    const focus = () => { const el = box.current?.querySelector<HTMLElement>(`[data-testid="group-head"][data-family="${CSS.escape(groups[t].family)}"]`); if (el) el.focus(); else if (tries++ < 6) requestAnimationFrame(focus); };
    requestAnimationFrame(focus);
  }, [groups, headRows, listRef]);

  return (
    <div ref={box} role="grid" data-layout={layout} data-grouped={glayout ? 'true' : undefined} aria-rowcount={rowCount} aria-colcount={cols} style={{ width: width ?? '100%', height: height ?? '100%' }}>
      {previewError && (
        <div role="alert">
          {t('grid.previewFail', { msg: previewError })}{' '}
          <button type="button" onClick={() => { setPreviewError(null); setInfoTry(n => n + 1); }}>{t('grid.retry')}</button>
        </div>
      )}
      {snap.error && snap.total !== null && (
        // Lỗi ở một trang khi đã có dữ liệu: giữ lưới, cho thử lại.
        <div role="alert">{t('grid.loadFail', { msg: snap.error })} <button type="button" onClick={() => paged?.retry()}>{t('grid.retry')}</button></div>
      )}
      {snap.error && snap.total === null ? (
        <div role="alert">{t('grid.loadFail', { msg: snap.error })} <button type="button" onClick={() => paged?.retry()}>{t('grid.retry')}</button></div>
      ) : snap.total === 0 ? (
        emptyState ?? <p>{t('grid.noMatch')}</p>
      ) : (
        <List
          data-testid="library-grid-scroller"
          listRef={listRef}
          rowComponent={Row}
          rowCount={rowCount + (bottomInset ? 1 : 0)}
          rowHeight={(i: number) => (i >= rowCount ? bottomInset : glayout && glayout[i].kind === 'head' ? HEADER_H : rowH)}
          overscanCount={2}
          defaultHeight={size.height}
          style={{ height: '100%', width: '100%' }}
          rowProps={{ headFamily, onHeadFocus: setActiveHead, onHeadNav: navHead, cols, total, paged, version: snap.version, applied, api, px, text: sampleText, mode: previewMode, selectedFaceId, selectedKey, selectedKeys, blobs, loader, onSelect, onPick, onContextMenu, onStale, fontSize, layout, boxHeight, rows: rowCount, isActive, isFavorite, onToggleFavorite, activeScopeOf, onToggleActive, gl: glayout, groups, collapsed, onToggleGroup: toggleGroup }}
          onRowsRendered={({ startIndex, stopIndex }) => {
            if (glayout && groups) { const r0 = glayout[startIndex]; if (r0) topFace.current = r0.kind === 'head' ? groups[r0.g].start : r0.first; } else topFace.current = startIndex * cols;
            if (glayout && groups) faceSpansOf(glayout, groups, startIndex, stopIndex).forEach(([a, b]) => paged?.ensureRange(a, b));
            else paged?.ensureRange(startIndex * cols, (stopIndex + 1) * cols - 1);
          }}
          onScroll={e => { velocity.sample((e.currentTarget as HTMLElement).scrollTop, performance.now()); loader?.onScroll(); }}
        />
      )}
    </div>
  );
}
