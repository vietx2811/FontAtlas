import { useEffect, useId, useRef, useState } from 'react';
import { t } from '../../i18n';

/**
 * Tag nổi gắn vào nút Compare khi chọn nhiều hơn mức nên so ("12 · nên ≤ 4" + dấu "!" theo motif NoteMark).
 * Không chiếm hàng nào trong layout (định vị tuyệt đối). Hover hoặc focus mở popover có câu giải thích và nút "Chỉ lấy 4 đầu";
 * popover ở lại khi con trỏ chuyển từ tag vào popover (trễ ngắn), Esc đóng.
 */
export function NarrowTag({ n, max, onNarrow }: { n: number; max: number; onNarrow(): void }): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const root = useRef<HTMLSpanElement>(null);
  const id = useId();
  const show = () => { clearTimeout(timer.current); setOpen(true); };
  const hide = () => { clearTimeout(timer.current); timer.current = setTimeout(() => setOpen(false), 120); };
  useEffect(() => () => clearTimeout(timer.current), []);
  useEffect(() => {
    if (!open) return;
    const esc = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopPropagation(); setOpen(false); } };
    document.addEventListener('keydown', esc, true);
    return () => document.removeEventListener('keydown', esc, true);
  }, [open]);
  return (
    <span ref={root} className="fa-narrow" onPointerEnter={show} onPointerLeave={hide}
      onFocus={show} onBlur={e => { if (!root.current?.contains(e.relatedTarget as Node)) hide(); }}>
      <span className="fa-narrow-tag" data-testid="narrow-tag" role="note" tabIndex={0} aria-describedby={open ? id : undefined} aria-label={t('topbar.narrow', { n, max })}>
        <b aria-hidden>!</b>{n} · {t('topbar.narrow.short', { max })}
      </span>
      {open && (
        <div className="fa-narrow-pop" id={id} role="group" aria-label={t('topbar.narrow.title')} data-testid="narrow-pop">
          <p>{t('topbar.narrow', { n, max })}</p>
          <button type="button" onClick={() => { onNarrow(); setOpen(false); }}>{t('topbar.useFirst', { max })}</button>
        </div>
      )}
    </span>
  );
}
