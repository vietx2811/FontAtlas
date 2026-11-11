import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { t, type I18nKey } from '../../i18n';
import { SORT_FIELDS, effectiveSort, isDefaultSort, type SortField } from '../../store/sortPrefs';
import { useUiStore } from '../../store/ui';
import { CARD_FIELD_ORDER, DEFAULT_CARD_FIELDS, useCardFields, type CardField } from '../../store/cardFields';
import { Icon } from '../sidebar/icons';
import { shortcutBlocked } from './TopBarActions';
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
  const sortPrefs = useUiStore(s => s.sort);
  const view = useUiStore(s => s.view);
  const hasQuery = useUiStore(s => s.query.trim().length > 0);
  const setSortBy = useUiStore(s => s.setSortBy);
  const setSortDesc = useUiStore(s => s.setSortDesc);
  const resetSort = useUiStore(s => s.resetSort);
  const locked = view === 'recent' || view === 'active';
  const eff = effectiveSort(sortPrefs, hasQuery);
  const by: SortField = eff.by;
  const desc = eff.desc;
  const sortChanged = !isDefaultSort(sortPrefs) || !!sortPrefs.explicit;
  const changed = sortChanged || fields.length !== DEFAULT_CARD_FIELDS.length || DEFAULT_CARD_FIELDS.some(k => !fields.includes(k));
  // Trả focus ĐỒNG BỘ (nút luôn còn trong DOM); rAF chỉ là dự phòng khi focus bị cướp lại ngay sau khi popover gỡ.
  const close = useCallback((refocus: boolean) => { setOpen(false); if (refocus) { btn.current?.focus(); requestAnimationFrame(() => { if (document.activeElement !== btn.current) btn.current?.focus(); }); } }, []);

  // ⌘J: bật/tắt. Mở rồi thì luôn đóng qua close(true) (trả focus về nút). Đóng → mở chỉ khi không gõ chữ / IME / hộp thoại, Compare... (shortcutBlocked).
  const openRef = useRef(open); openRef.current = open;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey || e.altKey || e.shiftKey || e.ctrlKey || e.defaultPrevented || e.isComposing || e.repeat || !(e.code === 'KeyJ' || e.key.toLowerCase() === 'j')) return;
      if (openRef.current) { e.preventDefault(); close(true); return; }
      if (shortcutBlocked(e)) return;
      e.preventDefault(); setOpen(true);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [close]);
  useLayoutEffect(() => {
    if (!open) return;
    const place = () => { const r = btn.current?.getBoundingClientRect(); if (r) setPos({ right: Math.max(8, Math.round(window.innerWidth - r.right)), top: Math.round(r.bottom + 6) }); };
    place(); window.addEventListener('resize', place);
    return () => window.removeEventListener('resize', place);
  }, [open]);
  useEffect(() => {
    if (!open) return;
    pop.current?.querySelector<HTMLElement>('h2')?.focus();
    // Bấm ra ngoài thì đóng; tooltip (portal) không phải "ngoài".
    const onDown = (e: PointerEvent) => { const el = e.target as Element; if (!pop.current?.contains(el) && !btn.current?.contains(el) && !el.closest?.('[role=tooltip]')) close(false); };
    document.addEventListener('pointerdown', onDown, true);
    return () => document.removeEventListener('pointerdown', onDown, true);
  }, [open, close]);
  // Esc xử lý tại chỗ (không nuốt Esc của sheet/hộp thoại khác); Tab quay vòng trong popover.
  const onPopKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(true); return; }
    if (e.key !== 'Tab' || !pop.current) return;
    const items = [...pop.current.querySelectorAll<HTMLElement>('input:not(:disabled), button:not(:disabled), select:not(:disabled), [tabindex="0"]')];
    if (!items.length) return;
    const first = items[0], last = items[items.length - 1], cur = document.activeElement;
    if (e.shiftKey && (cur === first || cur === pop.current.querySelector('h2'))) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && cur === last) { e.preventDefault(); first.focus(); }
  };
  // Focus rời hẳn khỏi popover (không phải về nút, không phải null do bấm vào chỗ không focus được) thì đóng.
  const onPopBlur = (e: React.FocusEvent) => {
    const to = e.relatedTarget as Node | null;
    if (to && !pop.current?.contains(to) && !btn.current?.contains(to)) close(false);
  };

  return (
    <>
      <Tooltip label={t('view.tip')} shortcut="⌘J">
        <button ref={btn} type="button" className="fa-iconbtn" data-testid="view-open" onKeyDown={e => { if (open && e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(true); } }} aria-label={t('view.open')} aria-haspopup="dialog" aria-expanded={open} onClick={() => setOpen(o => !o)}>
          <Icon name="sliders" size={16} />
        </button>
      </Tooltip>
      {open && createPortal(
        <div ref={pop} className="fa-viewpop" role="dialog" aria-label={t('view.title')} data-testid="view-pop" tabIndex={-1} onKeyDown={onPopKey} onBlur={onPopBlur} style={pos ? { right: pos.right, top: pos.top } : undefined}>
          <header><h2 tabIndex={-1}>{t('view.title')}</h2><button type="button" data-testid="view-reset" aria-disabled={!changed} onClick={() => { if (changed) { reset(); resetSort(); } }}>{t('view.reset')}</button></header>
          <fieldset data-testid="view-sort" aria-describedby={locked ? 'view-sort-lock' : undefined} aria-disabled={locked || undefined}>
            <legend>{t('view.sort')}</legend>
            {locked ? <p id="view-sort-lock" className="fa-viewpop-lock" data-testid="view-sort-locked">{t(view === 'recent' ? 'view.sort.locked.recent' : 'view.sort.locked.active')}</p> : <>
              <select data-testid="view-sort-by" aria-label={t('view.sort')} value={by} onChange={e => setSortBy(e.target.value as SortField)}>
                {SORT_FIELDS.filter(f => f !== 'match' || hasQuery).map(f => <option key={f} value={f}>{t(`view.sort.${f}` as I18nKey)}</option>)}
              </select>
              {by !== 'match' && <div className="fa-viewpop-dir" role="radiogroup" aria-label={t('view.sort.dir')}>
                {([false, true] as const).map(d => (
                  <button key={String(d)} type="button" role="radio" aria-checked={desc === d} data-testid={d ? 'view-sort-desc' : 'view-sort-asc'} onClick={() => setSortDesc(d)}>{t(`view.sort.${by}.${d ? 'desc' : 'asc'}` as I18nKey)}</button>
                ))}
              </div>}
              <p className="fa-viewpop-src">{t(`view.sort.src.${by}` as I18nKey)}</p>
            </>}
          </fieldset>
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
