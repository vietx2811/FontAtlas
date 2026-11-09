import { useEffect, useId, useRef } from 'react';

type Props = { title: string; onClose(): void; children: React.ReactNode; className?: string };

/** Hộp thoại dùng chung: chặn nền, Esc đóng, tiêu điểm vào nút đầu tiên. Kiểu `fa-modal*` nằm trong sidebar.css. */
export function Modal({ title, onClose, children, className }: Props): React.JSX.Element {
  const id = useId();
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => { box.current?.querySelector<HTMLElement>('input:not(:disabled), button')?.focus(); }, []);
  return (
    <div className="fa-modal-scrim" onMouseDown={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div ref={box} role="dialog" aria-modal="true" aria-labelledby={id} className={`fa-modal${className ? ` ${className}` : ''}`} onKeyDown={e => { if (e.key === 'Escape') { e.stopPropagation(); onClose(); } }}>
        <h2 id={id}>{title}</h2>
        {children}
      </div>
    </div>
  );
}
