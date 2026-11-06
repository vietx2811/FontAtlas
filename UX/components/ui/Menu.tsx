import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Tooltip } from './Tooltip';
import './menu.css';

export type MenuItem = {
  type?: 'item'; id: string; label: string; icon?: ReactNode; hint?: string;
  disabled?: boolean; /** Lý do hiện trong tooltip khi mục bị vô hiệu. */ reason?: string; danger?: boolean; checked?: boolean;
  items?: MenuEntry[]; onSelect?(): void;
};
export type MenuEntry = MenuItem | { type: 'sep'; id?: string } | { type: 'label'; id?: string; text: string };

/** Vị trí neo: điểm chuột, hoặc hình chữ nhật của nút (menu mở bên dưới, lật lên nếu thiếu chỗ). */
export type MenuAnchor = { x: number; y: number } | { rect: { left: number; right: number; top: number; bottom: number } };

type Props = {
  entries: MenuEntry[]; anchor: MenuAnchor; onClose(): void; label?: string;
  /** Mở bằng bàn phím: focus mục đầu tiên ngay. */
  focusFirst?: boolean;
  /** Phần tử được trả focus khi đóng. */
  returnFocus?: HTMLElement | null;
};

const EDGE = 8;
const SUB_DELAY_MS = 140;
const isItem = (e: MenuEntry): e is MenuItem => e.type === undefined || e.type === 'item';

function List({ entries, onClose, closeAll, depth, focusFirst, label }: {
  entries: MenuEntry[]; onClose(): void; closeAll(): void; depth: number; focusFirst?: boolean; label?: string;
}): React.JSX.Element {
  const ref = useRef<HTMLDivElement>(null);
  const [sub, setSubState] = useState<{ id: string; kb: boolean } | null>(null);
  const setSub = (id: string | null, kb = false) => setSubState(id === null ? null : { id, kb });
  const subTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const enabled = useCallback(() => [...(ref.current?.querySelectorAll<HTMLElement>(':scope > .fa-mi-wrap > .fa-mi:not([aria-disabled="true"])') ?? [])], []);
  useEffect(() => () => clearTimeout(subTimer.current), []);
  useEffect(() => { if (focusFirst) enabled()[0]?.focus(); else if (depth === 0) ref.current?.focus(); }, [focusFirst, depth, enabled]);

  const step = (from: Element | null, dir: 1 | -1) => {
    const list = enabled();
    if (!list.length) return;
    const i = from ? list.indexOf(from as HTMLElement) : -1;
    list[i < 0 ? (dir === 1 ? 0 : list.length - 1) : (i + dir + list.length) % list.length].focus();
  };
  const activate = (item: MenuItem, viaKeyboard: boolean) => {
    if (item.disabled) return;
    if (item.items) { setSub(item.id, viaKeyboard); return; }
    closeAll();
    item.onSelect?.();
  };
  const onKeyDown = (e: React.KeyboardEvent) => {
    const cur = (e.target as HTMLElement).closest('.fa-mi');
    const handled = () => { e.preventDefault(); e.stopPropagation(); };
    if (e.key === 'ArrowDown') { handled(); step(cur, 1); }
    else if (e.key === 'ArrowUp') { handled(); step(cur, -1); }
    else if (e.key === 'Home') { handled(); enabled()[0]?.focus(); }
    else if (e.key === 'End') { handled(); enabled().at(-1)?.focus(); }
    else if (e.key === 'ArrowRight') {
      const item = entries.find(x => isItem(x) && x.id === (cur as HTMLElement | null)?.dataset.id) as MenuItem | undefined;
      if (item?.items) { handled(); activate(item, true); }
    } else if (e.key === 'ArrowLeft' && depth > 0) { handled(); onClose(); }
    else if (e.key === 'Escape') { handled(); if (depth > 0) onClose(); else closeAll(); }
    else if (e.key === 'Tab') { handled(); closeAll(); }
    else if ((e.key === 'Enter' || e.key === ' ') && cur) {
      const item = entries.find(x => isItem(x) && x.id === (cur as HTMLElement).dataset.id) as MenuItem | undefined;
      if (item) { handled(); activate(item, true); }
    }
  };

  return (
    <div ref={ref} role="menu" aria-label={label} tabIndex={-1} className="fa-menu2" data-depth={depth} onKeyDown={onKeyDown}>
      {entries.map((e, i) => {
        if (e.type === 'sep') return <div key={e.id ?? `s${i}`} role="separator" className="fa-mi-sep" />;
        if (e.type === 'label') return <div key={e.id ?? `l${i}`} role="presentation" className="fa-mi-label">{e.text}</div>;
        const open = sub?.id === e.id && !!e.items;
        const btn = (
          <button type="button" role="menuitem" className="fa-mi" data-id={e.id} data-danger={e.danger ? 'true' : undefined}
            aria-disabled={e.disabled ? 'true' : undefined} aria-haspopup={e.items ? 'menu' : undefined} aria-expanded={e.items ? open : undefined}
            tabIndex={-1}
            onPointerEnter={ev => {
              if (e.disabled) return;
              (ev.currentTarget as HTMLElement).focus({ preventScroll: true });
              clearTimeout(subTimer.current);
              if (e.items) subTimer.current = setTimeout(() => setSub(e.id), SUB_DELAY_MS);
              else if (sub) subTimer.current = setTimeout(() => setSub(null), SUB_DELAY_MS);
            }}
            onClick={() => activate(e, false)}>
            <span className="fa-mi-ico" aria-hidden>{e.checked ? '✓' : e.icon}</span>
            <span className="fa-mi-lbl">{e.label}</span>
            {e.hint && <span className="fa-mi-hint">{e.hint}</span>}
            {e.items && <span className="fa-mi-chev" aria-hidden>›</span>}
          </button>
        );
        return (
          <div key={e.id} className="fa-mi-wrap">
            {e.disabled && e.reason ? <Tooltip label={e.reason}>{btn}</Tooltip> : btn}
            {open && <SubMenu parent={e} focusFirst={sub?.kb} onClose={() => { setSub(null); [...(ref.current?.querySelectorAll<HTMLElement>('.fa-mi') ?? [])].find(b => b.dataset.id === e.id)?.focus(); }} closeAll={closeAll} depth={depth + 1} />}
          </div>
        );
      })}
    </div>
  );
}

