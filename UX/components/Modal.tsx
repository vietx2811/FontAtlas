import { useEffect, useId, useRef } from 'react';
import { createPortal } from 'react-dom';

type Props = { title: string; onClose(): void; children: React.ReactNode; className?: string };

const FOCUSABLE = 'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';
/** Các Modal đang mở (cái cuối là trên cùng): chỉ cái trên cùng nhận Esc và Tab. */
const stack: symbol[] = [];

/**
 * Hộp thoại dùng chung. Chặn nền; Esc đóng KỂ CẢ khi tiêu điểm đang ở body; Tab quay vòng trong hộp; đóng xong trả tiêu điểm
 * về phần tử đã có trước khi mở. Nền chỉ đóng hộp khi bấm trọn (nhấn VÀ nhả) trên nền, không phải khi kéo chuột từ trong hộp ra ngoài.
 * Kiểu `fa-modal*` nằm trong sidebar.css.
 */
export function Modal({ title, onClose, children, className }: Props): React.JSX.Element {
  const id = useId();
  const box = useRef<HTMLDivElement>(null);
  const closeRef = useRef(onClose); closeRef.current = onClose;
  const pressedScrim = useRef(false);
  useEffect(() => {
    const me = Symbol('modal');
    stack.push(me);
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const first = box.current?.querySelector<HTMLElement>('input:not(:disabled), button');
    (first ?? box.current)?.focus();
    const on = (e: KeyboardEvent) => {
      if (stack[stack.length - 1] !== me) return;
      if (e.key === 'Escape') { e.stopPropagation(); e.preventDefault(); closeRef.current(); return; }
      if (e.key !== 'Tab' || !box.current) return;
      const items = [...box.current.querySelectorAll<HTMLElement>(FOCUSABLE)];
      if (!items.length) { e.preventDefault(); box.current.focus(); return; }
      const [head, tail] = [items[0], items[items.length - 1]];
      const cur = document.activeElement;
      if (!box.current.contains(cur)) { e.preventDefault(); head.focus(); }
      else if (e.shiftKey && (cur === head || cur === box.current)) { e.preventDefault(); tail.focus(); }
      else if (!e.shiftKey && cur === tail) { e.preventDefault(); head.focus(); }
    };
    document.addEventListener('keydown', on, true);
    return () => {
      document.removeEventListener('keydown', on, true);
      stack.splice(stack.indexOf(me), 1);
      if (before?.isConnected) before.focus();
    };
  }, []);
  // Portal vào <body>: hộp thoại dựng ở nơi nào trong cây (vd trong Top Bar) cũng không bị kẹt dưới stacking context của vùng đó.
  return createPortal(
    <div className="fa-modal-scrim"
      onMouseDown={e => { pressedScrim.current = e.target === e.currentTarget; }}
      onClick={e => { const ok = pressedScrim.current && e.target === e.currentTarget; pressedScrim.current = false; if (ok) onClose(); }}>
      <div ref={box} role="dialog" aria-modal="true" aria-labelledby={id} tabIndex={-1} className={`fa-modal${className ? ` ${className}` : ''}`}>
        <h2 id={id}>{title}</h2>
        {children}
      </div>
    </div>,
    document.body,
  );
}
