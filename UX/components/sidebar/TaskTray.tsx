// Khay "Tác vụ nền" ở chân thanh bên (beta.5). Ẩn hẳn khi rảnh; không lịch sử; chỉ có huỷ.
import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { Api } from '../../api';
import type { TaskDto } from '../../api/taskTypes';
import { formatNumber, t, tn, hasKey, type I18nKey } from '../../i18n';
import { buildRows, shownTasks, summarize, taskTitle, useTaskHubStore, type TaskHubStore, type TaskRow } from '../../store/taskHub';
import { Tooltip } from '../ui/Tooltip';
import { Icon, type IconName } from './icons';
import './taskTray.css';

const KIND_ICON: Record<TaskDto['kind'], IconName> = { scan: 'folder', analyze: 'sliders', embed: 'similar', cloudDownload: 'cloud', activation: 'power', resolveDuplicates: 'copy', export: 'download', other: 'more' };
const kindLabel = (k: TaskDto['kind']): string => t(`task.kind.${k}` as I18nKey);

function Ring({ value }: { value: number | null }): React.JSX.Element {
  const C = 2 * Math.PI * 9;
  return (
    <svg className={`fa-task-ring${value === null ? ' spin' : ''}`} viewBox="0 0 22 22" width="26" height="26" aria-hidden data-testid="task-ring" data-value={value === null ? 'indeterminate' : value.toFixed(2)}>
      <circle className="bgc" cx="11" cy="11" r="9" />
      <circle className="fgc" cx="11" cy="11" r="9" strokeDasharray={value === null ? `${C * 0.28} ${C}` : C} strokeDashoffset={value === null ? 0 : C * (1 - Math.min(1, Math.max(0, value)))} />
    </svg>
  );
}

/** "3/12" (items), "62%" (percent), "12 / 40 MB" (bytes); chưa biết tổng thì chỉ số đã làm. */
export function progressText(x: { done: number; total: number | null; unit: TaskDto['unit'] }): string {
  if (x.unit === 'percent') return `${formatNumber(Math.round(x.total ? (x.done / x.total) * 100 : x.done))}%`;
  const mb = (n: number) => formatNumber(Math.round(n / 1048576));
  if (x.unit === 'bytes') return x.total === null ? `${mb(x.done)} MB` : `${mb(x.done)}/${mb(x.total)} MB`;
  return x.total === null ? formatNumber(x.done) : `${formatNumber(x.done)}/${formatNumber(x.total)}`;
}

function Row({ row, confirming, onAsk, onCancel, onKeep }: { row: TaskRow; confirming: boolean; onAsk(): void; onCancel(): void; onKeep(): void }): React.JSX.Element {
  const first = row.tasks[0];
  const grouped = row.tasks.length > 1;
  const title = grouped ? t('task.group', { n: row.tasks.length }) : taskTitle(first);
  const frac = row.total && row.total > 0 ? Math.min(1, row.done / row.total) : null;
  const text = row.finished ? t('task.done') : row.state === 'queued' && row.done === 0 ? t('task.queued') : row.total === null && row.done === 0 ? t('task.unknownTotal') : progressText({ done: row.done, total: row.total, unit: first.unit });
  const kindTip = first.trigger === 'watcher' ? t('task.watcherHint') : hasKey(`task.kindTip.${row.kind}`) ? t(`task.kindTip.${row.kind}` as I18nKey) : kindLabel(row.kind);
  const cancelLabel = grouped ? t('task.cancelAll') : t('task.cancel');
  return (
    <li className="fa-task-row" data-testid="task-row" data-kind={row.kind} data-state={row.state}>
      <Tooltip label={kindTip}><span className="fa-task-kind"><Icon name={row.finished ? 'check' : KIND_ICON[row.kind]} size={15} /></span></Tooltip>
      <span className="fa-task-name" title={title}>{grouped ? `${kindLabel(row.kind)} · ${title}` : title}</span>
      <span className="fa-task-num" data-testid="task-num">{text}</span>
      {!row.finished && row.cancellable && !confirming && (
        row.state === 'cancelling'
          ? <span className="fa-task-cancelling">{t('task.cancelling')}</span>
          : <Tooltip label={cancelLabel}><button type="button" className="fa-task-x" aria-label={`${cancelLabel}: ${title}`} data-testid="task-cancel" onClick={row.confirmKey ? onAsk : onCancel}><Icon name="x" size={13} /></button></Tooltip>
      )}
      {confirming && row.confirmKey && (
        <span className="fa-task-confirm" role="group" aria-label={hasKey(row.confirmKey) ? t(row.confirmKey) : t('task.cancel')} data-testid="task-confirm">
          <span>{hasKey(row.confirmKey) ? t(row.confirmKey) : ''}</span>
          <button type="button" data-testid="task-keep" onClick={onKeep}>{t('task.keep')}</button>
          <button type="button" className="danger" data-testid="task-confirm-yes" onClick={onCancel}>{t('task.cancelYes')}</button>
        </span>
      )}
      {!row.finished && <span className="fa-task-bar" aria-hidden><i style={{ width: frac === null ? '30%' : `${Math.round(frac * 100)}%` }} className={frac === null ? 'indet' : ''} /></span>}
    </li>
  );
}

