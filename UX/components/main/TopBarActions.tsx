import { useEffect, useMemo } from 'react';
import type { Api } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { rowKey, useSelectionStore } from '../../store/selection';
import { useUiStore, type PanelTab } from '../../store/ui';
import { Icon, type IconName } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import './topbarActions.css';

const COMPARE_MAX = 4;

/** Mở panel chi tiết của font đầu tiên ở tab đã cho (chưa có màn Pairing Desk / Compare riêng — xem brief 08 mục TODO). */
function openOn(first: { hashHex: string; faceIndex: number }, tab: PanelTab): void {
  const ui = useUiStore.getState();
  ui.select(rowKey(first));
  ui.setPanelTab(tab);
}

/**
 * Cụm hành động Pair / Compare trên Top Bar, theo số font đang chọn (brief 08):
 * 0 → không hiện; 1 → Pair · Find similar; 2–4 → Compare · Pair these; >4 → như 2–4 kèm gợi ý thu hẹp.
 * Compare và Pair these chưa có command backend: mở tab gần nhất của font đầu tiên và báo rõ, không giả dữ liệu.
 */
export function TopBarActions({ api: _api }: { api: Api }): React.JSX.Element | null {
  const selected = useSelectionStore(s => s.rows);
  const rows = useMemo(() => [...selected.values()], [selected]);
  const n = rows.length;
  const first = rows[0];

  const pair = () => { if (first) openOn(first, 'pair'); };
  const similar = () => { if (first) openOn(first, 'sim'); };
  const compare = () => {
    if (!first || n < 2) return;
    openOn(first, 'anat');
    useNoticeStore.getState().push({ kind: 'info', text: t('topbar.compareSoon') });
  };
  const narrow = () => useSelectionStore.getState().replace(rows.slice(0, COMPARE_MAX), { key: rowKey(rows[0]), index: 0 });

  useEffect(() => {
    if (n === 0) return;
    const on = (e: KeyboardEvent) => {
      if (!e.shiftKey || e.metaKey || e.ctrlKey || e.altKey) return;
      const el = e.target as HTMLElement | null;
      if (el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName))) return;
      const k = e.key.toLowerCase();
      if (k === 'p') { e.preventDefault(); pair(); } else if (k === 'c' && n >= 2) { e.preventDefault(); compare(); }
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  });

  if (n === 0) return null;
  const btn = (icon: IconName, key: 'pair' | 'pairThese' | 'compare' | 'similar', kbd: string | null, primary: boolean, run: () => void) => (
    <Tooltip label={t(`topbar.${key}`)} shortcut={kbd ?? undefined}>
      <button type="button" className={`fa-act${primary ? ' pri' : ''}`} aria-label={t(`topbar.${key}`)} onClick={run}><Icon name={icon} size={15} /><span className="l">{t(`topbar.${key}`)}</span>{kbd && <kbd>{kbd}</kbd>}</button>
    </Tooltip>
  );
  return (
    <>
      <div className="fa-acts" role="group" aria-label={t('topbar.group')}>
        <span className="fa-act-n">{t('topbar.selected', { n })}<button type="button" aria-label={t('topbar.clear')} onClick={() => useSelectionStore.getState().clear()}><Icon name="x" size={12} /></button></span>
        {n === 1 ? <>{btn('pair', 'pair', '⇧P', true, pair)}{btn('similar', 'similar', null, false, similar)}</>
          : <>{btn('compare', 'compare', '⇧C', true, compare)}{btn('pair', 'pairThese', '⇧P', false, pair)}</>}
      </div>
      {n > COMPARE_MAX && <div className="fa-act-hint" role="status"><span>{t('topbar.narrow', { n })}</span><button type="button" onClick={narrow}>{t('topbar.useFirst')}</button></div>}
    </>
  );
}
