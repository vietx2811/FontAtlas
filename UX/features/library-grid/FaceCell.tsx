import { useEffect, useRef, useState } from 'react';
import './libraryGrid.css';
import type { FontFaceLoader } from './fontFaceLoader';
import { PreviewHttpError, type PreviewBlobs } from './previewBlobs';
import { cssSize, type FaceRow, type PxBucket } from './types';

export const HOVER_DELAY_MS = 150;
export type PreviewMode = 'mask' | 'img';
export const DEFAULT_PREVIEW_MODE: PreviewMode = 'mask';
export type CellState = 'loading' | 'ready' | 'error' | 'live';

type Props = {
  row: FaceRow | undefined; url: string | null; px: PxBucket; text: string; selected: boolean;
  mode: PreviewMode; blobs: PreviewBlobs; loader: FontFaceLoader | null; onSelect(row: FaceRow): void;
};

const NON_ASCII = /[^\x00-\x7F]/;

export function FaceCell({ row, url, px, text, selected, mode, blobs, loader, onSelect }: Props): React.JSX.Element {
  const { width: cw, height: ch } = cssSize(px);
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
        if (err instanceof PreviewHttpError && err.status === 410) return; // URL mới sắp tới
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
    return <div role="gridcell" data-state="loading" style={{ flex: 1, margin: 4, height: ch + 48, background: 'var(--muted-soft)', borderRadius: 8 }} />;
  }

  const shownState: CellState = live ? 'live' : state;
  const shownUrl = mode === 'mask' ? shown?.src ?? '' : imgShown ?? '';
  return (
    <div
      role="gridcell" tabIndex={0} aria-selected={selected}
      data-face-id={row.faceId} data-state={shownState} data-shown-url={shownUrl}
      onClick={() => onSelect(row)}
      onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(row); } }}
      onPointerEnter={() => {
        if (!canLive) return;
        hovering.current = true;
        hoverTimer.current = setTimeout(() => { hoverTimer.current = null; startLive(); }, HOVER_DELAY_MS);
      }}
      onPointerLeave={() => { hovering.current = false; if (!selected) stopLive(); }}
      style={{ flex: 1, minWidth: 0, margin: 4, padding: '8px 12px', background: 'var(--card)', border: '1px solid var(--border)', borderRadius: 8, overflow: 'hidden', boxSizing: 'border-box' }}
    >
      <div style={{ fontSize: 11, color: 'var(--muted)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {row.family}{row.subfamily ? ` ${row.subfamily}` : ''}
      </div>
      <div style={{ width: cw, height: ch, maxWidth: '100%' }}>
        {live ? (
          // Rust vẽ ở px trên canvas 2× → cỡ hiển thị px/2 css px, khớp ảnh preview (plan ghi nhầm px·3/4).
          <span data-live style={{ fontFamily: `"${live}"`, fontSize: px / 2, lineHeight: `${ch}px`, whiteSpace: 'nowrap' }}>{text}</span>
        ) : state === 'error' ? (
          <span style={{ fontSize: 12, color: 'var(--muted)', lineHeight: `${ch}px` }}>Không có preview</span>
        ) : mode === 'mask' ? (
          shown && <div data-mask aria-hidden className="fa-preview-mask" style={{ width: cw, height: ch, ['--preview-url' as string]: `url("${shown.blob}")` }} />
        ) : (
          url && <img className="fa-preview-img" src={url} width={cw} height={ch} alt=""
            onLoad={() => { setImgShown(url); setState('ready'); }} onError={() => setState('error')} />
        )}
      </div>
      {NON_ASCII.test(text) && !row.supportsVi && row.viMissing > 0 && (
        <div style={{ fontSize: 11, color: 'var(--accent)' }}>Thiếu {row.viMissing} ký tự tiếng Việt</div>
      )}
    </div>
  );
}
