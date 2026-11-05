import { useEffect, useRef, useState } from 'react';
import './libraryGrid.css';
import type { FontFaceLoader } from './fontFaceLoader';
import { PreviewHttpError, type PreviewBlobs } from './previewBlobs';
import { cssSize, previewBox, type FaceRow, type PxBucket } from './types';

export const HOVER_DELAY_MS = 150;
export type PreviewMode = 'mask' | 'img';
export const DEFAULT_PREVIEW_MODE: PreviewMode = 'mask';
export type CellState = 'loading' | 'ready' | 'error' | 'live';

type Props = {
  row: FaceRow | undefined; url: string | null; px: PxBucket; text: string; selected: boolean;
  mode: PreviewMode; blobs: PreviewBlobs; loader: FontFaceLoader | null; onSelect(row: FaceRow): void;
  /** Gặp 410: thế hệ backend đã đi tiếp → grid lấy lại thế hệ hiện tại. */
  onStale?(): void;
  /** Cỡ chữ mẫu (css px). Có → ảnh bucket co về cỡ này; không → cỡ gốc của bucket (harness cũ). */
  fontSize?: number; layout?: 'grid' | 'list'; active?: boolean; favorite?: boolean;
  onToggleFavorite?(row: FaceRow): void; boxHeight?: number;
};

export function FaceCell({ row, url, px, text, selected, mode, blobs, loader, onSelect, onStale, fontSize, layout = 'grid', active = false, favorite, onToggleFavorite, boxHeight }: Props): React.JSX.Element {
  const { width: cw, height: ch } = fontSize ? previewBox(px, fontSize) : cssSize(px);
  const [state, setState] = useState<Exclude<CellState, 'live'>>('loading');
  const [shown, setShown] = useState<{ src: string; blob: string } | null>(null);
  const shownRef = useRef<{ src: string; blob: string } | null>(null);
  const [imgShown, setImgShown] = useState<string | null>(null);
  const [live, setLive] = useState<string | null>(null);
  const hovering = useRef(false);
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const acquired = useRef(false);

  // mask: tải một lần → blob URL; giữ ảnh cũ cho tới khi ảnh mới sẵn sàng.
  useEffect(() => {
    if (mode !== 'mask' || !url || shownRef.current?.src === url) return;
    let alive = true;
    let done = false;
    let raf = 0;
    setState('loading');
    blobs.acquire(url).then(
      blob => {
        done = true;
        if (!alive) { blobs.release(url); return; }
        const prev = shownRef.current;
        shownRef.current = { src: url, blob };
        setShown(shownRef.current);
        if (prev) blobs.release(prev.src);
        raf = requestAnimationFrame(() => { if (alive) setState('ready'); });
      },
      err => {
        done = true;
        if (!alive) return;
        if (err instanceof PreviewHttpError && err.status === 410) { onStale?.(); return; } // xin thế hệ mới, URL mới sẽ tới
        setState('error');
      },
    );
    return () => {
      alive = false;
      cancelAnimationFrame(raf);
      if (!done) blobs.release(url);
    };
  }, [url, mode, blobs]);

  // img: URL đổi → chờ onLoad lại (trình duyệt giữ ảnh cũ trong lúc tải).
  useEffect(() => { if (mode === 'img' && url && url !== imgShown) setState('loading'); }, [url, mode, imgShown]);

  // Trả blob và font khi ô bị gỡ.
  useEffect(() => () => {
    if (shownRef.current) { blobs.release(shownRef.current.src); shownRef.current = null; }
  }, [blobs]);

  const canLive = !!row && row.faceIndex === 0 && !!loader;
  const startLive = () => {
    if (!row || !loader || acquired.current) return;
    acquired.current = true;
    loader.acquire(row).then(fam => { if (fam && (hovering.current || selected)) setLive(fam); });
  };
  const stopLive = () => {
    if (hoverTimer.current) { clearTimeout(hoverTimer.current); hoverTimer.current = null; }
    if (acquired.current && row && loader) { loader.release(row); acquired.current = false; }
    setLive(null);
  };
  useEffect(() => {
    if (selected && canLive) { startLive(); return () => { if (!hovering.current) stopLive(); }; }
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, canLive, row?.hashHex]);
  useEffect(() => () => stopLive(), []); // eslint-disable-line react-hooks/exhaustive-deps

  if (!row) {
    return <div role="gridcell" data-state="loading" data-layout={layout} className="fa-cell fa-cell-skeleton" style={{ height: boxHeight }} />;
  }

  const shownState: CellState = live ? 'live' : state;
  const shownUrl = mode === 'mask' ? shown?.src ?? '' : imgShown ?? '';
  const name = `${row.family}${row.subfamily ? ` ${row.subfamily}` : ''}`;
  const missing = row.viMissing > 0;
  return (
    <div
      role="gridcell" tabIndex={0} aria-selected={selected} className="fa-cell" data-layout={layout}
      data-face-id={row.faceId} data-state={shownState} data-shown-url={shownUrl} style={{ height: boxHeight }}
      onClick={() => onSelect(row)}
      onKeyDown={e => { if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); onSelect(row); } }}
      onPointerEnter={() => {
        if (!canLive) return;
        hovering.current = true;
        hoverTimer.current = setTimeout(() => { hoverTimer.current = null; startLive(); }, HOVER_DELAY_MS);
      }}
      onPointerLeave={() => { hovering.current = false; if (!selected) stopLive(); }}
    >
      {active && <span className="fa-dot" data-testid="active-dot" title="Đang bật" />}
      <button type="button" className="fa-star" aria-label="Yêu thích" aria-pressed={!!favorite} title="Yêu thích"
        onClick={e => { e.stopPropagation(); onToggleFavorite?.(row); }}>
        <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden fill={favorite ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"><path d="M12 3.5l2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8L3.5 9.7l5.9-.9z" /></svg>
      </button>
      <div className="fa-spec" data-preview-box style={{ height: ch }}>
        {live ? (
          // Rust vẽ ở px trên canvas 2× → cỡ hiển thị px/2 css px, khớp ảnh preview (plan ghi nhầm px·3/4).
          <span data-live style={{ fontFamily: `"${live}"`, fontSize: fontSize ?? px / 2, lineHeight: `${ch}px`, whiteSpace: 'nowrap' }}>{text}</span>
        ) : state === 'error' ? (
          <span className="fa-spec-error" style={{ lineHeight: `${ch}px` }}>Không có preview</span>
        ) : mode === 'mask' ? (
          shown && <div data-mask aria-hidden className="fa-preview-mask" style={{ width: cw, height: ch, ['--preview-url' as string]: `url("${shown.blob}")` }} />
        ) : (
          url && <img className="fa-preview-img" src={url} width={cw} height={ch} alt=""
            onLoad={() => { setImgShown(url); setState('ready'); }} onError={() => setState('error')} />
        )}
      </div>
      <div className="fa-meta">
        <span className="fa-name" data-testid="face-name">{name}</span>
        <span className={`fa-vi ${missing ? 'miss' : 'ok'}`} data-testid="vi-mark" data-warn={missing ? 'true' : undefined}
          title={missing ? `Thiếu ${row.viMissing} ký tự tiếng Việt` : 'Đủ dấu tiếng Việt'}>{missing ? `−${row.viMissing}` : '●'}</span>
      </div>
    </div>
  );
}
