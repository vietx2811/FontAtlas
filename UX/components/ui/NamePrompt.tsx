import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { MenuAnchor } from './Menu';
import './menu.css';
import { t } from '../../i18n';

type Props = { label: string; initial: string; submitLabel?: string; cancelLabel?: string; anchor: MenuAnchor; onSubmit(name: string): void; onCancel(): void };

/** Hộp nhỏ nhập một tên (Dự án mới…, Thẻ mới…). Enter tạo, Esc huỷ, bấm ngoài huỷ. */
export function NamePrompt({ label, initial, submitLabel = t('np.create'), cancelLabel = t('np.cancel'), anchor, onSubmit, onCancel }: Props): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState(initial);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const pt = 'rect' in anchor ? { x: anchor.rect.left, y: anchor.rect.bottom + 4 } : anchor;
    el.style.left = `${Math.max(8, Math.min(pt.x, window.innerWidth - el.offsetWidth - 8))}px`;
    el.style.top = `${Math.max(8, Math.min(pt.y, window.innerHeight - el.offsetHeight - 8))}px`;
    el.style.visibility = 'visible';
  }, [anchor]);
  useEffect(() => { input.current?.focus(); input.current?.select(); }, []);
  useEffect(() => {
    const down = (e: Event) => { if (!box.current?.contains(e.target as Node)) onCancel(); };
    document.addEventListener('pointerdown', down, true);
    return () => document.removeEventListener('pointerdown', down, true);
  }, [onCancel]);
  const submit = () => { const name = value.trim(); if (name) onSubmit(name); };
  return createPortal(
    <div ref={box} className="fa-prompt" role="dialog" aria-label={label} style={{ visibility: 'hidden' }}
      onKeyDown={e => { e.stopPropagation(); if (e.key === 'Escape') { e.preventDefault(); onCancel(); } }}>
      <label>{label}<input ref={input} value={value} aria-label={label} autoComplete="off" onChange={e => setValue(e.target.value)} onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); submit(); } }} /></label>
      <div className="row"><button type="button" onClick={onCancel}>{cancelLabel}</button><button type="button" data-primary onClick={submit} disabled={!value.trim()}>{submitLabel}</button></div>
    </div>,
    document.body,
  );
}
