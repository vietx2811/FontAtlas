import { LangBadge, viBadgeFromMissing } from './LangBadge';
import { memo, useEffect, useRef, useState } from 'react';
import './libraryGrid.css';
import '../../components/ui/faceTitle.css';
import type { FontFaceLoader } from './fontFaceLoader';
import { PreviewHttpError, type PreviewBlobs } from './previewBlobs';
import { Tooltip } from '../../components/ui/Tooltip';
import { NoteMark } from '../../components/ui/noteMark';
import { t } from '../../i18n';
import { ownerNote } from '../../components/activation/conflictModel';
import { useUiStore } from '../../store/ui';
import { useCardFields } from '../../store/cardFields';
import { cssSize, previewBox, type FaceRow, type PxBucket } from './types';
import { ownFontStack, useLoadedFamily } from './useLoadedFamily';

export const HOVER_DELAY_MS = 150;
/** Chiều cao thêm của hàng thông tin thứ 2 (giấy phép, Variable) ở dạng lưới. */
export const CARD_ROW2_H = 18;
const LICENSE_KEY: Record<string, 'view.license.ofl' | 'view.license.apache' | 'view.license.other' | 'view.license.unknown'> = { ofl: 'view.license.ofl', apache: 'view.license.apache', other: 'view.license.other', unknown: 'view.license.unknown' };

/** Mã lỗi preview của backend → câu giải thích trên thẻ font cloud (`not_found` là file mất thật, KHÔNG phải công tắc tắt). */
const CLOUD_ERR_KEY: Record<string, 'cloud.errTimeout' | 'cloud.errFailed' | 'cloud.errOff' | 'cloud.errMissing' | 'cloud.errBusy'> = {
  cloud_timeout: 'cloud.errTimeout', cloud_unavailable: 'cloud.errFailed', cloud_disabled: 'cloud.errOff', not_found: 'cloud.errMissing', cloud_busy: 'cloud.errBusy',
};

/** Nhãn định dạng theo đuôi file (TTF, OTF, TTC, WOFF, WOFF2…); không có đường dẫn thì không nhãn. */
export function fileFormat(path: string | null): string | null {
  const m = /\.([a-z0-9]{2,5})$/i.exec(path ?? '');
  return m ? m[1].toUpperCase() : null;
}
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
  /** Phạm vi bật (chỉ có nghĩa khi `active`): tooltip chấm xanh nói "phiên này" hay "Luôn bật". */
  /** `available`: không phải do Font Atlas bật — một bản giống hệt đã dùng được trên máy (chấm rỗng, không có nút nguồn). */
  activeScope?: 'session' | 'persistent' | 'available'; onToggleActive?(row: FaceRow): void;
  onToggleFavorite?(row: FaceRow): void; boxHeight?: number;
  /** Vị trí của ô trong truy vấn hiện tại (để chọn dải bằng index). */
  index?: number;
  /** Bấm chọn kèm phím bổ trợ: ⌘/Ctrl bật tắt, Shift chọn dải. Thiếu → dùng onSelect. */
  onPick?(row: FaceRow, index: number, mods: PickMods): void;
  /** Chuột phải, phím ContextMenu hoặc Shift+F10. `rect` có khi mở bằng bàn phím. */
  onContextMenu?(row: FaceRow, index: number, at: { x: number; y: number; keyboard: boolean; el: HTMLElement }): void;
};
export type PickMods = { meta: boolean; shift: boolean };

