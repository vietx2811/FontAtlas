import { useEffect, useMemo, useRef } from 'react';
import type { Api } from '../../api';
import { t } from '../../i18n';
import { useActivationFlow } from '../activation/useActivationFlow';
import { NarrowTag } from './NarrowTag';
import { useNoticeStore } from '../../store/notices';
import { rowKey, useSelectionStore } from '../../store/selection';
import { useUiStore, type PanelTab } from '../../store/ui';
import { Icon, type IconName } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import './topbarActions.css';

const COMPARE_MAX = 4; // ngưỡng gợi ý "nên ≤ 4"
const COMPARE_MAX_FACES = 8; // trần cứng của màn Compare
const PAIR_MAX_FACES = 4; // trần của Pair these

type Row = { hashHex: string; faceIndex: number };
/** Mở panel chi tiết của font đầu tiên ở tab đã cho (chưa có màn Pairing Desk / Compare riêng — xem brief 08 mục TODO). */
function openOn(first: Row, tab: PanelTab): void {
  const ui = useUiStore.getState();
  ui.select(rowKey(first));
  ui.setPanelTab(tab);
}
const currentRows = () => [...useSelectionStore.getState().rows.values()];
function doPair(): void {
  const rows = currentRows(); if (!rows.length) return;
  if (rows.length === 1) { openOn(rows[0], 'pair'); return; }
  if (rows.length > PAIR_MAX_FACES) useNoticeStore.getState().push({ kind: 'info', text: t('pair.cut', { n: rows.length }) });
  useUiStore.getState().openPairThese(rows.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
}
const doSimilar = (): void => { const r = currentRows()[0]; if (r) openOn(r, 'sim'); };
function doCompare(): void {
  const rows = currentRows(); if (rows.length < 2) return;
  if (rows.length > COMPARE_MAX_FACES) useNoticeStore.getState().push({ kind: 'info', text: t('topbar.compareCut', { n: rows.length }) });
  useUiStore.getState().openCompare(rows.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
}
/** Phím tắt chỉ chạy khi không có lớp phủ/hộp thoại/menu, không gõ chữ, không đang IME hay giữ phím. */
export function shortcutBlocked(e: KeyboardEvent): boolean {
  if (useUiStore.getState().compare || useUiStore.getState().pairThese) return true; // đang ở màn Compare/Pair these: ⇧P/⇧C không đổi thư viện ẩn
  const el = e.target as HTMLElement | null;
  if (el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName))) return true;
  return e.defaultPrevented || e.isComposing || e.repeat || !!document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"], [aria-modal="true"]');
}

type SelRow = { hashHex: string; faceIndex: number; missing?: boolean };
/**
 * Phím tắt ⌘⇧A bật/tắt kích hoạt cho cả lựa chọn (cả khi chọn 1 font). Không có nút riêng trên Top Bar: thanh chọn đã có nút
 * "Kích hoạt / Tắt kích hoạt" có nhãn, hai nút cùng việc chỉ làm rối. Font mất file bị bỏ qua; font đã dùng được nhờ bản giống hệt của
 * bên khác coi như đã bật. Hộp thoại (xung đột / đang được dự án giữ) nằm ở hàng đợi chung, không mất khi bỏ chọn.
 */
function ActivationShortcut({ api, rows }: { api: Api; rows: SelRow[] }): null {
  const flow = useActivationFlow(api);
  const active = useUiStore(s => s.active);
  const available = useUiStore(s => s.available);
  const usable = useMemo(() => rows.filter(r => !r.missing), [rows]);
  const allOn = usable.length > 0 && usable.every(r => active.has(r.hashHex) || available.has(r.hashHex));
  const run = () => { if (usable.length) void flow.toggle(usable.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })), !allOn); };
  const runRef = useRef(run); runRef.current = run;
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || !e.shiftKey || e.altKey || e.key.toLowerCase() !== 'a' || shortcutBlocked(e)) return;
      e.preventDefault(); runRef.current();
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, []);
  return null;
}

/**
 * Cụm hành động Pair / Compare trên Top Bar, theo số font đang chọn (brief 08):
 * 0 → không hiện; 1 → Pair · Find similar; 2–4 → Compare · Pair these; >4 → như 2–4 kèm gợi ý thu hẹp.
 * Compare và Pair these chưa có command backend: mở tab gần nhất của font đầu tiên và báo rõ, không giả dữ liệu.
 */
export function TopBarActions({ api }: { api?: Api } = {}): React.JSX.Element | null {
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
          : <><span className="fa-act-wrap">{btn('compare', 'compare', '⇧C', true, doCompare)}{n > COMPARE_MAX && <NarrowTag n={n} max={COMPARE_MAX} onNarrow={narrow} />}</span>{btn('pair', 'pairThese', '⇧P', false, doPair)}</>}
        {api && <ActivationShortcut api={api} rows={rows} />}
      </div>
    </>
  );
}
