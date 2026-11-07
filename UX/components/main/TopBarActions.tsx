import { useEffect, useMemo } from 'react';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { rowKey, useSelectionStore } from '../../store/selection';
import { useUiStore, type PanelTab } from '../../store/ui';
import { Icon, type IconName } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import './topbarActions.css';

const COMPARE_MAX = 4;

type Row = { hashHex: string; faceIndex: number };
/** Mở panel chi tiết của font đầu tiên ở tab đã cho (chưa có màn Pairing Desk / Compare riêng — xem brief 08 mục TODO). */
function openOn(first: Row, tab: PanelTab): void {
  const ui = useUiStore.getState();
  ui.select(rowKey(first));
  ui.setPanelTab(tab);
}
const soon = (key: 'topbar.pairSoon') => useNoticeStore.getState().push({ kind: 'info', text: t(key) });
const currentRows = () => [...useSelectionStore.getState().rows.values()];
function doPair(): void {
  const rows = currentRows(); if (!rows.length) return;
  openOn(rows[0], 'pair');
  if (rows.length >= 2) soon('topbar.pairSoon'); // Pair these: chưa có command riêng, mở Pair của font đầu
}
const doSimilar = (): void => { const r = currentRows()[0]; if (r) openOn(r, 'sim'); };
function doCompare(): void {
  const rows = currentRows(); if (rows.length < 2) return;
  useUiStore.getState().openCompare(rows.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
}
/** Phím tắt chỉ chạy khi không có lớp phủ/hộp thoại/menu, không gõ chữ, không đang IME hay giữ phím. */
export function shortcutBlocked(e: KeyboardEvent): boolean {
  const el = e.target as HTMLElement | null;
  if (el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName))) return true;
  return e.defaultPrevented || e.isComposing || e.repeat || !!document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"], [aria-modal="true"]');
}

/**
 * Cụm hành động Pair / Compare trên Top Bar, theo số font đang chọn (brief 08):
 * 0 → không hiện; 1 → Pair · Find similar; 2–4 → Compare · Pair these; >4 → như 2–4 kèm gợi ý thu hẹp.
 * Compare và Pair these chưa có command backend: mở tab gần nhất của font đầu tiên và báo rõ, không giả dữ liệu.
 */
export function TopBarActions(): React.JSX.Element | null {
  const selected = useSelectionStore(s => s.rows);
  const rows = useMemo(() => [...selected.values()], [selected]);
  const n = rows.length;
  const narrow = () => {
    const keep = rows.slice(0, COMPARE_MAX);
    const anchor = useSelectionStore.getState().anchor;
    useSelectionStore.getState().replace(keep, anchor && keep.some(r => rowKey(r) === anchor.key) ? anchor : { key: rowKey(keep[0]), index: 0 });
  };

  useEffect(() => {
    if (n === 0) return;
    const on = (e: KeyboardEvent) => {
      if (!e.shiftKey || e.metaKey || e.ctrlKey || e.altKey || shortcutBlocked(e)) return;
      const k = e.key.toLowerCase();
      if (k === 'p') { e.preventDefault(); doPair(); } else if (k === 'c' && n >= 2) { e.preventDefault(); doCompare(); }
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, [n]);

  if (n === 0) return null;
  const btn = (icon: IconName, key: 'pair' | 'pairThese' | 'compare' | 'similar', kbd: string | null, primary: boolean, run: () => void) => (
    <Tooltip label={`${t(`topbar.${key}`)} — ${t(`topbar.${key}.gloss`)}`} shortcut={kbd ?? undefined}>
      <button type="button" className={`fa-act${primary ? ' pri' : ''}`} aria-label={t(`topbar.${key}`)} onClick={run}><Icon name={icon} size={15} /><span className="l">{t(`topbar.${key}`)}</span>{kbd && <kbd>{kbd}</kbd>}</button>
    </Tooltip>
  );
  return (
    <>
      <div className="fa-acts" role="group" aria-label={t('topbar.group')}>
        <span className="fa-act-n">{t('topbar.selected', { n })}<button type="button" aria-label={t('topbar.clear')} onClick={() => useSelectionStore.getState().clear()}><Icon name="x" size={12} /></button></span>
        {n === 1 ? <>{btn('pair', 'pair', '⇧P', true, doPair)}{btn('similar', 'similar', null, false, doSimilar)}</>
          : <>{btn('compare', 'compare', '⇧C', true, doCompare)}{btn('pair', 'pairThese', '⇧P', false, doPair)}</>}
      </div>
      {n > COMPARE_MAX && <div className="fa-act-hint" role="status"><span>{t('topbar.narrow', { n })}</span><button type="button" onClick={narrow}>{t('topbar.useFirst')}</button></div>}
    </>
  );
}
