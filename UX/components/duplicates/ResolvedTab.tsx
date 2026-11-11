import { useCallback, useEffect, useRef, useState } from 'react';
import type { Api, ResolutionDto } from '../../api';
import { t } from '../../i18n';
import { NoteMark } from '../ui/noteMark';
import { useUiStore } from '../../store/ui';
import { baseName } from './dupPlan';
import { reasonText } from './dupText';
import { undoWithFeedback } from './undoFlow';

const when = (sec: number): string => new Date(sec * 1000).toLocaleString('vi-VN', { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' });
const rowName = (r: ResolutionDto): string => (r.family ? `${r.family}${r.style ? ` ${r.style}` : ''}` : baseName(r.loserPath));

/** Tab "Đã giải quyết": mới nhất trước, gom theo mẻ, hoàn tác từng quyết định hoặc cả mẻ (thứ tự ngược do backend xếp). */
export function ResolvedTab({ api }: { api: Api }): React.JSX.Element {
  const rev = useUiStore(s => s.libraryRev);
  const [list, setList] = useState<ResolutionDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showUndone, setShowUndone] = useState(false);
  const [busy, setBusy] = useState(false);
  const [hint, setHint] = useState<string | null>(null);
  const seq = useRef(0);
  const load = useCallback(() => {
    const mine = ++seq.current; setError(null);
    api.resolveList(showUndone).then(l => { if (mine === seq.current) setList(l); }, e => { if (mine === seq.current) setError(e instanceof Error ? e.message : String(e)); });
  }, [api, showUndone]);
  useEffect(() => { load(); return () => { seq.current++; }; }, [load, rev]);

  const undo = async (ids: number[]) => {
    if (busy) return;
    setBusy(true); setHint(null);
    try {
      const r = await undoWithFeedback(api, ids);
      const dep = r.failed.find(f => f.code === 'newer_resolution_depends');
      if (dep) setHint(t('dup.done.dependsHint'));
    } finally { setBusy(false); load(); }
  };

  const batches: Array<{ id: string; rows: ResolutionDto[] }> = [];
  for (const r of list ?? []) { const b = batches.find(x => x.id === r.batchId); if (b) b.rows.push(r); else batches.push({ id: r.batchId, rows: [r] }); }

  return (
    <div className="dup-tabbody" data-testid="dup-resolved">
      <div className="dup-toolbar">
        <label><input type="checkbox" checked={showUndone} onChange={e => setShowUndone(e.target.checked)} /> {t('dup.done.showUndone')}</label>
        <span className="dup-note">{t('dup.done.order')}</span>
      </div>
      <div aria-live="polite">{hint && <p className="dup-state warn" role="status">{hint}</p>}</div>
      {error && <div role="alert" className="dup-state warn">{t('dup.done.error', { msg: error })} <button type="button" onClick={load}>{t('dup.retry')}</button></div>}
      {!error && !list && <div role="status" className="dup-state">{t('dup.done.loading')}</div>}
      {list && list.length === 0 && <div className="dup-state">{t('dup.done.empty')}</div>}
      {batches.map((b, bi) => {
        const live = b.rows.filter(r => r.undoneAt === null && r.canUndo);
        return (
          <section key={b.id} className="dup-batch" aria-label={`${t('dup.done.batch', { n: batches.length - bi })} · ${when(b.rows[0].createdAt)}`} data-testid="dup-batch">
            <header>
              <b>{when(b.rows[0].createdAt)}</b>
              <span className="dup-count">{t('dup.done.batchCount', { n: b.rows.length })}</span>
              <span className="dup-sp" />
              {live.length > 1 && <button type="button" aria-disabled={busy} onClick={() => void undo(live.map(r => r.id))}>{t('dup.done.undoBatch')}</button>}
            </header>
            <ul>
              {b.rows.map(r => (
                <li key={r.id} data-testid="dup-res-row" data-undone={r.undoneAt !== null}>
                  <div className="dup-res-main">
                    <b>{rowName(r)}</b>
                    <span className="dup-fmt">{r.action === 'trash' ? t('dup.done.trash') : t('dup.done.hide')}</span>
                    <span className="dup-muted">{t('dup.done.keepTo', { keeper: baseName(r.keeperPath) })}</span>
                    {r.status === 'pending' && <NoteMark text={t('dup.done.pending')} inline />}
                    {r.wasActive && <NoteMark kind="info" text={t('dup.done.wasActive')} inline />}
                  </div>
                  <p className="dup-why">{reasonText(r.reason)}{r.linksMoved > 0 && ` · ${t('dup.done.links', { n: r.linksMoved })}`}</p>
                  <div className="dup-res-act">
                    {r.undoneAt !== null ? <span className="dup-muted">{t('dup.done.undoneAt')} · {when(r.undoneAt)}</span>
                      : r.canUndo ? <button type="button" aria-disabled={busy} onClick={() => void undo([r.id])}>{t('dup.done.undo')}</button>
                      : <span className="dup-muted" data-testid="cant-undo">{t('dup.done.cantUndo')}</span>}
                  </div>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </div>
  );
}

