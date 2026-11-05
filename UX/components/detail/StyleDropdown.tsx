import { useEffect, useId, useMemo, useRef, useState } from 'react';
import type { FamilyStyleDto } from '../../api';

export const styleLabel = (s: FamilyStyleDto): string => s.subfamily ?? s.instance?.name ?? 'Regular';
const weightOf = (s: FamilyStyleDto): number => s.weight ?? 400;

/** Kiểu đứng xếp theo weight, rồi kiểu nghiêng xếp theo weight. */
export function orderStyles(styles: FamilyStyleDto[]): { romans: FamilyStyleDto[]; italics: FamilyStyleDto[] } {
  const by = (a: FamilyStyleDto, b: FamilyStyleDto) => weightOf(a) - weightOf(b) || styleLabel(a).localeCompare(styleLabel(b));
  return { romans: styles.filter(s => !s.italic).sort(by), italics: styles.filter(s => s.italic).sort(by) };
}

type Props = {
  styles: FamilyStyleDto[]; value: string | null; onChange(label: string): void;
  /** Họ font đã nạp cho từng kiểu (null = dùng font giao diện + weight/italic CSS). */
  familyOf?(s: FamilyStyleDto): string | null;
};

export function StyleDropdown({ styles, value, onChange, familyOf }: Props): React.JSX.Element {
  const { romans, italics } = useMemo(() => orderStyles([...styles]), [styles]);
  const flat = useMemo(() => [...romans, ...italics], [romans, italics]);
  const current = flat.find(s => styleLabel(s) === value) ?? flat[0];
  const disabled = flat.length <= 1;
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const btn = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const uid = useId();
  const optId = (i: number) => `${uid}-opt-${i}`;

  const show = () => { if (disabled) return; setActive(Math.max(0, flat.indexOf(current))); setOpen(true); };
  const close = (refocus = false) => { setOpen(false); if (refocus) btn.current?.focus(); };
  const choose = (s: FamilyStyleDto) => { onChange(styleLabel(s)); close(true); };

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => { if (!root.current?.contains(e.target as Node)) setOpen(false); };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open]);
  useEffect(() => { if (open) { list.current?.focus(); document.getElementById(optId(active))?.scrollIntoView?.({ block: 'nearest' }); } }, [open, active]); // eslint-disable-line react-hooks/exhaustive-deps

  const css = (s: FamilyStyleDto): React.CSSProperties => {
    const fam = familyOf?.(s);
    return { fontFamily: fam ? `"${fam}", var(--font-ui)` : undefined, fontWeight: weightOf(s), fontStyle: s.italic ? 'italic' : 'normal' };
  };
  const option = (s: FamilyStyleDto) => {
    const i = flat.indexOf(s);
    const label = styleLabel(s);
    const selected = s === current;
    return (
      <li key={`${label}-${i}`} id={optId(i)} role="option" aria-label={label} aria-selected={selected} className={i === active ? 'kb' : undefined} style={css(s)}
        onMouseEnter={() => setActive(i)} onClick={() => choose(s)}>
        <span className="ck" aria-hidden>{selected ? '✓' : ''}</span>{label}<span className="wt">{weightOf(s)}</span>
      </li>
    );
  };

  const onListKey = (e: React.KeyboardEvent) => {
    const last = flat.length - 1;
    const go = (n: number) => { e.preventDefault(); setActive(Math.min(last, Math.max(0, n))); };
    if (e.key === 'ArrowDown') go(active + 1);
    else if (e.key === 'ArrowUp') go(active - 1);
    else if (e.key === 'Home') go(0);
    else if (e.key === 'End') go(last);
    else if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); choose(flat[active]); }
    else if (e.key === 'Escape') { e.preventDefault(); close(true); }
    else if (e.key === 'Tab') close();
  };

  return (
    <div ref={root} className={`fa-dd${open ? ' open' : ''}`}>
      <button ref={btn} type="button" className="fa-dd-btn" aria-label="Kiểu chữ" aria-haspopup="listbox" aria-expanded={open} disabled={disabled}
        onClick={() => (open ? close() : show())} onKeyDown={e => { if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); show(); } }}>
        <span className="cur" style={current ? css(current) : undefined}>{current ? styleLabel(current) : '—'}</span>
        <span className="cnt">{flat.length} kiểu</span>
        {!disabled && <span className="chev" aria-hidden />}
      </button>
      {open && (
        <ul ref={list} className="fa-dd-list" role="listbox" tabIndex={-1} aria-label="Kiểu chữ" aria-activedescendant={optId(active)} onKeyDown={onListKey}>
          {romans.map(option)}
          {italics.length > 0 && romans.length > 0 && <li role="separator" className="sep" />}
          {italics.map(option)}
        </ul>
      )}
    </div>
  );
}
