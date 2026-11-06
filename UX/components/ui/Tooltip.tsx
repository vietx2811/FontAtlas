import { cloneElement, useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import type { ReactElement, ReactNode, Ref } from 'react';
import { createPortal } from 'react-dom';
import './tooltip.css';

export const TOOLTIP_DELAY_MS = 450;
const GAP = 8;
const EDGE = 8;

type Props = {
  label: ReactNode;
  /** Phím tắt hiện cạnh chữ, ví dụ "⌘,". */
  shortcut?: string;
  /** Một phần tử DOM duy nhất (nút, span…) nhận được ref và các sự kiện chuột/bàn phím. */
  children: ReactElement;
  disabled?: boolean;
  /** Chỉ hiện khi nội dung của phần tử thật sự bị cắt (ellipsis). */
  onlyIfTruncated?: boolean;
  delay?: number;
};

type Handlers = {
  onPointerEnter?(e: React.PointerEvent): void; onPointerLeave?(e: React.PointerEvent): void; onPointerDown?(e: React.PointerEvent): void;
  onFocus?(e: React.FocusEvent): void; onBlur?(e: React.FocusEvent): void; onKeyDown?(e: React.KeyboardEvent): void; ref?: Ref<HTMLElement>;
  'aria-describedby'?: string;
};

let closeCurrent: (() => void) | null = null;

export const isTruncated = (el: Element | null): boolean => !!el && el.scrollWidth > el.clientWidth + 1;

const focusVisible = (el: Element): boolean => {
  try { return el.matches(':focus-visible'); } catch { return true; }
};

/** Chú thích nổi cho nút icon, huy hiệu và tên bị cắt. Hover ~450 ms, focus bàn phím hiện ngay, Esc ẩn. */
export function Tooltip({ label, shortcut, children, disabled, onlyIfTruncated, delay = TOOLTIP_DELAY_MS }: Props): React.JSX.Element {
  const id = useId();
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLElement | null>(null);
  const tip = useRef<HTMLDivElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const hide = useCallback(() => { clearTimeout(timer.current); timer.current = undefined; setOpen(false); }, []);
  const reveal = useCallback(() => {
    if (disabled || label === null || label === undefined || label === '' || label === false) return;
    if (onlyIfTruncated && !isTruncated(trigger.current)) return;
    closeCurrent?.();
    closeCurrent = hide;
    setOpen(true);
  }, [disabled, label, onlyIfTruncated, hide]);
  const schedule = useCallback(() => { clearTimeout(timer.current); timer.current = setTimeout(reveal, delay); }, [reveal, delay]);

  useEffect(() => () => { clearTimeout(timer.current); if (closeCurrent === hide) closeCurrent = null; }, [hide]);
  useEffect(() => { if (disabled && open) hide(); }, [disabled, open, hide]);

  // Đặt vị trí sau khi đo: lật xuống dưới nếu thiếu chỗ phía trên, kẹp ngang trong cửa sổ.
  useLayoutEffect(() => {
    const el = tip.current; const t = trigger.current;
    if (!open || !el || !t) return;
    const r = t.getBoundingClientRect();
    const w = el.offsetWidth; const h = el.offsetHeight;
    const above = r.top - h - GAP >= EDGE;
    const top = above ? r.top - h - GAP : r.bottom + GAP;
    const left = Math.min(Math.max(EDGE, r.left + r.width / 2 - w / 2), Math.max(EDGE, window.innerWidth - w - EDGE));
    el.style.left = `${Math.round(left)}px`;
    el.style.top = `${Math.round(top)}px`;
    el.dataset.placement = above ? 'top' : 'bottom';
    el.style.visibility = 'visible';
  }, [open]);

  // Cuộn hoặc đổi cỡ cửa sổ: vị trí cũ không còn đúng.
  useEffect(() => {
    if (!open) return;
    window.addEventListener('scroll', hide, true);
    window.addEventListener('resize', hide);
    return () => { window.removeEventListener('scroll', hide, true); window.removeEventListener('resize', hide); };
  }, [open, hide]);

  const child = children as ReactElement<Handlers>;
  const own = child.props;
  const setRef = (node: HTMLElement | null) => {
    trigger.current = node;
    const r = own.ref;
    if (typeof r === 'function') r(node);
    else if (r && typeof r === 'object') (r as { current: HTMLElement | null }).current = node;
  };
  const cloned = cloneElement(child, {
    ref: setRef,
    'aria-describedby': open ? [own['aria-describedby'], id].filter(Boolean).join(' ') : own['aria-describedby'],
    onPointerEnter: (e: React.PointerEvent) => { own.onPointerEnter?.(e); if (e.pointerType !== 'touch') schedule(); },
    onPointerLeave: (e: React.PointerEvent) => { own.onPointerLeave?.(e); hide(); },
    onPointerDown: (e: React.PointerEvent) => { own.onPointerDown?.(e); hide(); },
    onFocus: (e: React.FocusEvent) => { own.onFocus?.(e); if (focusVisible(e.currentTarget)) { clearTimeout(timer.current); reveal(); } },
    onBlur: (e: React.FocusEvent) => { own.onBlur?.(e); hide(); },
    onKeyDown: (e: React.KeyboardEvent) => { own.onKeyDown?.(e); if (e.key === 'Escape' && open) { hide(); e.stopPropagation(); } },
  } as Partial<Handlers>);

  return (
    <>
      {cloned}
      {open && createPortal(
        <div ref={tip} id={id} role="tooltip" className="fa-tip" style={{ visibility: 'hidden' }}>
          <span>{label}</span>
          {shortcut && <kbd>{shortcut}</kbd>}
        </div>,
        document.body,
      )}
    </>
  );
}