function SubMenu({ parent, onClose, closeAll, depth, focusFirst }: { parent: MenuItem; onClose(): void; closeAll(): void; depth: number; focusFirst?: boolean }): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null);
  // Lật sang trái nếu tràn mép phải; kéo lên nếu tràn mép dưới.
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    if (r.right > window.innerWidth - EDGE) el.dataset.flip = 'left';
    if (r.bottom > window.innerHeight - EDGE) el.style.top = `${-Math.min(r.top - EDGE, r.bottom - window.innerHeight + EDGE) - 5}px`;
  }, []);
  return (
    <div ref={box} className="fa-sub" data-sub-of={parent.id}>
      <List entries={parent.items ?? []} onClose={onClose} closeAll={closeAll} depth={depth} focusFirst={focusFirst} label={parent.label} />
    </div>
  );
}

/** Menu kiểu macOS: portal, mũi tên/Enter/Esc, submenu bằng →/←, đóng khi bấm ngoài hoặc cuộn. */
export function Menu({ entries, anchor, onClose, label, focusFirst, returnFocus }: Props): React.JSX.Element {
  const root = useRef<HTMLDivElement>(null);
  const closed = useRef(false);
  const close = useCallback(() => {
    if (closed.current) return;
    closed.current = true;
    onClose();
    returnFocus?.focus?.({ preventScroll: true });
  }, [onClose, returnFocus]);

  useLayoutEffect(() => {
    const el = root.current;
    if (!el) return;
    const w = el.offsetWidth; const h = el.offsetHeight;
    const vw = window.innerWidth; const vh = window.innerHeight;
    let left: number; let top: number;
    if ('rect' in anchor) {
      left = anchor.rect.left;
      top = anchor.rect.bottom + 4;
      if (top + h > vh - EDGE) top = Math.max(EDGE, anchor.rect.top - h - 4);
    } else { left = anchor.x; top = anchor.y; if (top + h > vh - EDGE) top = Math.max(EDGE, anchor.y - h); }
    if (left + w > vw - EDGE) left = Math.max(EDGE, ('rect' in anchor ? anchor.rect.right : anchor.x) - w);
    el.style.left = `${Math.round(left)}px`;
    el.style.top = `${Math.round(top)}px`;
    el.style.visibility = 'visible';
  }, [anchor]);

  useEffect(() => {
    const down = (e: Event) => { if (!root.current?.contains(e.target as Node)) close(); };
    const scroll = (e: Event) => { if (!root.current?.contains(e.target as Node)) close(); };
    document.addEventListener('pointerdown', down, true);
    document.addEventListener('contextmenu', down, true);
    window.addEventListener('scroll', scroll, true);
    window.addEventListener('wheel', close, { passive: true });
    window.addEventListener('resize', close);
    window.addEventListener('blur', close);
    return () => {
      document.removeEventListener('pointerdown', down, true);
      document.removeEventListener('contextmenu', down, true);
      window.removeEventListener('scroll', scroll, true);
      window.removeEventListener('wheel', close);
      window.removeEventListener('resize', close);
      window.removeEventListener('blur', close);
    };
  }, [close]);

  return createPortal(
    <div ref={root} className="fa-menu2-root" style={{ visibility: 'hidden' }} onContextMenu={e => e.preventDefault()}>
      <List entries={entries} onClose={close} closeAll={close} depth={0} focusFirst={focusFirst} label={label} />
    </div>,
    document.body,
  );
}