function FaceCellImpl({ row, url, px, text, selected, mode, blobs, loader, onSelect, onStale, fontSize, layout = 'grid', active = false, activeScope, onToggleActive, favorite, onToggleFavorite, boxHeight, index = 0, onPick, onContextMenu }: Props): React.JSX.Element {
  const fields = useCardFields(s => s.fields);
  const avail = useUiStore(st => (activeScope === 'available' ? st.available.get(row?.hashHex ?? '') : undefined));
  const { width: cw, height: ch } = fontSize ? previewBox(px, fontSize) : cssSize(px);
  const [state, setState] = useState<Exclude<CellState, 'live'>>('loading');
  const [errCode, setErrCode] = useState('');
  const [shown, setShown] = useState<{ src: string; blob: string } | null>(null);
  const shownRef = useRef<{ src: string; blob: string } | null>(null);
  const [imgShown, setImgShown] = useState<string | null>(null);
  const [live, setLive] = useState<string | null>(null);
  const hovering = useRef(false);
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const acquired = useRef(false);
  // Tên thẻ vẽ bằng chính font của thẻ; cuộn nhanh thì loader chưa nạp (ScrollVelocity), TTC face ≥ 1 giữ font giao diện.
  const nameFamily = useLoadedFamily(loader, row ? { hashHex: row.hashHex, faceIndex: row.faceIndex, fileSize: row.fileSize } : null, true);

  // mask: tải một lần → blob URL; giữ ảnh cũ cho tới khi ảnh mới sẵn sàng.
  useEffect(() => {
    if (mode !== 'mask' || !url || shownRef.current?.src === url) return;
    let alive = true;
    let done = false;
    let raf = 0;
    setState('loading');
    blobs.acquire(url, { cloud: !!row?.onCloudOnly }).then(
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
        setErrCode(err instanceof PreviewHttpError ? err.code : '');
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
  const format = fileFormat(row.path);
  const pick = (e: { metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }) => {
    if (onPick) onPick(row, index, { meta: e.metaKey || e.ctrlKey, shift: e.shiftKey });
    else onSelect(row);
  };
  // Chấm trạng thái đứng ĐẦU tên: xanh = đang bật, xám = chưa bật; có `onToggleActive` thì chính chấm là nút bật/tắt (Space khi focus).
  const dotScope = activeScope === 'persistent' ? 'persistent' : 'session';
  const onTip = t(activeScope === 'persistent' ? 'act.dot.persistent' : 'act.dot.session');
  const stateDot = active ? (
    onToggleActive ? (
      <Tooltip label={`${t('act.toggle')} · ${onTip}`}>
        <button type="button" className="fa-state" data-testid="cell-active" aria-label={t('act.toggle')} aria-pressed
          onClick={e => { e.stopPropagation(); onToggleActive(row); }} onKeyDown={e => e.stopPropagation()}>
          <span className="fa-dot" data-testid="active-dot" data-scope={dotScope} />
        </button>
      </Tooltip>
    ) : <Tooltip label={onTip}><span className="fa-state"><span className="fa-dot" data-testid="active-dot" data-scope={dotScope} /></span></Tooltip>
  ) : avail ? (
    <Tooltip label={t('act.available.dot', { who: ownerNote({ owner: avail.owner, existingPath: avail.path }) })}><span className="fa-state"><span className="fa-dot" data-testid="active-dot" data-scope="available" /></span></Tooltip>
  ) : onToggleActive ? (
    <Tooltip label={`${t('act.toggle')} · ${t('act.dot.off')}`}>
      <button type="button" className="fa-state" data-testid="cell-active" aria-label={t('act.toggle')} aria-pressed={false}
        onClick={e => { e.stopPropagation(); onToggleActive(row); }} onKeyDown={e => e.stopPropagation()}>
        <span className="fa-dot" data-scope="off" />
      </button>
    </Tooltip>
  ) : <span className="fa-state" aria-hidden><span className="fa-dot" data-scope="off" /></span>;
  const hasExtras = fields.includes('license') || fields.includes('variable');
  const extras = <>
    {fields.includes('license') && <span className="fa-fmt" data-testid="face-license">{LICENSE_KEY[row.licenseClass] ? t(LICENSE_KEY[row.licenseClass]) : row.licenseClass.toUpperCase()}</span>}
    {fields.includes('variable') && row.isVariable && <Tooltip label={t('view.var.tip')}><span className="fa-fmt" data-testid="face-variable">VAR</span></Tooltip>}
  </>;
  return (
    <div
      role="gridcell" tabIndex={0} aria-selected={selected} className="fa-cell" data-layout={layout}
      data-face-id={row.faceId} data-state={shownState} data-shown-url={shownUrl} style={{ height: boxHeight }}
      onClick={pick}
      onContextMenu={onContextMenu ? e => { e.preventDefault(); onContextMenu(row, index, { x: e.clientX, y: e.clientY, keyboard: false, el: e.currentTarget }); } : undefined}
      onKeyDown={e => {
        if (e.target !== e.currentTarget) return;
        if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); pick(e); }
        else if (onContextMenu && (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey))) {
          e.preventDefault();
          const r = e.currentTarget.getBoundingClientRect();
          onContextMenu(row, index, { x: r.left + 24, y: r.top + Math.min(r.height - 8, 40), keyboard: true, el: e.currentTarget });
        }
      }}
      onPointerEnter={() => {
        if (!canLive) return;
        hovering.current = true;
        hoverTimer.current = setTimeout(() => { hoverTimer.current = null; startLive(); }, HOVER_DELAY_MS);
      }}
      onPointerLeave={() => { hovering.current = false; if (!selected) stopLive(); }}
    >
      {/* theo trạng thái của ẢNH preview (state), không phải `shownState`: hover mở FontFace sống không được đổi ghi chú */}
      {row.onCloudOnly && (state === 'loading'
        ? <NoteMark kind="info" className="fa-note-cloud" testId="cloud-loading-note" text={t('cloud.loading')} />
        : state === 'error'
          ? <NoteMark kind="warn" className="fa-note-cloud" testId="cloud-error-note" text={t(CLOUD_ERR_KEY[errCode] ?? 'cloud.errFailed')} />
          : <NoteMark kind="info" className="fa-note-cloud" testId="cloud-only-note" text={t('cloud.note')} />)}
      <Tooltip label={favorite ? t('cell.unfavTip') : t('cell.favTip')}>
        <button type="button" className="fa-star" aria-label={t('cell.favTip')} aria-pressed={!!favorite}
          onClick={e => { e.stopPropagation(); onToggleFavorite?.(row); }} onKeyDown={e => e.stopPropagation()}>
          <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden fill={favorite ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"><path d="M12 3.5l2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8L3.5 9.7l5.9-.9z" /></svg>
        </button>
      </Tooltip>
      <div className="fa-spec" data-preview-box data-aligned={layout === 'grid' ? 'middle' : 'start'} style={{ minHeight: ch }}>
        {live ? (
          // Rust vẽ ở px trên canvas 2× → cỡ hiển thị px/2 css px, khớp ảnh preview (plan ghi nhầm px·3/4).
          <span data-live style={{ fontFamily: `"${live}"`, fontSize: fontSize ?? px / 2, lineHeight: `${ch}px`, whiteSpace: 'nowrap' }}>{text}</span>
        ) : state === 'error' ? (
          <span className="fa-spec-error" style={{ lineHeight: `${ch}px` }}>{t('cell.noPreview')}</span>
        ) : mode === 'mask' ? (
          shown
            ? <div data-mask aria-hidden className="fa-preview-mask" style={{ width: cw, height: ch, ['--preview-url' as string]: `url("${shown.blob}")`, ...(layout === 'grid' ? { transform: `translateY(${(ch * 0.14).toFixed(1)}px)` } : {}) }} />
            : row.onCloudOnly && state === 'loading' && <div data-cloud-skeleton aria-hidden className="fa-cloud-skeleton" style={{ width: Math.min(cw, 160), height: Math.max(10, Math.round(ch * 0.4)) }} />
        ) : (
          url && <img className="fa-preview-img" src={url} width={cw} height={ch} alt=""
            onLoad={() => { setImgShown(url); setState('ready'); }} onError={() => setState('error')} />
        )}
      </div>
      <div className="fa-meta">
        {stateDot}
        <Tooltip label={name} onlyIfTruncated>
          <span className={`fa-name${nameFamily ? ' fa-own-font' : ''}`} data-testid="face-name" data-own-font={nameFamily ?? undefined} style={nameFamily ? { fontFamily: ownFontStack(nameFamily) } : undefined}>{name}</span>
        </Tooltip>
        {format && fields.includes('format') && <Tooltip label={t('cell.format', { format })}><span className="fa-fmt" data-testid="face-format">{format}</span></Tooltip>}
        {fields.includes('lang') && <LangBadge badge={row.langBadge ?? viBadgeFromMissing(row.viMissing)} />}
        {layout === 'list' && extras}
      </div>
      {layout === 'grid' && hasExtras && <div className="fa-meta fa-meta2" data-testid="face-meta2">{extras}</div>}
    </div>
  );
}

/** memo: đổi vùng chọn/yêu thích chỉ vẽ lại đúng những thẻ có `selected`/`favorite`/`active` đổi (các callback truyền vào phải ổn định). */
export const FaceCell = memo(FaceCellImpl);
