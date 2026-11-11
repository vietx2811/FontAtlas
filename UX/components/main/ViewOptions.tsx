import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { t, type I18nKey } from '../../i18n';
import { CARD_FIELD_ORDER, DEFAULT_CARD_FIELDS, useCardFields, type CardField } from '../../store/cardFields';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import './viewOptions.css';

const label = (k: CardField) => t(`view.field.${k}` as I18nKey);
const gloss = (k: CardField) => t(`view.field.${k}.gloss` as I18nKey);

/** Nút "Hiển thị" trên Top Bar + popover chọn thông tin hiện trên thẻ. ⌘J bật/tắt; Esc đóng và trả focus về nút. */
export function ViewOptions(): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const btn = useRef<HTMLButtonElement>(null);
  const pop = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ right: number; top: number } | null>(null);
  const fields = useCardFields(s => s.fields);
  const toggle = useCardFields(s => s.toggle);
  const reset = useCardFields(s => s.reset);
  const changed = fields.length !== DEFAULT_CARD_FIELDS.length || DEFAULT_CARD_FIELDS.some(k => !fields.includes(k));
  const close = useCallback((refocus: boolean) => { setOpen(false); if (refocus) requestAnimationFrame(() => btn.current?.focus()); }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && !e.altKey && !e.shiftKey && !e.ctrlKey && (e.code === 'KeyJ' || e.key.toLowerCase() === 'j')) { e.preventDefault(); setOpen(o => !o); }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);
  useLayoutEffect(() => {
    if (!open) return;
    const place = () => { const r = btn.current?.getBoundingClientRect(); if (r) setPos({ right: Math.max(8, Math.round(window.innerWidth - r.right)), top: Math.round(r.bottom + 6) }); };
    place(); window.addEventListener('resize', place);
    return () => window.removeEventListener('resize', place);
  }, [open]);
  useEffect(() => {
    if (!open) return;
    pop.current?.querySelector<HTMLElement>('h2')?.focus();
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopImmediatePropagation(); e.preventDefault(); close(true); } };
    const onDown = (e: PointerEvent) => { const el = e.target as Node; if (!pop.current?.contains(el) && !btn.current?.contains(el)) close(false); };
    document.addEventListener('keydown', onKey, true);
    document.addEventListener('pointerdown', onDown, true);
    return () => { document.removeEventListener('keydown', onKey, true); document.removeEventListener('pointerdown', onDown, true); };
  }, [open, close]);

  return (
    <>
      <Tooltip label={t('view.tip')} shortcut="⌘J">
        <button ref={btn} type="button" className="fa-iconbtn" data-testid="view-open" aria-label={t('view.open')} aria-haspopup="dialog" aria-expanded={open} onClick={() => setOpen(o => !o)}>
          <Icon name="sliders" size={16} />
        </button>
      </Tooltip>
      {open && createPortal(
        <div ref={pop} className="fa-viewpop" role="dialog" aria-label={t('view.title')} data-testid="view-pop" style={pos ? { right: pos.right, top: pos.top } : undefined}>
          <header><h2 tabIndex={-1}>{t('view.title')}</h2><button type="button" data-testid="view-reset" disabled={!changed} onClick={reset}>{t('view.reset')}</button></header>
          <fieldset>
            <legend>{t('view.info')}</legend>
            {CARD_FIELD_ORDER.map(k => (
              <label key={k} data-field={k}>
                <input type="checkbox" checked={fields.includes(k)} onChange={() => toggle(k)} />
                <Tooltip label={gloss(k)}><span>{label(k)}</span></Tooltip>
              </label>
            ))}
          </fieldset>
        </div>, document.body)}
    </>
  );
}