export function TaskTray({ api, store = useTaskHubStore }: { api: Api; store?: TaskHubStore }): React.JSX.Element | null {
  const tasks = store(s => s.tasks);
  const doneMap = store(s => s.done);
  const running = useMemo(() => shownTasks({ rev: 0, tasks, done: {} }), [tasks]);
  const finished = useMemo(() => Object.values(doneMap), [doneMap]);
  const rows = useMemo(() => buildRows(running, finished), [running, finished]);
  const [open, setOpen] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  // Sidebar thu gọn có overflow:hidden → popover vẽ ra ngoài bằng portal + position:fixed cạnh nút.
  const [fixedAt, setFixedAt] = useState<{ left: number; bottom: number } | null>(null);
  const btn = useRef<HTMLButtonElement>(null);
  const pop = useRef<HTMLDivElement>(null);
  const id = useId();
  const idle = running.length === 0 && finished.length === 0;

  useEffect(() => {
    if (!idle) return;
    // Khay đã unmount cùng popover đang mở: focus rơi về <body>; trả về nút Cài đặt (cùng chân sidebar) để bàn phím không mất chỗ.
    const a = document.activeElement;
    if (open && (!a || a === document.body)) document.querySelector<HTMLElement>(`.fa-foot-actions button[aria-label="${t('side.settings')}"]`)?.focus();
    setOpen(false); setConfirming(null);
  }, [idle, open]);
  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => { const n = e.target as Node; if (!pop.current?.contains(n) && !btn.current?.contains(n)) setOpen(false); };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopPropagation(); setOpen(false); btn.current?.focus(); } };
    document.addEventListener('pointerdown', onDown, true); document.addEventListener('keydown', onKey, true);
    return () => { document.removeEventListener('pointerdown', onDown, true); document.removeEventListener('keydown', onKey, true); };
  }, [open]);

  if (idle) return null;
  const sum = summarize(running);
  const label = running.length === 0 ? t('task.title') : sum.count === 1 ? t('task.btnOne') : tn('task.btn', { n: sum.count });
  const head = running.length === 1 ? taskTitle(running[0]) : kindLabel(running[0]?.kind ?? 'other');
  const num = running.length === 0 ? '' : sum.single && sum.total !== null ? progressText({ done: sum.done, total: sum.total, unit: running[0].unit }) : sum.count > 1 ? formatNumber(sum.count) : '';
  const cancel = (r: TaskRow) => { setConfirming(null); r.tasks.filter(x => x.cancellable && x.state !== 'cancelling').forEach(x => { void api.taskCancel(x.id).catch(() => undefined); }); };

  const popover = (
        <div ref={pop} id={id} className={`fa-taskpop${fixedAt ? ' fixed' : ''}`} style={fixedAt ?? undefined} role="dialog" aria-label={t('task.title')} data-testid="task-popover">
          <ul className="fa-task-list">
            {rows.map(r => <Row key={r.key} row={r} confirming={confirming === r.key} onAsk={() => setConfirming(r.key)} onKeep={() => setConfirming(null)} onCancel={() => cancel(r)} />)}
          </ul>
        </div>
  );

  return (
    <div className="fa-taskwrap">
      <Tooltip label={`${label} · ${head}`} disabled={open}>
        <button ref={btn} type="button" className="fa-iconbtn fa-taskbtn" data-testid="task-button" aria-label={label} aria-expanded={open} aria-controls={id} aria-haspopup="dialog" onClick={() => { if (!open) { const r = btn.current?.getBoundingClientRect(); const col = !!btn.current?.closest('.fa-side[data-collapsed="true"]'); setFixedAt(col && r ? { left: r.right + 6, bottom: Math.max(8, window.innerHeight - r.bottom) } : null); } setOpen(o => !o); }}>
          {running.length === 0 ? <Icon name="check" size={16} /> : <Ring value={sum.progress} />}
          {num && <span className="fa-task-count" data-testid="task-count">{num}</span>}
        </button>
      </Tooltip>
      {open && (fixedAt ? createPortal(popover, document.body) : popover)}
    </div>
  );
}
