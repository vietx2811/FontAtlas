import { useEffect, useId, useRef, useState } from 'react';
import { t } from '../../i18n';

/**
 * Tag nổi gắn vào nút Compare khi chọn nhiều hơn mức nên so ("12 · nên ≤ 4" + dấu "!" theo motif NoteMark).
 * Không chiếm hàng nào trong layout (định vị tuyệt đối). Hover hoặc focus mở popover có câu giải thích và nút "Chỉ lấy 4 đầu";
 * popover ở lại khi con trỏ chuyển từ tag vào popover (trễ ngắn, chỉ cho con trỏ), mất focus ra ngoài thì đóng ngay, Esc đóng.
 * Bấm "Chỉ lấy 4 đầu" thì tag biến mất: tiêu điểm chuyển sang nút Compare cùng cụm.
 */
export function NarrowTag({ n, max, onNarrow }: { n: number; max: number; onNarrow(): void }): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const root = useRef<HTMLSpanElement>(null);
  const id = useId();
  const show = () => { clearTimeout(timer.current); setOpen(true); };
  // Con trỏ rời đi: đóng sau một nhịp ngắn, trừ khi tiêu điểm bàn phím còn trong tag/popover.
  const hide = () => { clearTimeout(timer.current); timer.current = setTimeout(() => { if (!root.current?.contains(document.activeElement)) setOpen(false); }, 120); };
  useEffect(() => () => clearTimeout(timer.current), []);
  useEffect(() => {
    if (!open) return;
    const esc = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || document.querySelector('[role="dialog"][aria-modal="true"]')) return; // đang có hộp thoại: Esc của nó
      e.stopPropagation(); setOpen(false);
    };
    document.addEventListener('keydown', esc, true);
    return () => document.removeEventListener('keydown', esc, true);
  }, [open]);
  const narrow = () => {
    const wrap = root.current?.parentElement;
    onNarrow(); setOpen(false);
    setTimeout(() => wrap?.querySelector<HTMLElement>('button')?.focus(), 0);
  };
  return (
    <span ref={root} className="fa-narrow" onPointerEnter={show} onPointerLeave={hide}
      onFocus={show} onBlur={e => { if (!root.current?.contains(e.relatedTarget as Node | null)) { clearTimeout(timer.current); setOpen(false); } }}>
      <span className="fa-narrow-tag" data-testid="narrow-tag" role="note" tabIndex={0} aria-describedby={`${id}-d`} aria-label={t('topbar.narrow', { n, max })}>
        <b aria-hidden>!</b>{n} · {t('topbar.narrow.short', { max })}
      </span>
      <span id={`${id}-d`} hidden>{t('topbar.narrow', { n, max })}</span>
      {open && (
        <div className="fa-narrow-pop" role="group" aria-label={t('topbar.narrow.title')} data-testid="narrow-pop">
          <p>{t('topbar.narrow', { n, max })}</p>
          <button type="button" onClick={narrow}>{t('topbar.useFirst', { max })}</button>
        </div>
      )}
    </span>
  );
}
