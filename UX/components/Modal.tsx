import { useEffect, useId, useRef } from 'react';
import { createPortal } from 'react-dom';

type Props = { title: string; onClose(): void; children: React.ReactNode; className?: string };

/** Hộp thoại dùng chung: chặn nền, Esc đóng, tiêu điểm vào nút đầu tiên. Kiểu `fa-modal*` nằm trong sidebar.css. */
export function Modal({ title, onClose, children, className }: Props): React.JSX.Element {
  const id = useId();
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => { box.current?.querySelector<HTMLElement>('input:not(:disabled), button')?.focus(); }, []);
  // Portal vào <body>: hộp thoại dựng ở nơi nào trong cây (vd trong Top Bar) cũng không bị kẹt dưới stacking context của vùng đó.
  return createPortal(
    <div className="fa-modal-scrim" onMouseDown={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div ref={box} role="dialog" aria-modal="true" aria-labelledby={id} className={`fa-modal${className ? ` ${className}` : ''}`} onKeyDown={e => { if (e.key === 'Escape') { e.stopPropagation(); onClose(); } }}>
        <h2 id={id}>{title}</h2>
        {children}
      </div>
    </div>,
    document.body,
  );
}
